// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Wasm entrypoints for contracts and services.

use crate::{
    ApplicationCallOutcome, CalleeContext, MessageContext, OperationContext, QueryContext,
    RawExecutionOutcome, SessionCallOutcome, SessionId,
};

#[linera_witty::wit_import(package = "linera:app")]
pub trait ContractEntrypoints {
    fn initialize(
        context: OperationContext,
        argument: Vec<u8>,
    ) -> Result<RawExecutionOutcome<Vec<u8>>, String>;

    fn execute_operation(
        context: OperationContext,
        operation: Vec<u8>,
    ) -> Result<RawExecutionOutcome<Vec<u8>>, String>;

    fn execute_message(
        context: MessageContext,
        message: Vec<u8>,
    ) -> Result<RawExecutionOutcome<Vec<u8>>, String>;

    fn handle_application_call(
        context: CalleeContext,
        argument: Vec<u8>,
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<ApplicationCallOutcome, String>;

    fn handle_session_call(
        context: CalleeContext,
        session_state: Vec<u8>,
        argument: Vec<u8>,
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<(SessionCallOutcome, Vec<u8>), String>;
}

#[linera_witty::wit_import(package = "linera:app")]
pub trait ServiceEntrypoints {
    fn handle_query(context: QueryContext, argument: Vec<u8>) -> Result<Vec<u8>, String>;
}
