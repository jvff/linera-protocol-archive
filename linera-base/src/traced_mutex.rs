use futures::lock::{Mutex, OwnedMutexGuard};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub struct TracedMutex<T> {
    name: Arc<str>,
    lock: Arc<Mutex<T>>,
}

impl<T> TracedMutex<T> {
    pub fn new(name: impl Into<String>, data: T) -> Self {
        TracedMutex {
            name: name.into().into(),
            lock: Arc::new(Mutex::new(data)),
        }
    }

    pub async fn lock(&self) -> TracedMutexGuard<T> {
        tracing::trace!(name = %self.name, "Locking");
        let guard = self.lock.clone().lock_owned().await;
        tracing::trace!(name = %self.name, "Locked");
        TracedMutexGuard {
            name: self.name.clone(),
            guard,
        }
    }
}

impl<T> Clone for TracedMutex<T> {
    fn clone(&self) -> Self {
        TracedMutex {
            name: self.name.clone(),
            lock: self.lock.clone(),
        }
    }
}

pub struct TracedMutexGuard<T> {
    name: Arc<str>,
    guard: OwnedMutexGuard<T>,
}

impl<T> Drop for TracedMutexGuard<T> {
    fn drop(&mut self) {
        tracing::trace!(name = %self.name, "Unlocking");
    }
}

impl<T> Deref for TracedMutexGuard<T> {
    type Target = OwnedMutexGuard<T>;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<T> DerefMut for TracedMutexGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}
