// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Imports for the service system APIs.

use linera_base::{
    data_types::{Amount, Timestamp},
    identifiers::{ApplicationId, ChainId},
};
use linera_witty::{
    guest::Guest, hlist, hlist_pat, GuestPointer, InstanceWithMemory, Layout, WitLoad, WitStore,
    WitType,
};
use std::mem::MaybeUninit;

#[link(wasm_import_module = "linera:app/service-system-api")]
extern "C" {
    #[link_name = "get-chain-id"]
    pub fn wit_get_chain_id(return_area: i32);

    #[link_name = "get-application-id"]
    pub fn wit_get_application_id(return_area: i32);

    #[link_name = "get-application-parameters"]
    pub fn wit_get_application_parameters(return_area: i32);

    #[link_name = "read-system-balance"]
    pub fn wit_read_system_balance(return_area: i32);

    #[link_name = "read-system-timestamp"]
    pub fn wit_read_system_timestamp() -> i64;

    #[link_name = "load-new"]
    pub fn wit_load_new() -> i32;

    #[link_name = "load-wait"]
    pub fn wit_load_wait(promise_id: i32, return_area: i32);

    #[link_name = "lock-new"]
    pub fn wit_lock_new() -> i32;

    #[link_name = "lock-wait"]
    pub fn wit_lock_wait(promise_id: i32, return_area: i32);

    #[link_name = "unlock-new"]
    pub fn wit_unlock_new() -> i32;

    #[link_name = "unlock-wait"]
    pub fn wit_unlock_wait(promise_id: i32, return_area: i32);

    #[link_name = "try-query-application-new"]
    pub fn wit_try_query_application_new(
        bytecode_id_chain_id_part_1: i64,
        bytecode_id_chain_id_part_2: i64,
        bytecode_id_chain_id_part_3: i64,
        bytecode_id_chain_id_part_4: i64,
        bytecode_id_height: i64,
        bytecode_id_index: i32,
        application_id_chain_id_part_1: i64,
        application_id_chain_id_part_2: i64,
        application_id_chain_id_part_3: i64,
        application_id_chain_id_part_4: i64,
        application_id_height: i64,
        application_id_index: i32,
        query_address: i32,
        query_length: i32,
    ) -> i32;

    #[link_name = "try-query-application-wait"]
    pub fn wit_try_query_application_wait(promise_id: i32, return_area: i32);

    #[link_name = "log"]
    pub fn wit_log(message_address: i32, message_length: i32, log_level: i32);
}

macro_rules! stack_buffer_for {
    ($wit_type:ty) => {
        MaybeUninit::<
            [u8; <$wit_type as WitType>::SIZE as usize
                + <$wit_type as WitType>::Layout::ALIGNMENT as usize],
        >::uninit()
    };
}

macro_rules! stack_buffer_address {
    ($buffer:ident, $wit_type:ty) => {
        GuestPointer::from($buffer.as_mut_ptr()).after_padding_for::<$wit_type>()
    };
}

pub fn get_chain_id() -> ChainId {
    let mut return_area = stack_buffer_for!(ChainId);
    let return_area_address = stack_buffer_address!(return_area, ChainId);

    let mut guest = Guest::default();
    let memory = guest.memory().expect("Failed to obtain `Memory` instance");

    unsafe { wit_get_chain_id(return_area_address.as_i32()) };

    ChainId::load(&memory, return_area_address).expect("Failed to load `ChainId`")
}

pub fn get_application_id() -> ApplicationId {
    let mut return_area = stack_buffer_for!(ApplicationId);
    let return_area_address = stack_buffer_address!(return_area, ApplicationId);
    let mut guest = Guest::default();
    let memory = guest.memory().expect("Failed to obtain `Memory` instance");

    unsafe { wit_get_application_id(return_area_address.as_i32()) };

    ApplicationId::load(&memory, return_area_address).expect("Failed to load `ApplicationId`")
}

pub fn get_application_parameters() -> Vec<u8> {
    let mut return_area = stack_buffer_for!(Vec<u8>);
    let return_area_address = stack_buffer_address!(return_area, Vec<u8>);
    let mut guest = Guest::default();
    let memory = guest.memory().expect("Failed to obtain `Memory` instance");

    unsafe { wit_get_application_parameters(return_area_address.as_i32()) };

    Vec::load(&memory, return_area_address).expect("Failed to load application parameters")
}

pub fn read_system_balance() -> Amount {
    let mut return_area = stack_buffer_for!(Amount);
    let return_area_address = stack_buffer_address!(return_area, Amount);
    let mut guest = Guest::default();
    let memory = guest.memory().expect("Failed to obtain `Memory` instance");

    unsafe { wit_read_system_balance(return_area_address.as_i32()) };

    Amount::load(&memory, return_area_address).expect("Failed to load `Amount`")
}

pub fn read_system_timestamp() -> Timestamp {
    let mut guest = Guest::default();
    let memory = guest.memory().expect("Failed to obtain `Memory` instance");

    let raw_timestamp = unsafe { wit_read_system_timestamp() };

    Timestamp::lift_from(hlist![raw_timestamp], &memory).expect("Failed to load `Timestamp`")
}

pub fn load_new() -> u32 {
    unsafe { wit_load_new() as u32 }
}

pub fn load_wait(promise_id: u32) -> Result<Vec<u8>, String> {
    let mut return_area = stack_buffer_for!(Result<Vec<u8>, String>);
    let return_area_address = stack_buffer_address!(return_area, Result<Vec<u8>, String>);
    let mut guest = Guest::default();
    let memory = guest.memory().expect("Failed to obtain `Memory` instance");

    unsafe { wit_load_wait(promise_id as i32, return_area_address.as_i32()) };

    Result::load(&memory, return_area_address).expect("Failed to load application state")
}

pub fn lock_new() -> u32 {
    unsafe { wit_lock_new() as u32 }
}

pub fn lock_wait(promise_id: u32) -> Result<bool, String> {
    let mut return_area = stack_buffer_for!(Result<(), String>);
    let return_area_address = stack_buffer_address!(return_area, Result<(), String>);
    let mut guest = Guest::default();
    let memory = guest.memory().expect("Failed to obtain `Memory` instance");

    unsafe { wit_lock_wait(promise_id as i32, return_area_address.as_i32()) };

    Result::load(&memory, return_area_address).expect("Failed to load lock result")
}

pub fn unlock_new() -> u32 {
    unsafe { wit_unlock_new() as u32 }
}

pub fn unlock_wait(promise_id: u32) -> Result<bool, String> {
    let mut return_area = stack_buffer_for!(Result<(), String>);
    let return_area_address = stack_buffer_address!(return_area, Result<(), String>);
    let mut guest = Guest::default();
    let memory = guest.memory().expect("Failed to obtain `Memory` instance");

    unsafe { wit_unlock_wait(promise_id as i32, return_area_address.as_i32()) };

    Result::load(&memory, return_area_address).expect("Failed to load lock result")
}

pub fn try_query_application_new(application_id: ApplicationId, query: Vec<u8>) -> u32 {
    let mut guest = Guest::default();
    let mut memory = guest.memory().expect("Failed to obtain `Memory` instance");

    let hlist_pat![
        bytecode_id_chain_id_part_1,
        bytecode_id_chain_id_part_2,
        bytecode_id_chain_id_part_3,
        bytecode_id_chain_id_part_4,
        bytecode_id_height,
        bytecode_id_index,
        application_id_chain_id_part_1,
        application_id_chain_id_part_2,
        application_id_chain_id_part_3,
        application_id_chain_id_part_4,
        application_id_height,
        application_id_index,
        query_address,
        query_length,
    ] = (application_id, query)
        .lower(&mut memory)
        .expect("Failed to lower `try_query_application` parameters");

    unsafe {
        wit_try_query_application_new(
            bytecode_id_chain_id_part_1,
            bytecode_id_chain_id_part_2,
            bytecode_id_chain_id_part_3,
            bytecode_id_chain_id_part_4,
            bytecode_id_height,
            bytecode_id_index,
            application_id_chain_id_part_1,
            application_id_chain_id_part_2,
            application_id_chain_id_part_3,
            application_id_chain_id_part_4,
            application_id_height,
            application_id_index,
            query_address,
            query_length,
        ) as u32
    }
}

pub fn try_query_application_wait(promise_id: u32) -> Result<Vec<u8>, String> {
    let mut return_area = stack_buffer_for!(Result<Vec<u8>, String>);
    let return_area_address = stack_buffer_address!(return_area, Result<Vec<u8>, String>);

    let mut guest = Guest::default();
    let memory = guest.memory().expect("Failed to obtain `Memory` instance");

    unsafe { wit_try_query_application_wait(promise_id as i32, return_area_address.as_i32()) };

    Result::load(&memory, return_area_address).expect("Failed to load query result")
}

pub fn log(message: String, level: log::Level) {
    let mut guest = Guest::default();
    let mut memory = guest.memory().expect("Failed to obtain `Memory` instance");
    let hlist_pat![message_address, message_length, log_level] = (message, level)
        .lower(&mut memory)
        .expect("Failed to store log message and level");

    unsafe { wit_log(message_address, message_length, log_level) };
}
