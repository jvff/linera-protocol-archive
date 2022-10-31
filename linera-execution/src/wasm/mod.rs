#![cfg(any(feature = "wasmer", feature = "wasmtime"))]

mod async_boundary;
mod common;
#[cfg(feature = "wasmer")]
#[path = "wasmer.rs"]
mod runtime;
#[cfg(feature = "wasmtime")]
#[path = "wasmtime.rs"]
mod runtime;

use self::{
    async_boundary::{ContextForwarder, GuestFuture, GuestFutureInterface},
    runtime::contract::{
        self, ApplyEffect, ApplyOperation, CallApplication, CallSession, PollCallApplication,
        PollCallSession, PollExecutionResult, PollQuery, QueryApplication,
    },
};
use crate::{
    system::Balance, ApplicationCallResult, CallResult, CalleeContext, EffectContext, EffectId,
    NewSession, OperationContext, QueryContext, QueryableStorage, RawExecutionResult,
    ReadableStorage, SessionCallResult, SessionId, UserApplication, WritableStorage,
};
use async_trait::async_trait;
use linera_base::{
    crypto::IncorrectHashSize,
    error::Error,
    messages::{ApplicationId, ChainId, Destination},
};
use std::task::Poll;

pub use self::runtime::WasmApplication;

#[async_trait]
impl UserApplication for WasmApplication {
    async fn execute_operation(
        &self,
        context: &OperationContext,
        storage: &dyn WritableStorage,
        operation: &[u8],
    ) -> Result<RawExecutionResult<Vec<u8>>, Error> {
        self.prepare_runtime(storage)?
            .apply_operation(context, operation)
            .await
    }

    async fn execute_effect(
        &self,
        context: &EffectContext,
        storage: &dyn WritableStorage,
        effect: &[u8],
    ) -> Result<RawExecutionResult<Vec<u8>>, Error> {
        self.prepare_runtime(storage)?
            .apply_effect(context, effect)
            .await
    }

    async fn call_application(
        &self,
        context: &CalleeContext,
        storage: &dyn WritableStorage,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<ApplicationCallResult, Error> {
        self.prepare_runtime(storage)?
            .call_application(context, argument, forwarded_sessions)
            .await
    }

    async fn call_session(
        &self,
        context: &CalleeContext,
        storage: &dyn WritableStorage,
        session_kind: u64,
        session_data: &mut Vec<u8>,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<SessionCallResult, Error> {
        self.prepare_runtime(storage)?
            .call_session(
                context,
                session_kind,
                session_data,
                argument,
                forwarded_sessions,
            )
            .await
    }

    async fn query_application(
        &self,
        context: &QueryContext,
        storage: &dyn QueryableStorage,
        argument: &[u8],
    ) -> Result<Vec<u8>, Error> {
        let wrapped_storage = WrappedQueryableStorage(storage);
        let storage_reference = &wrapped_storage;
        let result = self
            .prepare_runtime(storage_reference)?
            .query_application(context, argument)
            .await;
        result
    }
}

pub trait Runtime: Sized {
    type Contract: Contract<Self>;
    type Store;
    type StorageGuard;
    type Error;
}

pub trait Contract<R: Runtime> {
    fn apply_operation_new(
        &self,
        store: &mut R::Store,
        context: contract::OperationContext<'_>,
        operation: &[u8],
    ) -> Result<contract::ApplyOperation, R::Error>;

    fn apply_operation_poll(
        &self,
        store: &mut R::Store,
        future: &contract::ApplyOperation,
    ) -> Result<contract::PollExecutionResult, R::Error>;

    fn apply_effect_new(
        &self,
        store: &mut R::Store,
        context: contract::EffectContext<'_>,
        effect: &[u8],
    ) -> Result<contract::ApplyEffect, R::Error>;

    fn apply_effect_poll(
        &self,
        store: &mut R::Store,
        future: &contract::ApplyEffect,
    ) -> Result<contract::PollExecutionResult, R::Error>;

    fn call_application_new(
        &self,
        store: &mut R::Store,
        context: contract::CalleeContext<'_>,
        argument: &[u8],
        forwarded_sessions: &[contract::SessionId],
    ) -> Result<contract::CallApplication, R::Error>;

    fn call_application_poll(
        &self,
        store: &mut R::Store,
        future: &contract::CallApplication,
    ) -> Result<contract::PollCallApplication, R::Error>;

    fn call_session_new(
        &self,
        store: &mut R::Store,
        context: contract::CalleeContext<'_>,
        session: contract::SessionParam,
        argument: &[u8],
        forwarded_sessions: &[contract::SessionId],
    ) -> Result<contract::CallSession, R::Error>;

    fn call_session_poll(
        &self,
        store: &mut R::Store,
        future: &contract::CallSession,
    ) -> Result<contract::PollCallSession, R::Error>;

    fn query_application_new(
        &self,
        store: &mut R::Store,
        context: contract::QueryContext<'_>,
        argument: &[u8],
    ) -> Result<contract::QueryApplication, R::Error>;

    fn query_application_poll(
        &self,
        store: &mut R::Store,
        future: &contract::QueryApplication,
    ) -> Result<contract::PollQuery, R::Error>;
}

pub struct WritableRuntimeContext<R>
where
    R: Runtime,
{
    context_forwarder: ContextForwarder,
    contract: R::Contract,
    store: R::Store,
    storage_guard: R::StorageGuard,
}

impl<R> WritableRuntimeContext<R>
where
    R: Runtime,
{
    pub fn apply_operation(
        mut self,
        context: &OperationContext,
        operation: &[u8],
    ) -> GuestFuture<ApplyOperation, R> {
        let future = self
            .contract
            .apply_operation_new(&mut self.store, context.into(), operation);

        GuestFuture::new(future, self)
    }

    pub fn apply_effect(
        mut self,
        context: &EffectContext,
        effect: &[u8],
    ) -> GuestFuture<ApplyEffect, R> {
        let future = self
            .contract
            .apply_effect_new(&mut self.store, context.into(), effect);

        GuestFuture::new(future, self)
    }

    pub fn call_application(
        mut self,
        context: &CalleeContext,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> GuestFuture<CallApplication, R> {
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

        GuestFuture::new(future, self)
    }

    pub fn call_session(
        mut self,
        context: &CalleeContext,
        session_kind: u64,
        session_data: &mut Vec<u8>,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> GuestFuture<CallSession, R> {
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

        GuestFuture::new(future, self)
    }

    pub fn query_application(
        mut self,
        context: &QueryContext,
        argument: &[u8],
    ) -> GuestFuture<QueryApplication, R> {
        let future = self
            .contract
            .query_application_new(&mut self.store, context.into(), argument);

        GuestFuture::new(future, self)
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

impl<'argument> From<&'argument QueryContext> for contract::QueryContext<'argument> {
    fn from(host: &'argument QueryContext) -> Self {
        contract::QueryContext {
            chain_id: host.chain_id.0.as_bytes().as_slice(),
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

impl<'storage, R> GuestFutureInterface<R> for ApplyOperation
where
    R: Runtime,
{
    type Output = RawExecutionResult<Vec<u8>>;

    fn poll(
        &self,
        contract: &R::Contract,
        store: &mut R::Store,
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

impl<'storage, R> GuestFutureInterface<R> for ApplyEffect
where
    R: Runtime,
{
    type Output = RawExecutionResult<Vec<u8>>;

    fn poll(
        &self,
        contract: &R::Contract,
        store: &mut R::Store,
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

impl<'storage, R> GuestFutureInterface<R> for CallApplication
where
    R: Runtime,
{
    type Output = ApplicationCallResult;

    fn poll(
        &self,
        contract: &R::Contract,
        store: &mut R::Store,
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

impl<'storage, R> GuestFutureInterface<R> for CallSession
where
    R: Runtime,
{
    type Output = SessionCallResult;

    fn poll(
        &self,
        contract: &R::Contract,
        store: &mut R::Store,
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

impl<'storage, R> GuestFutureInterface<R> for QueryApplication
where
    R: Runtime,
{
    type Output = Vec<u8>;

    fn poll(
        &self,
        contract: &R::Contract,
        store: &mut R::Store,
    ) -> Poll<Result<Self::Output, linera_base::error::Error>> {
        match contract.query_application_poll(store, self) {
            Ok(PollQuery::Ready(Ok(result))) => Poll::Ready(Ok(result)),
            Ok(PollQuery::Ready(Err(message))) => {
                Poll::Ready(Err(linera_base::error::Error::UnknownApplication))
            }
            Ok(PollQuery::Pending) => Poll::Pending,
            Err(error) => Poll::Ready(Err(linera_base::error::Error::UnknownApplication)),
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

struct WrappedQueryableStorage<'storage>(&'storage dyn QueryableStorage);

#[async_trait]
impl ReadableStorage for WrappedQueryableStorage<'_> {
    fn chain_id(&self) -> ChainId {
        self.0.chain_id()
    }

    fn application_id(&self) -> ApplicationId {
        self.0.application_id()
    }

    fn read_system_balance(&self) -> Balance {
        self.0.read_system_balance()
    }

    async fn try_read_my_state(&self) -> Result<Vec<u8>, Error> {
        self.0.try_read_my_state().await
    }
}

#[async_trait]
impl WritableStorage for WrappedQueryableStorage<'_> {
    async fn try_read_and_lock_my_state(&self) -> Result<Vec<u8>, Error> {
        Err(Error::UnknownApplication)
    }

    fn save_and_unlock_my_state(&self, _state: Vec<u8>) {}

    fn unlock_my_state(&self) {}

    async fn try_call_application(
        &self,
        _authenticated: bool,
        _callee_id: ApplicationId,
        _argument: &[u8],
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<CallResult, Error> {
        Err(Error::UnknownApplication)
    }

    async fn try_call_session(
        &self,
        _authenticated: bool,
        _session_id: SessionId,
        _argument: &[u8],
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<CallResult, Error> {
        Err(Error::UnknownApplication)
    }
}
