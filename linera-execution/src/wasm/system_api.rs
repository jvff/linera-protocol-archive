// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Generates an implementation of `ContractSystemApi` for the provided `contract_system_api` type.
///
/// Generates the common code for contract system API types for all Wasm runtimes.
macro_rules! impl_contract_system_api {
    ($contract_system_api:ident<$runtime:lifetime>) => {
        impl_contract_system_api!(
            @generate $contract_system_api<$runtime>, wasmtime::Trap, $runtime, <$runtime>
        );
    };

    ($contract_system_api:ident) => {
        impl_contract_system_api!(@generate $contract_system_api, wasmer::RuntimeError, 'static);
    };

    (@generate $contract_system_api:ty, $trap:ty, $runtime:lifetime $(, <$param:lifetime> )?) => {
        impl$(<$param>)? contract_system_api::ContractSystemApi for $contract_system_api {
            type Error = ExecutionError;

            type Lock = Mutex<oneshot::Receiver<Result<(), ExecutionError>>>;

            fn error_to_trap(&mut self, error: Self::Error) -> $trap {
                error.into()
            }

            fn chain_id(&mut self) -> Result<contract_system_api::ChainId, Self::Error> {
                Ok(self
                    .runtime
                    .sync_request(|response| {
                        ContractRequest::Base(BaseRequest::ChainId { response })
                    })
                    .into())
            }

            fn application_id(
                &mut self,
            ) -> Result<contract_system_api::ApplicationId, Self::Error> {
                Ok(self
                    .runtime
                    .sync_request(|response| {
                        ContractRequest::Base(BaseRequest::ApplicationId { response })
                    })
                    .into())
            }

            fn application_parameters(&mut self) -> Result<Vec<u8>, Self::Error> {
                Ok(self.runtime.sync_request(|response| {
                    ContractRequest::Base(BaseRequest::ApplicationParameters { response })
                }))
            }

            fn read_system_balance(
                &mut self,
            ) -> Result<contract_system_api::Amount, Self::Error> {
                Ok(self
                    .runtime
                    .sync_request(|response| {
                        ContractRequest::Base(BaseRequest::ReadSystemBalance { response })
                    })
                    .into())
            }

            fn read_system_timestamp(
                &mut self,
            ) -> Result<contract_system_api::Timestamp, Self::Error> {
                Ok(self
                    .runtime
                    .sync_request(|response| {
                        ContractRequest::Base(BaseRequest::ReadSystemTimestamp { response })
                    })
                    .micros())
            }

            fn load(&mut self) -> Result<Vec<u8>, Self::Error> {
                Ok(self.runtime.sync_request(|response| {
                    ContractRequest::Base(BaseRequest::TryReadMyState {
                        response: response.into(),
                    })
                }))
            }

            fn load_and_lock(&mut self) -> Result<Option<Vec<u8>>, Self::Error> {
                Ok(self
                    .runtime
                    .sync_request(|response| ContractRequest::TryReadAndLockMyState { response }))
            }

            fn store_and_unlock(&mut self, state: &[u8]) -> Result<bool, Self::Error> {
                Ok(self
                    .runtime
                    .sync_request(|response| ContractRequest::SaveAndUnlockMyState {
                        state: state.to_owned(),
                        response,
                    }))
            }

            fn lock_new(&mut self) -> Result<Self::Lock, Self::Error> {
                Ok(Mutex::new(
                    self.queued_future_factory.enqueue(
                        self.runtime
                            .async_request(|response| {
                                ContractRequest::Base(BaseRequest::LockViewUserState { response })
                            })
                            .map_err(|_| WasmExecutionError::MissingRuntimeResponse.into()),
                    ),
                ))
            }

            fn lock_poll(
                &mut self,
                future: &Self::Lock,
            ) -> Result<contract_system_api::PollLock, Self::Error> {
                use contract_system_api::PollLock;
                let mut receiver = future
                    .try_lock()
                    .expect("Unexpected reentrant locking of `oneshot::Receiver`");
                match self.waker().with_context(|context| receiver.poll_unpin(context)) {
                    Poll::Pending => Ok(PollLock::Pending),
                    Poll::Ready(Ok(Ok(()))) => Ok(PollLock::ReadyLocked),
                    Poll::Ready(Ok(Err(ExecutionError::ViewError(ViewError::TryLockError(_))))) => {
                        Ok(PollLock::ReadyNotLocked)
                    }
                    Poll::Ready(Ok(Err(error))) => Err(error),
                    Poll::Ready(Err(_)) => panic!(
                        "`HostFutureQueue` dropped while guest Wasm instance is still executing",
                    ),
                }
            }

            fn try_call_application(
                &mut self,
                authenticated: bool,
                application: contract_system_api::ApplicationId,
                argument: &[u8],
                forwarded_sessions: &[Le<contract_system_api::SessionId>],
            ) -> Result<contract_system_api::CallResult, Self::Error> {
                let forwarded_sessions = forwarded_sessions
                    .iter()
                    .map(Le::get)
                    .map(SessionId::from)
                    .collect();

                Ok(self
                    .runtime
                    .sync_request(|response| ContractRequest::TryCallApplication {
                        authenticated,
                        callee_id: application.into(),
                        argument: argument.to_owned(),
                        forwarded_sessions,
                        response,
                    })
                    .into())
            }

            fn try_call_session(
                &mut self,
                authenticated: bool,
                session: contract_system_api::SessionId,
                argument: &[u8],
                forwarded_sessions: &[Le<contract_system_api::SessionId>],
            ) -> Result<contract_system_api::CallResult, Self::Error> {
                let forwarded_sessions = forwarded_sessions
                    .iter()
                    .map(Le::get)
                    .map(SessionId::from)
                    .collect();

                Ok(self
                    .runtime
                    .sync_request(|response| ContractRequest::TryCallSession {
                        authenticated,
                        session_id: session.into(),
                        argument: argument.to_owned(),
                        forwarded_sessions,
                        response,
                    })
                    .into())
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
    ($service_system_api:ident<$runtime:lifetime>) => {
        impl_service_system_api!(@generate $service_system_api<$runtime>, $runtime, <$runtime>);
    };

    ($service_system_api:ident) => {
        impl_service_system_api!(@generate $service_system_api, 'static);
    };

    (@generate $service_system_api:ty, $runtime:lifetime $(, <$param:lifetime> )?) => {
        impl$(<$param>)? service_system_api::ServiceSystemApi for $service_system_api {
            type Load = Mutex<tokio::sync::oneshot::Receiver<Vec<u8>>>;
            type Lock = Mutex<tokio::sync::oneshot::Receiver<()>>;
            type Unlock = Mutex<tokio::sync::oneshot::Receiver<()>>;
            type TryQueryApplication = Mutex<tokio::sync::oneshot::Receiver<Vec<u8>>>;

            fn chain_id(&mut self) -> service_system_api::ChainId {
                self
                    .runtime
                    .sync_request(|response| {
                        ServiceRequest::Base(BaseRequest::ChainId { response })
                    })
                    .into()
            }

            fn application_id(&mut self) -> service_system_api::ApplicationId {
                self
                    .runtime
                    .sync_request(|response| {
                        ServiceRequest::Base(BaseRequest::ApplicationId { response })
                    })
                    .into()
            }

            fn application_parameters(&mut self) -> Vec<u8> {
                self.runtime.sync_request(|response| {
                    ServiceRequest::Base(BaseRequest::ApplicationParameters { response })
                })
            }

            fn read_system_balance(&mut self) -> service_system_api::Amount {
                self
                    .runtime
                    .sync_request(|response| {
                        ServiceRequest::Base(BaseRequest::ReadSystemBalance { response })
                    })
                    .into()
            }

            fn read_system_timestamp(&mut self) -> service_system_api::Timestamp {
                self
                    .runtime
                    .sync_request(|response| {
                        ServiceRequest::Base(BaseRequest::ReadSystemTimestamp { response })
                    })
                    .micros()
            }

            fn load_new(&mut self) -> Self::Load {
                Mutex::new(
                    self.runtime
                        .async_request(|response| {
                            ServiceRequest::Base(BaseRequest::TryReadMyState {
                                response: response.into(),
                            })
                        }),
                )
            }

            fn load_poll(&mut self, future: &Self::Load) -> service_system_api::PollLoad {
                use service_system_api::PollLoad;
                let mut receiver = future
                    .try_lock()
                    .expect("Unexpected reentrant locking of `oneshot::Receiver`");
                match self.waker().with_context(|context| receiver.poll_unpin(context)) {
                    Poll::Pending => PollLoad::Pending,
                    Poll::Ready(Ok(bytes)) => PollLoad::Ready(Ok(bytes)),
                    Poll::Ready(Err(_)) => panic!(
                        "`RuntimeActor` dropped while guest Wasm instance is still executing",
                    ),
                }
            }

            fn lock_new(&mut self) -> Self::Lock {
                Mutex::new(
                    self.runtime
                        .async_request(|response| {
                            ServiceRequest::Base(BaseRequest::LockViewUserState { response })
                        })
                )
            }

            fn lock_poll(&mut self, future: &Self::Lock) -> service_system_api::PollLock {
                use service_system_api::PollLock;
                let mut receiver = future
                    .try_lock()
                    .expect("Unexpected reentrant locking of `oneshot::Receiver`");
                match self.waker().with_context(|context| receiver.poll_unpin(context)) {
                    Poll::Pending => PollLock::Pending,
                    Poll::Ready(Ok(())) => PollLock::Ready(Ok(())),
                    Poll::Ready(Err(_)) => panic!(
                        "`RuntimeActor` dropped while guest Wasm instance is still executing",
                    ),
                }
            }

            fn unlock_new(&mut self) -> Self::Unlock {
                Mutex::new(
                    self.runtime
                        .async_request(|response| {
                            ServiceRequest::Base(BaseRequest::UnlockViewUserState { response })
                        })
                )
            }

            fn unlock_poll(&mut self, future: &Self::Lock) -> service_system_api::PollUnlock {
                use service_system_api::PollUnlock;
                let mut receiver = future
                    .try_lock()
                    .expect("Unexpected reentrant locking of `oneshot::Receiver`");
                match self.waker().with_context(|context| receiver.poll_unpin(context)) {
                    Poll::Pending => PollUnlock::Pending,
                    Poll::Ready(Ok(())) => PollUnlock::Ready(Ok(())),
                    Poll::Ready(Err(_)) => panic!(
                        "`RuntimeActor` dropped while guest Wasm instance is still executing",
                    ),
                }
            }

            fn try_query_application_new(
                &mut self,
                application: service_system_api::ApplicationId,
                argument: &[u8],
            ) -> Self::TryQueryApplication {
                let argument = Vec::from(argument);

                Mutex::new(
                    self.runtime
                        .async_request(|response| {
                            ServiceRequest::TryQueryApplication {
                                queried_id: application.into(),
                                argument: argument.to_owned(),
                                response,
                            }
                        })
                )
            }

            fn try_query_application_poll(
                &mut self,
                future: &Self::TryQueryApplication,
            ) -> service_system_api::PollLoad {
                use service_system_api::PollLoad;
                let mut receiver = future
                    .try_lock()
                    .expect("Unexpected reentrant locking of `oneshot::Receiver`");
                match self.waker().with_context(|context| receiver.poll_unpin(context)) {
                    Poll::Pending => PollLoad::Pending,
                    Poll::Ready(Ok(result)) => PollLoad::Ready(Ok(result)),
                    Poll::Ready(Err(_)) => panic!(
                        "`RuntimeActor` dropped while guest Wasm instance is still executing",
                    ),
                }
            }

            fn log(&mut self, message: &str, level: service_system_api::LogLevel) {
                match level {
                    service_system_api::LogLevel::Trace => tracing::trace!("{message}"),
                    service_system_api::LogLevel::Debug => tracing::debug!("{message}"),
                    service_system_api::LogLevel::Info => tracing::info!("{message}"),
                    service_system_api::LogLevel::Warn => tracing::warn!("{message}"),
                    service_system_api::LogLevel::Error => tracing::error!("{message}"),
                }
            }
        }
    };
}

/// Generates an implementation of `ViewSystem` for the provided `view_system_api` type for
/// application services.
///
/// Generates the common code for view system API types for all Wasm runtimes.
macro_rules! impl_view_system_api_for_service {
    ($view_system_api:ident<$runtime:lifetime>) => {
        impl_view_system_api_for_service!(
            @generate $view_system_api<$runtime>, wasmtime::Trap, $runtime, <$runtime>
        );
    };

    ($view_system_api:ty, $trap:ty) => {
        impl_view_system_api_for_service!(
            @generate $view_system_api, $trap, 'static
        );
    };

    ($view_system_api:ty) => {
        impl_view_system_api_for_service!(@generate $view_system_api, wasmer::RuntimeError, 'static);
    };

    (@generate $view_system_api:ty, $trap:ty, $runtime:lifetime $(, <$param:lifetime> )?) => {
        impl$(<$param>)? view_system_api::ViewSystemApi for $view_system_api {
            type Error = ExecutionError;

            type ReadKeyBytes = Mutex<tokio::sync::oneshot::Receiver<Option<Vec<u8>>>>;
            type FindKeys = Mutex<tokio::sync::oneshot::Receiver<Vec<Vec<u8>>>>;
            type FindKeyValues = Mutex<tokio::sync::oneshot::Receiver<Vec<(Vec<u8>, Vec<u8>)>>>;
            type WriteBatch = ();

            fn error_to_trap(&mut self, error: Self::Error) -> $trap {
                error.into()
            }

            fn read_key_bytes_new(
                &mut self,
                key: &[u8],
            ) -> Result<Self::ReadKeyBytes, Self::Error> {
                Ok(Mutex::new(
                    self.runtime.async_request(|response| {
                        ServiceRequest::Base(BaseRequest::ReadKeyBytes {
                            key: key.to_owned(),
                            response,
                        })
                    }),
                ))
            }

            fn read_key_bytes_poll(
                &mut self,
                future: &Self::ReadKeyBytes,
            ) -> Result<view_system_api::PollReadKeyBytes, Self::Error> {
                use view_system_api::PollReadKeyBytes;
                let mut receiver = future
                    .try_lock()
                    .expect("Unexpected reentrant locking of `oneshot::Receiver`");
                match self.waker().with_context(|context| receiver.poll_unpin(context)) {
                    Poll::Pending => Ok(PollReadKeyBytes::Pending),
                    Poll::Ready(Ok(opt_list)) => Ok(PollReadKeyBytes::Ready(opt_list)),
                    Poll::Ready(Err(_)) => panic!(
                        "`RuntimeActor` dropped while guest Wasm instance is still executing",
                    ),
                }
            }

            fn find_keys_new(&mut self, key_prefix: &[u8]) -> Result<Self::FindKeys, Self::Error> {
                Ok(Mutex::new(
                    self.runtime.async_request(|response| {
                        ServiceRequest::Base(BaseRequest::FindKeysByPrefix {
                            key_prefix: key_prefix.to_owned(),
                            response,
                        })
                    }),
                ))
            }

            fn find_keys_poll(
                &mut self,
                future: &Self::FindKeys,
            ) -> Result<view_system_api::PollFindKeys, Self::Error> {
                use view_system_api::PollFindKeys;
                let mut receiver = future
                    .try_lock()
                    .expect("Unexpected reentrant locking of `oneshot::Receiver`");
                match self.waker().with_context(|context| receiver.poll_unpin(context)) {
                    Poll::Pending => Ok(PollFindKeys::Pending),
                    Poll::Ready(Ok(keys)) => Ok(PollFindKeys::Ready(keys)),
                    Poll::Ready(Err(_)) => panic!(
                        "`RuntimeActor` dropped while guest Wasm instance is still executing",
                    ),
                }
            }

            fn find_key_values_new(
                &mut self,
                key_prefix: &[u8],
            ) -> Result<Self::FindKeyValues, Self::Error> {
                Ok(Mutex::new(
                    self.runtime.async_request(|response| {
                        ServiceRequest::Base(BaseRequest::FindKeyValuesByPrefix {
                            key_prefix: key_prefix.to_owned(),
                            response,
                        })
                    }),
                ))
            }

            fn find_key_values_poll(
                &mut self,
                future: &Self::FindKeyValues,
            ) -> Result<view_system_api::PollFindKeyValues, Self::Error> {
                use view_system_api::PollFindKeyValues;
                let mut receiver = future
                    .try_lock()
                    .expect("Unexpected reentrant locking of `oneshot::Receiver`");
                match self.waker().with_context(|context| receiver.poll_unpin(context)) {
                    Poll::Pending => Ok(PollFindKeyValues::Pending),
                    Poll::Ready(Ok(key_values)) => Ok(PollFindKeyValues::Ready(key_values)),
                    Poll::Ready(Err(_)) => panic!(
                        "`RuntimeActor` dropped while guest Wasm instance is still executing",
                    ),
                }
            }

            fn write_batch_new(
                &mut self,
                _list_oper: Vec<view_system_api::WriteOperation>,
            ) -> Result<Self::WriteBatch, Self::Error> {
                Err(WasmExecutionError::WriteAttemptToReadOnlyStorage.into())
            }

            fn write_batch_poll(
                &mut self,
                _future: &Self::WriteBatch,
            ) -> Result<view_system_api::PollUnit, Self::Error> {
                Err(WasmExecutionError::WriteAttemptToReadOnlyStorage.into())
            }
        }
    };
}

/// Generates an implementation of `ViewSystem` for the provided `view_system_api` type for
/// application contracts.
///
/// Generates the common code for view system API types for all WASM runtimes.
macro_rules! impl_view_system_api_for_contract {
    ($view_system_api:ident<$runtime:lifetime>) => {
        impl_view_system_api_for_contract!(
            @generate $view_system_api<$runtime>, wasmtime::Trap, <$runtime>
        );
    };

    ($view_system_api:ty) => {
        impl_view_system_api_for_contract!(
            @generate $view_system_api, wasmer::RuntimeError
        );
    };

    (@generate $view_system_api:ty, $trap:ty $(, <$param:lifetime> )?) => {
        impl$(<$param>)? view_system_api::ViewSystemApi for $view_system_api {
            type Error = ExecutionError;

            type ReadKeyBytes = Mutex<oneshot::Receiver<Result<Option<Vec<u8>>, ExecutionError>>>;
            type FindKeys = Mutex<oneshot::Receiver<Result<Vec<Vec<u8>>, ExecutionError>>>;
            type FindKeyValues =
                Mutex<oneshot::Receiver<Result<Vec<(Vec<u8>, Vec<u8>)>, ExecutionError>>>;
            type WriteBatch = Mutex<oneshot::Receiver<Result<(), ExecutionError>>>;

            fn error_to_trap(&mut self, error: Self::Error) -> $trap {
                error.into()
            }

            fn read_key_bytes_new(
                &mut self,
                key: &[u8],
            ) -> Result<Self::ReadKeyBytes, Self::Error> {
                Ok(Mutex::new(
                    self.queued_future_factory.enqueue(
                        self.runtime.async_request(|response| {
                            ContractRequest::Base(BaseRequest::ReadKeyBytes {
                                key: key.to_owned(),
                                response,
                            })
                        })
                        .map_err(|_| WasmExecutionError::MissingRuntimeResponse.into()),
                    ),
                ))
            }

            fn read_key_bytes_poll(
                &mut self,
                future: &Self::ReadKeyBytes,
            ) -> Result<view_system_api::PollReadKeyBytes, Self::Error> {
                use view_system_api::PollReadKeyBytes;
                let mut receiver = future
                    .try_lock()
                    .expect("Unexpected reentrant locking of `oneshot::Receiver`");
                match self.waker().with_context(|context| receiver.poll_unpin(context)) {
                    Poll::Pending => Ok(PollReadKeyBytes::Pending),
                    Poll::Ready(Ok(Ok(opt_list))) => Ok(PollReadKeyBytes::Ready(opt_list)),
                    Poll::Ready(Ok(Err(error))) => Err(error),
                    Poll::Ready(Err(_)) => panic!(
                        "`HostFutureQueue` dropped while guest Wasm instance is still executing",
                    ),
                }
            }

            fn find_keys_new(&mut self, key_prefix: &[u8]) -> Result<Self::FindKeys, Self::Error> {
                Ok(Mutex::new(
                    self.queued_future_factory.enqueue(
                        self.runtime.async_request(|response| {
                            ContractRequest::Base(BaseRequest::FindKeysByPrefix {
                                key_prefix: key_prefix.to_owned(),
                                response,
                            })
                        })
                        .map_err(|_| WasmExecutionError::MissingRuntimeResponse.into()),
                    ),
                ))
            }

            fn find_keys_poll(
                &mut self,
                future: &Self::FindKeys,
            ) -> Result<view_system_api::PollFindKeys, Self::Error> {
                use view_system_api::PollFindKeys;
                let mut receiver = future
                    .try_lock()
                    .expect("Unexpected reentrant locking of `oneshot::Receiver`");
                match self.waker().with_context(|context| receiver.poll_unpin(context)) {
                    Poll::Pending => Ok(PollFindKeys::Pending),
                    Poll::Ready(Ok(Ok(keys))) => Ok(PollFindKeys::Ready(keys)),
                    Poll::Ready(Ok(Err(error))) => Err(error),
                    Poll::Ready(Err(_)) => panic!(
                        "`HostFutureQueue` dropped while guest Wasm instance is still executing",
                    ),
                }
            }

            fn find_key_values_new(
                &mut self,
                key_prefix: &[u8],
            ) -> Result<Self::FindKeyValues, Self::Error> {
                Ok(Mutex::new(
                    self.queued_future_factory.enqueue(
                        self.runtime.async_request(|response| {
                            ContractRequest::Base(BaseRequest::FindKeyValuesByPrefix {
                                key_prefix: key_prefix.to_owned(),
                                response,
                            })
                        })
                        .map_err(|_| WasmExecutionError::MissingRuntimeResponse.into()),
                    ),
                ))
            }

            fn find_key_values_poll(
                &mut self,
                future: &Self::FindKeyValues,
            ) -> Result<view_system_api::PollFindKeyValues, Self::Error> {
                use view_system_api::PollFindKeyValues;
                let mut receiver = future
                    .try_lock()
                    .expect("Unexpected reentrant locking of `oneshot::Receiver`");
                match self.waker().with_context(|context| receiver.poll_unpin(context)) {
                    Poll::Pending => Ok(PollFindKeyValues::Pending),
                    Poll::Ready(Ok(Ok(key_values))) => Ok(PollFindKeyValues::Ready(key_values)),
                    Poll::Ready(Ok(Err(error))) => Err(error),
                    Poll::Ready(Err(_)) => panic!(
                        "`HostFutureQueue` dropped while guest Wasm instance is still executing",
                    ),
                }
            }

            fn write_batch_new(
                &mut self,
                list_oper: Vec<view_system_api::WriteOperation>,
            ) -> Result<Self::WriteBatch, Self::Error> {
                let mut batch = Batch::new();
                for x in list_oper {
                    match x {
                        view_system_api::WriteOperation::Delete(key) => {
                            batch.delete_key(key.to_vec())
                        }
                        view_system_api::WriteOperation::Deleteprefix(key_prefix) => {
                            batch.delete_key_prefix(key_prefix.to_vec())
                        }
                        view_system_api::WriteOperation::Put(key_value) => {
                            batch.put_key_value_bytes(key_value.0.to_vec(), key_value.1.to_vec())
                        }
                    }
                }
                Ok(Mutex::new(
                    self.queued_future_factory.enqueue(
                        self.runtime.async_request(|response| {
                            ContractRequest::WriteBatchAndUnlock { batch, response }
                        })
                        .map_err(|_| WasmExecutionError::MissingRuntimeResponse.into()),
                    ),
                ))
            }

            fn write_batch_poll(
                &mut self,
                future: &Self::WriteBatch,
            ) -> Result<view_system_api::PollUnit, Self::Error> {
                use view_system_api::PollUnit;
                let mut receiver = future
                    .try_lock()
                    .expect("Unexpected reentrant locking of `oneshot::Receiver`");
                match self.waker().with_context(|context| receiver.poll_unpin(context)) {
                    Poll::Pending => Ok(PollUnit::Pending),
                    Poll::Ready(Ok(Ok(()))) => Ok(PollUnit::Ready),
                    Poll::Ready(Ok(Err(error))) => Err(error),
                    Poll::Ready(Err(_)) => panic!(
                        "`HostFutureQueue` dropped while guest Wasm instance is still executing",
                    ),
                }
            }
        }
    };
}
