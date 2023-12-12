// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Wasm entrypoints for contracts and services.

use crate::{
    ApplicationCallResult, CalleeContext, MessageContext, OperationContext, QueryContext,
    RawExecutionResult, SessionCallResult, SessionId,
};

#[linera_witty::wit_import(package = "linera")]
pub trait ContractEntrypoints {
    fn initialize(
        context: OperationContext,
        argument: Vec<u8>,
    ) -> Result<RawExecutionResult<Vec<u8>>, String>;

    fn execute_operation(
        context: OperationContext,
        operation: Vec<u8>,
    ) -> Result<RawExecutionResult<Vec<u8>>, String>;

    fn execute_message(
        context: MessageContext,
        message: Vec<u8>,
    ) -> Result<RawExecutionResult<Vec<u8>>, String>;

    fn handle_application_call(
        context: CalleeContext,
        argument: Vec<u8>,
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<ApplicationCallResult, String>;

    fn handle_session_call(
        context: CalleeContext,
        session_state: Vec<u8>,
        argument: Vec<u8>,
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<(SessionCallResult, Vec<u8>), String>;
}

#[linera_witty::wit_import(package = "linera")]
pub trait ServiceEntrypoints {
    fn handle_query(context: QueryContext, argument: Vec<u8>) -> Result<Vec<u8>, String>;
}
