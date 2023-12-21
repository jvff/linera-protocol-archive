// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types for using [`linera_views`] to store application state.

#[cfg(target_arch = "wasm32")]
mod aliases;
#[cfg(target_arch = "wasm32")]
mod system_api;
#[cfg(target_arch = "wasm32")]
mod wit;

#[cfg(target_arch = "wasm32")]
pub use self::{
    aliases::{
        ByteCollectionView, ByteMapView, ByteSetView, CollectionView, CustomCollectionView,
        CustomMapView, CustomSetView, LogView, MapView, QueueView, ReadGuardedView, RegisterView,
        SetView,
    },
    system_api::ViewStorageContext,
};
pub use linera_views::{
    self,
    common::CustomSerialize,
    views::{RootView, View, ViewError},
};
