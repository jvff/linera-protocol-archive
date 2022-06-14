use anyhow::{anyhow, bail, Result};
use futures::{SinkExt, StreamExt};
use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
};
use structopt::StructOpt;
use tokio::{
    net::{TcpListener, TcpStream, UdpSocket},
    sync::mpsc,
};
use tokio_util::{codec::Framed, udp::UdpFramed};
use zef_base::rpc;
use zef_service::{
    codec::Codec,
    config::{Import, ValidatorServerConfig},
    network::{ShardConfig, ValidatorNetworkConfig},
    transport::NetworkProtocol,
};

/// Options for running the proxy.
#[derive(Debug, StructOpt)]
#[structopt(
    name = "Zef Proxy",
    about = "A proxy to redirect incoming requests to Zef Server shards"
)]
pub struct ProxyOptions {
    /// Path to server configuration.
    config_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    let options = ProxyOptions::from_args();
    let config = ValidatorServerConfig::read(&options.config_path)?;

    match config.validator.network.protocol {
        NetworkProtocol::Tcp => run_tcp_proxy(config.validator.network).await,
        NetworkProtocol::Udp => run_udp_proxy(config.validator.network).await,
    }
}

async fn run_tcp_proxy(config: ValidatorNetworkConfig) -> Result<()> {
    let listener = TcpListener::bind((Ipv4Addr::new(0, 0, 0, 0), config.port)).await?;

    loop {
        match listener.accept().await {
            Ok((connection, _)) => spawn_tcp_connection_proxy(connection, config.clone()),
            Err(error) => log::debug!("Ignoring failed incoming connection. Error: {error}"),
        }
    }
}

fn spawn_tcp_connection_proxy(connection: TcpStream, config: ValidatorNetworkConfig) {
    tokio::spawn(async move {
        if let Err(error) = proxy_tcp_connection(connection, config).await {
            log::debug!("Error proxying connection: {error}");
        }
    });
}

async fn proxy_tcp_connection(connection: TcpStream, config: ValidatorNetworkConfig) -> Result<()> {
    let mut frontend_transport = Framed::new(connection, Codec);
    let request = frontend_transport
        .next()
        .await
        .ok_or_else(|| anyhow!("Disconnection before a request was received"))??;

    let shard = select_shard_for(&request, &config)?;
    let mut backend_transport = connect_to_shard_with_tcp(shard).await?;

    backend_transport.send(request).await?;
    let response = backend_transport
        .next()
        .await
        .ok_or_else(|| anyhow!("Lost connection to shard"))??;

    frontend_transport.send(response).await?;

    Ok(())
}

fn select_shard_for<'s>(
    request: &rpc::Message,
    config: &'s ValidatorNetworkConfig,
) -> Result<&'s ShardConfig> {
    let chain_id = match request {
        rpc::Message::BlockProposal(proposal) => proposal.content.block.chain_id,
        rpc::Message::Certificate(certificate) => certificate.value.chain_id(),
        rpc::Message::ChainInfoQuery(query) => query.chain_id,
        rpc::Message::Vote(_) | rpc::Message::ChainInfoResponse(_) | rpc::Message::Error(_) => {
            bail!("Can't proxy an incoming response message")
        }
        rpc::Message::CrossChainRequest(cross_chain_request) => {
            cross_chain_request.target_chain_id()
        }
    };

    Ok(config.get_shard_for(chain_id))
}

async fn connect_to_shard_with_tcp(shard: &ShardConfig) -> Result<Framed<TcpStream, Codec>> {
    let connection = TcpStream::connect((&*shard.host, shard.port)).await?;

    Ok(Framed::new(connection, Codec))
}

async fn run_udp_proxy(config: ValidatorNetworkConfig) -> Result<()> {
    let socket = UdpSocket::bind((Ipv4Addr::new(0, 0, 0, 0), config.port)).await?;
    let (response_sender, response_receiver) = mpsc::channel(10);
    let mut transport = UdpFramed::new(socket, Codec);

    loop {
        tokio::select! {
            result = transport.next().await => match result {
                Ok((message, sender_address)) => spawn_udp_request_proxy(
                    sender_address,
                    message,
                    response_sender.clone(),
                    config.clone(),
                ),
                Err(error) => log::debug!("Ignoring failed incoming request. Error: {error}"),
            },
            response = response_receiver.recv().await => {
                let response = response.expect("Internal channel should never be closed");

                if let Err(error) = transport.send(response).await {
                    log::debug!("Failed to send response: {error}");
                }
            }
        }
    }
}

fn spawn_udp_request_proxy(
    sender_address: SocketAddr,
    request: rpc::Message,
    response_sender: mpsc::Sender<()>,
    config: ValidatorNetworkConfig,
) {
    tokio::spawn(async move {
        if let Err(error) =
            proxy_udp_request(sender_address, request, config, response_sender).await
        {
            log::debug!("Error proxying request: {error}");
        }
    });
}

async fn proxy_udp_request(
    sender_address: SocketAddr,
    request: rpc::Message,
    response_sender: mpsc::Sender<()>,
    config: ValidatorNetworkConfig,
) -> Result<()> {
    let shard = select_shard_for(&request, &config)?;
    let mut backend_transport = connect_to_shard_with_udp(shard).await?;

    backend_transport.send(request).await?;
    let response = backend_transport
        .next()
        .await
        .ok_or_else(|| anyhow!("Failed to receive response from shard"))?;

    Ok(())
}

async fn connect_to_shard_with_udp(shard: &ShardConfig) -> Result<UdpFramed<Codec>> {
    todo!();
}
