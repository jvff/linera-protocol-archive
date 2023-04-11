// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions from WIT types to the types defined in the crate root.

use super::wit;
use linera_base::identifiers::SessionId;

impl From<(Vec<u8>, Vec<SessionId>)> for wit::CallResult {
    fn from((value, sessions): (Vec<u8>, Vec<SessionId>)) -> Self {
        wit::CallResult {
            value,
            sessions: sessions.into_iter().map(wit::SessionId::from).collect(),
        }
    }
}

impl From<SessionId> for wit::SessionId {
    fn from(session_id: SessionId) -> Self {
        wit::SessionId {
            application_id: session_id.application_id.into(),
            kind: session_id.kind,
            index: session_id.index,
        }
    }
}
