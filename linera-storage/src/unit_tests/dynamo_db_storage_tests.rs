use super::{DynamoDbStorage, TableName, TableStatus};
use crate::{
    test_utils::{list_tables, LocalStackTestContext},
    Storage,
};
use anyhow::Error;
use linera_base::{
    chain::ChainState,
    crypto::HashValue,
    execution::{ExecutionState, Operation},
    messages::{Block, BlockHeight, Certificate, ChainDescription, ChainId, Epoch, Value},
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

/// Test if a table is reused when a second storage instance is created later.
#[tokio::test]
#[ignore]
async fn table_is_reused() -> Result<(), Error> {
    let chain_id = ChainId::root(100);
    let chain_state = ChainState {
        next_block_height: BlockHeight(248),
        ..ChainState::new(chain_id)
    };

    let localstack = LocalStackTestContext::new().await?;
    let table: TableName = "table".parse().expect("Invalid table name");

    let (mut storage, first_table_status) =
        DynamoDbStorage::from_config(localstack.dynamo_db_config(), table.clone()).await?;
    assert_eq!(first_table_status, TableStatus::New);

    storage.write_chain(chain_state.clone()).await?;

    let (mut storage, second_table_status) =
        DynamoDbStorage::from_config(localstack.dynamo_db_config(), table.clone()).await?;
    assert_eq!(second_table_status, TableStatus::Existing);

    let stored_chain_state = storage.read_chain_or_default(chain_id).await?;
    assert_eq!(stored_chain_state, chain_state);

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

/// Test if retrieving inexistent certificates fails.
#[tokio::test]
#[ignore]
async fn retrieval_of_inexistent_certificate() -> Result<(), Error> {
    let certificate_hash = HashValue::new(&ChainDescription::Root(123));

    let localstack = LocalStackTestContext::new().await?;
    let mut storage = localstack.create_dynamo_db_storage().await?;

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
    let mut storage = localstack.create_dynamo_db_storage().await?;

    storage.write_chain(chain_state.clone()).await?;

    let stored_chain_state = storage
        .read_chain_or_default(chain_state.state.chain_id)
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
    let mut storage = localstack.create_dynamo_db_storage().await?;

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
    let mut storage = localstack.create_dynamo_db_storage().await?;

    storage.write_chain(chain_state).await?;
    storage.remove_chain(chain_id).await?;

    let retrieved_chain_state = storage.read_chain_or_default(chain_id).await?;
    let expected_chain_state = ChainState::new(chain_id);

    assert_eq!(retrieved_chain_state, expected_chain_state);

    Ok(())
}
