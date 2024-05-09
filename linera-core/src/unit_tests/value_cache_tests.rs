// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{borrow::Cow, collections::BTreeSet};

use linera_base::{crypto::CryptoHash, data_types::BlockHeight, identifiers::ChainId};
use linera_chain::data_types::{CertificateValue, HashedCertificateValue};
use linera_execution::committee::Epoch;

use super::CertificateValueCache;

/// Tests attempt to retrieve non-existent value.
#[tokio::test]
async fn test_retrieve_missing_value() {
    let cache = CertificateValueCache::default();
    let hash = CryptoHash::test_hash("Missing value");

    assert!(cache.get(&hash).await.is_none());
    assert!(cache.keys().await.is_empty());
}

/// Tests inserting a value in the cache.
#[tokio::test]
async fn test_insert_single_value() {
    let cache = CertificateValueCache::default();
    let value = create_dummy_value(0);
    let hash = value.hash();

    assert!(cache.insert(Cow::Borrowed(&value)).await);
    assert!(cache.contains(&hash).await);
    assert_eq!(cache.get(&hash).await, Some(value));
    assert_eq!(cache.keys().await, BTreeSet::from([hash]));
}

/// Creates a new dummy [`HashedCertificateValue`] to use in the tests.
fn create_dummy_value(height: impl Into<BlockHeight>) -> HashedCertificateValue {
    CertificateValue::Timeout {
        chain_id: ChainId(CryptoHash::test_hash("Fake chain ID")),
        height: height.into(),
        epoch: Epoch(0),
    }
    .into()
}
