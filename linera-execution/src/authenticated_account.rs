// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use linera_base::{
    ensure,
    identifiers::{Account, ChainId, Owner},
};
use thiserror::Error;

use crate::{runtime::ApplicationStatus, OperationContext};

/// An account that has been successfully authenticated to manage tokens.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AuthenticatedAccount {
    /// The chain where this account is located.
    chain_id: ChainId,
    /// The owner of the account.
    owner: AuthenticatedAccountOwner,
}

impl AuthenticatedAccount {
    /// Creates a new [`AuthenticatedAccount`] from an [`Account`]'s elements, if the `owner` can
    /// be authenticated by the system application's [`OperationContext`].
    pub(crate) fn new_in_system_application(
        context: &OperationContext,
        chain_id: ChainId,
        owner: Owner,
    ) -> Result<Self, UnauthorizedError> {
        Ok(AuthenticatedAccount {
            chain_id,
            owner: AuthenticatedAccountOwner::new_in_system_application(context, Some(owner))?,
        })
    }

    /// Creates a new [`AuthenticatedAccount`] from an [`Account`], if its `owner` can be
    /// authenticated by the user application runtime's [`ApplicationStatus`].
    pub(crate) fn new_in_user_application(
        application_status: &ApplicationStatus,
        account: Account,
    ) -> Result<Self, UnauthorizedError> {
        Ok(AuthenticatedAccount {
            chain_id: account.chain_id,
            owner: AuthenticatedAccountOwner::new_in_user_application(
                application_status,
                account.owner,
            )?,
        })
    }

    /// Returns the [`ChainId`] of the chain that has this account.
    pub fn chain_id(self) -> ChainId {
        self.chain_id
    }

    /// Returns the [`AuthenticatedAccountOwner`] of this account.
    pub fn owner(self) -> AuthenticatedAccountOwner {
        self.owner
    }

    /// Returns the [`Account`] without authentication.
    pub fn without_authentication(self) -> Account {
        Account {
            chain_id: self.chain_id,
            owner: self.owner.without_authentication(),
        }
    }
}

/// An account owner that has been successfully authenticated to manage tokens.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AuthenticatedAccountOwner {
    /// Shared ownership among all chain owners.
    Chain,
    /// A user represented by a single public key.
    User(Owner),
    // TODO(#2608): Support application accounts
}

impl AuthenticatedAccountOwner {
    /// Creates a new [`AuthenticatedAccountOwner`], if the `owner` can be authenticated by the
    /// system application's [`OperationContext`].
    pub(crate) fn new_in_system_application(
        context: &OperationContext,
        owner: Option<Owner>,
    ) -> Result<Self, UnauthorizedError> {
        Self::new_internal(context.authenticated_signer, owner)
    }

    /// Creates a new [`AuthenticatedAccountOwner`], if the `owner` can be authenticated by the
    /// user application runtime's [`ApplicationStatus`].
    pub(crate) fn new_in_user_application(
        application_status: &ApplicationStatus,
        owner: Option<Owner>,
    ) -> Result<Self, UnauthorizedError> {
        Self::new_internal(application_status.authenticated_signer(), owner)
    }

    /// Creates a new [`AuthenticatedAccountOwner`], if the `owner` can be authenticated by the
    /// `authenticated_signer`.
    fn new_internal(
        authenticated_signer: Option<Owner>,
        owner: Option<Owner>,
    ) -> Result<Self, UnauthorizedError> {
        if owner.is_some() {
            ensure!(authenticated_signer == owner, UnauthorizedError::new(owner));
        }

        match owner {
            Some(owner) => Ok(AuthenticatedAccountOwner::User(owner)),
            None => Ok(AuthenticatedAccountOwner::Chain),
        }
    }

    /// Returns the account without authentication.
    pub fn without_authentication(self) -> Option<Owner> {
        match self {
            AuthenticatedAccountOwner::Chain => None,
            AuthenticatedAccountOwner::User(owner) => Some(owner),
        }
    }
}

/// A failure to authenticate usage of an account.
#[derive(Clone, Debug, Error)]
#[error("Unauthorized to perform movement of tokens owned by {account_owner}")]
pub struct UnauthorizedError {
    account_owner: String,
}

impl UnauthorizedError {
    /// Creates a new [`UnauthorizedError`] with a readable [`String`] to represent the
    /// account.
    pub fn new(account: Option<Owner>) -> Self {
        let account_owner = account
            .as_ref()
            .map(Owner::to_string)
            .unwrap_or_else(|| "the chain owners".to_owned());

        UnauthorizedError { account_owner }
    }
}
