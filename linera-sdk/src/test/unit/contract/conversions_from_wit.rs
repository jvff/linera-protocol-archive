// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions from WIT types to the types defined in the crate root.

use super::wit;
use linera_base::identifiers::SessionId;
use linera_views::batch::WriteOperation;

impl From<wit::WriteOperation> for WriteOperation {
    fn from(operation: wit::WriteOperation) -> Self {
        match operation {
            wit::WriteOperation::Delete(key) => WriteOperation::Delete { key },
            wit::WriteOperation::Deleteprefix(key_prefix) => {
                WriteOperation::DeletePrefix { key_prefix }
            }
            wit::WriteOperation::Put((key, value)) => WriteOperation::Put { key, value },
        }
    }
}

impl From<wit::SessionId> for SessionId {
    fn from(session_id: wit::SessionId) -> Self {
        SessionId {
            application_id: session_id.application_id.into(),
            kind: session_id.kind,
            index: session_id.index,
        }
    }
}
