// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Contains boilerplate necessary for the contract to interface with the host runtime.
//!
//! Ideally, this code should be exported from [`linera-sdk`], but that's currently impossible due
//! to how [`wit_bindgen_guest_rust`] works. It expects concrete types to be available in its parent
//! module (which in this case is this module), so it has to exist in every contract
//! implementation.
//!
//! This should be fixable with a few changes to [`wit-bindgen`], but an alternative is to generate
//! the code with a procedural macro. For now, this module should be included by all implemented
//! contracts.

// Export the contract interface.
linera_sdk::export_contract!(Contract);

mod exported_futures;
mod system_api;

use self::exported_futures::{
    CallApplication, CallSession, ExecuteEffect, ExecuteOperation, Initialize,
};
use super::ApplicationState as Contract;

/// Mark the contract type to be exported.
impl linera_sdk::contract::Contract for Contract {
    type CallApplication = CallApplication;
    type CallSession = CallSession;
    type ExecuteEffect = ExecuteEffect;
    type ExecuteOperation = ExecuteOperation;
    type Initialize = Initialize;
}
