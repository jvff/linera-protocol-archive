// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{chain_guards::ChainGuards, DbStore, DbStoreInner};
use linera_execution::WasmRuntime;
use linera_views::memory::{create_memory_client_stream_queries, MemoryClient};
use std::sync::Arc;
#[cfg(any(test, feature = "test"))]
use {crate::TestClock, linera_views::test_utils::DelayedStoreClient, std::time::Duration};

type MemoryStore = DbStoreInner<MemoryClient>;

impl MemoryStore {
    pub fn new(wasm_runtime: Option<WasmRuntime>, max_stream_queries: usize) -> Self {
        let client = create_memory_client_stream_queries(max_stream_queries);
        Self {
            client,
            guards: ChainGuards::default(),
            user_applications: Arc::default(),
            wasm_runtime,
        }
    }
}

pub type MemoryStoreClient<C> = DbStore<MemoryClient, C>;

#[cfg(any(test, feature = "test"))]
pub type DelayedMemoryStoreClient<C> = DbStore<DelayedStoreClient<MemoryClient>, C>;

#[cfg(any(test, feature = "test"))]
impl MemoryStoreClient<TestClock> {
    pub async fn make_test_store(wasm_runtime: Option<WasmRuntime>) -> Self {
        let clock = TestClock::new();
        let max_stream_queries = linera_views::memory::TEST_MEMORY_MAX_STREAM_QUERIES;
        MemoryStoreClient::new(wasm_runtime, max_stream_queries, clock)
    }

    pub async fn make_delayed_test_store(
        delay: Duration,
        wasm_runtime: Option<WasmRuntime>,
    ) -> DelayedMemoryStoreClient<TestClock> {
        DbStore {
            client: Arc::new(DbStoreInner {
                client: DelayedStoreClient::new(
                    create_memory_client_stream_queries(
                        linera_views::memory::TEST_MEMORY_MAX_STREAM_QUERIES,
                    ),
                    delay,
                ),
                guards: ChainGuards::default(),
                user_applications: Arc::default(),
                wasm_runtime,
            }),
            clock: TestClock::new(),
        }
    }
}

impl<C> MemoryStoreClient<C> {
    pub fn new(wasm_runtime: Option<WasmRuntime>, max_stream_queries: usize, clock: C) -> Self {
        Self {
            client: Arc::new(MemoryStore::new(wasm_runtime, max_stream_queries)),
            clock,
        }
    }
}
