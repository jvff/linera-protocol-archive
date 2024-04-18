// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A separate actor that handles requests specific to a single chain.

use linera_base::identifiers::ChainId;
use linera_chain::ChainStateView;
use linera_storage::Storage;
use linera_views::views::ViewError;
use tokio::sync::mpsc;
use tracing::{instrument, trace};

use crate::worker::WorkerError;

/// A request for the [`ChainWorker`].
pub enum ChainWorkerRequest {}

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
            match request {}
        }

        trace!("`ChainWorker` finished");
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
