// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    crypto::BcsSignable,
    ensure,
    error::Error,
    messages::*,
    system::{SystemEffect, SystemExecutionState},
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub static SYSTEM: ApplicationId = ApplicationId(0);

pub static USER_APPLICATIONS: Lazy<
    Mutex<HashMap<ApplicationId, Arc<dyn UserApplication + Send + Sync + 'static>>>,
> = Lazy::new(|| {
    let m = HashMap::new();
    Mutex::new(m)
});

pub trait UserApplication {
    fn apply_operation(
        &self,
        context: &OperationContext,
        state: &mut Vec<u8>,
        operation: &[u8],
    ) -> Result<RawApplicationResult<Vec<u8>>, Error>;

    fn apply_effect(
        &self,
        context: &EffectContext,
        state: &mut Vec<u8>,
        operation: &[u8],
    ) -> Result<RawApplicationResult<Vec<u8>>, Error>;
}

#[derive(Debug, Clone)]
pub struct OperationContext {
    pub chain_id: ChainId,
    pub height: BlockHeight,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct EffectContext {
    pub chain_id: ChainId,
    pub height: BlockHeight,
    pub effect_id: EffectId,
}

impl From<OperationContext> for EffectId {
    fn from(context: OperationContext) -> Self {
        Self {
            chain_id: context.chain_id,
            height: context.height,
            index: context.index,
        }
    }
}

#[derive(Debug)]
pub struct RawApplicationResult<Effect> {
    pub effects: Vec<(Destination, Effect)>,
    pub subscribe: Option<(String, ChainId)>,
    pub unsubscribe: Option<(String, ChainId)>,
}

#[derive(Debug)]
pub enum ApplicationResult {
    System(RawApplicationResult<SystemEffect>),
    User(RawApplicationResult<Vec<u8>>),
}

impl<Effect> Default for RawApplicationResult<Effect> {
    fn default() -> Self {
        Self {
            effects: Vec::new(),
            subscribe: None,
            unsubscribe: None,
        }
    }
}

/// The authentication execution state of all applications.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "test"), derive(Eq, PartialEq))]
pub struct ExecutionState {
    /// System application.
    pub system: SystemExecutionState,
    /// User applications.
    pub users: HashMap<ApplicationId, Vec<u8>>,
}

impl ExecutionState {
    fn get_user_application(
        application_id: ApplicationId,
    ) -> Result<Arc<dyn UserApplication + Send + Sync + 'static>, Error> {
        let applications = USER_APPLICATIONS.lock().unwrap();
        Ok(applications
            .get(&application_id)
            .ok_or(Error::UnknownApplication)?
            .clone())
    }

    pub fn apply_operation(
        &mut self,
        application_id: ApplicationId,
        context: &OperationContext,
        operation: &Operation,
    ) -> Result<ApplicationResult, Error> {
        if application_id == SYSTEM {
            match operation {
                Operation::System(op) => {
                    let result = self.system.apply_operation(context, op)?;
                    Ok(ApplicationResult::System(result))
                }
                _ => Err(Error::InvalidOperation),
            }
        } else {
            let application = Self::get_user_application(application_id)?;
            let state = self.users.entry(application_id).or_default();
            match operation {
                Operation::System(_) => Err(Error::InvalidOperation),
                Operation::User(operation) => {
                    let result = application.apply_operation(context, state, operation)?;
                    Ok(ApplicationResult::User(result))
                }
            }
        }
    }

    pub fn apply_effect(
        &mut self,
        application_id: ApplicationId,
        context: &EffectContext,
        effect: &Effect,
    ) -> Result<ApplicationResult, Error> {
        if application_id == SYSTEM {
            match effect {
                Effect::System(effect) => {
                    let result = self.system.apply_effect(context, effect)?;
                    Ok(ApplicationResult::System(result))
                }
                _ => Err(Error::InvalidEffect),
            }
        } else {
            let application = Self::get_user_application(application_id)?;
            let state = self.users.entry(application_id).or_default();
            match effect {
                Effect::System(_) => Err(Error::InvalidEffect),
                Effect::User(effect) => {
                    let result = application.apply_effect(context, state, effect)?;
                    Ok(ApplicationResult::User(result))
                }
            }
        }
    }
}

impl From<SystemExecutionState> for ExecutionState {
    fn from(system: SystemExecutionState) -> Self {
        Self {
            system,
            users: HashMap::new(),
        }
    }
}

impl BcsSignable for ExecutionState {}

#[derive(Clone, Default)]
pub struct ApplicationRegistry {
    applications: HashMap<ApplicationId, Arc<dyn UserApplication + Send + Sync + 'static>>,
}

impl ApplicationRegistry {
    pub fn deploy_application(
        &mut self,
        id: ApplicationId,
        application_code: usize,
    ) -> Result<(), Error> {
        ensure!(
            !self.applications.contains_key(&id),
            Error::ApplicationRedeployment { id }
        );
        let application = match application_code {
            _ => return Err(Error::UnknownApplication),
        };
        self.applications.insert(id, application);
    }

    pub fn get_application(
        &self,
        id: ApplicationId,
    ) -> Option<Arc<dyn UserApplication + Send + Sync + 'static>> {
        self.applications.get(&id).cloned()
    }
}
