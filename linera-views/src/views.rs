// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::FutureExt;
use linera_base::ensure;
use std::{
    cmp::Eq,
    collections::{btree_map, BTreeMap, VecDeque},
    fmt::Debug,
    mem,
    ops::{Deref, DerefMut, Range},
    sync::Arc,
};
use thiserror::Error;
use tokio::sync::{Mutex, OwnedMutexGuard};

/// The context in which a view is operated. Typically, this includes the client to
/// connect to the database and the address of the current entry.
pub trait Context {
    /// The error type in use.
    type Error: std::error::Error + Debug + Send + Sync + From<ViewError> + From<std::io::Error>;
}

/// A view gives an exclusive access to read and write the data stored at an underlying
/// address in storage.
#[async_trait]
pub trait View<C: Context>: Sized {
    /// Create a view or a subview.
    async fn load(context: C) -> Result<Self, C::Error>;

    /// Discard all pending changes. After that `commit` should have no effect to storage.
    fn rollback(&mut self);

    /// Persist changes to storage. This consumes the view. If the view is dropped without
    /// calling `commit`, staged changes are simply lost.
    async fn commit(self) -> Result<(), C::Error>;

    /// Instead of persisting changes, clear all the data that belong to this view and its
    /// subviews.
    async fn delete(self) -> Result<(), C::Error>;
}

#[derive(Error, Debug)]
pub enum ViewError {
    #[error("the entry with key {0} was removed thus cannot be loaded any more")]
    RemovedEntry(String),

    #[error("failed to serialize value to calculate its hash")]
    Serialization(#[from] bcs::Error),
}

/// A view that adds a prefix to all the keys of the contained view.
#[derive(Debug, Clone)]
pub struct ScopedView<const INDEX: u64, W> {
    pub(crate) view: W,
}

impl<W, const INDEX: u64> std::ops::Deref for ScopedView<INDEX, W> {
    type Target = W;

    fn deref(&self) -> &W {
        &self.view
    }
}

impl<W, const INDEX: u64> std::ops::DerefMut for ScopedView<INDEX, W> {
    fn deref_mut(&mut self) -> &mut W {
        &mut self.view
    }
}

pub trait ScopedOperations: Context {
    /// Clone the context and advance the (otherwise implicit) base key for all read/write
    /// operations.
    fn clone_with_scope(&self, index: u64) -> Self;
}

#[async_trait]
impl<C, W, const INDEX: u64> View<C> for ScopedView<INDEX, W>
where
    C: Context + Send + Sync + ScopedOperations + 'static,
    W: View<C> + Send,
{
    async fn load(context: C) -> Result<Self, C::Error> {
        let view = W::load(context.clone_with_scope(INDEX)).await?;
        Ok(Self { view })
    }

    fn rollback(&mut self) {
        self.view.rollback();
    }

    async fn commit(mut self) -> Result<(), C::Error> {
        self.view.commit().await
    }

    async fn delete(self) -> Result<(), C::Error> {
        self.view.delete().await
    }
}

/// A view that supports modifying a single value of type `T`.
#[derive(Debug, Clone)]
pub struct RegisterView<C, T> {
    context: C,
    stored_value: T,
    update: Option<T>,
}

/// The context operations supporting [`RegisterView`].
#[async_trait]
pub trait RegisterOperations<T>: Context {
    async fn get(&mut self) -> Result<T, Self::Error>;

    async fn set(&mut self, value: T) -> Result<(), Self::Error>;

    async fn delete(&mut self) -> Result<(), Self::Error>;
}

#[async_trait]
impl<C, T> View<C> for RegisterView<C, T>
where
    C: RegisterOperations<T> + Send + Sync,
    T: Send + Sync,
{
    async fn load(mut context: C) -> Result<Self, C::Error> {
        let stored_value = context.get().await?;
        Ok(Self {
            context,
            stored_value,
            update: None,
        })
    }

    fn rollback(&mut self) {
        self.update = None
    }

    async fn commit(mut self) -> Result<(), C::Error> {
        if let Some(value) = self.update {
            self.context.set(value).await?;
        }
        Ok(())
    }

    async fn delete(mut self) -> Result<(), C::Error> {
        self.context.delete().await
    }
}

impl<C, T> RegisterView<C, T>
where
    C: Context,
{
    /// Access the current value in the register.
    pub fn get(&self) -> &T {
        match &self.update {
            None => &self.stored_value,
            Some(value) => value,
        }
    }

    /// Set the value in the register.
    pub fn set(&mut self, value: T) {
        self.update = Some(value);
    }
}

impl<C, T> RegisterView<C, T>
where
    C: Context,
    T: Clone,
{
    /// Obtain a mutable reference to the value in the register.
    pub fn get_mut(&mut self) -> &mut T {
        match &mut self.update {
            Some(value) => value,
            update => {
                *update = Some(self.stored_value.clone());
                update.as_mut().unwrap()
            }
        }
    }
}

/// A view that supports logging values of type `T`.
#[derive(Debug, Clone)]
pub struct AppendOnlyLogView<C, T> {
    context: C,
    stored_count: usize,
    new_values: Vec<T>,
}

/// The context operations supporting [`AppendOnlyLogView`].
#[async_trait]
pub trait AppendOnlyLogOperations<T>: Context {
    async fn count(&mut self) -> Result<usize, Self::Error>;

    async fn get(&mut self, index: usize) -> Result<Option<T>, Self::Error>;

    async fn read(&mut self, range: Range<usize>) -> Result<Vec<T>, Self::Error>;

    async fn append(&mut self, values: Vec<T>) -> Result<(), Self::Error>;

    async fn delete(&mut self) -> Result<(), Self::Error>;
}

#[async_trait]
impl<C, T> View<C> for AppendOnlyLogView<C, T>
where
    C: AppendOnlyLogOperations<T> + Send + Sync,
    T: Send + Sync + Clone,
{
    async fn load(mut context: C) -> Result<Self, C::Error> {
        let stored_count = context.count().await?;
        Ok(Self {
            context,
            stored_count,
            new_values: Vec::new(),
        })
    }

    fn rollback(&mut self) {
        self.new_values.clear();
    }

    async fn commit(mut self) -> Result<(), C::Error> {
        self.context.append(self.new_values).await?;
        Ok(())
    }

    async fn delete(mut self) -> Result<(), C::Error> {
        self.context.delete().await
    }
}

impl<C, T> AppendOnlyLogView<C, T>
where
    C: Context,
{
    /// Push a value to the end of the log.
    pub fn push(&mut self, value: T) {
        self.new_values.push(value);
    }

    /// Read the size of the log.
    pub fn count(&self) -> usize {
        self.stored_count + self.new_values.len()
    }
}

impl<C, T> AppendOnlyLogView<C, T>
where
    C: AppendOnlyLogOperations<T> + Send + Sync,
    T: Send + Sync + Clone,
{
    /// Read the logged values in the given range (including staged ones).
    pub async fn get(&mut self, index: usize) -> Result<Option<T>, C::Error> {
        if index < self.stored_count {
            self.context.get(index).await
        } else {
            Ok(self.new_values.get(index - self.stored_count).cloned())
        }
    }

    /// Read the logged values in the given range (including staged ones).
    pub async fn read(&mut self, mut range: Range<usize>) -> Result<Vec<T>, C::Error> {
        if range.end > self.count() {
            range.end = self.count();
        }
        if range.start >= range.end {
            return Ok(Vec::new());
        }
        let mut values = Vec::new();
        values.reserve(range.end - range.start);
        if range.start < self.stored_count {
            if range.end <= self.stored_count {
                values.extend(self.context.read(range.start..range.end).await?);
            } else {
                values.extend(self.context.read(range.start..self.stored_count).await?);
                values.extend(self.new_values[0..(range.end - self.stored_count)].to_vec());
            }
        } else {
            values.extend(
                self.new_values[(range.start - self.stored_count)..(range.end - self.stored_count)]
                    .to_vec(),
            );
        }
        Ok(values)
    }
}

/// A view that supports inserting and removing values indexed by a key.
#[derive(Debug, Clone)]
pub struct MapView<C, I, V> {
    context: C,
    updates: BTreeMap<I, Option<V>>,
}

/// The context operations supporting [`MapView`].
#[async_trait]
pub trait MapOperations<I, V>: Context {
    /// Obtain the value at the given index, if any.
    async fn get(&mut self, index: &I) -> Result<Option<V>, Self::Error>;

    /// Set a value.
    async fn insert(&mut self, index: I, value: V) -> Result<(), Self::Error>;

    /// Remove the entry at the given index.
    async fn remove(&mut self, index: I) -> Result<(), Self::Error>;

    /// Return the list of indices in the map.
    async fn indices(&mut self) -> Result<Vec<I>, Self::Error>;

    /// Delete the map and its entries from storage.
    async fn delete(&mut self) -> Result<(), Self::Error>;
}

#[async_trait]
impl<C, I, V> View<C> for MapView<C, I, V>
where
    C: MapOperations<I, V> + Send,
    I: Eq + Ord + Send + Sync,
    V: Clone + Send + Sync,
{
    async fn load(context: C) -> Result<Self, C::Error> {
        Ok(Self {
            context,
            updates: BTreeMap::new(),
        })
    }

    fn rollback(&mut self) {
        self.updates.clear();
    }

    async fn commit(mut self) -> Result<(), C::Error> {
        for (index, update) in self.updates {
            match update {
                None => {
                    self.context.remove(index).await?;
                }
                Some(value) => {
                    self.context.insert(index, value).await?;
                }
            }
        }
        Ok(())
    }

    async fn delete(mut self) -> Result<(), C::Error> {
        self.context.delete().await
    }
}

impl<C, I, V> MapView<C, I, V>
where
    C: Context,
    I: Eq + Ord,
{
    /// Set or insert a value.
    pub fn insert(&mut self, index: I, value: V) {
        self.updates.insert(index, Some(value));
    }

    /// Remove a value.
    pub fn remove(&mut self, index: I) {
        self.updates.insert(index, None);
    }
}

impl<C, I, V> MapView<C, I, V>
where
    C: MapOperations<I, V>,
    I: Eq + Ord + Sync + Clone,
    V: Clone,
{
    /// Read the value at the given position, if any.
    pub async fn get(&mut self, index: &I) -> Result<Option<V>, C::Error> {
        if let Some(update) = self.updates.get(index) {
            return Ok(update.as_ref().cloned());
        }
        self.context.get(index).await
    }

    /// Return the list of indices in the map.
    pub async fn indices(&mut self) -> Result<Vec<I>, C::Error> {
        let mut indices = Vec::new();
        for index in self.context.indices().await? {
            if !self.updates.contains_key(&index) {
                indices.push(index);
            }
        }
        for (index, entry) in &self.updates {
            if entry.is_some() {
                indices.push(index.clone());
            }
        }
        indices.sort();
        Ok(indices)
    }
}

/// A view that supports a FIFO queue for values of type `T`.
#[derive(Debug, Clone)]
pub struct QueueView<C, T> {
    context: C,
    stored_indices: Range<usize>,
    front_delete_count: usize,
    new_back_values: VecDeque<T>,
}

/// The context operations supporting [`QueueView`].
#[async_trait]
pub trait QueueOperations<T>: Context {
    async fn indices(&mut self) -> Result<Range<usize>, Self::Error>;

    async fn get(&mut self, index: usize) -> Result<Option<T>, Self::Error>;

    async fn read(&mut self, range: Range<usize>) -> Result<Vec<T>, Self::Error>;

    async fn delete_front(&mut self, count: usize) -> Result<(), Self::Error>;

    async fn append_back(&mut self, values: Vec<T>) -> Result<(), Self::Error>;

    /// Delete the queue from storage.
    async fn delete(&mut self) -> Result<(), Self::Error>;
}

#[async_trait]
impl<C, T> View<C> for QueueView<C, T>
where
    C: QueueOperations<T> + Send + Sync,
    T: Send + Sync + Clone,
{
    async fn load(mut context: C) -> Result<Self, C::Error> {
        let stored_indices = context.indices().await?;
        Ok(Self {
            context,
            stored_indices,
            front_delete_count: 0,
            new_back_values: VecDeque::new(),
        })
    }

    fn rollback(&mut self) {
        self.front_delete_count = 0;
        self.new_back_values.clear();
    }

    async fn commit(mut self) -> Result<(), C::Error> {
        self.context.delete_front(self.front_delete_count).await?;
        self.context
            .append_back(self.new_back_values.into_iter().collect())
            .await?;
        Ok(())
    }

    async fn delete(mut self) -> Result<(), C::Error> {
        self.context.delete().await
    }
}

impl<C, T> QueueView<C, T>
where
    C: QueueOperations<T> + Send + Sync,
    T: Send + Sync + Clone,
{
    /// Read the front value, if any.
    pub async fn front(&mut self) -> Result<Option<T>, C::Error> {
        let stored_remainder = self.stored_indices.len() - self.front_delete_count;
        if stored_remainder > 0 {
            self.context
                .get(self.stored_indices.end - stored_remainder)
                .await
        } else {
            Ok(self.new_back_values.front().cloned())
        }
    }

    /// Read the back value, if any.
    pub async fn back(&mut self) -> Result<Option<T>, C::Error> {
        match self.new_back_values.back() {
            Some(value) => Ok(Some(value.clone())),
            None if self.stored_indices.len() > self.front_delete_count => {
                self.context.get(self.stored_indices.end - 1).await
            }
            _ => Ok(None),
        }
    }

    /// Delete the front value, if any.
    pub fn delete_front(&mut self) {
        if self.front_delete_count < self.stored_indices.len() {
            self.front_delete_count += 1;
        } else {
            self.new_back_values.pop_front();
        }
    }

    /// Push a value to the end of the queue.
    pub fn push_back(&mut self, value: T) {
        self.new_back_values.push_back(value);
    }

    /// Read the size of the queue.
    pub fn count(&self) -> usize {
        self.stored_indices.len() - self.front_delete_count + self.new_back_values.len()
    }

    /// Read the `count` next values in the queue (including staged ones).
    pub async fn read_front(&mut self, mut count: usize) -> Result<Vec<T>, C::Error> {
        if count > self.count() {
            count = self.count();
        }
        if count == 0 {
            return Ok(Vec::new());
        }
        let mut values = Vec::new();
        values.reserve(count);
        let stored_remainder = self.stored_indices.len() - self.front_delete_count;
        let start = self.stored_indices.end - stored_remainder;
        if count <= stored_remainder {
            values.extend(self.context.read(start..(start + count)).await?);
        } else {
            values.extend(self.context.read(start..self.stored_indices.end).await?);
            values.extend(
                self.new_back_values
                    .range(0..(count - stored_remainder))
                    .cloned(),
            );
        }
        Ok(values)
    }

    /// Read the `count` last values in the queue (including staged ones).
    pub async fn read_back(&mut self, mut count: usize) -> Result<Vec<T>, C::Error> {
        if count > self.count() {
            count = self.count();
        }
        if count == 0 {
            return Ok(Vec::new());
        }
        let mut values = Vec::new();
        values.reserve(count);
        let new_back_len = self.new_back_values.len();
        if count <= new_back_len {
            values.extend(
                self.new_back_values
                    .range((new_back_len - count)..new_back_len)
                    .cloned(),
            );
        } else {
            let start = self.stored_indices.end + new_back_len - count;
            values.extend(self.context.read(start..self.stored_indices.end).await?);
            values.extend(self.new_back_values.iter().cloned());
        }
        Ok(values)
    }
}

/// A view that supports accessing a collection of views of the same kind, indexed by a
/// key.
#[derive(Debug, Clone)]
pub struct CollectionView<C, I, W> {
    context: C,
    updates: BTreeMap<I, Option<W>>,
}

/// The context operations supporting [`CollectionView`].
#[async_trait]
pub trait CollectionOperations<I>: Context {
    /// Clone the context and advance the (otherwise implicit) base key for all read/write
    /// operations.
    fn clone_with_scope(&self, index: &I) -> Self;

    /// Add the index to the list of indices.
    async fn add_index(&mut self, index: I) -> Result<(), Self::Error>;

    /// Remove the index from the list of indices.
    async fn remove_index(&mut self, index: I) -> Result<(), Self::Error>;

    /// Return the list of indices in the collection.
    async fn indices(&mut self) -> Result<Vec<I>, Self::Error>;
}

#[async_trait]
impl<C, I, W> View<C> for CollectionView<C, I, W>
where
    C: CollectionOperations<I> + Send,
    I: Send + Sync + Debug + Clone,
    W: View<C> + Send,
{
    async fn load(context: C) -> Result<Self, C::Error> {
        Ok(Self {
            context,
            updates: BTreeMap::new(),
        })
    }

    fn rollback(&mut self) {
        self.updates.clear();
    }

    async fn commit(mut self) -> Result<(), C::Error> {
        for (index, update) in self.updates {
            match update {
                Some(view) => {
                    view.commit().await?;
                    self.context.add_index(index).await?;
                }
                None => {
                    let context = self.context.clone_with_scope(&index);
                    self.context.remove_index(index).await?;
                    let view = W::load(context).await?;
                    view.delete().await?;
                }
            }
        }
        Ok(())
    }

    async fn delete(mut self) -> Result<(), C::Error> {
        for index in self.context.indices().await? {
            let context = self.context.clone_with_scope(&index);
            self.context.remove_index(index).await?;
            let view = W::load(context).await?;
            view.delete().await?;
        }
        Ok(())
    }
}

impl<C, I, W> CollectionView<C, I, W>
where
    C: CollectionOperations<I> + Send,
    I: Eq + Ord + Sync + Clone + Send + Debug,
    W: View<C>,
{
    /// Obtain a subview for the data at the given index in the collection. Return an
    /// error if `remove_entry` was used earlier on this index from the same [`CollectionView`].
    pub async fn load_entry(&mut self, index: I) -> Result<CollectionEntry<'_, I, W>, C::Error> {
        let entry = match self.updates.entry(index.clone()) {
            btree_map::Entry::Occupied(e) => match e.into_mut() {
                Some(view) => Ok(view),
                None => Err(C::Error::from(ViewError::RemovedEntry(format!(
                    "{:?}",
                    index
                )))),
            },
            btree_map::Entry::Vacant(e) => {
                let context = self.context.clone_with_scope(&index);
                let view = W::load(context).await?;
                Ok(e.insert(Some(view)).as_mut().unwrap())
            }
        }?;

        Ok(CollectionEntry { index, entry })
    }

    /// Mark the entry so that it is removed in the next commit.
    pub fn remove_entry(&mut self, index: I) {
        self.updates.insert(index, None);
    }

    /// Return the list of indices in the collection.
    pub async fn indices(&mut self) -> Result<Vec<I>, C::Error> {
        let mut indices = Vec::new();
        for index in self.context.indices().await? {
            if !self.updates.contains_key(&index) {
                indices.push(index);
            }
        }
        for (index, entry) in &self.updates {
            if entry.is_some() {
                indices.push(index.clone());
            }
        }
        indices.sort();
        Ok(indices)
    }
}

/// An active entry inside a [`CollectionView`].
pub struct CollectionEntry<'entry, Index, Entry> {
    index: Index,
    entry: &'entry mut Entry,
}

impl<Index, Entry> CollectionEntry<'_, Index, Entry> {
    /// The index of this entry.
    pub fn index(&self) -> &Index {
        &self.index
    }
}

impl<'entry, Index, Entry> Deref for CollectionEntry<'entry, Index, Entry> {
    type Target = Entry;

    fn deref(&self) -> &Self::Target {
        &*self.entry
    }
}

impl<'entry, Index, Entry> DerefMut for CollectionEntry<'entry, Index, Entry> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.entry
    }
}

/// A view that supports accessing a collection of views of the same kind, indexed by a key, in a
/// concurrent manner.
///
/// This is a variant of [`CollectionView`] that uses locks to guard each entry.
#[derive(Debug, Clone)]
pub struct SharedCollectionView<C, I, W> {
    context: C,
    updates: BTreeMap<I, Arc<Mutex<Option<W>>>>,
}

#[async_trait]
impl<C, I, W> View<C> for SharedCollectionView<C, I, W>
where
    C: CollectionOperations<I> + Send + Clone,
    I: Send + Sync + Debug + Clone + Ord,
    W: View<C> + Send,
{
    async fn load(context: C) -> Result<Self, C::Error> {
        Ok(Self {
            context,
            updates: BTreeMap::new(),
        })
    }

    fn rollback(&mut self) {
        // TODO: Maybe make `View::rollback` async?
        self.try_rollback()
            .now_or_never()
            .expect("Attempt to rollback a `SharedCollectionView` while an entry was still locked");
    }

    async fn commit(mut self) -> Result<(), C::Error> {
        for (index, update) in self.updates {
            match update.lock().await.take() {
                Some(view) => {
                    view.commit().await?;
                    self.context.add_index(index).await?;
                }
                None => {
                    let context = self.context.clone_with_scope(&index);
                    self.context.remove_index(index).await?;
                    let view = W::load(context).await?;
                    view.delete().await?;
                }
            }
        }
        Ok(())
    }

    async fn delete(mut self) -> Result<(), C::Error> {
        for index in self.context.indices().await? {
            let _guard = match self.updates.get(&index) {
                Some(entry) => Some(entry.lock().await),
                None => None,
            };
            let context = self.context.clone_with_scope(&index);
            self.context.remove_index(index).await?;
            let view = W::load(context).await?;
            view.delete().await?;
        }
        Ok(())
    }
}

impl<C, I, W> SharedCollectionView<C, I, W>
where
    C: CollectionOperations<I> + Clone + Send,
    I: Eq + Ord + Sync + Clone + Send + Debug,
    W: View<C>,
{
    /// Obtain a subview for the data at the given index in the collection. Return an
    /// error if `remove_entry` was used earlier on this index from the same [`CollectionView`].
    ///
    /// Note that the returned [`SharedCollectionEntry`] holds a lock on the entry, which means
    /// that subsequent calls to methods that access the same entry (or all entries, like `commit`,
    /// `rollback` and `delete`) won't complete until the lock is dropped.
    pub async fn load_entry(
        &mut self,
        index: I,
    ) -> Result<SharedCollectionEntry<C, I, W>, C::Error> {
        let entry = match self.updates.entry(index.clone()) {
            btree_map::Entry::Occupied(entry) => entry.into_mut(),
            btree_map::Entry::Vacant(entry) => {
                let context = self.context.clone_with_scope(&index);
                let view = W::load(context).await?;
                entry.insert(Arc::new(Mutex::new(Some(view))))
            }
        }
        .clone()
        .lock_owned()
        .await;

        ensure!(
            entry.is_some(),
            C::Error::from(ViewError::RemovedEntry(format!("{:?}", index)))
        );

        dbg!(format!("Creating entry: {index:?}"));
        Ok(SharedCollectionEntry {
            context: self.context.clone(),
            index,
            entry,
        })
    }

    /// Mark the entry so that it is removed in the next commit.
    ///
    /// If the entry is being editted, awaits until its lock is released.
    pub async fn remove_entry(&mut self, index: I) {
        match self.updates.entry(index) {
            btree_map::Entry::Occupied(entry) => {
                entry.get().lock().await.take();
            }
            btree_map::Entry::Vacant(entry) => {
                entry.insert(Arc::new(Mutex::new(None)));
            }
        }
    }

    /// Return the list of indices in the collection.
    ///
    /// If any entry is being editted, awaits until all entry locks are released.
    pub async fn indices(&mut self) -> Result<Vec<I>, C::Error> {
        let mut indices = Vec::new();
        for index in self.context.indices().await? {
            if !self.updates.contains_key(&index) {
                indices.push(index);
            }
        }
        for (index, entry) in &self.updates {
            if entry.lock().await.is_some() {
                indices.push(index.clone());
            }
        }
        indices.sort();
        Ok(indices)
    }

    /// Restore the view to its initial state.
    ///
    /// If any entry is being editted, awaits until all entry locks are released.
    pub async fn try_rollback(&mut self) {
        for (_, update) in mem::take(&mut self.updates) {
            update.lock().await;
        }
    }
}

/// An active entry inside a [`SharedCollectionView`].
pub struct SharedCollectionEntry<Context, Index, Entry> {
    context: Context,
    index: Index,
    entry: OwnedMutexGuard<Option<Entry>>,
}

impl<Context, Index, Entry> SharedCollectionEntry<Context, Index, Entry>
where
    Context: CollectionOperations<Index>,
    Entry: View<Context>,
    Index: Debug,
{
    /// The index of this entry.
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Commit this entry.
    pub async fn commit(mut self) -> Result<(), Context::Error> {
        dbg!(format!("Committing entry: {:?}", &self.index));
        if let Some(entry) = self.entry.take() {
            entry.commit().await?;
            let entry_context = self.context.clone_with_scope(&self.index);
            self.context.add_index(self.index).await?;
            *self.entry = Some(Entry::load(entry_context).await?);
        } else {
            self.context.remove_index(self.index).await?;
        }
        Ok(())
    }
}

impl<Context, Index, Entry> Deref for SharedCollectionEntry<Context, Index, Entry> {
    type Target = Entry;

    fn deref(&self) -> &Self::Target {
        self.entry
            .as_ref()
            .expect("Created a SharedCollectionEntry for a removed entry")
    }
}

impl<Context, Index, Entry> DerefMut for SharedCollectionEntry<Context, Index, Entry> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.entry
            .as_mut()
            .expect("Created a SharedCollectionEntry for a removed entry")
    }
}
