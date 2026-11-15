// SPDX-License-Identifier: AGPL-3.0-or-later
//! Evaluation rulings: a quantifier over a concrete value of an unbounded
//! collection type is decided by visiting that value (FR-335-AC-6, FSpec
//! FR-346-AC-7), and a deep recursion is bounded only by the work budget
//! (NFR-130-AC-1).

use filament_eval::FamilyOutcome;
use filament_exact::{
    form_collection, ChargePoint, CollectionKind, CollectionType, Integer, IntegerInterval,
    LimitKind, Meter, Outcome, ScalarLimits, Value, ValueType,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::trace;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    function p using v(s: Set<Int[0, 9999]>): Boolean pure { forall(x in s : x < 500) }\n\
    function e using v(s: Set<Int[0, 9999]>): Boolean pure { exists(x in s : x = 7) }\n\
    function d using v(n: Integer): Integer pure decreases(n) { if n > 0 then d(n - 1) else 0 }\n";

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

fn package() -> CheckedPackage {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("a", "evaluation-rulings"),
        "unit.native",
        UNIT.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles");
    CheckedPackage::link(
        declarations
            .check(CheckMode::Linked, CheckingLimits::default())
            .expect("the package checks"),
    )
}

/// The unbounded `Set<Int[0, 9999]>` holding `0..count`.
fn set_of(count: i64) -> Value {
    let element = ValueType::Int(
        IntegerInterval::new(Integer::from(0_i64), Integer::from(9999_i64)).unwrap(),
    );
    let collection = CollectionType::new(CollectionKind::Set, element, None);
    let members = (0..count)
        .map(|n| Value::Integer(Integer::from(n)))
        .collect();
    let mut meter = Meter::new(UNLIMITED);
    match form_collection(&collection, members, &mut meter).expect("members in the element type") {
        Outcome::Completed(set) => set,
        other => panic!("the set forms on an unlimited meter: {other:?}"),
    }
}

fn call(
    package: &CheckedPackage,
    name: &str,
    arguments: Vec<Value>,
    meter: &mut Meter,
) -> Outcome<Value> {
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("a function name is an identifier"),
            arguments,
            &ObjectEnvironment::default(),
            meter,
        )
        .expect("the call runs");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        other => panic!("a kernel outcome, not {other:?}"),
    }
}

fn boolean(outcome: &Outcome<Value>) -> Option<bool> {
    match outcome {
        Outcome::Completed(Value::Boolean(value)) => Some(*value),
        _ => None,
    }
}

/// FR-335-AC-6: `forall` and `exists` over a concrete value of an unbounded
/// set type are decided by visiting the value's occurrences, and a meter too
/// small for the visits stops at `collection.visit`.
#[trace("TC-1909", "FR-335-AC-6")]
#[trace("FSpec-TC-1402", "FSpec-FR-346-AC-7")]
#[test]
#[ignore = "QSL-2174: quantifier over a concrete unbounded value visits that value, confirmation pending a run"]
fn a_quantifier_over_a_concrete_unbounded_value_visits_it() {
    let package = package();
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        boolean(&call(&package, "p", vec![set_of(1_000)], &mut meter)),
        Some(false)
    );
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        boolean(&call(&package, "p", vec![set_of(100)], &mut meter)),
        Some(true)
    );
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        boolean(&call(&package, "e", vec![set_of(100)], &mut meter)),
        Some(true)
    );

    // Three work units cover the call but not a hundred visits.
    let mut short = Meter::new(ScalarLimits {
        work_units: 3,
        ..UNLIMITED
    });
    let Outcome::Incomplete(stopped) = call(&package, "p", vec![set_of(100)], &mut short) else {
        panic!("a three-unit meter cannot visit a hundred occurrences");
    };
    assert_eq!(stopped.charge_point, ChargePoint::CollectionVisit);
}

/// NFR-130-AC-1: a recursion 200,000 calls deep completes on an unlimited
/// meter, and on a short meter stops `Incomplete` at the work budget, with no
/// depth, frame or memory refusal.
#[trace("TC-1930", "NFR-130-AC-1")]
#[test]
#[ignore = "QSL-2183: deep recursion bounded only by work_units, confirmation pending a run"]
fn a_deep_recursion_stops_only_on_the_work_budget() {
    let package = package();
    let depth = Value::Integer(Integer::from(200_000_i64));
    let mut meter = Meter::new(UNLIMITED);
    let outcome = call(&package, "d", vec![depth.clone()], &mut meter);
    assert!(
        matches!(&outcome, Outcome::Completed(Value::Integer(zero)) if *zero == Integer::from(0_i64)),
        "{outcome:?}"
    );
    let needed = meter.consumed(LimitKind::WorkUnits);
    assert!(needed >= 200_000, "at least one unit per call: {needed}");

    let mut short = Meter::new(ScalarLimits {
        work_units: needed - 1,
        ..UNLIMITED
    });
    let Outcome::Incomplete(stopped) = call(&package, "d", vec![depth], &mut short) else {
        panic!("one unit short of the work stops incomplete");
    };
    assert_eq!(stopped.limit_kind, LimitKind::WorkUnits);
}
