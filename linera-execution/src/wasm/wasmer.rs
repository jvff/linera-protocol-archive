wit_bindgen_host_wasmer_rust::export!("../linera-contracts/api.wit");
wit_bindgen_host_wasmer_rust::import!("../linera-contracts/contract.wit");

use self::{api::PollLoad, contract::Contract};
use super::{
    async_boundary::{ContextForwarder, HostFuture},
    Runtime, WritableRuntimeContext,
};
use crate::WritableStorage;
use std::{marker::PhantomData, mem, sync::Arc, task::Poll};
use thiserror::Error;
use tokio::sync::Mutex;
use wasmer::{imports, Module, RuntimeError, Store};

pub struct Wasmer<'storage> {
    _lifetime: PhantomData<&'storage ()>,
}

impl<'storage> Runtime for Wasmer<'storage> {
    type Contract = Contract;
    type Store = Store;
    type StorageGuard = StorageGuard<'storage>;
    type Error = RuntimeError;
}

#[derive(Default)]
pub struct WasmApplication {}

impl WasmApplication {
    pub fn prepare_runtime<'storage>(
        &self,
        storage: &'storage dyn WritableStorage,
    ) -> Result<WritableRuntimeContext<Wasmer<'storage>>, PrepareRuntimeError> {
        let mut store = Store::default();
        let module = Module::from_file(
            &store,
            "/project/linera-contracts/example/target/wasm32-unknown-unknown/debug/linera_contract_example.wasm",
        ).map_err(wit_bindgen_host_wasmer_rust::anyhow::Error::from)?; // TODO: Remove `map_err` if Wasmer issue #3267 is fixed
        let mut imports = imports! {};
        let context_forwarder = ContextForwarder::default();
        let (api, storage_guard) = Api::new(context_forwarder.clone(), storage);
        let api_setup = api::add_to_imports(&mut store, &mut imports, api);
        let (contract, instance) =
            contract::Contract::instantiate(&mut store, &module, &mut imports)?;

        api_setup(&instance, &store)?;

        Ok(WritableRuntimeContext {
            context_forwarder,
            contract,
            store,
            storage_guard,
        })
    }
}

#[derive(Debug, Error)]
pub enum PrepareRuntimeError {
    #[error("Failed to instantiate smart contract Wasm module")]
    Instantiate(#[from] wit_bindgen_host_wasmer_rust::anyhow::Error),
}

impl From<PrepareRuntimeError> for linera_base::error::Error {
    fn from(error: PrepareRuntimeError) -> Self {
        // TODO
        linera_base::error::Error::UnknownApplication
    }
}

impl<'storage> super::Contract<Wasmer<'storage>> for Contract {
    fn apply_operation_new(
        &self,
        store: &mut Store,
        context: contract::OperationContext<'_>,
        operation: &[u8],
    ) -> Result<contract::ApplyOperation, RuntimeError> {
        Contract::apply_operation_new(self, store, context, operation)
    }

    fn apply_operation_poll(
        &self,
        store: &mut Store,
        future: &contract::ApplyOperation,
    ) -> Result<contract::PollExecutionResult, RuntimeError> {
        Contract::apply_operation_poll(self, store, future)
    }

    fn apply_effect_new(
        &self,
        store: &mut Store,
        context: contract::EffectContext<'_>,
        effect: &[u8],
    ) -> Result<contract::ApplyEffect, RuntimeError> {
        Contract::apply_effect_new(self, store, context, effect)
    }

    fn apply_effect_poll(
        &self,
        store: &mut Store,
        future: &contract::ApplyEffect,
    ) -> Result<contract::PollExecutionResult, RuntimeError> {
        Contract::apply_effect_poll(self, store, future)
    }

    fn call_application_new(
        &self,
        store: &mut Store,
        context: contract::CalleeContext<'_>,
        argument: &[u8],
        forwarded_sessions: &[contract::SessionId],
    ) -> Result<contract::CallApplication, RuntimeError> {
        Contract::call_application_new(self, store, context, argument, forwarded_sessions)
    }

    fn call_application_poll(
        &self,
        store: &mut Store,
        future: &contract::CallApplication,
    ) -> Result<contract::PollCallApplication, RuntimeError> {
        Contract::call_application_poll(self, store, future)
    }

    fn call_session_new(
        &self,
        store: &mut Store,
        context: contract::CalleeContext<'_>,
        session: contract::SessionParam,
        argument: &[u8],
        forwarded_sessions: &[contract::SessionId],
    ) -> Result<contract::CallSession, RuntimeError> {
        Contract::call_session_new(self, store, context, session, argument, forwarded_sessions)
    }

    fn call_session_poll(
        &self,
        store: &mut Store,
        future: &contract::CallSession,
    ) -> Result<contract::PollCallSession, RuntimeError> {
        Contract::call_session_poll(self, store, future)
    }

    fn query_application_new(
        &self,
        store: &mut Store,
        context: contract::QueryContext<'_>,
        argument: &[u8],
    ) -> Result<contract::QueryApplication, RuntimeError> {
        contract::Contract::query_application_new(self, store, context, argument)
    }

    fn query_application_poll(
        &self,
        store: &mut Store,
        future: &contract::QueryApplication,
    ) -> Result<contract::PollQuery, RuntimeError> {
        Contract::query_application_poll(self, store, future)
    }
}

pub struct Api {
    context: ContextForwarder,
    storage: Arc<Mutex<Option<&'static dyn WritableStorage>>>,
}

impl Api {
    pub fn new(context: ContextForwarder, storage: &dyn WritableStorage) -> (Self, StorageGuard) {
        let storage_without_lifetime = unsafe { mem::transmute(storage) };
        let storage = Arc::new(Mutex::new(Some(storage_without_lifetime)));

        let guard = StorageGuard {
            storage: storage.clone(),
            _lifetime: PhantomData,
        };

        (Api { context, storage }, guard)
    }

    fn storage(&self) -> &'static dyn WritableStorage {
        *self
            .storage
            .try_lock()
            .expect("Unexpected concurrent storage access by contract")
            .as_ref()
            .expect("Contract called storage after it should have stopped")
    }
}

impl api::Api for Api {
    type Load = HostFuture<'static, Result<Vec<u8>, linera_base::error::Error>>;
    type LoadAndLock = HostFuture<'static, Result<Vec<u8>, linera_base::error::Error>>;

    fn load_new(&mut self) -> Self::Load {
        HostFuture::new(self.storage().try_read_my_state())
    }

    fn load_poll(&mut self, future: &Self::Load) -> PollLoad {
        match future.poll(&mut self.context) {
            Poll::Pending => PollLoad::Pending,
            Poll::Ready(Ok(bytes)) => PollLoad::Ready(Ok(bytes)),
            Poll::Ready(Err(error)) => PollLoad::Ready(Err(error.to_string())),
        }
    }

    fn load_and_lock_new(&mut self) -> Self::LoadAndLock {
        HostFuture::new(self.storage().try_read_and_lock_my_state())
    }

    fn load_and_lock_poll(&mut self, future: &Self::LoadAndLock) -> PollLoad {
        match future.poll(&mut self.context) {
            Poll::Pending => PollLoad::Pending,
            Poll::Ready(Ok(bytes)) => PollLoad::Ready(Ok(bytes)),
            Poll::Ready(Err(error)) => PollLoad::Ready(Err(error.to_string())),
        }
    }

    fn store_and_unlock(&mut self, state: &[u8]) -> bool {
        self.storage().save_and_unlock_my_state(state.to_owned());
        // TODO
        true
    }
}

pub struct StorageGuard<'storage> {
    storage: Arc<Mutex<Option<&'static dyn WritableStorage>>>,
    _lifetime: PhantomData<&'storage ()>,
}

impl Drop for StorageGuard<'_> {
    fn drop(&mut self) {
        self.storage
            .try_lock()
            .expect("Guard dropped while storage is still in use")
            .take();
    }
}
