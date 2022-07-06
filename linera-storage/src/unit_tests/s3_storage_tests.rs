use super::{S3Storage, CERTIFICATE_BUCKET, CHAIN_BUCKET};
use crate::Storage;
use aws_sdk_s3::Endpoint;
use linera_base::{
    chain::ChainState,
    crypto::HashValue,
    execution::{ExecutionState, Operation},
    messages::{Block, BlockHeight, Certificate, ChainId, Epoch, Value},
};
use std::{
    env::{self, VarError},
    error::Error,
};
use thiserror::Error;

/// Name of the environment variable with the address to a LocalStack instance.
const LOCALSTACK_ENDPOINT: &str = "LOCALSTACK_ENDPOINT";

/// Create a new [`aws_sdk_s3::Config`] for tests, using a LocalStack instance.
///
/// An address to the LocalStack instance must be specified using a [`LOCALSTACK_ENDPOINT`]
/// environment variable.
async fn new_local_stack_config() -> Result<aws_sdk_s3::Config, LocalStackEndpointError> {
    let base_config = aws_config::load_from_env().await;
    let localstack_endpoint = Endpoint::immutable(env::var(LOCALSTACK_ENDPOINT)?.parse()?);

    let s3_config = aws_sdk_s3::config::Builder::from(&base_config)
        .endpoint_resolver(localstack_endpoint)
        .build();

    Ok(s3_config)
}

/// Test if the necessary buckets are created if needed.
#[tokio::test]
#[ignore]
async fn buckets_are_created() -> Result<(), Box<dyn Error>> {
    let config = new_local_stack_config().await?;
    let client = aws_sdk_s3::Client::from_conf(config);

    let initial_buckets = list_buckets(&client).await?;

    for bucket in [CERTIFICATE_BUCKET, CHAIN_BUCKET] {
        if initial_buckets.contains(&bucket.to_owned()) {
            client.delete_bucket().bucket(bucket).send().await?;
        }
    }

    let config = new_local_stack_config().await?;
    let _storage = S3Storage::from_config(config).await?;

    let buckets = list_buckets(&client).await?;

    assert!(buckets.contains(&CERTIFICATE_BUCKET.to_owned()));
    assert!(buckets.contains(&CHAIN_BUCKET.to_owned()));

    Ok(())
}

/// Helper function to list the names of buckets registered on S3.
async fn list_buckets(client: &aws_sdk_s3::Client) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(client
        .list_buckets()
        .send()
        .await?
        .buckets
        .expect("List of buckets was not returned")
        .into_iter()
        .filter_map(|bucket| bucket.name)
        .collect())
}

/// Test if certificates are stored and retrieved correctly.
#[tokio::test]
#[ignore]
async fn certificate_storage_round_trip() -> Result<(), Box<dyn Error>> {
    let block = Block {
        epoch: Epoch::from(0),
        chain_id: ChainId::root(1),
        incoming_messages: Vec::new(),
        operations: vec![Operation::CloseChain],
        previous_block_hash: None,
        height: BlockHeight::default(),
    };
    let value = Value::ConfirmedBlock {
        block,
        effects: Vec::new(),
        state_hash: HashValue::new(&ExecutionState::new(ChainId::root(1))),
    };
    let certificate = Certificate::new(value, vec![]);

    let config = new_local_stack_config().await?;
    let mut storage = S3Storage::from_config(config).await?;

    storage.write_certificate(certificate.clone()).await?;

    let stored_certificate = storage.read_certificate(certificate.hash).await?;

    assert_eq!(certificate, stored_certificate);

    Ok(())
}

/// Test if chain states are stored and retrieved correctly.
#[tokio::test]
#[ignore]
async fn chain_storage_round_trip() -> Result<(), Box<dyn Error>> {
    let config = new_local_stack_config().await?;
    let mut storage = S3Storage::from_config(config).await?;

    let chain_id = ChainId::root(1);
    let chain_state = ChainState {
        next_block_height: BlockHeight(100),
        ..ChainState::new(chain_id)
    };

    storage.write_chain(chain_state.clone()).await?;

    let stored_chain_state = storage
        .read_chain_or_default(chain_state.state.chain_id)
        .await?;

    assert_eq!(chain_state, stored_chain_state);

    Ok(())
}

#[derive(Debug, Error)]
pub enum LocalStackEndpointError {
    #[error("Missing LocalStack endpoint address in {LOCALSTACK_ENDPOINT:?} environment variable")]
    Missing(#[from] VarError),

    #[error("LocalStack endpoint address is not a valid URI")]
    Invalid(#[from] http::uri::InvalidUri),
}
