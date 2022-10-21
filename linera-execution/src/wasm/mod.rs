use crate::{
    ApplicationCallResult, CalleeContext, EffectContext, OperationContext, QueryContext,
    QueryableStorage, RawExecutionResult, SessionCallResult, SessionId, UserApplication,
    WritableStorage,
};
use async_trait::async_trait;
use linera_base::error::Error;

#[cfg(feature = "wasmer")]
mod wasmer;
#[cfg(feature = "wasmtime")]
mod wasmtime;

pub struct WasmApplication {}

#[async_trait]
impl UserApplication for WasmApplication {
    async fn execute_operation(
        &self,
        context: &OperationContext,
        storage: &dyn WritableStorage,
        operation: &[u8],
    ) -> Result<RawExecutionResult<Vec<u8>>, Error> {
        let (contract, store, context) = self.prepare_runtime(storage);
        let external_future = contract.apply_operation_new();
        // self.call_async(
        // future::poll_fn(|context| {

        // })
    }

    async fn execute_effect(
        &self,
        context: &EffectContext,
        storage: &dyn WritableStorage,
        effect: &[u8],
    ) -> Result<RawExecutionResult<Vec<u8>>, Error> {
        todo!();
    }

    async fn call_application(
        &self,
        context: &CalleeContext,
        storage: &dyn WritableStorage,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<ApplicationCallResult, Error> {
        todo!();
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
        todo!();
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
