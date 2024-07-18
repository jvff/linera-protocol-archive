// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Operations that don't persist any changes to the chain state.

use linera_base::{
    data_types::{ArithmeticError, Timestamp},
    ensure,
};
use linera_chain::{
    data_types::{
        Block, BlockExecutionOutcome, BlockProposal, ExecutedBlock, HashedCertificateValue,
        IncomingMessage, MessageAction, ProposalContent,
    },
    manager,
};
use linera_execution::{Query, Response, UserApplicationDescription, UserApplicationId};
use linera_storage::Storage;
use linera_views::views::{View, ViewError};
#[cfg(with_testing)]
use {
    linera_base::{crypto::CryptoHash, data_types::BlockHeight, identifiers::BytecodeId},
    linera_chain::data_types::{Certificate, Event, Origin},
    linera_execution::BytecodeLocation,
};

use super::{check_block_epoch, ChainWorkerState};
use crate::{
    data_types::{ChainInfo, ChainInfoQuery, ChainInfoResponse},
    worker::WorkerError,
};

/// Wrapper type that rolls back changes to the `chain` state when dropped.
pub struct ChainWorkerStateWithTemporaryChanges<'state, StorageClient>(
    &'state mut ChainWorkerState<StorageClient>,
)
where
    StorageClient: Storage + Clone + Send + Sync + 'static,
    ViewError: From<StorageClient::StoreError>;

impl<'state, StorageClient> ChainWorkerStateWithTemporaryChanges<'state, StorageClient>
where
    StorageClient: Storage + Clone + Send + Sync + 'static,
    ViewError: From<StorageClient::StoreError>,
{
    /// Creates a new [`ChainWorkerStateWithAttemptedChanges`] instance to temporarily change the
    /// `state`.
    pub(super) async fn new(state: &'state mut ChainWorkerState<StorageClient>) -> Self {
        assert!(
            !state.chain.has_pending_changes().await,
            "`ChainStateView` has unexpected leftover changes"
        );

        ChainWorkerStateWithTemporaryChanges(state)
    }

    /// Returns a stored [`Certificate`] for the chain's block at the requested [`BlockHeight`].
    #[cfg(with_testing)]
    pub(super) async fn read_certificate(
        &mut self,
        height: BlockHeight,
    ) -> Result<Option<Certificate>, WorkerError> {
        self.0.ensure_is_active()?;
        let certificate_hash = match self.0.chain.confirmed_log.get(height.try_into()?).await? {
            Some(hash) => hash,
            None => return Ok(None),
        };
        let certificate = self.0.storage.read_certificate(certificate_hash).await?;
        Ok(Some(certificate))
    }

    /// Searches for an event in one of the chain's inboxes.
    #[cfg(with_testing)]
    pub(super) async fn find_event_in_inbox(
        &mut self,
        inbox_id: Origin,
        certificate_hash: CryptoHash,
        height: BlockHeight,
        index: u32,
    ) -> Result<Option<Event>, WorkerError> {
        self.0.ensure_is_active()?;

        let mut inbox = self.0.chain.inboxes.try_load_entry_mut(&inbox_id).await?;
        let mut events = inbox.added_events.iter_mut().await?;

        Ok(events
            .find(|event| {
                event.certificate_hash == certificate_hash
                    && event.height == height
                    && event.index == index
            })
            .cloned())
    }

    /// Queries an application's state on the chain.
    pub(super) async fn query_application(
        &mut self,
        query: Query,
    ) -> Result<Response, WorkerError> {
        self.0.ensure_is_active()?;
        let local_time = self.0.storage.clock().current_time();
        let response = self
            .0
            .chain
            .query_application(
                local_time,
                query,
                &mut self.0.execution_state_receiver,
                &mut self.0.runtime_request_sender,
            )
            .await?;
        Ok(response)
    }

    /// Returns the [`BytecodeLocation`] for the requested [`BytecodeId`], if it is known by the
    /// chain.
    #[cfg(with_testing)]
    pub(super) async fn read_bytecode_location(
        &mut self,
        bytecode_id: BytecodeId,
    ) -> Result<Option<BytecodeLocation>, WorkerError> {
        self.0.ensure_is_active()?;
        let response = self.0.chain.read_bytecode_location(bytecode_id).await?;
        Ok(response)
    }

    /// Returns an application's description.
    pub(super) async fn describe_application(
        &mut self,
        application_id: UserApplicationId,
    ) -> Result<UserApplicationDescription, WorkerError> {
        self.0.ensure_is_active()?;
        let response = self.0.chain.describe_application(application_id).await?;
        Ok(response)
    }

    /// Executes a block without persisting any changes to the state.
    pub(super) async fn stage_block_execution(
        &mut self,
        block: Block,
    ) -> Result<(ExecutedBlock, ChainInfoResponse), WorkerError> {
        self.0.ensure_is_active()?;

        let local_time = self.0.storage.clock().current_time();
        let signer = block.authenticated_signer;

        let executed_block = Box::pin(self.0.chain.execute_block(&block, local_time, None))
            .await?
            .with(block);

        let mut response = ChainInfoResponse::new(&self.0.chain, None);
        if let Some(signer) = signer {
            response.info.requested_owner_balance = self
                .0
                .chain
                .execution_state
                .system
                .balances
                .get(&signer)
                .await?;
        }

        Ok((executed_block, response))
    }

    /// Validates a block proposed to extend this chain.
    pub(super) async fn validate_block(
        &mut self,
        proposal: &BlockProposal,
    ) -> Result<Option<(BlockExecutionOutcome, Timestamp)>, WorkerError> {
        let BlockProposal {
            content:
                ProposalContent {
                    block,
                    round,
                    forced_oracle_responses,
                },
            owner,
            hashed_certificate_values,
            hashed_blobs,
            validated_block_certificate,
            signature: _,
        } = proposal;
        ensure!(
            validated_block_certificate.is_some() == forced_oracle_responses.is_some(),
            WorkerError::InvalidBlockProposal(
                "Must contain a validation certificate if and only if \
                 oracle responses are forced from a previous round"
                    .to_string()
            )
        );
        self.0.ensure_is_active()?;
        // Check the epoch.
        let (epoch, committee) = self
            .0
            .chain
            .execution_state
            .system
            .current_committee()
            .expect("chain is active");
        check_block_epoch(epoch, block)?;
        // Check the authentication of the block.
        let public_key = self
            .0
            .chain
            .manager
            .get()
            .verify_owner(proposal)
            .ok_or(WorkerError::InvalidOwner)?;
        proposal.check_signature(public_key)?;
        if let Some(lite_certificate) = validated_block_certificate {
            // Verify that this block has been validated by a quorum before.
            lite_certificate.check(committee)?;
        } else if let Some(signer) = block.authenticated_signer {
            // Check the authentication of the operations in the new block.
            ensure!(signer == *owner, WorkerError::InvalidSigner(signer));
        }
        // Check if the chain is ready for this new block proposal.
        // This should always pass for nodes without voting key.
        self.0.chain.tip_state.get().verify_block_chaining(block)?;
        if self.0.chain.manager.get().check_proposed_block(proposal)? == manager::Outcome::Skip {
            return Ok(None);
        }
        // Update the inboxes so that we can verify the provided hashed certificate values are
        // legitimately required.
        // Actual execution happens below, after other validity checks.
        self.0.chain.remove_events_from_inboxes(block).await?;
        // Verify that all required bytecode hashed certificate values and blobs are available, and no
        // unrelated ones provided.
        self.0
            .check_no_missing_blobs(
                block,
                block.published_blob_ids(),
                hashed_certificate_values,
                hashed_blobs,
            )
            .await?;
        // Write the values so that the bytecode is available during execution.
        // TODO(#2199): We should not persist anything in storage before the block is confirmed.
        self.0
            .storage
            .write_hashed_certificate_values(hashed_certificate_values)
            .await?;
        let local_time = self.0.storage.clock().current_time();
        ensure!(
            block.timestamp.duration_since(local_time) <= self.0.config.grace_period,
            WorkerError::InvalidTimestamp
        );
        self.0.storage.clock().sleep_until(block.timestamp).await;
        let local_time = self.0.storage.clock().current_time();
        let outcome = Box::pin(self.0.chain.execute_block(
            block,
            local_time,
            forced_oracle_responses.clone(),
        ))
        .await?;
        if let Some(lite_certificate) = &validated_block_certificate {
            let value = HashedCertificateValue::new_validated(outcome.clone().with(block.clone()));
            lite_certificate
                .clone()
                .with_value(value)
                .ok_or_else(|| WorkerError::InvalidLiteCertificate)?;
        }
        if round.is_fast() {
            ensure!(
                outcome
                    .oracle_responses
                    .iter()
                    .flatten()
                    .all(|response| response.is_permitted_in_fast_blocks()),
                WorkerError::FastBlockUsingOracles
            );
        }
        // Check if the counters of tip_state would be valid.
        self.0
            .chain
            .tip_state
            .get()
            .verify_counters(block, &outcome)?;
        // Verify that the resulting chain would have no unconfirmed incoming messages.
        self.0.chain.validate_incoming_messages().await?;
        Ok(Some((outcome, local_time)))
    }

    /// Prepares a [`ChainInfoResponse`] for a [`ChainInfoQuery`].
    pub(super) async fn prepare_chain_info_response(
        &mut self,
        query: ChainInfoQuery,
    ) -> Result<ChainInfoResponse, WorkerError> {
        let chain = &self.0.chain;
        let mut info = ChainInfo::from(chain);
        if query.request_committees {
            info.requested_committees = Some(chain.execution_state.system.committees.get().clone());
        }
        if let Some(owner) = query.request_owner_balance {
            info.requested_owner_balance =
                chain.execution_state.system.balances.get(&owner).await?;
        }
        if let Some(next_block_height) = query.test_next_block_height {
            ensure!(
                chain.tip_state.get().next_block_height == next_block_height,
                WorkerError::UnexpectedBlockHeight {
                    expected_block_height: next_block_height,
                    found_block_height: chain.tip_state.get().next_block_height
                }
            );
        }
        if query.request_pending_messages {
            let mut messages = Vec::new();
            let origins = chain.inboxes.indices().await?;
            let inboxes = chain.inboxes.try_load_entries(&origins).await?;
            let action = if *chain.execution_state.system.closed.get() {
                MessageAction::Reject
            } else {
                MessageAction::Accept
            };
            for (origin, inbox) in origins.into_iter().zip(inboxes) {
                for event in inbox.added_events.elements().await? {
                    messages.push(IncomingMessage {
                        origin: origin.clone(),
                        event: event.clone(),
                        action,
                    });
                }
            }

            info.requested_pending_messages = messages;
        }
        if let Some(range) = query.request_sent_certificate_hashes_in_range {
            let start: usize = range.start.try_into()?;
            let end = match range.limit {
                None => chain.confirmed_log.count(),
                Some(limit) => start
                    .checked_add(usize::try_from(limit).map_err(|_| ArithmeticError::Overflow)?)
                    .ok_or(ArithmeticError::Overflow)?
                    .min(chain.confirmed_log.count()),
            };
            let keys = chain.confirmed_log.read(start..end).await?;
            info.requested_sent_certificate_hashes = keys;
        }
        if let Some(start) = query.request_received_log_excluding_first_nth {
            let start = usize::try_from(start).map_err(|_| ArithmeticError::Overflow)?;
            info.requested_received_log = chain.received_log.read(start..).await?;
        }
        if query.request_manager_values {
            info.manager.add_values(chain.manager.get());
        }
        Ok(ChainInfoResponse::new(info, self.0.config.key_pair()))
    }
}

impl<StorageClient> Drop for ChainWorkerStateWithTemporaryChanges<'_, StorageClient>
where
    StorageClient: Storage + Clone + Send + Sync + 'static,
    ViewError: From<StorageClient::StoreError>,
{
    fn drop(&mut self) {
        self.0.chain.rollback();
    }
}
