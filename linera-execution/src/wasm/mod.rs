#[cfg(feature = "wasmer")]
#[path = "wasmer.rs"]
mod runtime;
#[cfg(feature = "wasmtime")]
#[path = "wasmtime.rs"]
mod runtime;

use crate::{
    ApplicationCallResult, CalleeContext, EffectContext, OperationContext, QueryContext,
    QueryableStorage, RawExecutionResult, SessionCallResult, SessionId, UserApplication,
    WritableStorage,
};
use async_trait::async_trait;
use linera_base::error::Error;

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
        todo!();
    }
}
