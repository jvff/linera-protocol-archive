use super::{BucketName, BucketStatus, S3Storage};
use crate::Storage;
use anyhow::{Context, Error};
use async_trait::async_trait;
use linera_base::{
    chain::ChainState,
    crypto::HashValue,
    execution::{ExecutionState, SYSTEM},
    messages::{
        Block, BlockHeight, Certificate, ChainDescription, ChainId, Epoch, Operation, Value,
    },
    system::SystemOperation,
};
use linera_views::test_utils::{list_buckets, LocalStackTestContext};

/// Test if the bucket for the storage is created when needed.
#[tokio::test]
#[ignore]
async fn bucket_is_created() -> Result<(), Error> {
    let localstack = LocalStackTestContext::new().await?;
    let client = aws_sdk_s3::Client::from_conf(localstack.s3_config());
    let bucket: BucketName = "linera".parse().expect("Invalid bucket name");

    let initial_buckets = list_buckets(&client).await?;
    assert!(!initial_buckets.contains(bucket.as_ref()));

    let (_storage, bucket_status) =
        S3Storage::from_config(localstack.s3_config(), bucket.clone()).await?;

    let buckets = list_buckets(&client).await?;
    assert!(buckets.contains(bucket.as_ref()));
    assert_eq!(bucket_status, BucketStatus::New);

    Ok(())
}

/// Test if two independent buckets for two separate storages are created.
#[tokio::test]
#[ignore]
async fn separate_buckets_are_created() -> Result<(), Error> {
    let localstack = LocalStackTestContext::new().await?;
    let client = aws_sdk_s3::Client::from_conf(localstack.s3_config());
    let first_bucket: BucketName = "first".parse().expect("Invalid bucket name");
    let second_bucket: BucketName = "second".parse().expect("Invalid bucket name");

    let initial_buckets = list_buckets(&client).await?;
    assert!(!initial_buckets.contains(first_bucket.as_ref()));
    assert!(!initial_buckets.contains(second_bucket.as_ref()));

    let (_storage, first_bucket_status) =
        S3Storage::from_config(localstack.s3_config(), first_bucket.clone()).await?;
    let (_storage, second_bucket_status) =
        S3Storage::from_config(localstack.s3_config(), second_bucket.clone()).await?;

    let buckets = list_buckets(&client).await?;
    assert!(buckets.contains(first_bucket.as_ref()));
    assert!(buckets.contains(second_bucket.as_ref()));
    assert_eq!(first_bucket_status, BucketStatus::New);
    assert_eq!(second_bucket_status, BucketStatus::New);

    Ok(())
}

/// Test if certificates are stored and retrieved correctly.
#[tokio::test]
#[ignore]
async fn certificate_storage_round_trip() -> Result<(), Error> {
    let block = Block {
        epoch: Epoch::from(0),
        chain_id: ChainId::root(1),
        incoming_messages: Vec::new(),
        operations: vec![(SYSTEM, Operation::System(SystemOperation::CloseChain))],
        previous_block_hash: None,
        height: BlockHeight::default(),
    };
    let value = Value::ConfirmedBlock {
        block,
        effects: Vec::new(),
        state_hash: HashValue::new(&ExecutionState::new(ChainId::root(1))),
    };
    let certificate = Certificate::new(value, vec![]);

    let localstack = LocalStackTestContext::new().await?;
    let mut storage = localstack.create_s3_storage().await?;

    storage.write_certificate(certificate.clone()).await?;

    let stored_certificate = storage.read_certificate(certificate.hash).await?;

    assert_eq!(certificate, stored_certificate);

    Ok(())
}

/// Test if retrieving inexistent certificates fails.
#[tokio::test]
#[ignore]
async fn retrieval_of_inexistent_certificate() -> Result<(), Error> {
    let certificate_hash = HashValue::new(&ChainDescription::Root(123));

    let localstack = LocalStackTestContext::new().await?;
    let mut storage = localstack.create_s3_storage().await?;

    let result = storage.read_certificate(certificate_hash).await;

    assert!(result.is_err());

    Ok(())
}

/// Test if chain states are stored and retrieved correctly.
#[tokio::test]
#[ignore]
async fn chain_storage_round_trip() -> Result<(), Error> {
    let chain_id = ChainId::root(1);
    let chain_state = ChainState {
        next_block_height: BlockHeight(100),
        ..ChainState::new(chain_id)
    };

    let localstack = LocalStackTestContext::new().await?;
    let mut storage = localstack.create_s3_storage().await?;

    storage.write_chain(chain_state.clone()).await?;

    let stored_chain_state = storage
        .read_chain_or_default(chain_state.state.system.chain_id)
        .await?;

    assert_eq!(chain_state, stored_chain_state);

    Ok(())
}

/// Test if retrieving inexistent chain states creates new [`ChainState`] instances.
#[tokio::test]
#[ignore]
async fn retrieval_of_inexistent_chain_state() -> Result<(), Error> {
    let chain_id = ChainId::root(5);

    let localstack = LocalStackTestContext::new().await?;
    let mut storage = localstack.create_s3_storage().await?;

    let chain_state = storage.read_chain_or_default(chain_id).await?;
    let expected_chain_state = ChainState::new(chain_id);

    assert_eq!(chain_state, expected_chain_state);

    Ok(())
}

/// Test if chain states are stored and retrieved correctly.
#[tokio::test]
#[ignore]
async fn removal_of_chain_state() -> Result<(), Error> {
    let chain_id = ChainId::root(9);
    let chain_state = ChainState {
        next_block_height: BlockHeight(300),
        ..ChainState::new(chain_id)
    };

    let localstack = LocalStackTestContext::new().await?;
    let mut storage = localstack.create_s3_storage().await?;

    storage.write_chain(chain_state).await?;
    storage.remove_chain(chain_id).await?;

    let retrieved_chain_state = storage.read_chain_or_default(chain_id).await?;
    let expected_chain_state = ChainState::new(chain_id);

    assert_eq!(retrieved_chain_state, expected_chain_state);

    Ok(())
}

/// Extension trait to make it easier to create [`S3Storage`] instances from a
/// [`LocalStackTestContext`].
#[async_trait]
trait CreateS3Storage {
    /// Create a new [`S3Storage`] instance, using a LocalStack instance.
    async fn create_s3_storage(&self) -> Result<S3Storage, Error>;
}

#[async_trait]
impl CreateS3Storage for LocalStackTestContext {
    async fn create_s3_storage(&self) -> Result<S3Storage, Error> {
        let bucket = "linera".parse().context("Invalid S3 bucket name")?;
        let (storage, _) = S3Storage::from_config(self.s3_config(), bucket).await?;
        Ok(storage)
    }
}
