// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-563, FR-295: the JIT against the interpreter over generated units.

use filament_eval::ExecutionBackendChoice;
use filament_exact::Cancel;
use fsl_conformance::{run_parity, ParityInputs, ParityLimits};
use fsl_jit::Jit;

/// Smoke test, deliberately untagged (TC-1386 verifies the runner itself, in
/// `fsl-conformance`): the first 200 admitted units at the default seed show no
/// finding. It stops before unit 555, so it is not the gate.
#[test]
fn parity_smoke_over_the_first_200_generated_units() {
    let report = gate(200);
    let backend = &report.backends[0];
    assert!(backend.compared > 0);
    assert_eq!(
        report.exit_code(),
        0,
        "{} findings, first: {:#?}",
        backend.total_findings,
        backend.findings.first()
    );
}

/// The generated program exercises exact integers larger than the machine
/// register width through nested calls and arithmetic.
#[test]
fn the_jit_passes_the_parity_gate_over_300_generated_units() {
    let report = gate(300);
    assert_eq!(
        report.exit_code(),
        0,
        "{:#?}",
        report.backends[0].findings.first()
    );
}

fn gate(count: u64) -> fsl_conformance::ParityReport {
    let corpus = tempfile::tempdir().expect("a parity corpus directory");
    std::fs::write(corpus.path().join("complete-v1.json"), "{\"vectors\":[]}")
        .expect("an empty fixture corpus");
    run_parity(
        &[(ExecutionBackendChoice::Jit, &Jit, Some(fsl_jit::coverage()))],
        &ParityInputs {
            corpus: corpus.path().to_path_buf(),
            count,
            seed: fsl_conformance::DEFAULT_PARITY_SEED,
            clauses: Vec::new(),
        },
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("the gate runs")
}
