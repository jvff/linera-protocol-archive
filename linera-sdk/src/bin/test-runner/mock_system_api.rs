// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use wasmtime::{Caller, Extern, Func, Linker};
use wit_bindgen_host_wasmtime_rust::rt::get_memory;

/// Retrieves a function exported from the guest WebAssembly module.
fn get_function(caller: &mut Caller<'_, ()>, name: &str) -> Option<Func> {
    match caller.get_export(name)? {
        Extern::Func(function) => Some(function),
        _ => None,
    }
}

/// Copies data from the `source_offset` to the `destination_offset` inside the guest WebAssembly
/// module's memory.
fn copy_memory_slices(
    caller: &mut Caller<'_, ()>,
    source_offset: i32,
    destination_offset: i32,
    size: i32,
) {
    let memory = get_memory(caller, "memory").expect("Missing `memory` export in the module.");
    let memory_data = memory.data_mut(caller);

    let size = usize::try_from(size).expect("Invalid size of memory slice to copy");

    let source_start = usize::try_from(source_offset).expect("Invalid pointer to copy data from");
    let source_end = source_start + size;

    let destination_start =
        usize::try_from(destination_offset).expect("Invalid pointer to copy data to");

    memory_data.copy_within(source_start..source_end, destination_start);
}

/// Adds the mock system APIs to the linker, so that they are available to guest WebAsembly
/// modules.
///
/// The system APIs are proxied back to the guest module, to be handled by the functions exported
/// from `linera_sdk::test::unit`.
pub fn add_to_linker(linker: &mut Linker<()>) -> Result<()> {
    linker.func_wrap1_async(
        "writable_system",
        "chain-id: func() -> record { part1: u64, part2: u64, part3: u64, part4: u64 }",
        move |mut caller: Caller<'_, ()>, return_offset: i32| {
            Box::new(async move {
                let function = get_function(
                    &mut caller,
                    "mock-writable-chain-id: \
                        func() -> record { part1: u64, part2: u64, part3: u64, part4: u64 }",
                )
                .expect(
                    "Missing `mock-writable-chain-id` function in the module. \
                    Please ensure `linera_sdk::test::mock_chain_id` was called",
                );

                let (result_offset,) = function
                    .typed::<(), (i32,), _>(&mut caller)
                    .expect("Incorrect `mock-writable-chain-id` function signature")
                    .call_async(&mut caller, ())
                    .await
                    .expect("Failed to call `mock-writable-chain-id` function");

                copy_memory_slices(&mut caller, result_offset, return_offset, 32);
            })
        },
    )?;
    linker.func_wrap1_async(
        "writable_system",
        "application-id: func() -> record { \
            bytecode-id: record { \
                chain-id: record { part1: u64, part2: u64, part3: u64, part4: u64 }, \
                height: u64, \
                index: u64 \
            }, \
            creation: record { \
                chain-id: record { part1: u64, part2: u64, part3: u64, part4: u64 }, \
                height: u64, \
                index: u64 \
            } \
        }",
        move |mut caller: Caller<'_, ()>, return_offset: i32| {
            Box::new(async move {
                let function = get_function(
                    &mut caller,
                    "mock-writable-application-id: func() -> record { \
                        bytecode-id: record { \
                            chain-id: record { part1: u64, part2: u64, part3: u64, part4: u64 }, \
                            height: u64, \
                            index: u64 \
                        }, \
                        creation: record { \
                            chain-id: record { part1: u64, part2: u64, part3: u64, part4: u64 }, \
                            height: u64, \
                            index: u64 \
                        } \
                    }",
                )
                .expect(
                    "Missing `mock-writable-application-id` function in the module. \
                    Please ensure `linera_sdk::test::mock_application_id` was called",
                );

                let (result_offset,) = function
                    .typed::<(), (i32,), _>(&mut caller)
                    .expect("Incorrect `mock-writable-application-id` function signature")
                    .call_async(&mut caller, ())
                    .await
                    .expect("Failed to call `mock-writable-application-id` function");

                copy_memory_slices(&mut caller, result_offset, return_offset, 96);
            })
        },
    )?;
    linker.func_wrap1_async(
        "writable_system",
        "application-parameters: func() -> list<u8>",
        move |mut caller: Caller<'_, ()>, return_offset: i32| {
            Box::new(async move {
                let function = get_function(
                    &mut caller,
                    "mock-writable-application-parameters: func() -> list<u8>",
                )
                .expect(
                    "Missing `mock-writable-application-parameters` function in the module. \
                    Please ensure `linera_sdk::test::mock_application_parameters` was called",
                );

                let (result_offset,) = function
                    .typed::<(), (i32,), _>(&mut caller)
                    .expect("Incorrect `mock-writable-application-parameters` function signature")
                    .call_async(&mut caller, ())
                    .await
                    .expect("Failed to call `mock-writable-application-parameters` function");

                copy_memory_slices(&mut caller, result_offset, return_offset, 8);
            })
        },
    )?;
    linker.func_wrap1_async(
        "writable_system",
        "read-system-balance: func() -> record { lower-half: u64, upper-half: u64 }",
        move |mut caller: Caller<'_, ()>, return_offset: i32| {
            Box::new(async move {
                let function = get_function(
                    &mut caller,
                    "mock-writable-read-system-balance: \
                        func() -> record { lower-half: u64, upper-half: u64 }",
                )
                .expect(
                    "Missing `mock-writable-read-system-balance` function in the module. \
                    Please ensure `linera_sdk::test::mock_system_balance` was called",
                );

                let (result_offset,) = function
                    .typed::<(), (i32,), _>(&mut caller)
                    .expect("Incorrect `mock-writable-read-system-balance` function signature")
                    .call_async(&mut caller, ())
                    .await
                    .expect("Failed to call `mock-writable-read-system-balance` function");

                copy_memory_slices(&mut caller, result_offset, return_offset, 16);
            })
        },
    )?;
    linker.func_wrap0_async(
        "writable_system",
        "read-system-timestamp: func() -> u64",
        move |mut caller: Caller<'_, ()>| {
            Box::new(async move {
                let function = get_function(
                    &mut caller,
                    "mock-writable-read-system-timestamp: func() -> u64",
                )
                .expect(
                    "Missing `mock-writable-read-system-timestamp` function in the module. \
                    Please ensure `linera_sdk::test::mock_system_timestamp` was called",
                );

                let (timestamp,) = function
                    .typed::<(), (i64,), _>(&mut caller)
                    .expect("Incorrect `mock-writable-read-system-timestamp` function signature")
                    .call_async(&mut caller, ())
                    .await
                    .expect("Failed to call `mock-writable-read-system-timestamp` function");

                timestamp
            })
        },
    )?;
    linker.func_wrap3_async(
        "writable_system",
        "log: func(message: string, level: enum { trace, debug, info, warn, error }) -> unit",
        move |mut caller: Caller<'_, ()>, message_address: i32, message_length: i32, level: i32| {
            Box::new(async move {
                let function = get_function(
                    &mut caller,
                    "mock-writable-log: func(\
                        message: string, \
                        level: enum { trace, debug, info, warn, error }\
                    ) -> unit",
                )
                .expect(
                    "Missing `mock-writable-log` function in the module. \
                    Please ensure `linera_sdk` is compiled with the `test` feature enabled",
                );

                let alloc_function = get_function(&mut caller, "cabi_realloc").expect(
                    "Missing `cabi_realloc` function in the module. \
                    Please ensure `linera_sdk` is compiled in with the module",
                );

                let new_message_address = alloc_function
                    .typed::<(i32, i32, i32, i32), i32, _>(&mut caller)
                    .expect("Incorrect `cabi_realloc` function signature")
                    .call_async(&mut caller, (0, 0, 1, message_length))
                    .await
                    .expect("Failed to call `cabi_realloc` function");

                copy_memory_slices(
                    &mut caller,
                    message_address,
                    new_message_address,
                    message_length,
                );

                function
                    .typed::<(i32, i32, i32), (), _>(&mut caller)
                    .expect("Incorrect `mock-writable-log` function signature")
                    .call_async(&mut caller, (new_message_address, message_length, level))
                    .await
                    .expect("Failed to call `mock-writable-log` function");
            })
        },
    )?;

    linker.func_wrap1_async(
        "queryable_system",
        "chain-id: func() -> record { part1: u64, part2: u64, part3: u64, part4: u64 }",
        move |mut caller: Caller<'_, ()>, return_offset: i32| {
            Box::new(async move {
                let function = get_function(
                    &mut caller,
                    "mock-queryable-chain-id: \
                        func() -> record { part1: u64, part2: u64, part3: u64, part4: u64 }",
                )
                .expect(
                    "Missing `mock-queryable-chain-id` function in the module. \
                    Please ensure `linera_sdk::test::mock_chain_id` was called",
                );

                let (result_offset,) = function
                    .typed::<(), (i32,), _>(&mut caller)
                    .expect("Incorrect `mock-queryable-chain-id` function signature")
                    .call_async(&mut caller, ())
                    .await
                    .expect("Failed to call `mock-queryable-chain-id` function");

                copy_memory_slices(&mut caller, result_offset, return_offset, 32);
            })
        },
    )?;
    linker.func_wrap1_async(
        "queryable_system",
        "application-id: func() -> record { \
            bytecode-id: record { \
                chain-id: record { part1: u64, part2: u64, part3: u64, part4: u64 }, \
                height: u64, \
                index: u64 \
            }, \
            creation: record { \
                chain-id: record { part1: u64, part2: u64, part3: u64, part4: u64 }, \
                height: u64, \
                index: u64 \
            } \
        }",
        move |mut caller: Caller<'_, ()>, return_offset: i32| {
            Box::new(async move {
                let function = get_function(
                    &mut caller,
                    "mock-queryable-application-id: func() -> record { \
                        bytecode-id: record { \
                            chain-id: record { part1: u64, part2: u64, part3: u64, part4: u64 }, \
                            height: u64, \
                            index: u64 \
                        }, \
                        creation: record { \
                            chain-id: record { part1: u64, part2: u64, part3: u64, part4: u64 }, \
                            height: u64, \
                            index: u64 \
                        } \
                    }",
                )
                .expect(
                    "Missing `mock-queryable-application-id` function in the module. \
                    Please ensure `linera_sdk::test::mock_application_id` was called",
                );

                let (result_offset,) = function
                    .typed::<(), (i32,), _>(&mut caller)
                    .expect("Incorrect `mock-queryable-application-id` function signature")
                    .call_async(&mut caller, ())
                    .await
                    .expect("Failed to call `mock-queryable-application-id` function");

                copy_memory_slices(&mut caller, result_offset, return_offset, 96);
            })
        },
    )?;
    linker.func_wrap1_async(
        "queryable_system",
        "application-parameters: func() -> list<u8>",
        move |mut caller: Caller<'_, ()>, return_offset: i32| {
            Box::new(async move {
                let function = get_function(
                    &mut caller,
                    "mock-queryable-application-parameters: func() -> list<u8>",
                )
                .expect(
                    "Missing `mock-queryable-application-parameters` function in the module. \
                    Please ensure `linera_sdk::test::mock_application_parameters` was called",
                );

                let (result_offset,) = function
                    .typed::<(), (i32,), _>(&mut caller)
                    .expect("Incorrect `mock-queryable-application-parameters` function signature")
                    .call_async(&mut caller, ())
                    .await
                    .expect("Failed to call `mock-queryable-application-parameters` function");

                copy_memory_slices(&mut caller, result_offset, return_offset, 8);
            })
        },
    )?;
    linker.func_wrap1_async(
        "queryable_system",
        "read-system-balance: func() -> record { lower-half: u64, upper-half: u64 }",
        move |mut caller: Caller<'_, ()>, return_offset: i32| {
            Box::new(async move {
                let function = get_function(
                    &mut caller,
                    "mock-queryable-read-system-balance: \
                        func() -> record { lower-half: u64, upper-half: u64 }",
                )
                .expect(
                    "Missing `mock-queryable-read-system-balance` function in the module. \
                    Please ensure `linera_sdk::test::mock_system_balance` was called",
                );

                let (result_offset,) = function
                    .typed::<(), (i32,), _>(&mut caller)
                    .expect("Incorrect `mock-queryable-read-system-balance` function signature")
                    .call_async(&mut caller, ())
                    .await
                    .expect("Failed to call `mock-queryable-read-system-balance` function");

                copy_memory_slices(&mut caller, result_offset, return_offset, 16);
            })
        },
    )?;
    linker.func_wrap0_async(
        "queryable_system",
        "read-system-timestamp: func() -> u64",
        move |mut caller: Caller<'_, ()>| {
            Box::new(async move {
                let function = get_function(
                    &mut caller,
                    "mock-queryable-read-system-timestamp: func() -> u64",
                )
                .expect(
                    "Missing `mock-queryable-read-system-timestamp` function in the module. \
                    Please ensure `linera_sdk::test::mock_system_timestamp` was called",
                );

                let (timestamp,) = function
                    .typed::<(), (i64,), _>(&mut caller)
                    .expect("Incorrect `mock-queryable-read-system-timestamp` function signature")
                    .call_async(&mut caller, ())
                    .await
                    .expect("Failed to call `mock-queryable-read-system-timestamp` function");

                timestamp
            })
        },
    )?;
    linker.func_wrap3_async(
        "queryable_system",
        "log: func(message: string, level: enum { trace, debug, info, warn, error }) -> unit",
        move |mut caller: Caller<'_, ()>, message_address: i32, message_length: i32, level: i32| {
            Box::new(async move {
                let function = get_function(
                    &mut caller,
                    "mock-queryable-log: func(\
                        message: string, \
                        level: enum { trace, debug, info, warn, error }\
                    ) -> unit",
                )
                .expect(
                    "Missing `mock-queryable-log` function in the module. \
                    Please ensure `linera_sdk` is compiled with the `test` feature enabled",
                );

                let alloc_function = get_function(&mut caller, "cabi_realloc").expect(
                    "Missing `cabi_realloc` function in the module. \
                    Please ensure `linera_sdk` is compiled in with the module",
                );

                let new_message_address = alloc_function
                    .typed::<(i32, i32, i32, i32), i32, _>(&mut caller)
                    .expect("Incorrect `cabi_realloc` function signature")
                    .call_async(&mut caller, (0, 0, 1, message_length))
                    .await
                    .expect("Failed to call `cabi_realloc` function");

                copy_memory_slices(
                    &mut caller,
                    message_address,
                    new_message_address,
                    message_length,
                );

                function
                    .typed::<(i32, i32, i32), (), _>(&mut caller)
                    .expect("Incorrect `mock-queryable-log` function signature")
                    .call_async(&mut caller, (new_message_address, message_length, level))
                    .await
                    .expect("Failed to call `mock-queryable-log` function");
            })
        },
    )?;

    Ok(())
}
