// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use linera_sdk::views::RegisterView;
use linera_views::views::WasmRootView;

/// The application state.
#[derive(WasmRootView)]
pub struct ReentrantCounter {
    pub value: RegisterView<u128>,
}
