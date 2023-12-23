// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Types and macros useful for writing an application service.

mod storage;
#[cfg(target_arch = "wasm32")]
pub mod system_api;
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_arch = "wasm32"), path = "system_api_stubs.rs")]
pub mod system_api;

pub use self::storage::ServiceStateStorage;
use crate::{util::BlockingWait, ServiceLogger};
use std::future::Future;

// Import the system interface.
// wit_bindgen_guest_rust::import!("service_system_api.wit");

/// Declares an implementation of the [`Service`][`crate::Service`] trait, exporting it from the
/// Wasm module.
///
/// Generates the necessary boilerplate for implementing the service WIT interface, exporting the
/// necessary resource types and functions so that the host can call the service application.
#[macro_export]
macro_rules! service {
    ($application:ty) => {
        // Export the service interface.
        #[cfg(target_arch = "wasm32")]
        #[export_name = "handle-query"]
        extern "C" fn handle_query(
            parameters_area: i32,
            // context: $crate::service::wit_types::QueryContext,
            // argument: Vec<u8>,
        ) {
        // ) -> Result<Vec<u8>, String> {
            use $crate::witty::{guest::Guest, GuestPointer, InstanceWithMemory, WitLoad};

            let mut guest = Guest::default();
            let mut memory = guest
                .memory()
                .expect("Failed to create guest `Memory` instance");

            let (context, argument) = <($crate::QueryContext, Vec<u8>) as WitLoad>::load(
                &memory,
                GuestPointer::from(parameters_area),
            )
            .expect("Failed to load `handle_query` parameters");

            $crate::service::run_async_entrypoint(
                <
                    <$application as $crate::Service>::Storage as $crate::ServiceStateStorage
                >::handle_query(context, argument),
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
