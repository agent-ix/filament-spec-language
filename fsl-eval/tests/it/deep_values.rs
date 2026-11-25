// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-262-AC-1 (TC-734): call depth is bounded by work fuel only, and
//! 100,000-deep values and value types evaluate, compare, clone and drop on a
//! 512 KiB stack.
//!
//! Every case runs on a thread spawned with a 512 KiB stack, so a recursive
//! walk anywhere in the path overflows the test instead of passing quietly.

use filament_eval::{
    CallFailure, Evaluation, ExecutionBackendChoice, FamilyOutcome, SessionLimits,
};
use filament_exact::{
    FieldValue, Integer, LimitKind, Meter, Outcome, ScalarLimits, Value, ValueType,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::execution::{ExecuteLimits, ExecuteRequest, ExecuteSelection, SuppliedBackends};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::trace;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

const DEPTH: u64 = 100_000;

fn on_small_stack<T: Send + 'static>(run: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(run)
        .expect("spawns")
        .join()
        .expect("no overflow or panic on a 512 KiB stack")
}

fn limits(work_units: u64) -> ScalarLimits {
    ScalarLimits {
        integer_bits: 256,
        decimal_digits: 256,
        scale_expansion: 1_024,
        text_input_bytes: 1_048_576,
        text_scalars: 1_048_576,
        normalized_scalars: 1_048_576,
        unit_edges: 1_024,
        value_occurrences: 2_000_000,
        work_units,
        result_units: 2_000_000,
    }
}

fn package_of(declarations: &str) -> CheckedPackage {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n{declarations}\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "deep-values"),
        "deep-values.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("assembly");
    CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .expect("the package checks"),
    )
}

fn call(
    package: &CheckedPackage,
    function: &str,
    argument: u64,
    meter: &mut Meter,
) -> FamilyOutcome<Value> {
    package
        .call(
            &QualifiedName::unqualified(function).expect("an identifier"),
            vec![Value::Integer(Integer::from(argument))],
            &ObjectEnvironment::default(),
            meter,
        )
        .expect("the call is admitted")
        .outcome
}

const TALLY: &str = "function tally using v(n: Int[0, 1000000]): Integer pure decreases(n) \
                     { if n = 0 then 0 else tally(n - 1) + 1 }";

fn assert_completed_with(outcome: &FamilyOutcome<Value>, expected: i64) {
    assert!(
        matches!(outcome, FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(n)))
            if *n == Integer::from(expected)),
        "{outcome:?}"
    );
}

#[trace("TC-734", "FR-262-AC-1")]
#[test]
pub(super) fn a_100_000_deep_recursive_call_completes_and_fuel_is_exact_at_the_last_charge() {
    on_small_stack(|| {
        let package = package_of(TALLY);
        let mut meter = Meter::new(limits(50_000_000));
        let outcome = call(&package, "tally", DEPTH, &mut meter);
        assert_completed_with(&outcome, 100_000);
        let spent = meter.consumed(LimitKind::WorkUnits);
        assert!(
            spent > DEPTH,
            "each recursive call must charge, spent {spent}"
        );

        // One unit short is a typed incomplete at the denied charge.
        let mut short = Meter::new(limits(spent - 1));
        let FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) =
            call(&package, "tally", DEPTH, &mut short)
        else {
            panic!("fuel exhaustion must be a typed incomplete");
        };
        assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
        assert_eq!(incomplete.limit, spent - 1);
        assert!(incomplete.consumed < spent);

        // Exactly the spend completes with the same value and spends it all.
        let mut exact = Meter::new(limits(spent));
        assert_completed_with(&call(&package, "tally", DEPTH, &mut exact), 100_000);
        assert_eq!(exact.consumed(LimitKind::WorkUnits), spent);
    });
}

#[trace("TC-734", "FR-262-AC-1")]
#[test]
pub(super) fn a_call_that_exceeds_fuel_returns_the_typed_refusal_not_a_crash() {
    on_small_stack(|| {
        let package = package_of(TALLY);
        let mut meter = Meter::new(limits(1_000));
        let FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) =
            call(&package, "tally", DEPTH, &mut meter)
        else {
            panic!("a 100,000-deep call on 1,000 work units must be incomplete");
        };
        assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
        assert_eq!(incomplete.limit, 1_000);
    });
}

/// TC-1121 FR-403-AC-7's recursive-count vector through the no_std entry on
/// the required 512 KiB stack.
pub(super) fn no_std_recursive_count_on_small_stack() -> Result<Evaluation, CallFailure> {
    on_small_stack(|| {
        let package = package_of(TALLY);
        let cancel = filament_exact::Cancel::new();
        let request = ExecuteRequest {
            selection: ExecuteSelection::Function {
                name: QualifiedName::unqualified("tally").expect("function name"),
                arguments: vec![Value::Integer(Integer::from(DEPTH))],
                environment: ObjectEnvironment::default(),
            },
            backend: ExecutionBackendChoice::Interpreter,
        };
        let limits = ExecuteLimits {
            accounting: limits(50_000_000),
            prepare: fsl_eval::execution::PrepareLimits::default(),
            session: SessionLimits::default(),
        };
        let mut run = filament_semantic_value::supplied_stop::SuppliedRun::new(
            filament_semantic_value::supplied_stop::SuppliedLimits::default(),
            cancel.clone(),
        )
        .expect("supplied run initializes");
        let evaluation = fsl_eval::execution::execute(
            &package,
            &request,
            &SuppliedBackends::default(),
            &limits,
            &mut run,
            &cancel,
        )?;
        assert_completed_with(&evaluation.outcome, DEPTH as i64);
        Ok(evaluation)
    })
}

const CHAIN: &str = "record Link { head: Int[0,9]; tail: Option<Link>; } \
    function chain using v(n: Int[0,100000]): Option<Link> pure decreases(n) \
    { if n = 0 then none else Some(Link { head: 7, tail: chain(n - 1) }) }";

fn chain_length(value: &Value) -> u64 {
    let mut cursor = value;
    let mut length = 0;
    loop {
        let Value::Option(option) = cursor else {
            panic!("a chain is an option")
        };
        let Some(Value::Composite(link)) = option.payload() else {
            assert!(option.payload().is_none(), "a present link is a record");
            return length;
        };
        assert!(
            matches!(&link.slots()[0], FieldValue::Present(Value::Integer(head))
            if *head == Integer::from(7_i64))
        );
        let FieldValue::Present(tail) = &link.slots()[1] else {
            panic!("the optional-valued tail has a value")
        };
        cursor = tail;
        length += 1;
    }
}

#[test]
pub(super) fn a_100_000_long_list_evaluates_compares_clones_and_drops() {
    on_small_stack(|| {
        let package = package_of(CHAIN);
        let mut accounting = limits(50_000_000);
        // Result retention charges each successively larger returned value.
        accounting.result_units = 4 * DEPTH * (DEPTH + 1);
        let mut meter = Meter::new(accounting);
        let FamilyOutcome::Evaluated(Outcome::Completed(value)) =
            call(&package, "chain", DEPTH, &mut meter)
        else {
            panic!("step 1: the list evaluates");
        };
        assert_eq!(chain_length(&value), DEPTH);
        let cloned = value.clone();
        let mut comparison = Meter::new(limits(50_000_000));
        assert!(matches!(
            filament_exact::planned_equality(&value, &cloned, &mut comparison),
            Outcome::Completed(true)
        ));
        // A list one link shorter is not equal: the comparison is not
        // trivially true.
        let mut shorter_meter = Meter::new(accounting);
        let FamilyOutcome::Evaluated(Outcome::Completed(shorter)) =
            call(&package, "chain", DEPTH - 1, &mut shorter_meter)
        else {
            panic!("the shorter list evaluates");
        };
        let mut unequal = Meter::new(limits(50_000_000));
        assert!(matches!(
            filament_exact::planned_equality(&value, &shorter, &mut unequal),
            Outcome::Completed(false)
        ));
        drop(cloned);
        drop(shorter);
        drop(value);
    });
}

#[test]
pub(super) fn a_100_000_deep_value_type_clones_compares_hashes_formats_and_drops() {
    on_small_stack(|| {
        let nest = |base: ValueType| {
            let mut value_type = base;
            for _ in 0..DEPTH {
                value_type = ValueType::Option(Arc::new(value_type));
            }
            value_type
        };
        let value_type = nest(ValueType::Boolean);
        let cloned = value_type.clone();
        assert_eq!(value_type, cloned);
        assert_ne!(
            value_type,
            nest(ValueType::Integer),
            "the leaf decides equality"
        );
        let hash = |item: &ValueType| {
            let mut hasher = DefaultHasher::new();
            item.hash(&mut hasher);
            hasher.finish()
        };
        assert_eq!(hash(&value_type), hash(&cloned));
        let debug = format!("{value_type:?}");
        assert_eq!(debug.matches("Option(").count(), 100_000);
        assert!(debug.contains("Boolean"));
        drop(cloned);
        drop(value_type);
    });
}
