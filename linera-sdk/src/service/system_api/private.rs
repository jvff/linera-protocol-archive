// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Functions and types that interface with the system API available to application services but
//! that shouldn't be used by applications directly.

#![allow(missing_docs)]

pub use self::linera::app::service_system_api as wit;
use crate::{util::yield_once, views::ViewStorageContext};
use linera_base::identifiers::ApplicationId;
use linera_views::views::View;
use serde::de::DeserializeOwned;

// Import the system interface.
wit_bindgen::generate!({
    path: "linera-sdk/wit",
    inline:
        "package linera:app-gen;\
        world service-system-api-only { import linera:app/service-system-api; }",
    world: "service-system-api-only",
});

/// Loads the application state, without locking it for writes.
pub async fn load<State>() -> State
where
    State: Default + DeserializeOwned,
{
    let promise = wit::load_new();
    yield_once().await;
    let bytes = wit::load_wait(promise).expect("Failed to load application state");
    if bytes.is_empty() {
        State::default()
    } else {
        bcs::from_bytes(&bytes).expect("Invalid application state")
    }
}

/// Loads the application state (and locks it for writes).
pub async fn lock_and_load_view<State: View<ViewStorageContext>>() -> State {
    let promise = wit::lock_new();
    yield_once().await;
    wit::lock_wait(promise).expect("Failed to lock application state");
    load_view_using::<State>().await
}

/// Unlocks the service state previously loaded.
pub async fn unlock_view() {
    let promise = wit::unlock_new();
    yield_once().await;
    wit::unlock_wait(promise).expect("Failed to unlock application state");
}

/// Helper function to load the service state or create a new one if it doesn't exist.
pub async fn load_view_using<State: View<ViewStorageContext>>() -> State {
    let context = ViewStorageContext::default();
    State::load(context)
        .await
        .expect("Failed to load application state")
}

/// Retrieves the current application parameters.
pub fn current_application_parameters() -> Vec<u8> {
    wit::get_application_parameters()
}

/// Queries another application.
pub async fn query_application(
    application: ApplicationId,
    argument: &[u8],
) -> Result<Vec<u8>, String> {
    let promise = wit::try_query_application_new(application.into(), argument);
    yield_once().await;
    wit::try_query_application_wait(promise)
}
