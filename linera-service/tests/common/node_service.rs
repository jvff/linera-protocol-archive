// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types to interface with a node service.

use std::ops::{Deref, DerefMut};

use linera_service::cli_wrappers;

/// A type to interface with [`cli_wrappers::NodeService`] while keeping track of some
/// extra information inside the test process.
pub struct NodeService {
    service: cli_wrappers::NodeService,
}

impl From<cli_wrappers::NodeService> for NodeService {
    fn from(service: cli_wrappers::NodeService) -> Self {
        NodeService { service }
    }
}

impl Deref for NodeService {
    type Target = cli_wrappers::NodeService;

    fn deref(&self) -> &Self::Target {
        &self.service
    }
}

impl DerefMut for NodeService {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.service
    }
}
