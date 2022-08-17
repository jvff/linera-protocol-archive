use crate::localstack;
use aws_sdk_dynamodb::Client;
use linera_base::ensure;
use std::str::FromStr;
use thiserror::Error;

#[cfg(test)]
#[path = "unit_tests/dynamo_db_storage_tests.rs"]
pub mod dynamo_db_storage_tests;

/// Storage layer that uses Amazon DynamoDB.
#[derive(Clone, Debug)]
pub struct DynamoDbStorage {
    client: Client,
    table: TableName,
}

impl DynamoDbStorage {
    /// Create a new [`DynamoDbStorage`] instance.
    pub async fn new(table: TableName) -> Result<(Self, TableStatus), CreateTableError> {
        let config = aws_config::load_from_env().await;

        DynamoDbStorage::from_config(&config, table).await
    }

    /// Create a new [`DynamoDbStorage`] instance using the provided `config` parameters.
    pub async fn from_config(
        config: impl Into<aws_sdk_dynamodb::Config>,
        table: TableName,
    ) -> Result<(Self, TableStatus), CreateTableError> {
        let storage = DynamoDbStorage {
            client: Client::from_conf(config.into()),
            table,
        };

        let table_status = storage.create_table_if_needed().await?;

        Ok((storage, table_status))
    }

    /// Create a new [`DynamoDbStorage`] instance using a LocalStack endpoint.
    ///
    /// Requires a [`LOCALSTACK_ENDPOINT`] environment variable with the endpoint address to connect
    /// to the LocalStack instance. Creates the table if it doesn't exist yet, reporting a
    /// [`TableStatus`] to indicate if the table was created or if it already exists.
    pub async fn with_localstack(table: TableName) -> Result<(Self, TableStatus), LocalStackError> {
        let base_config = aws_config::load_from_env().await;
        let config = aws_sdk_dynamodb::config::Builder::from(&base_config)
            .endpoint_resolver(localstack::get_endpoint()?)
            .build();

        Ok(DynamoDbStorage::from_config(config, table).await?)
    }

    async fn create_table_if_needed(&self) -> Result<TableStatus, CreateTableError> {
        Ok(TableStatus::Existing)
    }
}

/// Status of a table at the creation time of a [`DynamoDbStorage`] instance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableStatus {
    /// Table was created during the construction of the [`DynamoDbStorage`] instance.
    New,
    /// Table already existed when the [`DynamoDbStorage`] instance was created.
    Existing,
}

/// A DynamoDB table name.
///
/// Table names must follow some [naming
/// rules](https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.NamingRulesDataTypes.html#HowItWorks.NamingRules),
/// so this type ensures that they are properly validated.
#[derive(Clone, Debug)]
pub struct TableName(String);

impl FromStr for TableName {
    type Err = InvalidTableName;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        ensure!(string.len() >= 3, InvalidTableName::TooShort);
        ensure!(string.len() <= 255, InvalidTableName::TooLong);
        ensure!(
            string
                .chars()
                .all(|character| character.is_ascii_alphanumeric()
                    || character == '.'
                    || character == '-'
                    || character == '_'),
            InvalidTableName::InvalidCharacter
        );

        Ok(TableName(string.to_owned()))
    }
}

impl AsRef<String> for TableName {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

/// Error when validating a table name.
#[derive(Debug, Error)]
pub enum InvalidTableName {
    #[error("Table name must have at least 3 characters")]
    TooShort,

    #[error("Table name must be at most 63 characters")]
    TooLong,

    #[error("Table name must only contain lowercase letters, numbers, periods and hyphens")]
    InvalidCharacter,
}

/// Error when creating a table for a new [`DynamoDbStorage`] instance.
#[derive(Debug, Error)]
pub enum CreateTableError {}

/// Error when creating a [`DynamoDbStorage`] instance using a LocalStack instance.
#[derive(Debug, Error)]
pub enum LocalStackError {
    #[error(transparent)]
    Endpoint(#[from] localstack::EndpointError),

    #[error(transparent)]
    CreateTable(#[from] CreateTableError),
}
