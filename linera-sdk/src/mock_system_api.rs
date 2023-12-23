// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Hooks for mocking system APIs inside unit tests.

#![allow(missing_docs)]

use anyhow::anyhow;
use linera_base::{
    data_types::{Amount, Timestamp},
    ensure,
    identifiers::{ApplicationId, BytecodeId, ChainId, MessageId},
};
use linera_views::batch::WriteOperation;
use linera_witty::{Instance, Runtime, RuntimeError, RuntimeMemory};
use std::{any::Any, marker::PhantomData};

/// A map of resources allocated on the host side.
#[derive(Default)]
pub struct Resources(Vec<Box<dyn Any + Send + 'static>>);

impl Resources {
    /// Adds a resource to the map, returning its handle.
    pub fn insert(&mut self, value: impl Any + Send + 'static) -> i32 {
        let handle = self.0.len().try_into().expect("Resources map overflow");

        self.0.push(Box::new(value));

        handle
    }

    /// Returns an immutable reference to a resource referenced by the provided `handle`.
    pub fn get<T: 'static>(&self, handle: i32) -> &T {
        self.0[usize::try_from(handle).expect("Invalid handle")]
            .downcast_ref()
            .expect("Incorrect handle type")
    }
}

/// A resource representing a query.
#[derive(Clone)]
struct Query {
    application_id: ApplicationId,
    query: Vec<u8>,
}

#[linera_witty::wit_import(package = "linera:app")]
pub trait MockSystemApi {
    fn mocked_chain_id() -> ChainId;
    fn mocked_application_id() -> ApplicationId;
    fn mocked_application_parameters() -> Vec<u8>;
    fn mocked_read_system_balance() -> Amount;
    fn mocked_read_system_timestamp() -> Timestamp;
    fn mocked_log(message: String, level: log::Level);
    fn mocked_load() -> Vec<u8>;
    fn mocked_load_and_lock() -> Option<Vec<u8>>;
    fn mocked_store_and_unlock(value: Vec<u8>) -> bool;
    fn mocked_lock() -> bool;
    fn mocked_unlock() -> bool;
    fn mocked_read_multi_values_bytes(keys: Vec<Vec<u8>>) -> Vec<Option<Vec<u8>>>;
    fn mocked_read_value_bytes(key: Vec<u8>) -> Option<Vec<u8>>;
    fn mocked_find_keys(prefix: Vec<u8>) -> Vec<Vec<u8>>;
    fn mocked_find_key_values(prefix: Vec<u8>) -> Vec<(Vec<u8>, Vec<u8>)>;
    fn mocked_write_batch(operations: Vec<WriteOperation>);
    fn mocked_try_query_application(
        application: ApplicationId,
        query: Vec<u8>,
    ) -> Result<Vec<u8>, String>;
}

#[derive(Default)]
pub struct ContractSystemApi<Caller>(PhantomData<Caller>);

#[linera_witty::wit_export(package = "linera:app")]
impl<Caller> ContractSystemApi<Caller>
where
    Caller: Instance<UserData = Resources> + InstanceForMockSystemApi,
    <Caller::Runtime as Runtime>::Memory: RuntimeMemory<Caller>,
{
    fn get_chain_id(caller: &mut Caller) -> Result<ChainId, RuntimeError> {
        MockSystemApi::new(caller).mocked_chain_id()
    }

    fn get_application_id(caller: &mut Caller) -> Result<ApplicationId, RuntimeError> {
        MockSystemApi::new(caller).mocked_application_id()
    }

    fn get_application_parameters(caller: &mut Caller) -> Result<Vec<u8>, RuntimeError> {
        MockSystemApi::new(caller).mocked_application_parameters()
    }

    fn read_system_balance(caller: &mut Caller) -> Result<Amount, RuntimeError> {
        MockSystemApi::new(caller).mocked_read_system_balance()
    }

    fn read_system_timestamp(caller: &mut Caller) -> Result<Timestamp, RuntimeError> {
        MockSystemApi::new(caller).mocked_read_system_timestamp()
    }

    fn load(caller: &mut Caller) -> Result<Vec<u8>, RuntimeError> {
        MockSystemApi::new(caller).mocked_load()
    }

    fn load_and_lock(caller: &mut Caller) -> Result<Option<Vec<u8>>, RuntimeError> {
        MockSystemApi::new(caller).mocked_load_and_lock()
    }

    fn store_and_unlock(caller: &mut Caller, state: Vec<u8>) -> Result<bool, RuntimeError> {
        MockSystemApi::new(caller).mocked_store_and_unlock(state)
    }

    fn lock_new(_caller: &mut Caller) -> Result<u32, RuntimeError> {
        Ok(0)
    }

    fn lock_wait(caller: &mut Caller, _promise_id: u32) -> Result<(), RuntimeError> {
        ensure!(
            MockSystemApi::new(caller).mocked_lock()?,
            RuntimeError::Custom(anyhow!("`mocked_lock` function returned a failure"))
        );
        Ok(())
    }

    fn log(caller: &mut Caller, message: String, level: log::Level) -> Result<(), RuntimeError> {
        MockSystemApi::new(caller).mocked_log(message, level)
    }
}

#[derive(Default)]
pub struct ServiceSystemApi<Caller>(PhantomData<Caller>);

#[linera_witty::wit_export(package = "linera:app")]
impl<Caller> ServiceSystemApi<Caller>
where
    Caller: Instance<UserData = Resources> + InstanceForMockSystemApi,
    <Caller::Runtime as Runtime>::Memory: RuntimeMemory<Caller>,
{
    fn get_chain_id(caller: &mut Caller) -> Result<ChainId, RuntimeError> {
        MockSystemApi::new(caller).mocked_chain_id()
    }

    fn get_application_id(caller: &mut Caller) -> Result<ApplicationId, RuntimeError> {
        MockSystemApi::new(caller).mocked_application_id()
    }

    fn get_application_parameters(caller: &mut Caller) -> Result<Vec<u8>, RuntimeError> {
        MockSystemApi::new(caller).mocked_application_parameters()
    }

    fn read_system_balance(caller: &mut Caller) -> Result<Amount, RuntimeError> {
        MockSystemApi::new(caller).mocked_read_system_balance()
    }

    fn read_system_timestamp(caller: &mut Caller) -> Result<Timestamp, RuntimeError> {
        MockSystemApi::new(caller).mocked_read_system_timestamp()
    }

    fn load_new(_caller: &mut Caller) -> Result<u32, RuntimeError> {
        Ok(0)
    }

    fn load_wait(
        caller: &mut Caller,
        _promise_id: u32,
    ) -> Result<Result<Vec<u8>, String>, RuntimeError> {
        MockSystemApi::new(caller).mocked_load().map(Ok)
    }

    fn lock_new(_caller: &mut Caller) -> Result<u32, RuntimeError> {
        Ok(0)
    }

    fn lock_wait(
        caller: &mut Caller,
        _promise_id: u32,
    ) -> Result<Result<bool, String>, RuntimeError> {
        ensure!(
            MockSystemApi::new(caller).mocked_lock()?,
            RuntimeError::Custom(anyhow!("`mocked_lock` function returned a failure"))
        );
        Ok(Ok(true))
    }

    fn try_query_application_new(
        caller: &mut Caller,
        application_id: ApplicationId,
        query: Vec<u8>,
    ) -> Result<u32, RuntimeError> {
        let resource = Query {
            application_id,
            query,
        };

        Ok(caller.user_data_mut().insert(resource) as u32)
    }

    fn try_query_application_wait(
        caller: &mut Caller,
        promise_id: u32,
    ) -> Result<Result<Vec<u8>, String>, RuntimeError> {
        let resource = caller
            .user_data_mut()
            .get::<Query>(promise_id as i32)
            .clone();

        MockSystemApi::new(caller)
            .mocked_try_query_application(resource.application_id, resource.query)
    }

    fn log(caller: &mut Caller, message: String, level: log::Level) -> Result<(), RuntimeError> {
        MockSystemApi::new(caller).mocked_log(message, level)
    }
}

#[derive(Default)]
pub struct ViewSystemApi<Caller>(PhantomData<Caller>);

#[linera_witty::wit_export(package = "linera:app")]
impl<Caller> ViewSystemApi<Caller>
where
    Caller: Instance<UserData = Resources> + InstanceForMockSystemApi,
    <Caller::Runtime as Runtime>::Memory: RuntimeMemory<Caller>,
{
    fn read_multi_values_bytes_new(
        caller: &mut Caller,
        keys: Vec<Vec<u8>>,
    ) -> Result<u32, RuntimeError> {
        Ok(caller.user_data_mut().insert(keys) as u32)
    }

    fn read_multi_values_bytes_wait(
        caller: &mut Caller,
        promise_id: u32,
    ) -> Result<Vec<Option<Vec<u8>>>, RuntimeError> {
        let keys = caller
            .user_data_mut()
            .get::<Vec<Vec<u8>>>(promise_id as i32)
            .clone();

        MockSystemApi::new(caller).mocked_read_multi_values_bytes(keys)
    }

    fn read_value_bytes_new(caller: &mut Caller, key: Vec<u8>) -> Result<u32, RuntimeError> {
        Ok(caller.user_data_mut().insert(key) as u32)
    }

    fn read_value_bytes_wait(
        caller: &mut Caller,
        promise_id: u32,
    ) -> Result<Option<Vec<u8>>, RuntimeError> {
        let key = caller
            .user_data_mut()
            .get::<Vec<u8>>(promise_id as i32)
            .clone();

        MockSystemApi::new(caller).mocked_read_value_bytes(key)
    }

    fn find_keys_new(caller: &mut Caller, prefix: Vec<u8>) -> Result<u32, RuntimeError> {
        Ok(caller.user_data_mut().insert(prefix) as u32)
    }

    fn find_keys_wait(caller: &mut Caller, promise_id: u32) -> Result<Vec<Vec<u8>>, RuntimeError> {
        let prefix = caller
            .user_data_mut()
            .get::<Vec<u8>>(promise_id as i32)
            .clone();

        MockSystemApi::new(caller).mocked_find_keys(prefix)
    }

    fn find_key_values_new(caller: &mut Caller, prefix: Vec<u8>) -> Result<u32, RuntimeError> {
        Ok(caller.user_data_mut().insert(prefix) as u32)
    }

    #[allow(clippy::type_complexity)]
    fn find_key_values_wait(
        caller: &mut Caller,
        promise_id: u32,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>, RuntimeError> {
        let prefix = caller
            .user_data_mut()
            .get::<Vec<u8>>(promise_id as i32)
            .clone();

        MockSystemApi::new(caller).mocked_find_key_values(prefix)
    }

    fn write_batch(
        caller: &mut Caller,
        operations: Vec<WriteOperation>,
    ) -> Result<(), RuntimeError> {
        MockSystemApi::new(caller).mocked_write_batch(operations)
    }
}
