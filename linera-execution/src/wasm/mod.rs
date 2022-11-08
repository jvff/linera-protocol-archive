// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Support for user applications compiled as WebAssembly (WASM) modules.
//!
//! Requires a WebAssembly runtime to be selected and enabled using one of the following features:
//!
//! - `wasmer` enables the [Wasmer](https://wasmer.io/) runtime
//! - `wasmtime` enables the [Wasmtime](https://wasmtime.dev/) runtime

#![cfg(any(feature = "wasmer", feature = "wasmtime"))]

mod async_boundary;
mod common;
#[cfg(feature = "wasmer")]
#[path = "wasmer.rs"]
mod runtime;
#[cfg(feature = "wasmtime")]
#[path = "wasmtime.rs"]
mod runtime;

use crate::{
    ApplicationCallResult, CalleeContext, EffectContext, ExecutionError, OperationContext,
    QueryContext, QueryableStorage, RawExecutionResult, SessionCallResult, SessionId,
    UserApplication, WritableStorage,
};
use async_trait::async_trait;
use std::path::PathBuf;

/// A user application in a compiled WebAssembly module.
pub struct WasmApplication {
    bytecode_file: PathBuf,
}

impl WasmApplication {
    /// Create a new [`WasmApplication`] using the WebAssembly module in `bytecode_file`.
    pub fn new(bytecode_file: impl Into<PathBuf>) -> Self {
        WasmApplication {
            bytecode_file: bytecode_file.into(),
        }
    }
}

#[async_trait]
impl UserApplication for WasmApplication {
    async fn execute_operation(
        &self,
        context: &OperationContext,
        storage: &dyn WritableStorage,
        operation: &[u8],
    ) -> Result<RawExecutionResult<Vec<u8>>, ExecutionError> {
        todo!();
    }

    async fn execute_effect(
        &self,
        context: &EffectContext,
        storage: &dyn WritableStorage,
        effect: &[u8],
    ) -> Result<RawExecutionResult<Vec<u8>>, ExecutionError> {
        todo!();
    }

    async fn call_application(
        &self,
        context: &CalleeContext,
        storage: &dyn WritableStorage,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<ApplicationCallResult, ExecutionError> {
        todo!();
    }

    async fn call_session(
        &self,
        context: &CalleeContext,
        storage: &dyn WritableStorage,
        session_kind: u64,
        session_data: &mut Vec<u8>,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<SessionCallResult, ExecutionError> {
        todo!();
    }

    async fn query_application(
        &self,
        context: &QueryContext,
        storage: &dyn QueryableStorage,
        argument: &[u8],
    ) -> Result<Vec<u8>, ExecutionError> {
        todo!();
    }
}
