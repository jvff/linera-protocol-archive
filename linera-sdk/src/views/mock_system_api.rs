// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A mock system API for interfacing with the key-value store.

#![allow(static_mut_refs)]

use std::collections::HashMap;

use linera_base::sync::Lazy;
use linera_views::batch::{Batch, WriteOperation};

/// The storage contents.
static mut STORE: Lazy<HashMap<Vec<u8>, Vec<u8>>> = Lazy::new(HashMap::new);

/// Helper type to keep track of created promises by one of the functions.
#[derive(Default)]
struct PromiseRegistry<T> {
    promises: HashMap<u32, T>,
    id_counter: u32,
}

impl<T> PromiseRegistry<T> {
    /// Creates a new promise tracking the internal `value`.
    pub fn register(&mut self, value: T) -> u32 {
        let id = self.id_counter;
        self.id_counter += 1;
        self.promises.insert(id, value);
        id
    }

    /// Retrieves a tracked promise by its ID.
    pub fn take(&mut self, id: u32) -> T {
        self.promises
            .remove(&id)
            .expect("Use of an invalid promise ID")
    }
}

/// Promises tracked for the `contains_key` API.
static mut CONTAINS_KEY_PROMISES: Lazy<PromiseRegistry<bool>> = Lazy::new(PromiseRegistry::default);

/// Checks if `key` is present in the storage, returning a promise to retrieve the final
/// value.
pub(crate) fn contains_key_new(key: &[u8]) -> u32 {
    unsafe { Lazy::force(&CONTAINS_KEY_PROMISES) };
    unsafe { Lazy::get_mut(&mut CONTAINS_KEY_PROMISES) }
        .expect("`Lazy::force` should initialize it")
        .register(unsafe { STORE.contains_key(key) })
}

/// Returns if the key used in the respective call to [`contains_key_new`] is present.
pub(crate) fn contains_key_wait(promise: u32) -> bool {
    unsafe { Lazy::get_mut(&mut CONTAINS_KEY_PROMISES) }
        .expect("Registry should be initialized when promise is created")
        .take(promise)
}

/// Promises tracked for the `read_multi_values_bytes` API.
static mut READ_MULTI_PROMISES: Lazy<PromiseRegistry<Vec<Option<Vec<u8>>>>> =
    Lazy::new(PromiseRegistry::default);

/// Reads the values addressed by `keys` from the store, returning a promise to retrieve
/// the final value.
pub(crate) fn read_multi_values_bytes_new(keys: &[Vec<u8>]) -> u32 {
    unsafe { Lazy::force(&READ_MULTI_PROMISES) };
    unsafe { Lazy::get_mut(&mut READ_MULTI_PROMISES) }
        .expect("`Lazy::force` should initialize it")
        .register(
            keys.iter()
                .map(|key| unsafe { STORE.get(key) }.cloned())
                .collect(),
        )
}

/// Returns the values read from storage by the respective
/// [`read_multi_values_bytes_new`] call.
pub(crate) fn read_multi_values_bytes_wait(promise: u32) -> Vec<Option<Vec<u8>>> {
    unsafe { Lazy::get_mut(&mut READ_MULTI_PROMISES) }
        .expect("Registry should be initialized when promise is created")
        .take(promise)
}

/// Promises tracked for the `read_multi_values_bytes` API.
static mut READ_SINGLE_PROMISES: Lazy<PromiseRegistry<Option<Vec<u8>>>> =
    Lazy::new(PromiseRegistry::default);

/// Reads a value addressed by `key` from the storage, returning a promise to retrieve the
/// final value.
pub(crate) fn read_value_bytes_new(key: &[u8]) -> u32 {
    unsafe { Lazy::force(&READ_SINGLE_PROMISES) };
    unsafe { Lazy::get_mut(&mut READ_SINGLE_PROMISES) }
        .expect("`Lazy::force` should initialize it")
        .register(unsafe { STORE.get(key) }.cloned())
}

/// Returns the value read from storage by the respective [`read_value_bytes_new`] call.
pub(crate) fn read_value_bytes_wait(promise: u32) -> Option<Vec<u8>> {
    unsafe { Lazy::get_mut(&mut READ_SINGLE_PROMISES) }
        .expect("Registry should be initialized when promise is created")
        .take(promise)
}

/// Promises tracked for the `read_multi_values_bytes` API.
static mut FIND_KEYS_PROMISES: Lazy<PromiseRegistry<Vec<Vec<u8>>>> =
    Lazy::new(PromiseRegistry::default);

/// Finds keys in the storage that start with `key_prefix`, returning a promise to
/// retrieve the final value.
pub(crate) fn find_keys_new(key_prefix: &[u8]) -> u32 {
    unsafe { Lazy::force(&FIND_KEYS_PROMISES) };
    unsafe { Lazy::get_mut(&mut FIND_KEYS_PROMISES) }
        .expect("`Lazy::force` should initialize it")
        .register(
            unsafe { STORE.keys() }
                .filter(|key| key.starts_with(key_prefix))
                .cloned()
                .collect(),
        )
}

/// Returns the keys found in storage by the respective [`find_keys_new`] call.
pub(crate) fn find_keys_wait(promise: u32) -> Vec<Vec<u8>> {
    unsafe { Lazy::get_mut(&mut FIND_KEYS_PROMISES) }
        .expect("Registry should be initialized when promise is created")
        .take(promise)
}

/// Promises tracked for the `read_multi_values_bytes` API.
#[allow(clippy::type_complexity)]
static mut FIND_KEY_VALUES_PROMISES: Lazy<PromiseRegistry<Vec<(Vec<u8>, Vec<u8>)>>> =
    Lazy::new(PromiseRegistry::default);

/// Finds key-value pairs in the storage in which the key starts with `key_prefix`,
/// returning a promise to retrieve the final value.
pub(crate) fn find_key_values_new(key_prefix: &[u8]) -> u32 {
    unsafe { Lazy::force(&FIND_KEY_VALUES_PROMISES) };
    unsafe { Lazy::get_mut(&mut FIND_KEY_VALUES_PROMISES) }
        .expect("`Lazy::force` should initialize it")
        .register(
            unsafe { STORE.iter() }
                .filter(|(key, _)| key.starts_with(key_prefix))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        )
}

/// Returns the key-value pairs found in storage by the respective [`find_key_values_new`]
/// call.
pub(crate) fn find_key_values_wait(promise: u32) -> Vec<(Vec<u8>, Vec<u8>)> {
    unsafe { Lazy::get_mut(&mut FIND_KEY_VALUES_PROMISES) }
        .expect("Registry should be initialized when promise is created")
        .take(promise)
}

/// Writes a `batch` of operations to storage.
pub(crate) fn write_batch(batch: Batch) {
    unsafe { Lazy::force(&STORE) };
    let store =
        unsafe { Lazy::get_mut(&mut STORE) }.expect("`Lazy::force` should initialize the store");

    for operation in batch.operations {
        match operation {
            WriteOperation::Delete { key } => {
                store.remove(&key);
            }
            WriteOperation::DeletePrefix { key_prefix } => {
                store.retain(|key, _| key.starts_with(&key_prefix));
            }
            WriteOperation::Put { key, value } => {
                store.insert(key, value);
            }
        }
    }
}
