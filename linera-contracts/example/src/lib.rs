use {
    futures::{channel::oneshot, join},
    std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    },
};

wit_bindgen_rust::export!("../contract.wit");
wit_bindgen_rust::import!("../api.wit");

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
    let (sender, receiver) = oneshot::channel();

    let sender_task = async move {
        sender.send(10).expect("Receiver task dropped unexpectedly");
    };

    let receiver_task = async move { receiver.await.expect("Sender task stopped without sending") };

    let (value, ()) = join!(receiver_task, sender_task);
    exported(value).await
}

fn exported(input: u32) -> api::Exported {
    api::Exported::new(input)
}

impl Future for api::Exported {
    type Output = u32;

    fn poll(self: Pin<&mut Self>, _context: &mut Context) -> Poll<Self::Output> {
        match api::Exported::poll(&self) {
            api::Poll::Ready(value) => Poll::Ready(value),
            api::Poll::Pending => Poll::Pending,
        }
    }
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
