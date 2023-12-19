// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Generator of WIT files representing the interface between Linera applications and nodes.

use linera_execution::{
    ContractEntrypoints, ContractSystemApi, ServiceEntrypoints, ServiceSystemApi, ViewSystemApi,
};
use linera_sdk::MockSystemApi;
use linera_witty::{
    wit_generation::{WitInterfaceWriter, WitWorldWriter},
    MockInstance,
};
use std::{io, path::Path};

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let base_directory = Path::new("linera-sdk/wit");

    WitInterfaceWriter::new::<ContractEntrypoints<MockInstance<()>>>()
        .write_to_file(base_directory.join("contract-entrypoints.wit"))
        .await?;
    WitInterfaceWriter::new::<ServiceEntrypoints<MockInstance<()>>>()
        .write_to_file(base_directory.join("service-entrypoints.wit"))
        .await?;
    WitInterfaceWriter::new::<MockSystemApi<MockInstance<()>>>()
        .write_to_file(base_directory.join("mock-system-api.wit"))
        .await?;

    WitInterfaceWriter::new::<ContractSystemApi<MockInstance<_>>>()
        .write_to_file(base_directory.join("contract-system-api.wit"))
        .await?;
    WitInterfaceWriter::new::<ServiceSystemApi<MockInstance<_>>>()
        .write_to_file(base_directory.join("service-system-api.wit"))
        .await?;
    WitInterfaceWriter::new::<ViewSystemApi<MockInstance<_>>>()
        .write_to_file(base_directory.join("view-system-api.wit"))
        .await?;

    WitWorldWriter::new("linera:app", "contract")
        .export::<ContractEntrypoints<MockInstance<()>>>()
        .import::<ContractSystemApi<MockInstance<_>>>()
        .import::<ViewSystemApi<MockInstance<_>>>()
        .write_to_file(base_directory.join("contract.wit"))
        .await?;
    WitWorldWriter::new("linera:app", "service")
        .export::<ServiceEntrypoints<MockInstance<()>>>()
        .import::<ServiceSystemApi<MockInstance<_>>>()
        .import::<ViewSystemApi<MockInstance<_>>>()
        .write_to_file(base_directory.join("service.wit"))
        .await?;
    WitWorldWriter::new("linera:app", "unit-tests")
        .export::<MockSystemApi<MockInstance<()>>>()
        .import::<ContractSystemApi<MockInstance<_>>>()
        .import::<ServiceSystemApi<MockInstance<_>>>()
        .import::<ViewSystemApi<MockInstance<_>>>()
        .write_to_file(base_directory.join("unit-tests.wit"))
        .await?;

    Ok(())
}
