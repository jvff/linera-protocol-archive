use super::{DynamoDbStorage, TableName, TableStatus};
use crate::{
    test_utils::{list_tables, LocalStackTestContext},
    Storage,
};
use anyhow::Error;
use linera_base::{
    crypto::HashValue,
    execution::{ExecutionState, Operation},
    messages::{Block, BlockHeight, Certificate, ChainId, Epoch, Value},
};

/// Test if the table for the storage is created when needed.
#[tokio::test]
#[ignore]
async fn table_is_created() -> Result<(), Error> {
    let localstack = LocalStackTestContext::new().await?;
    let client = aws_sdk_dynamodb::Client::from_conf(localstack.dynamo_db_config());
    let table: TableName = "linera".parse().expect("Invalid table name");

    let initial_tables = list_tables(&client).await?;
    assert!(!initial_tables.contains(table.as_ref()));

    let (_storage, table_status) =
        DynamoDbStorage::from_config(localstack.dynamo_db_config(), table.clone()).await?;

    let tables = list_tables(&client).await?;
    assert!(tables.contains(table.as_ref()));
    assert_eq!(table_status, TableStatus::New);

    Ok(())
}

/// Test if two independent tables for two separate storages are created.
#[tokio::test]
#[ignore]
async fn separate_tables_are_created() -> Result<(), Error> {
    let localstack = LocalStackTestContext::new().await?;
    let client = aws_sdk_dynamodb::Client::from_conf(localstack.dynamo_db_config());
    let first_table: TableName = "first".parse().expect("Invalid table name");
    let second_table: TableName = "second".parse().expect("Invalid table name");

    let initial_tables = list_tables(&client).await?;
    assert!(!initial_tables.contains(first_table.as_ref()));
    assert!(!initial_tables.contains(second_table.as_ref()));

    let (_storage, first_table_status) =
        DynamoDbStorage::from_config(localstack.dynamo_db_config(), first_table.clone()).await?;
    let (_storage, second_table_status) =
        DynamoDbStorage::from_config(localstack.dynamo_db_config(), second_table.clone()).await?;

    let tables = list_tables(&client).await?;
    assert!(tables.contains(first_table.as_ref()));
    assert!(tables.contains(second_table.as_ref()));
    assert_eq!(first_table_status, TableStatus::New);
    assert_eq!(second_table_status, TableStatus::New);

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

    let localstack = LocalStackTestContext::new().await?;
    let mut storage = localstack.create_dynamo_db_storage().await?;

    storage.write_certificate(certificate.clone()).await?;

    let stored_certificate = storage.read_certificate(certificate.hash).await?;

    assert_eq!(certificate, stored_certificate);

    Ok(())
}
