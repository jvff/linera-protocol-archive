// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Implementations of how requests should be handled inside a [`RuntimeActor`].

use super::requests::{BaseRequest, ContractRequest, ServiceRequest};
use crate::{BaseRuntime, ContractRuntime, ExecutionError, ServiceRuntime};
use async_trait::async_trait;
use linera_views::views::ViewError;

/// A type that is able to handle incoming `Request`s.
#[async_trait]
pub trait RequestHandler<Request> {
    /// Handles a `Request`.
    ///
    /// Returns an error if the request could not be handled and no further requests should be sent
    /// to this handler.
    async fn handle_request(&self, request: Request) -> Result<(), ExecutionError>;
}

#[async_trait]
impl<Runtime> RequestHandler<BaseRequest> for &Runtime
where
    Runtime: BaseRuntime + ?Sized,
{
    async fn handle_request(&self, request: BaseRequest) -> Result<(), ExecutionError> {
        match request {
            BaseRequest::ChainId { response } => response.send(self.chain_id()),
            BaseRequest::ApplicationId { response } => response.send(self.application_id()),
            BaseRequest::ApplicationParameters { response } => {
                response.send(self.application_parameters())
            }
            BaseRequest::ReadSystemBalance { response } => {
                response.send(self.read_system_balance())
            }
            BaseRequest::ReadSystemTimestamp { response } => {
                response.send(self.read_system_timestamp())
            }
            BaseRequest::TryReadMyState { response } => {
                response.send(self.try_read_my_state().await?)
            }
            BaseRequest::LockViewUserState { response } => {
                let _ = response.send(self.lock_view_user_state().await?);
            }
            BaseRequest::UnlockViewUserState { response } => {
                let _ = response.send(self.unlock_view_user_state().await?);
            }
            BaseRequest::ReadKeyBytes { key, response } => {
                let _ = response.send(self.read_key_bytes(key).await?);
            }
            BaseRequest::FindKeysByPrefix {
                key_prefix,
                response,
            } => {
                let _ = response.send(self.find_keys_by_prefix(key_prefix).await?);
            }
            BaseRequest::FindKeyValuesByPrefix {
                key_prefix,
                response,
            } => {
                let _ = response.send(self.find_key_values_by_prefix(key_prefix).await?);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<Runtime> RequestHandler<ContractRequest> for &Runtime
where
    Runtime: ContractRuntime + ?Sized,
{
    async fn handle_request(&self, request: ContractRequest) -> Result<(), ExecutionError> {
        tracing::info!("Received request {request:?}");
        // Use unit arguments in `Response::send` in order to have compile errors if the return
        // value of the called function changes.
        #[allow(clippy::unit_arg)]
        match request {
            ContractRequest::Base(base_request) => (*self).handle_request(base_request).await?,
            ContractRequest::RemainingFuel { response } => response.send(self.remaining_fuel()),
            ContractRequest::SetRemainingFuel {
                remaining_fuel,
                response,
            } => response.send(self.set_remaining_fuel(remaining_fuel)),
            ContractRequest::TryReadAndLockMyState { response } => {
                response.send(match self.try_read_and_lock_my_state().await {
                    Ok(bytes) => Some(bytes),
                    Err(ExecutionError::ViewError(ViewError::NotFound(_))) => None,
                    Err(error) => return Err(error),
                })
            }
            ContractRequest::SaveAndUnlockMyState { state, response } => {
                response.send(self.save_and_unlock_my_state(state).is_ok())
            }
            ContractRequest::UnlockMyState { response } => response.send(self.unlock_my_state()),
            ContractRequest::WriteBatchAndUnlock { batch, response } => {
                let _ = response.send(self.write_batch_and_unlock(batch).await?);
            }
            ContractRequest::TryCallApplication {
                authenticated,
                callee_id,
                argument,
                forwarded_sessions,
                response,
            } => response.send(
                self.try_call_application(authenticated, callee_id, &argument, forwarded_sessions)
                    .await?,
            ),
            ContractRequest::TryCallSession {
                authenticated,
                session_id,
                argument,
                forwarded_sessions,
                response,
            } => response.send(
                self.try_call_session(authenticated, session_id, &argument, forwarded_sessions)
                    .await?,
            ),
        }

        Ok(())
    }
}

#[async_trait]
impl<Runtime> RequestHandler<ServiceRequest> for &Runtime
where
    Runtime: ServiceRuntime + ?Sized,
{
    async fn handle_request(&self, request: ServiceRequest) -> Result<(), ExecutionError> {
        // Use unit arguments in `Response::send` in order to have compile errors if the return
        // value of the called function changes.
        #[allow(clippy::unit_arg)]
        match request {
            ServiceRequest::Base(base_request) => (*self).handle_request(base_request).await?,
            ServiceRequest::TryQueryApplication {
                queried_id,
                argument,
                response,
            } => {
                let _ = response.send(self.try_query_application(queried_id, &argument).await?);
            }
        }

        Ok(())
    }
}
