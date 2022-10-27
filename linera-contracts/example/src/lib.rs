use {
    async_trait::async_trait,
    futures::{channel::oneshot, future, join},
    linera_sdk::{
        Application, ApplicationCallResult, ApplicationId, BlockHeight, CalleeContext, ChainId,
        Destination, EffectContext, EffectId, ExecutionResult, ExportedFuture, OperationContext,
        QueryContext, Session, SessionCallResult, SessionId,
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
    ) -> Result<ExecutionResult, Self::Error> {
        self.balance += 1;
        Ok(ExecutionResult {
            effects: vec![],
            subscribe: vec![],
            unsubscribe: vec![],
        })
    }

    async fn apply_effect(
        &mut self,
        context: &EffectContext,
        effect: &[u8],
    ) -> Result<ExecutionResult, Self::Error> {
        todo!();
    }

    async fn call_application(
        &mut self,
        context: &CalleeContext,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<ApplicationCallResult, Self::Error> {
        todo!();
    }

    async fn call_session(
        &mut self,
        context: &CalleeContext,
        session: Session,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<SessionCallResult, Self::Error> {
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
    future: ExportedFuture<Result<ExecutionResult, Error>>,
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

    fn poll(&self) -> contract::PollExecutionResult {
        self.future.poll()
    }
}

pub struct ApplyEffect {
    future: ExportedFuture<Result<ExecutionResult, Error>>,
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

    fn poll(&self) -> contract::PollExecutionResult {
        self.future.poll()
    }
}

pub struct CallApplication {
    future: ExportedFuture<Result<ApplicationCallResult, Error>>,
}

impl contract::CallApplication for CallApplication {
    fn new(
        context: contract::CalleeContext,
        argument: Vec<u8>,
        forwarded_sessions: Vec<contract::SessionId>,
    ) -> Handle<Self> {
        Handle::new(CallApplication {
            future: ExportedFuture::new(async move {
                let mut contract = Contract::load().await;

                let forwarded_sessions = forwarded_sessions
                    .into_iter()
                    .map(SessionId::from)
                    .collect();

                let result = contract
                    .call_application(&context.into(), &argument, forwarded_sessions)
                    .await;
                if result.is_ok() {
                    contract.store().await;
                }
                result
            }),
        })
    }

    fn poll(&self) -> contract::PollCallApplication {
        self.future.poll()
    }
}

pub struct CallSession {
    future: ExportedFuture<Result<SessionCallResult, Error>>,
}

impl contract::CallSession for CallSession {
    fn new(
        context: contract::CalleeContext,
        session: contract::Session,
        argument: Vec<u8>,
        forwarded_sessions: Vec<contract::SessionId>,
    ) -> Handle<Self> {
        Handle::new(CallSession {
            future: ExportedFuture::new(async move {
                let mut contract = Contract::load().await;

                let forwarded_sessions = forwarded_sessions
                    .into_iter()
                    .map(SessionId::from)
                    .collect();

                let result = contract
                    .call_session(
                        &context.into(),
                        session.into(),
                        &argument,
                        forwarded_sessions,
                    )
                    .await;
                if result.is_ok() {
                    contract.store().await;
                }
                result
            }),
        })
    }

    fn poll(&self) -> contract::PollCallSession {
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

impl From<contract::SessionId> for SessionId {
    fn from(session_id: contract::SessionId) -> Self {
        SessionId {
            application_id: ApplicationId(session_id.application_id),
            kind: session_id.kind,
            index: session_id.index,
        }
    }
}

impl From<contract::Session> for Session {
    fn from(session: contract::Session) -> Self {
        Session {
            kind: session.kind,
            data: session.data,
        }
    }
}

impl From<ApplicationCallResult> for contract::ApplicationCallResult {
    fn from(result: ApplicationCallResult) -> Self {
        let create_sessions = result
            .create_sessions
            .into_iter()
            .map(contract::Session::from)
            .collect();

        contract::ApplicationCallResult {
            create_sessions,
            execution_result: result.execution_result.into(),
            value: result.value,
        }
    }
}

impl From<Session> for contract::Session {
    fn from(new_session: Session) -> Self {
        contract::Session {
            kind: new_session.kind,
            data: new_session.data,
        }
    }
}

impl From<SessionCallResult> for contract::SessionCallResult {
    fn from(result: SessionCallResult) -> Self {
        contract::SessionCallResult {
            inner: result.inner.into(),
            data: result.data,
        }
    }
}

impl From<ExecutionResult> for contract::ExecutionResult {
    fn from(result: ExecutionResult) -> Self {
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

        contract::ExecutionResult {
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

impl From<Poll<Result<ExecutionResult, Error>>> for contract::PollExecutionResult {
    fn from(poll: Poll<Result<ExecutionResult, Error>>) -> Self {
        use contract::PollExecutionResult;
        match poll {
            Poll::Pending => PollExecutionResult::Pending,
            Poll::Ready(Ok(value)) => PollExecutionResult::Ready(Ok(value.into())),
            Poll::Ready(Err(value)) => PollExecutionResult::Ready(Err(value.to_string())),
        }
    }
}

impl From<Poll<Result<ApplicationCallResult, Error>>> for contract::PollCallApplication {
    fn from(poll: Poll<Result<ApplicationCallResult, Error>>) -> Self {
        use contract::PollCallApplication;
        match poll {
            Poll::Pending => PollCallApplication::Pending,
            Poll::Ready(Ok(result)) => PollCallApplication::Ready(Ok(result.into())),
            Poll::Ready(Err(value)) => PollCallApplication::Ready(Err(value.to_string())),
        }
    }
}

impl From<Poll<Result<SessionCallResult, Error>>> for contract::PollCallSession {
    fn from(poll: Poll<Result<SessionCallResult, Error>>) -> Self {
        use contract::PollCallSession;
        match poll {
            Poll::Pending => PollCallSession::Pending,
            Poll::Ready(Ok(result)) => PollCallSession::Ready(Ok(result.into())),
            Poll::Ready(Err(value)) => PollCallSession::Ready(Err(value.to_string())),
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
