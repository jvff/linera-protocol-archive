// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    borrow::{Borrow, Cow},
    collections::{BTreeMap, VecDeque},
    fmt::Debug,
    iter,
    marker::PhantomData,
    ops::{Bound, RangeBounds, RangeInclusive},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use concurrent_map::ConcurrentMap;
use futures::FutureExt as _;
use linera_base::prometheus_util::{self, MeasureLatency};
use linera_base::sync::Lazy;
use prometheus::HistogramVec;
use thiserror::Error;

use crate::{
    batch::{Batch, DeletePrefixExpander, WriteOperation},
    common::{
        get_interval, get_upper_bound_option, AdminKeyValueStore, CommonStoreConfig, Context,
        ContextFromStore, KeyIterable, KeyValueStore, ReadableKeyValueStore, WritableKeyValueStore,
    },
    value_splitting::DatabaseConsistencyError,
    views::ViewError,
};

/// The initial configuration of the system
#[derive(Debug)]
pub struct MemoryStoreConfig {
    /// The common configuration of the key value store
    pub common_config: CommonStoreConfig,
}

impl MemoryStoreConfig {
    /// Creates a `MemoryStoreConfig`. `max_concurrent_queries` and `cache_size` are not used.
    pub fn new(max_stream_queries: usize) -> Self {
        let common_config = CommonStoreConfig {
            max_concurrent_queries: None,
            max_stream_queries,
            cache_size: 1000,
        };
        Self { common_config }
    }
}

/// The number of streams for the test
pub const TEST_MEMORY_MAX_STREAM_QUERIES: usize = 10;

/// The data is serialized in memory just like for RocksDB / DynamoDB
/// The analog of the database is the BTreeMap
#[derive(Debug)]
pub struct MemoryStoreMap {
    buckets: [InstrumentedRwLock<BTreeMap<Vec<u8>, Vec<u8>>>; 256],
}

impl Default for MemoryStoreMap {
    fn default() -> Self {
        macro_rules! init_array {
            ($( $comma:tt )*) => {
                [ $( InstrumentedRwLock::default() $comma )* ]
            }
        }

        MemoryStoreMap {
            buckets: init_array!(
                ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,
                ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,
                ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,
                ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,

                ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,
                ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,
                ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,
                ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,
            ),
        }
    }
}

impl MemoryStoreMap {
    /// Contains key
    pub fn contains_key(&self, key: &[u8]) -> bool {
        self.read_bucket(key).contains_key(key)
    }

    /// Get
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.read_bucket(key).get(key).cloned()
    }

    /// Get many
    pub fn get_many<Key>(&self, keys: impl IntoIterator<Item = Key>) -> Vec<Option<Vec<u8>>>
    where
        Key: Borrow<Vec<u8>>,
    {
        let (values, _bucket_guards) = keys
            .into_iter()
            .map(|key| {
                let bucket = self.read_bucket(key.borrow());
                let value = bucket.get(key.borrow()).cloned();
                (value, bucket)
            })
            .unzip::<_, _, _, Vec<_>>();
        values
    }

    /// Range
    pub fn range<'iter, Range, Key>(
        &'iter self,
        range: Range,
    ) -> MemoryStoreMapRange<'iter, Range, Key>
    where
        Range: RangeBounds<Key> + 'iter,
        Key: Borrow<[u8]> + Ord,
        Vec<u8>: Borrow<Key>,
    {
        let mut buckets = self.read_buckets_in_range(&range);
        let cursor = Self::next_cursor(&mut buckets, range.start_bound());

        MemoryStoreMapRange {
            range,
            cursor,
            buckets,
            _key: PhantomData,
        }
    }

    /// Edit
    pub fn edit_ranges<Range, Key>(
        &self,
        ranges: impl Iterator<Item = Range>,
    ) -> MemoryStoreMapMutRef<'_>
    where
        Range: RangeBounds<Key>,
        Key: Borrow<[u8]>,
    {
        let mut bucket_bitmap = [0_u64; 4];

        for range in ranges {
            for bucket_index in Self::bucket_indices_for(&range) {
                let slot = bucket_index >> 6;
                let mask = 1 << (bucket_index & 0x3F);

                bucket_bitmap[slot] |= mask;
            }
        }

        let bucket_flags = bucket_bitmap
            .into_iter()
            .map(|slot| {
                (0..64).map(move |offset| {
                    let selected_bit = slot & (1 << offset);
                    selected_bit != 0
                })
            })
            .flatten();

        macro_rules! init_array {
            ($( $comma:tt )*) => {
                [ $( None $comma )* ]
            }
        }

        let mut buckets = init_array!(
            ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,
            ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,
            ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,
            ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,

            ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,
            ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,
            ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,
            ,,,,,,,, ,,,,,,,, ,,,,,,,, ,,,,,,,,
        );

        for (index, (should_lock, bucket)) in bucket_flags.zip(&self.buckets).enumerate() {
            if should_lock {
                buckets[index] = Some(bucket.write());
            }
        }

        MemoryStoreMapMutRef { buckets }
    }

    fn read_bucket(&self, key: &[u8]) -> RwLockReadGuard<'_, BTreeMap<Vec<u8>, Vec<u8>>> {
        self.buckets[Self::bucket_index_of(key)].read()
    }

    fn read_buckets_in_range<Key>(
        &self,
        range: &impl RangeBounds<Key>,
    ) -> VecDeque<RwLockReadGuard<'_, BTreeMap<Vec<u8>, Vec<u8>>>>
    where
        Key: Borrow<[u8]>,
    {
        Self::bucket_indices_for(range)
            .map(|index| self.buckets[index].read())
            .collect()
    }

    fn bucket_index_of(key: &[u8]) -> usize {
        key.iter().copied().next().unwrap_or(0) as usize
    }

    fn bucket_indices_for<Key>(range: &impl RangeBounds<Key>) -> RangeInclusive<usize>
    where
        Key: Borrow<[u8]>,
    {
        let start_bucket_index = match range.start_bound() {
            Bound::Included(start_key) | Bound::Excluded(start_key) => {
                Self::bucket_index_of(start_key.borrow())
            }
            Bound::Unbounded => 0,
        };
        let end_bucket_index = match range.end_bound() {
            Bound::Included(end_key) | Bound::Excluded(end_key) => {
                Self::bucket_index_of(end_key.borrow())
            }
            Bound::Unbounded => 255,
        };

        start_bucket_index..=end_bucket_index
    }

    fn next_cursor<'lock, Key>(
        buckets: &'lock mut VecDeque<RwLockReadGuard<'lock, BTreeMap<Vec<u8>, Vec<u8>>>>,
        cursor_position: Bound<&Key>,
    ) -> Option<(&'lock Vec<u8>, &'lock Vec<u8>)>
    where
        Key: Ord,
        Vec<u8>: Borrow<Key>,
    {
        let range = (cursor_position, Bound::<&Key>::Unbounded);

        loop {
            match buckets.front_mut()?.range(range).next() {
                Some(_) => return buckets.front()?.range(range).next(),
                None => {
                    buckets.pop_front();
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct MemoryStoreMapMutRef<'lock> {
    buckets: [Option<RwLockWriteGuard<'lock, BTreeMap<Vec<u8>, Vec<u8>>>>; 256],
}

impl MemoryStoreMapMutRef<'_> {
    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let bucket_index = MemoryStoreMap::bucket_index_of(&key);
        let bucket = self
            .buckets
            .get_mut(bucket_index)
            .expect("`buckets` array should have an entry for every byte value")
            .as_mut()
            .expect("`MemoryStoreMapMutRef` should only be used to insert in locked buckets");

        bucket.insert(key, value);
    }

    pub fn remove(&mut self, key: &[u8]) {
        let bucket_index = MemoryStoreMap::bucket_index_of(&key);
        let bucket = self
            .buckets
            .get_mut(bucket_index)
            .expect("`buckets` array should have an entry for every byte value")
            .as_mut()
            .expect("`MemoryStoreMapMutRef` should only be used to remove from locked buckets");

        bucket.remove(key);
    }

    pub fn remove_range<Key>(&mut self, range: impl RangeBounds<Key>)
    where
        Key: Borrow<[u8]> + Ord,
        Vec<u8>: Borrow<Key> + Ord,
    {
        let mut bucket_indices = MemoryStoreMap::bucket_indices_for(&range).peekable();
        let first_bucket_index = bucket_indices
            .next()
            .expect("Range should map to at least one bucket");
        let first_bucket = self
            .buckets
            .get_mut(first_bucket_index)
            .expect("`buckets` array should have an entry for every byte value")
            .as_mut()
            .expect("`MemoryStoreMapMutRef` should only be used to remove from locked buckets");

        if bucket_indices.peek().is_none() {
            let bucket = first_bucket;
            let keys_to_remove = bucket
                .range(range)
                .map(|(key, _value)| key.to_owned())
                .collect::<Vec<_>>();

            for key in keys_to_remove {
                bucket.remove(key.borrow());
            }
        } else {
            let first_range = (range.start_bound(), Bound::Unbounded);

            if let Some((first_key, _value)) = first_bucket.range(first_range).next() {
                let first_key = first_key.borrow().borrow().to_vec();
                first_bucket.split_off::<Vec<u8>>(&first_key);
            }

            let mut bucket_index = bucket_indices
                .next()
                .expect("Peek should have checked for a second item");

            while bucket_indices.peek().is_some() {
                let bucket = self
                    .buckets
                    .get_mut(bucket_index)
                    .expect("`buckets` array should have an entry for every byte value")
                    .as_mut()
                    .expect(
                        "`MemoryStoreMapMutRef` should only be used to remove from locked buckets",
                    );

                bucket.clear();
                bucket_index = bucket_indices
                    .next()
                    .expect("Peek should have checked for the next item");
            }

            let last_bucket = self
                .buckets
                .get_mut(bucket_index)
                .expect("`buckets` array should have an entry for every byte value")
                .as_mut()
                .expect("`MemoryStoreMapMutRef` should only be used to remove from locked buckets");

            let end_bound = match range.end_bound() {
                Bound::Unbounded => Bound::Unbounded,
                Bound::Included(key) => Bound::Excluded(key),
                Bound::Excluded(key) => Bound::Excluded(key),
            };

            let first_retained_key = last_bucket.range((end_bound, Bound::Unbounded)).next();
            if let Some((key, _value)) = first_retained_key {
                let key = key.borrow().borrow().to_vec();
                let new_last_bucket = last_bucket.split_off::<Vec<u8>>(&key);
                **last_bucket = new_last_bucket;
            }
        }
    }
}

pub struct MemoryStoreMapRange<'iter, Range, Key> {
    range: Range,
    cursor: Option<(&'iter Vec<u8>, &'iter Vec<u8>)>,
    buckets: VecDeque<RwLockReadGuard<'iter, BTreeMap<Vec<u8>, Vec<u8>>>>,
    _key: PhantomData<Key>,
}

impl<'iter, Range, Key> Iterator for MemoryStoreMapRange<'iter, Range, Key>
where
    Range: RangeBounds<Key>,
    Key: Borrow<Vec<u8>>,
    Key: Ord,
{
    type Item = (&'iter Vec<u8>, &'iter Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        let (current_key, current_value) = self.cursor.take()?;

        self.cursor = MemoryStoreMap::next_cursor(&mut self.buckets, Bound::Excluded(current_key));

        Some((current_key, current_value))
    }
}

/// A virtual DB client where data are persisted in memory.
#[derive(Clone)]
pub struct MemoryStore {
    /// The map used for storing the data.
    pub map: Arc<MemoryStoreMap>,
    /// The maximum number of queries used for the stream.
    pub max_stream_queries: usize,
}

#[cfg(with_metrics)]
static READ_LOCK_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    prometheus_util::register_histogram_vec(
        "memory_store_read_lock_latency",
        "Latency to acquire the memory store's read lock",
        &[],
        Some(vec![
            0.000_1, 0.000_25, 0.000_5, 0.001, 0.002_5, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5,
            1.0, 2.5, 5.0, 10.0, 25.0, 50.0,
        ]),
    )
    .expect("Counter creation should not fail")
});

#[cfg(with_metrics)]
static WRITE_LOCK_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    prometheus_util::register_histogram_vec(
        "memory_store_write_lock_latency",
        "Latency to acquire the memory store's write lock",
        &[],
        Some(vec![
            0.000_1, 0.000_25, 0.000_5, 0.001, 0.002_5, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5,
            1.0, 2.5, 5.0, 10.0, 25.0, 50.0,
        ]),
    )
    .expect("Counter creation should not fail")
});

/// Wrap
#[derive(Debug, Default)]
pub struct InstrumentedRwLock<T>(RwLock<T>);

impl<T> InstrumentedRwLock<T> {
    /// Read
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        let _measurement = READ_LOCK_LATENCY.measure_latency();
        self.0.read().expect("Poisoned `RwLock`")
    }

    /// Write
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        let _measurement = WRITE_LOCK_LATENCY.measure_latency();
        self.0.write().expect("Poisoned `RwLock`")
    }
}

impl ReadableKeyValueStore<MemoryStoreError> for MemoryStore {
    const MAX_KEY_SIZE: usize = usize::MAX;
    type Keys = Vec<Vec<u8>>;
    type KeyValues = Vec<(Vec<u8>, Vec<u8>)>;

    fn max_stream_queries(&self) -> usize {
        self.max_stream_queries
    }

    async fn read_value_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>, MemoryStoreError> {
        Ok(self.map.get(key))
    }

    async fn contains_key(&self, key: &[u8]) -> Result<bool, MemoryStoreError> {
        Ok(self.map.contains_key(key))
    }

    async fn read_multi_values_bytes(
        &self,
        keys: Vec<Vec<u8>>,
    ) -> Result<Vec<Option<Vec<u8>>>, MemoryStoreError> {
        Ok(self.map.get_many(keys))
    }

    async fn find_keys_by_prefix(
        &self,
        key_prefix: &[u8],
    ) -> Result<Vec<Vec<u8>>, MemoryStoreError> {
        let mut values = Vec::new();
        let len = key_prefix.len();
        for (key, _value) in self.map.range(get_interval(key_prefix.to_vec())) {
            values.push(key[len..].to_vec())
        }
        Ok(values)
    }

    async fn find_key_values_by_prefix(
        &self,
        key_prefix: &[u8],
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>, MemoryStoreError> {
        let mut key_values = Vec::new();
        let len = key_prefix.len();
        for (key, value) in self.map.range(get_interval(key_prefix.to_vec())) {
            let key_value = (key[len..].to_vec(), value.to_vec());
            key_values.push(key_value);
        }
        Ok(key_values)
    }
}

impl WritableKeyValueStore<MemoryStoreError> for MemoryStore {
    const MAX_VALUE_SIZE: usize = usize::MAX;

    async fn write_batch(&self, batch: Batch, _base_key: &[u8]) -> Result<(), MemoryStoreError> {
        let ranges_to_lock = batch.operations.iter().map(|operation| match operation {
            WriteOperation::Put { key, .. } | WriteOperation::Delete { key } => (
                Bound::Included(Cow::Borrowed(key.as_slice())),
                Bound::Excluded(Cow::Borrowed(key.as_slice())),
            ),
            WriteOperation::DeletePrefix { key_prefix } => {
                let upper_bound = match get_upper_bound_option(key_prefix) {
                    None => Bound::Unbounded,
                    Some(upper_bound) => Bound::Excluded(Cow::Owned(upper_bound)),
                };

                (
                    Bound::Included(Cow::Borrowed(key_prefix.as_slice())),
                    upper_bound,
                )
            }
        });
        let mut map = self.map.edit_ranges(ranges_to_lock);

        for operation in batch.operations {
            match operation {
                WriteOperation::Put { key, value } => map.insert(key, value),
                WriteOperation::Delete { key } => map.remove(&key),
                WriteOperation::DeletePrefix { key_prefix } => {
                    map.remove_range(get_interval(key_prefix))
                }
            }
        }

        Ok(())
    }

    async fn clear_journal(&self, _base_key: &[u8]) -> Result<(), MemoryStoreError> {
        Ok(())
    }
}

impl AdminKeyValueStore for MemoryStore {
    type Error = MemoryStoreError;
    type Config = MemoryStoreConfig;

    async fn connect(config: &Self::Config, _namespace: &str) -> Result<Self, MemoryStoreError> {
        let max_stream_queries = config.common_config.max_stream_queries;
        let map = Arc::new(MemoryStoreMap::default());
        Ok(MemoryStore {
            map,
            max_stream_queries,
        })
    }

    async fn list_all(_config: &Self::Config) -> Result<Vec<String>, MemoryStoreError> {
        Ok(Vec::new())
    }

    async fn exists(_config: &Self::Config, _namespace: &str) -> Result<bool, MemoryStoreError> {
        Ok(false)
    }

    async fn create(_config: &Self::Config, _namespace: &str) -> Result<(), MemoryStoreError> {
        Ok(())
    }

    async fn delete(_config: &Self::Config, _namespace: &str) -> Result<(), MemoryStoreError> {
        Ok(())
    }
}

impl KeyValueStore for MemoryStore {
    type Error = MemoryStoreError;
}

/// An implementation of [`crate::common::Context`] that stores all values in memory.
pub type MemoryContext<E> = ContextFromStore<E, MemoryStore>;

impl<E> MemoryContext<E> {
    /// Creates a [`MemoryContext`].
    pub fn new(max_stream_queries: usize, extra: E) -> Self {
        let common_config = CommonStoreConfig {
            max_concurrent_queries: None,
            max_stream_queries,
            cache_size: 1000,
        };
        let config = MemoryStoreConfig { common_config };
        let namespace = "linera";
        let store = MemoryStore::connect(&config, namespace)
            .now_or_never()
            .unwrap()
            .unwrap();
        let base_key = Vec::new();
        Self {
            store,
            base_key,
            extra,
        }
    }
}

/// Provides a `MemoryContext<()>` that can be used for tests.
/// It is not named create_memory_test_context because it is massively
/// used and so we want to have a short name.
pub fn create_memory_context() -> MemoryContext<()> {
    MemoryContext::new(TEST_MEMORY_MAX_STREAM_QUERIES, ())
}

/// Creates a test memory client for working.
pub fn create_memory_store_stream_queries(max_stream_queries: usize) -> MemoryStore {
    let common_config = CommonStoreConfig {
        max_concurrent_queries: None,
        max_stream_queries,
        cache_size: 1000,
    };
    let config = MemoryStoreConfig { common_config };
    let namespace = "linera";
    MemoryStore::connect(&config, namespace)
        .now_or_never()
        .unwrap()
        .unwrap()
}

/// Creates a test memory store for working.
pub fn create_memory_store() -> MemoryStore {
    create_memory_store_stream_queries(TEST_MEMORY_MAX_STREAM_QUERIES)
}

/// The error type for [`MemoryContext`].
#[derive(Error, Debug)]
pub enum MemoryStoreError {
    /// Serialization error with BCS.
    #[error("BCS error: {0}")]
    Bcs(#[from] bcs::Error),

    /// The value is too large for the MemoryStore
    #[error("The value is too large for the MemoryStore")]
    TooLargeValue,

    /// The database is not consistent
    #[error(transparent)]
    DatabaseConsistencyError(#[from] DatabaseConsistencyError),
}

impl From<MemoryStoreError> for ViewError {
    fn from(error: MemoryStoreError) -> Self {
        Self::StoreError {
            backend: "memory".to_string(),
            error: error.to_string(),
        }
    }
}

impl DeletePrefixExpander for MemoryContext<()> {
    type Error = MemoryStoreError;

    async fn expand_delete_prefix(&self, key_prefix: &[u8]) -> Result<Vec<Vec<u8>>, Self::Error> {
        let mut vector_list = Vec::new();
        for key in <Vec<Vec<u8>> as KeyIterable<Self::Error>>::iterator(
            &self.find_keys_by_prefix(key_prefix).await?,
        ) {
            vector_list.push(key?.to_vec());
        }
        Ok(vector_list)
    }
}
