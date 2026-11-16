// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-862: the `native-replay/1` request and the replay it runs. The
//! request's `request` member is FSpec FR-336's replay request mapping, read
//! by [`crate::replay_request::read`], and the replay is `fsl_replay::replay`
//! (FR-098).

use std::path::Path;

use fsl_foundation::serde_object::Object;
use serde_json::{json, Value};

use super::{
    check_file_count, intake_with_bytes, wire, FileCounts, NativeResult, Result, RunCause,
    RunResult,
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

/// Read the `native-replay/1` request `bytes`, whose byte provision files
/// resolve against `directory` under `dependent_bytes`, and replay it.
pub(super) fn run_bytes(
    directory: &Path,
    bytes: &[u8],
    dependent_bytes: usize,
) -> Result<RunResult> {
    let Object(envelope): Object<wire::Envelope<'_>> = serde_json::from_slice(bytes)?;
    let limits = fsl_replay::ReplayLimits::default();
    let mut intake = intake_with_bytes(directory, FileCounts::default(), dependent_bytes)?;
    let mut files = 0_usize;
    let request = crate::replay_request::read(
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
    let result = fsl_replay::replay(request, limits);
    let outcome = fsl_replay::OutcomeDocument::from_replay(&result);
    match result {
        Ok(result) => {
            let row = result.to_function_json().map_err(RunCause::ReplayOutput)?;
            let row: Value = serde_json::from_str(&row).map_err(RunCause::Output)?;
            Ok(RunResult {
                exit_code: outcome.category().exit_code(),
                value: NativeResult::replay_document(json!({ "results": [row] })),
                outcome: Some(outcome),
                requirements: None,
            })
        }
        Err(refusal) => Err(RunCause::Replay(Box::new(ReplayFailure {
            refusal,
            outcome,
        }))),
    }
}
