// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use linera_sdk::{base::WithContractAbi, Contract, ContractRuntime, EmptyState};

pub struct LlmContract {
    state: EmptyState,
}

linera_sdk::contract!(LlmContract);

impl WithContractAbi for LlmContract {
    type Abi = llm::LlmAbi;
}

impl Contract for LlmContract {
    type State = EmptyState;
    type Message = ();
    type InstantiationArgument = ();
    type Parameters = ();

    async fn new(state: Self::State, _runtime: ContractRuntime<Self>) -> Self {
        LlmContract { state }
    }

    fn state_mut(&mut self) -> &mut Self::State {
        &mut self.state
    }

    async fn instantiate(&mut self, _value: ()) {}

    async fn execute_operation(&mut self, _operation: ()) -> Self::Response {}

    async fn execute_message(&mut self, _message: ()) {
        panic!("Llm application doesn't support any cross-chain messages");
    }
}
