// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Runtime independent code for interfacing with user applications in WebAssembly modules.

use super::{
    async_boundary::{GuestFutureActor, GuestFutureInterface},
    async_determinism::HostFutureQueue,
    ExecutionError,
};
use crate::{
    ApplicationCallResult, CalleeContext, MessageContext, OperationContext, QueryContext,
    RawExecutionResult, SessionCallResult, SessionId,
};
use futures::{future, TryFutureExt};
use std::future::Future;

/// Types that are specific to the context of an application ready to be executedy by a WebAssembly
/// runtime.
pub trait ApplicationRuntimeContext: Sized {
    /// The error emitted by the runtime when the application traps (panics).
    type Error: Into<ExecutionError> + Send + Unpin;

    /// How to store the application's in-memory state.
    type Store: Send + Unpin;

    /// Extra runtime-specific data.
    type Extra: Send + Unpin;

    /// Finalizes the runtime context, running any extra clean-up operations.
    fn finalize(_context: &mut WasmRuntimeContext<Self>) {}
}

/// Common interface to calling a user contract in a WebAssembly module.
pub trait Contract: ApplicationRuntimeContext {
    /// The WIT type for the resource representing the guest future
    /// [`initialize`][crate::Contract::initialize] method.
    type Initialize: GuestFutureInterface<Self, Output = RawExecutionResult<Vec<u8>>> + Send + Unpin;

    /// The WIT type for the resource representing the guest future
    /// [`execute_operation`][crate::Contract::execute_operation] method.
    type ExecuteOperation: GuestFutureInterface<Self, Output = RawExecutionResult<Vec<u8>>>
        + Send
        + Unpin;

    /// The WIT type for the resource representing the guest future
    /// [`execute_message`][crate::Contract::execute_message] method.
    type ExecuteMessage: GuestFutureInterface<Self, Output = RawExecutionResult<Vec<u8>>>
        + Send
        + Unpin;

    /// The WIT type for the resource representing the guest future
    /// [`handle_application_call`][crate::Contract::handle_application_call] method.
    type HandleApplicationCall: GuestFutureInterface<Self, Output = ApplicationCallResult>
        + Send
        + Unpin;

    /// The WIT type for the resource representing the guest future
    /// [`handle_session_call`][crate::Contract::handle_session_call] method.
    type HandleSessionCall: GuestFutureInterface<Self, Output = (SessionCallResult, Vec<u8>)>
        + Send
        + Unpin;

    /// The WIT type eqivalent for [`Poll<Result<RawExecutionResult<Vec<u8>>, String>>`].
    type PollExecutionResult;

    /// The WIT type eqivalent for [`Poll<Result<ApplicationCallResult, String>>`].
    type PollCallApplication;

    /// The WIT type eqivalent for [`Poll<Result<SessionCallResult, String>>`].
    type PollCallSession;

    /// Configures the amount of fuel available before executing the contract.
    fn configure_fuel(context: &mut WasmRuntimeContext<Self>);

    /// Creates a new future for the user application to initialize itself on the owner chain.
    fn initialize_new(
        &self,
        store: &mut Self::Store,
        context: OperationContext,
        argument: Vec<u8>,
    ) -> Result<Self::Initialize, Self::Error>;

    /// Polls a user contract future that's initializing the application.
    fn initialize_poll(
        &self,
        store: &mut Self::Store,
        future: &Self::Initialize,
    ) -> Result<Self::PollExecutionResult, Self::Error>;

    /// Creates a new future for the user application to execute an operation.
    fn execute_operation_new(
        &self,
        store: &mut Self::Store,
        context: OperationContext,
        operation: Vec<u8>,
    ) -> Result<Self::ExecuteOperation, Self::Error>;

    /// Polls a user contract future that's executing an operation.
    fn execute_operation_poll(
        &self,
        store: &mut Self::Store,
        future: &Self::ExecuteOperation,
    ) -> Result<Self::PollExecutionResult, Self::Error>;

    /// Creates a new future for the user contract to execute a message.
    fn execute_message_new(
        &self,
        store: &mut Self::Store,
        context: MessageContext,
        message: Vec<u8>,
    ) -> Result<Self::ExecuteMessage, Self::Error>;

    /// Polls a user contract future that's executing a message.
    fn execute_message_poll(
        &self,
        store: &mut Self::Store,
        future: &Self::ExecuteMessage,
    ) -> Result<Self::PollExecutionResult, Self::Error>;

    /// Creates a new future for the user contract to handle a call from another contract.
    fn handle_application_call_new(
        &self,
        store: &mut Self::Store,
        context: CalleeContext,
        argument: Vec<u8>,
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<Self::HandleApplicationCall, Self::Error>;

    /// Polls a user contract future that's handling a call from another contract.
    fn handle_application_call_poll(
        &self,
        store: &mut Self::Store,
        future: &Self::HandleApplicationCall,
    ) -> Result<Self::PollCallApplication, Self::Error>;

    /// Creates a new future for the user contract to handle a session call from another
    /// contract.
    fn handle_session_call_new(
        &self,
        store: &mut Self::Store,
        context: CalleeContext,
        session_state: Vec<u8>,
        argument: Vec<u8>,
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<Self::HandleSessionCall, Self::Error>;

    /// Polls a user contract future that's handling a session call from another contract.
    fn handle_session_call_poll(
        &self,
        store: &mut Self::Store,
        future: &Self::HandleSessionCall,
    ) -> Result<Self::PollCallSession, Self::Error>;
}

/// Common interface to calling a user service in a WebAssembly module.
pub trait Service: ApplicationRuntimeContext {
    /// The WIT type for the resource representing the guest future
    /// [`query_application`][crate::Service::query_application] method.
    type QueryApplication: GuestFutureInterface<Self, Output = Vec<u8>> + Send + Unpin;

    /// The WIT type eqivalent for [`Poll<Result<Vec<u8>, String>>`].
    type PollQuery;

    /// Creates a new future for the user application to handle a query.
    fn query_application_new(
        &self,
        store: &mut Self::Store,
        context: QueryContext,
        argument: Vec<u8>,
    ) -> Result<Self::QueryApplication, Self::Error>;

    /// Polls a user service future that's handling a query.
    fn query_application_poll(
        &self,
        store: &mut Self::Store,
        future: &Self::QueryApplication,
    ) -> Result<Self::PollQuery, Self::Error>;
}

/// Wrapper around all types necessary to call an asynchronous method of a Wasm application.
pub struct WasmRuntimeContext<A>
where
    A: ApplicationRuntimeContext,
{
    /// The application type.
    pub(crate) application: A,

    /// A queue of host futures called by the guest that must complete deterministically.
    pub(crate) future_queue: HostFutureQueue,

    /// The application's memory state.
    pub(crate) store: A::Store,

    /// Guard type to clean up any host state after the call to the Wasm application finishes.
    #[allow(dead_code)]
    pub(crate) extra: A::Extra,
}

impl<A> WasmRuntimeContext<A>
where
    A: Contract + Send + Unpin + 'static,
{
    /// Calls the guest Wasm module's implementation of
    /// [`UserApplication::initialize`][`linera_execution::UserApplication::initialize`].
    ///
    /// This method returns a [`Future`][`std::future::Future`], and is equivalent to
    ///
    /// ```ignore
    /// pub async fn initialize(
    ///     mut self,
    ///     context: &OperationContext,
    ///     argument: &[u8],
    /// ) -> Result<RawExecutionResult<Vec<u8>>, ExecutionError>
    /// ```
    pub fn initialize(
        mut self,
        context: &OperationContext,
        argument: &[u8],
    ) -> impl Future<Output = Result<RawExecutionResult<Vec<u8>>, ExecutionError>> {
        A::configure_fuel(&mut self);

        future::ready(self.application.initialize_new(
            &mut self.store,
            *context,
            argument.to_owned(),
        ))
        .err_into()
        .and_then(move |future| GuestFutureActor::spawn(future, self))
    }

    /// Calls the guest Wasm module's implementation of
    /// [`UserApplication::execute_operation`][`linera_execution::UserApplication::execute_operation`].
    ///
    /// This method returns a [`Future`][`std::future::Future`], and is equivalent to
    ///
    /// ```ignore
    /// pub async fn execute_operation(
    ///     mut self,
    ///     context: &OperationContext,
    ///     operation: &[u8],
    /// ) -> Result<RawExecutionResult<Vec<u8>>, ExecutionError>
    /// ```
    pub fn execute_operation(
        mut self,
        context: &OperationContext,
        operation: &[u8],
    ) -> impl Future<Output = Result<RawExecutionResult<Vec<u8>>, ExecutionError>> {
        A::configure_fuel(&mut self);

        future::ready(self.application.execute_operation_new(
            &mut self.store,
            *context,
            operation.to_owned(),
        ))
        .err_into()
        .and_then(|future| GuestFutureActor::spawn(future, self))
    }

    /// Calls the guest Wasm module's implementation of
    /// [`UserApplication::execute_message`][`linera_execution::UserApplication::execute_message`].
    ///
    /// This method returns a [`Future`][`std::future::Future`], and is equivalent to
    ///
    /// ```ignore
    /// pub async fn execute_message(
    ///     mut self,
    ///     context: &MessageContext,
    ///     message: &[u8],
    /// ) -> Result<RawExecutionResult<Vec<u8>>, ExecutionError>
    /// ```
    pub fn execute_message(
        mut self,
        context: &MessageContext,
        message: &[u8],
    ) -> impl Future<Output = Result<RawExecutionResult<Vec<u8>>, ExecutionError>> {
        A::configure_fuel(&mut self);

        future::ready(self.application.execute_message_new(
            &mut self.store,
            *context,
            message.to_owned(),
        ))
        .err_into()
        .and_then(|future| GuestFutureActor::spawn(future, self))
    }

    /// Calls the guest Wasm module's implementation of
    /// [`UserApplication::handle_application_call`][`linera_execution::UserApplication::handle_application_call`].
    ///
    /// This method returns a [`Future`][`std::future::Future`], and is equivalent to
    ///
    /// ```ignore
    /// pub async fn handle_application_call(
    ///     mut self,
    ///     context: &CalleeContext,
    ///     argument: &[u8],
    ///     forwarded_sessions: Vec<SessionId>,
    /// ) -> Result<ApplicationCallResult, ExecutionError>
    /// ```
    pub fn handle_application_call(
        mut self,
        context: &CalleeContext,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> impl Future<Output = Result<ApplicationCallResult, ExecutionError>> {
        A::configure_fuel(&mut self);

        future::ready(self.application.handle_application_call_new(
            &mut self.store,
            *context,
            argument.to_owned(),
            forwarded_sessions,
        ))
        .err_into()
        .and_then(|future| GuestFutureActor::spawn(future, self))
    }

    /// Calls the guest Wasm module's implementation of
    /// [`UserApplication::handle_session_call`][`linera_execution::UserApplication::handle_session_call`].
    ///
    /// This method returns a [`Future`][`std::future::Future`], and is equivalent to
    ///
    /// ```ignore
    /// pub async fn handle_session_call(
    ///     mut self,
    ///     context: &CalleeContext,
    ///     session_state: &[u8],
    ///     argument: &[u8],
    ///     forwarded_sessions: Vec<SessionId>,
    /// ) -> Result<(SessionCallResult, Vec<u8>), ExecutionError>
    /// ```
    pub fn handle_session_call(
        mut self,
        context: &CalleeContext,
        session_state: &[u8],
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> impl Future<Output = Result<(SessionCallResult, Vec<u8>), ExecutionError>> {
        A::configure_fuel(&mut self);

        future::ready(self.application.handle_session_call_new(
            &mut self.store,
            *context,
            session_state.to_owned(),
            argument.to_owned(),
            forwarded_sessions,
        ))
        .err_into()
        .and_then(|future| GuestFutureActor::spawn(future, self))
    }
}

impl<A> WasmRuntimeContext<A>
where
    A: Service + Send + Unpin + 'static,
{
    /// Calls the guest Wasm module's implementation of
    /// [`UserApplication::query_application`][`linera_execution::UserApplication::query_application`].
    ///
    /// This method returns a [`Future`][`std::future::Future`], and is equivalent to
    ///
    /// ```ignore
    /// pub async fn query_application(
    ///     mut self,
    ///     context: &QueryContext,
    ///     argument: &[u8],
    /// ) -> Result<Vec<u8>, ExecutionError>
    /// ```
    pub fn query_application(
        mut self,
        context: &QueryContext,
        argument: &[u8],
    ) -> impl Future<Output = Result<Vec<u8>, ExecutionError>> {
        future::ready(self.application.query_application_new(
            &mut self.store,
            *context,
            argument.to_owned(),
        ))
        .err_into()
        .and_then(|future| GuestFutureActor::spawn(future, self))
    }
}

impl<A> Drop for WasmRuntimeContext<A>
where
    A: ApplicationRuntimeContext,
{
    fn drop(&mut self) {
        A::finalize(self);
    }
}
