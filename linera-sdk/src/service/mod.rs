// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Types and macros useful for writing an application service.

mod conversions_from_wit;
mod conversions_to_wit;
mod storage;
#[cfg(target_arch = "wasm32")]
pub mod system_api;
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_arch = "wasm32"), path = "system_api_stubs.rs")]
pub mod system_api;
pub(crate) mod wit;

pub use self::storage::ServiceStateStorage;
use crate::{util::BlockingWait, QueryContext, ServiceLogger};
use std::future::Future;

/// Declares an implementation of the [`Service`][`crate::Service`] trait, exporting it from the
/// Wasm module.
///
/// Generates the necessary boilerplate for implementing the service WIT interface, exporting the
/// necessary resource types and functions so that the host can call the service application.
#[macro_export]
macro_rules! service {
    ($application:ty) => {
        #[doc(hidden)]
        #[no_mangle]
        fn __service_handle_query(
            context: $crate::QueryContext,
            argument: Vec<u8>,
        ) -> Result<Vec<u8>, String> {
            $crate::service::run_async_entrypoint(
                <
                    <$application as $crate::Service>::Storage as $crate::ServiceStateStorage
                >::handle_query(context, argument),
            )
        }

        /// Stub of a `main` entrypoint so that the binary doesn't fail to compile on targets other
        /// than WebAssembly.
        #[cfg(not(target_arch = "wasm32"))]
        fn main() {}

        #[doc(hidden)]
        #[no_mangle]
        fn __contract_initialize(
            _: $crate::OperationContext,
            _: Vec<u8>,
        ) -> Result<$crate::ExecutionOutcome<Vec<u8>>, String> {
            unreachable!("Contract entrypoint should not be called in service");
        }

        #[doc(hidden)]
        #[no_mangle]
        fn __contract_execute_operation(
            _: $crate::OperationContext,
            _: Vec<u8>,
        ) -> Result<$crate::ExecutionOutcome<Vec<u8>>, String> {
            unreachable!("Contract entrypoint should not be called in service");
        }

        #[doc(hidden)]
        #[no_mangle]
        fn __contract_execute_message(
            context: $crate::MessageContext,
            message: Vec<u8>,
        ) -> Result<$crate::ExecutionOutcome<Vec<u8>>, String> {
            unreachable!("Contract entrypoint should not be called in service");
        }

        #[doc(hidden)]
        #[no_mangle]
        fn __contract_handle_application_call(
            _: $crate::CalleeContext,
            _: Vec<u8>,
            _: Vec<$crate::SessionId>,
        ) -> Result<$crate::ApplicationCallOutcome<Vec<u8>, Vec<u8>, Vec<u8>>, String> {
            unreachable!("Contract entrypoint should not be called in service");
        }

        #[doc(hidden)]
        #[no_mangle]
        fn __contract_handle_session_call(
            _: $crate::CalleeContext,
            _: Vec<u8>,
            _: Vec<u8>,
            _: Vec<$crate::SessionId>,
        ) -> Result<($crate::RawSessionCallOutcome, Vec<u8>), String> {
            unreachable!("Contract entrypoint should not be called in service");
        }
    };
}

/// Runs an asynchronous entrypoint in a blocking manner, by repeatedly polling the entrypoint
/// future.
pub fn run_async_entrypoint<Entrypoint, Output, Error>(
    entrypoint: Entrypoint,
) -> Result<Output, String>
where
    Entrypoint: Future<Output = Result<Output, Error>> + Send,
    Output: Send + 'static,
    Error: ToString + 'static,
{
    ServiceLogger::install();

    entrypoint
        .blocking_wait()
        .map_err(|error| error.to_string())
}

// Import entrypoint proxy functions that applications implement with the `service!` macro.
extern "Rust" {
    fn __service_handle_query(context: QueryContext, argument: Vec<u8>) -> Result<Vec<u8>, String>;
}
