// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the Fungible Token application.

#![cfg(not(target_arch = "wasm32"))]

use crowd_funding::{CrowdFundingAbi, InitializationArgument, Operation};
use fungible::{AccountOwner, FungibleTokenAbi};
use linera_sdk::{
    base::{Amount, Timestamp},
    test::TestValidator,
};

/// Test transfering tokens across microchains.
///
/// Creates the application on a `sender_chain`, initializing it with a single account with some
/// tokens for that chain's owner. Transfers some of those tokens to a new `receiver_chain`, and
/// checks that the balances on each microchain are correct.
#[tokio::test]
async fn simple_successful_campaign() {
    let initial_amount = Amount::from(100);
    let target_amount = Amount::from(220);
    let pledge_amount = Amount::from(75);

    let (validator, bytecode_id) = TestValidator::with_current_bytecode().await;

    let fungible_publisher_chain = validator.new_chain().await;
    let mut token_chain = validator.new_chain().await;
    let mut campaign_chain = validator.new_chain().await;
    let backer1_chain = validator.new_chain().await;
    let backer2_chain = validator.new_chain().await;
    let backer3_chain = validator.new_chain().await;

    let campaign_account = AccountOwner::from(campaign_chain.public_key());
    let backer1_account = AccountOwner::from(backer1_chain.public_key());
    let backer2_account = AccountOwner::from(backer2_chain.public_key());
    let backer3_account = AccountOwner::from(backer3_chain.public_key());

    let initial_tokens = fungible::InitialStateBuilder::default()
        .with_account(backer1_account, initial_amount)
        .with_account(backer2_account, initial_amount)
        .with_account(backer3_account, initial_amount);
    let fungible_bytecode_id = fungible_publisher_chain
        .publish_bytecodes_in("../fungible")
        .await;
    let token_id = token_chain
        .create_application::<FungibleTokenAbi>(
            fungible_bytecode_id,
            (),
            initial_tokens.build(),
            vec![],
        )
        .await;

    let campaign_state = InitializationArgument {
        owner: campaign_account,
        deadline: Timestamp::from(u64::MAX),
        target: target_amount,
    };
    let campaign_id = campaign_chain
        .create_application::<CrowdFundingAbi>(
            bytecode_id,
            token_id,
            campaign_state,
            vec![token_id.forget_abi()],
        )
        .await;

    let backers = [
        (&backer1_chain, backer1_account),
        (&backer2_chain, backer2_account),
        (&backer3_chain, backer3_account),
    ];

    for (backer_chain, backer_account) in backers {
        backer_chain.register_application(campaign_id).await;

        backer_chain
            .add_block(|block| {
                block.with_operation(
                    token_id,
                    fungible::Operation::Claim {
                        source_account: fungible::Account {
                            chain_id: token_chain.id(),
                            owner: backer_account,
                        },
                        amount: pledge_amount,
                        target_account: fungible::Account {
                            chain_id: backer_chain.id(),
                            owner: backer_account,
                        },
                    },
                );
            })
            .await;

        token_chain.handle_received_effects().await;
        backer_chain.handle_received_effects().await;

        backer_chain
            .add_block(|block| {
                block.with_operation(
                    campaign_id,
                    Operation::PledgeWithTransfer {
                        owner: backer_account,
                        amount: pledge_amount,
                    },
                );
            })
            .await;
    }

    campaign_chain.handle_received_effects().await;

    assert_eq!(
        FungibleTokenAbi::query_account(token_id, &campaign_chain, campaign_account).await,
        None
    );

    campaign_chain
        .add_block(|block| {
            block.with_operation(campaign_id, Operation::Collect);
        })
        .await;

    assert_eq!(
        FungibleTokenAbi::query_account(token_id, &campaign_chain, campaign_account).await,
        Some(
            pledge_amount
                .saturating_add(pledge_amount)
                .saturating_add(pledge_amount)
        ),
    );

    for (backer_chain, backer_account) in backers {
        assert_eq!(
            FungibleTokenAbi::query_account(token_id, &token_chain, backer_account).await,
            Some(initial_amount.saturating_sub(pledge_amount)),
        );
        assert_eq!(
            FungibleTokenAbi::query_account(token_id, backer_chain, backer_account).await,
            Some(Amount::from(0)),
        );
        assert_eq!(
            FungibleTokenAbi::query_account(token_id, &campaign_chain, backer_account).await,
            Some(Amount::from(0)),
        );
    }
}
