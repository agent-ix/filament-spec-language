// SPDX-License-Identifier: AGPL-3.0-or-later
//! Quantity division reaches the checked outcome through command intake.

use fsl_command::command::{self, CheckInputs};
use ix_trace_rs::trace;

/// The source of filament-examples PR 76's quantity-scale-by-division case.
#[trace("TC-1824", "FR-680-AC-5", "FSpec-FR-142-AC-41")]
#[test]
fn quantity_scale_by_division_corpus_case_checks() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("quantity-scale-by-division.fsl");
    std::fs::write(
        &file,
        "// FSpec FR-142-AC-41\n\
         language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         dimension T;\n\
         unit s: T = rational(1, 1);\n\
         unit ms: T = rational(1, 1000) * s;\n\
         function f using v(d: Quantity<ms>): Quantity<ms> pure { d / 2 }\n",
    )
    .unwrap();
    let outcome = command::check_outcome(&file, &CheckInputs::default());
    assert_eq!(
        outcome.document().category().exit_code(),
        0,
        "{:?}",
        outcome.document()
    );
}
