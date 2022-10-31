wit_bindgen_host_wasmtime_rust::export!("../linera-contracts/api.wit");
wit_bindgen_host_wasmtime_rust::import!("../linera-contracts/contract.wit");

use self::{
    api::{ApiTables, PollLoad},
    contract::{Contract, ContractData},
};
use super::{
    async_boundary::{ContextForwarder, HostFuture},
    Runtime, WasmApplication, WritableRuntimeContext,
};
use crate::WritableStorage;
use std::{fmt::Debug, marker::PhantomData, task::Poll};
use thiserror::Error;
use wasmtime::{Engine, Linker, Module, Store, Trap};

pub struct Wasmtime<'storage> {
    _lifetime: PhantomData<&'storage ()>,
}

impl<'storage> Runtime for Wasmtime<'storage> {
    type Contract = Contract<Data<'storage>>;
    type Store = Store<Data<'storage>>;
    type StorageGuard = ();
    type Error = Trap;
}

impl WasmApplication {
    pub fn prepare_runtime<'storage>(
        &self,
        storage: &'storage dyn WritableStorage,
    ) -> Result<WritableRuntimeContext<Wasmtime<'storage>>, PrepareRuntimeError> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);

        api::add_to_linker(&mut linker, Data::api)?;

        let module = Module::from_file(&engine, &self.bytecode_file)?;
        let context_forwarder = ContextForwarder::default();
        let data = Data::new(storage, context_forwarder.clone());
        let mut store = Store::new(&engine, data);
        let (contract, _instance) =
            Contract::instantiate(&mut store, &module, &mut linker, Data::contract)?;

        Ok(WritableRuntimeContext {
            context_forwarder,
            contract,
            store,
            storage_guard: (),
        })
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

impl<'storage> super::Contract<Wasmtime<'storage>> for Contract<Data<'storage>> {
    fn apply_operation_new(
        &self,
        store: &mut Store<Data<'storage>>,
        context: contract::OperationContext<'_>,
        operation: &[u8],
    ) -> Result<contract::ApplyOperation, Trap> {
        Contract::apply_operation_new(self, store, context, operation)
    }

    fn apply_operation_poll(
        &self,
        store: &mut Store<Data<'storage>>,
        future: &contract::ApplyOperation,
    ) -> Result<contract::PollExecutionResult, Trap> {
        Contract::apply_operation_poll(self, store, future)
    }

    fn apply_effect_new(
        &self,
        store: &mut Store<Data<'storage>>,
        context: contract::EffectContext<'_>,
        effect: &[u8],
    ) -> Result<contract::ApplyEffect, Trap> {
        Contract::apply_effect_new(self, store, context, effect)
    }

    fn apply_effect_poll(
        &self,
        store: &mut Store<Data<'storage>>,
        future: &contract::ApplyEffect,
    ) -> Result<contract::PollExecutionResult, Trap> {
        Contract::apply_effect_poll(self, store, future)
    }

    fn call_application_new(
        &self,
        store: &mut Store<Data<'storage>>,
        context: contract::CalleeContext<'_>,
        argument: &[u8],
        forwarded_sessions: &[contract::SessionId],
    ) -> Result<contract::CallApplication, Trap> {
        Contract::call_application_new(self, store, context, argument, forwarded_sessions)
    }

    fn call_application_poll(
        &self,
        store: &mut Store<Data<'storage>>,
        future: &contract::CallApplication,
    ) -> Result<contract::PollCallApplication, Trap> {
        Contract::call_application_poll(self, store, future)
    }

    fn call_session_new(
        &self,
        store: &mut Store<Data<'storage>>,
        context: contract::CalleeContext<'_>,
        session: contract::SessionParam,
        argument: &[u8],
        forwarded_sessions: &[contract::SessionId],
    ) -> Result<contract::CallSession, Trap> {
        Contract::call_session_new(self, store, context, session, argument, forwarded_sessions)
    }

    fn call_session_poll(
        &self,
        store: &mut Store<Data<'storage>>,
        future: &contract::CallSession,
    ) -> Result<contract::PollCallSession, Trap> {
        Contract::call_session_poll(self, store, future)
    }

    fn query_application_new(
        &self,
        store: &mut Store<Data<'storage>>,
        context: contract::QueryContext<'_>,
        argument: &[u8],
    ) -> Result<contract::QueryApplication, Trap> {
        contract::Contract::query_application_new(self, store, context, argument)
    }

    fn query_application_poll(
        &self,
        store: &mut Store<Data<'storage>>,
        future: &contract::QueryApplication,
    ) -> Result<contract::PollQuery, Trap> {
        Contract::query_application_poll(self, store, future)
    }
}

pub struct Api<'storage> {
    context: ContextForwarder,
    storage: &'storage dyn WritableStorage,
}

impl<'storage> api::Api for Api<'storage> {
    type Load = HostFuture<'storage, Result<Vec<u8>, linera_base::error::Error>>;
    type LoadAndLock = HostFuture<'storage, Result<Vec<u8>, linera_base::error::Error>>;

    fn load_new(&mut self) -> Self::Load {
        HostFuture::new(self.storage.try_read_my_state())
    }

    fn load_poll(&mut self, future: &Self::Load) -> PollLoad {
        match future.poll(&mut self.context) {
            Poll::Pending => PollLoad::Pending,
            Poll::Ready(Ok(bytes)) => PollLoad::Ready(Ok(bytes)),
            Poll::Ready(Err(error)) => PollLoad::Ready(Err(error.to_string())),
        }
    }

    fn load_and_lock_new(&mut self) -> Self::LoadAndLock {
        HostFuture::new(self.storage.try_read_and_lock_my_state())
    }

    fn load_and_lock_poll(&mut self, future: &Self::LoadAndLock) -> PollLoad {
        match future.poll(&mut self.context) {
            Poll::Pending => PollLoad::Pending,
            Poll::Ready(Ok(bytes)) => PollLoad::Ready(Ok(bytes)),
            Poll::Ready(Err(error)) => PollLoad::Ready(Err(error.to_string())),
        }
    }

    fn store_and_unlock(&mut self, state: &[u8]) -> bool {
        self.storage.save_and_unlock_my_state(state.to_owned());
        // TODO
        true
    }
}

#[derive(Debug, Error)]
pub enum PrepareRuntimeError {
    #[error("Failed to instantiate smart contract Wasm module")]
    Instantiate(#[from] wit_bindgen_host_wasmtime_rust::anyhow::Error),
}

impl From<PrepareRuntimeError> for linera_base::error::Error {
    fn from(error: PrepareRuntimeError) -> Self {
        // TODO
        linera_base::error::Error::UnknownApplication
    }
}
