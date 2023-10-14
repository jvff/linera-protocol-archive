// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Support for user applications compiled as WebAssembly (Wasm) modules.
//!
//! Requires a WebAssembly runtime to be selected and enabled using one of the following features:
//!
//! - `wasmer` enables the [Wasmer](https://wasmer.io/) runtime
//! - `wasmtime` enables the [Wasmtime](https://wasmtime.dev/) runtime

#![cfg(any(feature = "wasmer", feature = "wasmtime"))]

mod async_boundary;
mod async_determinism;
mod common;
mod module_cache;
mod runtime_actor;
mod sanitizer;
#[macro_use]
mod system_api;
#[cfg(feature = "wasmer")]
#[path = "wasmer.rs"]
mod wasmer;
#[cfg(feature = "wasmtime")]
#[path = "wasmtime.rs"]
mod wasmtime;

use self::{runtime_actor::RuntimeActor, sanitizer::sanitize};
use crate::{
    ApplicationCallResult, Bytecode, CalleeContext, ContractRuntime, ExecutionError,
    MessageContext, OperationContext, QueryContext, RawExecutionResult, ServiceRuntime,
    SessionCallResult, SessionId, UserApplication, WasmRuntime,
};
use async_trait::async_trait;
use std::{path::Path, sync::Arc};
use thiserror::Error;

/// A user application in a compiled WebAssembly module.
pub enum WasmApplication {
    #[cfg(feature = "wasmer")]
    Wasmer {
        contract: (::wasmer::Engine, ::wasmer::Module),
        service: Arc<::wasmer::Module>,
    },
    #[cfg(feature = "wasmtime")]
    Wasmtime {
        contract: Arc<::wasmtime::Module>,
        service: Arc<::wasmtime::Module>,
    },
}

impl WasmApplication {
    /// Creates a new [`WasmApplication`] using the WebAssembly module with the provided bytecodes.
    pub async fn new(
        contract_bytecode: Bytecode,
        service_bytecode: Bytecode,
        runtime: WasmRuntime,
    ) -> Result<Self, WasmExecutionError> {
        let contract_bytecode = if runtime.needs_sanitizer() {
            // Ensure bytecode normalization whenever wasmer and wasmtime are possibly
            // compared.
            sanitize(contract_bytecode).map_err(WasmExecutionError::LoadContractModule)?
        } else {
            contract_bytecode
        };
        match runtime {
            #[cfg(feature = "wasmer")]
            WasmRuntime::Wasmer | WasmRuntime::WasmerWithSanitizer => {
                Self::new_with_wasmer(contract_bytecode, service_bytecode).await
            }
            #[cfg(feature = "wasmtime")]
            WasmRuntime::Wasmtime | WasmRuntime::WasmtimeWithSanitizer => {
                Self::new_with_wasmtime(contract_bytecode, service_bytecode).await
            }
        }
    }

    /// Creates a new [`WasmApplication`] using the WebAssembly module in `bytecode_file`.
    pub async fn from_files(
        contract_bytecode_file: impl AsRef<Path>,
        service_bytecode_file: impl AsRef<Path>,
        runtime: WasmRuntime,
    ) -> Result<Self, WasmExecutionError> {
        WasmApplication::new(
            Bytecode::load_from_file(contract_bytecode_file)
                .await
                .map_err(anyhow::Error::from)
                .map_err(WasmExecutionError::LoadContractModule)?,
            Bytecode::load_from_file(service_bytecode_file)
                .await
                .map_err(anyhow::Error::from)
                .map_err(WasmExecutionError::LoadServiceModule)?,
            runtime,
        )
        .await
    }
}

/// Errors that can occur when executing a user application in a WebAssembly module.
#[cfg(any(feature = "wasmer", feature = "wasmtime"))]
#[derive(Debug, Error)]
pub enum WasmExecutionError {
    #[error("Failed to load contract Wasm module: {_0}")]
    LoadContractModule(#[source] anyhow::Error),
    #[error("Failed to load service Wasm module: {_0}")]
    LoadServiceModule(#[source] anyhow::Error),
    #[cfg(feature = "wasmtime")]
    #[error("Failed to create and configure Wasmtime runtime")]
    CreateWasmtimeEngine(#[source] anyhow::Error),
    #[cfg(feature = "wasmer")]
    #[error("Failed to execute Wasm module (Wasmer)")]
    ExecuteModuleInWasmer(#[from] ::wasmer::RuntimeError),
    #[cfg(feature = "wasmtime")]
    #[error("Failed to execute Wasm module (Wasmtime)")]
    ExecuteModuleInWasmtime(#[from] ::wasmtime::Trap),
    #[error("Attempt to use a system API to write to read-only storage")]
    WriteAttemptToReadOnlyStorage,
    #[error("Runtime failed to respond to application")]
    MissingRuntimeResponse,
    #[error("Execution of guest future was aborted")]
    Aborted,
}

#[async_trait]
impl UserApplication for WasmApplication {
    async fn initialize(
        &self,
        context: &OperationContext,
        runtime: &dyn ContractRuntime,
        argument: &[u8],
    ) -> Result<RawExecutionResult<Vec<u8>>, ExecutionError> {
        use tracing::Instrument;
        let span = tracing::info_span!("WasmApplication::initialize");
        let _guard = span.enter();
        tracing::info!("Starting");
        let (runtime_actor, runtime_requests) = RuntimeActor::new(runtime);
        let context = *context;
        let argument = argument.to_owned();

        let wasm_task = match self {
            #[cfg(feature = "wasmtime")]
            WasmApplication::Wasmtime { contract, .. } => {
                tracing::info!("Preparing contract runtime with wasmtime");
                let instance =
                    Self::prepare_contract_runtime_with_wasmtime(contract, runtime_requests)?;
                let subspan = tracing::info_span!("WasmExecutionContext::initialize");

                tokio::spawn(async move {
                    instance
                        .initialize(&context, &argument)
                        .instrument(subspan)
                        .await
                })
            }
            #[cfg(feature = "wasmer")]
            WasmApplication::Wasmer { contract, .. } => {
                tracing::info!("Preparing contract runtime with wasmer");
                let instance =
                    Self::prepare_contract_runtime_with_wasmer(contract, runtime_requests)?;
                let subspan = tracing::info_span!("WasmExecutionContext::initialize");

                tokio::spawn(async move {
                    instance
                        .initialize(&context, &argument)
                        .instrument(subspan)
                        .await
                })
            }
        };
        tracing::info!("Running actor");

        runtime_actor
            .run()
            .instrument(tracing::info_span!("RuntimeActor"))
            .await?;
        tracing::info!("Waiting for Wasm task");
        let ret = wasm_task
            .await
            .expect("Panic while running Wasm guest instance");
        tracing::info!("Finished");
        ret
    }

    async fn execute_operation(
        &self,
        context: &OperationContext,
        runtime: &dyn ContractRuntime,
        operation: &[u8],
    ) -> Result<RawExecutionResult<Vec<u8>>, ExecutionError> {
        let (runtime_actor, runtime_requests) = RuntimeActor::new(runtime);
        let context = *context;
        let operation = operation.to_owned();

        let wasm_task = match self {
            #[cfg(feature = "wasmtime")]
            WasmApplication::Wasmtime { contract, .. } => {
                let instance =
                    Self::prepare_contract_runtime_with_wasmtime(contract, runtime_requests)?;

                tokio::spawn(async move { instance.execute_operation(&context, &operation).await })
            }
            #[cfg(feature = "wasmer")]
            WasmApplication::Wasmer { contract, .. } => {
                let instance =
                    Self::prepare_contract_runtime_with_wasmer(contract, runtime_requests)?;

                tokio::spawn(async move { instance.execute_operation(&context, &operation).await })
            }
        };

        runtime_actor.run().await?;
        wasm_task
            .await
            .expect("Panic while running Wasm guest instance")
    }

    async fn execute_message(
        &self,
        context: &MessageContext,
        runtime: &dyn ContractRuntime,
        message: &[u8],
    ) -> Result<RawExecutionResult<Vec<u8>>, ExecutionError> {
        let (runtime_actor, runtime_requests) = RuntimeActor::new(runtime);
        let context = *context;
        let message = message.to_owned();

        let wasm_task = match self {
            #[cfg(feature = "wasmtime")]
            WasmApplication::Wasmtime { contract, .. } => {
                let instance =
                    Self::prepare_contract_runtime_with_wasmtime(contract, runtime_requests)?;

                tokio::spawn(async move { instance.execute_message(&context, &message).await })
            }
            #[cfg(feature = "wasmer")]
            WasmApplication::Wasmer { contract, .. } => {
                let instance =
                    Self::prepare_contract_runtime_with_wasmer(contract, runtime_requests)?;

                tokio::spawn(async move { instance.execute_message(&context, &message).await })
            }
        };

        runtime_actor.run().await?;
        wasm_task
            .await
            .expect("Panic while running Wasm guest instance")
    }

    async fn handle_application_call(
        &self,
        context: &CalleeContext,
        runtime: &dyn ContractRuntime,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<ApplicationCallResult, ExecutionError> {
        let (runtime_actor, runtime_requests) = RuntimeActor::new(runtime);
        let context = *context;
        let argument = argument.to_owned();

        let wasm_task = match self {
            #[cfg(feature = "wasmtime")]
            WasmApplication::Wasmtime { contract, .. } => {
                let instance =
                    Self::prepare_contract_runtime_with_wasmtime(contract, runtime_requests)?;

                tokio::spawn(async move {
                    instance
                        .handle_application_call(&context, &argument, forwarded_sessions)
                        .await
                })
            }
            #[cfg(feature = "wasmer")]
            WasmApplication::Wasmer { contract, .. } => {
                let instance =
                    Self::prepare_contract_runtime_with_wasmer(contract, runtime_requests)?;

                tokio::spawn(async move {
                    instance
                        .handle_application_call(&context, &argument, forwarded_sessions)
                        .await
                })
            }
        };

        runtime_actor.run().await?;
        wasm_task
            .await
            .expect("Panic while running Wasm guest instance")
    }

    async fn handle_session_call(
        &self,
        context: &CalleeContext,
        runtime: &dyn ContractRuntime,
        session_state: &mut Vec<u8>,
        argument: &[u8],
        forwarded_sessions: Vec<SessionId>,
    ) -> Result<SessionCallResult, ExecutionError> {
        let (runtime_actor, runtime_requests) = RuntimeActor::new(runtime);
        let context = *context;
        let argument = argument.to_owned();
        let initial_session_state = session_state.to_owned();

        let wasm_task = match self {
            #[cfg(feature = "wasmtime")]
            WasmApplication::Wasmtime { contract, .. } => {
                let instance =
                    Self::prepare_contract_runtime_with_wasmtime(contract, runtime_requests)?;

                tokio::spawn(async move {
                    instance
                        .handle_session_call(
                            &context,
                            &initial_session_state,
                            &argument,
                            forwarded_sessions,
                        )
                        .await
                })
            }
            #[cfg(feature = "wasmer")]
            WasmApplication::Wasmer { contract, .. } => {
                let instance =
                    Self::prepare_contract_runtime_with_wasmer(contract, runtime_requests)?;

                tokio::spawn(async move {
                    instance
                        .handle_session_call(
                            &context,
                            &initial_session_state,
                            &argument,
                            forwarded_sessions,
                        )
                        .await
                })
            }
        };

        runtime_actor.run().await?;
        let (result, updated_session_state) = wasm_task
            .await
            .expect("Panic while running Wasm guest instance")?;
        *session_state = updated_session_state;
        Ok(result)
    }

    async fn query_application(
        &self,
        context: &QueryContext,
        runtime: &dyn ServiceRuntime,
        argument: &[u8],
    ) -> Result<Vec<u8>, ExecutionError> {
        use tracing::Instrument;
        let span = tracing::info_span!("WasmApplication::query_application");
        let _guard = span.enter();
        tracing::info!("Starting");
        let (runtime_actor, runtime_requests) = RuntimeActor::new(runtime);
        let context = *context;
        let argument = argument.to_owned();

        let wasm_task = match self {
            #[cfg(feature = "wasmtime")]
            WasmApplication::Wasmtime { service, .. } => {
                tracing::info!("Preparing contract runtime with wasmtime");
                let instance =
                    Self::prepare_service_runtime_with_wasmtime(service, runtime_requests)?;
                let subspan = tracing::info_span!("WasmExecutionContext::query_application");

                tokio::spawn(async move {
                    instance
                        .query_application(&context, &argument)
                        .instrument(subspan)
                        .await
                })
            }
            #[cfg(feature = "wasmer")]
            WasmApplication::Wasmer { service, .. } => {
                tracing::info!("Preparing contract runtime with wasmer");
                let instance =
                    Self::prepare_service_runtime_with_wasmer(service, runtime_requests)?;
                let subspan = tracing::info_span!("WasmExecutionContext::query_application");

                tokio::spawn(async move {
                    instance
                        .query_application(&context, &argument)
                        .instrument(subspan)
                        .await
                })
            }
        };

        tracing::info!("Running actor");
        runtime_actor
            .run()
            .instrument(tracing::info_span!("RuntimeActor"))
            .await?;
        tracing::info!("Waiting for Wasm task");
        let ret = wasm_task
            .await
            .expect("Panic while running Wasm guest instance");
        tracing::info!("Finished");
        ret
    }
}

/// This assumes that the current directory is one of the crates.
#[cfg(any(test, feature = "test"))]
pub mod test {
    use crate::{WasmApplication, WasmRuntime};
    use once_cell::sync::OnceCell;

    fn build_applications() -> Result<(), std::io::Error> {
        tracing::info!("Building example applications with cargo");
        let output = std::process::Command::new("cargo")
            .current_dir("../examples")
            .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
            .output()?;
        if !output.status.success() {
            panic!(
                "Failed to build example applications.\n\n\
                stdout:\n-------\n{}\n\n\
                stderr:\n-------\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
        Ok(())
    }

    pub fn get_example_bytecode_paths(name: &str) -> Result<(String, String), std::io::Error> {
        let name = name.replace('-', "_");
        static INSTANCE: OnceCell<()> = OnceCell::new();
        INSTANCE.get_or_try_init(build_applications)?;
        Ok((
            format!("../examples/target/wasm32-unknown-unknown/release/{name}_contract.wasm"),
            format!("../examples/target/wasm32-unknown-unknown/release/{name}_service.wasm"),
        ))
    }

    pub async fn build_example_application(
        name: &str,
        wasm_runtime: impl Into<Option<WasmRuntime>>,
    ) -> Result<WasmApplication, anyhow::Error> {
        let (contract, service) = get_example_bytecode_paths(name)?;
        let application = WasmApplication::from_files(
            &contract,
            &service,
            wasm_runtime.into().unwrap_or_default(),
        )
        .await?;
        Ok(application)
    }
}
