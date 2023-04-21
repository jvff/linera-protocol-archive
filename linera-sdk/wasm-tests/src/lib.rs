// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Tests that should be executed inside a WebAssembly environment.
//!
//! Includes tests for the mocked system APIs.

#![cfg(test)]
#![cfg(target_arch = "wasm32")]

use futures::FutureExt;
use linera_sdk::{
    base::{
        ApplicationId, Balance, BlockHeight, BytecodeId, ChainId, EffectId, SessionId, Timestamp,
    },
    contract, service, test, ContractLogger, ServiceLogger,
};
use linera_views::{
    common::Context,
    map_view::MapView,
    register_view::RegisterView,
    views::{HashableView, RootView, View},
};
use webassembly_test::webassembly_test;

/// Test if the chain ID getter API is mocked successfully.
#[webassembly_test]
fn mock_chain_id() {
    let chain_id = ChainId([0, 1, 2, 3].into());

    test::mock_chain_id(chain_id);

    assert_eq!(contract::system_api::current_chain_id(), chain_id);
    assert_eq!(service::system_api::current_chain_id(), chain_id);
}

/// Test if the application ID getter API is mocked successfully.
#[webassembly_test]
fn mock_application_id() {
    let application_id = ApplicationId {
        bytecode_id: BytecodeId(EffectId {
            chain_id: ChainId([0, 1, 2, 3].into()),
            height: BlockHeight::from(4),
            index: 5,
        }),
        creation: EffectId {
            chain_id: ChainId([6, 7, 8, 9].into()),
            height: BlockHeight::from(10),
            index: 11,
        },
    };

    test::mock_application_id(application_id);

    assert_eq!(
        contract::system_api::current_application_id(),
        application_id
    );
    assert_eq!(
        service::system_api::current_application_id(),
        application_id
    );
}

/// Test if the application parameters getter API is mocked successfully.
#[webassembly_test]
fn mock_application_parameters() {
    let parameters = vec![0, 1, 2, 3, 4, 5, 6];

    test::mock_application_parameters(parameters.clone());

    assert_eq!(
        contract::system_api::current_application_parameters(),
        parameters
    );
    assert_eq!(
        service::system_api::current_application_parameters(),
        parameters
    );
}

/// Test if the system balance getter API is mocked successfully.
#[webassembly_test]
fn mock_system_balance() {
    let balance = Balance::from(0x00010203_04050607_08090a0b_0c0d0e0f);

    test::mock_system_balance(balance);

    assert_eq!(contract::system_api::current_system_balance(), balance);
    assert_eq!(service::system_api::current_system_balance(), balance);
}

/// Test if the system timestamp getter API is mocked successfully.
#[webassembly_test]
fn mock_system_timestamp() {
    let timestamp = Timestamp::from(0x00010203_04050607);

    test::mock_system_timestamp(timestamp);

    assert_eq!(contract::system_api::current_system_time(), timestamp);
    assert_eq!(service::system_api::current_system_time(), timestamp);
}

/// Test if messages logged by a contract can be inspected.
#[webassembly_test]
fn mock_contract_log() {
    ContractLogger::install();

    log::trace!("Trace");
    log::debug!("Debug");
    log::info!("Info");
    log::warn!("Warn");
    log::error!("Error");

    let expected = vec![
        (log::Level::Trace, "Trace".to_owned()),
        (log::Level::Debug, "Debug".to_owned()),
        (log::Level::Info, "Info".to_owned()),
        (log::Level::Warn, "Warn".to_owned()),
        (log::Level::Error, "Error".to_owned()),
    ];

    assert_eq!(test::log_messages(), expected);
}

/// Test if messages logged by a service can be inspected.
#[webassembly_test]
fn mock_service_log() {
    ServiceLogger::install();

    log::trace!("Trace");
    log::debug!("Debug");
    log::info!("Info");
    log::warn!("Warn");
    log::error!("Error");

    let expected = vec![
        (log::Level::Trace, "Trace".to_owned()),
        (log::Level::Debug, "Debug".to_owned()),
        (log::Level::Info, "Info".to_owned()),
        (log::Level::Warn, "Warn".to_owned()),
        (log::Level::Error, "Error".to_owned()),
    ];

    assert_eq!(test::log_messages(), expected);
}

/// Test loading a mocked application state without locking it.
#[webassembly_test]
fn mock_load_blob_state() {
    let state = vec![0, 1, 2, 3, 4, 5, 6];

    test::mock_application_state(
        bcs::to_bytes(&state).expect("Failed to serialize vector using BCS"),
    );

    assert_eq!(contract::system_api::load::<Vec<u8>>(), state);
    assert_eq!(service::system_api::load().now_or_never(), Some(state));
}

/// Test loading and locking a mocked application state.
#[webassembly_test]
fn mock_load_and_lock_blob_state() {
    let state = vec![0, 1, 2, 3, 4, 5, 6];

    test::mock_application_state(
        bcs::to_bytes(&state).expect("Failed to serialize vector using BCS"),
    );

    assert_eq!(
        contract::system_api::load_and_lock::<Vec<u8>>(),
        Some(state)
    );
}

/// A dummy view to test the key value store.
#[derive(RootView)]
struct DummyView<C> {
    one: RegisterView<C, u8>,
    two: RegisterView<C, u16>,
    three: RegisterView<C, u32>,
    map: MapView<C, u8, i8>,
}

/// Test if views are loaded from a memory key-value store.
#[webassembly_test]
fn mock_load_view() {
    let store = test::mock_key_value_store();
    let mut initial_view = DummyView::load(store)
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to initialize `DummyView` with the mock key value store");

    initial_view.one.set(1);
    initial_view.two.set(2);
    initial_view.three.set(3);
    initial_view
        .save()
        .now_or_never()
        .expect("Persisting a view to memory should be instantaneous")
        .expect("Failed to persist view state");

    let contract_view = contract::system_api::load_and_lock_view::<DummyView<_>>()
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to lock view");

    assert_eq!(initial_view.one.get(), contract_view.one.get());
    assert_eq!(initial_view.two.get(), contract_view.two.get());
    assert_eq!(initial_view.three.get(), contract_view.three.get());

    let service_view = service::system_api::lock_and_load_view::<DummyView<_>>()
        .now_or_never()
        .expect("Memory key value store should always resolve immediately");

    assert_eq!(initial_view.one.get(), service_view.one.get());
    assert_eq!(initial_view.two.get(), service_view.two.get());
    assert_eq!(initial_view.three.get(), service_view.three.get());
}

/// Test if key prefix search works in the mocked key-value store.
#[webassembly_test]
fn mock_find_keys() {
    let store = test::mock_key_value_store();
    let mut initial_view = DummyView::load(store)
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to initialize `DummyView` with the mock key value store");

    let keys = [32, 36, 40, 44];

    for &key in &keys {
        initial_view
            .map
            .insert(&key, -(key as i8))
            .expect("Failed to insert value into dumy map view");
    }

    initial_view
        .save()
        .now_or_never()
        .expect("Persisting a view to memory should be instantaneous")
        .expect("Failed to persist view state");

    let contract_view = contract::system_api::load_and_lock_view::<DummyView<_>>()
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to lock view");

    let contract_keys = contract_view
        .map
        .indices()
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to load keys of dummy map view");

    assert_eq!(contract_keys, keys);

    let service_view = service::system_api::lock_and_load_view::<DummyView<_>>()
        .now_or_never()
        .expect("Memory key value store should always resolve immediately");

    let service_keys = service_view
        .map
        .indices()
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to load keys of dummy map view");

    assert_eq!(service_keys, keys);
}

/// Test if key prefix search works in the mocked key-value store.
#[webassembly_test]
fn mock_find_key_value_pairs() {
    let store = test::mock_key_value_store();
    let mut initial_view = DummyView::load(store)
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to initialize `DummyView` with the mock key value store");

    let keys = [32, 36, 40, 44];
    let mut expected_pairs = Vec::new();

    for &key in &keys {
        let value = -(key as i8);

        initial_view
            .map
            .insert(&key, value)
            .expect("Failed to insert value into dumy map view");

        expected_pairs.push((key, value));
    }

    initial_view
        .save()
        .now_or_never()
        .expect("Persisting a view to memory should be instantaneous")
        .expect("Failed to persist view state");

    let contract_view = contract::system_api::load_and_lock_view::<DummyView<_>>()
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to lock view");

    let mut contract_pairs = Vec::new();

    contract_view
        .map
        .for_each_index_value(|key, value| {
            contract_pairs.push((key, value));
            Ok(())
        })
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to load key value pairs of dummy map view");

    assert_eq!(contract_pairs, expected_pairs);

    let service_view = service::system_api::lock_and_load_view::<DummyView<_>>()
        .now_or_never()
        .expect("Memory key value store should always resolve immediately");

    let mut service_pairs = Vec::new();

    service_view
        .map
        .for_each_index_value(|key, value| {
            service_pairs.push((key, value));
            Ok(())
        })
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to load key value pairs of dummy map view");

    assert_eq!(service_pairs, expected_pairs);
}

/// Test the write operations of the key-value store.
#[webassembly_test]
fn mock_write_batch() {
    let store = test::mock_key_value_store();
    let mut initial_view = DummyView::load(store.clone())
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to initialize `DummyView` with the mock key value store");

    let keys = [17, 23, 31, 37];
    let mut expected_pairs = Vec::new();

    for &key in &keys {
        let value = -(key as i8);

        initial_view
            .map
            .insert(&key, value)
            .expect("Failed to insert value into dumy map view");

        expected_pairs.push((key, value));
    }

    initial_view.one.set(1);
    initial_view.two.set(2);
    initial_view
        .two
        .hash()
        .now_or_never()
        .expect("Access to mock key-value store should be immediate")
        .expect("Failed to calculate the hash of a `RegisterView`");
    initial_view.three.set(3);
    initial_view
        .save()
        .now_or_never()
        .expect("Persisting a view to memory should be instantaneous")
        .expect("Failed to persist view state");

    let mut altered_view = contract::system_api::load_and_lock_view::<DummyView<_>>()
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to lock view");

    altered_view.one.set(100);
    altered_view.two.clear();
    altered_view.map.clear();

    altered_view
        .save()
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to store key value pairs of dummy map view");

    let loaded_view = DummyView::load(store)
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to initialize `DummyView` with the mock key value store");

    let loaded_keys = loaded_view
        .map
        .indices()
        .now_or_never()
        .expect("Memory key value store should always resolve immediately")
        .expect("Failed to load keys of dummy map view");

    assert_eq!(loaded_view.one.get(), altered_view.one.get());
    assert_eq!(loaded_view.two.get(), altered_view.two.get());
    assert_eq!(loaded_view.three.get(), initial_view.three.get());
    assert!(loaded_keys.is_empty());
}

static mut INTERCEPTED_AUTHENTICATED: Option<bool> = None;
static mut INTERCEPTED_APPLICATION_ID: Option<ApplicationId> = None;
static mut INTERCEPTED_ARGUMENT: Option<Vec<u8>> = None;
static mut INTERCEPTED_FORWARDED_SESSIONS: Option<Vec<SessionId>> = None;

/// Test mocking cross-application calls.
#[webassembly_test]
fn mock_cross_application_call() {
    let response = vec![0xff, 0xfe, 0xfd];
    let new_sessions = vec![
        SessionId {
            application_id: ApplicationId {
                bytecode_id: BytecodeId(EffectId {
                    chain_id: ChainId([0xfc, 0xfb, 0xfa, 0xf9].into()),
                    height: BlockHeight::from(0xf8),
                    index: 0xf7,
                }),
                creation: EffectId {
                    chain_id: ChainId([0xf6, 0xf5, 0xf4, 0xf3].into()),
                    height: BlockHeight::from(0xf2),
                    index: 0xf1,
                },
            },
            index: 0xf0,
            kind: 0xef,
        },
        SessionId {
            application_id: ApplicationId {
                bytecode_id: BytecodeId(EffectId {
                    chain_id: ChainId([0xee, 0xed, 0xec, 0xeb].into()),
                    height: BlockHeight::from(0xea),
                    index: 0xe9,
                }),
                creation: EffectId {
                    chain_id: ChainId([0xe8, 0xe7, 0xe6, 0xe5].into()),
                    height: BlockHeight::from(0xe4),
                    index: 0xe3,
                },
            },
            index: 0xe2,
            kind: 0xe1,
        },
        SessionId {
            application_id: ApplicationId {
                bytecode_id: BytecodeId(EffectId {
                    chain_id: ChainId([0xe0, 0xdf, 0xde, 0xdd].into()),
                    height: BlockHeight::from(0xdc),
                    index: 0xdb,
                }),
                creation: EffectId {
                    chain_id: ChainId([0xda, 0xd9, 0xd8, 0xd7].into()),
                    height: BlockHeight::from(0xd6),
                    index: 0xd5,
                },
            },
            index: 0xd4,
            kind: 0xd3,
        },
    ];

    let expected_response = response.clone();
    let expected_new_sessions = new_sessions.clone();

    test::mock_try_call_application(
        move |authenticated, application_id, argument, forwarded_sessions| {
            unsafe {
                INTERCEPTED_AUTHENTICATED = Some(authenticated);
                INTERCEPTED_APPLICATION_ID = Some(application_id);
                INTERCEPTED_ARGUMENT = Some(argument);
                INTERCEPTED_FORWARDED_SESSIONS = Some(forwarded_sessions);
            }

            (response.clone(), new_sessions.clone())
        },
    );

    let authenticated = true;
    let application_id = ApplicationId {
        bytecode_id: BytecodeId(EffectId {
            chain_id: ChainId([0, 1, 2, 3].into()),
            height: BlockHeight::from(4),
            index: 5,
        }),
        creation: EffectId {
            chain_id: ChainId([6, 7, 8, 9].into()),
            height: BlockHeight::from(10),
            index: 11,
        },
    };
    let argument = vec![17, 23, 31, 37];
    let forwarded_sessions = vec![
        SessionId {
            application_id: ApplicationId {
                bytecode_id: BytecodeId(EffectId {
                    chain_id: ChainId([100, 101, 102, 103].into()),
                    height: BlockHeight::from(104),
                    index: 105,
                }),
                creation: EffectId {
                    chain_id: ChainId([106, 107, 108, 109].into()),
                    height: BlockHeight::from(110),
                    index: 111,
                },
            },
            index: 112,
            kind: 113,
        },
        SessionId {
            application_id: ApplicationId {
                bytecode_id: BytecodeId(EffectId {
                    chain_id: ChainId([114, 115, 116, 117].into()),
                    height: BlockHeight::from(118),
                    index: 119,
                }),
                creation: EffectId {
                    chain_id: ChainId([120, 121, 122, 123].into()),
                    height: BlockHeight::from(124),
                    index: 125,
                },
            },
            index: 126,
            kind: 127,
        },
    ];

    let (response, new_sessions) = contract::system_api::call_application_without_persisting_state(
        authenticated,
        application_id,
        &argument,
        forwarded_sessions.clone(),
    )
    .now_or_never()
    .expect("Mock cross-application call should return immediately");

    assert_eq!(
        unsafe { INTERCEPTED_AUTHENTICATED.take() },
        Some(authenticated)
    );
    assert_eq!(
        unsafe { INTERCEPTED_APPLICATION_ID.take() },
        Some(application_id)
    );
    assert_eq!(unsafe { INTERCEPTED_ARGUMENT.take() }, Some(argument));
    assert_eq!(
        unsafe { INTERCEPTED_FORWARDED_SESSIONS.take() },
        Some(forwarded_sessions)
    );

    assert_eq!(response, expected_response);
    assert_eq!(new_sessions, expected_new_sessions);
}
