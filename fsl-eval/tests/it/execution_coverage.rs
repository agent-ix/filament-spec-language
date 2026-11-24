// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560: checked function constructs are tested against typed backend coverage.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::execution::coverage::{entry_within_coverage, first_outside};
use fsl_eval::execution::{
    BackendCoverage, CoverageOperation, CoverageOperator, CoverageValueType,
};
use fsl_forms::build_unit;
use fsl_foundation::SourceIdentity;
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;
use std::collections::BTreeSet;

fn package() -> CheckedPackage {
    let source = SourceIdentity::new("fsl-eval", "coverage.native");
    let bytes = b"language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        function inc using v(x: Int[0, 9]): Int[0, 10] pure { let y = x in y + 1 }\n";
    let parsed = fsl_cst::parse(source, "coverage.native", bytes, fsl_cst::Limits::default())
        .expect("source parses");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("forms build");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "coverage".to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .expect("declarations assemble");
    CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .expect("package checks"),
    )
}

fn coverage(with_let: bool) -> BackendCoverage {
    let expression_forms = ["literal", "name", "operator", "if"]
        .into_iter()
        .chain(with_let.then_some("let"))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    BackendCoverage {
        expression_forms,
        value_types: [CoverageValueType::Integer].into_iter().collect(),
        operations: [CoverageOperation {
            operator: CoverageOperator::Add,
            operand_types: vec![CoverageValueType::Integer, CoverageValueType::Integer],
            result_type: CoverageValueType::Integer,
        }]
        .into_iter()
        .collect(),
    }
}

/// A parameter/let read is the ADR-012 `name` form and is never silently
/// treated as an uncovered-free local node.
#[trace("TC-1381", "FR-560-AC-8")]
#[test]
fn local_reads_require_name_form_coverage() {
    let package = package();
    let mut covered = coverage(true);
    covered.expression_forms.remove("name");
    let outside = first_outside(&package, "inc", &covered)
        .expect("coverage traversal succeeds")
        .expect("the local read is outside coverage");
    assert_eq!(
        outside.construct,
        filament_eval::UntranslatableConstruct::ExpressionForm("name".into())
    );
    assert!(!entry_within_coverage(&package, "inc", &covered).unwrap());
}

/// The coverage walk finds a `let` even when its nested local reads and
/// operation are covered.
#[trace("TC-1381", "FR-560-AC-8")]
#[test]
fn let_is_a_covered_form_only_when_explicitly_listed() {
    let package = package();
    let outside = first_outside(&package, "inc", &coverage(false))
        .expect("coverage traversal succeeds")
        .expect("let is outside coverage");
    assert_eq!(
        outside.construct,
        filament_eval::UntranslatableConstruct::ExpressionForm("let".into())
    );
    assert!(entry_within_coverage(&package, "inc", &coverage(true)).unwrap());
}
