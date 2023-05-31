// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the Counter application.

#![cfg(not(target_arch = "wasm32"))]

use linera_sdk::test::TestValidator;

/// Test setting a counter and testing its coherency across microchains.
///
/// Creates the application on a `chain`, initializing it with a 42 then add 15 and obtain 57.
/// which is then checked.
#[tokio::test]
async fn single_chain_test() {
    let (validator, bytecode_id) = TestValidator::with_current_bytecode().await;
    let mut chain = validator.new_chain().await;

    let initial_state = 42u64;
    let application_id = chain
        .create_application::<counter::CounterAbi>(bytecode_id, (), initial_state, vec![])
        .await;

    let increment = 15u64;
    chain
        .add_block(|block| {
            block.with_operation(application_id, increment);
        })
        .await;

    let final_value = initial_state + increment;
    let response = chain
        .query(application_id, "{ value }".into())
        .await
        .data
        .into_json()
        .expect("Unexpected non-JSON query response");
    let state_value = response["value"].as_u64().expect("Failed to get the u64");
    assert_eq!(state_value, final_value);
}
