// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use counter::CounterAbi;
use linera_sdk::{base::WithContractAbi, Contract, ContractRuntime};

use self::state::Counter;

pub struct CounterContract {
    state: Counter,
    runtime: ContractRuntime<Self>,
}

linera_sdk::contract!(CounterContract);

impl WithContractAbi for CounterContract {
    type Abi = CounterAbi;
}

impl Contract for CounterContract {
    type State = Counter;
    type Message = ();
    type InstantiationArgument = u64;
    type Parameters = ();

    async fn new(state: Counter, runtime: ContractRuntime<Self>) -> Self {
        CounterContract { state, runtime }
    }

    fn state_mut(&mut self) -> &mut Self::State {
        &mut self.state
    }

    async fn instantiate(&mut self, value: u64) {
        // Validate that the application parameters were configured correctly.
        self.runtime.application_parameters();

        self.state.value.set(value);
    }

    async fn execute_operation(&mut self, operation: u64) -> u64 {
        let new_value = self.state.value.get() + operation;
        self.state.value.set(new_value);
        new_value
    }

    async fn execute_message(&mut self, _message: ()) {
        panic!("Counter application doesn't support any cross-chain messages");
    }
}

#[cfg(test)]
mod tests {
    use futures::FutureExt;
    use linera_sdk::{
        test::{mock_application_parameters, mock_key_value_store, test_contract_runtime},
        util::BlockingWait,
        views::{View, ViewStorageContext},
        Contract,
    };
    use webassembly_test::webassembly_test;

    use super::{Counter, CounterContract};

    #[webassembly_test]
    fn operation() {
        let initial_value = 72_u64;
        let mut counter = create_and_instantiate_counter(initial_value);

        let increment = 42_308_u64;

        let response = counter
            .execute_operation(increment)
            .now_or_never()
            .expect("Execution of counter operation should not await anything");

        let expected_value = initial_value + increment;

        assert_eq!(response, expected_value);
        assert_eq!(*counter.state.value.get(), initial_value + increment);
    }

    // TODO(#1372): Rewrite this tests once it's possible to test for panics
    // #[webassembly_test]
    // fn message() {
    // let initial_value = 72_u64;
    // let mut counter = create_and_instantiate_counter(initial_value);

    // counter
    // .execute_message(())
    // .now_or_never()
    // .expect("Execution of counter operation should not await anything");

    // assert_eq!(*counter.state.value.get(), initial_value);
    // }

    #[webassembly_test]
    fn cross_application_call() {
        let initial_value = 2_845_u64;
        let mut counter = create_and_instantiate_counter(initial_value);

        let increment = 8_u64;

        let response = counter
            .execute_operation(increment)
            .now_or_never()
            .expect("Execution of counter operation should not await anything");

        let expected_value = initial_value + increment;

        assert_eq!(response, expected_value);
        assert_eq!(*counter.state.value.get(), expected_value);
    }

    fn create_and_instantiate_counter(initial_value: u64) -> CounterContract {
        mock_key_value_store();
        mock_application_parameters(&());

        let mut contract = CounterContract {
            state: Counter::load(ViewStorageContext::default())
                .blocking_wait()
                .expect("Failed to read from mock key value store"),
            runtime: test_contract_runtime(),
        };

        contract
            .instantiate(initial_value)
            .now_or_never()
            .expect("Initialization of counter state should not await anything");

        assert_eq!(*contract.state.value.get(), initial_value);

        contract
    }
}
