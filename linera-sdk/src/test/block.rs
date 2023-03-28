// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::TestValidator;
use crate::ToBcsBytes;
use linera_base::{
    data_types::Timestamp,
    identifiers::{ApplicationId, ChainId, EffectId, Owner},
};
use linera_chain::data_types::{
    Block, Certificate, HashedValue, LiteVote, Message, SignatureAggregator,
};
use linera_execution::system::SystemOperation;
use std::mem;

pub struct BlockBuilder {
    block: Block,
    incoming_effects: Vec<EffectId>,
    validator: TestValidator,
}

impl BlockBuilder {
    pub(crate) fn new(
        chain_id: ChainId,
        owner: Owner,
        previous_block: Option<&Certificate>,
        validator: TestValidator,
    ) -> Self {
        let previous_block_hash = previous_block.map(|certificate| certificate.value.hash());
        let height = previous_block
            .and_then(|certificate| certificate.value.block().height.try_add_one().ok())
            .unwrap_or_default();

        BlockBuilder {
            block: Block {
                epoch: 0.into(),
                chain_id,
                incoming_messages: vec![],
                operations: vec![],
                previous_block_hash,
                height,
                authenticated_signer: Some(owner),
                timestamp: Timestamp::from(0),
            },
            incoming_effects: Vec::new(),
            validator,
        }
    }

    pub(crate) fn with_system_operation(&mut self, operation: SystemOperation) -> &mut Self {
        self.block
            .operations
            .push((linera_execution::ApplicationId::System, operation.into()));
        self
    }

    pub fn with_operation(
        &mut self,
        application: ApplicationId,
        operation: impl ToBcsBytes,
    ) -> &mut Self {
        self.block.operations.push((
            application.into(),
            operation
                .to_bcs_bytes()
                .expect("Failed to serialize operation")
                .into(),
        ));
        self
    }

    pub fn with_message(&mut self, effect_id: EffectId) -> &mut Self {
        self.incoming_effects.push(effect_id);
        self
    }

    pub(crate) fn with_raw_messages(
        &mut self,
        messages: impl IntoIterator<Item = Message>,
    ) -> &mut Self {
        self.block.incoming_messages.extend(messages);
        self
    }

    pub(crate) async fn sign(mut self, validator: &TestValidator) -> Certificate {
        self.collect_incoming_effects().await;

        let (effects, info) = validator
            .worker()
            .await
            .stage_block_execution(&self.block)
            .await
            .expect("Failed to execute block");
        let state_hash = info.info.state_hash.expect("Missing execution state hash");

        let value = HashedValue::new_confirmed(self.block, effects, state_hash);
        let vote = LiteVote::new(value.lite(), validator.key_pair());
        let mut builder = SignatureAggregator::new(value, validator.committee());
        builder
            .append(vote.validator, vote.signature)
            .unwrap()
            .unwrap()
    }

    async fn collect_incoming_effects(&mut self) {
        let chain_id = self.block.chain_id;

        for effect_id in mem::take(&mut self.incoming_effects) {
            let message = self
                .validator
                .worker()
                .await
                .find_incoming_message(chain_id, effect_id)
                .await
                .expect("Failed to find message to receive in block")
                .expect("Message that block should consume has not been emitted");

            self.block.incoming_messages.push(message);
        }
    }
}
