// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use criterion::{criterion_group, criterion_main, Criterion};
use futures::{
    stream::{self, FuturesUnordered},
    Stream, StreamExt,
};
use linera_base::{
    data_types::Amount,
    identifiers::{Account, ChainId, Owner},
};
use linera_chain::data_types::Certificate;
use linera_execution::system::Recipient;
use linera_sdk::test::{ActiveChain, TestValidator};
use tokio::runtime::Runtime;

/// Benchmarks several transactions transfering tokens across chains.
fn cross_chain_native_token_transfers(criterion: &mut Criterion) {
    let chain_count = 100;
    let accounts_per_chain = 1;
    let transfers_per_account = 100;

    criterion.bench_function("same_chain_native_token_transfers", |bencher| {
        bencher
            .to_async(Runtime::new().expect("Failed to create Tokio runtime"))
            .iter_custom(|iterations| async move {
                let mut total_time = Duration::ZERO;

                for _ in 0..iterations {
                    let chains = setup_native_token_balances(
                        chain_count,
                        accounts_per_chain,
                        transfers_per_account,
                    )
                    .await;

                    let transfers = prepare_transfers(chains, transfers_per_account);

                    let measurement = Instant::now();
                    transfers.collect::<()>().await;
                    total_time += measurement.elapsed();
                }

                total_time
            })
    });

    let metrics = prometheus::TextEncoder::new()
        .encode_to_string(&prometheus::gather())
        .expect("Failed to format collected metrics");
    println!("METRICS");
    println!("{metrics}");
}

/// Benchmarks several certificates transfering tokens across chains.
fn cross_chain_native_token_transfer_certificates(criterion: &mut Criterion) {
    let chain_count = 100;
    let accounts_per_chain = 1;
    let transfers_per_account = 100;

    criterion.bench_function(
        "same_chain_native_token_transfers_certificates",
        |bencher| {
            bencher
                .to_async(Runtime::new().expect("Failed to create Tokio runtime"))
                .iter_custom(|iterations| async move {
                    let mut total_time = Duration::ZERO;

                    for _ in 0..iterations {
                        let chains = setup_native_token_balances(
                            chain_count,
                            accounts_per_chain,
                            transfers_per_account,
                        )
                        .await;
                        let setup_validator = chains[0].validator().clone();
                        let key_pair = setup_validator.key_pair().copy();

                        prepare_transfers(chains, transfers_per_account)
                            .collect::<()>()
                            .await;

                        let mut certificates = setup_validator.get_all_certificates().await;

                        let bench_validator = TestValidator::with_key_pair(key_pair).await;
                        let admin_chain_certificates = certificates
                            .remove(&ChainId::root(0))
                            .expect("Missing admin chain certificates");
                        recreate_chains(&bench_validator, admin_chain_certificates).await;

                        let measurement = Instant::now();
                        replay_certificates(&bench_validator, certificates).await;
                        total_time += measurement.elapsed();
                    }

                    total_time
                })
        },
    );

    let metrics = prometheus::TextEncoder::new()
        .encode_to_string(&prometheus::gather())
        .expect("Failed to format collected metrics");
    println!("METRICS");
    println!("{metrics}");
}

/// Provides each chain used in the benchmark with enough tokens to transfer.
async fn setup_native_token_balances(
    chain_count: usize,
    accounts_per_chain: usize,
    transfers_per_account: usize,
) -> Vec<ActiveChain> {
    let initial_balance = transfers_per_account as u128;

    let validator = TestValidator::new().await;
    let chains = stream::iter(0..chain_count)
        .then(|_| validator.new_chain())
        .collect::<Vec<_>>()
        .await;

    let admin_chain = validator.get_chain(&ChainId::root(0));

    for chain in &chains {
        let recipient = Recipient::Account(Account {
            chain_id: chain.id(),
            owner: Some(chain.public_key().into()),
        });

        // TODO: Support benchmarking chains with multiple owner accounts
        assert_eq!(accounts_per_chain, 1);
        admin_chain
            .add_block(|block| {
                block.with_native_token_transfer(
                    None,
                    recipient,
                    Amount::from_tokens(initial_balance),
                );
            })
            .await;

        chain.handle_received_messages().await;
    }

    chains
}

/// Returns a stream that concurrently adds blocks to all `chains` to transfer tokens.
fn prepare_transfers(
    chains: Vec<ActiveChain>,
    transfers_per_account: usize,
) -> impl Stream<Item = ()> {
    let accounts = chains
        .iter()
        .map(|chain| Account {
            chain_id: chain.id(),
            owner: Some(chain.public_key().into()),
        })
        .collect::<Vec<_>>();

    let chain_transfers = chains
        .into_iter()
        .enumerate()
        .map(|(index, chain)| {
            let chain_id = chain.id();
            let sender = Some(Owner::from(chain.public_key()));

            let transfers = accounts
                .iter()
                .copied()
                .filter(move |recipient| recipient.chain_id != chain_id)
                .cycle()
                .skip(index)
                .take(transfers_per_account)
                .map(Recipient::Account)
                .map(move |recipient| (sender, recipient))
                .collect::<Vec<_>>();

            (chain, transfers)
        })
        .collect::<Vec<_>>();

    chain_transfers
        .into_iter()
        .map(move |(chain, transfers)| async move {
            tokio::spawn(async move {
                for (sender, recipient) in transfers {
                    chain
                        .add_block(|block| {
                            block.with_native_token_transfer(sender, recipient, Amount::ONE);
                        })
                        .await;
                }
            })
            .await
            .unwrap();
        })
        .collect::<FuturesUnordered<_>>()
}

/// Executes the admin chain certificates, recreating the chains and providing them with initial
/// tokens.
async fn recreate_chains(validator: &TestValidator, certificates: Vec<Certificate>) {
    let chain = validator.get_chain(&ChainId::root(0));
    // let certificates_after_genesis = certificates.into_iter().skip(1);

    for certificate in certificates {
        chain.add_certified_block(certificate).await;
    }
}

/// Executes all the `certificates` to add blocks to the chains.
///
/// Runs a separate task for each chain.
async fn replay_certificates(
    validator: &TestValidator,
    certificates: HashMap<ChainId, Vec<Certificate>>,
) {
    certificates
        .into_iter()
        .enumerate()
        .map(move |(block_height, (chain_id, chain_certificates))| {
            let validator = validator.clone();
            tokio::spawn(async move {
                for certificate in chain_certificates {
                    validator
                        .handle_raw_certificate(certificate)
                        .await
                        .unwrap_or_else(|_| {
                            panic!(
                                "Failed to handle certificate for \
                                block #{block_height} of chain {chain_id}"
                            )
                        });
                }
            })
        })
        .collect::<FuturesUnordered<_>>()
        .for_each(|result| async move {
            assert!(result.is_ok(), "Failed to replay certificates");
        })
        .await;
}

criterion_group!(
    benches,
    cross_chain_native_token_transfers,
    cross_chain_native_token_transfer_certificates
);
criterion_main!(benches);
