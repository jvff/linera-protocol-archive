use super::{S3Storage, CERTIFICATE_BUCKET, CHAIN_BUCKET};
use aws_sdk_s3::Endpoint;
use aws_types::SdkConfig;
use std::{
    env::{self, VarError},
    error::Error,
};
use thiserror::Error;
use tokio::sync::{Mutex, MutexGuard};

/// A static lock to prevent multiple tests from using the same LocalStack instance at the same
/// time.
static LOCALSTACK_GUARD: Mutex<()> = Mutex::const_new(());

/// Name of the environment variable with the address to a LocalStack instance.
const LOCALSTACK_ENDPOINT: &str = "LOCALSTACK_ENDPOINT";

/// A type to help tests that need a LocalStack instance.
struct LocalStackTestContext {
    base_config: SdkConfig,
    endpoint: Endpoint,
    _guard: MutexGuard<'static, ()>,
}

impl LocalStackTestContext {
    /// Creates an instance of [`LocalStackTestContext`], loading the necessary LocalStack
    /// configuration.
    ///
    /// An address to the LocalStack instance must be specified using a [`LOCALSTACK_ENDPOINT`]
    /// environment variable.
    ///
    /// This also locks the [`LOCALSTACK_GUARD`] to enforce only one test has access to the
    /// LocalStack instance.
    pub async fn new() -> Result<LocalStackTestContext, Box<dyn Error>> {
        let base_config = aws_config::load_from_env().await;
        let endpoint = Self::load_endpoint()?;
        let _guard = LOCALSTACK_GUARD.lock().await;

        let context = LocalStackTestContext {
            base_config,
            endpoint,
            _guard,
        };

        context.clear().await?;

        Ok(context)
    }

    /// Creates an [`Endpoint`] using the configuration in the [`LOCALSTACK_ENDPOINT`] environment
    /// variable.
    fn load_endpoint() -> Result<Endpoint, LocalStackEndpointError> {
        Ok(Endpoint::immutable(env::var(LOCALSTACK_ENDPOINT)?.parse()?))
    }

    /// Create a new [`aws_sdk_s3::Config`] for tests, using a LocalStack instance.
    pub fn config(&self) -> aws_sdk_s3::Config {
        aws_sdk_s3::config::Builder::from(&self.base_config)
            .endpoint_resolver(self.endpoint.clone())
            .build()
    }

    /// Remove all buckets from the LocalStack S3 storage.
    async fn clear(&self) -> Result<(), Box<dyn Error>> {
        let client = aws_sdk_s3::Client::from_conf(self.config());

        for bucket in list_buckets(&client).await? {
            let objects = client.list_objects().bucket(&bucket).send().await?;

            for object in objects.contents().into_iter().flatten() {
                if let Some(key) = object.key.as_ref() {
                    client
                        .delete_object()
                        .bucket(&bucket)
                        .key(key)
                        .send()
                        .await?;
                }
            }

            client.delete_bucket().bucket(bucket).send().await?;
        }

        Ok(())
    }
}

/// Test if the necessary buckets are created if needed.
#[tokio::test]
#[ignore]
async fn buckets_are_created() -> Result<(), Box<dyn Error>> {
    let localstack = LocalStackTestContext::new().await?;
    let client = aws_sdk_s3::Client::from_conf(localstack.config());

    let initial_buckets = list_buckets(&client).await?;

    assert!(!initial_buckets.contains(&CERTIFICATE_BUCKET.to_owned()));
    assert!(!initial_buckets.contains(&CHAIN_BUCKET.to_owned()));

    let _storage = S3Storage::from_config(localstack.config()).await?;

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

#[derive(Debug, Error)]
pub enum LocalStackEndpointError {
    #[error("Missing LocalStack endpoint address in {LOCALSTACK_ENDPOINT:?} environment variable")]
    Missing(#[from] VarError),

    #[error("LocalStack endpoint address is not a valid URI")]
    Invalid(#[from] http::uri::InvalidUri),
}
