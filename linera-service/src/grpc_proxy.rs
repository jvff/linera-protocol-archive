use anyhow::Result;
use async_trait::async_trait;
use linera_rpc::{
    config::{ValidatorInternalNetworkConfig, ValidatorPublicNetworkConfig},
    grpc_network::{
        grpc_network::{validator_worker_server::ValidatorWorker, ChainInfoResult},
        BlockProposal, Certificate, ChainInfoQuery, CrossChainRequest,
    },
    pool::ClientPool,
};
use linera_service::config::{Import, ValidatorServerConfig};
use std::{net::SocketAddr, path::PathBuf, str::FromStr};
use structopt::StructOpt;
use tonic::{transport::Server, Request, Response, Status};
use tonic::transport::Channel;
use linera_base::messages::ChainId;
use linera_chain::messages::{BlockAndRound, Value};
use linera_rpc::config::ShardConfig;
use linera_rpc::grpc_network::grpc_network::validator_worker_client::ValidatorWorkerClient;
use linera_rpc::grpc_network::grpc_network::validator_worker_server::ValidatorWorkerServer;

/// Boilerplate to extract the underlying chain id, use it to get the corresponding shard
/// and forward the message.
macro_rules! proxy {
    ($self:ident, $handler:ident, $req:ident) => {{
        let inner = $req.into_inner();
        let shard = $self.shard_for(&inner).expect("todo: map to status");
        let mut client = $self.client_for_shard(&shard).await.expect("todo: map to status");
        client.$handler(inner).await
    }};
}

/// Options for running the proxy.
#[derive(Debug, StructOpt)]
#[structopt(
    name = "Linera gRPC Proxy",
    about = "A proxy to redirect incoming requests to Linera Server shards"
)]
pub struct GrpcProxyOptions {
    /// Path to server configuration.
    config_path: PathBuf,
}

#[derive(Clone)]
pub struct GrpcProxy {
    public_config: ValidatorPublicNetworkConfig,
    internal_config: ValidatorInternalNetworkConfig,
    pool: ClientPool,
}

impl GrpcProxy {
    async fn spawn(
        public_config: ValidatorPublicNetworkConfig,
        internal_config: ValidatorInternalNetworkConfig,
    ) -> Result<()> {
        let grpc_proxy = GrpcProxy {
            public_config,
            internal_config,
            pool: ClientPool::new(),
        };

        let address = grpc_proxy.address()?;

        Ok(Server::builder()
            .add_service(grpc_proxy.into_server())
            .serve(address)
            .await?)
    }

    fn into_server(self) -> ValidatorWorkerServer<Self> {
        ValidatorWorkerServer::new(self)
    }

    fn address(&self) -> Result<SocketAddr> {
        Ok(SocketAddr::from_str(&format!(
            "0.0.0.0:{}",
            self.public_config.port
        ))?)
    }

    fn shard_for(&self, proxyable: &impl Proxyable) -> Option<ShardConfig> {
        Some(self.internal_config.get_shard_for(proxyable.chain_id()?).clone())
    }

    // todo: if we want to use a pool here we'll need to wrap it up in an Arc<Mutex>>
    async fn client_for_shard(&self, shard: &ShardConfig) -> Result<ValidatorWorkerClient<Channel>> {
        let address = format!("{}:{}", shard.host, shard.port);
        let client = ValidatorWorkerClient::connect(address).await?;
        Ok(client)
    }
}

#[async_trait]
impl ValidatorWorker for GrpcProxy {
    async fn handle_block_proposal(
        &self,
        request: Request<BlockProposal>,
    ) -> Result<Response<ChainInfoResult>, Status> {
        proxy!(self, handle_block_proposal, request)
    }

    async fn handle_certificate(
        &self,
        request: Request<Certificate>,
    ) -> Result<Response<ChainInfoResult>, Status> {
        proxy!(self, handle_certificate, request)
    }

    async fn handle_chain_info_query(
        &self,
        request: Request<ChainInfoQuery>,
    ) -> Result<Response<ChainInfoResult>, Status> {
        proxy!(self, handle_chain_info_query, request)
    }

    async fn handle_cross_chain_request(
        &self,
        request: Request<CrossChainRequest>,
    ) -> Result<Response<CrossChainRequest>, Status> {
        proxy!(self, handle_cross_chain_request, request)
    }
}

/// Types which are proxyable and expose the appropriate methods to be handled
/// by the `GrpcProxy`
trait Proxyable {
    fn chain_id(&self) -> Option<ChainId>;
}

impl Proxyable for BlockProposal {
    fn chain_id(&self) -> Option<ChainId> {
        match bcs::from_bytes::<BlockAndRound>(&self.content) {
            Ok(block_and_round) => Some(block_and_round.block.chain_id),
            Err(_) => None
        }
    }
}

impl Proxyable for Certificate {
    fn chain_id(&self) -> Option<ChainId> {
        match bcs::from_bytes::<Value>(&self.value) {
            Ok(value) => Some(value.chain_id()),
            Err(_) => None
        }
    }
}

impl Proxyable for ChainInfoQuery {
    fn chain_id(&self) -> Option<ChainId> {
        match self.chain_id.as_ref().map(|id| ChainId::try_from(id.clone()))? {
            Ok(id) => Some(id),
            Err(_) => None
        }
    }
}

impl Proxyable for CrossChainRequest {
    fn chain_id(&self) -> Option<ChainId> {
        match linera_core::messages::CrossChainRequest::try_from(self.clone()) {
            Ok(cross_chain_request) => Some(cross_chain_request.target_chain_id()),
            Err(_) => None
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let options = GrpcProxyOptions::from_args();
    let config = ValidatorServerConfig::read(&options.config_path)?;

    let handler = GrpcProxy::spawn(
        config.validator.network,
        config.internal_network,
    );

    if let Err(error) = handler.await {
        log::error!("Failed to run proxy: {error}");
    }

    Ok(())
}
