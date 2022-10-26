use {
    async_trait::async_trait,
    futures::{channel::oneshot, future, join},
    linera_sdk::{
        Application, ApplicationId, ApplicationResult, BlockHeight, CalleeContext, ChainId,
        Destination, EffectContext, EffectId, ExportedFuture, OperationContext, QueryContext,
    },
    serde::{Deserialize, Serialize},
    std::task::Poll,
    thiserror::Error,
    wit_bindgen_rust::Handle,
};

wit_bindgen_rust::export!("../contract.wit");
wit_bindgen_rust::import!("../api.wit");

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct Contract {
    balance: u128,
}

impl contract::Contract for Contract {}

impl Contract {
    async fn load() -> Self {
        let future = api::Get::new();
        let load_result: Result<Vec<u8>, String> =
            future::poll_fn(|_context| future.poll().into()).await;
        let bytes = load_result.expect("Failed to load application state");
        if bytes.is_empty() {
            Self::default()
        } else {
            bcs::from_bytes(&bytes).expect("Invalid contract state")
        }
    }

    async fn store(self) {
        api::set(&bcs::to_bytes(&self).expect("State serialization failed"));
    }
}

#[async_trait]
impl Application for Contract {
    type Error = Error;

    async fn apply_operation(
        &mut self,
        context: &OperationContext,
        operation: &[u8],
    ) -> Result<ApplicationResult, Self::Error> {
        self.balance += 1;
        Ok(ApplicationResult {
            effects: vec![],
            subscribe: vec![],
            unsubscribe: vec![],
        })
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
                let mut contract = Contract::load().await;
                let result = contract.apply_operation(&context.into(), &operation).await;
                if result.is_ok() {
                    contract.store().await;
                }
                result
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
                let mut contract = Contract::load().await;
                let result = contract.apply_effect(&context.into(), &effect).await;
                if result.is_ok() {
                    contract.store().await;
                }
                result
            }),
        })
    }

    fn poll(&self) -> contract::PollApplicationResult {
        self.future.poll()
    }
}

pub struct Call {
    future: ExportedFuture<Result<(Vec<u8>, ApplicationResult), Error>>,
}

impl contract::Call for Call {
    fn new(context: contract::CalleeContext, name: String, argument: Vec<u8>) -> Handle<Self> {
        Handle::new(Call {
            future: ExportedFuture::new(async move {
                let mut contract = Contract::load().await;
                let result = contract.call(&context.into(), &name, &argument).await;
                if result.is_ok() {
                    contract.store().await;
                }
                result
            }),
        })
    }

    fn poll(&self) -> contract::PollCall {
        self.future.poll()
    }
}

pub struct Query {
    future: ExportedFuture<Result<Vec<u8>, Error>>,
}

impl contract::Query for Query {
    fn new(context: contract::QueryContext, name: String, argument: Vec<u8>) -> Handle<Self> {
        Handle::new(Query {
            future: ExportedFuture::new(async move {
                let contract = Contract::load().await;
                contract.query(&context.into(), &name, &argument).await
            }),
        })
    }

    fn poll(&self) -> contract::PollQuery {
        self.future.poll()
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

impl From<contract::EffectContext> for EffectContext {
    fn from(contract_context: contract::EffectContext) -> Self {
        EffectContext {
            chain_id: ChainId::from_bytes_unchecked(&contract_context.chain_id),
            height: BlockHeight(contract_context.height),
            effect_id: contract_context.effect_id.into(),
        }
    }
}

impl From<contract::EffectId> for EffectId {
    fn from(effect_id: contract::EffectId) -> Self {
        EffectId {
            chain_id: ChainId::from_bytes_unchecked(&effect_id.chain_id),
            height: BlockHeight(effect_id.height),
            index: effect_id.index,
        }
    }
}

impl From<contract::CalleeContext> for CalleeContext {
    fn from(contract_context: contract::CalleeContext) -> Self {
        CalleeContext {
            chain_id: ChainId::from_bytes_unchecked(&contract_context.chain_id),
            authenticated_caller_id: contract_context.authenticated_caller_id.map(ApplicationId),
        }
    }
}

impl From<contract::QueryContext> for QueryContext {
    fn from(contract_context: contract::QueryContext) -> Self {
        QueryContext {
            chain_id: ChainId::from_bytes_unchecked(&contract_context.chain_id),
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

impl From<Poll<Result<(Vec<u8>, ApplicationResult), Error>>> for contract::PollCall {
    fn from(poll: Poll<Result<(Vec<u8>, ApplicationResult), Error>>) -> Self {
        use contract::PollCall;
        match poll {
            Poll::Pending => PollCall::Pending,
            Poll::Ready(Ok((response, result))) => PollCall::Ready(Ok((response, result.into()))),
            Poll::Ready(Err(value)) => PollCall::Ready(Err(value.to_string())),
        }
    }
}

impl From<Poll<Result<Vec<u8>, Error>>> for contract::PollQuery {
    fn from(poll: Poll<Result<Vec<u8>, Error>>) -> Self {
        use contract::PollQuery;
        match poll {
            Poll::Pending => PollQuery::Pending,
            Poll::Ready(Ok(response)) => PollQuery::Ready(Ok(response)),
            Poll::Ready(Err(value)) => PollQuery::Ready(Err(value.to_string())),
        }
    }
}

impl From<api::PollGet> for Poll<Result<Vec<u8>, String>> {
    fn from(poll_get: api::PollGet) -> Poll<Result<Vec<u8>, String>> {
        match poll_get {
            api::PollGet::Ready(bytes) => Poll::Ready(bytes),
            api::PollGet::Pending => Poll::Pending,
        }
    }
}
