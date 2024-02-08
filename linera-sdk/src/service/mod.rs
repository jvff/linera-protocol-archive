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
        static mut SERVICE_RETURN_AREA: $crate::buffer_type_for!(Result<Vec<u8>, String>) =
            std::mem::MaybeUninit::uninit();

        // Export the service interface.
        #[cfg(target_arch = "wasm32")]
        #[export_name = "linera:app/service-entrypoints#handle-query"]
        extern "C" fn handle_query(
            chain_id_part1: i64,
            chain_id_part2: i64,
            chain_id_part3: i64,
            chain_id_part4: i64,
            argument_address: i32,
            argument_length: i32,
        ) -> i32 {
            use $crate::witty::{guest::Guest, GuestPointer, InstanceWithMemory, WitLoad, WitStore};

            let mut guest = Guest::default();
            let mut memory = guest
                .memory()
                .expect("Failed to create guest `Memory` instance");

            let (context, argument) = <($crate::QueryContext, Vec<u8>) as WitLoad>::lift_from(
                $crate::witty::hlist![
                    chain_id_part1,
                    chain_id_part2,
                    chain_id_part3,
                    chain_id_part4,
                    argument_address,
                    argument_length,
                ],
                &memory,
            )
            .expect("Failed to load `handle_query` parameters");

            let result = $crate::service::run_async_entrypoint(
                <
                    <$application as $crate::Service>::Storage as $crate::ServiceStateStorage
                >::handle_query(context, argument),
            );


            let result_address = GuestPointer::from(unsafe { SERVICE_RETURN_AREA.as_mut_ptr() })
                .after_padding_for::<Result<Vec<u8>, String>>();

            result
                .store(&mut memory, result_address)
                .expect("Failed to store `handle_query` result");

            result_address.as_i32()
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
