wit_bindgen_wasmtime::export!("../linera-contracts/api.wit");
wit_bindgen_wasmtime::import!("../linera-contracts/contract.wit");

use self::{
    api::{ApiTables, PollGet},
    contract::{
        ApplyEffect, ApplyOperation, CallApplication, CallSession, Contract, ContractData,
        PollCallApplication, PollCallSession, PollExecutionResult,
    },
};
use crate::{
    ApplicationCallResult, CalleeContext, EffectContext, NewSession, OperationContext,
    RawExecutionResult, SessionCallResult, SessionId, WritableStorage,
};
use futures::future::BoxFuture;
use linera_base::{
    crypto::IncorrectHashSize,
    messages::{Destination, EffectId},
};
use std::{
    any::type_name,
    fmt::{self, Debug, Formatter},
    future::Future,
    marker::PhantomData,
    mem,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use thiserror::Error;
use tokio::sync::Mutex;
use wasmtime::{Engine, Linker, Module, Store, Trap};

#[derive(Default)]
pub struct WasmApplication {}

impl WasmApplication {
    pub fn prepare_runtime<'storage>(
        &self,
        storage: &'storage dyn WritableStorage,
    ) -> Result<WritableRuntimeContext<'storage>, PrepareRuntimeError> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);

        api::add_to_linker(&mut linker, Data::api)?;

        let module = Module::from_file(
            &engine,
            "/project/linera-contracts/example/target/wasm32-unknown-unknown/debug/linera_contract_example.wasm",
        )?;
        let context_forwarder = ContextForwarder::default();
        let data = Data::new(storage, context_forwarder.clone());
        let mut store = Store::new(&engine, data);
        let (contract, _instance) =
            Contract::instantiate(&mut store, &module, &mut linker, Data::contract)?;

        Ok(WritableRuntimeContext {
            context_forwarder,
            contract,
            store,
        })
    }
}

pub struct WritableRuntimeContext<'data> {
    context_forwarder: ContextForwarder,
    contract: Contract<Data<'data>>,
    store: Store<Data<'data>>,
}

impl<'data> WritableRuntimeContext<'data> {
    pub fn apply_operation(
        mut self,
        context: &OperationContext,
        operation: &[u8],
    ) -> ExternalFuture<'data, ApplyOperation> {
        let future = self
            .contract
            .apply_operation_new(&mut self.store, context.into(), operation);

        ExternalFuture::new(
            future,
            self.context_forwarder.clone(),
            self.contract,
            self.store,
        )
    }

    pub fn apply_effect(
        mut self,
        context: &EffectContext,
        effect: &[u8],
    ) -> ExternalFuture<'data, ApplyEffect> {
        let future = self
            .contract
            .apply_effect_new(&mut self.store, context.into(), effect);

        ExternalFuture::new(
            future,
            self.context_forwarder.clone(),
            self.contract,
            self.store,
        )
    }

    pub fn call_application(
        mut self,
        context: &CalleeContext,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> ExternalFuture<'data, CallApplication> {
        let forwarded_sessions: Vec<_> = forwarded_sessions
            .into_iter()
            .map(contract::SessionId::from)
            .collect();

        let future = self.contract.call_application_new(
            &mut self.store,
            context.into(),
            argument,
            &forwarded_sessions,
        );

        ExternalFuture::new(
            future,
            self.context_forwarder.clone(),
            self.contract,
            self.store,
        )
    }

    pub fn call_session(
        mut self,
        context: &CalleeContext,
        session_kind: u64,
        session_data: &mut Vec<u8>,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> ExternalFuture<'data, CallSession> {
        let forwarded_sessions: Vec<_> = forwarded_sessions
            .into_iter()
            .map(contract::SessionId::from)
            .collect();

        let session = contract::SessionParam {
            kind: session_kind,
            data: &*session_data,
        };

        let future = self.contract.call_session_new(
            &mut self.store,
            context.into(),
            session,
            argument,
            &forwarded_sessions,
        );

        ExternalFuture::new(
            future,
            self.context_forwarder.clone(),
            self.contract,
            self.store,
        )
    }
}

pub struct Data<'storage> {
    contract: ContractData,
    api: Api<'storage>,
    api_tables: ApiTables<Api<'storage>>,
}

impl<'storage> Data<'storage> {
    pub fn new(storage: &'storage dyn WritableStorage, context: ContextForwarder) -> Self {
        Data {
            contract: ContractData::default(),
            api: Api { storage, context },
            api_tables: ApiTables::default(),
        }
    }

    pub fn contract(&mut self) -> &mut ContractData {
        &mut self.contract
    }

    pub fn api(&mut self) -> (&mut Api<'storage>, &mut ApiTables<Api<'storage>>) {
        (&mut self.api, &mut self.api_tables)
    }
}

pub struct Api<'storage> {
    context: ContextForwarder,
    storage: &'storage dyn WritableStorage,
}

impl<'storage> api::Api for Api<'storage> {
    type Get = ExportedFuture<'storage, Result<Vec<u8>, linera_base::error::Error>>;

    fn get_new(&mut self) -> Self::Get {
        let future = self.storage.try_read_and_lock_my_state();
        ExportedFuture::new(self.storage.try_read_and_lock_my_state())
    }

    fn get_poll(&mut self, future: &Self::Get) -> PollGet {
        match future.poll(&mut self.context) {
            Poll::Pending => PollGet::Pending,
            Poll::Ready(Ok(bytes)) => PollGet::Ready(Ok(bytes)),
            Poll::Ready(Err(error)) => PollGet::Ready(Err(error.to_string())),
        }
    }

    fn set(&mut self, state: &[u8]) -> bool {
        self.storage.save_and_unlock_my_state(state.to_owned());
        // TODO
        true
    }
}

#[derive(Debug, Error)]
pub enum PrepareRuntimeError {
    #[error("Failed to instantiate smart contract Wasm module")]
    Instantiate(#[from] wit_bindgen_wasmtime::anyhow::Error),
}

impl From<PrepareRuntimeError> for linera_base::error::Error {
    fn from(error: PrepareRuntimeError) -> Self {
        // TODO
        linera_base::error::Error::UnknownApplication
    }
}

impl<'argument> From<&'argument OperationContext> for contract::OperationContext<'argument> {
    fn from(host: &'argument OperationContext) -> Self {
        contract::OperationContext {
            chain_id: host.chain_id.0.as_bytes().as_slice(),
            height: host.height.0,
            index: host
                .index
                .try_into()
                .expect("Operation index should fit in an `u64`"),
        }
    }
}

impl<'argument> From<&'argument EffectContext> for contract::EffectContext<'argument> {
    fn from(host: &'argument EffectContext) -> Self {
        contract::EffectContext {
            chain_id: host.chain_id.0.as_bytes().as_slice(),
            height: host.height.0,
            effect_id: (&host.effect_id).into(),
        }
    }
}

impl<'argument> From<&'argument EffectId> for contract::EffectId<'argument> {
    fn from(host: &'argument EffectId) -> Self {
        contract::EffectId {
            chain_id: host.chain_id.0.as_bytes().as_slice(),
            height: host.height.0,
            index: host
                .index
                .try_into()
                .expect("Effect index should fit in an `u64`"),
        }
    }
}

impl<'argument> From<&'argument CalleeContext> for contract::CalleeContext<'argument> {
    fn from(host: &'argument CalleeContext) -> Self {
        contract::CalleeContext {
            chain_id: host.chain_id.0.as_bytes().as_slice(),
            authenticated_caller_id: host.authenticated_caller_id.map(|app_id| app_id.0),
        }
    }
}

impl From<SessionId> for contract::SessionId {
    fn from(host: SessionId) -> Self {
        contract::SessionId {
            application_id: host.application_id.0,
            kind: host.kind,
            index: host.index,
        }
    }
}

pub enum ExternalFuture<'data, Future> {
    FailedToCreate(Trap),
    Active {
        context_forwarder: ContextForwarder,
        contract: Contract<Data<'data>>,
        store: Store<Data<'data>>,
        future: Future,
    },
}

impl<'data, Future> ExternalFuture<'data, Future> {
    pub fn new(
        creation_result: Result<Future, Trap>,
        context_forwarder: ContextForwarder,
        contract: Contract<Data<'data>>,
        store: Store<Data<'data>>,
    ) -> Self {
        match creation_result {
            Ok(future) => ExternalFuture::Active {
                context_forwarder,
                contract,
                store,
                future,
            },
            Err(trap) => ExternalFuture::FailedToCreate(trap),
        }
    }
}

impl<InnerFuture> Future for ExternalFuture<'_, InnerFuture>
where
    InnerFuture: ExternalFutureInterface + Unpin,
{
    type Output = Result<InnerFuture::Output, linera_base::error::Error>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        match self.get_mut() {
            ExternalFuture::FailedToCreate(_) => {
                Poll::Ready(Err(linera_base::error::Error::UnknownApplication))
            }
            ExternalFuture::Active {
                context_forwarder,
                contract,
                store,
                future,
            } => {
                let _context_guard = context_forwarder.forward(context);
                future.poll(contract, store)
            }
        }
    }
}

pub trait ExternalFutureInterface {
    type Output;

    fn poll<'data>(
        &self,
        contract: &Contract<Data<'data>>,
        store: &mut Store<Data<'data>>,
    ) -> Poll<Result<Self::Output, linera_base::error::Error>>;
}

impl ExternalFutureInterface for ApplyOperation {
    type Output = RawExecutionResult<Vec<u8>>;

    fn poll<'data>(
        &self,
        contract: &Contract<Data<'data>>,
        store: &mut Store<Data<'data>>,
    ) -> Poll<Result<Self::Output, linera_base::error::Error>> {
        match contract.apply_operation_poll(store, self) {
            Ok(PollExecutionResult::Ready(Ok(result))) => Poll::Ready(result.try_into()),
            Ok(PollExecutionResult::Ready(Err(_message))) => {
                Poll::Ready(Err(linera_base::error::Error::UnknownApplication))
            }
            Ok(PollExecutionResult::Pending) => Poll::Pending,
            Err(_) => Poll::Ready(Err(linera_base::error::Error::UnknownApplication)),
        }
    }
}

impl ExternalFutureInterface for ApplyEffect {
    type Output = RawExecutionResult<Vec<u8>>;

    fn poll<'data>(
        &self,
        contract: &Contract<Data<'data>>,
        store: &mut Store<Data<'data>>,
    ) -> Poll<Result<Self::Output, linera_base::error::Error>> {
        match contract.apply_effect_poll(store, self) {
            Ok(PollExecutionResult::Ready(Ok(result))) => Poll::Ready(result.try_into()),
            Ok(PollExecutionResult::Ready(Err(_message))) => {
                Poll::Ready(Err(linera_base::error::Error::UnknownApplication))
            }
            Ok(PollExecutionResult::Pending) => Poll::Pending,
            Err(_) => Poll::Ready(Err(linera_base::error::Error::UnknownApplication)),
        }
    }
}

impl ExternalFutureInterface for CallApplication {
    type Output = ApplicationCallResult;

    fn poll<'data>(
        &self,
        contract: &Contract<Data<'data>>,
        store: &mut Store<Data<'data>>,
    ) -> Poll<Result<Self::Output, linera_base::error::Error>> {
        match contract.call_application_poll(store, self) {
            Ok(PollCallApplication::Ready(Ok(result))) => Poll::Ready(result.try_into()),
            Ok(PollCallApplication::Ready(Err(_message))) => {
                Poll::Ready(Err(linera_base::error::Error::UnknownApplication))
            }
            Ok(PollCallApplication::Pending) => Poll::Pending,
            Err(_) => Poll::Ready(Err(linera_base::error::Error::UnknownApplication)),
        }
    }
}

impl ExternalFutureInterface for CallSession {
    type Output = SessionCallResult;

    fn poll<'data>(
        &self,
        contract: &Contract<Data<'data>>,
        store: &mut Store<Data<'data>>,
    ) -> Poll<Result<Self::Output, linera_base::error::Error>> {
        match contract.call_session_poll(store, self) {
            Ok(PollCallSession::Ready(Ok(result))) => Poll::Ready(result.try_into()),
            Ok(PollCallSession::Ready(Err(_message))) => {
                Poll::Ready(Err(linera_base::error::Error::UnknownApplication))
            }
            Ok(PollCallSession::Pending) => Poll::Pending,
            Err(_) => Poll::Ready(Err(linera_base::error::Error::UnknownApplication)),
        }
    }
}

impl TryFrom<contract::SessionCallResult> for SessionCallResult {
    type Error = linera_base::error::Error;

    fn try_from(result: contract::SessionCallResult) -> Result<Self, Self::Error> {
        Ok(SessionCallResult {
            inner: result.inner.try_into()?,
            close_session: result.data.is_some(),
        })
    }
}

impl TryFrom<contract::ApplicationCallResult> for ApplicationCallResult {
    type Error = linera_base::error::Error;

    fn try_from(result: contract::ApplicationCallResult) -> Result<Self, Self::Error> {
        let create_sessions = result
            .create_sessions
            .into_iter()
            .map(NewSession::from)
            .collect();

        Ok(ApplicationCallResult {
            create_sessions,
            execution_result: result.execution_result.try_into()?,
            value: result.value,
        })
    }
}

impl TryFrom<contract::ExecutionResult> for RawExecutionResult<Vec<u8>> {
    type Error = linera_base::error::Error;

    fn try_from(result: contract::ExecutionResult) -> Result<Self, Self::Error> {
        let effects = result
            .effects
            .into_iter()
            .map(|(destination, effect)| Ok((destination.try_into()?, effect)))
            .collect::<Result<_, IncorrectHashSize>>()
            .map_err(|_| linera_base::error::Error::UnknownApplication)?;

        let subscribe = result
            .subscribe
            .into_iter()
            .map(|(channel_id, chain_id)| Ok((channel_id, chain_id.as_slice().try_into()?)))
            .collect::<Result<_, IncorrectHashSize>>()
            .map_err(|_| linera_base::error::Error::UnknownApplication)?;

        let unsubscribe = result
            .unsubscribe
            .into_iter()
            .map(|(channel_id, chain_id)| Ok((channel_id, chain_id.as_slice().try_into()?)))
            .collect::<Result<_, IncorrectHashSize>>()
            .map_err(|_| linera_base::error::Error::UnknownApplication)?;

        Ok(RawExecutionResult {
            effects,
            subscribe,
            unsubscribe,
        })
    }
}

impl TryFrom<contract::Destination> for Destination {
    type Error = IncorrectHashSize;

    fn try_from(guest: contract::Destination) -> Result<Self, Self::Error> {
        Ok(match guest {
            contract::Destination::Recipient(chain_id) => {
                Destination::Recipient(chain_id.as_slice().try_into()?)
            }
            contract::Destination::Subscribers(channel_id) => Destination::Subscribers(channel_id),
        })
    }
}

impl From<contract::SessionResult> for NewSession {
    fn from(guest: contract::SessionResult) -> Self {
        NewSession {
            kind: guest.kind,
            data: guest.data,
        }
    }
}

pub struct ExportedFuture<'future, Output> {
    future: Mutex<BoxFuture<'future, Output>>,
}

impl<Output> Debug for ExportedFuture<'_, Output> {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter
            .debug_struct(&format!("ExportedFuture<'_, {}>", type_name::<Output>()))
            .finish_non_exhaustive()
    }
}

impl<'future, Output> ExportedFuture<'future, Output> {
    pub fn new(future: impl Future<Output = Output> + Send + 'future) -> Self {
        ExportedFuture {
            future: Mutex::new(Box::pin(future)),
        }
    }

    pub fn poll(&self, context: &mut ContextForwarder) -> Poll<Output> {
        let mut context_reference = context
            .0
            .try_lock()
            .expect("Unexpected concurrent contract call");

        let context = context_reference
            .as_mut()
            .expect("Contract called without an async task context");

        let mut future = self
            .future
            .try_lock()
            .expect("Contract can't call the future concurrently because it's single threaded");

        future.as_mut().poll(context)
    }
}

#[derive(Clone, Default)]
pub struct ContextForwarder(Arc<Mutex<Option<&'static mut Context<'static>>>>);

impl ContextForwarder {
    pub fn forward<'context>(
        &mut self,
        context: &'context mut Context,
    ) -> ActiveContextGuard<'context> {
        let mut context_reference = self
            .0
            .try_lock()
            .expect("Unexpected concurrent task context access");

        assert!(
            context_reference.is_none(),
            "`ContextForwarder` accessed by concurrent tasks"
        );

        *context_reference = Some(unsafe { mem::transmute(context) });

        ActiveContextGuard {
            context: self.0.clone(),
            lifetime: PhantomData,
        }
    }
}

pub struct ActiveContextGuard<'context> {
    context: Arc<Mutex<Option<&'static mut Context<'static>>>>,
    lifetime: PhantomData<&'context mut ()>,
}

impl Drop for ActiveContextGuard<'_> {
    fn drop(&mut self) {
        let mut context_reference = self
            .context
            .try_lock()
            .expect("Unexpected concurrent task context access");

        *context_reference = None;
    }
}
