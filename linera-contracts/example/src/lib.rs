use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

wit_bindgen_guest_rust::export!("../contract.wit");

pub struct Contract;

impl contract::Contract for Contract {
    fn example() -> contract::Poll {
        let future = unsafe { FUTURE.get_or_insert_with(|| Box::pin(future())) };
        let waker = unsafe { Waker::from_raw(WAKER) };
        let mut context = Context::from_waker(&waker);

        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Pending if unsafe { SHOULD_AWAKE } => unsafe { SHOULD_AWAKE = false },
                Poll::Pending => return contract::Poll::Pending,
                Poll::Ready(value) => return contract::Poll::Ready(value),
            }
        }
    }
}

pub async fn future() -> u32 {
    futures::pending!();
    10
}

const WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
const WAKER: RawWaker = RawWaker::new(0 as *const (), &WAKER_VTABLE);

static mut FUTURE: Option<Pin<Box<dyn Future<Output = u32>>>> = None;
static mut SHOULD_AWAKE: bool = false;

fn clone(internal_waker: *const ()) -> RawWaker {
    RawWaker::new(internal_waker, &WAKER_VTABLE)
}

unsafe fn wake(_internal_waker: *const ()) {
    SHOULD_AWAKE = true;
}

unsafe fn wake_by_ref(_internal_waker: *const ()) {
    SHOULD_AWAKE = true;
}

fn drop(_internal_waker: *const ()) {}
