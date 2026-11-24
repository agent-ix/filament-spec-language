// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-563: run the parity gate against every non-interpreter backend linked
//! into the workspace.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use filament_eval::ExecutionBackendChoice;
use filament_exact::Cancel;
use fsl_conformance::{run_parity, BackendEntry, ParityInputs, ParityLimits, ParityRefusal};
use crate::error::{Error, Result};
use crate::Outcome;

const USAGE: &str = "usage: cargo xtask parity [--corpus <dir>]";

/// Run the parity gate with the pinned complete-V1 fixture unless the caller
/// supplies a corpus directory.
pub fn run(operands: &[OsString]) -> Result<Outcome> {
    let corpus = corpus_arg(operands)?;
    if let Some(corpus) = corpus {
        run_with_corpus(&corpus)
    } else {
        let published = crate::published_fspec::directory("proposals/");
        run_with_corpus(
            &published
                .path()
                .join("proposals/conformance-corpus-v1/fixtures"),
        )
    }
}

/// Run against an explicitly selected complete-V1 corpus directory.
pub fn run_with_corpus(corpus: &Path) -> Result<Outcome> {
    let report = report_with_corpus(corpus)?;
    let code = report.exit_code();
    Ok(Outcome {
        stdout: format!("{report:#?}"),
        code,
    })
}

/// Return the typed parity report for a corpus directory.
pub fn report_with_corpus(corpus: &Path) -> Result<fsl_conformance::ParityReport> {
    let inputs = ParityInputs::new(corpus);
    let backends: [BackendEntry<'_>; 1] = [(
        ExecutionBackendChoice::Jit,
        &fsl_jit::Jit,
        Some(fsl_jit::coverage()),
    )];
    run_parity(&backends, &inputs, &ParityLimits::default(), &Cancel::new())
        .map_err(|refusal| Error::QualificationRefusal(refusal_message(refusal)))
}

fn corpus_arg(operands: &[OsString]) -> Result<Option<PathBuf>> {
    let mut corpus = None;
    let mut index = 0;
    while index < operands.len() {
        let option = operands[index].to_str().ok_or(Error::Usage(USAGE))?;
        if option != "--corpus" {
            return Err(Error::QualificationRefusal(format!(
                "unknown option {option}"
            )));
        }
        if corpus.is_some() {
            return Err(Error::QualificationRefusal(
                "duplicate option --corpus".to_owned(),
            ));
        }
        index += 1;
        let path = operands.get(index).ok_or(Error::Usage(USAGE))?;
        corpus = Some(PathBuf::from(path));
        index += 1;
    }
    Ok(corpus)
}

fn refusal_message(refusal: ParityRefusal) -> String {
    match refusal {
        ParityRefusal::EmptyScope => "empty-scope".to_owned(),
        ParityRefusal::UnreadableCorpus(path) => format!("unreadable-corpus: {}", path.display()),
        ParityRefusal::Cancelled => "cancelled".to_owned(),
        ParityRefusal::ReferencePrepareFailed(failure) => {
            format!("reference-prepare-failed: {failure:?}")
        }
        ParityRefusal::ComparisonIncomplete(incomplete) => {
            format!("comparison-incomplete: {incomplete:?}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// TC-1387 FR-563-AC-8: parity rejects an unknown CLI option by name.
    #[trace("TC-1387", "FR-563-AC-8")]
    #[test]
    fn unknown_option_is_a_named_refusal() {
        let error = match run(&[OsString::from("--no-such-option")]) {
            Ok(_) => panic!("unknown option is refused"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("--no-such-option"));
    }
}
