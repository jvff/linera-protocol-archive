// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A worker to handle a single chain.

mod actor;
mod state;

pub use self::{
    actor::{ChainWorkerActor, ChainWorkerRequest},
    state::ChainWorkerState,
};
