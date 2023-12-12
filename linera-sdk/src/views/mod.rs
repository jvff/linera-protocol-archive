// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types for using [`linera_views`] to store application state.

mod aliases;
#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_arch = "wasm32"), path = "system_api_stubs.rs")]
mod system_api;
#[cfg(target_arch = "wasm32")]
mod system_api;
#[cfg(target_arch = "wasm32")]
mod wit;

pub(crate) use self::system_api::AppStateStore;
pub use self::system_api::ViewStorageContext;
pub use linera_views::{
    self,
    common::CustomSerialize,
    views::{RootView, View, ViewError},
};
