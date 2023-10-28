use futures::lock::{Mutex, OwnedMutexGuard};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};
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
        let lock_future = TraceWaker {
            name: self.name.clone(),
            future: Box::pin(self.lock.clone().lock_owned()),
        };
        let guard = lock_future.await;
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

pub struct TraceWaker<F> {
    name: Arc<str>,
    future: Pin<Box<F>>,
}

impl<F> Future for TraceWaker<F>
where
    F: Future,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();

        let data = Box::leak(Box::new(TraceWakerData {
            name: this.name.clone(),
            original_waker: context.waker().clone(),
        }));

        let trace_waker = unsafe {
            Waker::from_raw(RawWaker::new(
                data as *const _ as *const (),
                &TRACE_WAKER_VTABLE,
            ))
        };

        this.future
            .as_mut()
            .poll(&mut Context::from_waker(&trace_waker))
    }
}

#[derive(Clone)]
pub struct TraceWakerData {
    name: Arc<str>,
    original_waker: Waker,
}

const TRACE_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    trace_waker_clone,
    trace_waker_wake,
    trace_waker_wake_by_ref,
    trace_waker_drop,
);

fn trace_waker_clone(raw_data: *const ()) -> RawWaker {
    let data = unsafe { &*(raw_data as *const TraceWakerData) };
    let new_data = Box::leak(Box::new(data.clone()));

    RawWaker::new(new_data as *const _ as *const (), &TRACE_WAKER_VTABLE)
}

fn trace_waker_wake(raw_data: *const ()) {
    trace_waker_wake_by_ref(raw_data);
    trace_waker_drop(raw_data);
}

fn trace_waker_wake_by_ref(raw_data: *const ()) {
    let data = raw_data as *const TraceWakerData;

    tracing::trace!(name = %unsafe { &(*data).name }, "Waking future");
    unsafe { (*data).original_waker.wake_by_ref() }
}

fn trace_waker_drop(raw_data: *const ()) {
    let _data = unsafe { Box::from_raw(raw_data as *mut TraceWakerData) };
}
