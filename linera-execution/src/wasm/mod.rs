#[cfg(feature = "wasmer")]
#[path = "wasmer.rs"]
mod runtime;
#[cfg(feature = "wasmtime")]
#[path = "wasmtime.rs"]
mod runtime;

use crate::{
    system::Balance, ApplicationCallResult, CallResult, CalleeContext, EffectContext,
    OperationContext, QueryContext, QueryableStorage, RawExecutionResult, ReadableStorage,
    SessionCallResult, SessionId, UserApplication, WritableStorage,
};
use async_trait::async_trait;
use linera_base::{
    error::Error,
    messages::{ApplicationId, ChainId},
};

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

    fn save_and_unlock_my_state(&self, state: Vec<u8>) {}

    fn unlock_my_state(&self) {}

    async fn try_call_application(
        &self,
        authenticated: bool,
        callee_id: ApplicationId,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<CallResult, Error> {
        Err(Error::UnknownApplication)
    }

    async fn try_call_session(
        &self,
        authenticated: bool,
        session_id: SessionId,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<CallResult, Error> {
        Err(Error::UnknownApplication)
    }
}
