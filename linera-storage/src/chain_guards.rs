// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module contains some helper types to prevent concurrent access to the same chain data.
//!
//! The [`ChainGuards`] type controls the active guards. It can be cheaply cloned and shared
//! between threads.
//!
//! The [`ChainGuard`] type is used to guard a single chain. There is always a single live instance
//! for a given chain, and new instances for the same chain can only be created when the previous
//! instance is dropped.

use dashmap::DashMap;
use linera_base::{
    identifiers::ChainId,
    locks::{AsyncMutex, OwnedAsyncMutexGuard, WeakAsyncMutex},
};
use std::{
    fmt::{self, Debug, Formatter},
    sync::Arc,
};

#[cfg(test)]
#[path = "unit_tests/chain_guards.rs"]
mod unit_tests;

/// The internal map type.
///
/// Every chain ID is mapped to a weak reference to an asynchronous [`AsyncMutex`].
///
/// Attempting to upgrade the weak reference allows checking if there's a live guard for that chain.
type ChainGuardMap = DashMap<ChainId, WeakAsyncMutex<()>>;

/// Manager of [`ChainGuard`]s for chains.
///
/// The [`ChainGuard::guard`] method can be used to obtain a guard for a specific chain. The guard
/// is always guaranteed to be the only live guard for that chain.
#[derive(Clone, Debug, Default)]
pub struct ChainGuards {
    guards: Arc<ChainGuardMap>,
}

impl ChainGuards {
    /// Obtains a guard for a specified chain, waiting if there's already a live guard.
    ///
    /// Only one guard can be active for a chain, so if there's already a guard for the requested
    /// chain, this method will wait until it is able to obtain the guard.
    ///
    /// The lock used for the guard is stored in a shared [`ChainGuardMap`]. A weak reference is
    /// stored, because the goal is to remove the map entry as soon as possible, and the weak
    /// reference can only be upgraded if there's another attempt waiting to create a guard for
    /// the same chain.
    pub async fn guard(&self, chain_id: ChainId) -> ChainGuard {
        let guard = self.get_or_create_lock(chain_id);
        ChainGuard {
            chain_id,
            guards: self.guards.clone(),
            guard: Some(guard.lock_owned().await),
        }
    }

    /// Obtains the lock used for guarding a chain.
    ///
    /// When obtaining a lock, first a write lock to the map entry is obtained. If there is no
    /// entry, a new lock for that entry is created.
    ///
    /// Then, an attempt is made to upgrade the weak reference into a strong reference. If that
    /// succeeds, there's already a live guard for that chain, and that strong reference to the lock
    /// can be returned to wait until it's possible to create the guard.
    ///
    /// If upgrading the weak reference fails, then there is no more live guards, but the entry
    /// hasn't been removed yet. A new lock must be created and inserted in the entry.
    ///
    /// It's important that the returned lock is only locked after the write lock for the map entry
    /// is released at the end of this method, to avoid deadlocks. See [`ChainGuard::drop`] for
    /// more details.
    fn get_or_create_lock(&self, chain_id: ChainId) -> AsyncMutex<()> {
        let mut new_guard_holder = None;
        let mut guard_reference = self.guards.entry(chain_id).or_insert_with(|| {
            let (new_guard, weak_reference) = Self::create_new_mutex(chain_id);
            new_guard_holder = Some(new_guard);
            weak_reference
        });
        guard_reference.upgrade().unwrap_or_else(|| {
            let (new_guard, weak_reference) = Self::create_new_mutex(chain_id);
            *guard_reference = weak_reference;
            new_guard
        })
    }

    /// Creates a new [`AsyncMutex`], returning both a strong reference and a weak reference to it.
    fn create_new_mutex(chain_id: ChainId) -> (AsyncMutex<()>, WeakAsyncMutex<()>) {
        let new_guard = AsyncMutex::new(format!("ChainGuard for {chain_id}"), ());
        let weak_reference = new_guard.downgrade();
        (new_guard, weak_reference)
    }

    /// Obtains the current number of active guards.
    #[cfg(test)]
    pub(crate) fn active_guards(&self) -> usize {
        self.guards.len()
    }
}

/// A guard for a specific chain.
///
/// Only one instance for a chain is guaranteed to exist at any given moment.
///
/// When the instance is dropped, the lock is released and a new instance can be created. If no new
/// instances are waiting to be created, the entry in the map is removed.
pub struct ChainGuard {
    chain_id: ChainId,
    guards: Arc<ChainGuardMap>,
    guard: Option<OwnedAsyncMutexGuard<()>>,
}

impl Drop for ChainGuard {
    /// Releases the lock and removes the entry from the map if possible.
    ///
    /// Only removes the entry from the map if there are no active attempts to acquire the lock.
    /// This is checked through the number of strong references to the lock, since every attempt to
    /// lock the guard uses a strong reference to the underlying lock.
    fn drop(&mut self) {
        self.guards.remove_if(&self.chain_id, |_, weak_reference| {
            // The mutex is unlocked inside `remove_if` to avoid a race condition with
            // `get_or_create_lock`. Both `remove_if` here and `entry` in `get_or_create_lock` will
            // acquire a write lock to the map, so only one of them will fully execute at any given
            // moment.
            self.guard.take();
            weak_reference.no_longer_upgradable()
        });
    }
}

impl Debug for ChainGuard {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .debug_struct("ChainGuard")
            .field("chain_id", &self.chain_id)
            .finish_non_exhaustive()
    }
}
