// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Implementations of how requests should be handled inside a [`RuntimeActor`].

use super::requests::BaseRequest;
use crate::{BaseRuntime, ExecutionError};
use async_trait::async_trait;

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
                response.send(self.unlock_view_user_state().await?)
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
