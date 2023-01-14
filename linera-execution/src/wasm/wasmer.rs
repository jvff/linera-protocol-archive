// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

// SPDX-License-Identifier: Apache-2.0

//! Code specific to the usage of the [Wasmer](https://wasmer.io/) runtime.

// Export the writable system interface used by a user contract.
wit_bindgen_host_wasmer_rust::export!("../linera-sdk/writable_system.wit");

// Export the queryable system interface used by a user service.
wit_bindgen_host_wasmer_rust::export!("../linera-sdk/queryable_system.wit");

// Import the interface implemented by a user contract.
wit_bindgen_host_wasmer_rust::import!("../linera-sdk/contract.wit");

// Import the interface implemented by a user service.
wit_bindgen_host_wasmer_rust::import!("../linera-sdk/service.wit");

use self::{contract::Contract, service::Service};
use super::{
    async_boundary::{ContextForwarder, HostFuture, HostFutureQueue},
    common::{self, Runtime, WasmRuntimeContext},
    WasmApplication, WasmExecutionError,
};
use crate::{CallResult, ExecutionError, QueryableStorage, SessionId, WritableStorage};
use std::{marker::PhantomData, mem, sync::Arc, task::Poll};
use tokio::sync::Mutex;
use wasmer::{imports, Module, RuntimeError, Store};
use wit_bindgen_host_wasmer_rust::Le;
use linera_views::common::Batch;

/// Type representing the [Wasmer](https://wasmer.io/) contract runtime.
///
/// The runtime has a lifetime so that it does not outlive the trait object used to export the
/// system API.
pub struct ContractWasmer<'storage> {
    _lifetime: PhantomData<&'storage ()>,
}

impl<'storage> Runtime for ContractWasmer<'storage> {
    type Application = Contract;
    type Store = Store;
    type StorageGuard = StorageGuard<'storage, &'static dyn WritableStorage>;
    type Error = RuntimeError;
}

/// Type representing the [Wasmer](https://wasmer.io/) service runtime.
pub struct ServiceWasmer<'storage> {
    _lifetime: PhantomData<&'storage ()>,
}

impl<'storage> Runtime for ServiceWasmer<'storage> {
    type Application = Service;
    type Store = Store;
    type StorageGuard = StorageGuard<'storage, &'static dyn QueryableStorage>;
    type Error = RuntimeError;
}

impl WasmApplication {
    /// Prepare a runtime instance to call into the WASM contract.
    pub fn prepare_contract_runtime<'storage>(
        &self,
        storage: &'storage dyn WritableStorage,
    ) -> Result<WasmRuntimeContext<ContractWasmer<'storage>>, WasmExecutionError> {
        let mut store = Store::default();
        let module = Module::new(&store, &self.contract_bytecode)
            .map_err(wit_bindgen_host_wasmer_rust::anyhow::Error::from)?;
        let mut imports = imports! {};
        let context_forwarder = ContextForwarder::default();
        let (system_api, storage_guard) =
            SystemApi::new_writable(context_forwarder.clone(), storage);
        let system_api_setup =
            writable_system::add_to_imports(&mut store, &mut imports, system_api);
        let (application, instance) = Contract::instantiate(&mut store, &module, &mut imports)?;

        system_api_setup(&instance, &store)?;

        Ok(WasmRuntimeContext {
            context_forwarder,
            application,
            store,
            _storage_guard: storage_guard,
        })
    }

    /// Prepare a runtime instance to call into the WASM service.
    pub fn prepare_service_runtime<'storage>(
        &self,
        storage: &'storage dyn QueryableStorage,
    ) -> Result<WasmRuntimeContext<ServiceWasmer<'storage>>, WasmExecutionError> {
        let mut store = Store::default();
        let module = Module::new(&store, &self.service_bytecode)
            .map_err(wit_bindgen_host_wasmer_rust::anyhow::Error::from)?;
        let mut imports = imports! {};
        let context_forwarder = ContextForwarder::default();
        let (system_api, storage_guard) =
            SystemApi::new_queryable(context_forwarder.clone(), storage);
        let system_api_setup =
            queryable_system::add_to_imports(&mut store, &mut imports, system_api);
        let (application, instance) = Service::instantiate(&mut store, &module, &mut imports)?;

        system_api_setup(&instance, &store)?;

        Ok(WasmRuntimeContext {
            context_forwarder,
            application,
            store,
            _storage_guard: storage_guard,
        })
    }
}

impl<'storage> common::Contract<ContractWasmer<'storage>> for Contract {
    fn initialize_new(
        &self,
        store: &mut Store,
        context: contract::OperationContext,
        argument: &[u8],
    ) -> Result<contract::Initialize, RuntimeError> {
        Self::initialize_new(self, store, context, argument)
    }

    fn initialize_poll(
        &self,
        store: &mut Store,
        future: &contract::Initialize,
    ) -> Result<contract::PollExecutionResult, RuntimeError> {
        Self::initialize_poll(self, store, future)
    }

    fn execute_operation_new(
        &self,
        store: &mut Store,
        context: contract::OperationContext,
        operation: &[u8],
    ) -> Result<contract::ExecuteOperation, RuntimeError> {
        Self::execute_operation_new(self, store, context, operation)
    }

    fn execute_operation_poll(
        &self,
        store: &mut Store,
        future: &contract::ExecuteOperation,
    ) -> Result<contract::PollExecutionResult, RuntimeError> {
        Self::execute_operation_poll(self, store, future)
    }

    fn execute_effect_new(
        &self,
        store: &mut Store,
        context: contract::EffectContext,
        effect: &[u8],
    ) -> Result<contract::ExecuteEffect, RuntimeError> {
        Self::execute_effect_new(self, store, context, effect)
    }

    fn execute_effect_poll(
        &self,
        store: &mut Store,
        future: &contract::ExecuteEffect,
    ) -> Result<contract::PollExecutionResult, RuntimeError> {
        Self::execute_effect_poll(self, store, future)
    }

    fn call_application_new(
        &self,
        store: &mut Store,
        context: contract::CalleeContext,
        argument: &[u8],
        forwarded_sessions: &[contract::SessionId],
    ) -> Result<contract::CallApplication, RuntimeError> {
        Self::call_application_new(self, store, context, argument, forwarded_sessions)
    }

    fn call_application_poll(
        &self,
        store: &mut Store,
        future: &contract::CallApplication,
    ) -> Result<contract::PollCallApplication, RuntimeError> {
        Self::call_application_poll(self, store, future)
    }

    fn call_session_new(
        &self,
        store: &mut Store,
        context: contract::CalleeContext,
        session: contract::SessionParam,
        argument: &[u8],
        forwarded_sessions: &[contract::SessionId],
    ) -> Result<contract::CallSession, RuntimeError> {
        Self::call_session_new(self, store, context, session, argument, forwarded_sessions)
    }

    fn call_session_poll(
        &self,
        store: &mut Store,
        future: &contract::CallSession,
    ) -> Result<contract::PollCallSession, RuntimeError> {
        Self::call_session_poll(self, store, future)
    }
}

impl<'storage> common::Service<ServiceWasmer<'storage>> for Service {
    fn query_application_new(
        &self,
        store: &mut Store,
        context: service::QueryContext,
        argument: &[u8],
    ) -> Result<service::QueryApplication, RuntimeError> {
        Self::query_application_new(self, store, context, argument)
    }

    fn query_application_poll(
        &self,
        store: &mut Store,
        future: &service::QueryApplication,
    ) -> Result<service::PollQuery, RuntimeError> {
        Self::query_application_poll(self, store, future)
    }
}

/// Implementation to forward system calls from the guest WASM module to the host implementation.
pub struct SystemApi<S> {
    context: ContextForwarder,
    storage: Arc<Mutex<Option<S>>>,
    future_queue: HostFutureQueue,
}

impl SystemApi<&'static dyn WritableStorage> {
    /// Create a new [`SystemApi`] instance, ensuring that the lifetime of the [`WritableStorage`]
    /// trait object is respected.
    ///
    /// # Safety
    ///
    /// This method uses a [`mem::transmute`] call to erase the lifetime of the `storage` trait
    /// object reference. However, this is safe because the lifetime is transfered to the returned
    /// [`StorageGuard`], which removes the unsafe reference from memory when it is dropped,
    /// ensuring the lifetime is respected.
    ///
    /// The [`StorageGuard`] instance must be kept alive while the trait object is still expected to
    /// be alive and usable by the WASM application.
    pub fn new_writable(
        context: ContextForwarder,
        storage: &dyn WritableStorage,
    ) -> (Self, StorageGuard<'_, &'static dyn WritableStorage>) {
        let storage_without_lifetime = unsafe { mem::transmute(storage) };
        let storage = Arc::new(Mutex::new(Some(storage_without_lifetime)));

        let guard = StorageGuard {
            storage: storage.clone(),
            _lifetime: PhantomData,
        };

        (
            SystemApi {
                context,
                storage,
                future_queue: HostFutureQueue::new(),
            },
            guard,
        )
    }
}

impl SystemApi<&'static dyn QueryableStorage> {
    /// Same as `new_writable`. Didn't find how to factorizing the code.
    pub fn new_queryable(
        context: ContextForwarder,
        storage: &dyn QueryableStorage,
    ) -> (Self, StorageGuard<'_, &'static dyn QueryableStorage>) {
        let storage_without_lifetime = unsafe { mem::transmute(storage) };
        let storage = Arc::new(Mutex::new(Some(storage_without_lifetime)));

        let guard = StorageGuard {
            storage: storage.clone(),
            _lifetime: PhantomData,
        };

        (
            SystemApi {
                context,
                storage,
                future_queue: HostFutureQueue::new(),
            },
            guard,
        )
    }
}

impl<S: Copy> SystemApi<S> {
    /// Safely obtain the [`WritableStorage`] trait object instance to handle a system call.
    ///
    /// # Panics
    ///
    /// If there is a concurrent call from the WASM application (which is impossible as long as it
    /// is executed in a single thread) or if the trait object is no longer alive (or more
    /// accurately, if the [`StorageGuard`] returned by [`Self::new`] was dropped to indicate it's
    /// no longer alive).
    fn storage(&self) -> S {
        *self
            .storage
            .try_lock()
            .expect("Unexpected concurrent storage access by application")
            .as_ref()
            .expect("Application called storage after it should have stopped")
    }
}

impl writable_system::WritableSystem for SystemApi<&'static dyn WritableStorage> {
    type Load = HostFuture<'static, Result<Vec<u8>, ExecutionError>>;
    type Lock = HostFuture<'static, Result<(), ExecutionError>>;
    type LoadAndLock = HostFuture<'static, Result<Vec<u8>, ExecutionError>>;
    type ReadKeyBytes = HostFuture<'static, Result<Option<Vec<u8>>, ExecutionError>>;
    type FindStrippedKeys = HostFuture<'static, Result<Vec<Vec<u8>>, ExecutionError>>;
    type FindStrippedKeyValues = HostFuture<'static, Result<Vec<(Vec<u8>,Vec<u8>)>, ExecutionError>>;
    type WriteBatch = HostFuture<'static, Result<(), ExecutionError>>;
    type TryCallApplication = HostFuture<'static, Result<CallResult, ExecutionError>>;
    type TryCallSession = HostFuture<'static, Result<CallResult, ExecutionError>>;

    fn chain_id(&mut self) -> writable_system::ChainId {
        self.storage().chain_id().into()
    }

    fn application_id(&mut self) -> writable_system::ApplicationId {
        self.storage().application_id().into()
    }

    fn read_system_balance(&mut self) -> writable_system::SystemBalance {
        self.storage().read_system_balance().into()
    }

    fn read_system_timestamp(&mut self) -> writable_system::Timestamp {
        self.storage().read_system_timestamp().micros()
    }

    fn load_new(&mut self) -> Self::Load {
        self.future_queue.add(self.storage().try_read_my_state())
    }

    fn load_poll(&mut self, future: &Self::Load) -> writable_system::PollLoad {
        use writable_system::PollLoad;
        match future.poll(&mut self.context) {
            Poll::Pending => PollLoad::Pending,
            Poll::Ready(Ok(bytes)) => PollLoad::Ready(Ok(bytes)),
            Poll::Ready(Err(error)) => PollLoad::Ready(Err(error.to_string())),
        }
    }

    fn load_and_lock_new(&mut self) -> Self::LoadAndLock {
        self.future_queue
            .add(self.storage().try_read_and_lock_my_state())
    }

    fn load_and_lock_poll(&mut self, future: &Self::LoadAndLock) -> writable_system::PollLoad {
        use writable_system::PollLoad;
        match future.poll(&mut self.context) {
            Poll::Pending => PollLoad::Pending,
            Poll::Ready(Ok(bytes)) => PollLoad::Ready(Ok(bytes)),
            Poll::Ready(Err(error)) => PollLoad::Ready(Err(error.to_string())),
        }
    }

    fn lock_new(&mut self) -> Self::Lock {
        HostFuture::new(self.storage().lock_userkv_state())
    }

    fn lock_poll(&mut self, future: &Self::Lock) -> writable_system::PollLock {
        use writable_system::PollLock;
        match future.poll(&mut self.context) {
            Poll::Pending => PollLock::Pending,
            Poll::Ready(Ok(())) => PollLock::Ready(Ok(())),
            Poll::Ready(Err(error)) => PollLock::Ready(Err(error.to_string())),
        }
    }

    fn read_key_bytes_new(&mut self, key: &[u8]) -> Self::ReadKeyBytes {
        HostFuture::new(self.storage().pass_userkv_read_key_bytes(key.to_owned()))
    }

    fn read_key_bytes_poll(&mut self, future: &Self::ReadKeyBytes) -> writable_system::PollReadKeyBytes {
        use writable_system::PollReadKeyBytes;
        match future.poll(&mut self.context) {
            Poll::Pending => PollReadKeyBytes::Pending,
            Poll::Ready(Ok(opt_list)) => PollReadKeyBytes::Ready(Ok(opt_list)),
            Poll::Ready(Err(error)) => PollReadKeyBytes::Ready(Err(error.to_string())),
        }
    }

    fn find_stripped_keys_new(&mut self, key_prefix: &[u8]) -> Self::FindStrippedKeys {
        HostFuture::new(self.storage().pass_userkv_find_stripped_keys_by_prefix(key_prefix.to_owned()))
    }

    fn find_stripped_keys_poll(&mut self, future: &Self::FindStrippedKeys) -> writable_system::PollFindStrippedKeys {
        use writable_system::PollFindStrippedKeys;
        match future.poll(&mut self.context) {
            Poll::Pending => PollFindStrippedKeys::Pending,
            Poll::Ready(Ok(list_stripped_keys)) => PollFindStrippedKeys::Ready(Ok(list_stripped_keys)),
            Poll::Ready(Err(error)) => PollFindStrippedKeys::Ready(Err(error.to_string())),
        }
    }

    fn find_stripped_key_values_new(&mut self, key_prefix: &[u8]) -> Self::FindStrippedKeyValues {
        HostFuture::new(self.storage().pass_userkv_find_stripped_key_values_by_prefix(key_prefix.to_owned()))
    }

    fn find_stripped_key_values_poll(&mut self, future: &Self::FindStrippedKeyValues) -> writable_system::PollFindStrippedKeyValues {
        use writable_system::PollFindStrippedKeyValues;
        match future.poll(&mut self.context) {
            Poll::Pending => PollFindStrippedKeyValues::Pending,
            Poll::Ready(Ok(list_stripped_key_values)) => PollFindStrippedKeyValues::Ready(Ok(list_stripped_key_values)),
            Poll::Ready(Err(error)) => PollFindStrippedKeyValues::Ready(Err(error.to_string())),
        }
    }

    fn store_and_unlock(&mut self, state: &[u8]) -> bool {
        self.storage()
            .save_and_unlock_my_state(state.to_owned())
            .is_ok()
    }

    fn write_batch_new(&mut self, list_oper: Vec<writable_system::WriteOperation>) -> Self::WriteBatch {
        let mut batch = Batch::default();
        for x in list_oper {
            match x {
                writable_system::WriteOperation::Delete(key) => batch.delete_key(key.to_vec()),
                writable_system::WriteOperation::Deleteprefix(key_prefix) => batch.delete_key_prefix(key_prefix.to_vec()),
                writable_system::WriteOperation::Put(key_value) => batch.put_key_value_bytes(key_value.0.to_vec(), key_value.1.to_vec()),
            }
        }
        HostFuture::new(self.storage().write_batch_and_unlock(batch))
    }

    fn write_batch_poll(&mut self, future: &Self::WriteBatch) -> writable_system::PollWriteBatch {
        use writable_system::PollWriteBatch;
        match future.poll(&mut self.context) {
            Poll::Pending => PollWriteBatch::Pending,
            Poll::Ready(Ok(())) => PollWriteBatch::Ready(Ok(())),
            Poll::Ready(Err(error)) => PollWriteBatch::Ready(Err(error.to_string())),
        }
    }

    fn try_call_application_new(
        &mut self,
        authenticated: bool,
        application: writable_system::ApplicationId,
        argument: &[u8],
        forwarded_sessions: &[Le<writable_system::SessionId>],
    ) -> Self::TryCallApplication {
        let storage = self.storage();
        let forwarded_sessions = forwarded_sessions
            .iter()
            .map(Le::get)
            .map(SessionId::from)
            .collect();
        let argument = Vec::from(argument);

        self.future_queue.add(async move {
            storage
                .try_call_application(
                    authenticated,
                    application.into(),
                    &argument,
                    forwarded_sessions,
                )
                .await
        })
    }

    fn try_call_application_poll(
        &mut self,
        future: &Self::TryCallApplication,
    ) -> writable_system::PollCallResult {
        use writable_system::PollCallResult;
        match future.poll(&mut self.context) {
            Poll::Pending => PollCallResult::Pending,
            Poll::Ready(Ok(result)) => PollCallResult::Ready(Ok(result.into())),
            Poll::Ready(Err(error)) => PollCallResult::Ready(Err(error.to_string())),
        }
    }

    fn try_call_session_new(
        &mut self,
        authenticated: bool,
        session: writable_system::SessionId,
        argument: &[u8],
        forwarded_sessions: &[Le<writable_system::SessionId>],
    ) -> Self::TryCallApplication {
        let storage = self.storage();
        let forwarded_sessions = forwarded_sessions
            .iter()
            .map(Le::get)
            .map(SessionId::from)
            .collect();
        let argument = Vec::from(argument);

        self.future_queue.add(async move {
            storage
                .try_call_session(authenticated, session.into(), &argument, forwarded_sessions)
                .await
        })
    }

    fn try_call_session_poll(
        &mut self,
        future: &Self::TryCallApplication,
    ) -> writable_system::PollCallResult {
        use writable_system::PollCallResult;
        match future.poll(&mut self.context) {
            Poll::Pending => PollCallResult::Pending,
            Poll::Ready(Ok(result)) => PollCallResult::Ready(Ok(result.into())),
            Poll::Ready(Err(error)) => PollCallResult::Ready(Err(error.to_string())),
        }
    }
}

impl queryable_system::QueryableSystem for SystemApi<&'static dyn QueryableStorage> {
    type Load = HostFuture<'static, Result<Vec<u8>, ExecutionError>>;
    type Lock = HostFuture<'static, Result<(), ExecutionError>>;
    type Unlock = HostFuture<'static, Result<(), ExecutionError>>;
    type ReadKeyBytes = HostFuture<'static, Result<Option<Vec<u8>>, ExecutionError>>;
    type FindStrippedKeys = HostFuture<'static, Result<Vec<Vec<u8>>, ExecutionError>>;
    type FindStrippedKeyValues = HostFuture<'static, Result<Vec<(Vec<u8>,Vec<u8>)>, ExecutionError>>;

    fn chain_id(&mut self) -> queryable_system::ChainId {
        self.storage().chain_id().into()
    }

    fn application_id(&mut self) -> queryable_system::ApplicationId {
        self.storage().application_id().into()
    }

    fn read_system_balance(&mut self) -> queryable_system::SystemBalance {
        self.storage().read_system_balance().into()
    }

    fn read_system_timestamp(&mut self) -> queryable_system::Timestamp {
        self.storage().read_system_timestamp().micros()
    }

    fn load_new(&mut self) -> Self::Load {
        self.future_queue.add(self.storage().try_read_my_state())
    }

    fn load_poll(&mut self, future: &Self::Load) -> queryable_system::PollLoad {
        use queryable_system::PollLoad;
        match future.poll(&mut self.context) {
            Poll::Pending => PollLoad::Pending,
            Poll::Ready(Ok(bytes)) => PollLoad::Ready(Ok(bytes)),
            Poll::Ready(Err(error)) => PollLoad::Ready(Err(error.to_string())),
        }
    }

    fn lock_new(&mut self) -> Self::Lock {
        HostFuture::new(self.storage().lock_userkv_state())
    }

    fn lock_poll(&mut self, future: &Self::Lock) -> queryable_system::PollLock {
        use queryable_system::PollLock;
        match future.poll(&mut self.context) {
            Poll::Pending => PollLock::Pending,
            Poll::Ready(Ok(())) => PollLock::Ready(Ok(())),
            Poll::Ready(Err(error)) => PollLock::Ready(Err(error.to_string())),
        }
    }

    fn unlock_new(&mut self) -> Self::Unlock {
        HostFuture::new(self.storage().unlock_userkv_state())
    }

    fn unlock_poll(&mut self, future: &Self::Lock) -> queryable_system::PollUnlock {
        use queryable_system::PollUnlock;
        match future.poll(&mut self.context) {
            Poll::Pending => PollUnlock::Pending,
            Poll::Ready(Ok(())) => PollUnlock::Ready(Ok(())),
            Poll::Ready(Err(error)) => PollUnlock::Ready(Err(error.to_string())),
        }
    }

    fn read_key_bytes_new(&mut self, key: &[u8]) -> Self::ReadKeyBytes {
        HostFuture::new(self.storage().pass_userkv_read_key_bytes(key.to_owned()))
    }

    fn read_key_bytes_poll(&mut self, future: &Self::ReadKeyBytes) -> queryable_system::PollReadKeyBytes {
        use queryable_system::PollReadKeyBytes;
        match future.poll(&mut self.context) {
            Poll::Pending => PollReadKeyBytes::Pending,
            Poll::Ready(Ok(opt_list)) => PollReadKeyBytes::Ready(Ok(opt_list)),
            Poll::Ready(Err(error)) => PollReadKeyBytes::Ready(Err(error.to_string())),
        }
    }

    fn find_stripped_keys_new(&mut self, key_prefix: &[u8]) -> Self::FindStrippedKeys {
        HostFuture::new(self.storage().pass_userkv_find_stripped_keys_by_prefix(key_prefix.to_owned()))
    }

    fn find_stripped_keys_poll(&mut self, future: &Self::FindStrippedKeys) -> queryable_system::PollFindStrippedKeys {
        use queryable_system::PollFindStrippedKeys;
        match future.poll(&mut self.context) {
            Poll::Pending => PollFindStrippedKeys::Pending,
            Poll::Ready(Ok(list_stripped_keys)) => PollFindStrippedKeys::Ready(Ok(list_stripped_keys)),
            Poll::Ready(Err(error)) => PollFindStrippedKeys::Ready(Err(error.to_string())),
        }
    }

    fn find_stripped_key_values_new(&mut self, key_prefix: &[u8]) -> Self::FindStrippedKeyValues {
        HostFuture::new(self.storage().pass_userkv_find_stripped_key_values_by_prefix(key_prefix.to_owned()))
    }

    fn find_stripped_key_values_poll(&mut self, future: &Self::FindStrippedKeyValues) -> queryable_system::PollFindStrippedKeyValues {
        use queryable_system::PollFindStrippedKeyValues;
        match future.poll(&mut self.context) {
            Poll::Pending => PollFindStrippedKeyValues::Pending,
            Poll::Ready(Ok(list_stripped_key_values)) => PollFindStrippedKeyValues::Ready(Ok(list_stripped_key_values)),
            Poll::Ready(Err(error)) => PollFindStrippedKeyValues::Ready(Err(error.to_string())),
        }
    }
}

/// A guard to unsure that the [`WritableStorage`] trait object isn't called after it's no longer
/// borrowed.
pub struct StorageGuard<'storage, S> {
    storage: Arc<Mutex<Option<S>>>,
    _lifetime: PhantomData<&'storage ()>,
}

impl<S> Drop for StorageGuard<'_, S> {
    fn drop(&mut self) {
        self.storage
            .try_lock()
            .expect("Guard dropped while storage is still in use")
            .take();
    }
}
