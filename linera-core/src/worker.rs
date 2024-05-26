// Copyright (c) Facebook, Inc. and its affiliates.
// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    borrow::Cow,
    collections::{hash_map, BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use futures::{future, FutureExt};
use linera_base::{
    crypto::{CryptoHash, KeyPair},
    data_types::{ArithmeticError, BlockHeight, HashedBlob, Round},
    doc_scalar, ensure,
    identifiers::{BlobId, ChainId, Owner},
};
use linera_chain::{
    data_types::{
        Block, BlockExecutionOutcome, BlockProposal, Certificate, CertificateValue, ExecutedBlock,
        HashedCertificateValue, LiteCertificate, Medium, MessageBundle, Origin, OutgoingMessage,
        Target,
    },
    manager::{self},
    ChainError, ChainStateView,
};
use linera_execution::{
    committee::Epoch, BytecodeLocation, Query, Response, UserApplicationDescription,
    UserApplicationId,
};
use linera_storage::Storage;
use linera_views::{
    log_view::LogView,
    views::{RootView, ViewError},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{oneshot, Mutex};
use tracing::{error, instrument, trace, warn};
#[cfg(with_testing)]
use {
    linera_base::identifiers::{BytecodeId, Destination, MessageId},
    linera_chain::data_types::{ChannelFullName, IncomingMessage, MessageAction},
};
#[cfg(with_metrics)]
use {
    linera_base::{prometheus_util, sync::Lazy},
    prometheus::{HistogramVec, IntCounterVec},
};

use crate::{
    chain_worker::{ChainWorkerConfig, ChainWorkerState},
    data_types::{ChainInfoQuery, ChainInfoResponse, CrossChainRequest},
    value_cache::ValueCache,
};

#[cfg(test)]
#[path = "unit_tests/worker_tests.rs"]
mod worker_tests;

#[cfg(with_metrics)]
static NUM_ROUNDS_IN_CERTIFICATE: Lazy<HistogramVec> = Lazy::new(|| {
    prometheus_util::register_histogram_vec(
        "num_rounds_in_certificate",
        "Number of rounds in certificate",
        &["certificate_value", "round_type"],
        Some(vec![
            0.5, 1.0, 2.0, 3.0, 4.0, 6.0, 8.0, 10.0, 15.0, 25.0, 50.0,
        ]),
    )
    .expect("Counter creation should not fail")
});

#[cfg(with_metrics)]
static NUM_ROUNDS_IN_BLOCK_PROPOSAL: Lazy<HistogramVec> = Lazy::new(|| {
    prometheus_util::register_histogram_vec(
        "num_rounds_in_block_proposal",
        "Number of rounds in block proposal",
        &["round_type"],
        Some(vec![
            0.5, 1.0, 2.0, 3.0, 4.0, 6.0, 8.0, 10.0, 15.0, 25.0, 50.0,
        ]),
    )
    .expect("Counter creation should not fail")
});

#[cfg(with_metrics)]
static TRANSACTION_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    prometheus_util::register_int_counter_vec("transaction_count", "Transaction count", &[])
        .expect("Counter creation should not fail")
});

#[cfg(with_metrics)]
static NUM_BLOCKS: Lazy<IntCounterVec> = Lazy::new(|| {
    prometheus_util::register_int_counter_vec("num_blocks", "Number of blocks added to chains", &[])
        .expect("Counter creation should not fail")
});

/// Interface provided by each physical shard (aka "worker") of a validator or a local node.
/// * All commands return either the current chain info or an error.
/// * Repeating commands produces no changes and returns no error.
/// * Some handlers may return cross-chain requests, that is, messages
///   to be communicated to other workers of the same validator.
#[cfg_attr(not(web), async_trait)]
#[cfg_attr(web, async_trait(?Send))]
pub trait ValidatorWorker {
    /// Proposes a new block. In case of success, the chain info contains a vote on the new
    /// block.
    async fn handle_block_proposal(
        &mut self,
        proposal: BlockProposal,
    ) -> Result<(ChainInfoResponse, NetworkActions), WorkerError>;

    /// Processes a certificate, e.g. to extend a chain with a confirmed block.
    async fn handle_lite_certificate<'a>(
        &mut self,
        certificate: LiteCertificate<'a>,
        notify_message_delivery: Option<oneshot::Sender<()>>,
    ) -> Result<(ChainInfoResponse, NetworkActions), WorkerError>;

    /// Processes a certificate, e.g. to extend a chain with a confirmed block.
    async fn handle_certificate(
        &mut self,
        certificate: Certificate,
        hashed_certificate_values: Vec<HashedCertificateValue>,
        hashed_blobs: Vec<HashedBlob>,
        notify_message_delivery: Option<oneshot::Sender<()>>,
    ) -> Result<(ChainInfoResponse, NetworkActions), WorkerError>;

    /// Handles information queries on chains.
    async fn handle_chain_info_query(
        &self,
        query: ChainInfoQuery,
    ) -> Result<(ChainInfoResponse, NetworkActions), WorkerError>;

    /// Handles a (trusted!) cross-chain request.
    async fn handle_cross_chain_request(
        &mut self,
        request: CrossChainRequest,
    ) -> Result<NetworkActions, WorkerError>;
}

/// Instruct the networking layer to send cross-chain requests and/or push notifications.
#[derive(Default, Debug)]
pub struct NetworkActions {
    /// The cross-chain requests
    pub cross_chain_requests: Vec<CrossChainRequest>,
    /// The push notifications.
    pub notifications: Vec<Notification>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// Notification that a chain has a new certified block or a new message.
pub struct Notification {
    pub chain_id: ChainId,
    pub reason: Reason,
}

doc_scalar!(
    Notification,
    "Notify that a chain has a new certified block or a new message"
);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
/// Reason for the notification.
pub enum Reason {
    NewBlock {
        height: BlockHeight,
        hash: CryptoHash,
    },
    NewIncomingMessage {
        origin: Origin,
        height: BlockHeight,
    },
    NewRound {
        height: BlockHeight,
        round: Round,
    },
}

/// Error type for [`ValidatorWorker`].
#[derive(Debug, Error)]
pub enum WorkerError {
    #[error(transparent)]
    CryptoError(#[from] linera_base::crypto::CryptoError),

    #[error(transparent)]
    ArithmeticError(#[from] ArithmeticError),

    #[error(transparent)]
    ViewError(#[from] linera_views::views::ViewError),

    #[error(transparent)]
    ChainError(#[from] Box<linera_chain::ChainError>),

    // Chain access control
    #[error("Block was not signed by an authorized owner")]
    InvalidOwner,

    #[error("Operations in the block are not authenticated by the proper signer")]
    InvalidSigner(Owner),

    // Chaining
    #[error(
        "Was expecting block height {expected_block_height} but found {found_block_height} instead"
    )]
    UnexpectedBlockHeight {
        expected_block_height: BlockHeight,
        found_block_height: BlockHeight,
    },
    #[error("Cannot confirm a block before its predecessors: {current_block_height:?}")]
    MissingEarlierBlocks { current_block_height: BlockHeight },
    #[error("Unexpected epoch {epoch:}: chain {chain_id:} is at {chain_epoch:}")]
    InvalidEpoch {
        chain_id: ChainId,
        chain_epoch: Epoch,
        epoch: Epoch,
    },

    // Other server-side errors
    #[error("Invalid cross-chain request")]
    InvalidCrossChainRequest,
    #[error("The block does contain the hash that we expected for the previous block")]
    InvalidBlockChaining,
    #[error("The given state hash is not what we computed after executing the block")]
    IncorrectStateHash,
    #[error(
        "
        The given messages are not what we computed after executing the block.\n\
        Computed: {computed:#?}\n\
        Submitted: {submitted:#?}\n
    "
    )]
    IncorrectMessages {
        computed: Vec<OutgoingMessage>,
        submitted: Vec<OutgoingMessage>,
    },
    #[error("The given message counts are not what we computed after executing the block")]
    IncorrectMessageCounts,
    #[error("The timestamp of a Tick operation is in the future.")]
    InvalidTimestamp,
    #[error("We don't have the value for the certificate.")]
    MissingCertificateValue,
    #[error("The hash certificate doesn't match its value.")]
    InvalidLiteCertificate,
    #[error("An additional value was provided that is not required: {value_hash}.")]
    UnneededValue { value_hash: CryptoHash },
    #[error("An additional blob was provided that is not required: {blob_id}.")]
    UnneededBlob { blob_id: BlobId },
    #[error("The following values containing application bytecode are missing: {0:?}.")]
    ApplicationBytecodesNotFound(Vec<BytecodeLocation>),
    #[error("The certificate in the block proposal is not a ValidatedBlock")]
    MissingExecutedBlockInProposal,
    #[error("Fast blocks cannot query oracles")]
    FastBlockUsingOracles,
    #[error("The following blobs are missing: {0:?}.")]
    BlobsNotFound(Vec<BlobId>),
    #[error("The following values containing application bytecode are missing: {0:?} and the following blobs are missing: {1:?}.")]
    ApplicationBytecodesAndBlobsNotFound(Vec<BytecodeLocation>, Vec<BlobId>),
}

impl From<linera_chain::ChainError> for WorkerError {
    fn from(chain_error: linera_chain::ChainError) -> Self {
        WorkerError::ChainError(Box::new(chain_error))
    }
}

/// State of a worker in a validator or a local node.
#[derive(Clone)]
pub struct WorkerState<StorageClient> {
    /// A name used for logging
    nickname: String,
    /// Access to local persistent storage.
    storage: StorageClient,
    /// Configuration options for the [`ChainWorker`]s.
    chain_worker_config: ChainWorkerConfig,
    /// Cached hashed certificate values by hash.
    recent_hashed_certificate_values: Arc<ValueCache<CryptoHash, HashedCertificateValue>>,
    /// Cached hashed blobs by `BlobId`.
    recent_hashed_blobs: Arc<ValueCache<BlobId, HashedBlob>>,
    /// One-shot channels to notify callers when messages of a particular chain have been
    /// delivered.
    delivery_notifiers: Arc<Mutex<DeliveryNotifiers>>,
}

pub(crate) type DeliveryNotifiers =
    HashMap<ChainId, BTreeMap<BlockHeight, Vec<oneshot::Sender<()>>>>;

impl<StorageClient> WorkerState<StorageClient> {
    pub fn new(nickname: String, key_pair: Option<KeyPair>, storage: StorageClient) -> Self {
        WorkerState {
            nickname,
            storage,
            chain_worker_config: ChainWorkerConfig::default().with_key_pair(key_pair),
            recent_hashed_certificate_values: Arc::new(ValueCache::default()),
            recent_hashed_blobs: Arc::new(ValueCache::default()),
            delivery_notifiers: Arc::default(),
        }
    }

    pub fn new_for_client(
        nickname: String,
        storage: StorageClient,
        recent_hashed_certificate_values: Arc<ValueCache<CryptoHash, HashedCertificateValue>>,
        recent_hashed_blobs: Arc<ValueCache<BlobId, HashedBlob>>,
        delivery_notifiers: Arc<Mutex<DeliveryNotifiers>>,
    ) -> Self {
        WorkerState {
            nickname,
            storage,
            chain_worker_config: ChainWorkerConfig::default(),
            recent_hashed_certificate_values,
            recent_hashed_blobs,
            delivery_notifiers,
        }
    }

    pub fn with_allow_inactive_chains(mut self, value: bool) -> Self {
        self.chain_worker_config.allow_inactive_chains = value;
        self
    }

    pub fn with_allow_messages_from_deprecated_epochs(mut self, value: bool) -> Self {
        self.chain_worker_config
            .allow_messages_from_deprecated_epochs = value;
        self
    }

    /// Returns an instance with the specified grace period, in microseconds.
    ///
    /// Blocks with a timestamp this far in the future will still be accepted, but the validator
    /// will wait until that timestamp before voting.
    pub fn with_grace_period(mut self, grace_period: Duration) -> Self {
        self.chain_worker_config.grace_period = grace_period;
        self
    }

    pub fn nickname(&self) -> &str {
        &self.nickname
    }

    pub fn recent_hashed_blobs(&self) -> Arc<ValueCache<BlobId, HashedBlob>> {
        self.recent_hashed_blobs.clone()
    }

    /// Returns the storage client so that it can be manipulated or queried.
    #[cfg(not(feature = "test"))]
    pub(crate) fn storage_client(&self) -> &StorageClient {
        &self.storage
    }

    /// Returns the storage client so that it can be manipulated or queried by tests in other
    /// crates.
    #[cfg(feature = "test")]
    pub fn storage_client(&self) -> &StorageClient {
        &self.storage
    }

    #[cfg(test)]
    pub(crate) fn with_key_pair(mut self, key_pair: Option<Arc<KeyPair>>) -> Self {
        self.chain_worker_config.key_pair = key_pair;
        self
    }

    pub(crate) async fn full_certificate(
        &mut self,
        certificate: LiteCertificate<'_>,
    ) -> Result<Certificate, WorkerError> {
        self.recent_hashed_certificate_values
            .full_certificate(certificate)
            .await
    }

    pub(crate) async fn recent_hashed_certificate_value(
        &mut self,
        hash: &CryptoHash,
    ) -> Option<HashedCertificateValue> {
        self.recent_hashed_certificate_values.get(hash).await
    }

    pub(crate) async fn recent_blob(&mut self, blob_id: &BlobId) -> Option<HashedBlob> {
        self.recent_hashed_blobs.get(blob_id).await
    }
}

impl<StorageClient> WorkerState<StorageClient>
where
    StorageClient: Storage + Clone + Send + Sync + 'static,
    ViewError: From<StorageClient::ContextError>,
{
    // NOTE: This only works for non-sharded workers!
    #[cfg(with_testing)]
    pub async fn fully_handle_certificate(
        &mut self,
        certificate: Certificate,
        hashed_certificate_values: Vec<HashedCertificateValue>,
        hashed_blobs: Vec<HashedBlob>,
    ) -> Result<ChainInfoResponse, WorkerError> {
        self.fully_handle_certificate_with_notifications(
            certificate,
            hashed_certificate_values,
            hashed_blobs,
            None,
        )
        .await
    }

    #[inline]
    pub(crate) async fn fully_handle_certificate_with_notifications(
        &mut self,
        certificate: Certificate,
        hashed_certificate_values: Vec<HashedCertificateValue>,
        hashed_blobs: Vec<HashedBlob>,
        mut notifications: Option<&mut Vec<Notification>>,
    ) -> Result<ChainInfoResponse, WorkerError> {
        let (response, actions) = self
            .handle_certificate(certificate, hashed_certificate_values, hashed_blobs, None)
            .await?;
        if let Some(notifications) = notifications.as_mut() {
            notifications.extend(actions.notifications);
        }
        let mut requests = VecDeque::from(actions.cross_chain_requests);
        while let Some(request) = requests.pop_front() {
            let actions = self.handle_cross_chain_request(request).await?;
            requests.extend(actions.cross_chain_requests);
            if let Some(notifications) = notifications.as_mut() {
                notifications.extend(actions.notifications);
            }
        }
        Ok(response)
    }

    /// Tries to execute a block proposal without any verification other than block execution.
    pub async fn stage_block_execution(
        &mut self,
        block: Block,
    ) -> Result<(ExecutedBlock, ChainInfoResponse), WorkerError> {
        self.create_chain_worker(block.chain_id)
            .await?
            .stage_block_execution(block)
            .await
    }

    // Schedule a notification when cross-chain messages are delivered up to the given height.
    async fn register_delivery_notifier(
        &mut self,
        chain_id: ChainId,
        height: BlockHeight,
        actions: &NetworkActions,
        notify_when_messages_are_delivered: Option<oneshot::Sender<()>>,
    ) {
        if let Some(notifier) = notify_when_messages_are_delivered {
            if actions
                .cross_chain_requests
                .iter()
                .any(|request| request.has_messages_lower_or_equal_than(height))
            {
                self.delivery_notifiers
                    .lock()
                    .await
                    .entry(chain_id)
                    .or_default()
                    .entry(height)
                    .or_default()
                    .push(notifier);
            } else {
                // No need to wait. Also, cross-chain requests may not trigger the
                // notifier later, even if we register it.
                if let Err(()) = notifier.send(()) {
                    warn!("Failed to notify message delivery to caller");
                }
            }
        }
    }

    /// Executes a [`Query`] for an application's state on a specific chain.
    pub async fn query_application(
        &mut self,
        chain_id: ChainId,
        query: Query,
    ) -> Result<Response, WorkerError> {
        self.create_chain_worker(chain_id)
            .await?
            .query_application(query)
            .await
    }

    #[cfg(with_testing)]
    pub async fn read_bytecode_location(
        &mut self,
        chain_id: ChainId,
        bytecode_id: BytecodeId,
    ) -> Result<Option<BytecodeLocation>, WorkerError> {
        self.create_chain_worker(chain_id)
            .await?
            .read_bytecode_location(bytecode_id)
            .await
    }

    pub async fn describe_application(
        &mut self,
        chain_id: ChainId,
        application_id: UserApplicationId,
    ) -> Result<UserApplicationDescription, WorkerError> {
        self.create_chain_worker(chain_id)
            .await?
            .describe_application(application_id)
            .await
    }

    /// Creates an `UpdateRecipient` request that informs the `recipient` about new
    /// cross-chain messages from `sender`.
    async fn create_cross_chain_request(
        &self,
        confirmed_log: &LogView<StorageClient::Context, CryptoHash>,
        height_map: Vec<(Medium, Vec<BlockHeight>)>,
        sender: ChainId,
        recipient: ChainId,
    ) -> Result<CrossChainRequest, WorkerError> {
        // Load all the certificates we will need, regardless of the medium.
        let heights =
            BTreeSet::from_iter(height_map.iter().flat_map(|(_, heights)| heights).copied());
        let heights_usize = heights
            .iter()
            .copied()
            .map(usize::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let hashes = confirmed_log
            .multi_get(heights_usize.clone())
            .await?
            .into_iter()
            .zip(heights_usize)
            .map(|(maybe_hash, height)| {
                maybe_hash.ok_or_else(|| ViewError::not_found("confirmed log entry", height))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let certificates = self.storage.read_certificates(hashes).await?;
        let certificates = heights
            .into_iter()
            .zip(certificates)
            .collect::<HashMap<_, _>>();
        // For each medium, select the relevant messages.
        let bundle_vecs = height_map
            .into_iter()
            .map(|(medium, heights)| {
                let bundles = heights
                    .into_iter()
                    .map(|height| {
                        certificates
                            .get(&height)?
                            .message_bundle_for(&medium, recipient)
                    })
                    .collect::<Option<_>>()?;
                Some((medium, bundles))
            })
            .collect::<Option<_>>()
            .ok_or_else(|| ChainError::InternalError("missing certificates".to_string()))?;
        Ok(CrossChainRequest::UpdateRecipient {
            sender,
            recipient,
            bundle_vecs,
        })
    }

    /// Loads pending cross-chain requests.
    async fn create_network_actions(
        &self,
        chain: &ChainStateView<StorageClient::Context>,
    ) -> Result<NetworkActions, WorkerError> {
        let mut heights_by_recipient: BTreeMap<_, BTreeMap<_, _>> = Default::default();
        let targets = chain.outboxes.indices().await?;
        let outboxes = chain.outboxes.try_load_entries(&targets).await?;
        for (target, outbox) in targets.into_iter().zip(outboxes) {
            let heights = outbox.queue.elements().await?;
            heights_by_recipient
                .entry(target.recipient)
                .or_default()
                .insert(target.medium, heights);
        }
        let mut actions = NetworkActions::default();
        let chain_id = chain.chain_id();
        for (recipient, height_map) in heights_by_recipient {
            let request = self
                .create_cross_chain_request(
                    &chain.confirmed_log,
                    height_map.into_iter().collect(),
                    chain_id,
                    recipient,
                )
                .await?;
            actions.cross_chain_requests.push(request);
        }
        Ok(actions)
    }

    /// Processes a confirmed block (aka a commit).
    async fn process_confirmed_block(
        &mut self,
        certificate: Certificate,
        hashed_certificate_values: &[HashedCertificateValue],
        hashed_blobs: &[HashedBlob],
        notify_when_messages_are_delivered: Option<oneshot::Sender<()>>,
    ) -> Result<(ChainInfoResponse, NetworkActions), WorkerError> {
        let CertificateValue::ConfirmedBlock { executed_block, .. } = certificate.value() else {
            panic!("Expecting a confirmation certificate");
        };
        let block = &executed_block.block;
        let BlockExecutionOutcome {
            messages,
            message_counts,
            state_hash,
            oracle_records,
        } = &executed_block.outcome;
        let mut chain = self.storage.load_chain(block.chain_id).await?;
        // Check that the chain is active and ready for this confirmation.
        let tip = chain.tip_state.get().clone();
        if tip.next_block_height < block.height {
            return Err(WorkerError::MissingEarlierBlocks {
                current_block_height: tip.next_block_height,
            });
        }
        if tip.next_block_height > block.height {
            // Block was already confirmed.
            let info = ChainInfoResponse::new(&chain, self.key_pair());
            let actions = self.create_network_actions(&chain).await?;
            self.register_delivery_notifier(
                block.chain_id,
                block.height,
                &actions,
                notify_when_messages_are_delivered,
            )
            .await;
            return Ok((info, actions));
        }
        if tip.is_first_block() && !chain.is_active() {
            let local_time = self.storage.clock().current_time();
            for message in &block.incoming_messages {
                if chain
                    .execute_init_message(
                        message.id(),
                        &message.event.message,
                        message.event.timestamp,
                        local_time,
                    )
                    .await?
                {
                    break;
                }
            }
        }
        chain.ensure_is_active()?;
        // Verify the certificate.
        let (epoch, committee) = chain
            .execution_state
            .system
            .current_committee()
            .expect("chain is active");
        Self::check_block_epoch(epoch, block)?;
        certificate.check(committee)?;
        // This should always be true for valid certificates.
        ensure!(
            tip.block_hash == block.previous_block_hash,
            WorkerError::InvalidBlockChaining
        );
        let pending_blobs = &chain.manager.get().pending_blobs;
        // Verify that all required bytecode hashed certificate values and blobs are available, and no unrelated ones provided.
        self.check_no_missing_blobs(
            block,
            hashed_certificate_values,
            hashed_blobs,
            pending_blobs,
        )
        .await?;
        // Persist certificate and hashed certificate values.
        self.recent_hashed_certificate_values
            .insert_all(hashed_certificate_values.iter().map(Cow::Borrowed))
            .await;
        for hashed_blob in hashed_blobs {
            self.cache_recent_blob(Cow::Borrowed(hashed_blob)).await;
        }

        let blobs_in_block = self.get_blobs(block.blob_ids(), pending_blobs).await?;
        let (result_hashed_certificate_value, result_blobs, result_certificate) = tokio::join!(
            self.storage
                .write_hashed_certificate_values(hashed_certificate_values),
            self.storage.write_hashed_blobs(&blobs_in_block),
            self.storage.write_certificate(&certificate)
        );
        result_hashed_certificate_value?;
        result_blobs?;
        result_certificate?;
        // Execute the block and update inboxes.
        chain.remove_events_from_inboxes(block).await?;
        let local_time = self.storage.clock().current_time();
        let verified_outcome = chain
            .execute_block(block, local_time, Some(oracle_records.clone()))
            .await?;
        // We should always agree on the messages and state hash.
        ensure!(
            *messages == verified_outcome.messages,
            WorkerError::IncorrectMessages {
                computed: verified_outcome.messages,
                submitted: messages.clone(),
            }
        );
        ensure!(
            *message_counts == verified_outcome.message_counts,
            WorkerError::IncorrectMessageCounts
        );
        ensure!(
            *state_hash == verified_outcome.state_hash,
            WorkerError::IncorrectStateHash
        );
        // Advance to next block height.
        let tip = chain.tip_state.get_mut();
        tip.block_hash = Some(certificate.hash());
        tip.next_block_height.try_add_assign_one()?;
        tip.num_incoming_messages += block.incoming_messages.len() as u32;
        tip.num_operations += block.operations.len() as u32;
        tip.num_outgoing_messages += messages.len() as u32;
        chain.confirmed_log.push(certificate.hash());
        let info = ChainInfoResponse::new(&chain, self.key_pair());
        let mut actions = self.create_network_actions(&chain).await?;
        actions.notifications.push(Notification {
            chain_id: block.chain_id,
            reason: Reason::NewBlock {
                height: block.height,
                hash: certificate.value.hash(),
            },
        });
        // Persist chain.
        chain.save().await?;
        // Notify the caller when cross-chain messages are delivered.
        self.register_delivery_notifier(
            block.chain_id,
            block.height,
            &actions,
            notify_when_messages_are_delivered,
        )
        .await;
        self.recent_hashed_certificate_values
            .insert(Cow::Owned(certificate.value))
            .await;

        #[cfg(with_metrics)]
        NUM_BLOCKS.with_label_values(&[]).inc();

        Ok((info, actions))
    }

    /// Returns an error if the block requires bytecode or a blob we don't have, or if unrelated bytecode
    /// hashed certificate values or blobs were provided.
    async fn check_no_missing_blobs(
        &self,
        block: &Block,
        hashed_certificate_values: &[HashedCertificateValue],
        hashed_blobs: &[HashedBlob],
        pending_blobs: &BTreeMap<BlobId, HashedBlob>,
    ) -> Result<(), WorkerError> {
        let missing_bytecodes = self
            .get_missing_bytecodes(block, hashed_certificate_values)
            .await?;
        let missing_blobs = self
            .get_missing_blobs(block, hashed_blobs, pending_blobs)
            .await?;

        if missing_bytecodes.is_empty() {
            if missing_blobs.is_empty() {
                Ok(())
            } else {
                Err(WorkerError::BlobsNotFound(missing_blobs))
            }
        } else if missing_blobs.is_empty() {
            Err(WorkerError::ApplicationBytecodesNotFound(missing_bytecodes))
        } else {
            Err(WorkerError::ApplicationBytecodesAndBlobsNotFound(
                missing_bytecodes,
                missing_blobs,
            ))
        }
    }

    /// Returns the blobs required by the block that we don't have, or an error if unrelated blobs were provided.
    async fn get_missing_blobs(
        &self,
        block: &Block,
        hashed_blobs: &[HashedBlob],
        pending_blobs: &BTreeMap<BlobId, HashedBlob>,
    ) -> Result<Vec<BlobId>, WorkerError> {
        let mut required_blob_ids = block.blob_ids();
        // Find all certificates containing blobs used when executing this block.
        for hashed_blob in hashed_blobs {
            let blob_id = hashed_blob.id();
            ensure!(
                required_blob_ids.remove(&blob_id),
                WorkerError::UnneededBlob { blob_id }
            );
        }

        Ok(self
            .recent_hashed_blobs
            .subtract_cached_items_from::<_, Vec<_>>(required_blob_ids, |id| id)
            .await
            .into_iter()
            .filter(|blob_id| !pending_blobs.contains_key(blob_id))
            .collect::<Vec<_>>())
    }

    async fn get_blobs(
        &self,
        blob_ids: HashSet<BlobId>,
        pending_blobs: &BTreeMap<BlobId, HashedBlob>,
    ) -> Result<Vec<HashedBlob>, WorkerError> {
        let (found_blobs, not_found_blobs): (HashMap<BlobId, HashedBlob>, HashSet<BlobId>) =
            self.recent_hashed_blobs.try_get_many(blob_ids).await;

        let mut blobs = found_blobs.into_values().collect::<Vec<_>>();
        for blob_id in not_found_blobs {
            if let Some(blob) = pending_blobs.get(&blob_id) {
                blobs.push(blob.clone());
            }
        }

        Ok(blobs)
    }

    /// Returns an error if the block requires bytecode we don't have, or if unrelated bytecode
    /// hashed certificate values were provided.
    async fn get_missing_bytecodes(
        &self,
        block: &Block,
        hashed_certificate_values: &[HashedCertificateValue],
    ) -> Result<Vec<BytecodeLocation>, WorkerError> {
        // Find all certificates containing bytecode used when executing this block.
        let mut required_locations_left: HashMap<_, _> = block
            .bytecode_locations()
            .into_keys()
            .map(|bytecode_location| (bytecode_location.certificate_hash, bytecode_location))
            .collect();
        for value in hashed_certificate_values {
            let value_hash = value.hash();
            ensure!(
                required_locations_left.remove(&value_hash).is_some(),
                WorkerError::UnneededValue { value_hash }
            );
        }
        let tasks = self
            .recent_hashed_certificate_values
            .subtract_cached_items_from::<_, Vec<_>>(
                required_locations_left.into_values(),
                |location| &location.certificate_hash,
            )
            .await
            .into_iter()
            .map(|location| {
                self.storage
                    .contains_hashed_certificate_value(location.certificate_hash)
                    .map(move |result| (location, result))
            })
            .collect::<Vec<_>>();
        let mut missing_locations = vec![];
        for (location, result) in future::join_all(tasks).await {
            match result {
                Ok(true) => {}
                Ok(false) => missing_locations.push(location),
                Err(err) => Err(err)?,
            }
        }

        Ok(missing_locations.into_iter().collect())
    }

    /// Processes a validated block issued from a multi-owner chain.
    async fn process_validated_block(
        &mut self,
        certificate: Certificate,
    ) -> Result<(ChainInfoResponse, NetworkActions, bool), WorkerError> {
        let block = match certificate.value() {
            CertificateValue::ValidatedBlock {
                executed_block: ExecutedBlock { block, .. },
            } => block,
            _ => panic!("Expecting a validation certificate"),
        };
        let chain_id = block.chain_id;
        let height = block.height;
        // Check that the chain is active and ready for this confirmation.
        // Verify the certificate. Returns a catch-all error to make client code more robust.
        let mut chain = self.storage.load_active_chain(chain_id).await?;
        let (epoch, committee) = chain
            .execution_state
            .system
            .current_committee()
            .expect("chain is active");
        Self::check_block_epoch(epoch, block)?;
        certificate.check(committee)?;
        let mut actions = NetworkActions::default();
        let already_validated_block = chain.tip_state.get().already_validated_block(height)?;
        let should_skip_validated_block = || {
            chain
                .manager
                .get()
                .check_validated_block(&certificate)
                .map(|outcome| outcome == manager::Outcome::Skip)
        };
        if already_validated_block || should_skip_validated_block()? {
            // If we just processed the same pending block, return the chain info unchanged.
            return Ok((
                ChainInfoResponse::new(&chain, self.key_pair()),
                actions,
                true,
            ));
        }
        self.recent_hashed_certificate_values
            .insert(Cow::Borrowed(&certificate.value))
            .await;
        let old_round = chain.manager.get().current_round;
        chain.manager.get_mut().create_final_vote(
            certificate,
            self.key_pair(),
            self.storage.clock().current_time(),
        );
        let info = ChainInfoResponse::new(&chain, self.key_pair());
        chain.save().await?;
        let round = chain.manager.get().current_round;
        if round > old_round {
            actions.notifications.push(Notification {
                chain_id,
                reason: Reason::NewRound { height, round },
            })
        }
        Ok((info, actions, false))
    }

    /// Processes a leader timeout issued from a multi-owner chain.
    async fn process_timeout(
        &mut self,
        certificate: Certificate,
    ) -> Result<(ChainInfoResponse, NetworkActions), WorkerError> {
        let CertificateValue::Timeout { chain_id, .. } = certificate.value() else {
            panic!("Expecting a leader timeout certificate");
        };
        self.create_chain_worker(*chain_id)
            .await?
            .process_timeout(certificate)
            .await
    }

    async fn process_cross_chain_update(
        &mut self,
        origin: Origin,
        recipient: ChainId,
        bundles: Vec<MessageBundle>,
    ) -> Result<Option<BlockHeight>, WorkerError> {
        self.create_chain_worker(recipient)
            .await?
            .process_cross_chain_update(origin, bundles)
            .await
    }

    /// Inserts a [`HashedCertificateValue`] into the worker's cache.
    pub(crate) async fn cache_recent_hashed_certificate_value<'a>(
        &mut self,
        value: Cow<'a, HashedCertificateValue>,
    ) -> bool {
        self.recent_hashed_certificate_values.insert(value).await
    }

    /// Inserts a [`HashedBlob`] into the worker's cache.
    pub async fn cache_recent_blob<'a>(&mut self, hashed_blob: Cow<'a, HashedBlob>) -> bool {
        self.recent_hashed_blobs.insert(hashed_blob).await
    }

    /// Returns a stored [`Certificate`] for a chain's block.
    #[cfg(with_testing)]
    pub async fn read_certificate(
        &self,
        chain_id: ChainId,
        height: BlockHeight,
    ) -> Result<Option<Certificate>, WorkerError> {
        self.create_chain_worker(chain_id)
            .await?
            .read_certificate(height)
            .await
    }

    /// Returns an [`IncomingMessage`] that's awaiting to be received by the chain specified by
    /// `chain_id`.
    #[cfg(with_testing)]
    pub async fn find_incoming_message(
        &self,
        chain_id: ChainId,
        message_id: MessageId,
    ) -> Result<Option<IncomingMessage>, WorkerError> {
        let sender = message_id.chain_id;
        let index = usize::try_from(message_id.index).map_err(|_| ArithmeticError::Overflow)?;
        let Some(certificate) = self.read_certificate(sender, message_id.height).await? else {
            return Ok(None);
        };
        let Some(messages) = certificate.value().messages() else {
            return Ok(None);
        };
        let Some(outgoing_message) = messages.get(index).cloned() else {
            return Ok(None);
        };

        let medium = match outgoing_message.destination {
            Destination::Recipient(_) => Medium::Direct,
            Destination::Subscribers(name) => {
                let application_id = outgoing_message.message.application_id();
                Medium::Channel(ChannelFullName {
                    application_id,
                    name,
                })
            }
        };
        let origin = Origin { sender, medium };

        let Some(event) = self
            .create_chain_worker(chain_id)
            .await?
            .find_event_in_inbox(
                origin.clone(),
                certificate.hash(),
                message_id.height,
                message_id.index,
            )
            .await?
        else {
            return Ok(None);
        };

        assert_eq!(event.message, outgoing_message.message);

        Ok(Some(IncomingMessage {
            origin,
            event,
            action: MessageAction::Accept,
        }))
    }

    /// Returns an error if the block is not at the expected epoch.
    fn check_block_epoch(chain_epoch: Epoch, block: &Block) -> Result<(), WorkerError> {
        ensure!(
            block.epoch == chain_epoch,
            WorkerError::InvalidEpoch {
                chain_id: block.chain_id,
                epoch: block.epoch,
                chain_epoch
            }
        );
        Ok(())
    }

    /// Creates a [`ChainWorkerState`] instance for a specific chain.
    async fn create_chain_worker(
        &self,
        chain_id: ChainId,
    ) -> Result<ChainWorkerState<StorageClient>, WorkerError> {
        ChainWorkerState::new(
            self.chain_worker_config.clone(),
            self.storage.clone(),
            self.recent_hashed_certificate_values.clone(),
            self.recent_hashed_blobs.clone(),
            chain_id,
        )
        .await
    }
}

impl<StorageClient> WorkerState<StorageClient> {
    /// Gets a reference to the [`KeyPair`], if available.
    fn key_pair(&self) -> Option<&KeyPair> {
        self.chain_worker_config.key_pair()
    }
}

#[cfg_attr(not(web), async_trait)]
#[cfg_attr(web, async_trait(?Send))]
impl<StorageClient> ValidatorWorker for WorkerState<StorageClient>
where
    StorageClient: Storage + Clone + Send + Sync + 'static,
    ViewError: From<StorageClient::ContextError>,
{
    #[instrument(skip_all, fields(
        nick = self.nickname,
        chain_id = format!("{:.8}", proposal.content.block.chain_id),
        height = %proposal.content.block.height,
    ))]
    async fn handle_block_proposal(
        &mut self,
        proposal: BlockProposal,
    ) -> Result<(ChainInfoResponse, NetworkActions), WorkerError> {
        trace!("{} <-- {:?}", self.nickname, proposal);
        #[cfg(with_metrics)]
        let round = proposal.content.round;
        let response = self
            .create_chain_worker(proposal.content.block.chain_id)
            .await?
            .handle_block_proposal(proposal)
            .await?;
        #[cfg(with_metrics)]
        NUM_ROUNDS_IN_BLOCK_PROPOSAL
            .with_label_values(&[round.type_name()])
            .observe(round.number() as f64);
        Ok(response)
    }

    // Other fields will be included in handle_certificate's span.
    #[instrument(skip_all, fields(hash = %certificate.value.value_hash))]
    /// Processes a certificate, e.g. to extend a chain with a confirmed block.
    async fn handle_lite_certificate<'a>(
        &mut self,
        certificate: LiteCertificate<'a>,
        notify_when_messages_are_delivered: Option<oneshot::Sender<()>>,
    ) -> Result<(ChainInfoResponse, NetworkActions), WorkerError> {
        let full_cert = self.full_certificate(certificate).await?;
        self.handle_certificate(
            full_cert,
            vec![],
            vec![],
            notify_when_messages_are_delivered,
        )
        .await
    }

    /// Processes a certificate.
    #[instrument(skip_all, fields(
        nick = self.nickname,
        chain_id = format!("{:.8}", certificate.value().chain_id()),
        height = %certificate.value().height(),
    ))]
    async fn handle_certificate(
        &mut self,
        certificate: Certificate,
        hashed_certificate_values: Vec<HashedCertificateValue>,
        hashed_blobs: Vec<HashedBlob>,
        notify_when_messages_are_delivered: Option<oneshot::Sender<()>>,
    ) -> Result<(ChainInfoResponse, NetworkActions), WorkerError> {
        trace!("{} <-- {:?}", self.nickname, certificate);
        ensure!(
            certificate.value().is_confirmed() || hashed_certificate_values.is_empty(),
            WorkerError::UnneededValue {
                value_hash: hashed_certificate_values[0].hash(),
            }
        );

        #[cfg(with_metrics)]
        let (round, log_str, mut confirmed_transactions, mut duplicated) = (
            certificate.round,
            certificate.value().to_log_str(),
            0u64,
            false,
        );

        let (info, actions) = match certificate.value() {
            CertificateValue::ValidatedBlock { .. } => {
                // Confirm the validated block.
                let validation_outcomes = self.process_validated_block(certificate).await?;
                #[cfg(with_metrics)]
                {
                    duplicated = validation_outcomes.2;
                }
                let (info, actions, _) = validation_outcomes;
                (info, actions)
            }
            CertificateValue::ConfirmedBlock {
                executed_block: _executed_block,
            } => {
                #[cfg(with_metrics)]
                {
                    confirmed_transactions = (_executed_block.block.incoming_messages.len()
                        + _executed_block.block.operations.len())
                        as u64;
                }
                // Execute the confirmed block.
                self.process_confirmed_block(
                    certificate,
                    &hashed_certificate_values,
                    &hashed_blobs,
                    notify_when_messages_are_delivered,
                )
                .await?
            }
            CertificateValue::Timeout { .. } => {
                // Handle the leader timeout.
                self.process_timeout(certificate).await?
            }
        };

        #[cfg(with_metrics)]
        if !duplicated {
            NUM_ROUNDS_IN_CERTIFICATE
                .with_label_values(&[log_str, round.type_name()])
                .observe(round.number() as f64);
            if confirmed_transactions > 0 {
                TRANSACTION_COUNT
                    .with_label_values(&[])
                    .inc_by(confirmed_transactions);
            }
        }
        Ok((info, actions))
    }

    #[instrument(skip_all, fields(
        nick = self.nickname,
        chain_id = format!("{:.8}", query.chain_id)
    ))]
    async fn handle_chain_info_query(
        &self,
        query: ChainInfoQuery,
    ) -> Result<(ChainInfoResponse, NetworkActions), WorkerError> {
        trace!("{} <-- {:?}", self.nickname, query);
        let result = async move {
            self.create_chain_worker(query.chain_id)
                .await?
                .handle_chain_info_query(query)
                .await
        }
        .await;
        trace!("{} --> {:?}", self.nickname, result);
        result
    }

    #[instrument(skip_all, fields(
        nick = self.nickname,
        chain_id = format!("{:.8}", request.target_chain_id())
    ))]
    async fn handle_cross_chain_request(
        &mut self,
        request: CrossChainRequest,
    ) -> Result<NetworkActions, WorkerError> {
        trace!("{} <-- {:?}", self.nickname, request);
        match request {
            CrossChainRequest::UpdateRecipient {
                sender,
                recipient,
                bundle_vecs,
            } => {
                let mut height_by_origin = Vec::new();
                for (medium, bundles) in bundle_vecs {
                    let origin = Origin { sender, medium };
                    if let Some(height) = self
                        .process_cross_chain_update(origin.clone(), recipient, bundles)
                        .await?
                    {
                        height_by_origin.push((origin, height));
                    }
                }
                if height_by_origin.is_empty() {
                    return Ok(NetworkActions::default());
                }
                let mut notifications = Vec::new();
                let mut latest_heights = Vec::new();
                for (origin, height) in height_by_origin {
                    latest_heights.push((origin.medium.clone(), height));
                    notifications.push(Notification {
                        chain_id: recipient,
                        reason: Reason::NewIncomingMessage { origin, height },
                    });
                }
                let cross_chain_requests = vec![CrossChainRequest::ConfirmUpdatedRecipient {
                    sender,
                    recipient,
                    latest_heights,
                }];
                Ok(NetworkActions {
                    cross_chain_requests,
                    notifications,
                })
            }
            CrossChainRequest::ConfirmUpdatedRecipient {
                sender,
                recipient,
                latest_heights,
            } => {
                let latest_heights = latest_heights
                    .into_iter()
                    .map(|(medium, height)| (Target { recipient, medium }, height))
                    .collect();
                let height_with_fully_delivered_messages = self
                    .create_chain_worker(sender)
                    .await?
                    .confirm_updated_recipient(latest_heights)
                    .await?;
                // Handle delivery notifiers for this chain, if any.
                if let hash_map::Entry::Occupied(mut map) =
                    self.delivery_notifiers.lock().await.entry(sender)
                {
                    while let Some(entry) = map.get_mut().first_entry() {
                        if entry.key() > &height_with_fully_delivered_messages {
                            break;
                        }
                        let notifiers = entry.remove();
                        trace!("Notifying {} callers", notifiers.len());
                        for notifier in notifiers {
                            if let Err(()) = notifier.send(()) {
                                warn!("Failed to notify message delivery to caller");
                            }
                        }
                    }
                    if map.get().is_empty() {
                        map.remove();
                    }
                }
                Ok(NetworkActions::default())
            }
        }
    }
}
