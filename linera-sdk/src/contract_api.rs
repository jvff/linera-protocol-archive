use crate::{Application, ApplicationResult};
use wit_bindgen_rust::Handle;

wit_bindgen_rust::export!("contract.wit");

pub struct Contract;

impl contract::Contract for Contract {}

pub struct ApplyOperation<App>
where
    App: Application,
{
    future: BoxFuture<'static, Result<ApplicationResult, App::Error>>,
}

impl<App> contract::ApplyOperation for ApplyOperation<App>
where
    App: Application,
{
    fn new(context: contract::OperationContext, operation: Vec<u8>) -> Handle<Self> {
        let future = Box::pin(async move {
            let contract = Contract;
            contract.apply_operation(context, operation).await
        });
        ApplyOperation { future }
    }

    fn poll(&self) -> contract::PollRawApplicationResult {
        todo!();
    }
}

pub struct ApplyEffect;

impl contract::ApplyEffect for ApplyEffect {
    fn new(context: contract::EffectContext, effect: Vec<u8>) -> Handle<Self> {
        todo!();
    }

    fn poll(&self) -> contract::PollRawApplicationResult {
        todo!();
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
