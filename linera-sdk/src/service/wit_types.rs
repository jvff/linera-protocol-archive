// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Internal module with code generated by [`wit-bindgen`](https://github.com/jvff/wit-bindgen).

#![allow(missing_docs)]

// Export the service interface.
wit_bindgen::generate!({
    path: "linera-sdk/wit",
    inline:
        "package linera:app-gen;\
        world service-entrypoints-only { export linera:app/service-entrypoints; }",
    world: "service-entrypoints-only",
    stubs,
});

pub use self::exports::linera::app::service_entrypoints::*;
