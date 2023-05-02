// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use linera_sdk::views::RegisterView;
use linera_views::views::WasmView;

/// The application state.
#[derive(WasmView)]
pub struct ReentrantCounter {
    pub value: RegisterView<u128>,
}
