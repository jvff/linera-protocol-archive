// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types for using [`linera_views`] to store application state.

mod conversions_from_wit;
mod system_api;

pub use self::system_api::ViewStorageContext;
use linera_views::{log_view, map_view, register_view};

// Import the views system interface.
wit_bindgen_guest_rust::import!("view_system.wit");

/// A view for storing a map between keys and values.
pub type CustomMapView<K, V> = map_view::CustomMapView<ViewStorageContext, K, V>;
/// A view for storing a sequence of elements.
pub type LogView<T> = log_view::LogView<ViewStorageContext, T>;
/// A view for storing a map between keys and values.
pub type MapView<K, V> = map_view::MapView<ViewStorageContext, K, V>;
/// A view for storing plain data types.
pub type RegisterView<T> = register_view::RegisterView<ViewStorageContext, T>;
