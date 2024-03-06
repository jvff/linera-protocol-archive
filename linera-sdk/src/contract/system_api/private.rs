// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Functions and types that interface with the system API available to application contracts but
//! that shouldn't be used by applications directly.

use super::super::wit_system_api as wit;
use linera_base::identifiers::{ApplicationId, SessionId};

/// Retrieves the current application parameters.
pub fn current_application_parameters() -> Vec<u8> {
    wit::application_parameters()
}

/// Calls another application.
pub fn call_application(
    authenticated: bool,
    application: ApplicationId,
    argument: &[u8],
    forwarded_sessions: Vec<SessionId>,
) -> (Vec<u8>, Vec<SessionId>) {
    let forwarded_sessions = forwarded_sessions
        .into_iter()
        .map(wit::SessionId::from)
        .collect::<Vec<_>>();

    wit::try_call_application(
        authenticated,
        application.into(),
        argument,
        &forwarded_sessions,
    )
    .into()
}

/// Calls another application's session.
pub fn call_session(
    authenticated: bool,
    session: SessionId,
    argument: &[u8],
    forwarded_sessions: Vec<SessionId>,
) -> (Vec<u8>, Vec<SessionId>) {
    let forwarded_sessions = forwarded_sessions
        .into_iter()
        .map(wit::SessionId::from)
        .collect::<Vec<_>>();

    wit::try_call_session(authenticated, session.into(), argument, &forwarded_sessions).into()
}
