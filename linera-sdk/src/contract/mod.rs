// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Types and macros useful for writing an application contract.

mod storage;
#[cfg(target_arch = "wasm32")]
pub mod system_api;
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_arch = "wasm32"), path = "system_api_stubs.rs")]
pub mod system_api;

pub use self::storage::ContractStateStorage;
use super::log::ContractLogger;
use crate::{util::BlockingWait, Contract};
use std::future::Future;

// Import the system interface.
// wit_bindgen_guest_rust::import!("contract_system_api.wit");

/// Declares an implementation of the [`Contract`][`crate::Contract`] trait, exporting it from the
/// Wasm module.
///
/// Generates the necessary boilerplate for implementing the contract WIT interface, exporting the
/// necessary resource types and functions so that the host can call the contract application.
#[macro_export]
macro_rules! contract {
    ($application:ty) => {
        // Export the contract interface.
        #[cfg(target_arch = "wasm32")]
        #[export_name = "initialize"]
        extern "C" fn initialize(
            parameters_area: i32,
            // context: $crate::contract::wit_types::OperationContext,
            // argument: Vec<u8>,
        ) {
            // ) -> Result<$crate::contract::wit_types::ExecutionResult, String> {
            use $crate::witty::{guest::Guest, GuestPointer, InstanceWithMemory, WitLoad};

            let mut guest = Guest::default();
            let mut memory = guest
                .memory()
                .expect("Failed to create guest `Memory` instance");

            let (context, argument) = <($crate::OperationContext, Vec<u8>) as WitLoad>::load(
                &memory,
                GuestPointer::from(parameters_area),
            )
            .expect("Failed to load `initialize` parameters");

            $crate::contract::run_async_entrypoint::<$application, _, _, _>(
                move |mut application| async move {
                    let argument = serde_json::from_slice(&argument)?;

                    application
                        .initialize(&context, argument)
                        .await
                        .map(|result| (application, result))
                },
            );
        }

        #[cfg(target_arch = "wasm32")]
        #[export_name = "execute-operation"]
        extern "C" fn execute_operation(
            parameters_area: i32,
            // context: $crate::contract::wit_types::OperationContext,
            // operation: Vec<u8>,
        ) {
            // ) -> Result<$crate::contract::wit_types::ExecutionResult, String> {
            use $crate::witty::{guest::Guest, GuestPointer, InstanceWithMemory, WitLoad};

            let mut guest = Guest::default();
            let mut memory = guest
                .memory()
                .expect("Failed to create guest `Memory` instance");

            let (context, operation) = <($crate::OperationContext, Vec<u8>) as WitLoad>::load(
                &memory,
                GuestPointer::from(parameters_area),
            )
            .expect("Failed to load `execute_operation` parameters");

            $crate::contract::run_async_entrypoint::<$application, _, _, _>(
                move |mut application| async move {
                    let operation: <$application as $crate::abi::ContractAbi>::Operation =
                        bcs::from_bytes(&operation)?;

                    application
                        .execute_operation(&context, operation)
                        .await
                        .map(|result| (application, result))
                },
            );
        }

        #[cfg(target_arch = "wasm32")]
        #[export_name = "execute-message"]
        extern "C" fn execute_message(
            parameters_area: i32,
            // context: $crate::contract::wit_types::MessageContext,
            // message: Vec<u8>,
        ) {
            // ) -> Result<$crate::contract::wit_types::ExecutionResult, String> {
            use $crate::witty::{guest::Guest, GuestPointer, InstanceWithMemory, WitLoad};

            let mut guest = Guest::default();
            let mut memory = guest
                .memory()
                .expect("Failed to create guest `Memory` instance");

            let (context, message) = <($crate::MessageContext, Vec<u8>) as WitLoad>::load(
                &memory,
                GuestPointer::from(parameters_area),
            )
            .expect("Failed to load `execute_message` parameters");

            $crate::contract::run_async_entrypoint::<$application, _, _, _>(
                move |mut application| async move {
                    let message: <$application as $crate::abi::ContractAbi>::Message =
                        bcs::from_bytes(&message)?;

                    application
                        .execute_message(&context, message)
                        .await
                        .map(|result| (application, result))
                },
            );
        }

        #[cfg(target_arch = "wasm32")]
        #[export_name = "handle-application-call"]
        extern "C" fn handle_application_call(
            parameters_area: i32,
            // context: $crate::contract::wit_types::CalleeContext,
            // argument: Vec<u8>,
            // forwarded_sessions: Vec<$crate::contract::wit_types::SessionId>,
        ) {
            // ) -> Result<$crate::contract::wit_types::ApplicationCallResult, String> {
            use $crate::witty::{guest::Guest, GuestPointer, InstanceWithMemory, WitLoad};

            let mut guest = Guest::default();
            let mut memory = guest
                .memory()
                .expect("Failed to create guest `Memory` instance");

            let (context, argument, forwarded_sessions) =
                <($crate::CalleeContext, Vec<u8>, Vec<$crate::SessionId>) as WitLoad>::load(
                    &memory,
                    GuestPointer::from(parameters_area),
                )
                .expect("Failed to load `handle_application_call` parameters");

            $crate::contract::run_async_entrypoint::<$application, _, _, _>(
                move |mut application| async move {
                    let argument: <$application as $crate::abi::ContractAbi>::ApplicationCall =
                        bcs::from_bytes(&argument)?;

                    application
                        .handle_application_call(&context, argument, forwarded_sessions)
                        .await
                        .map(|result| (application, result))
                },
            );
        }

        #[cfg(target_arch = "wasm32")]
        #[export_name = "handle-session-call"]
        extern "C" fn handle_session_call(
            parameters_area: i32,
            // context: $crate::contract::wit_types::CalleeContext,
            // session_state: Vec<u8>,
            // argument: Vec<u8>,
            // forwarded_sessions: Vec<$crate::contract::wit_types::SessionId>,
        ) {
            // ) -> Result<$crate::contract::wit_types::SessionCallResult, String> {
            use $crate::witty::{guest::Guest, GuestPointer, InstanceWithMemory, WitLoad};

            let mut guest = Guest::default();
            let mut memory = guest
                .memory()
                .expect("Failed to create guest `Memory` instance");

            let (context, session_state, argument, forwarded_sessions) =
                <(
                    $crate::CalleeContext,
                    Vec<u8>,
                    Vec<u8>,
                    Vec<$crate::SessionId>,
                ) as WitLoad>::load(&memory, GuestPointer::from(parameters_area))
                .expect("Failed to load `handle_session_call` parameters");

            $crate::contract::run_async_entrypoint::<$application, _, _, _>(
                move |mut application| async move {
                    let session_state: <$application as $crate::abi::ContractAbi>::SessionState =
                        bcs::from_bytes(&session_state)?;
                    let argument: <$application as $crate::abi::ContractAbi>::SessionCall =
                        bcs::from_bytes(&argument)?;

                    application
                        .handle_session_call(&context, session_state, argument, forwarded_sessions)
                        .await
                        .map(|result| (application, result))
                },
            );
        }

        /// Stub of a `main` entrypoint so that the binary doesn't fail to compile on targets other
        /// than WebAssembly.
        #[cfg(not(target_arch = "wasm32"))]
        fn main() {}
    };
}

/// Runs an asynchronous entrypoint in a blocking manner, by repeatedly polling the entrypoint
/// future.
pub fn run_async_entrypoint<Application, Entrypoint, Output, Error>(
    entrypoint: impl FnOnce(Application) -> Entrypoint + Send,
) -> Result<Output, String>
where
    Application: Contract,
    Entrypoint: Future<Output = Result<(Application, Output), Error>> + Send,
    Output: Send + 'static,
    Error: ToString + 'static,
{
    ContractLogger::install();

    <Application as Contract>::Storage::execute_with_state(entrypoint)
        .blocking_wait()
        .map_err(|error| error.to_string())
}
