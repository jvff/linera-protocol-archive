use super::{DynamoDbStorage, TableName, TableStatus};
use crate::test_utils::{list_tables, LocalStackTestContext};
use anyhow::Error;

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
