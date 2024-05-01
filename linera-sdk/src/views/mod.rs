// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types for using [`linera_views`] to store application state.

mod aliases;
mod conversions_to_wit;
#[cfg(with_testing)]
mod mock_system_api;
mod system_api;

pub use linera_views::{
    self,
    common::CustomSerialize,
    views::{RootView, View, ViewError},
};

pub use self::aliases::{
    ByteCollectionView, ByteMapView, ByteSetView, CollectionView, CustomCollectionView,
    CustomMapView, CustomSetView, LogView, MapView, QueueView, ReadGuardedView, RegisterView,
    SetView,
};
pub(crate) use self::system_api::WitInterface;
pub use self::system_api::{KeyValueStore, ViewStorageContext};
