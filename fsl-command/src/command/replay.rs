// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-862: the `native-replay/1` request and the replay it runs. The
//! request's `request` member is FSpec FR-336's replay request mapping, read
//! by [`crate::replay_request::read`], and the replay is `fsl_replay::replay`
//! (FR-098).

use std::path::Path;

use fsl_foundation::serde_object::Object;
use serde_json::{json, Value};

use super::limits::CommandLimits;
use super::{
    check_file_count, intake_with_bytes, wire, FileCounts, NativeResult, Result, RunCause,
    RunError, RunLimits, RunResult,
};
use crate::replay_request::RequestDefect;

impl From<RequestDefect> for RunCause {
    fn from(defect: RequestDefect) -> Self {
        Self::ReplayRequest(defect)
    }
}

/// A replay that `fsl_replay::replay` refused, with the outcome document
/// FR-286 gives that refusal.
#[derive(Debug)]
pub struct ReplayFailure {
    /// FR-098's refusal.
    pub refusal: fsl_replay::ReplayRefusal,
    /// The `replay` outcome document of the refusal.
    pub outcome: fsl_replay::OutcomeDocument,
}

/// Whether `bytes` is an envelope of the `native-replay/1` format.
pub(super) fn is_replay_request(bytes: &[u8]) -> bool {
    #[derive(serde::Deserialize)]
    struct Peek {
        format: String,
    }
    serde_json::from_slice::<Peek>(bytes).is_ok_and(|peek| {
        peek.format == fsl_foundation::wire_format::WireFormat::ReplayRequest.as_str()
    })
}

/// FR-862: replay the `native-replay/1` request at `path` under the caller's
/// byte ceilings and `caller`'s settings. `replay.input_bytes`
/// bounds the replay reads; every other set stage or accounting setting
/// replaces the request's own entry for it.
///
/// # Errors
///
/// `invalid-request` naming `limits` when `caller` was built for another
/// operation, before any work; the request file's read refusals, `invalid-request` naming `/format` for a request of another
/// format, and the replay request reader's and replay's own refusals.
pub fn replay_with_limits(
    path: &Path,
    limits: RunLimits,
    caller: &CommandLimits,
) -> std::result::Result<RunResult, Box<RunError>> {
    caller
        .require(super::limits::LimitedOperation::Replay)
        .map_err(|mismatch| {
            Box::new(RunError {
                cause: RunCause::ClauseRequest {
                    member: "limits".to_owned(),
                    reason: mismatch.to_string(),
                },
                sources: crate::Sources::default(),
            })
        })?;
    super::with_request_limits(path, limits.request_bytes, |directory, bytes| {
        if let Some(format) = request_format(bytes) {
            if format != fsl_foundation::wire_format::WireFormat::ReplayRequest.as_str() {
                return Err(RunCause::FormatNotRead {
                    message: format!(
                        "invalid request: format {format} is not read by replay; replay reads native-replay/1"
                    ),
                    accepted: vec![fsl_foundation::wire_format::WireFormat::ReplayRequest.as_str()],
                });
            }
        }
        super::request_shape::admit(super::request_shape::Family::Run, bytes)?;
        run_bytes(
            directory,
            path.to_str(),
            bytes,
            limits.dependent_bytes,
            caller,
        )
    })
}

/// The string `format` of the request document `bytes`, when it has one.
fn request_format(bytes: &[u8]) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Peek {
        format: String,
    }
    serde_json::from_slice::<Peek>(bytes)
        .ok()
        .map(|peek| peek.format)
}

/// Read the `native-replay/1` request `bytes`, whose byte provision files
/// resolve against `directory` under `dependent_bytes`, and replay it.
pub(super) fn run_bytes(
    directory: &Path,
    request_path: Option<&str>,
    bytes: &[u8],
    dependent_bytes: usize,
    caller: &CommandLimits,
) -> Result<RunResult> {
    let Object(envelope): Object<wire::Envelope<'_>> = serde_json::from_slice(bytes)?;
    let limits = fsl_replay::ReplayLimits::default().with_input_bytes(caller.replay_input_bytes());
    let mut intake = intake_with_bytes(directory, FileCounts::default(), dependent_bytes)?;
    let mut files = 0_usize;
    let mut request = crate::replay_request::read(
        envelope.request.get().as_bytes(),
        limits.input_bytes,
        |path, _| {
            files = files.saturating_add(1);
            let read = check_file_count(FileCounts {
                programs: files,
                ..FileCounts::default()
            })
            .and_then(|()| {
                let remaining = intake.remaining;
                intake.file(path, remaining)
            });
            read.map_err(|cause| RunCause::ReplayBytes {
                bytes_path: path.to_owned(),
                cause: Box::new(cause),
            })
        },
        Vec::new(),
    )?;
    caller.apply_to_replay(&mut request.stage_limits, &mut request.accounting_limits);
    let result = fsl_replay::replay(request, limits);
    match result {
        Ok(result) => {
            let row = result
                .to_function_json()
                .map_err(|error| RunCause::ReplayOutput {
                    outcome: Box::new(fsl_replay::OutcomeDocument::from_replay_output_failure(
                        0, &error,
                    )),
                    error,
                })?;
            let row: Value = serde_json::from_str(&row).map_err(RunCause::Output)?;
            let outcome = fsl_replay::OutcomeDocument::from_replay_with_row(&result, row.clone());
            Ok(RunResult {
                exit_code: outcome.category().exit_code(),
                value: NativeResult::replay_document(json!({ "results": [row] })),
                outcome: Some(outcome),
                requirements: None,
                sources: crate::Sources::default(),
            })
        }
        Err(refusal) => {
            let outcome =
                fsl_replay::OutcomeDocument::from_replay_refusal_at(&refusal, request_path);
            Err(RunCause::Replay(Box::new(ReplayFailure {
                refusal,
                outcome,
            })))
        }
    }
}
