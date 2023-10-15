// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg(any(feature = "rocksdb", feature = "aws", feature = "scylladb"))]

mod common;

use async_graphql::InputType;
use common::INTEGRATION_TEST_GUARD;
use linera_base::{
    data_types::{Amount, Timestamp},
    identifiers::ChainId,
};
use linera_service::cli_wrappers::{
    ApplicationWrapper, ClientWrapper, Database, LocalNetwork, Network,
};
use serde_json::{json, Value};
use std::{collections::BTreeMap, time::Duration};
use tracing::{info, warn};

fn get_fungible_account_owner(client: &ClientWrapper) -> fungible::AccountOwner {
    let owner = client.get_owner().unwrap();
    fungible::AccountOwner::User(owner)
}

struct FungibleApp(ApplicationWrapper<fungible::FungibleTokenAbi>);

impl FungibleApp {
    async fn get_amount(&self, account_owner: &fungible::AccountOwner) -> Amount {
        let query = format!("accounts(accountOwner: {})", account_owner.to_value());
        let response_body = self.0.query(&query).await;
        serde_json::from_value(response_body["accounts"].clone()).unwrap_or_default()
    }

    async fn assert_balances(
        &self,
        accounts: impl IntoIterator<Item = (fungible::AccountOwner, Amount)>,
    ) {
        for (account_owner, amount) in accounts {
            let value = self.get_amount(&account_owner).await;
            assert_eq!(value, amount);
        }
    }

    async fn transfer(
        &self,
        account_owner: &fungible::AccountOwner,
        amount_transfer: Amount,
        destination: fungible::Account,
    ) -> Value {
        let mutation = format!(
            "transfer(owner: {}, amount: \"{}\", targetAccount: {})",
            account_owner.to_value(),
            amount_transfer,
            destination.to_value(),
        );
        self.0.mutate(mutation).await
    }

    async fn claim(&self, source: fungible::Account, target: fungible::Account, amount: Amount) {
        // Claiming tokens from chain1 to chain2.
        let mutation = format!(
            "claim(sourceAccount: {}, amount: \"{}\", targetAccount: {})",
            source.to_value(),
            amount,
            target.to_value()
        );

        self.0.mutate(mutation).await;
    }
}

struct MatchingEngineApp(ApplicationWrapper<matching_engine::MatchingEngineAbi>);

impl MatchingEngineApp {
    async fn get_account_info(
        &self,
        account_owner: &fungible::AccountOwner,
    ) -> Vec<matching_engine::OrderId> {
        let query = format!(
            "accountInfo(accountOwner: {}) {{ orders }}",
            account_owner.to_value()
        );
        let response_body = self.0.query(query).await;
        serde_json::from_value(response_body["accountInfo"]["orders"].clone()).unwrap()
    }

    async fn order(&self, order: matching_engine::Order) -> Value {
        let mutation = format!("executeOrder(order: {})", order.to_value());
        self.0.mutate(mutation).await
    }
}

struct AmmApp(ApplicationWrapper<amm::AmmAbi>);

impl AmmApp {
    async fn add_liquidity(
        &self,
        owner: fungible::AccountOwner,
        max_token0_amount: Amount,
        max_token1_amount: Amount,
    ) {
        let operation = amm::Operation::AddLiquidity {
            owner,
            max_token0_amount,
            max_token1_amount,
        };

        let mutation = format!("operation(operation: {})", operation.to_value(),);
        self.0.mutate(mutation).await;
    }

    async fn remove_liquidity(
        &self,
        owner: fungible::AccountOwner,
        token_to_remove_idx: u32,
        token_to_remove_amount: Amount,
    ) {
        let operation = amm::Operation::RemoveLiquidity {
            owner,
            token_to_remove_idx,
            token_to_remove_amount,
        };

        let mutation = format!("operation(operation: {})", operation.to_value(),);
        self.0.mutate(mutation).await;
    }

    async fn swap(
        &self,
        owner: fungible::AccountOwner,
        input_token_idx: u32,
        input_amount: Amount,
    ) {
        let operation = amm::Operation::Swap {
            owner,
            input_token_idx,
            input_amount,
        };

        let mutation = format!("operation(operation: {})", operation.to_value(),);
        self.0.mutate(mutation).await;
    }
}

#[cfg(feature = "rocksdb")]
#[test_log::test(tokio::test)]
async fn test_rocks_db_wasm_end_to_end_counter() {
    run_wasm_end_to_end_counter(Database::RocksDb).await
}

#[cfg(feature = "aws")]
#[test_log::test(tokio::test)]
async fn test_dynamo_db_wasm_end_to_end_counter() {
    run_wasm_end_to_end_counter(Database::DynamoDb).await
}

#[cfg(feature = "scylladb")]
#[test_log::test(tokio::test)]
async fn test_scylla_db_wasm_end_to_end_counter() {
    run_wasm_end_to_end_counter(Database::ScyllaDb).await
}

async fn run_wasm_end_to_end_counter(database: Database) {
    use counter::CounterAbi;
    let _guard = INTEGRATION_TEST_GUARD.lock().await;

    let network = Network::Grpc;
    let mut local_net = LocalNetwork::new_for_testing(database, network).unwrap();
    let client = local_net.make_client(network);

    let original_counter_value = 35;
    let increment = 5;

    local_net.generate_initial_validator_config().await.unwrap();
    client.create_genesis_config().await.unwrap();
    let chain = client.get_wallet().default_chain().unwrap();
    local_net.run().await.unwrap();
    let (contract, service) = local_net.build_example("counter").await.unwrap();

    let application_id = client
        .publish_and_create::<CounterAbi>(
            contract,
            service,
            &(),
            &original_counter_value,
            &[],
            None,
        )
        .await
        .unwrap();
    let mut node_service = client.run_node_service(None).await.unwrap();

    let application = node_service.make_application(&chain, &application_id).await;

    let counter_value: u64 = application.query_json("value").await;
    assert_eq!(counter_value, original_counter_value);

    let mutation = format!("increment(value: {increment})");
    application.mutate(mutation).await;

    let counter_value: u64 = application.query_json("value").await;
    assert_eq!(counter_value, original_counter_value + increment);

    node_service.assert_is_running();
}

#[cfg(feature = "rocksdb")]
#[test_log::test(tokio::test)]
async fn test_rocks_db_wasm_end_to_end_counter_publish_create() {
    run_wasm_end_to_end_counter_publish_create(Database::RocksDb).await
}

#[cfg(feature = "aws")]
#[test_log::test(tokio::test)]
async fn test_dynamo_db_wasm_end_to_end_counter_publish_create() {
    run_wasm_end_to_end_counter_publish_create(Database::DynamoDb).await
}

#[cfg(feature = "scylladb")]
#[test_log::test(tokio::test)]
async fn test_scylla_db_wasm_end_to_end_counter_publish_create() {
    run_wasm_end_to_end_counter_publish_create(Database::ScyllaDb).await
}

async fn run_wasm_end_to_end_counter_publish_create(database: Database) {
    use counter::CounterAbi;

    let _guard = INTEGRATION_TEST_GUARD.lock().await;

    let network = Network::Grpc;
    let mut local_net = LocalNetwork::new_for_testing(database, network).unwrap();
    let client = local_net.make_client(network);

    let original_counter_value = 35;
    let increment = 5;

    local_net.generate_initial_validator_config().await.unwrap();
    client.create_genesis_config().await.unwrap();
    let chain = client.get_wallet().default_chain().unwrap();
    local_net.run().await.unwrap();
    let (contract, service) = local_net.build_example("counter").await.unwrap();

    let bytecode_id = client
        .publish_bytecode(contract, service, None)
        .await
        .unwrap();
    let application_id = client
        .create_application::<CounterAbi>(&bytecode_id, &original_counter_value, None)
        .await
        .unwrap();
    let mut node_service = client.run_node_service(None).await.unwrap();

    let application = node_service.make_application(&chain, &application_id).await;

    let counter_value: u64 = application.query_json("value").await;
    assert_eq!(counter_value, original_counter_value);

    let mutation = format!("increment(value: {increment})");
    application.mutate(mutation).await;

    let counter_value: u64 = application.query_json("value").await;
    assert_eq!(counter_value, original_counter_value + increment);

    node_service.assert_is_running();
}

#[cfg(feature = "rocksdb")]
#[test_log::test(tokio::test)]
async fn test_rocks_db_wasm_end_to_end_social_user_pub_sub() {
    run_wasm_end_to_end_social_user_pub_sub(Database::RocksDb).await
}

#[cfg(feature = "aws")]
#[test_log::test(tokio::test)]
async fn test_dynamo_db_wasm_end_to_end_social_user_pub_sub() {
    run_wasm_end_to_end_social_user_pub_sub(Database::DynamoDb).await
}

#[cfg(feature = "scylladb")]
#[test_log::test(tokio::test)]
async fn test_scylla_db_wasm_end_to_end_social_user_pub_sub() {
    run_wasm_end_to_end_social_user_pub_sub(Database::ScyllaDb).await
}

async fn run_wasm_end_to_end_social_user_pub_sub(database: Database) {
    use social::SocialAbi;
    let _guard = INTEGRATION_TEST_GUARD.lock().await;

    let network = Network::Grpc;
    let mut local_net = LocalNetwork::new_for_testing(database, network).unwrap();
    let client1 = local_net.make_client(network);
    let client2 = local_net.make_client(network);

    // Create initial server and client config.
    local_net.generate_initial_validator_config().await.unwrap();
    client1.create_genesis_config().await.unwrap();
    client2.wallet_init(&[]).await.unwrap();

    // Start local network.
    local_net.run().await.unwrap();
    let (contract, service) = local_net.build_example("social").await.unwrap();

    let chain1 = client1.get_wallet().default_chain().unwrap();
    let chain2 = client1.open_and_assign(&client2).await.unwrap();
    let bytecode_id = client1
        .publish_bytecode(contract, service, None)
        .await
        .unwrap();
    let application_id = client1
        .create_application::<SocialAbi>(&bytecode_id, &(), None)
        .await
        .unwrap();

    let mut node_service1 = client1.run_node_service(8080).await.unwrap();
    let mut node_service2 = client2.run_node_service(8081).await.unwrap();

    node_service1.process_inbox(&chain1).await;

    // Request the application so chain 2 has it, too.
    node_service2
        .request_application(&chain2, &application_id)
        .await;

    let app2 = node_service2
        .make_application(&chain2, &application_id)
        .await;
    let hash = app2
        .mutate(format!("requestSubscribe(field0: \"{chain1}\")"))
        .await;

    // The returned hash should now be the latest one.
    let query = format!("query {{ chain(chainId: \"{chain2}\") {{ tipState {{ blockHash }} }} }}");
    let response = node_service2.query_node(&query).await;
    assert_eq!(hash, response["chain"]["tipState"]["blockHash"]);

    let app1 = node_service1
        .make_application(&chain1, &application_id)
        .await;
    app1.mutate("post(field0: \"Linera Social is the new Mastodon!\")")
        .await;

    // Instead of retrying, we could call `node_service1.process_inbox(chain1).await` here.
    // However, we prefer to test the notification system for a change.
    let expected_response = json!({ "receivedPostsKeys": [
        { "author": chain1, "index": 0 }
    ]});
    'success: {
        for i in 0..10 {
            tokio::time::sleep(Duration::from_secs(i)).await;
            let response = app2
                .query("receivedPostsKeys(count: 5) { author, index }")
                .await;
            if response == expected_response {
                info!("Confirmed post");
                break 'success;
            }
            warn!("Waiting to confirm post: {}", response);
        }
        panic!("Failed to confirm post");
    }

    node_service1.assert_is_running();
    node_service2.assert_is_running();
}

#[cfg(feature = "rocksdb")]
#[test_log::test(tokio::test)]
async fn test_rocks_db_wasm_end_to_end_fungible() {
    run_wasm_end_to_end_fungible(Database::RocksDb).await
}

#[cfg(feature = "aws")]
#[test_log::test(tokio::test)]
async fn test_dynamo_db_wasm_end_to_end_fungible() {
    run_wasm_end_to_end_fungible(Database::DynamoDb).await
}

#[cfg(feature = "scylladb")]
#[test_log::test(tokio::test)]
async fn test_scylla_db_wasm_end_to_end_fungible() {
    run_wasm_end_to_end_fungible(Database::ScyllaDb).await
}

async fn run_wasm_end_to_end_fungible(database: Database) {
    use fungible::{FungibleTokenAbi, InitialState};

    let _guard = INTEGRATION_TEST_GUARD.lock().await;

    let network = Network::Grpc;
    let mut local_net = LocalNetwork::new_for_testing(database, network).unwrap();
    let client1 = local_net.make_client(network);
    let client2 = local_net.make_client(network);

    local_net.generate_initial_validator_config().await.unwrap();
    client1.create_genesis_config().await.unwrap();
    client2.wallet_init(&[]).await.unwrap();

    // Create initial server and client config.
    local_net.run().await.unwrap();
    let (contract, service) = local_net.build_example("fungible").await.unwrap();

    let chain1 = client1.get_wallet().default_chain().unwrap();
    let chain2 = client1.open_and_assign(&client2).await.unwrap();

    // The players
    let account_owner1 = get_fungible_account_owner(&client1);
    let account_owner2 = get_fungible_account_owner(&client2);
    // The initial accounts on chain1
    let accounts = BTreeMap::from([
        (account_owner1, Amount::from_tokens(5)),
        (account_owner2, Amount::from_tokens(2)),
    ]);
    let state = InitialState { accounts };
    // Setting up the application and verifying
    let application_id = client1
        .publish_and_create::<FungibleTokenAbi>(contract, service, &(), &state, &[], None)
        .await
        .unwrap();

    let mut node_service1 = client1.run_node_service(8080).await.unwrap();
    let mut node_service2 = client2.run_node_service(8081).await.unwrap();

    let app1 = FungibleApp(
        node_service1
            .make_application(&chain1, &application_id)
            .await,
    );
    app1.assert_balances([
        (account_owner1, Amount::from_tokens(5)),
        (account_owner2, Amount::from_tokens(2)),
    ])
    .await;

    // Transferring
    app1.transfer(
        &account_owner1,
        Amount::ONE,
        fungible::Account {
            chain_id: chain2,
            owner: account_owner2,
        },
    )
    .await;

    // Checking the final values on chain1 and chain2.
    app1.assert_balances([
        (account_owner1, Amount::from_tokens(4)),
        (account_owner2, Amount::from_tokens(2)),
    ])
    .await;

    // Fungible didn't exist on chain2 initially but now it does and we can talk to it.
    let app2 = FungibleApp(
        node_service2
            .make_application(&chain2, &application_id)
            .await,
    );

    app2.assert_balances(BTreeMap::from([
        (account_owner1, Amount::ZERO),
        (account_owner2, Amount::ONE),
    ]))
    .await;

    // Claiming more money from chain1 to chain2.
    app2.claim(
        fungible::Account {
            chain_id: chain1,
            owner: account_owner2,
        },
        fungible::Account {
            chain_id: chain2,
            owner: account_owner2,
        },
        Amount::from_tokens(2),
    )
    .await;

    // Make sure that the cross-chain communication happens fast enough.
    node_service1.process_inbox(&chain1).await;
    node_service2.process_inbox(&chain2).await;

    // Checking the final value
    app1.assert_balances([
        (account_owner1, Amount::from_tokens(4)),
        (account_owner2, Amount::ZERO),
    ])
    .await;
    app2.assert_balances([
        (account_owner1, Amount::ZERO),
        (account_owner2, Amount::from_tokens(3)),
    ])
    .await;

    node_service1.assert_is_running();
    node_service2.assert_is_running();
}

#[cfg(feature = "rocksdb")]
#[test_log::test(tokio::test)]
async fn test_rocks_db_wasm_end_to_end_same_wallet_fungible() {
    run_wasm_end_to_end_same_wallet_fungible(Database::RocksDb).await
}

#[cfg(feature = "aws")]
#[test_log::test(tokio::test)]
async fn test_dynamo_db_wasm_end_to_end_same_wallet_fungible() {
    run_wasm_end_to_end_same_wallet_fungible(Database::DynamoDb).await
}

#[cfg(feature = "scylladb")]
#[test_log::test(tokio::test)]
async fn test_scylla_db_wasm_end_to_end_same_wallet_fungible() {
    run_wasm_end_to_end_same_wallet_fungible(Database::ScyllaDb).await
}

async fn run_wasm_end_to_end_same_wallet_fungible(database: Database) {
    use fungible::{FungibleTokenAbi, InitialState};

    let _guard = INTEGRATION_TEST_GUARD.lock().await;

    let network = Network::Grpc;
    let mut local_net = LocalNetwork::new_for_testing(database, network).unwrap();
    let client1 = local_net.make_client(network);

    local_net.generate_initial_validator_config().await.unwrap();
    client1.create_genesis_config().await.unwrap();

    // Create initial server and client config.
    local_net.run().await.unwrap();
    let (contract, service) = local_net.build_example("fungible").await.unwrap();

    let chain1 = client1.get_wallet().default_chain().unwrap();
    let chain2 = ChainId::root(2);

    // The players
    let account_owner1 = get_fungible_account_owner(&client1);
    let account_owner2 = {
        let wallet = client1.get_wallet();
        let user_chain = wallet.get(chain2).unwrap();
        let public_key = user_chain.key_pair.as_ref().unwrap().public();
        fungible::AccountOwner::User(public_key.into())
    };
    // The initial accounts on chain1
    let accounts = BTreeMap::from([
        (account_owner1, Amount::from_tokens(5)),
        (account_owner2, Amount::from_tokens(2)),
    ]);
    let state = InitialState { accounts };
    // Setting up the application and verifying
    let application_id = client1
        .publish_and_create::<FungibleTokenAbi>(contract, service, &(), &state, &[], None)
        .await
        .unwrap();

    let mut node_service = client1.run_node_service(8080).await.unwrap();

    let app1 = FungibleApp(
        node_service
            .make_application(&chain1, &application_id)
            .await,
    );
    app1.assert_balances([
        (account_owner1, Amount::from_tokens(5)),
        (account_owner2, Amount::from_tokens(2)),
    ])
    .await;

    // Transferring
    app1.transfer(
        &account_owner1,
        Amount::ONE,
        fungible::Account {
            chain_id: chain2,
            owner: account_owner2,
        },
    )
    .await;

    // Checking the final values on chain1 and chain2.
    app1.assert_balances([
        (account_owner1, Amount::from_tokens(4)),
        (account_owner2, Amount::from_tokens(2)),
    ])
    .await;

    let app2 = FungibleApp(
        node_service
            .make_application(&chain2, &application_id)
            .await,
    );
    app2.assert_balances([(account_owner2, Amount::ONE)]).await;

    node_service.assert_is_running();
}

#[cfg(feature = "rocksdb")]
#[test_log::test(tokio::test)]
async fn test_rocks_db_wasm_end_to_end_crowd_funding() {
    run_wasm_end_to_end_crowd_funding(Database::RocksDb).await
}

#[cfg(feature = "aws")]
#[test_log::test(tokio::test)]
async fn test_dynamo_db_wasm_end_to_end_crowd_funding() {
    run_wasm_end_to_end_crowd_funding(Database::DynamoDb).await
}

#[cfg(feature = "scylladb")]
#[test_log::test(tokio::test)]
async fn test_scylla_db_wasm_end_to_end_crowd_funding() {
    run_wasm_end_to_end_crowd_funding(Database::ScyllaDb).await
}

async fn run_wasm_end_to_end_crowd_funding(database: Database) {
    use crowd_funding::{CrowdFundingAbi, InitializationArgument};
    use fungible::{FungibleTokenAbi, InitialState};

    let _guard = INTEGRATION_TEST_GUARD.lock().await;

    let network = Network::Grpc;
    let mut local_net = LocalNetwork::new_for_testing(database, network).unwrap();
    let client1 = local_net.make_client(network);
    let client2 = local_net.make_client(network);

    local_net.generate_initial_validator_config().await.unwrap();
    client1.create_genesis_config().await.unwrap();
    client2.wallet_init(&[]).await.unwrap();

    // Create initial server and client config.
    local_net.run().await.unwrap();
    let (contract_fungible, service_fungible) = local_net.build_example("fungible").await.unwrap();

    let chain1 = client1.get_wallet().default_chain().unwrap();
    let chain2 = client1.open_and_assign(&client2).await.unwrap();

    // The players
    let account_owner1 = get_fungible_account_owner(&client1); // operator
    let account_owner2 = get_fungible_account_owner(&client2); // contributor

    // The initial accounts on chain1
    let accounts = BTreeMap::from([(account_owner1, Amount::from_tokens(6))]);
    let state_fungible = InitialState { accounts };

    // Setting up the application fungible
    let application_id_fungible = client1
        .publish_and_create::<FungibleTokenAbi>(
            contract_fungible,
            service_fungible,
            &(),
            &state_fungible,
            &[],
            None,
        )
        .await
        .unwrap();

    // Setting up the application crowd funding
    let deadline = Timestamp::from(std::u64::MAX);
    let target = Amount::ONE;
    let state_crowd = InitializationArgument {
        owner: account_owner1,
        deadline,
        target,
    };
    let (contract_crowd, service_crowd) = local_net.build_example("crowd-funding").await.unwrap();
    let application_id_crowd = client1
        .publish_and_create::<CrowdFundingAbi>(
            contract_crowd,
            service_crowd,
            // TODO(#723): This hack will disappear soon.
            &application_id_fungible,
            &state_crowd,
            &[application_id_fungible.forget_abi()],
            None,
        )
        .await
        .unwrap();

    let mut node_service1 = client1.run_node_service(8080).await.unwrap();
    let mut node_service2 = client2.run_node_service(8081).await.unwrap();

    let app_fungible1 = FungibleApp(
        node_service1
            .make_application(&chain1, &application_id_fungible)
            .await,
    );

    let app_crowd1 = node_service1
        .make_application(&chain1, &application_id_crowd)
        .await;

    // Transferring tokens to user2 on chain2
    app_fungible1
        .transfer(
            &account_owner1,
            Amount::ONE,
            fungible::Account {
                chain_id: chain2,
                owner: account_owner2,
            },
        )
        .await;

    // Register the campaign on chain2.
    node_service2
        .request_application(&chain2, &application_id_crowd)
        .await;

    let app_crowd2 = node_service2
        .make_application(&chain2, &application_id_crowd)
        .await;

    // Transferring
    let mutation = format!(
        "pledgeWithTransfer(owner: {}, amount: \"{}\")",
        account_owner2.to_value(),
        Amount::ONE,
    );
    app_crowd2.mutate(mutation).await;

    // Make sure that the pledge is processed fast enough by client1.
    node_service1.process_inbox(&chain1).await;

    // Ending the campaign.
    app_crowd1.mutate("collect").await;

    // The rich gets their money back.
    app_fungible1
        .assert_balances([(account_owner1, Amount::from_tokens(6))])
        .await;

    node_service1.assert_is_running();
    node_service2.assert_is_running();
}

#[cfg(feature = "rocksdb")]
#[test_log::test(tokio::test)]
async fn test_rocks_db_wasm_end_to_end_matching_engine() {
    run_wasm_end_to_end_matching_engine(Database::RocksDb).await
}

#[cfg(feature = "aws")]
#[test_log::test(tokio::test)]
async fn test_dynamo_db_wasm_end_to_end_matching_engine() {
    run_wasm_end_to_end_matching_engine(Database::DynamoDb).await
}

#[cfg(feature = "scylladb")]
#[test_log::test(tokio::test)]
async fn test_scylla_db_wasm_end_to_end_matching_engine() {
    run_wasm_end_to_end_matching_engine(Database::ScyllaDb).await
}

async fn run_wasm_end_to_end_matching_engine(database: Database) {
    use fungible::{FungibleTokenAbi, InitialState};
    use matching_engine::{MatchingEngineAbi, OrderNature, Parameters, Price};

    let _guard = INTEGRATION_TEST_GUARD.lock().await;

    let network = Network::Grpc;
    let mut local_net = LocalNetwork::new_for_testing(database, network).unwrap();
    let client_admin = local_net.make_client(network);
    let client_a = local_net.make_client(network);
    let client_b = local_net.make_client(network);

    local_net.generate_initial_validator_config().await.unwrap();
    client_admin.create_genesis_config().await.unwrap();
    client_a.wallet_init(&[]).await.unwrap();
    client_b.wallet_init(&[]).await.unwrap();

    // Create initial server and client config.
    local_net.run().await.unwrap();
    let (contract_fungible, service_fungible) = local_net.build_example("fungible").await.unwrap();
    let (contract_matching, service_matching) =
        local_net.build_example("matching-engine").await.unwrap();

    let chain_admin = client_admin.get_wallet().default_chain().unwrap();
    let chain_a = client_admin.open_and_assign(&client_a).await.unwrap();
    let chain_b = client_admin.open_and_assign(&client_b).await.unwrap();

    // The players
    let owner_admin = get_fungible_account_owner(&client_admin);
    let owner_a = get_fungible_account_owner(&client_a);
    let owner_b = get_fungible_account_owner(&client_b);
    // The initial accounts on chain_a and chain_b
    let accounts0 = BTreeMap::from([(owner_a, Amount::from_tokens(10))]);
    let state_fungible0 = InitialState {
        accounts: accounts0,
    };
    let accounts1 = BTreeMap::from([(owner_b, Amount::from_tokens(9))]);
    let state_fungible1 = InitialState {
        accounts: accounts1,
    };

    // Setting up the application fungible on chain_a and chain_b
    let token0 = client_a
        .publish_and_create::<FungibleTokenAbi>(
            contract_fungible.clone(),
            service_fungible.clone(),
            &(),
            &state_fungible0,
            &[],
            None,
        )
        .await
        .unwrap();
    let token1 = client_b
        .publish_and_create::<FungibleTokenAbi>(
            contract_fungible,
            service_fungible,
            &(),
            &state_fungible1,
            &[],
            None,
        )
        .await
        .unwrap();

    // Now creating the service and exporting the applications
    let mut node_service_admin = client_admin.run_node_service(8080).await.unwrap();
    let mut node_service_a = client_a.run_node_service(8081).await.unwrap();
    let mut node_service_b = client_b.run_node_service(8082).await.unwrap();

    let app_fungible0_a = FungibleApp(node_service_a.make_application(&chain_a, &token0).await);
    let app_fungible1_b = FungibleApp(node_service_b.make_application(&chain_b, &token1).await);
    app_fungible0_a
        .assert_balances([
            (owner_a, Amount::from_tokens(10)),
            (owner_b, Amount::ZERO),
            (owner_admin, Amount::ZERO),
        ])
        .await;
    app_fungible1_b
        .assert_balances([
            (owner_a, Amount::ZERO),
            (owner_b, Amount::from_tokens(9)),
            (owner_admin, Amount::ZERO),
        ])
        .await;

    node_service_admin
        .request_application(&chain_admin, &token0)
        .await;
    let app_fungible0_admin = FungibleApp(
        node_service_admin
            .make_application(&chain_admin, &token0)
            .await,
    );
    node_service_admin
        .request_application(&chain_admin, &token1)
        .await;
    let app_fungible1_admin = FungibleApp(
        node_service_admin
            .make_application(&chain_admin, &token1)
            .await,
    );
    app_fungible0_admin
        .assert_balances([
            (owner_a, Amount::ZERO),
            (owner_b, Amount::ZERO),
            (owner_admin, Amount::ZERO),
        ])
        .await;
    app_fungible1_admin
        .assert_balances([
            (owner_a, Amount::ZERO),
            (owner_b, Amount::ZERO),
            (owner_admin, Amount::ZERO),
        ])
        .await;

    // Setting up the application matching engine.
    let parameter = Parameters {
        tokens: [token0, token1],
    };
    let bytecode_id = node_service_admin
        .publish_bytecode(&chain_admin, contract_matching, service_matching)
        .await;
    let application_id_matching = node_service_admin
        .create_application::<MatchingEngineAbi>(
            &chain_admin,
            &bytecode_id,
            &parameter,
            &(),
            &[token0.forget_abi(), token1.forget_abi()],
        )
        .await;
    let app_matching_admin = MatchingEngineApp(
        node_service_admin
            .make_application(&chain_admin, &application_id_matching)
            .await,
    );
    node_service_a
        .request_application(&chain_a, &application_id_matching)
        .await;
    let app_matching_a = MatchingEngineApp(
        node_service_a
            .make_application(&chain_a, &application_id_matching)
            .await,
    );
    node_service_b
        .request_application(&chain_b, &application_id_matching)
        .await;
    let app_matching_b = MatchingEngineApp(
        node_service_b
            .make_application(&chain_b, &application_id_matching)
            .await,
    );

    // Now creating orders
    for price in [1, 2] {
        // 1 is expected not to match, but 2 is expected to match
        app_matching_a
            .order(matching_engine::Order::Insert {
                owner: owner_a,
                amount: Amount::from_tokens(3),
                nature: OrderNature::Bid,
                price: Price { price },
            })
            .await;
    }
    for price in [4, 2] {
        // price 2 is expected to match, but not 4.
        app_matching_b
            .order(matching_engine::Order::Insert {
                owner: owner_b,
                amount: Amount::from_tokens(4),
                nature: OrderNature::Ask,
                price: Price { price },
            })
            .await;
    }
    node_service_admin.process_inbox(&chain_admin).await;
    node_service_a.process_inbox(&chain_a).await;
    node_service_b.process_inbox(&chain_b).await;

    // Now reading the order_ids
    let order_ids_a = app_matching_admin.get_account_info(&owner_a).await;
    let order_ids_b = app_matching_admin.get_account_info(&owner_b).await;
    // The deal that occurred is that 6 token0 were exchanged for 3 token1.
    assert_eq!(order_ids_a.len(), 1); // The order of price 2 is completely filled.
    assert_eq!(order_ids_b.len(), 2); // The order of price 2 is partially filled.

    // Now cancelling all the orders
    for (owner, order_ids) in [(owner_a, order_ids_a), (owner_b, order_ids_b)] {
        for order_id in order_ids {
            app_matching_admin
                .order(matching_engine::Order::Cancel { owner, order_id })
                .await;
        }
    }
    node_service_admin.process_inbox(&chain_admin).await;

    // Check balances
    app_fungible0_a
        .assert_balances([
            (owner_a, Amount::from_tokens(1)),
            (owner_b, Amount::from_tokens(0)),
        ])
        .await;
    app_fungible0_admin
        .assert_balances([
            (owner_a, Amount::from_tokens(3)),
            (owner_b, Amount::from_tokens(6)),
        ])
        .await;
    app_fungible1_b
        .assert_balances([
            (owner_a, Amount::from_tokens(0)),
            (owner_b, Amount::from_tokens(1)),
        ])
        .await;
    app_fungible1_admin
        .assert_balances([
            (owner_a, Amount::from_tokens(3)),
            (owner_b, Amount::from_tokens(5)),
        ])
        .await;

    node_service_admin.assert_is_running();
    node_service_a.assert_is_running();
    node_service_b.assert_is_running();
}

#[cfg(feature = "rocksdb")]
#[test_log::test(tokio::test)]
async fn test_rocks_db_wasm_end_to_end_amm() {
    run_wasm_end_to_end_amm(Database::RocksDb).await
}

#[cfg(feature = "aws")]
#[test_log::test(tokio::test)]
async fn test_dynamo_db_wasm_end_to_end_amm() {
    run_wasm_end_to_end_amm(Database::DynamoDb).await
}

#[cfg(feature = "scylladb")]
#[test_log::test(tokio::test)]
async fn test_scylla_db_wasm_end_to_end_amm() {
    run_wasm_end_to_end_amm(Database::ScyllaDb).await
}

async fn run_wasm_end_to_end_amm(database: Database) {
    use amm::{AmmAbi, Parameters};
    use fungible::{FungibleTokenAbi, InitialState};
    let _guard = INTEGRATION_TEST_GUARD.lock().await;

    let network = Network::Grpc;
    let mut local_net = LocalNetwork::new_for_testing(database, network).unwrap();
    let client_admin = local_net.make_client(network);
    let client0 = local_net.make_client(network);
    let client1 = local_net.make_client(network);

    local_net.generate_initial_validator_config().await.unwrap();
    client_admin.create_genesis_config().await.unwrap();
    client0.wallet_init(&[]).await.unwrap();
    client1.wallet_init(&[]).await.unwrap();

    local_net.run().await.unwrap();
    let (contract_fungible, service_fungible) = local_net.build_example("fungible").await.unwrap();
    let (contract_amm, service_amm) = local_net.build_example("amm").await.unwrap();

    // Admin chain
    let chain_admin = client_admin.get_wallet().default_chain().unwrap();

    // User chains
    let chain0 = client_admin.open_and_assign(&client0).await.unwrap();
    let chain1 = client_admin.open_and_assign(&client1).await.unwrap();

    // Admin user
    let owner_admin = get_fungible_account_owner(&client_admin);

    // Users
    let owner0 = get_fungible_account_owner(&client0);
    let owner1 = get_fungible_account_owner(&client1);

    let mut node_service_admin = client_admin.run_node_service(8080).await.unwrap();
    let mut node_service0 = client0.run_node_service(8081).await.unwrap();
    let mut node_service1 = client1.run_node_service(8082).await.unwrap();

    // Amounts of token0 that will be owned by each user
    let state_fungible0 = InitialState {
        accounts: BTreeMap::from([
            (owner0, Amount::ZERO),
            (owner1, Amount::from_tokens(50)),
            (owner_admin, Amount::from_tokens(200)),
        ]),
    };

    // Amounts of token1 that will be owned by each user
    let state_fungible1 = InitialState {
        accounts: BTreeMap::from([
            (owner0, Amount::from_tokens(50)),
            (owner1, Amount::ZERO),
            (owner_admin, Amount::from_tokens(200)),
        ]),
    };

    // Create fungible applications on the Admin chain, which will hold
    // the token0 and token1 amounts
    let fungible_bytecode_id = node_service_admin
        .publish_bytecode(&chain_admin, contract_fungible, service_fungible)
        .await;

    let token0 = node_service_admin
        .create_application::<FungibleTokenAbi>(
            &chain_admin,
            &fungible_bytecode_id,
            &(),
            &state_fungible0,
            &[],
        )
        .await;
    let token1 = node_service_admin
        .create_application::<FungibleTokenAbi>(
            &chain_admin,
            &fungible_bytecode_id,
            &(),
            &state_fungible1,
            &[],
        )
        .await;

    // Create wrappers
    let app_fungible0_admin = FungibleApp(
        node_service_admin
            .make_application(&chain_admin, &token0)
            .await,
    );
    let app_fungible1_admin = FungibleApp(
        node_service_admin
            .make_application(&chain_admin, &token1)
            .await,
    );

    // Check initial balances
    app_fungible0_admin
        .assert_balances([
            (owner0, Amount::ZERO),
            (owner1, Amount::from_tokens(50)),
            (owner_admin, Amount::from_tokens(200)),
        ])
        .await;
    app_fungible1_admin
        .assert_balances([
            (owner0, Amount::from_tokens(50)),
            (owner1, Amount::ZERO),
            (owner_admin, Amount::from_tokens(200)),
        ])
        .await;

    let parameters = Parameters {
        tokens: [token0, token1],
    };

    // Create AMM application on Admin chain
    let bytecode_id = node_service_admin
        .publish_bytecode(&chain_admin, contract_amm, service_amm)
        .await;
    let application_id_amm = node_service_admin
        .create_application::<AmmAbi>(
            &chain_admin,
            &bytecode_id,
            &parameters,
            &(),
            &[token0.forget_abi(), token1.forget_abi()],
        )
        .await;

    let owner_amm = fungible::AccountOwner::Application(application_id_amm.forget_abi());

    // Create AMM wrappers
    let app_amm_admin = AmmApp(
        node_service_admin
            .make_application(&chain_admin, &application_id_amm)
            .await,
    );
    node_service0
        .request_application(&chain0, &application_id_amm)
        .await;
    let app_amm0 = AmmApp(
        node_service0
            .make_application(&chain0, &application_id_amm)
            .await,
    );
    node_service1
        .request_application(&chain1, &application_id_amm)
        .await;
    let app_amm1 = AmmApp(
        node_service1
            .make_application(&chain1, &application_id_amm)
            .await,
    );

    // Initial balances for both tokens are 0

    // Adding liquidity for token0 and token1 by owner0
    // We have to call it from the AMM instance in the owner0's
    // user chain, chain0, so it's properly authenticated
    app_amm_admin
        .add_liquidity(
            owner_admin,
            Amount::from_tokens(100),
            Amount::from_tokens(100),
        )
        .await;

    // Ownership of owner0 tokens should be with the AMM now
    app_fungible0_admin
        .assert_balances([
            (owner0, Amount::ZERO),
            (owner1, Amount::from_tokens(50)),
            (owner_admin, Amount::from_tokens(100)),
            (owner_amm, Amount::from_tokens(100)),
        ])
        .await;
    app_fungible1_admin
        .assert_balances([
            (owner0, Amount::from_tokens(50)),
            (owner1, Amount::ZERO),
            (owner_admin, Amount::from_tokens(100)),
            (owner_amm, Amount::from_tokens(100)),
        ])
        .await;

    // Adding liquidity for token0 and token1 by owner1 now
    app_amm_admin
        .add_liquidity(
            owner_admin,
            Amount::from_tokens(100),
            Amount::from_tokens(100),
        )
        .await;

    app_fungible0_admin
        .assert_balances([
            (owner0, Amount::ZERO),
            (owner1, Amount::from_tokens(50)),
            (owner_admin, Amount::ZERO),
            (owner_amm, Amount::from_tokens(200)),
        ])
        .await;
    app_fungible1_admin
        .assert_balances([
            (owner0, Amount::from_tokens(50)),
            (owner1, Amount::ZERO),
            (owner_admin, Amount::ZERO),
            (owner_amm, Amount::from_tokens(200)),
        ])
        .await;

    app_amm1.swap(owner1, 0, Amount::from_tokens(50)).await;
    node_service_admin.process_inbox(&chain_admin).await;

    app_fungible0_admin
        .assert_balances([
            (owner0, Amount::ZERO),
            (owner1, Amount::ZERO),
            (owner_admin, Amount::ZERO),
            (owner_amm, Amount::from_tokens(250)),
        ])
        .await;
    app_fungible1_admin
        .assert_balances([
            (owner0, Amount::from_tokens(50)),
            (owner1, Amount::from_tokens(40)),
            (owner_admin, Amount::ZERO),
            (owner_amm, Amount::from_tokens(160)),
        ])
        .await;

    // Can only remove liquidity from main chain
    app_amm_admin
        .remove_liquidity(owner1, 0, Amount::from_tokens(62))
        .await;

    app_fungible0_admin
        .assert_balances([
            (owner0, Amount::ZERO),
            (owner1, Amount::from_tokens(62)),
            (owner_admin, Amount::ZERO),
            (owner_amm, Amount::from_tokens(188)),
        ])
        .await;
    app_fungible1_admin
        .assert_balances([
            (owner0, Amount::from_tokens(50)),
            (owner1, Amount::from_milli(79_680)),
            (owner_admin, Amount::ZERO),
            (owner_amm, Amount::from_milli(120_320)),
        ])
        .await;

    app_amm1.swap(owner1, 0, Amount::from_tokens(50)).await;
    node_service_admin.process_inbox(&chain_admin).await;

    app_fungible0_admin
        .assert_balances([
            (owner0, Amount::ZERO),
            (owner1, Amount::from_tokens(12)),
            (owner_admin, Amount::ZERO),
            (owner_amm, Amount::from_tokens(238)),
        ])
        .await;
    app_fungible1_admin
        .assert_balances([
            (owner0, Amount::from_tokens(50)),
            (owner1, Amount::from_atto(104_957310924369747899)),
            (owner_admin, Amount::ZERO),
            (owner_amm, Amount::from_atto(95_042689075630252101)),
        ])
        .await;

    app_amm0.swap(owner0, 1, Amount::from_tokens(50)).await;
    node_service_admin.process_inbox(&chain_admin).await;

    app_fungible0_admin
        .assert_balances([
            (owner0, Amount::from_atto(82_044810916287757646)),
            (owner1, Amount::from_tokens(12)),
            (owner_admin, Amount::ZERO),
            (owner_amm, Amount::from_atto(155_955189083712242354)),
        ])
        .await;
    app_fungible1_admin
        .assert_balances([
            (owner0, Amount::ZERO),
            (owner1, Amount::from_atto(104_957310924369747899)),
            (owner_admin, Amount::ZERO),
            (owner_amm, Amount::from_atto(145_042689075630252101)),
        ])
        .await;

    node_service_admin.assert_is_running();
    node_service0.assert_is_running();
    node_service1.assert_is_running();
}

#[cfg(feature = "rocksdb")]
#[tokio::test]
async fn test_rocks_db_wasm_end_to_end_review() {
    run_wasm_end_to_end_review(Database::RocksDb).await
}

#[cfg(feature = "aws")]
#[test_log::test(tokio::test)]
async fn test_dynamo_db_wasm_end_to_end_review() {
    run_wasm_end_to_end_review(Database::DynamoDb).await
}

#[cfg(feature = "scylladb")]
#[test_log::test(tokio::test)]
async fn test_scylla_db_wasm_end_to_end_review() {
    run_wasm_end_to_end_review(Database::ScyllaDb).await
}

async fn run_wasm_end_to_end_review(database: Database) {
    console_subscriber::init();
    let wasm_path = std::env::current_dir()
        .unwrap()
        .join("../../res-peer/target/wasm32-unknown-unknown/release/");
    let _guard = INTEGRATION_TEST_GUARD.lock().await;

    let network = Network::Grpc;
    let mut local_net = LocalNetwork::new_for_testing(database, network).unwrap();
    let client_admin = local_net.make_client(network);
    let client0 = local_net.make_client(network);

    local_net.generate_initial_validator_config().await.unwrap();
    client_admin.create_genesis_config().await.unwrap();
    client0.wallet_init(&[]).await.unwrap();
    local_net.run().await.unwrap();

    let chain_admin = client_admin.get_wallet().default_chain().unwrap();
    let chain0 = client_admin.open_and_assign(&client0).await.unwrap();

    // Create credit application on Admin chain
    let credit_app_id = client_admin
        .publish_and_create::<credit::CreditAbi>(
            wasm_path.join("credit_contract.wasm"),
            wasm_path.join("credit_service.wasm"),
            &(),
            &credit::InitialState {
                initial_supply: Amount::from_tokens(100000),
                amount_alive_ms: 70,
            },
            &[],
            chain_admin,
        )
        .await
        .unwrap();

    // Create foundation application on Admin chain
    let foundation_app_id = client_admin
        .publish_and_create::<foundation::FoundationAbi>(
            wasm_path.join("foundation_contract.wasm"),
            wasm_path.join("foundation_service.wasm"),
            &(),
            &foundation::InitialState {
                review_reward_percent: 10,
                review_reward_factor: 2,
                author_reward_percent: 10,
                author_reward_factor: 2,
                activity_reward_percent: 10,
            },
            &[],
            chain_admin,
        )
        .await
        .unwrap();

    // Create feed application on Admin chain
    let feed_app_id = client_admin
        .publish_and_create::<feed::FeedAbi>(
            wasm_path.join("feed_contract.wasm"),
            wasm_path.join("feed_service.wasm"),
            &feed::FeedParameters {
                credit_app_id,
                foundation_app_id,
            },
            &feed::InitialState {
                react_interval_ms: 600000,
            },
            &[credit_app_id.forget_abi(), foundation_app_id.forget_abi()],
            chain_admin,
        )
        .await
        .unwrap();

    // Create market application on Admin chain
    let market_app_id = client_admin
        .publish_and_create::<market::MarketAbi>(
            wasm_path.join("market_contract.wasm"),
            wasm_path.join("market_service.wasm"),
            &market::MarketParameters {
                credit_app_id,
                foundation_app_id,
            },
            &market::InitialState {
                credits_per_linera: Amount::from_milli(13),
                max_credits_percent: 30,
                trade_fee_percent: 3,
            },
            &[credit_app_id.forget_abi(), foundation_app_id.forget_abi()],
            chain_admin,
        )
        .await
        .unwrap();

    // Create res-peer application on Admin chain
    let review_app_id = client_admin
        .publish_and_create::<review::ReviewAbi>(
            wasm_path.join("review_contract.wasm"),
            wasm_path.join("review_service.wasm"),
            &review::ReviewParameters {
                feed_app_id,
                credit_app_id,
                foundation_app_id,
                market_app_id,
            },
            &review::InitialState {
                content_approved_threshold: 60,
                content_rejected_threshold: 30,
                asset_approved_threshold: 60,
                asset_rejected_threshold: 30,
                reviewer_approved_threshold: 60,
                reviewer_rejected_threshold: 30,
            },
            &[
                feed_app_id.forget_abi(),
                credit_app_id.forget_abi(),
                foundation_app_id.forget_abi(),
                market_app_id.forget_abi(),
            ],
            chain_admin,
        )
        .await
        .unwrap();

    let mut node_service_admin = client_admin.run_node_service(8080).await.unwrap();
    let mut node_service0 = client0.run_node_service(8081).await.unwrap();

    let app_review_admin = node_service_admin
        .make_application(&chain_admin, &review_app_id)
        .await;
    node_service0
        .request_application(&chain0, &review_app_id)
        .await;
    let app_review0 = node_service0
        .make_application(&chain0, &review_app_id)
        .await;

    tokio::time::sleep(Duration::from_secs(1)).await;
    app_review0.mutate("requestSubscribe").await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    app_review_admin.mutate("requestSubscribe").await;

    node_service_admin.assert_is_running();
    node_service0.assert_is_running();
}
