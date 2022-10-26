// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use linera_base::{
    error::Error,
    messages::{ApplicationId, BlockHeight, ChainDescription, ChainId},
};
use linera_execution::{
    ExecutionResult, ExecutionRuntimeContext, ExecutionStateView, Operation, OperationContext,
    RawExecutionResult, SystemExecutionState, TestExecutionRuntimeContext, WasmApplication,
};
use linera_views::{
    memory::MemoryContext,
    views::{Context, View},
};
use std::sync::Arc;

#[tokio::test]
async fn test_wasm_application() {
    let mut state = SystemExecutionState::default();
    state.description = Some(ChainDescription::Root(0));
    let mut view =
        ExecutionStateView::<MemoryContext<TestExecutionRuntimeContext>>::from_system_state(state)
            .await;
    let app_id = ApplicationId(1);
    view.context()
        .extra()
        .user_applications()
        .insert(app_id, Arc::new(WasmApplication::default()));

    let context = OperationContext {
        chain_id: ChainId::root(0),
        height: BlockHeight(0),
        index: 0,
    };
    let result = view
        .execute_operation(app_id, &context, &Operation::User(vec![1]))
        .await
        .unwrap();
    assert_eq!(
        result,
        vec![ExecutionResult::User(app_id, RawExecutionResult::default())]
    );

    // let context = QueryContext {
    // chain_id: ChainId::root(0),
    // };
    // assert_eq!(
    // view.query_application(app_id, &context, &Query::User(vec![]))
    // .await
    // .unwrap(),
    // Response::User(vec![1])
    // );
}
