// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Generates an implementation of `WritableSystem` for the provided `contract_system_api` type.
///
/// Generates the common code for contract system API types for all WASM runtimes.
macro_rules! impl_writable_system {
    ($contract_system_api:ident<$runtime:lifetime>) => {
        impl_writable_system!(
            @generate $contract_system_api<$runtime>, wasmtime::Trap, $runtime, <$runtime>
        );
    };

    ($contract_system_api:ident) => {
        impl_writable_system!(@generate $contract_system_api, wasmer::RuntimeError, 'static);
    };

    (@generate $contract_system_api:ty, $trap:ty, $runtime:lifetime $(, <$param:lifetime> )?) => {
        impl$(<$param>)? WritableSystem for $contract_system_api {
            type Error = ExecutionError;

            type Lock = HostFuture<$runtime, Result<(), ExecutionError>>;

            fn error_to_trap(&mut self, error: Self::Error) -> $trap {
                error.into()
            }

            fn chain_id(&mut self) -> Result<writable_system::ChainId, Self::Error> {
                Ok(self.runtime().chain_id().into())
            }

            fn application_id(&mut self) -> Result<writable_system::ApplicationId, Self::Error> {
                Ok(self.runtime().application_id().into())
            }

            fn application_parameters(&mut self) -> Result<Vec<u8>, Self::Error> {
                Ok(self.runtime().application_parameters())
            }

            fn read_system_balance(
                &mut self,
            ) -> Result<writable_system::Balance, Self::Error> {
                Ok(self.runtime().read_system_balance().into())
            }

            fn read_system_timestamp(&mut self) -> Result<writable_system::Timestamp, Self::Error> {
                Ok(self.runtime().read_system_timestamp().micros())
            }

            fn load(&mut self) -> Result<Vec<u8>, Self::Error> {
                Self::block_on(self.runtime().try_read_my_state())
            }

            fn load_and_lock(&mut self) -> Result<Option<Vec<u8>>, Self::Error> {
                match Self::block_on(self.runtime().try_read_and_lock_my_state()) {
                    Ok(bytes) => Ok(Some(bytes)),
                    Err(ExecutionError::ViewError(ViewError::NotFound(_))) => Ok(None),
                    Err(error) => Err(error),
                }
            }

            fn store_and_unlock(&mut self, state: &[u8]) -> Result<bool, Self::Error> {
                Ok(self
                    .runtime()
                    .save_and_unlock_my_state(state.to_owned())
                    .is_ok())
            }

            fn lock_new(&mut self) -> Result<Self::Lock, Self::Error> {
                Ok(self
                    .queued_future_factory
                    .enqueue(self.runtime().lock_view_user_state()))
            }

            fn lock_poll(
                &mut self,
                future: &Self::Lock,
            ) -> Result<writable_system::PollLock, Self::Error> {
                use writable_system::PollLock;
                match future.poll(&mut *self.waker()) {
                    Poll::Pending => Ok(PollLock::Pending),
                    Poll::Ready(Ok(())) => Ok(PollLock::ReadyLocked),
                    Poll::Ready(Err(ExecutionError::ViewError(ViewError::TryLockError(_)))) => {
                        Ok(PollLock::ReadyNotLocked)
                    }
                    Poll::Ready(Err(error)) => Err(error),
                }
            }

            fn try_call_application(
                &mut self,
                authenticated: bool,
                application: writable_system::ApplicationId,
                argument: &[u8],
                forwarded_sessions: &[Le<writable_system::SessionId>],
            ) -> Result<writable_system::CallResult, Self::Error> {
                let forwarded_sessions = forwarded_sessions
                    .iter()
                    .map(Le::get)
                    .map(SessionId::from)
                    .collect();
                let argument = Vec::from(argument);

                Self::block_on(self.runtime().try_call_application(
                    authenticated,
                    application.into(),
                    &argument,
                    forwarded_sessions,
                ))
                .map(writable_system::CallResult::from)
            }

            fn try_call_session(
                &mut self,
                authenticated: bool,
                session: writable_system::SessionId,
                argument: &[u8],
                forwarded_sessions: &[Le<writable_system::SessionId>],
            ) -> Result<writable_system::CallResult, Self::Error> {
                let forwarded_sessions = forwarded_sessions
                    .iter()
                    .map(Le::get)
                    .map(SessionId::from)
                    .collect();
                let argument = Vec::from(argument);

                Self::block_on(self.runtime().try_call_session(
                    authenticated,
                    session.into(),
                    &argument,
                    forwarded_sessions,
                ))
                .map(writable_system::CallResult::from)
            }

            fn log(
                &mut self,
                message: &str,
                level: writable_system::LogLevel,
            ) -> Result<(), Self::Error> {
                match level {
                    writable_system::LogLevel::Trace => tracing::trace!("{message}"),
                    writable_system::LogLevel::Debug => tracing::debug!("{message}"),
                    writable_system::LogLevel::Info => tracing::info!("{message}"),
                    writable_system::LogLevel::Warn => tracing::warn!("{message}"),
                    writable_system::LogLevel::Error => tracing::error!("{message}"),
                }
                Ok(())
            }
        }

        impl$(<$param>)? $contract_system_api {
            /// Calls a `future` in a blocking manner.
            fn block_on<F>(future: F) -> F::Output
            where
                F: std::future::Future + Send,
                F::Output: Send,
            {
                let runtime = tokio::runtime::Handle::current();

                std::thread::scope(|scope| {
                    scope
                        .spawn(|| runtime.block_on(future))
                        .join()
                        .expect("Panic when running a future in a blocking manner")
                })
            }
        }
    };
}

/// Generates an implementation of `QueryableSystem` for the provided `service_system_api` type.
///
/// Generates the common code for service system API types for all WASM runtimes.
macro_rules! impl_queryable_system {
    ($service_system_api:ident<$runtime:lifetime>) => {
        impl_queryable_system!(
            @generate $service_system_api<$runtime>, wasmtime::Trap, $runtime, <$runtime>
        );
    };

    ($service_system_api:ident) => {
        impl_queryable_system!(@generate $service_system_api, wasmer::RuntimeError, 'static);
    };

    (@generate $service_system_api:ty, $trap:ty, $runtime:lifetime $(, <$param:lifetime> )?) => {
        impl$(<$param>)? QueryableSystem for $service_system_api {
            type Error = ExecutionError;

            type Load = HostFuture<$runtime, Result<Vec<u8>, ExecutionError>>;
            type Lock = HostFuture<$runtime, Result<(), ExecutionError>>;
            type Unlock = HostFuture<$runtime, Result<(), ExecutionError>>;
            type ReadKeyBytes = HostFuture<$runtime, Result<Option<Vec<u8>>, ExecutionError>>;
            type FindKeys = HostFuture<$runtime, Result<Vec<Vec<u8>>, ExecutionError>>;
            type FindKeyValues =
                HostFuture<$runtime, Result<Vec<(Vec<u8>, Vec<u8>)>, ExecutionError>>;
            type TryQueryApplication = HostFuture<$runtime, Result<Vec<u8>, ExecutionError>>;

            fn error_to_trap(&mut self, error: Self::Error) -> $trap {
                error.into()
            }

            fn chain_id(&mut self) -> Result<queryable_system::ChainId, Self::Error> {
                Ok(self.runtime().chain_id().into())
            }

            fn application_id(&mut self) -> Result<queryable_system::ApplicationId, Self::Error> {
                Ok(self.runtime().application_id().into())
            }

            fn application_parameters(&mut self) -> Result<Vec<u8>, Self::Error> {
                Ok(self.runtime().application_parameters())
            }

            fn read_system_balance(&mut self) -> Result<queryable_system::Balance, Self::Error> {
                Ok(self.runtime().read_system_balance().into())
            }

            fn read_system_timestamp(
                &mut self,
            ) -> Result<queryable_system::Timestamp, Self::Error> {
                Ok(self.runtime().read_system_timestamp().micros())
            }

            fn load_new(&mut self) -> Result<Self::Load, Self::Error> {
                Ok(HostFuture::new(self.runtime().try_read_my_state()))
            }

            fn load_poll(
                &mut self,
                future: &Self::Load,
            ) -> Result<queryable_system::PollLoad, Self::Error> {
                use queryable_system::PollLoad;
                Ok(match future.poll(&mut *self.waker()) {
                    Poll::Pending => PollLoad::Pending,
                    Poll::Ready(Ok(bytes)) => PollLoad::Ready(Ok(bytes)),
                    Poll::Ready(Err(error)) => PollLoad::Ready(Err(error.to_string())),
                })
            }

            fn lock_new(&mut self) -> Result<Self::Lock, Self::Error> {
                Ok(HostFuture::new(self.runtime().lock_view_user_state()))
            }

            fn lock_poll(
                &mut self,
                future: &Self::Lock,
            ) -> Result<queryable_system::PollLock, Self::Error> {
                use queryable_system::PollLock;
                Ok(match future.poll(&mut *self.waker()) {
                    Poll::Pending => PollLock::Pending,
                    Poll::Ready(Ok(())) => PollLock::Ready(Ok(())),
                    Poll::Ready(Err(error)) => PollLock::Ready(Err(error.to_string())),
                })
            }

            fn unlock_new(&mut self) -> Result<Self::Unlock, Self::Error> {
                Ok(HostFuture::new(self.runtime().unlock_view_user_state()))
            }

            fn unlock_poll(
                &mut self,
                future: &Self::Lock,
            ) -> Result<queryable_system::PollUnlock, Self::Error> {
                use queryable_system::PollUnlock;
                Ok(match future.poll(&mut *self.waker()) {
                    Poll::Pending => PollUnlock::Pending,
                    Poll::Ready(Ok(())) => PollUnlock::Ready(Ok(())),
                    Poll::Ready(Err(error)) => PollUnlock::Ready(Err(error.to_string())),
                })
            }

            fn read_key_bytes_new(
                &mut self,
                key: &[u8],
            ) -> Result<Self::ReadKeyBytes, Self::Error> {
                Ok(HostFuture::new(self.runtime().read_key_bytes(key.to_owned())))
            }

            fn read_key_bytes_poll(
                &mut self,
                future: &Self::ReadKeyBytes,
            ) -> Result<queryable_system::PollReadKeyBytes, Self::Error> {
                use queryable_system::PollReadKeyBytes;
                match future.poll(&mut *self.waker()) {
                    Poll::Pending => Ok(PollReadKeyBytes::Pending),
                    Poll::Ready(Ok(opt_list)) => Ok(PollReadKeyBytes::Ready(opt_list)),
                    Poll::Ready(Err(error)) => Err(error),
                }
            }

            fn find_keys_new(&mut self, key_prefix: &[u8]) -> Result<Self::FindKeys, Self::Error> {
                Ok(HostFuture::new(self.runtime().find_keys_by_prefix(key_prefix.to_owned())))
            }

            fn find_keys_poll(
                &mut self,
                future: &Self::FindKeys,
            ) -> Result<queryable_system::PollFindKeys, Self::Error> {
                use queryable_system::PollFindKeys;
                match future.poll(&mut *self.waker()) {
                    Poll::Pending => Ok(PollFindKeys::Pending),
                    Poll::Ready(Ok(keys)) => Ok(PollFindKeys::Ready(keys)),
                    Poll::Ready(Err(error)) => Err(error),
                }
            }

            fn find_key_values_new(
                &mut self,
                key_prefix: &[u8],
            ) -> Result<Self::FindKeyValues, Self::Error> {
                Ok(HostFuture::new(
                    self.runtime()
                        .find_key_values_by_prefix(key_prefix.to_owned()),
                ))
            }

            fn find_key_values_poll(
                &mut self,
                future: &Self::FindKeyValues,
            ) -> Result<queryable_system::PollFindKeyValues, Self::Error> {
                use queryable_system::PollFindKeyValues;
                match future.poll(&mut *self.waker()) {
                    Poll::Pending => Ok(PollFindKeyValues::Pending),
                    Poll::Ready(Ok(key_values)) => Ok(PollFindKeyValues::Ready(key_values)),
                    Poll::Ready(Err(error)) => Err(error),
                }
            }

            fn try_query_application_new(
                &mut self,
                application: queryable_system::ApplicationId,
                argument: &[u8],
            ) -> Result<Self::TryQueryApplication, Self::Error> {
                let runtime = self.runtime();
                let argument = Vec::from(argument);

                Ok(HostFuture::new(async move {
                    runtime
                        .try_query_application(application.into(), &argument)
                        .await
                }))
            }

            fn try_query_application_poll(
                &mut self,
                future: &Self::TryQueryApplication,
            ) -> Result<queryable_system::PollLoad, Self::Error> {
                use queryable_system::PollLoad;
                Ok(match future.poll(&mut *self.waker()) {
                    Poll::Pending => PollLoad::Pending,
                    Poll::Ready(Ok(result)) => PollLoad::Ready(Ok(result)),
                    Poll::Ready(Err(error)) => PollLoad::Ready(Err(error.to_string())),
                })
            }

            fn log(
                &mut self,
                message: &str,
                level: queryable_system::LogLevel,
            ) -> Result<(), Self::Error> {
                match level {
                    queryable_system::LogLevel::Trace => tracing::trace!("{message}"),
                    queryable_system::LogLevel::Debug => tracing::debug!("{message}"),
                    queryable_system::LogLevel::Info => tracing::info!("{message}"),
                    queryable_system::LogLevel::Warn => tracing::warn!("{message}"),
                    queryable_system::LogLevel::Error => tracing::error!("{message}"),
                }
                Ok(())
            }
        }
    };
}

/// Generates an implementation of `ViewSystem` for the provided `view_system_api` type.
///
/// Generates the common code for view system API types for all WASM runtimes.
macro_rules! impl_view_system {
    ($view_system_api:ident<$runtime:lifetime>) => {
        impl_view_system!(
            @generate $view_system_api<$runtime>, wasmtime::Trap, $runtime, <$runtime>
        );
    };

    ($view_system_api:ty) => {
        impl_view_system!(@generate $view_system_api, wasmer::RuntimeError, 'static);
    };

    (@generate $view_system_api:ty, $trap:ty, $runtime:lifetime $(, <$param:lifetime> )?) => {
        impl$(<$param>)? ViewSystem for $view_system_api {
            type Error = ExecutionError;

            type ReadKeyBytes = HostFuture<$runtime, Result<Option<Vec<u8>>, ExecutionError>>;
            type FindKeys = HostFuture<$runtime, Result<Vec<Vec<u8>>, ExecutionError>>;
            type FindKeyValues =
                HostFuture<$runtime, Result<Vec<(Vec<u8>, Vec<u8>)>, ExecutionError>>;
            type WriteBatch = HostFuture<$runtime, Result<(), ExecutionError>>;

            fn error_to_trap(&mut self, error: Self::Error) -> $trap {
                error.into()
            }

            fn read_key_bytes_new(
                &mut self,
                key: &[u8],
            ) -> Result<Self::ReadKeyBytes, Self::Error> {
                Ok(self.new_host_future(self.runtime().read_key_bytes(key.to_owned())))
            }

            fn read_key_bytes_poll(
                &mut self,
                future: &Self::ReadKeyBytes,
            ) -> Result<view_system::PollReadKeyBytes, Self::Error> {
                use view_system::PollReadKeyBytes;
                match future.poll(&mut *self.waker()) {
                    Poll::Pending => Ok(PollReadKeyBytes::Pending),
                    Poll::Ready(Ok(opt_list)) => Ok(PollReadKeyBytes::Ready(opt_list)),
                    Poll::Ready(Err(error)) => Err(error),
                }
            }

            fn find_keys_new(&mut self, key_prefix: &[u8]) -> Result<Self::FindKeys, Self::Error> {
                Ok(self.new_host_future(self.runtime().find_keys_by_prefix(key_prefix.to_owned())))
            }

            fn find_keys_poll(
                &mut self,
                future: &Self::FindKeys,
            ) -> Result<view_system::PollFindKeys, Self::Error> {
                use view_system::PollFindKeys;
                match future.poll(&mut *self.waker()) {
                    Poll::Pending => Ok(PollFindKeys::Pending),
                    Poll::Ready(Ok(keys)) => Ok(PollFindKeys::Ready(keys)),
                    Poll::Ready(Err(error)) => Err(error),
                }
            }

            fn find_key_values_new(
                &mut self,
                key_prefix: &[u8],
            ) -> Result<Self::FindKeyValues, Self::Error> {
                Ok(HostFuture::new(
                    self.runtime()
                        .find_key_values_by_prefix(key_prefix.to_owned()),
                ))
            }

            fn find_key_values_poll(
                &mut self,
                future: &Self::FindKeyValues,
            ) -> Result<view_system::PollFindKeyValues, Self::Error> {
                use view_system::PollFindKeyValues;
                match future.poll(&mut *self.waker()) {
                    Poll::Pending => Ok(PollFindKeyValues::Pending),
                    Poll::Ready(Ok(key_values)) => Ok(PollFindKeyValues::Ready(key_values)),
                    Poll::Ready(Err(error)) => Err(error),
                }
            }

            fn write_batch_new(
                &mut self,
                list_oper: Vec<view_system::WriteOperation>,
            ) -> Result<Self::WriteBatch, Self::Error> {
                let mut batch = Batch::new();
                for x in list_oper {
                    match x {
                        view_system::WriteOperation::Delete(key) => {
                            batch.delete_key(key.to_vec())
                        }
                        view_system::WriteOperation::Deleteprefix(key_prefix) => {
                            batch.delete_key_prefix(key_prefix.to_vec())
                        }
                        view_system::WriteOperation::Put(key_value) => {
                            batch.put_key_value_bytes(key_value.0.to_vec(), key_value.1.to_vec())
                        }
                    }
                }
                Ok(self.new_host_future(self.contract_runtime()?.write_batch_and_unlock(batch)))
            }

            fn write_batch_poll(
                &mut self,
                future: &Self::WriteBatch,
            ) -> Result<view_system::PollUnit, Self::Error> {
                use view_system::PollUnit;
                match future.poll(&mut *self.waker()) {
                    Poll::Pending => Ok(PollUnit::Pending),
                    Poll::Ready(Ok(())) => Ok(PollUnit::Ready),
                    Poll::Ready(Err(error)) => Err(error),
                }
            }
        }
    };
}
