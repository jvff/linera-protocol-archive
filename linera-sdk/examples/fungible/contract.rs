// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg(target_arch = "wasm32")]

mod state;

use self::state::{AccountOwner, ApplicationState, FungibleToken};
use async_trait::async_trait;
use ed25519_dalek::{PublicKey, Signature};
use linera_sdk::{
    ApplicationCallResult, CalleeContext, ChainId, Contract, EffectContext, ExecutionResult,
    OperationContext, Session, SessionCallResult, SessionId,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[async_trait]
impl Contract for FungibleToken {
    type Error = Error;

    async fn initialize(
        &mut self,
        _context: &OperationContext,
        argument: &[u8],
    ) -> Result<ExecutionResult, Self::Error> {
        *self = bcs::from_bytes(argument).map_err(Error::InvalidInitialState)?;
        Ok(ExecutionResult::default())
    }

    async fn execute_operation(
        &mut self,
        _context: &OperationContext,
        operation: &[u8],
    ) -> Result<ExecutionResult, Self::Error> {
        let signed_transfer: SignedTransfer =
            bcs::from_bytes(operation).map_err(Error::InvalidOperation)?;
        let (source, transfer) = signed_transfer.check()?;

        self.debit(source, transfer.amount)?;

        Ok(self.finish_transfer(transfer))
    }

    async fn execute_effect(
        &mut self,
        _context: &EffectContext,
        effect: &[u8],
    ) -> Result<ExecutionResult, Self::Error> {
        let credit: Credit = bcs::from_bytes(effect).map_err(Error::InvalidEffect)?;

        self.credit(credit.destination, credit.amount);

        Ok(ExecutionResult::default())
    }

    async fn call_application(
        &mut self,
        context: &CalleeContext,
        argument: &[u8],
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<ApplicationCallResult, Self::Error> {
        let transfer: ApplicationTransfer =
            bcs::from_bytes(argument).map_err(Error::InvalidArgument)?;
        let caller = context
            .authenticated_caller_id
            .ok_or(Error::MissingSourceApplication)?;
        let source = AccountOwner::Application(caller);

        self.debit(source, transfer.amount())?;

        Ok(self.finish_application_transfer(transfer))
    }

    async fn call_session(
        &mut self,
        _context: &CalleeContext,
        _session: Session,
        _argument: &[u8],
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<SessionCallResult, Self::Error> {
        todo!();
    }
}

impl FungibleToken {
    /// Credit an account or forward it into a session or another micro-chain.
    fn finish_application_transfer(
        &mut self,
        application_transfer: ApplicationTransfer,
    ) -> ApplicationCallResult {
        let mut result = ApplicationCallResult::default();

        match application_transfer {
            ApplicationTransfer::Static(transfer) => {
                result.execution_result = self.finish_transfer(transfer);
            }
            ApplicationTransfer::Dynamic(amount) => {
                result.create_sessions.push(Session {
                    kind: 0,
                    data: bcs::to_bytes(&amount).expect("Serializing a `u128` should not fail"),
                });
            }
        }

        result
    }

    /// Credit an account or forward it to another micro-chain.
    fn finish_transfer(&mut self, transfer: Transfer) -> ExecutionResult {
        if transfer.destination_chain == Self::current_chain_id() {
            self.credit(transfer.destination_account, transfer.amount);
            ExecutionResult::default()
        } else {
            ExecutionResult::default()
                .with_effect(transfer.destination_chain, &Credit::from(transfer))
        }
    }
}

/// The transfer operation.
#[derive(Deserialize, Serialize)]
pub struct SignedTransfer {
    source: PublicKey,
    signature: Signature,
    transfer: Transfer,
}

/// A cross-application transfer request.
#[derive(Deserialize, Serialize)]
pub enum ApplicationTransfer {
    /// A static transfer to a specific destination.
    Static(Transfer),
    /// A dynamic transfer into a session, that can then be credited to destinations later.
    Dynamic(u128),
}

impl ApplicationTransfer {
    /// The amount of tokens to be transfered.
    pub fn amount(&self) -> u128 {
        match self {
            ApplicationTransfer::Static(transfer) => transfer.amount,
            ApplicationTransfer::Dynamic(amount) => *amount,
        }
    }
}

/// A transfer payload.
#[derive(Deserialize, Serialize)]
pub struct Transfer {
    destination_account: AccountOwner,
    destination_chain: ChainId,
    amount: u128,
}

impl SignedTransfer {
    /// Check that the [`SignedTransfer`] is correctly signed.
    ///
    /// If correctly signed, returns the source of the transfer and the [`Transfer`].
    pub fn check(self) -> Result<(AccountOwner, Transfer), Error> {
        let transfer =
            bcs::to_bytes(&self.transfer).expect("Serialization of transfer should not fail");

        self.source.verify_strict(&transfer, &self.signature)?;

        Ok((AccountOwner::Key(self.source), self.transfer))
    }
}

/// The credit effect.
#[derive(Deserialize, Serialize)]
pub struct Credit {
    destination: AccountOwner,
    amount: u128,
}

impl From<Transfer> for Credit {
    fn from(transfer: Transfer) -> Self {
        Credit {
            destination: transfer.destination_account,
            amount: transfer.amount,
        }
    }
}

/// An error that can occur during the contract execution.
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid serialized initial state.
    #[error("Serialized initial state is invalid")]
    InvalidInitialState(#[source] bcs::Error),

    /// Invalid serialized [`SignedTransfer`].
    #[error("Operation is not a valid serialized signed transfer")]
    InvalidOperation(#[source] bcs::Error),

    /// Incorrect signature for transfer.
    #[error("Operation does not have a valid signature")]
    IncorrectSignature(#[from] ed25519_dalek::SignatureError),

    /// Invalid serialized [`Credit`].
    #[error("Effect is not a valid serialized credit operation")]
    InvalidEffect(#[source] bcs::Error),

    /// Cross-application call without a source application ID.
    #[error("Applications must identify themselves to perform transfers")]
    MissingSourceApplication,

    /// Invalid serialized [`Transfer`].
    #[error("Cross-application call argument is not a valid serialized transfer")]
    InvalidArgument(#[source] bcs::Error),

    /// Insufficient balance in source account.
    #[error("Source account does not have sufficient balance for transfer")]
    InsufficientBalance(#[from] state::InsufficientBalanceError),
}

#[path = "../boilerplate/contract/mod.rs"]
mod boilerplate;
