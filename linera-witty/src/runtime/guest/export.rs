// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The hard-coded items imported from the host.

use std::any::TypeId;

/// Things that are exported to the guest.
///
/// These are made available as exports from the [`Guest`] runtime.
pub enum Export {
    Memory,
    Function { pointer: *mut (), signature: TypeId },
}
