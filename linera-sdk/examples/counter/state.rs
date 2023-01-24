// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use linera_views::register_view::RegisterView;
use linera_views::common::Context;
use linera_views::views::{View, ContainerView};

/// The application state.
#[derive(ContainerView)]
pub struct Counter<C>
{
    pub value: RegisterView<C, u128>,
}

