use crate::localstack;
use aws_sdk_dynamodb::{
    model::{
        AttributeDefinition, AttributeValue, KeySchemaElement, KeyType, ProvisionedThroughput,
        ScalarAttributeType,
    },
    types::SdkError,
    Client,
};
use linera_base::{ensure, error::Error};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Display, str::FromStr};
use thiserror::Error;

/// The attribute name of the primary key.
const KEY_ATTRIBUTE: &str = "key";

/// The attribute name of the table value blob.
const VALUE_ATTRIBUTE: &str = "value";

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

    /// Create the storage table if it doesn't exist.
    async fn create_table_if_needed(&self) -> Result<TableStatus, CreateTableError> {
        self.client
            .create_table()
            .table_name(self.table.as_ref())
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name(KEY_ATTRIBUTE)
                    .attribute_type(ScalarAttributeType::S)
                    .build(),
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name(KEY_ATTRIBUTE)
                    .key_type(KeyType::Hash)
                    .build(),
            )
            .provisioned_throughput(
                ProvisionedThroughput::builder()
                    .read_capacity_units(10)
                    .write_capacity_units(10)
                    .build(),
            )
            .send()
            .await?;

        Ok(TableStatus::New)
    }

    /// Build the key attribute value for a table item.
    fn build_key(&self, prefix: &str, key: impl Display) -> (String, AttributeValue) {
        (
            KEY_ATTRIBUTE.to_owned(),
            AttributeValue::S(format!("{}-{}", prefix, key)),
        )
    }

    /// Build the value attribute for storing a table item.
    fn build_value(&self, value: &impl Serialize) -> (String, AttributeValue) {
        (
            VALUE_ATTRIBUTE.to_owned(),
            AttributeValue::S(ron::to_string(value).expect("Serialization failed")),
        )
    }

    /// Retrieve a generic `Item` from the table using the provided `key` prefixed with `prefix`.
    ///
    /// The `Item` is deserialized using [`ron`].
    async fn get_item<Item>(
        &mut self,
        prefix: &str,
        key: impl Display,
    ) -> Result<Item, DynamoDbStorageError>
    where
        Item: DeserializeOwned,
    {
        let response = self
            .client
            .get_item()
            .table_name(self.table.as_ref())
            .set_key(Some([self.build_key(prefix, key)].into()))
            .send()
            .await?;

        let string = response
            .item()
            .ok_or(DynamoDbStorageError::ItemNotFound)?
            .get(VALUE_ATTRIBUTE)
            .ok_or(DynamoDbStorageError::MissingValue)?
            .as_s()
            .map_err(DynamoDbStorageError::wrong_value_type)?;

        let item = ron::from_str(string)?;

        Ok(item)
    }

    /// Store a generic `value` into the table using the provided `key` prefixed with `prefix`.
    ///
    /// The value is serialized using [`ron`].
    async fn put_item(
        &self,
        prefix: &str,
        key: impl Display,
        value: &impl Serialize,
    ) -> Result<(), DynamoDbStorageError> {
        let item = [self.build_key(prefix, key), self.build_value(value)].into();

        self.client
            .put_item()
            .table_name(self.table.as_ref())
            .set_item(Some(item))
            .send()
            .await?;

        Ok(())
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

/// Errors that occur when using [`DynamoDbStorage`].
#[derive(Debug, Error)]
pub enum DynamoDbStorageError {
    #[error(transparent)]
    Put(#[from] Box<SdkError<aws_sdk_dynamodb::error::PutItemError>>),

    #[error(transparent)]
    Get(#[from] Box<SdkError<aws_sdk_dynamodb::error::GetItemError>>),

    #[error("Item not found in table")]
    ItemNotFound,

    #[error("The stored value attribute is missing")]
    MissingValue,

    #[error("Value was stored as {0}, but it was expected to be stored as a string")]
    WrongValueType(String),

    #[error(transparent)]
    Deserialization(#[from] ron::Error),
}

impl<InnerError> From<SdkError<InnerError>> for DynamoDbStorageError
where
    DynamoDbStorageError: From<Box<SdkError<InnerError>>>,
{
    fn from(error: SdkError<InnerError>) -> Self {
        Box::new(error).into()
    }
}

impl DynamoDbStorageError {
    /// Create a [`DynamoDbStorageError::WrongValueType`] instance based on the returned value type.
    ///
    /// # Panics
    ///
    /// If the value type is in the correct type, a string scalar.
    pub fn wrong_value_type(value: &AttributeValue) -> Self {
        let type_description = match value {
            AttributeValue::B(_) => "a binary blob",
            AttributeValue::Bool(_) => "a boolean",
            AttributeValue::Bs(_) => "a list of binary blobs",
            AttributeValue::L(_) => "a list",
            AttributeValue::M(_) => "a map",
            AttributeValue::N(_) => "a number",
            AttributeValue::Ns(_) => "a list of numbers",
            AttributeValue::Null(_) => "a null value",
            AttributeValue::Ss(_) => "a list of strings",
            AttributeValue::S(_) => unreachable!("creating an error type for the correct type"),
            _ => "an unknown type",
        }
        .to_owned();

        DynamoDbStorageError::WrongValueType(type_description)
    }

    /// Convert the error into an instance of the main [`Error`] type.
    pub fn into_base_error(self) -> Error {
        Error::StorageIoError {
            error: self.to_string(),
        }
    }
}

/// Error when creating a table for a new [`DynamoDbStorage`] instance.
#[derive(Debug, Error)]
pub enum CreateTableError {
    #[error(transparent)]
    CreateTable(#[from] SdkError<aws_sdk_dynamodb::error::CreateTableError>),
}

/// Error when creating a [`DynamoDbStorage`] instance using a LocalStack instance.
#[derive(Debug, Error)]
pub enum LocalStackError {
    #[error(transparent)]
    Endpoint(#[from] localstack::EndpointError),

    #[error(transparent)]
    CreateTable(#[from] Box<CreateTableError>),
}

impl From<CreateTableError> for LocalStackError {
    fn from(error: CreateTableError) -> Self {
        Box::new(error).into()
    }
}
