// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A separate actor that handles requests specific to a single chain.

use linera_base::{data_types::BlockHeight, identifiers::ChainId};
use linera_chain::{
    data_types::{Block, ExecutedBlock, Target},
    ChainStateView,
};
use linera_execution::{Query, Response, UserApplicationDescription, UserApplicationId};
use linera_storage::Storage;
use linera_views::views::{RootView, View, ViewError};
use tokio::sync::{mpsc, oneshot};
use tracing::{instrument, trace};

use crate::{data_types::ChainInfoResponse, worker::WorkerError};

/// A request for the [`ChainWorker`].
pub enum ChainWorkerRequest {
    /// Query an application's state.
    QueryApplication {
        query: Query,
        callback: oneshot::Sender<Result<Response, WorkerError>>,
    },

    /// Describe an application.
    DescribeApplication {
        application_id: UserApplicationId,
        callback: oneshot::Sender<Result<UserApplicationDescription, WorkerError>>,
    },

    /// Execute a block but discard any changes to the chain state.
    StageBlockExecution {
        block: Block,
        callback: oneshot::Sender<Result<(ExecutedBlock, ChainInfoResponse), WorkerError>>,
    },

    /// Handle cross-chain request to confirm that the recipient was updated.
    ConfirmUpdatedRecipient {
        latest_heights: Vec<(Target, BlockHeight)>,
        callback: oneshot::Sender<Result<BlockHeight, WorkerError>>,
    },
}

/// Configuration parameters for the [`ChainWorker`].
#[derive(Clone, Debug, Default)]
pub struct ChainWorkerConfig {
    /// Whether inactive chains are allowed in storage.
    pub allow_inactive_chains: bool,
    /// Whether new messages from deprecated epochs are allowed.
    pub allow_messages_from_deprecated_epochs: bool,
}

/// The actor worker type.
pub struct ChainWorker<StorageClient>
where
    StorageClient: Storage + Send + Sync + 'static,
    ViewError: From<StorageClient::ContextError>,
{
    config: ChainWorkerConfig,
    storage: StorageClient,
    chain: ChainStateView<StorageClient::Context>,
    incoming_requests: mpsc::UnboundedReceiver<ChainWorkerRequest>,
    knows_chain_is_active: bool,
}

impl<StorageClient> ChainWorker<StorageClient>
where
    StorageClient: Storage + Send + Sync + 'static,
    ViewError: From<StorageClient::ContextError>,
{
    /// Spawns a new task to run the [`ChainWorker`], returning an endpoint for sending
    /// requests to the worker.
    pub async fn spawn(
        config: ChainWorkerConfig,
        storage: StorageClient,
        chain_id: ChainId,
    ) -> Result<mpsc::UnboundedSender<ChainWorkerRequest>, WorkerError> {
        let chain = storage.load_chain(chain_id).await?;
        let (sender, receiver) = mpsc::unbounded_channel();
        let worker = ChainWorker {
            config,
            storage,
            chain,
            incoming_requests: receiver,
            knows_chain_is_active: false,
        };

        tokio::spawn(worker.run());

        Ok(sender)
    }

    /// Runs the worker until there are no more incoming requests.
    #[instrument(skip_all, fields(chain_id = format!("{:.8}", self.chain.chain_id())))]
    async fn run(mut self) {
        trace!("Starting `ChainWorker`");

        while let Some(request) = self.incoming_requests.recv().await {
            match request {
                ChainWorkerRequest::QueryApplication { query, callback } => {
                    let _ = callback.send(self.query_application(query).await);
                }
                ChainWorkerRequest::DescribeApplication {
                    application_id,
                    callback,
                } => {
                    let _ = callback.send(self.describe_application(application_id).await);
                }
                ChainWorkerRequest::StageBlockExecution { block, callback } => {
                    let _ = callback.send(self.stage_block_execution(block).await);
                }
                ChainWorkerRequest::ConfirmUpdatedRecipient {
                    latest_heights,
                    callback,
                } => {
                    let _ = callback.send(self.confirm_updated_recipient(latest_heights).await);
                }
            }
        }

        trace!("`ChainWorker` finished");
    }

    /// Queries an application's state on the chain.
    async fn query_application(&mut self, query: Query) -> Result<Response, WorkerError> {
        self.ensure_is_active()?;
        let response = self.chain.query_application(query).await?;
        Ok(response)
    }

    /// Returns an application's description.
    async fn describe_application(
        &mut self,
        application_id: UserApplicationId,
    ) -> Result<UserApplicationDescription, WorkerError> {
        self.ensure_is_active()?;
        let response = self.chain.describe_application(application_id).await?;
        Ok(response)
    }

    /// Executes a block without persisting any changes to the state.
    async fn stage_block_execution(
        &mut self,
        block: Block,
    ) -> Result<(ExecutedBlock, ChainInfoResponse), WorkerError> {
        self.ensure_is_active()?;

        let local_time = self.storage.clock().current_time();
        let signer = block.authenticated_signer;

        let executed_block = self
            .chain
            .execute_block(&block, local_time)
            .await?
            .with(block);

        let mut response = ChainInfoResponse::new(&self.chain, None);
        if let Some(signer) = signer {
            response.info.requested_owner_balance = self
                .chain
                .execution_state
                .system
                .balances
                .get(&signer)
                .await?;
        }

        self.chain.rollback();

        Ok((executed_block, response))
    }

    /// Handles the cross-chain request confirming that the recipient was updated.
    async fn confirm_updated_recipient(
        &mut self,
        latest_heights: Vec<(Target, BlockHeight)>,
    ) -> Result<BlockHeight, WorkerError> {
        let mut height_with_fully_delivered_messages = BlockHeight::ZERO;

        for (target, height) in latest_heights {
            let fully_delivered = self
                .chain
                .mark_messages_as_received(&target, height)
                .await?
                && self.chain.all_messages_delivered_up_to(height);

            if fully_delivered && height > height_with_fully_delivered_messages {
                height_with_fully_delivered_messages = height;
            }
        }

        self.chain.save().await?;

        Ok(height_with_fully_delivered_messages)
    }

    /// Ensures that the current chain is active, returning an error otherwise.
    fn ensure_is_active(&mut self) -> Result<(), WorkerError> {
        if !self.knows_chain_is_active {
            self.chain.ensure_is_active()?;
            self.knows_chain_is_active = true;
        }
        Ok(())
    }
}
