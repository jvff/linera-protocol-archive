// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Types for the exported futures for the contract endpoints.
//!
//! Each type is called by the code generated by [`wit-bindgen-guest-rust`] when the host calls the guest
//! Wasm module's respective endpoint. This module contains the code to forward the call to the
//! contract type that implements [`Contract`].

use crate::{
    contract::{system_api, wit_types},
    views::ViewStorageContext,
    ApplicationCallResult, Contract, ContractLogger, ExecutionResult, ExportedFuture,
    SessionCallResult, SessionId, SimpleStateStorage, ViewStateStorage,
};
use async_trait::async_trait;
use futures::TryFutureExt;
use linera_views::views::RootView;
use serde::{de::DeserializeOwned, Serialize};
use std::{future::Future, marker::PhantomData, mem};

/// The storage APIs used by a contract.
#[async_trait]
pub trait ContractStateStorage<Application> {
    /// Loads the `Application` state and locks it for writing.
    async fn load_and_lock() -> Application;

    /// Stores the `Application` state and unlocks it for reads and writes.
    async fn store_and_unlock(state: Application);

    /// Executes an `operation` with the `Application` state.
    ///
    /// The state is only stored back in storage if the `operation` succeeds. Otherwise, the error
    /// is returned as a [`String`].
    async fn execute_with_state<Operation, AsyncOperation, Success, Error>(
        operation: Operation,
    ) -> Result<Success, String>
    where
        Operation: FnOnce(Application) -> AsyncOperation,
        AsyncOperation: Future<Output = Result<(Application, Success), Error>> + Send,
        Application: Send,
        Operation: Send,
        Success: Send + 'static,
        Error: ToString + 'static,
    {
        let application = Self::load_and_lock().await;

        operation(application)
            .and_then(|(application, result)| async move {
                Self::store_and_unlock(application).await;
                Ok(result)
            })
            .await
            .map_err(|error| error.to_string())
    }

    /// Executes an `operation`, persisting the `Application` `state` before execution and reloading
    /// the `state` afterwards.
    async fn execute_with_released_state<Operation>(
        state: &mut Application,
        operation: impl FnOnce() -> Operation + Send,
    ) -> Operation::Output
    where
        Operation: Future + Send,
        Operation::Output: Send;
}

#[async_trait]
impl<Application> ContractStateStorage<Application> for SimpleStateStorage<Application>
where
    Application: Contract + Default + DeserializeOwned + Serialize + Send + 'static,
{
    async fn load_and_lock() -> Application {
        system_api::load_and_lock().expect("Failed to lock contract state")
    }

    async fn store_and_unlock(state: Application) {
        system_api::store_and_unlock(state).await;
    }

    async fn execute_with_released_state<Operation>(
        state: &mut Application,
        operation: impl FnOnce() -> Operation + Send,
    ) -> Operation::Output
    where
        Operation: Future + Send,
        Operation::Output: Send,
    {
        Self::store_and_unlock(mem::take(state)).await;
        let result = operation().await;
        *state = Self::load_and_lock().await;
        result
    }
}

#[async_trait]
impl<Application> ContractStateStorage<Application> for ViewStateStorage<Application>
where
    Application: Contract + RootView<ViewStorageContext> + Send + 'static,
{
    async fn load_and_lock() -> Application {
        system_api::load_and_lock_view()
            .await
            .expect("Failed to lock contract view")
    }

    async fn store_and_unlock(state: Application) {
        system_api::store_and_unlock_view(state).await;
    }

    async fn execute_with_released_state<Operation>(
        state: &mut Application,
        operation: impl FnOnce() -> Operation + Send,
    ) -> Operation::Output
    where
        Operation: Future + Send,
        Operation::Output: Send,
    {
        state.save().await.expect("Failed to save view state");
        let result = operation().await;
        *state = Self::load_and_lock().await;
        result
    }
}

/// Future implementation exported from the guest to allow the host to call
/// [`Contract::execute_operation`].
///
/// Loads the `Application` state and calls its
/// [`execute_operation`][Contract::execute_operation] method.
pub struct ExecuteOperation<Application: Contract> {
    future: ExportedFuture<Result<ExecutionResult<Application::Message>, String>>,
    _application: PhantomData<Application>,
}

impl<Application> ExecuteOperation<Application>
where
    Application: Contract + 'static,
{
    /// Creates the exported future that the host can poll.
    ///
    /// This is called from the host.
    pub fn new(context: wit_types::OperationContext, operation: Vec<u8>) -> Self {
        ContractLogger::install();
        ExecuteOperation {
            future: ExportedFuture::new(Application::Storage::execute_with_state(
                move |mut application| async move {
                    let operation: Application::Operation = bcs::from_bytes(&operation)?;
                    application
                        .execute_operation(&context.into(), operation)
                        .await
                        .map(|result| (application, result))
                },
            )),
            _application: PhantomData,
        }
    }

    /// Polls the future export from the guest.
    ///
    /// This is called from the host.
    pub fn poll(&self) -> wit_types::PollExecutionResult {
        self.future.poll()
    }
}

/// Future implementation exported from the guest to allow the host to call
/// [`Contract::execute_message`].
///
/// Loads the `Application` state and calls its [`execute_message`][Contract::execute_message]
/// method.
pub struct ExecuteMessage<Application: Contract> {
    future: ExportedFuture<Result<ExecutionResult<Application::Message>, String>>,
    _application: PhantomData<Application>,
}

impl<Application> ExecuteMessage<Application>
where
    Application: Contract + 'static,
{
    /// Creates the exported future that the host can poll.
    ///
    /// This is called from the host.
    pub fn new(context: wit_types::MessageContext, message: Vec<u8>) -> Self {
        ContractLogger::install();
        ExecuteMessage {
            future: ExportedFuture::new(Application::Storage::execute_with_state(
                move |mut application| async move {
                    let message: Application::Message = bcs::from_bytes(&message)?;
                    application
                        .execute_message(&context.into(), message)
                        .await
                        .map(|result| (application, result))
                },
            )),
            _application: PhantomData,
        }
    }

    /// Polls the future export from the guest.
    ///
    /// This is called from the host.
    pub fn poll(&self) -> wit_types::PollExecutionResult {
        self.future.poll()
    }
}

/// Future implementation exported from the guest to allow the host to call
/// [`Contract::handle_application_call`].
///
/// Loads the `Application` state and calls its
/// [`handle_application_call`][Contract::handle_application_call] method.
#[allow(clippy::type_complexity)]
pub struct HandleApplicationCall<Application: Contract> {
    future: ExportedFuture<
        Result<
            ApplicationCallResult<
                Application::Message,
                Application::Response,
                Application::SessionState,
            >,
            String,
        >,
    >,
    _application: PhantomData<Application>,
}

impl<Application> HandleApplicationCall<Application>
where
    Application: Contract + 'static,
{
    /// Creates the exported future that the host can poll.
    ///
    /// This is called from the host.
    pub fn new(
        context: wit_types::CalleeContext,
        argument: Vec<u8>,
        forwarded_sessions: Vec<wit_types::SessionId>,
    ) -> Self {
        ContractLogger::install();
        HandleApplicationCall {
            future: ExportedFuture::new(Application::Storage::execute_with_state(
                move |mut application| async move {
                    let argument: Application::ApplicationCall = bcs::from_bytes(&argument)?;
                    let forwarded_sessions = forwarded_sessions
                        .into_iter()
                        .map(SessionId::from)
                        .collect();

                    application
                        .handle_application_call(&context.into(), argument, forwarded_sessions)
                        .await
                        .map(|result| (application, result))
                },
            )),
            _application: PhantomData,
        }
    }

    /// Polls the future export from the guest.
    ///
    /// This is called from the host.
    pub fn poll(&self) -> wit_types::PollApplicationCallResult {
        self.future.poll()
    }
}

/// Future implementation exported from the guest to allow the host to call
/// [`Contract::handle_session_call`].
///
/// Loads the `Application` state and calls its
/// [`handle_session_call`][Contract::handle_session_call] method.
#[allow(clippy::type_complexity)]
pub struct HandleSessionCall<Application: Contract> {
    future: ExportedFuture<
        Result<
            SessionCallResult<
                Application::Message,
                Application::Response,
                Application::SessionState,
            >,
            String,
        >,
    >,
    _application: PhantomData<Application>,
}

impl<Application> HandleSessionCall<Application>
where
    Application: Contract + 'static,
{
    /// Creates the exported future that the host can poll.
    ///
    /// This is called from the host.
    pub fn new(
        context: wit_types::CalleeContext,
        session: Vec<u8>,
        argument: Vec<u8>,
        forwarded_sessions: Vec<wit_types::SessionId>,
    ) -> Self {
        ContractLogger::install();
        HandleSessionCall {
            future: ExportedFuture::new(Application::Storage::execute_with_state(
                move |mut application| async move {
                    let session: Application::SessionState = bcs::from_bytes(&session)?;
                    let argument: Application::SessionCall = bcs::from_bytes(&argument)?;
                    let forwarded_sessions = forwarded_sessions
                        .into_iter()
                        .map(SessionId::from)
                        .collect();

                    application
                        .handle_session_call(&context.into(), session, argument, forwarded_sessions)
                        .await
                        .map(|result| (application, result))
                },
            )),
            _application: PhantomData,
        }
    }

    /// Polls the future export from the guest.
    ///
    /// This is called from the host.
    pub fn poll(&self) -> wit_types::PollSessionCallResult {
        self.future.poll()
    }
}
