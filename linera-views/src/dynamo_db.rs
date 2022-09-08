// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::localstack;
use aws_sdk_dynamodb::{
    model::{
        AttributeDefinition, AttributeValue, KeySchemaElement, KeyType, ProvisionedThroughput,
        ScalarAttributeType,
    },
    types::{Blob, SdkError},
    Client,
};
use linera_base::ensure;
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use thiserror::Error;
use tokio::sync::OwnedMutexGuard;

#[cfg(test)]
#[path = "unit_tests/dynamo_db_context_tests.rs"]
pub mod dynamo_db_context_tests;

/// The attribute name of the partition key.
const PARTITION_ATTRIBUTE: &str = "partition";

/// A dummy value to use as the partition key.
const DUMMY_PARTITION_KEY: &[u8] = &[0];

/// The attribute name of the primary key (used as a sort key).
const KEY_ATTRIBUTE: &str = "item_key";

/// The attribute name of the table value blob.
const VALUE_ATTRIBUTE: &str = "item_value";

/// A implementation of [`Context`] based on DynamoDB.
#[derive(Debug, Clone)]
pub struct DynamoDbContext<E> {
    client: Client,
    table: TableName,
    lock: Arc<OwnedMutexGuard<()>>,
    key_prefix: Vec<u8>,
    extra: E,
}

impl<E> DynamoDbContext<E> {
    /// Create a new [`DynamoDbContext`] instance.
    pub async fn new(
        table: TableName,
        lock: OwnedMutexGuard<()>,
        key_prefix: Vec<u8>,
        extra: E,
    ) -> Result<(Self, TableStatus), CreateTableError> {
        let config = aws_config::load_from_env().await;

        DynamoDbContext::from_config(&config, table, lock, key_prefix, extra).await
    }

    /// Create a new [`DynamoDbContext`] instance using the provided `config` parameters.
    pub async fn from_config(
        config: impl Into<aws_sdk_dynamodb::Config>,
        table: TableName,
        lock: OwnedMutexGuard<()>,
        key_prefix: Vec<u8>,
        extra: E,
    ) -> Result<(Self, TableStatus), CreateTableError> {
        let storage = DynamoDbContext {
            client: Client::from_conf(config.into()),
            table,
            lock: Arc::new(lock),
            key_prefix,
            extra,
        };

        let table_status = storage.create_table_if_needed().await?;

        Ok((storage, table_status))
    }

    /// Create a new [`DynamoDbContext`] instance using a LocalStack endpoint.
    ///
    /// Requires a [`LOCALSTACK_ENDPOINT`] environment variable with the endpoint address to connect
    /// to the LocalStack instance. Creates the table if it doesn't exist yet, reporting a
    /// [`TableStatus`] to indicate if the table was created or if it already exists.
    pub async fn with_localstack(
        table: TableName,
        lock: OwnedMutexGuard<()>,
        key_prefix: Vec<u8>,
        extra: E,
    ) -> Result<(Self, TableStatus), LocalStackError> {
        let base_config = aws_config::load_from_env().await;
        let config = aws_sdk_dynamodb::config::Builder::from(&base_config)
            .endpoint_resolver(localstack::get_endpoint()?)
            .build();

        Ok(DynamoDbContext::from_config(config, table, lock, key_prefix, extra).await?)
    }

    /// Create the storage table if it doesn't exist.
    ///
    /// Attempts to create the table and ignores errors that indicate that it already exists.
    async fn create_table_if_needed(&self) -> Result<TableStatus, CreateTableError> {
        let result = self
            .client
            .create_table()
            .table_name(self.table.as_ref())
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name(PARTITION_ATTRIBUTE)
                    .attribute_type(ScalarAttributeType::B)
                    .build(),
            )
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name(KEY_ATTRIBUTE)
                    .attribute_type(ScalarAttributeType::B)
                    .build(),
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name(PARTITION_ATTRIBUTE)
                    .key_type(KeyType::Hash)
                    .build(),
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name(KEY_ATTRIBUTE)
                    .key_type(KeyType::Range)
                    .build(),
            )
            .provisioned_throughput(
                ProvisionedThroughput::builder()
                    .read_capacity_units(10)
                    .write_capacity_units(10)
                    .build(),
            )
            .send()
            .await;

        match result {
            Ok(_) => Ok(TableStatus::New),
            Err(error) if error.is_resource_in_use_exception() => Ok(TableStatus::Existing),
            Err(error) => Err(error.into()),
        }
    }

    /// Build the key attributes for a table item.
    ///
    /// The key is composed of two attributes that are both binary blobs. The first attribute is a
    /// partition key and is currently just a dummy value that ensures all items are in the same
    /// partion. This is necessary for range queries to work correctly.
    ///
    /// The second attribute is the actual key value, which is generated by concatenating the
    /// context prefix with the bytes obtained from serializing `key` using [`bcs`].
    fn build_key(&self, key: &impl Serialize) -> HashMap<String, AttributeValue> {
        let key_bytes = [
            self.key_prefix.as_slice(),
            &bcs::to_bytes(key).expect("Serialization of key failed"),
        ]
        .concat();

        [
            (
                PARTITION_ATTRIBUTE.to_owned(),
                AttributeValue::B(Blob::new(DUMMY_PARTITION_KEY)),
            ),
            (
                KEY_ATTRIBUTE.to_owned(),
                AttributeValue::B(Blob::new(key_bytes)),
            ),
        ]
        .into()
    }

    /// Retrieve a generic `Item` from the table using the provided `key` prefixed by the current
    /// context.
    ///
    /// The `Item` is deserialized using [`bcs`].
    async fn get_item<Item>(
        &mut self,
        key: &impl Serialize,
    ) -> Result<Option<Item>, DynamoDbContextError>
    where
        Item: DeserializeOwned,
    {
        let response = self
            .client
            .get_item()
            .table_name(self.table.as_ref())
            .set_key(Some(self.build_key(key)))
            .send()
            .await?;

        let item = match response.item() {
            Some(item) => item,
            None => return Ok(None),
        };
        let bytes = item
            .get(VALUE_ATTRIBUTE)
            .ok_or(DynamoDbContextError::MissingValue)?
            .as_b()
            .map_err(DynamoDbContextError::wrong_value_type)?;

        let item = bcs::from_bytes(bytes.as_ref())?;

        Ok(item)
    }
}

/// Status of a table at the creation time of a [`DynamoDbContext`] instance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableStatus {
    /// Table was created during the construction of the [`DynamoDbContext`] instance.
    New,
    /// Table already existed when the [`DynamoDbContext`] instance was created.
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

/// Errors that occur when using [`DynamoDbContext`].
#[derive(Debug, Error)]
pub enum DynamoDbContextError {
    #[error(transparent)]
    Get(#[from] Box<SdkError<aws_sdk_dynamodb::error::GetItemError>>),

    #[error("The stored value attribute is missing")]
    MissingValue,

    #[error("Value was stored as {0}, but it was expected to be stored as a binary blob")]
    WrongValueType(String),

    #[error("Failed to deserialize value")]
    ValueDeserialization(#[from] bcs::Error),
}

impl From<SdkError<aws_sdk_dynamodb::error::GetItemError>> for DynamoDbContextError {
    fn from(error: SdkError<aws_sdk_dynamodb::error::GetItemError>) -> Self {
        Box::new(error).into()
    }
}

impl DynamoDbContextError {
    /// Create a [`DynamoDbContextError::WrongValueType`] instance based on the returned value type.
    ///
    /// # Panics
    ///
    /// If the value type is in the correct type, a binary blob.
    pub fn wrong_value_type(value: &AttributeValue) -> Self {
        let type_description = match value {
            AttributeValue::B(_) => unreachable!("creating an error type for the correct type"),
            AttributeValue::Bool(_) => "a boolean",
            AttributeValue::Bs(_) => "a list of binary blobs",
            AttributeValue::L(_) => "a list",
            AttributeValue::M(_) => "a map",
            AttributeValue::N(_) => "a number",
            AttributeValue::Ns(_) => "a list of numbers",
            AttributeValue::Null(_) => "a null value",
            AttributeValue::S(_) => "a string",
            AttributeValue::Ss(_) => "a list of strings",
            _ => "an unknown type",
        }
        .to_owned();

        DynamoDbContextError::WrongValueType(type_description)
    }
}

/// Error when creating a table for a new [`DynamoDbContext`] instance.
#[derive(Debug, Error)]
pub enum CreateTableError {
    #[error(transparent)]
    CreateTable(#[from] SdkError<aws_sdk_dynamodb::error::CreateTableError>),
}

/// Error when creating a [`DynamoDbContext`] instance using a LocalStack instance.
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

/// A helper trait to add a `SdkError<CreateTableError>::is_resource_in_use_exception()` method.
trait IsResourceInUseException {
    /// Check if the error is a resource is in use exception.
    fn is_resource_in_use_exception(&self) -> bool;
}

impl IsResourceInUseException for SdkError<aws_sdk_dynamodb::error::CreateTableError> {
    fn is_resource_in_use_exception(&self) -> bool {
        matches!(
            self,
            SdkError::ServiceError {
                err: aws_sdk_dynamodb::error::CreateTableError {
                    kind: aws_sdk_dynamodb::error::CreateTableErrorKind::ResourceInUseException(_),
                    ..
                },
                ..
            }
        )
    }
}
