// SPDX-License-Identifier: AGPL-3.0-or-later
//! `CheckMode` on `PackageDeclarations::check` (FR-495): `Linked` proves each
//! definedness obligation at check, `Kernel` records none and leaves it to
//! the evaluator. Every assertion compares typed causes and locations.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    CheckCause, CheckRefusal, CheckedGraph, MeasureObligation, Obligation, PackageDeclarations,
};
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::value::{admit_integer_division, DefinitionRole};
use quire_semantic_value::checking::{CheckMode, CheckingLimits};
use quire_semantic_value::location::ExpressionOwner as Origin;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"quire.value.complete/v1\";\n";

fn check(declarations: &str, mode: CheckMode) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "check-mode"),
        "check-mode.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let mut package = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"));
    package.integer_division = Some(
        admit_integer_division(
            &[DefinitionRole::IntegerDivisionEuclidean.reference()],
            None,
        )
        .expect("the Euclidean definition admits"),
    );
    package.check(mode, CheckingLimits::default())
}

fn refusals(declarations: &str, mode: CheckMode) -> Vec<CheckRefusal> {
    check(declarations, mode).expect_err("the unit is refused")
}

/// The one refusal of `declarations` under `mode`.
fn refusal(declarations: &str, mode: CheckMode) -> CheckRefusal {
    let mut all = refusals(declarations, mode);
    assert_eq!(all.len(), 1, "{declarations}: {all:?}");
    all.remove(0)
}

/// `declarations` is refused with `obligation` under `Linked`, and checks
/// under `Kernel`, which leaves the obligation to the evaluator.
fn kernel_drops(declarations: &str, obligation: impl Fn(&Obligation) -> bool) {
    match refusal(declarations, CheckMode::Linked).cause {
        CheckCause::Unproved(found) => assert!(obligation(&found), "{declarations}: {found:?}"),
        other => panic!("{declarations}: not an obligation: {other:?}"),
    }
    check(declarations, CheckMode::Kernel).expect("Kernel records no obligation");
}

/// FR-562-AC-6 (checker half): a result bound the addition's interval does
/// not prove is `unproved-range` under `Linked` and a run-time obligation
/// under `Kernel`; a proved bound checks under both.
#[trace("FR-562-AC-6", "TC-1384")]
#[test]
fn an_unproved_result_bound_checks_only_under_kernel() {
    kernel_drops(
        "function s using v(a: Int[0, 9], b: Int[0, 9]): Int[0, 9] pure { a + b }",
        |found| matches!(found, Obligation::Range { .. }),
    );
    let source = "function s7 using v(a: Int[0, 4], b: Int[0, 5]): Int[0, 9] pure { a + b }";
    check(source, CheckMode::Linked).expect("the interval [0, 9] proves the bound");
    check(source, CheckMode::Kernel).expect("Kernel checks it too");
}

/// FR-495-AC-3 (checker half): a decimal division by a literal zero is
/// `unproved-nonzero` under `Linked` and checks under `Kernel`.
#[trace("FR-495-AC-3", "TC-1284")]
#[test]
fn an_unproved_nonzero_divisor_checks_only_under_kernel() {
    kernel_drops(
        "type P = Decimal[-1000, 1000; 0, 2; nearest-even];\n\
         function d4 using v(): P pure { decimal(1, 0) / decimal(0, 3) }",
        |found| matches!(found, Obligation::Nonzero),
    );
}

/// `value(x)` with no proved `present(x)` is `unproved-presence` under
/// `Linked`; `Kernel` leaves it to the evaluator.
#[trace("FR-495-AC-10", "TC-1419")]
#[test]
fn an_unproved_presence_checks_only_under_kernel() {
    kernel_drops(
        "function p using v(x: Option<Integer>): Integer pure { value(x) }",
        |found| matches!(found, Obligation::Presence),
    );
    let guarded = "function g using v(x: Option<Integer>): Integer pure \
                   { if present(x) then value(x) else 0 }";
    for mode in [CheckMode::Linked, CheckMode::Kernel] {
        check(guarded, mode).unwrap_or_else(|refusals| panic!("{mode:?}: {refusals:?}"));
    }
}

/// FR-104 "Definedness facts": `Kernel` reaches function bodies only, so a
/// state clause still refuses its unproved obligation.
#[trace("FR-495-AC-11", "TC-1419")]
#[test]
fn a_state_clause_still_refuses_its_obligation_under_kernel() {
    let clause = "invariant NoGuard using v on Config::ConfigVersion at current { \
                  deref(value(self.parent)).versionNumber < self.versionNumber }\n";
    let guarded = "invariant ParentOrder using v on Config::ConfigVersion at current { \
                   present(self.parent) implies deref(value(self.parent)).versionNumber \
                   < self.versionNumber }\n";
    for mode in [CheckMode::Linked, CheckMode::Kernel] {
        let graph = crate::model_operations::admit_and_assemble_with_body(
            &crate::model_operations::tc465_document(),
            guarded,
        )
        .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
        .check(mode, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{mode:?}: the guarded clause checks: {refusals:?}"));
        assert!(graph.state_clause("ParentOrder").is_some(), "{mode:?}");
    }
    for mode in [CheckMode::Linked, CheckMode::Kernel] {
        let declarations = crate::model_operations::admit_and_assemble_with_body(
            &crate::model_operations::tc465_document(),
            clause,
        )
        .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"));
        let refusals = declarations
            .check(mode, CheckingLimits::default())
            .expect_err("the clause is refused");
        assert!(
            matches!(
                refusals.as_slice(),
                [CheckRefusal {
                    cause: CheckCause::Unproved(Obligation::Presence),
                    ..
                }]
            ),
            "{mode:?}: {refusals:?}"
        );
    }
}

/// `Kernel` drops obligations only. A recursion with no `decreases` measure
/// is refused `missing-measure` at its call in both modes, and a body whose
/// result bound `Linked` refuses first still reaches that refusal under
/// `Kernel`.
#[trace("FR-495-AC-12", "TC-1419")]
#[test]
fn kernel_mode_still_refuses_a_nonterminating_recursion() {
    let missing = |refusal: &CheckRefusal, path: &[usize]| {
        assert_eq!(
            refusal.cause,
            CheckCause::UnprovedDecrease {
                cycle: vec!["f".to_owned(), "f".to_owned()],
                obligation: MeasureObligation::MissingMeasure,
            }
        );
        assert!(
            matches!(&refusal.location.origin, Origin::Body { function, .. } if function == "f"),
            "{:?}",
            refusal.location
        );
        assert_eq!(refusal.location.path(), path);
    };
    let source = "function f using v(x: Int[0, 9]): Int[0, 9] pure { f(x) }";
    for mode in [CheckMode::Linked, CheckMode::Kernel] {
        missing(&refusal(source, mode), &[]);
    }
    let source = "function f using v(x: Int[0, 9]): Int[0, 9] pure { f(x) + 1 }";
    assert!(matches!(
        refusal(source, CheckMode::Linked).cause,
        CheckCause::Unproved(Obligation::Range { .. })
    ));
    missing(&refusal(source, CheckMode::Kernel), &[0]);
}

/// FR-495: `Kernel` leaves a function body's obligation to the evaluator
/// but a termination measure is checked as under `Linked`, so an unproved
/// obligation inside `decreases(..)` is refused in both modes.
#[trace("FR-495-AC-12", "TC-1419")]
#[test]
fn kernel_mode_still_proves_a_termination_measure() {
    let source = "function w using v(x: Option<Integer>): Int[0, 9] pure \
                  decreases(value(x)) { 0 }";
    for mode in [CheckMode::Linked, CheckMode::Kernel] {
        assert!(
            matches!(
                refusal(source, mode).cause,
                CheckCause::Unproved(Obligation::Presence)
            ),
            "{mode:?}"
        );
    }
}

/// A rational result outside its declared domain is `unproved-range` under
/// `Linked`; `Kernel` leaves the bound to the evaluator.
#[trace("FR-495-AC-14", "TC-1421")]
#[test]
fn an_unproved_rational_range_checks_only_under_kernel() {
    kernel_drops(
        "function r using v(a: Rational[0, 1; 1, 2], b: Rational[0, 1; 1, 2]): \
         Rational[0, 1; 1, 2] pure { a + b }",
        |found| matches!(found, Obligation::RationalRange),
    );
}

/// A rational `/` whose divisor may be zero is `unproved-nonzero` under
/// `Linked`; `Kernel` leaves the divisor to the evaluator.
#[trace("FR-495-AC-15", "TC-1421")]
#[test]
fn an_unproved_rational_divisor_checks_only_under_kernel() {
    kernel_drops(
        "function q using v(a: Rational[0, 1; 1, 2], b: Rational[0, 1; 1, 2]): \
         Rational[0, 100; 1, 100] pure { a / b }",
        |found| matches!(found, Obligation::Nonzero),
    );
}

/// A reduction with no identity over a possibly empty source is refused as
/// an unproved nonempty-reduction obligation under `Linked`; `Kernel` leaves the emptiness to the
/// evaluator.
#[trace("FR-495-AC-16", "TC-1421")]
#[test]
fn an_unproved_nonempty_reduction_checks_only_under_kernel() {
    kernel_drops(
        "type S = Int[0, 3];\n\
         function m using v(q: Sequence<S>[0, 2]): S pure { reduce<S>(acc, x in q: acc) }",
        |found| matches!(found, Obligation::NonemptyReduction { .. }),
    );
    let nonempty = "type S = Int[0, 3];\n\
                    function m using v(q: Sequence<S>[1, 2]): S pure { reduce<S>(acc, x in q: acc) }";
    for mode in [CheckMode::Linked, CheckMode::Kernel] {
        check(nonempty, mode).unwrap_or_else(|refusals| panic!("{mode:?}: {refusals:?}"));
    }
}
