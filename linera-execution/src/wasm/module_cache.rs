// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A cache of compiled WebAssembly modules.
//!
//! The cache is limited by the total size of cached bytecodes. Note that this is a heuristic to
//! estimate the total memory usage by the cache, since it's currently not possible to determine
//! the size of a generic `Module`.

use crate::{wasm::WasmExecutionError, Bytecode};
use std::{cmp::Ordering, collections::HashMap, num::NonZeroU64, sync::Arc};

const DEFAULT_MAX_CACHE_SIZE: u64 = 512 * 1024 * 1024;

/// A cache of compiled WebAssembly modules.
///
/// The cache prioritizes entries based on their [`Metadata`].
pub struct ModuleCache<Module> {
    modules: HashMap<Bytecode, Entry<Module>>,
    access_clock: u64,
    total_size: u64,
    max_size: u64,
}

/// An entry in the [`ModuleCache`].
///
/// Contains the `Module` iteslf and some extra metadata based on its usage.
pub struct Entry<Module> {
    module: Arc<Module>,
    last_access: u64,
    access_count: Option<NonZeroU64>,
}

/// Information on a cache entry used to find eviction candidates.
///
/// Entries most recently used are prioritized over entries least recently used. Entries that are
/// more accessed are prioritized than entries that are less accessed. Entries for smaller
/// bytecodes are prioritized over entries for smaller bytecodes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Metadata {
    last_access: u64,
    access_count: Option<NonZeroU64>,
    bytecode_size: u64,
}

impl<Module> Default for ModuleCache<Module> {
    fn default() -> Self {
        ModuleCache {
            modules: HashMap::new(),
            access_clock: 0,
            total_size: 0,
            max_size: DEFAULT_MAX_CACHE_SIZE,
        }
    }
}

impl<Module> ModuleCache<Module> {
    /// Returns a `Module` for the requested `bytecode`, creating it with `module_builder` and
    /// adding it to the cache if it doesn't already exist in the cache.
    pub fn get_or_insert_with<E>(
        &mut self,
        bytecode: Bytecode,
        module_builder: impl FnOnce(Bytecode) -> Result<Module, E>,
    ) -> Result<Arc<Module>, WasmExecutionError>
    where
        WasmExecutionError: From<E>,
    {
        if let Some(module) = self.get(&bytecode) {
            Ok(module)
        } else {
            let module = Arc::new(module_builder(bytecode.clone())?);
            self.insert(bytecode, module.clone());
            Ok(module)
        }
    }

    /// Returns a `Module` for the requested `bytecode` if it's in the cache.
    pub fn get(&mut self, bytecode: &Bytecode) -> Option<Arc<Module>> {
        if !self.modules.contains_key(bytecode) {
            return None;
        }

        let current_access_time = self.tick_access_clock();
        let entry = self
            .modules
            .get_mut(bytecode)
            .expect("Missing value that was checked to be present");

        if let Some(access_count) = entry.access_count.as_mut() {
            *access_count = access_count.saturating_add(1);
        }

        entry.last_access = current_access_time;

        Some(entry.module.clone())
    }

    /// Inserts a `bytecode` and its compiled `module` in the cache.
    pub fn insert(&mut self, bytecode: Bytecode, module: Arc<Module>) {
        let bytecode_size = bytecode.as_ref().len() as u64;

        if self.total_size + bytecode_size > self.max_size {
            self.reduce_size_to(self.max_size - bytecode_size);
        }

        let entry = Entry {
            module,
            last_access: self.tick_access_clock(),
            access_count: NonZeroU64::new(1),
        };

        self.modules.insert(bytecode, entry);
    }

    /// Marks entries from the cache to be evicted so that the total size of cached bytecodes
    /// afterwards is less than `new_size`.
    fn reduce_size_to(&mut self, new_size: u64) {
        let mut eviction_candidates = self.modules.iter_mut().collect::<Vec<_>>();
        eviction_candidates.sort_unstable_by_key(|candidate| Metadata::from(candidate));

        while self.total_size > new_size {
            let (bytecode_to_remove, entry_to_remove) = eviction_candidates
                .pop()
                .expect("Removed all entries and still failed to clear enough space");
            let bytecode_size: u64 = bytecode_to_remove.as_ref().len() as u64;

            entry_to_remove.access_count = None;
            self.total_size -= bytecode_size;
        }
    }

    /// Increments the logical clock used to keep track of cache entry hits.
    fn tick_access_clock(&mut self) -> u64 {
        if self.access_clock == u64::MAX {
            self.reset_access_clock();
            assert!(
                self.access_clock < u64::MAX,
                "Module cache should never have `u64::MAX` entries"
            );
        }

        self.access_clock += 1;
        self.access_clock - 1
    }

    /// Resets the logical clock used to keep track of cache entry hits, to control how it
    /// overflows.
    ///
    /// All entries in the cache have their access time reset to lower values so that they become
    /// compacted, and the clock is reset to the next value after the latest access time.
    fn reset_access_clock(&mut self) {
        self.access_clock = self.modules.len() as u64;
        if self.access_clock == u64::MAX {
            self.evict_one();
            self.access_clock -= 1;
        }

        let mut entries = self.modules.values_mut().collect::<Vec<_>>();
        entries.sort_unstable_by_key(|entry| entry.last_access);

        for (new_last_access, mut entry) in (0_u64..).zip(entries) {
            entry.last_access = new_last_access;
        }
    }

    /// Evicts a single entry from the cache.
    fn evict_one(&mut self) {
        self.mark_entry_to_evict_one();
        self.remove_marked_for_eviction();
    }

    /// Searches for the next entry to be evicted and marks it to be removed.
    fn mark_entry_to_evict_one(&mut self) {
        let maybe_entry_to_remove = self.modules.iter_mut().fold(
            (Metadata::MAX, None),
            |(worst_metadata, best_candidate), candidate| {
                let candidate_metadata = Metadata::from(&candidate);

                if candidate_metadata < worst_metadata {
                    (candidate_metadata, Some(candidate.1))
                } else {
                    (worst_metadata, best_candidate)
                }
            },
        );

        let (metadata, Some(entry_to_remove)) = maybe_entry_to_remove
            else { panic!("`evict_one` must only be called when cache is not empty") };

        entry_to_remove.access_count = None;
        self.total_size -= metadata.bytecode_size;
    }

    /// Removes entries marked to be evicted.
    fn remove_marked_for_eviction(&mut self) {
        self.modules
            .retain(|_bytecode, entry| entry.access_count.is_some());
    }
}

impl<Module> From<&(&Bytecode, &mut Entry<Module>)> for Metadata {
    fn from((bytecode, entry): &(&Bytecode, &mut Entry<Module>)) -> Self {
        Metadata {
            last_access: entry.last_access,
            access_count: entry.access_count,
            bytecode_size: bytecode.as_ref().len() as u64,
        }
    }
}

impl Metadata {
    /// The worst possible access metadata.
    const MAX: Self = Metadata {
        last_access: 0,
        access_count: NonZeroU64::new(1),
        bytecode_size: u64::MAX,
    };
}

impl Ord for Metadata {
    fn cmp(&self, other: &Self) -> Ordering {
        self.last_access
            .cmp(&other.last_access)
            .then(self.access_count.cmp(&other.access_count))
            .then(self.bytecode_size.cmp(&other.bytecode_size).reverse())
    }
}

impl PartialOrd for Metadata {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
