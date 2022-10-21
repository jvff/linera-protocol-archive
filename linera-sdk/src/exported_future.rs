use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

pub struct ExportedFuture<Output> {
    future: RefCell<Pin<Box<dyn Future<Output = Output>>>>,
    should_wake: Arc<AtomicBool>,
}

impl<Output> ExportedFuture<Output> {
    pub fn new(future: impl Future<Output = Output> + 'static) -> Self {
        ExportedFuture {
            future: RefCell::new(Box::pin(future)),
            should_wake: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn poll<CustomPoll>(&self) -> CustomPoll
    where
        CustomPoll: From<Poll<Output>>,
    {
        let should_wake = ShouldWake::new(self.should_wake.clone());
        let waker = should_wake.into_waker();
        let mut context = Context::from_waker(&waker);
        let mut future = self.future.borrow_mut();

        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Pending if self.should_wake.swap(false, Ordering::AcqRel) => continue,
                poll => return CustomPoll::from(poll),
            }
        }
    }
}

#[allow(clippy::redundant_allocation)]
#[derive(Clone)]
struct ShouldWake(Box<Arc<AtomicBool>>);

impl ShouldWake {
    pub fn new(should_wake: Arc<AtomicBool>) -> Self {
        ShouldWake(Box::new(should_wake))
    }

    pub fn into_waker(self) -> Waker {
        let raw_waker = RawWaker::new(unsafe { self.stay_alive() }, &WAKER_VTABLE);
        unsafe { Waker::from_raw(raw_waker) }
    }

    unsafe fn unwrap_from(pointer: *const ()) -> Self {
        let payload = Box::from_raw(pointer as *mut _);
        ShouldWake(payload)
    }

    unsafe fn stay_alive(self) -> *const () {
        Box::leak(self.0) as *const _ as *const ()
    }

    fn wake(&self) {
        self.0.store(true, Ordering::Release);
    }
}

const WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

unsafe fn clone(internal_waker: *const ()) -> RawWaker {
    let should_wake = ShouldWake::unwrap_from(internal_waker);
    let new_internal_waker = should_wake.clone().stay_alive();
    should_wake.stay_alive();
    RawWaker::new(new_internal_waker, &WAKER_VTABLE)
}

unsafe fn wake(internal_waker: *const ()) {
    let should_wake = ShouldWake::unwrap_from(internal_waker);
    should_wake.wake();
}

unsafe fn wake_by_ref(internal_waker: *const ()) {
    let should_wake = ShouldWake::unwrap_from(internal_waker);
    should_wake.wake();
    should_wake.stay_alive();
}

unsafe fn drop(internal_waker: *const ()) {
    let _ = ShouldWake::unwrap_from(internal_waker);
}
