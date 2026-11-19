// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1038 (FSpec FR-824): a `native-run/1` request's `verification_plan`
//! member. [`request`] is its strict reader and [`outcome`] the writer of the
//! `filament.verification-plan-outcome/v1` document.

pub mod outcome;
pub mod request;

use super::{Result, RunCause};
use serde_json::Value;

/// Read the `verification_plan` member of the `native-run/1` request in
/// `bytes`, when it carries one, before any stage runs.
///
/// # Errors
///
/// The reader's refusal of a malformed member.
pub(super) fn check_member(bytes: &[u8]) -> Result<()> {
    let document: Value = serde_json::from_slice(bytes).map_err(RunCause::Json)?;
    if document.get("format").and_then(Value::as_str)
        != Some(fsl_foundation::wire_format::WireFormat::RunRequest.as_str())
    {
        return Ok(());
    }
    match document
        .get("request")
        .and_then(|body| body.get("verification_plan"))
    {
        Some(member) => request::read(member, request::Route::Source).map(drop),
        None => Ok(()),
    }
}
