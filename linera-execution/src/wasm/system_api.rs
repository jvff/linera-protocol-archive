// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::WasmExecutionError;
use crate::{BaseRuntime, CallResult, ContractRuntime, ServiceRuntime};
use linera_base::{
    data_types::{Amount, Timestamp},
    identifiers::{ApplicationId, ChainId, SessionId},
};
use linera_views::batch::{Batch, WriteOperation};
use linera_witty::{Instance, RuntimeError};
use std::{any::Any, collections::HashMap, marker::PhantomData};
use tracing::log;

pub struct SystemApiData<Runtime> {
    runtime: Runtime,
    active_promises: HashMap<u32, Box<dyn Any + Send + Sync>>,
    promise_counter: u32,
}

impl<Runtime> SystemApiData<Runtime> {
    pub fn new(runtime: Runtime) -> Self {
        SystemApiData {
            runtime,
            active_promises: HashMap::new(),
            promise_counter: 0,
        }
    }

    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    fn register_promise<Promise>(&mut self, promise: Promise) -> Result<u32, RuntimeError>
    where
        Promise: Send + Sync + 'static,
    {
        let id = self.promise_counter;

        self.active_promises.insert(id, Box::new(promise));
        self.promise_counter += 1;

        Ok(id)
    }

    fn take_promise<Promise>(&mut self, promise_id: u32) -> Result<Promise, RuntimeError>
    where
        Promise: Send + Sync + 'static,
    {
        let type_erased_promise = self
            .active_promises
            .remove(&promise_id)
            .ok_or_else(|| RuntimeError::Custom(WasmExecutionError::UnknownPromise.into()))?;

        type_erased_promise
            .downcast()
            .map(|boxed_promise| *boxed_promise)
            .map_err(|_| RuntimeError::Custom(WasmExecutionError::IncorrectPromise.into()))
    }
}

#[derive(Default)]
pub struct ContractSystemApi<Caller>(PhantomData<Caller>);

#[linera_witty::wit_export(package = "linera:app")]
impl<Caller, Runtime> ContractSystemApi<Caller>
where
    Caller: Instance<UserData = SystemApiData<Runtime>>,
    Runtime: ContractRuntime + Send + 'static,
{
    fn get_chain_id(caller: &mut Caller) -> Result<ChainId, RuntimeError> {
        tracing::error!("get_chain_id");
        caller
            .user_data_mut()
            .runtime
            .chain_id()
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn get_application_id(caller: &mut Caller) -> Result<ApplicationId, RuntimeError> {
        tracing::error!("get_applicaiton_id");
        caller
            .user_data_mut()
            .runtime
            .application_id()
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn get_application_parameters(caller: &mut Caller) -> Result<Vec<u8>, RuntimeError> {
        tracing::error!("get_applicaiton_parameters");
        caller
            .user_data_mut()
            .runtime
            .application_parameters()
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn read_system_balance(caller: &mut Caller) -> Result<Amount, RuntimeError> {
        tracing::error!("read_system_balance");
        caller
            .user_data_mut()
            .runtime
            .read_system_balance()
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn read_system_timestamp(caller: &mut Caller) -> Result<Timestamp, RuntimeError> {
        tracing::error!("read_system_timestamp");
        caller
            .user_data_mut()
            .runtime
            .read_system_timestamp()
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn load(caller: &mut Caller) -> Result<Vec<u8>, RuntimeError> {
        tracing::error!("load");
        caller
            .user_data_mut()
            .runtime
            .try_read_my_state()
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn load_and_lock(caller: &mut Caller) -> Result<Option<Vec<u8>>, RuntimeError> {
        tracing::error!("load_and_lock");
        caller
            .user_data_mut()
            .runtime
            .try_read_and_lock_my_state()
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn store_and_unlock(caller: &mut Caller, state: Vec<u8>) -> Result<bool, RuntimeError> {
        tracing::error!("store_and_unlock");
        caller
            .user_data_mut()
            .runtime
            .save_and_unlock_my_state(state)
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn lock_new(caller: &mut Caller) -> Result<u32, RuntimeError> {
        tracing::error!("lock_new");
        let mut data = caller.user_data_mut();
        let promise = data
            .runtime
            .lock_new()
            .map_err(|error| RuntimeError::Custom(error.into()))?;

        data.register_promise(promise)
    }

    fn lock_wait(caller: &mut Caller, promise_id: u32) -> Result<(), RuntimeError> {
        tracing::error!("lock_wait");
        let mut data = caller.user_data_mut();
        let promise = data.take_promise(promise_id)?;

        data.runtime
            .lock_wait(&promise)
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn try_call_application(
        caller: &mut Caller,
        authenticated: bool,
        callee_id: ApplicationId,
        argument: Vec<u8>,
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<CallResult, RuntimeError> {
        tracing::error!("try_call_application");
        caller
            .user_data_mut()
            .runtime
            .try_call_application(authenticated, callee_id, argument, forwarded_sessions)
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn try_call_session(
        caller: &mut Caller,
        authenticated: bool,
        session_id: SessionId,
        argument: Vec<u8>,
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<CallResult, RuntimeError> {
        tracing::error!("try_call_session");
        caller
            .user_data_mut()
            .runtime
            .try_call_session(authenticated, session_id, argument, forwarded_sessions)
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn log(_caller: &mut Caller, message: String, level: log::Level) -> Result<(), RuntimeError> {
        match level {
            log::Level::Trace => tracing::trace!("{message}"),
            log::Level::Debug => tracing::debug!("{message}"),
            log::Level::Info => tracing::info!("{message}"),
            log::Level::Warn => tracing::warn!("{message}"),
            log::Level::Error => tracing::error!("{message}"),
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct ServiceSystemApi<Caller>(PhantomData<Caller>);

#[linera_witty::wit_export(package = "linera:app")]
impl<Caller, Runtime> ServiceSystemApi<Caller>
where
    Caller: Instance<UserData = SystemApiData<Runtime>>,
    Runtime: ServiceRuntime + Send + 'static,
{
    fn get_chain_id(caller: &mut Caller) -> Result<ChainId, RuntimeError> {
        caller
            .user_data_mut()
            .runtime
            .chain_id()
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn get_application_id(caller: &mut Caller) -> Result<ApplicationId, RuntimeError> {
        caller
            .user_data_mut()
            .runtime
            .application_id()
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn get_application_parameters(caller: &mut Caller) -> Result<Vec<u8>, RuntimeError> {
        caller
            .user_data_mut()
            .runtime
            .application_parameters()
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn read_system_balance(caller: &mut Caller) -> Result<Amount, RuntimeError> {
        caller
            .user_data_mut()
            .runtime
            .read_system_balance()
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn read_system_timestamp(caller: &mut Caller) -> Result<Timestamp, RuntimeError> {
        caller
            .user_data_mut()
            .runtime
            .read_system_timestamp()
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    // TODO(#1152): Remove simple-storage APIs
    fn load_new(caller: &mut Caller) -> Result<u32, RuntimeError> {
        let mut data = caller.user_data_mut();
        let promise = data
            .runtime
            .try_read_my_state_new()
            .map_err(|error| RuntimeError::Custom(error.into()))?;

        data.register_promise(promise)
    }

    // TODO(#1152): Remove simple-storage APIs
    fn load_wait(
        caller: &mut Caller,
        promise_id: u32,
    ) -> Result<Result<Vec<u8>, String>, RuntimeError> {
        let mut data = caller.user_data_mut();
        let promise = data.take_promise(promise_id)?;

        data.runtime
            .try_read_my_state_wait(&promise)
            .map(Ok)
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn lock_new(caller: &mut Caller) -> Result<u32, RuntimeError> {
        let mut data = caller.user_data_mut();
        let promise = data
            .runtime
            .lock_new()
            .map_err(|error| RuntimeError::Custom(error.into()))?;

        data.register_promise(promise)
    }

    fn lock_wait(
        caller: &mut Caller,
        promise_id: u32,
    ) -> Result<Result<bool, String>, RuntimeError> {
        let mut data = caller.user_data_mut();
        let promise = data.take_promise(promise_id)?;

        data.runtime
            .lock_wait(&promise)
            .map(|()| Ok(true))
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn unlock_new(caller: &mut Caller) -> Result<u32, RuntimeError> {
        let mut data = caller.user_data_mut();
        let promise = data
            .runtime
            .unlock_new()
            .map_err(|error| RuntimeError::Custom(error.into()))?;

        data.register_promise(promise)
    }

    fn unlock_wait(
        caller: &mut Caller,
        promise_id: u32,
    ) -> Result<Result<bool, String>, RuntimeError> {
        let mut data = caller.user_data_mut();
        let promise = data.take_promise(promise_id)?;

        data.runtime
            .unlock_wait(&promise)
            .map(|()| Ok(true))
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn try_query_application_new(
        caller: &mut Caller,
        application: ApplicationId,
        argument: Vec<u8>,
    ) -> Result<u32, RuntimeError> {
        let mut data = caller.user_data_mut();
        let promise = data
            .runtime
            .try_query_application_new(application, argument)
            .map_err(|error| RuntimeError::Custom(error.into()))?;

        data.register_promise(promise)
    }

    fn try_query_application_wait(
        caller: &mut Caller,
        promise_id: u32,
    ) -> Result<Result<Vec<u8>, String>, RuntimeError> {
        let mut data = caller.user_data_mut();
        let promise = data.take_promise(promise_id)?;

        data.runtime
            .try_query_application_wait(&promise)
            .map(Ok)
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn log(_caller: &mut Caller, message: String, level: log::Level) -> Result<(), RuntimeError> {
        match level {
            log::Level::Trace => tracing::trace!("{message}"),
            log::Level::Debug => tracing::debug!("{message}"),
            log::Level::Info => tracing::info!("{message}"),
            log::Level::Warn => tracing::warn!("{message}"),
            log::Level::Error => tracing::error!("{message}"),
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct ViewSystemApi<Caller>(PhantomData<Caller>);

#[linera_witty::wit_export(package = "linera:app")]
impl<Caller, Runtime> ViewSystemApi<Caller>
where
    Caller: Instance<UserData = SystemApiData<Runtime>>,
    Runtime: BaseRuntime + Send + 'static,
{
    fn contains_key_new(caller: &mut Caller, key: Vec<u8>) -> Result<u32, RuntimeError> {
        tracing::error!("contains_key_new");
        let mut data = caller.user_data_mut();
        let promise = data
            .runtime
            .contains_key_new(key)
            .map_err(|error| RuntimeError::Custom(error.into()))?;

        data.register_promise(promise)
    }

    fn contains_key_wait(caller: &mut Caller, promise_id: u32) -> Result<bool, RuntimeError> {
        tracing::error!("contains_key_wait");
        let mut data = caller.user_data_mut();
        let promise = data.take_promise(promise_id)?;

        data.runtime
            .contains_key_wait(&promise)
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn read_multi_values_bytes_new(
        caller: &mut Caller,
        keys: Vec<Vec<u8>>,
    ) -> Result<u32, RuntimeError> {
        tracing::error!("read_multi_values_bytes_new");
        let mut data = caller.user_data_mut();
        let promise = data
            .runtime
            .read_multi_values_bytes_new(keys)
            .map_err(|error| RuntimeError::Custom(error.into()))?;

        data.register_promise(promise)
    }

    fn read_multi_values_bytes_wait(
        caller: &mut Caller,
        promise_id: u32,
    ) -> Result<Vec<Option<Vec<u8>>>, RuntimeError> {
        tracing::error!("read_multi_values_bytes_wait");
        let mut data = caller.user_data_mut();
        let promise = data.take_promise(promise_id)?;

        data.runtime
            .read_multi_values_bytes_wait(&promise)
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn read_value_bytes_new(caller: &mut Caller, key: Vec<u8>) -> Result<u32, RuntimeError> {
        tracing::error!("read_value_bytes_new");
        let mut data = caller.user_data_mut();
        let promise = data
            .runtime
            .read_value_bytes_new(key)
            .map_err(|error| RuntimeError::Custom(error.into()))?;

        data.register_promise(promise)
    }

    fn read_value_bytes_wait(
        caller: &mut Caller,
        promise_id: u32,
    ) -> Result<Option<Vec<u8>>, RuntimeError> {
        tracing::error!("read_value_bytes_wait");
        let mut data = caller.user_data_mut();
        let promise = data.take_promise(promise_id)?;

        data.runtime
            .read_value_bytes_wait(&promise)
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn find_keys_new(caller: &mut Caller, key_prefix: Vec<u8>) -> Result<u32, RuntimeError> {
        tracing::error!("find_keys_new");
        let mut data = caller.user_data_mut();
        let promise = data
            .runtime
            .find_keys_by_prefix_new(key_prefix)
            .map_err(|error| RuntimeError::Custom(error.into()))?;

        data.register_promise(promise)
    }

    fn find_keys_wait(caller: &mut Caller, promise_id: u32) -> Result<Vec<Vec<u8>>, RuntimeError> {
        tracing::error!("find_keys_wait");
        let mut data = caller.user_data_mut();
        let promise = data.take_promise(promise_id)?;

        data.runtime
            .find_keys_by_prefix_wait(&promise)
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn find_key_values_new(caller: &mut Caller, key_prefix: Vec<u8>) -> Result<u32, RuntimeError> {
        tracing::error!("find_key_values_new");
        let mut data = caller.user_data_mut();
        let promise = data
            .runtime
            .find_key_values_by_prefix_new(key_prefix)
            .map_err(|error| RuntimeError::Custom(error.into()))?;

        data.register_promise(promise)
    }

    #[allow(clippy::type_complexity)]
    fn find_key_values_wait(
        caller: &mut Caller,
        promise_id: u32,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>, RuntimeError> {
        tracing::error!("find_key_values_wait");
        let mut data = caller.user_data_mut();
        let promise = data.take_promise(promise_id)?;

        data.runtime
            .find_key_values_by_prefix_wait(&promise)
            .map_err(|error| RuntimeError::Custom(error.into()))
    }

    fn write_batch(
        caller: &mut Caller,
        operations: Vec<WriteOperation>,
    ) -> Result<(), RuntimeError> {
        tracing::error!("write_batch");
        caller
            .user_data_mut()
            .runtime
            .write_batch_and_unlock(Batch { operations })
            .map_err(|error| RuntimeError::Custom(error.into()))
    }
}

/// Generates an implementation of `ContractSystemApi` for the provided `contract_system_api` type.
///
/// Generates the common code for contract system API types for all Wasm runtimes.
macro_rules! impl_contract_system_api {
    ($trap:ty) => {
        impl<T: crate::ContractRuntime + Send + Sync + 'static>
            contract_system_api::ContractSystemApi for T
        {
            type Error = ExecutionError;

            fn error_to_trap(&mut self, error: Self::Error) -> $trap {
                error.into()
            }

            fn chain_id(&mut self) -> Result<contract_system_api::ChainId, Self::Error> {
                BaseRuntime::chain_id(self).map(|chain_id| chain_id.into())
            }

            fn application_id(
                &mut self,
            ) -> Result<contract_system_api::ApplicationId, Self::Error> {
                BaseRuntime::application_id(self).map(|application_id| application_id.into())
            }

            fn application_parameters(&mut self) -> Result<Vec<u8>, Self::Error> {
                BaseRuntime::application_parameters(self)
            }

            fn read_system_balance(&mut self) -> Result<contract_system_api::Amount, Self::Error> {
                BaseRuntime::read_system_balance(self).map(|balance| balance.into())
            }

            fn read_system_timestamp(
                &mut self,
            ) -> Result<contract_system_api::Timestamp, Self::Error> {
                BaseRuntime::read_system_timestamp(self).map(|timestamp| timestamp.micros())
            }

            fn try_call_application(
                &mut self,
                authenticated: bool,
                application: contract_system_api::ApplicationId,
                argument: &[u8],
                forwarded_sessions: &[Le<contract_system_api::SessionId>],
            ) -> Result<contract_system_api::CallOutcome, Self::Error> {
                let forwarded_sessions = forwarded_sessions
                    .iter()
                    .map(Le::get)
                    .map(SessionId::from)
                    .collect();

                ContractRuntime::try_call_application(
                    self,
                    authenticated,
                    application.into(),
                    argument.to_vec(),
                    forwarded_sessions,
                )
                .map(|call_outcome| call_outcome.into())
            }

            fn try_call_session(
                &mut self,
                authenticated: bool,
                session: contract_system_api::SessionId,
                argument: &[u8],
                forwarded_sessions: &[Le<contract_system_api::SessionId>],
            ) -> Result<contract_system_api::CallOutcome, Self::Error> {
                let forwarded_sessions = forwarded_sessions
                    .iter()
                    .map(Le::get)
                    .map(SessionId::from)
                    .collect();

                ContractRuntime::try_call_session(
                    self,
                    authenticated,
                    session.into(),
                    argument.to_vec(),
                    forwarded_sessions,
                )
                .map(|call_outcome| call_outcome.into())
            }

            fn log(
                &mut self,
                message: &str,
                level: contract_system_api::LogLevel,
            ) -> Result<(), Self::Error> {
                match level {
                    contract_system_api::LogLevel::Trace => tracing::trace!("{message}"),
                    contract_system_api::LogLevel::Debug => tracing::debug!("{message}"),
                    contract_system_api::LogLevel::Info => tracing::info!("{message}"),
                    contract_system_api::LogLevel::Warn => tracing::warn!("{message}"),
                    contract_system_api::LogLevel::Error => tracing::error!("{message}"),
                }
                Ok(())
            }
        }
    };
}

/// Generates an implementation of `ServiceSystemApi` for the provided `service_system_api` type.
///
/// Generates the common code for service system API types for all Wasm runtimes.
macro_rules! impl_service_system_api {
    ($trap:ty) => {
        impl<T: crate::ServiceRuntime + Send + Sync + 'static> service_system_api::ServiceSystemApi
            for T
        {
            type Error = ExecutionError;

            fn error_to_trap(&mut self, error: Self::Error) -> $trap {
                error.into()
            }

            fn chain_id(&mut self) -> Result<service_system_api::ChainId, Self::Error> {
                BaseRuntime::chain_id(self).map(|chain_id| chain_id.into())
            }

            fn application_id(&mut self) -> Result<service_system_api::ApplicationId, Self::Error> {
                BaseRuntime::application_id(self).map(|application_id| application_id.into())
            }

            fn application_parameters(&mut self) -> Result<Vec<u8>, Self::Error> {
                BaseRuntime::application_parameters(self)
            }

            fn read_system_balance(&mut self) -> Result<service_system_api::Amount, Self::Error> {
                BaseRuntime::read_system_balance(self).map(|balance| balance.into())
            }

            fn read_system_timestamp(
                &mut self,
            ) -> Result<service_system_api::Timestamp, Self::Error> {
                BaseRuntime::read_system_timestamp(self).map(|timestamp| timestamp.micros())
            }

            fn try_query_application(
                &mut self,
                application: service_system_api::ApplicationId,
                argument: &[u8],
            ) -> Result<Vec<u8>, Self::Error> {
                ServiceRuntime::try_query_application(self, application.into(), argument.to_vec())
            }

            fn log(
                &mut self,
                message: &str,
                level: service_system_api::LogLevel,
            ) -> Result<(), Self::Error> {
                match level {
                    service_system_api::LogLevel::Trace => tracing::trace!("{message}"),
                    service_system_api::LogLevel::Debug => tracing::debug!("{message}"),
                    service_system_api::LogLevel::Info => tracing::info!("{message}"),
                    service_system_api::LogLevel::Warn => tracing::warn!("{message}"),
                    service_system_api::LogLevel::Error => tracing::error!("{message}"),
                }

                Ok(())
            }
        }
    };
}

/// Generates an implementation of `ViewSystem` for the provided `view_system_api` type for
/// applications.
///
/// Generates the common code for view system API types for all WASM runtimes.
macro_rules! impl_view_system_api {
    ($trap:ty) => {
        impl<T: crate::BaseRuntime + Send + Sync + 'static> view_system_api::ViewSystemApi for T {
            type Error = ExecutionError;

            type ContainsKey = <Self as BaseRuntime>::ContainsKey;
            type ReadMultiValuesBytes = <Self as BaseRuntime>::ReadMultiValuesBytes;
            type ReadValueBytes = <Self as BaseRuntime>::ReadValueBytes;
            type FindKeys = <Self as BaseRuntime>::FindKeysByPrefix;
            type FindKeyValues = <Self as BaseRuntime>::FindKeyValuesByPrefix;

            fn error_to_trap(&mut self, error: Self::Error) -> $trap {
                error.into()
            }

            fn contains_key_new(&mut self, key: &[u8]) -> Result<Self::ContainsKey, Self::Error> {
                self.contains_key_new(key.to_vec())
            }

            fn contains_key_wait(
                &mut self,
                promise: &Self::ContainsKey,
            ) -> Result<bool, Self::Error> {
                self.contains_key_wait(promise)
            }

            fn read_multi_values_bytes_new(
                &mut self,
                keys: Vec<&[u8]>,
            ) -> Result<Self::ReadMultiValuesBytes, Self::Error> {
                let keys = keys.into_iter().map(Vec::from).collect();
                self.read_multi_values_bytes_new(keys)
            }

            fn read_multi_values_bytes_wait(
                &mut self,
                promise: &Self::ReadMultiValuesBytes,
            ) -> Result<Vec<Option<Vec<u8>>>, Self::Error> {
                self.read_multi_values_bytes_wait(promise)
            }

            fn read_value_bytes_new(
                &mut self,
                key: &[u8],
            ) -> Result<Self::ReadValueBytes, Self::Error> {
                self.read_value_bytes_new(key.to_vec())
            }

            fn read_value_bytes_wait(
                &mut self,
                promise: &Self::ReadValueBytes,
            ) -> Result<Option<Vec<u8>>, Self::Error> {
                self.read_value_bytes_wait(promise)
            }

            fn find_keys_new(&mut self, key_prefix: &[u8]) -> Result<Self::FindKeys, Self::Error> {
                self.find_keys_by_prefix_new(key_prefix.to_vec())
            }

            fn find_keys_wait(
                &mut self,
                promise: &Self::FindKeys,
            ) -> Result<Vec<Vec<u8>>, Self::Error> {
                self.find_keys_by_prefix_wait(promise)
            }

            fn find_key_values_new(
                &mut self,
                key_prefix: &[u8],
            ) -> Result<Self::FindKeyValues, Self::Error> {
                self.find_key_values_by_prefix_new(key_prefix.to_vec())
            }

            fn find_key_values_wait(
                &mut self,
                promise: &Self::FindKeyValues,
            ) -> Result<Vec<(Vec<u8>, Vec<u8>)>, Self::Error> {
                self.find_key_values_by_prefix_wait(promise)
            }

            // TODO(#1153): the wit name is wrong
            fn write_batch(
                &mut self,
                operations: Vec<view_system_api::WriteOperation>,
            ) -> Result<(), Self::Error> {
                let mut batch = linera_views::batch::Batch::new();
                for operation in operations {
                    match operation {
                        view_system_api::WriteOperation::Delete(key) => {
                            batch.delete_key(key.to_vec())
                        }
                        view_system_api::WriteOperation::Deleteprefix(key_prefix) => {
                            batch.delete_key_prefix(key_prefix.to_vec())
                        }
                        view_system_api::WriteOperation::Put((key, value)) => {
                            batch.put_key_value_bytes(key.to_vec(), value.to_vec())
                        }
                    }
                }
                // Hack: The following is a no-op for services.
                self.write_batch(batch)
            }
        }
    };
}
