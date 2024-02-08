// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
#[path = "unit_tests/shared_view.rs"]
mod tests;

use crate::{
    batch::Batch,
    common::Context,
    views::{RootView, SharableView, View, ViewError},
};
use async_lock::{Mutex, MutexGuardArc, RwLock, RwLockReadGuardArc, RwLockWriteGuardArc};
use async_trait::async_trait;
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[cfg(not(target_arch = "wasm32"))]
use crate::{increment_counter, SAVE_VIEW_COUNTER};

/// A way to safely share a [`View`] among multiple readers and at most one writer.
///
/// [`View`]s represent some data persisted in storage, but it also contains some state in
/// memory that caches the storage state and that queues changes to the persisted state to
/// be sent later. This means that two views referencing the same data in storage may have
/// state conflicts in memory, and that's why they can't be trivially shared (using
/// [`Clone`] for example).
///
/// The [`SharedView`] provides a way to share an inner [`View`] more safely, by ensuring
/// that only one writer is staging changes to the view, and than when it is writing those
/// changes to storage there aren't any more readers for the same view which would have
/// their internal state become invalid. The readers are not able to see the changes the
/// writer is staging, and the writer can only save its staged changes after all readers
/// have finished.
pub struct SharedView<C, V> {
    shared_view: Arc<RwLock<V>>,
    staging_view: Arc<Mutex<V>>,
    dirty: Arc<AtomicBool>,
    _context: PhantomData<C>,
}

impl<C, V> SharedView<C, V>
where
    V: SharableView<C>,
{
    /// Wraps a `view` in a [`SharedView`].
    pub fn new(mut view: V) -> Result<Self, ViewError> {
        Ok(SharedView {
            shared_view: Arc::new(RwLock::new(view.share_unchecked()?)),
            staging_view: Arc::new(Mutex::new(view)),
            dirty: Arc::new(AtomicBool::new(false)),
            _context: PhantomData,
        })
    }

    /// Returns a [`ReadOnlyViewReference`] to the inner [`View`].
    ///
    /// If there is a writer with a [`ReadWriteViewReference`] to the inner [`View`], waits
    /// until that writer is finished.
    pub async fn inner(&self) -> Result<ReadOnlyViewReference<V>, ViewError> {
        let mut staging_view = self.staging_view.lock().await;

        let view = if self.dirty.load(Ordering::Acquire) {
            let mut shared_view = self.shared_view.write_arc().await;

            if self.dirty.swap(false, Ordering::AcqRel) {
                *shared_view = staging_view.share_unchecked()?;
            }

            RwLockWriteGuardArc::downgrade(shared_view)
        } else {
            self.shared_view.read_arc().await
        };

        Ok(ReadOnlyViewReference { view })
    }

    /// Returns a [`ReadWriteViewReference`] to the inner [`View`].
    ///
    /// Waits until the previous writer is finished if there is one. There can only be one
    /// [`ReadWriteViewReference`] to the same inner [`View`].
    pub async fn inner_mut(&mut self) -> ReadWriteViewReference<V> {
        let staging_view = self.staging_view.lock_arc().await;

        self.dirty.store(true, Ordering::Release);

        ReadWriteViewReference {
            staging_view,
            shared_view: Arc::clone(&self.shared_view),
        }
    }
}

/// A read-only reference to a [`SharedView`].
pub struct ReadOnlyViewReference<V> {
    view: RwLockReadGuardArc<V>,
}

impl<V> Deref for ReadOnlyViewReference<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

/// A read-write reference to a [`SharedView`].
pub struct ReadWriteViewReference<V> {
    shared_view: Arc<RwLock<V>>,
    staging_view: MutexGuardArc<V>,
}

impl<V> Deref for ReadWriteViewReference<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.staging_view
    }
}

impl<V> DerefMut for ReadWriteViewReference<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.staging_view
    }
}

#[async_trait]
impl<C, V> View<C> for ReadWriteViewReference<V>
where
    C: Send + 'static,
    V: View<C>,
{
    fn context(&self) -> &C {
        self.deref().context()
    }

    async fn load(_context: C) -> Result<Self, ViewError> {
        unreachable!("`ReadWriteViewReference` should not be loaded directly");
    }

    fn rollback(&mut self) {
        self.deref_mut().rollback();
    }

    fn clear(&mut self) {
        self.deref_mut().clear();
    }

    fn flush(&mut self, batch: &mut Batch) -> Result<(), ViewError> {
        self.deref_mut().flush(batch)
    }
}

#[async_trait]
impl<C, V> RootView<C> for ReadWriteViewReference<V>
where
    C: Context + Send + 'static,
    V: SharableView<C> + Send + Sync,
    ViewError: From<C::Error>,
{
    async fn save(&mut self) -> Result<(), ViewError> {
        let mut shared_view = self.shared_view.write().await;

        #[cfg(not(target_arch = "wasm32"))]
        increment_counter(
            &SAVE_VIEW_COUNTER,
            "SharedView",
            &self.staging_view.context().base_key(),
        );

        let mut batch = Batch::new();
        self.staging_view.flush(&mut batch)?;
        self.staging_view.context().write_batch(batch).await?;

        *shared_view = self.staging_view.share_unchecked()?;

        Ok(())
    }
}
