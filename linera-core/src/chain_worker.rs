// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A separate actor that handles requests specific to a single chain.

use linera_base::identifiers::ChainId;
use linera_chain::{
    data_types::{Block, ExecutedBlock},
    ChainStateView,
};
use linera_execution::{Query, Response};
use linera_storage::Storage;
use linera_views::views::{View, ViewError};
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

    /// Execute a block but discard any changes to the chain state.
    StageBlockExecution {
        block: Block,
        callback: oneshot::Sender<Result<(ExecutedBlock, ChainInfoResponse), WorkerError>>,
    },
}

/// The actor worker type.
pub struct ChainWorker<StorageClient>
where
    StorageClient: Storage + Send + Sync + 'static,
    ViewError: From<StorageClient::ContextError>,
{
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
        storage: StorageClient,
        chain_id: ChainId,
    ) -> Result<mpsc::UnboundedSender<ChainWorkerRequest>, WorkerError> {
        let chain = storage.load_chain(chain_id).await?;
        let (sender, receiver) = mpsc::unbounded_channel();
        let worker = ChainWorker {
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
                ChainWorkerRequest::StageBlockExecution { block, callback } => {
                    let _ = callback.send(self.stage_block_execution(block).await);
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

    /// Ensures that the current chain is active, returning an error otherwise.
    fn ensure_is_active(&mut self) -> Result<(), WorkerError> {
        if !self.knows_chain_is_active {
            self.chain.ensure_is_active()?;
            self.knows_chain_is_active = true;
        }
        Ok(())
    }
}
