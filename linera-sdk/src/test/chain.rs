// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A reference to a single micro-chain inside a [`TestValidator`].
//!
//! This allows manipulating a test micro-chain.

use super::{BlockBuilder, TestValidator};
use linera_base::{
    crypto::{KeyPair, PublicKey},
    identifiers::{ChainDescription, ChainId},
};
use linera_chain::data_types::Certificate;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A reference to a single micro-chain inside a [`TestValidator`].
pub struct ActiveChain {
    key_pair: KeyPair,
    description: ChainDescription,
    tip: Arc<Mutex<Option<Certificate>>>,
    validator: TestValidator,
}

impl Clone for ActiveChain {
    fn clone(&self) -> Self {
        ActiveChain {
            key_pair: self.key_pair.copy(),
            description: self.description,
            tip: self.tip.clone(),
            validator: self.validator.clone(),
        }
    }
}

impl ActiveChain {
    /// Creates a new [`ActiveChain`] instance referencing a new empty micro-chain in the
    /// `validator`.
    ///
    /// The micro-chain has a single owner that uses the `key_pair` to produce blocks. The
    /// `description` is used as the identifier of the micro-chain.
    pub fn new(key_pair: KeyPair, description: ChainDescription, validator: TestValidator) -> Self {
        ActiveChain {
            key_pair,
            description,
            tip: Arc::default(),
            validator,
        }
    }

    /// Returns the [`ChainId`] of this micro-chain.
    pub fn id(&self) -> ChainId {
        self.description.into()
    }

    /// Returns the [`PublicKey`] of the owner of this micro-chain.
    pub fn public_key(&self) -> PublicKey {
        self.key_pair.public()
    }

    /// Adds a block to this micro-chain.
    ///
    /// The `block_builder` parameter is a closure that should use the [`BlockBuilder`] parameter
    /// to provide the block's contents.
    pub async fn add_block(&self, block_builder: impl FnOnce(&mut BlockBuilder)) {
        let mut tip = self.tip.lock().await;
        let mut block = BlockBuilder::new(
            self.description.into(),
            self.key_pair.public().into(),
            tip.as_ref(),
            self.validator.clone(),
        );

        block_builder(&mut block);

        let certificate = block.sign().await;

        self.validator
            .worker()
            .await
            .fully_handle_certificate(certificate.clone(), vec![])
            .await
            .expect("Rejected certificate");

        *tip = Some(certificate);
    }
}
