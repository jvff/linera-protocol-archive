// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg(target_arch = "wasm32")]

mod state;

use self::boilerplate::system_api::print_log;
use self::state::Counter;
use crate::boilerplate::system_api::WasmContext;
use async_trait::async_trait;
use linera_sdk::{
    ApplicationCallResult, CalleeContext, Contract, EffectContext, ExecutionResult,
    OperationContext, Session, SessionCallResult, SessionId,
};
use thiserror::Error;

/// Alias to the application type, so that the boilerplate module can reference it.
pub type ApplicationState = Counter<WasmContext>;

#[async_trait]
impl Contract for ApplicationState {
    type Error = Error;

    async fn initialize(
        &mut self,
        _context: &OperationContext,
        argument: &[u8],
    ) -> Result<ExecutionResult, Self::Error> {
        print_log("initialize begin".to_string());
        //        println!("argument={:?}", argument);
        //        let value : Result<u128,bcs::Error> = bcs::from_bytes(argument);
        //        println!("value={:?}", value);
        self.value.set(bcs::from_bytes(argument)?);
        Ok(ExecutionResult::default())
    }

    async fn execute_operation(
        &mut self,
        _context: &OperationContext,
        operation: &[u8],
    ) -> Result<ExecutionResult, Self::Error> {
        print_log("counter : execute_operation".to_string());
        let increment: u128 = bcs::from_bytes(operation)?;
        let mut value: u128 = *self.value.get();
        value += increment;
        self.value.set(value);
        Ok(ExecutionResult::default())
    }

    async fn execute_effect(
        &mut self,
        _context: &EffectContext,
        _effect: &[u8],
    ) -> Result<ExecutionResult, Self::Error> {
        print_log("execute_effect".to_string());
        Err(Error::EffectsNotSupported)
    }

    async fn call_application(
        &mut self,
        _context: &CalleeContext,
        argument: &[u8],
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<ApplicationCallResult, Self::Error> {
        print_log("counter : call_application".to_string());
        let increment: u128 = bcs::from_bytes(argument)?;
        let mut value = *self.value.get();
        value += increment;
        self.value.set(value);
        Ok(ApplicationCallResult {
            value: bcs::to_bytes(&value).expect("Serialization should not fail"),
            ..ApplicationCallResult::default()
        })
    }

    async fn call_session(
        &mut self,
        _context: &CalleeContext,
        _session: Session,
        _argument: &[u8],
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<SessionCallResult, Self::Error> {
        print_log("call_session".to_string());
        Err(Error::SessionsNotSupported)
    }
}

/// An error that can occur during the contract execution.
#[derive(Debug, Error)]
pub enum Error {
    /// Counter application doesn't support any cross-chain effects.
    #[error("Counter application doesn't support any cross-chain effects")]
    EffectsNotSupported,

    /// Counter application doesn't support any cross-application sessions.
    #[error("Counter application doesn't support any cross-application sessions")]
    SessionsNotSupported,

    /// Invalid serialized increment value.
    #[error("Invalid serialized increment value")]
    InvalidIncrement(#[from] bcs::Error),
}

#[path = "../boilerplate/contract/mod.rs"]
mod boilerplate;

#[cfg(test)]
mod tests {
    use super::{Counter, Error};
    use futures::FutureExt;
    use linera_sdk::{
        ApplicationCallResult, BlockHeight, CalleeContext, ChainId, Contract, EffectContext,
        EffectId, ExecutionResult, OperationContext, Session,
    };
    use webassembly_test::webassembly_test;

    #[webassembly_test]
    fn operation() {
        print_log("operation".to_string());
        let initial_value = 72_u128;
        let mut counter = create_and_initialize_counter(initial_value);

        let increment = 42_308_u128;
        let operation = bcs::to_bytes(&increment).expect("Increment value is not serializable");

        let result = counter
            .execute_operation(&dummy_operation_context(), &operation)
            .now_or_never()
            .expect("Execution of counter operation should not await anything");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ExecutionResult::default());
        assert_eq!(counter.value, initial_value + increment);
    }

    #[webassembly_test]
    fn effect() {
        print_log("effect".to_string());
        let initial_value = 72_u128;
        let mut counter = create_and_initialize_counter(initial_value);

        let result = counter
            .execute_effect(&dummy_effect_context(), &[])
            .now_or_never()
            .expect("Execution of counter operation should not await anything");

        assert!(matches!(result, Err(Error::EffectsNotSupported)));
        assert_eq!(counter.value, initial_value);
    }

    #[webassembly_test]
    fn cross_application_call() {
        print_log("cross_application_call".to_string());
        let initial_value = 2_845_u128;
        let mut counter = create_and_initialize_counter(initial_value);

        let increment = 8_u128;
        let argument = bcs::to_bytes(&increment).expect("Increment value is not serializable");

        let result = counter
            .call_application(&dummy_callee_context(), &argument, vec![])
            .now_or_never()
            .expect("Execution of counter operation should not await anything");

        let expected_value = initial_value + increment;
        let expected_result = ApplicationCallResult {
            value: bcs::to_bytes(&expected_value).expect("Expected value is not serializable"),
            create_sessions: vec![],
            execution_result: ExecutionResult::default(),
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_result);
        assert_eq!(counter.value, expected_value);
    }

    #[webassembly_test]
    fn sessions() {
        print_log("sessions".to_string());
        let initial_value = 72_u128;
        let mut counter = create_and_initialize_counter(initial_value);

        let result = counter
            .call_session(&dummy_callee_context(), Session::default(), &[], vec![])
            .now_or_never()
            .expect("Execution of counter operation should not await anything");

        assert!(matches!(result, Err(Error::SessionsNotSupported)));
        assert_eq!(counter.value, initial_value);
    }

    fn create_and_initialize_counter(initial_value: u128) -> Counter {
        print_log("create_and_initialize_counter".to_string());
        let mut counter = Counter::default();
        let initial_argument =
            bcs::to_bytes(&initial_value).expect("Initial value is not serializable");

        let result = counter
            .initialize(&dummy_operation_context(), &initial_argument)
            .now_or_never()
            .expect("Initialization of counter state should not await anything");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ExecutionResult::default());
        assert_eq!(counter.value, initial_value);

        counter
    }

    fn dummy_operation_context() -> OperationContext {
        print_log("dummy_operation_context".to_string());
        OperationContext {
            chain_id: ChainId([0; 8].into()),
            height: BlockHeight(0),
            index: 0,
        }
    }

    fn dummy_effect_context() -> EffectContext {
        print_log("dummy_effect_context".to_string());
        EffectContext {
            chain_id: ChainId([0; 8].into()),
            height: BlockHeight(0),
            effect_id: EffectId {
                chain_id: ChainId([1; 8].into()),
                height: BlockHeight(1),
                index: 1,
            },
        }
    }

    fn dummy_callee_context() -> CalleeContext {
        print_log("dummy_callee_context".to_string());
        CalleeContext {
            chain_id: ChainId([0; 8].into()),
            authenticated_caller_id: None,
        }
    }
}
