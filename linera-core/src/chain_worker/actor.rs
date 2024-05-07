// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! An actor that runs a chain worker.

use linera_base::{data_types::BlockHeight, identifiers::ChainId};
use linera_chain::data_types::{Block, ExecutedBlock, Target};
use linera_execution::{Query, Response, UserApplicationDescription, UserApplicationId};
use linera_storage::Storage;
use linera_views::views::ViewError;
use tokio::sync::{mpsc, oneshot};
use tracing::{instrument, trace};

use super::{config::ChainWorkerConfig, state::ChainWorkerState};
use crate::{data_types::ChainInfoResponse, worker::WorkerError};

/// A request for the [`ChainWorkerActor`].
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

/// The actor worker type.
pub struct ChainWorkerActor<StorageClient>
where
    StorageClient: Storage + Clone + Send + Sync + 'static,
    ViewError: From<StorageClient::ContextError>,
{
    worker: ChainWorkerState<StorageClient>,
    incoming_requests: mpsc::UnboundedReceiver<ChainWorkerRequest>,
}

impl<StorageClient> ChainWorkerActor<StorageClient>
where
    StorageClient: Storage + Clone + Send + Sync + 'static,
    ViewError: From<StorageClient::ContextError>,
{
    /// Spawns a new task to run the [`ChainWorkerActor`], returning an endpoint for sending
    /// requests to the worker.
    pub async fn spawn(
        config: ChainWorkerConfig,
        storage: StorageClient,
        chain_id: ChainId,
    ) -> Result<mpsc::UnboundedSender<ChainWorkerRequest>, WorkerError> {
        let worker = ChainWorkerState::new(config, storage, chain_id).await?;
        let (sender, receiver) = mpsc::unbounded_channel();

        let actor = ChainWorkerActor {
            worker,
            incoming_requests: receiver,
        };

        tokio::spawn(actor.run());

        Ok(sender)
    }

    /// Runs the worker until there are no more incoming requests.
    #[instrument(skip_all, fields(chain_id = format!("{:.8}", self.worker.chain_id())))]
    async fn run(mut self) {
        trace!("Starting `ChainWorkerActor`");

        while let Some(request) = self.incoming_requests.recv().await {
            match request {
                ChainWorkerRequest::QueryApplication { query, callback } => {
                    let _ = callback.send(self.worker.query_application(query).await);
                }
                ChainWorkerRequest::DescribeApplication {
                    application_id,
                    callback,
                } => {
                    let _ = callback.send(self.worker.describe_application(application_id).await);
                }
                ChainWorkerRequest::StageBlockExecution { block, callback } => {
                    let _ = callback.send(self.worker.stage_block_execution(block).await);
                }
                ChainWorkerRequest::ConfirmUpdatedRecipient {
                    latest_heights,
                    callback,
                } => {
                    let _ =
                        callback.send(self.worker.confirm_updated_recipient(latest_heights).await);
                }
            }
        }

        trace!("`ChainWorkerActor` finished");
    }
}
