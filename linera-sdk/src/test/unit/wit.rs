// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Internal module with code generated by [`wit-bindgen`](https://github.com/jvff/wit-bindgen).

#![allow(missing_docs)]

wit_bindgen::generate!({
    world: "unit-tests",
    exports: {
        "linera:app/mock-system-api": super::MockSystemApi,
    },
});

pub(super) use self::exports::linera::app::mock_system_api::Guest;
