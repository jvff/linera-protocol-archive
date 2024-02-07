// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
#[path = "unit_tests/shared_view.rs"]
mod tests;

use crate::{
    batch::Batch,
    common::Context,
    views::{RootView, View, ViewError},
};
use async_lock::{Mutex, MutexGuardArc, RwLock, RwLockReadGuardArc};
use async_trait::async_trait;
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
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
    view: V,
    save_lock: Arc<RwLock<()>>,
    writer_active: Arc<Mutex<()>>,
    _context: PhantomData<C>,
}

impl<C, V> SharedView<C, V>
where
    V: View<C>,
{
    /// Wraps a `view` in a [`SharedView`].
    pub fn new(view: V) -> Self {
        SharedView {
            view,
            save_lock: Arc::new(RwLock::new(())),
            writer_active: Arc::new(Mutex::new(())),
            _context: PhantomData,
        }
    }

    /// Returns a [`ReadOnlyViewReference`] to the inner [`View`].
    ///
    /// If there is a writer with a [`ReadWriteViewReference`] to the inner [`View`], waits
    /// until that writer is finished.
    pub async fn inner(&mut self) -> Result<ReadOnlyViewReference<V>, ViewError> {
        let _no_writer_check = self.writer_active.lock().await;
        let read_lock = self.save_lock.read_arc().await;

        Ok(ReadOnlyViewReference {
            view: self.view.share_unchecked()?,
            _read_lock: read_lock,
        })
    }

    /// Returns a [`ReadWriteViewReference`] to the inner [`View`].
    ///
    /// Waits until the previous writer is finished if there is one. There can only be one
    /// [`ReadWriteViewReference`] to the same inner [`View`].
    pub async fn inner_mut(&mut self) -> Result<ReadWriteViewReference<V>, ViewError> {
        let writer_guard = self.writer_active.lock_arc().await;

        Ok(ReadWriteViewReference {
            view: self.view.share_unchecked()?,
            save_lock: self.save_lock.clone(),
            _writer_guard: writer_guard,
        })
    }
}

/// A read-only reference to a [`SharedView`].
pub struct ReadOnlyViewReference<V> {
    view: V,
    _read_lock: RwLockReadGuardArc<()>,
}

impl<V> Deref for ReadOnlyViewReference<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

/// A read-write reference to a [`SharedView`].
pub struct ReadWriteViewReference<V> {
    view: V,
    save_lock: Arc<RwLock<()>>,
    _writer_guard: MutexGuardArc<()>,
}

impl<V> Deref for ReadWriteViewReference<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl<V> DerefMut for ReadWriteViewReference<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.view
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

    fn share_unchecked(&mut self) -> Result<Self, ViewError> {
        unreachable!(
            "`ReadWriteViewReference` should not be shared without going through its parent \
            `SharedView`"
        );
    }
}

#[async_trait]
impl<C, V> RootView<C> for ReadWriteViewReference<V>
where
    C: Context + Send + 'static,
    V: View<C> + Send,
    ViewError: From<C::Error>,
{
    async fn save(&mut self) -> Result<(), ViewError> {
        let _save_guard = self.save_lock.write().await;

        #[cfg(not(target_arch = "wasm32"))]
        increment_counter(
            &SAVE_VIEW_COUNTER,
            "SharedView",
            &self.view.context().base_key(),
        );

        let mut batch = Batch::new();
        self.view.flush(&mut batch)?;
        self.view.context().write_batch(batch).await?;
        Ok(())
    }
}
