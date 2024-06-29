// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::thread;

use linera_execution::{ExecutionRequest, QueryContext, ServiceRuntimeRequest, ServiceSyncRuntime};

/// Spawns a thread running the [`ServiceSyncRuntime`] actor.
///
/// Returns the endpoints to communicate with the actor.
pub fn spawn_service_runtime_actor(
    context: QueryContext,
) -> (
    futures::channel::mpsc::UnboundedReceiver<ExecutionRequest>,
    std::sync::mpsc::Sender<ServiceRuntimeRequest>,
) {
    let (execution_state_sender, execution_state_receiver) = futures::channel::mpsc::unbounded();
    let (request_sender, request_receiver) = std::sync::mpsc::channel();

    thread::spawn(move || {
        ServiceSyncRuntime::new(execution_state_sender, context).run(request_receiver)
    });

    (execution_state_receiver, request_sender)
}
