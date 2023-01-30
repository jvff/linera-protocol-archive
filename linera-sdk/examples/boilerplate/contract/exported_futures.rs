// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Types for the exported futures for the contract endpoints.
//!
//! Each type is called by the code generated by [`wit_bindgen_rust`] when the host calls the guest
//! WASM module's respective endpoint. This module contains the code to forward the call to the
//! contract type that implements [`linera_sdk::Contract`].

use super::{super::ApplicationState, contract};
use linera_sdk::{
    ApplicationCallResult, Contract, ContractLogger, ExecutionResult, ExportedFuture,
    SessionCallResult, SessionId,
};
use wit_bindgen_guest_rust::Handle;

pub struct Initialize {
    future: ExportedFuture<Result<ExecutionResult, String>>,
}

impl contract::Initialize for Initialize {
    fn new(context: contract::OperationContext, argument: Vec<u8>) -> Handle<Self> {
        ContractLogger::install();

        Handle::new(Initialize {
            future: ExportedFuture::new(async move {
                let mut application = ApplicationState::load_and_lock().await;
                let result = application.initialize(&context.into(), &argument).await;
                if result.is_ok() {
                    application.store_and_unlock().await;
                }
                result.map_err(|error| error.to_string())
            }),
        })
    }

    fn poll(&self) -> contract::PollExecutionResult {
        self.future.poll()
    }
}

pub struct ExecuteOperation {
    future: ExportedFuture<Result<ExecutionResult, String>>,
}

impl contract::ExecuteOperation for ExecuteOperation {
    fn new(context: contract::OperationContext, operation: Vec<u8>) -> Handle<Self> {
        ContractLogger::install();

        Handle::new(ExecuteOperation {
            future: ExportedFuture::new(async move {
                let mut application = ApplicationState::load_and_lock().await;
                let result = application
                    .execute_operation(&context.into(), &operation)
                    .await;
                if result.is_ok() {
                    application.store_and_unlock().await;
                }
                result.map_err(|error| error.to_string())
            }),
        })
    }

    fn poll(&self) -> contract::PollExecutionResult {
        self.future.poll()
    }
}

pub struct ExecuteEffect {
    future: ExportedFuture<Result<ExecutionResult, String>>,
}

impl contract::ExecuteEffect for ExecuteEffect {
    fn new(context: contract::EffectContext, effect: Vec<u8>) -> Handle<Self> {
        ContractLogger::install();

        Handle::new(ExecuteEffect {
            future: ExportedFuture::new(async move {
                let mut application = ApplicationState::load_and_lock().await;
                let result = application.execute_effect(&context.into(), &effect).await;
                if result.is_ok() {
                    application.store_and_unlock().await;
                }
                result.map_err(|error| error.to_string())
            }),
        })
    }

    fn poll(&self) -> contract::PollExecutionResult {
        self.future.poll()
    }
}

pub struct CallApplication {
    future: ExportedFuture<Result<ApplicationCallResult, String>>,
}

impl contract::CallApplication for CallApplication {
    fn new(
        context: contract::CalleeContext,
        argument: Vec<u8>,
        forwarded_sessions: Vec<contract::SessionId>,
    ) -> Handle<Self> {
        ContractLogger::install();

        Handle::new(CallApplication {
            future: ExportedFuture::new(async move {
                let mut application = ApplicationState::load_and_lock().await;

                let forwarded_sessions = forwarded_sessions
                    .into_iter()
                    .map(SessionId::from)
                    .collect();

                let result = application
                    .call_application(&context.into(), &argument, forwarded_sessions)
                    .await;
                if result.is_ok() {
                    application.store_and_unlock().await;
                }
                result.map_err(|error| error.to_string())
            }),
        })
    }

    fn poll(&self) -> contract::PollCallApplication {
        self.future.poll()
    }
}

pub struct CallSession {
    future: ExportedFuture<Result<SessionCallResult, String>>,
}

impl contract::CallSession for CallSession {
    fn new(
        context: contract::CalleeContext,
        session: contract::Session,
        argument: Vec<u8>,
        forwarded_sessions: Vec<contract::SessionId>,
    ) -> Handle<Self> {
        ContractLogger::install();

        Handle::new(CallSession {
            future: ExportedFuture::new(async move {
                let mut application = ApplicationState::load_and_lock().await;

                let forwarded_sessions = forwarded_sessions
                    .into_iter()
                    .map(SessionId::from)
                    .collect();

                let result = application
                    .call_session(
                        &context.into(),
                        session.into(),
                        &argument,
                        forwarded_sessions,
                    )
                    .await;
                if result.is_ok() {
                    application.store_and_unlock().await;
                }
                result.map_err(|error| error.to_string())
            }),
        })
    }

    fn poll(&self) -> contract::PollCallSession {
        self.future.poll()
    }
}
