use {
    async_trait::async_trait,
    futures::{channel::oneshot, future::BoxFuture, join},
    linera_sdk::{
        Application, ApplicationResult, BlockHeight, CalleeContext, ChainId, Destination,
        EffectContext, ExportedFuture, OperationContext, QueryContext,
    },
    std::task::Poll,
    thiserror::Error,
    wit_bindgen_rust::Handle,
};

wit_bindgen_rust::export!("../contract.wit");
wit_bindgen_rust::import!("../api.wit");

pub struct Contract;

impl contract::Contract for Contract {}

#[async_trait]
impl Application for Contract {
    type Error = Error;

    async fn apply_operation(
        &mut self,
        context: &OperationContext,
        operation: &[u8],
    ) -> Result<ApplicationResult, Self::Error> {
        todo!();
    }

    async fn apply_effect(
        &mut self,
        context: &EffectContext,
        effect: &[u8],
    ) -> Result<ApplicationResult, Self::Error> {
        todo!();
    }

    async fn call(
        &mut self,
        context: &CalleeContext,
        name: &str,
        argument: &[u8],
    ) -> Result<(Vec<u8>, ApplicationResult), Self::Error> {
        todo!();
    }

    async fn query(
        &self,
        context: &QueryContext,
        name: &str,
        argument: &[u8],
    ) -> Result<Vec<u8>, Self::Error> {
        todo!();
    }
}

pub struct ApplyOperation {
    future: ExportedFuture<Result<ApplicationResult, Error>>,
}

impl contract::ApplyOperation for ApplyOperation {
    fn new(context: contract::OperationContext, operation: Vec<u8>) -> Handle<Self> {
        Handle::new(ApplyOperation {
            future: ExportedFuture::new(async move {
                let mut contract = Contract;
                contract.apply_operation(&context.into(), &operation).await
            }),
        })
    }

    fn poll(&self) -> contract::PollApplicationResult {
        self.future.poll()
    }
}

pub struct ApplyEffect {
    future: ExportedFuture<Result<ApplicationResult, Error>>,
}

impl contract::ApplyEffect for ApplyEffect {
    fn new(context: contract::EffectContext, effect: Vec<u8>) -> Handle<Self> {
        Handle::new(ApplyEffect {
            future: ExportedFuture::new(async move {
                let mut contract = Contract;
                contract.apply_effect(&context.into(), &effect).await
            }),
        })
    }

    fn poll(&self) -> contract::PollApplicationResult {
        self.future.poll()
    }
}

pub struct Call;

impl contract::Call for Call {
    fn new(context: contract::CalleeContext, name: String, argument: Vec<u8>) -> Handle<Self> {
        todo!();
    }

    fn poll(&self) -> contract::PollCall {
        todo!();
    }
}

pub struct Query;

impl contract::Query for Query {
    fn new(context: contract::QueryContext, name: String, argument: Vec<u8>) -> Handle<Self> {
        todo!();
    }

    fn poll(&self) -> contract::PollQuery {
        todo!();
    }
}

#[derive(Debug, Error)]
pub enum Error {}

impl From<contract::OperationContext> for OperationContext {
    fn from(contract_context: contract::OperationContext) -> Self {
        OperationContext {
            chain_id: ChainId::from_bytes_unchecked(&contract_context.chain_id),
            height: BlockHeight(contract_context.height),
            index: contract_context.index,
        }
    }
}

impl From<ApplicationResult> for contract::ApplicationResult {
    fn from(result: ApplicationResult) -> Self {
        let effects = result
            .effects
            .into_iter()
            .map(|(destination, effect)| (destination.into(), effect))
            .collect();

        let subscribe = result
            .subscribe
            .into_iter()
            .map(|(channel_id, chain_id)| (channel_id, chain_id.to_bytes().to_vec()))
            .collect();

        let unsubscribe = result
            .unsubscribe
            .into_iter()
            .map(|(channel_id, chain_id)| (channel_id, chain_id.to_bytes().to_vec()))
            .collect();

        contract::ApplicationResult {
            effects,
            subscribe,
            unsubscribe,
        }
    }
}

impl From<Destination> for contract::Destination {
    fn from(destination: Destination) -> Self {
        match destination {
            Destination::Recipient(chain_id) => {
                contract::Destination::Recipient(chain_id.to_bytes().to_vec())
            }
            Destination::Subscribers(channel_id) => contract::Destination::Subscribers(channel_id),
        }
    }
}

impl From<Poll<Result<ApplicationResult, Error>>> for contract::PollApplicationResult {
    fn from(poll: Poll<Result<ApplicationResult, Error>>) -> Self {
        use contract::PollApplicationResult;
        match poll {
            Poll::Pending => PollApplicationResult::Pending,
            Poll::Ready(Ok(value)) => PollApplicationResult::Ready(Ok(value.into())),
            Poll::Ready(Err(value)) => PollApplicationResult::Ready(Err(value.to_string())),
        }
    }
}

// impl contract::Contract for Contract {
// fn example() -> contract::Poll {
// let future = unsafe { FUTURE.get_or_insert_with(|| Box::pin(future())) };
// let waker = unsafe { Waker::from_raw(WAKER) };
// let mut context = Context::from_waker(&waker);

// loop {
// match future.as_mut().poll(&mut context) {
// Poll::Pending if unsafe { SHOULD_AWAKE } => unsafe { SHOULD_AWAKE = false },
// Poll::Pending => return contract::Poll::Pending,
// Poll::Ready(value) => return contract::Poll::Ready(value),
// }
// }
// }
// }

// pub async fn future() -> u32 {
// let (sender, receiver) = oneshot::channel();

// let sender_task = async move {
// sender.send(10).expect("Receiver task dropped unexpectedly");
// };

// let receiver_task = async move { receiver.await.expect("Sender task stopped without sending") };

// let (value, ()) = join!(receiver_task, sender_task);
// exported(value).await
// }

// fn exported(input: u32) -> api::Exported {
// api::Exported::new(input)
// }

// impl Future for api::Exported {
// type Output = u32;

// fn poll(self: Pin<&mut Self>, _context: &mut Context) -> Poll<Self::Output> {
// match api::Exported::poll(&self) {
// api::Poll::Ready(value) => Poll::Ready(value),
// api::Poll::Pending => Poll::Pending,
// }
// }
// }

// const WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
// const WAKER: RawWaker = RawWaker::new(0 as *const (), &WAKER_VTABLE);

// static mut FUTURE: Option<Pin<Box<dyn Future<Output = u32>>>> = None;
// static mut SHOULD_AWAKE: bool = false;

// fn clone(internal_waker: *const ()) -> RawWaker {
// RawWaker::new(internal_waker, &WAKER_VTABLE)
// }

// unsafe fn wake(_internal_waker: *const ()) {
// SHOULD_AWAKE = true;
// }

// unsafe fn wake_by_ref(_internal_waker: *const ()) {
// SHOULD_AWAKE = true;
// }

// fn drop(_internal_waker: *const ()) {}
