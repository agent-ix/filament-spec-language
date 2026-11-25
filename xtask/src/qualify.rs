// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-567: the `qualify` gate's verdict covers the final path the driver
//! calls. The gate runs three parts in every run: the FR-350 `corpus`, the
//! FR-565 `lifecycle-matrix` and the FR-563 `parity` over the non-interpreter
//! backends the workspace links.

use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

use fsl_conformance::{lifecycle_matrix, run_case, MatrixCell};
use serde::Serialize;

use crate::error::Result;
use crate::{qualification, Outcome};

/// How a part of the gate settled.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PartResult {
    /// The part passed.
    Passed,
    /// The part failed; its `failures` name what.
    Failed,
    /// `parity` only: the workspace links no non-interpreter backend, which
    /// passes.
    NoBackend,
}

impl PartResult {
    /// The result's report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::NoBackend => "no-backend",
        }
    }
}

/// What one part of the gate settled. The report lists the parts as
/// `corpus`, `lifecycle-matrix`, `parity`.
#[derive(Debug, Serialize)]
pub struct PartReport {
    /// The part's name: `corpus`, `lifecycle-matrix` or `parity`.
    pub part: &'static str,
    /// How the part settled.
    pub result: PartResult,
    /// What failed, as `lifecycle-matrix/<verb>/<kind>` or `parity/<backend>`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<String>,
}

impl PartReport {
    /// Whether the part lets the run be `qualified`: `no-backend` is a pass.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.result != PartResult::Failed
    }
}

/// The inputs of [`run`].
pub struct QualifyInputs<'a> {
    /// The FSpec checkout the corpus schema is read from.
    pub fspec: PathBuf,
    /// The `qualify` command's operands.
    pub operands: Vec<OsString>,
    /// A replacement matrix cell set. A test may make the `lifecycle-matrix`
    /// part fail with it; the command line offers none.
    #[cfg(feature = "test-support")]
    pub matrix: Option<Vec<MatrixCell>>,
    /// A test-only parity replacement; the CLI cannot select an oracle or
    /// backend.
    #[cfg(feature = "test-support")]
    pub parity: Option<ParityTestOverride<'a>>,
    /// A replacement generated-unit count for the `parity` part. A test that
    /// asserts nothing about generated coverage sets a small one; the command
    /// line offers none.
    #[cfg(feature = "test-support")]
    pub parity_count: Option<u64>,
    /// Keep the test-support lifetime well-formed in normal builds.
    #[cfg(not(feature = "test-support"))]
    pub _marker: std::marker::PhantomData<&'a ()>,
}

/// Test-only substitution for the parity oracle and supplied backends.
#[cfg(feature = "test-support")]
pub struct ParityTestOverride<'a> {
    /// The substituted reference evaluator.
    pub reference: &'a dyn fsl_eval::execution::SuppliedBackend,
    /// The non-interpreter backends to qualify.
    pub backends: Vec<fsl_conformance::BackendEntry<'a>>,
}

/// The `lifecycle-matrix` part: every `Case` gives a document or item whose
/// category and exit code are the case's expected ones.
fn lifecycle_matrix_part(cells: &[MatrixCell]) -> PartReport {
    let failures: Vec<String> = cells
        .iter()
        .filter_map(|cell| {
            let MatrixCell::Case(case) = cell else {
                return None;
            };
            let passes = run_case(case).is_ok_and(|outcome| {
                outcome.category() == case.expected_category
                    && outcome.exit_code() == case.expected_exit
            });
            (!passes).then(|| format!("lifecycle-matrix/{}/{}", case.verb.name(), case.kind.name()))
        })
        .collect();
    PartReport {
        part: "lifecycle-matrix",
        result: if failures.is_empty() {
            PartResult::Passed
        } else {
            PartResult::Failed
        },
        failures,
    }
}

/// The `parity` part (FR-563, FR-567) over the corpus already preflighted by
/// this qualification run.
pub(crate) fn parity_part(corpus: &Path) -> Result<PartReport> {
    let report = crate::parity::report_with_corpus(corpus)?;
    Ok(parity_report_part(report))
}

#[cfg(feature = "test-support")]
pub(crate) fn parity_part_with_reference(
    reference: &dyn fsl_eval::execution::SuppliedBackend,
    backends: &[fsl_conformance::BackendEntry<'_>],
    inputs: &fsl_conformance::ParityInputs,
) -> Result<PartReport> {
    if backends.is_empty() {
        return Ok(PartReport {
            part: "parity",
            result: PartResult::NoBackend,
            failures: Vec::new(),
        });
    }
    let report = fsl_conformance::run_parity_with_reference(
        reference,
        backends,
        inputs,
        &fsl_conformance::ParityLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .map_err(|refusal| crate::error::Error::QualificationRefusal(refusal.to_string()))?;
    Ok(parity_report_part(report))
}

fn parity_report_part(report: fsl_conformance::ParityReport) -> PartReport {
    let failures = report
        .backends
        .iter()
        .flat_map(|backend| {
            backend.findings.iter().map(move |finding| {
                format!(
                    "parity/{}: {:?}: unit={} entry={} arguments={:?} reference={:?} result={:?}",
                    format!("{:?}", backend.backend).to_ascii_lowercase(),
                    finding.kind,
                    finding.unit,
                    finding.entry,
                    finding.arguments,
                    finding.reference,
                    finding.result,
                )
            })
        })
        .collect::<Vec<_>>();
    PartReport {
        part: "parity",
        result: if report.exit_code() == 0 {
            PartResult::Passed
        } else {
            PartResult::Failed
        },
        failures,
    }
}

/// The parts after `corpus`, in report order.
pub(crate) fn lifecycle_part(replacement: Option<&[MatrixCell]>) -> PartReport {
    lifecycle_matrix_part(replacement.unwrap_or_else(|| lifecycle_matrix()))
}

/// Run the gate: the corpus against FSL, the lifecycle matrix and the parity
/// of the linked backends, with the verdict `qualified` only when all three
/// pass (FR-567). The exit contract is FR-353's.
///
/// # Errors
///
/// [`crate::error::Error::QualificationRefusal`] for a refusal before any
/// verdict.
pub fn run(inputs: &QualifyInputs<'_>) -> Result<Outcome> {
    execute(inputs).map(|execution| execution.outcome)
}

/// [`run`], keeping the operation bytes of the passed corpus vectors as
/// [`qualification::execute`] does.
///
/// # Errors
///
/// As [`run`].
pub fn execute(inputs: &QualifyInputs<'_>) -> Result<qualification::Execution> {
    #[cfg(feature = "test-support")]
    let replacement = inputs.matrix.as_deref();
    #[cfg(not(feature = "test-support"))]
    let replacement = None;
    #[cfg(feature = "test-support")]
    if inputs.parity.is_some() || inputs.parity_count.is_some() {
        let run_parity = |corpus: &Path| {
            let mut parity_inputs = fsl_conformance::ParityInputs::new(corpus);
            if let Some(count) = inputs.parity_count {
                parity_inputs.count = count;
            }
            match &inputs.parity {
                Some(parity) => {
                    parity_part_with_reference(parity.reference, &parity.backends, &parity_inputs)
                }
                None => crate::parity::report_with_inputs(&parity_inputs).map(parity_report_part),
            }
        };
        return qualification::execute_with_parts_and_parity(
            &inputs.fspec,
            &inputs.operands,
            replacement,
            Some(&run_parity),
        );
    }
    qualification::execute_with_parts(&inputs.fspec, &inputs.operands, replacement)
}
