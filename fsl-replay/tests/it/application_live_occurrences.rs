// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-013: one live-occurrence limit across the application scheduler and FSL machine.

use filament_eval::{
    Admission, Body, BodyFactory, BodyFailure, Evaluation, FamilyOutcome, LiveContext, Resume,
    SessionLimits, Transition,
};
use filament_exact::{
    ChargePoint, Identifier, Incomplete, Integer, LimitKind, Meter, Outcome, ScalarLimits, Value,
};
use filament_semantic_value::location::{ExpressionLocation, ExpressionOwner};
use filament_semantic_value::object_closure::ObjectClosure;
use fsl_foundation::SourceIdentity;
use fsl_replay::{
    ApplicationEnvironment, CallableSelection, CheckedClosureHandle, CompileUnit, DependencyInput,
    ReplayLimits, StageLimits, acquire_checked_closure, call_site,
};
use ix_trace_rs::trace;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";
const PATH: &str = "live-occurrences.native";

struct Fixture {
    handle: CheckedClosureHandle,
    p: CallableSelection,
    c: CallableSelection,
}

#[derive(Clone)]
struct RequestFactory {
    callee: fsl_replay::NodeKey,
    arguments: Vec<Value>,
}

impl BodyFactory for RequestFactory {
    fn begin(&self) -> Box<dyn Body> {
        Box::new(RequestBody {
            callee: self.callee.clone(),
            arguments: self.arguments.clone(),
        })
    }
}

struct RequestBody {
    callee: fsl_replay::NodeKey,
    arguments: Vec<Value>,
}

impl Body for RequestBody {
    fn resume(
        &mut self,
        input: Resume,
        _context: &mut LiveContext,
        _meter: &mut Meter,
    ) -> Result<Transition, BodyFailure> {
        match input {
            Resume::Start(_) => Ok(Transition::Call(filament_eval::ChildRequest {
                callee: self.callee.clone(),
                arguments: self.arguments.clone(),
                site: ExpressionLocation::at(ExpressionOwner::Expression, &[]),
            })),
            Resume::Child(result) => Ok(Transition::Done(*result)),
            Resume::Again => panic!("requesting host body resumes with its child result"),
        }
    }
}

fn identity() -> SourceIdentity {
    SourceIdentity::new("test", "live-occurrences")
}

fn source(c_has_three_arguments: bool) -> String {
    let c_parameters = if c_has_three_arguments {
        "a: Integer, b: Integer, x: Integer"
    } else {
        "a: Integer, b: Integer"
    };
    let c_arguments = if c_has_three_arguments {
        "a, b, x"
    } else {
        "a, b, 0"
    };
    let p_arguments = if c_has_three_arguments {
        "a, b, 0"
    } else {
        "a, b"
    };
    format!(
        "{HEADER}function n using v(a: Integer, b: Integer, x: Integer): Integer pure {{ 0 }}\nfunction c using v({c_parameters}): Integer pure {{ n({c_arguments}) }}\nfunction p using v(a: Integer, b: Integer): Integer pure {{ c({p_arguments}) }}\n"
    )
}

fn key(source: &str, name: &str) -> fsl_replay::NodeKey {
    let selection = fsl_replay::QualifiedName::new(vec![
        Identifier::new(name).expect("function name is an identifier"),
    ])
    .expect("function name is non-empty");
    call_site(
        identity(),
        "fixture".to_owned(),
        PATH,
        source.as_bytes(),
        [],
        &DependencyInput::default(),
        &selection,
    )
    .expect("function selection locates in source")
    .site
    .function
}

fn fixture(c_has_three_arguments: bool) -> Fixture {
    let source = source(c_has_three_arguments);
    let dependencies = DependencyInput::default();
    let p_key = key(&source, "p");
    let c_key = key(&source, "c");
    let acquired = acquire_checked_closure(
        CompileUnit {
            source: identity(),
            package_name: "fixture".to_owned(),
            path: PATH,
            bytes: source.as_bytes(),
        },
        [],
        &dependencies,
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("checked closure is acquired");
    let handle = acquired.handle().clone();
    let p = handle
        .lookup_callable(handle.package_name(), &p_key)
        .expect("p is an admitted callable");
    let c = handle
        .lookup_callable(handle.package_name(), &c_key)
        .expect("c is an admitted callable");
    Fixture { handle, p, c }
}

fn integer(value: u64) -> Value {
    Value::Integer(Integer::from(value))
}

fn meter() -> Meter {
    let large = u64::from(u32::MAX);
    Meter::new(ScalarLimits {
        integer_bits: large,
        decimal_digits: large,
        scale_expansion: large,
        text_input_bytes: large,
        text_scalars: large,
        normalized_scalars: large,
        unit_edges: large,
        value_occurrences: large,
        work_units: 100_000,
        result_units: large,
    })
}

fn run(fixture: &Fixture, host_root: bool, c_has_three_arguments: bool, limit: u64) -> Evaluation {
    let mut registry = fixture.handle.registry();
    if host_root {
        let arguments = if c_has_three_arguments {
            vec![integer(1), integer(2), integer(3)]
        } else {
            vec![integer(1), integer(2)]
        };
        assert_eq!(
            registry.register(
                &fixture.p,
                fixture.p.signature(),
                RequestFactory {
                    callee: fixture.c.key().clone(),
                    arguments,
                },
            ),
            Ok(Admission::Accepted)
        );
    }
    let environment = ApplicationEnvironment::new(ObjectClosure::default());
    let mut meter = meter();
    registry
        .begin_call(
            &fixture.p,
            vec![integer(1), integer(2)],
            SessionLimits {
                live_occurrences: limit,
                held: 0,
            },
            &environment,
            &mut meter,
        )
        .expect("root application is admitted")
        .run()
        .expect("application yields its evaluation")
}

fn assert_incomplete(
    evaluation: &Evaluation,
    expected_limit: u64,
    consumed: u64,
    next_charge: u64,
) {
    let FamilyOutcome::Evaluated(Outcome::Incomplete(Incomplete {
        limit_kind,
        limit: actual_limit,
        consumed: actual_consumed,
        next_charge: actual_next_charge,
        charge_point,
    })) = &evaluation.outcome
    else {
        panic!(
            "expected live-occurrence incomplete, got {:?}",
            evaluation.outcome
        );
    };
    assert_eq!(*limit_kind, LimitKind::LiveOccurrences);
    assert_eq!(*actual_limit, expected_limit);
    assert_eq!(*actual_consumed, consumed);
    assert_eq!(*actual_next_charge, Integer::from(next_charge));
    assert_eq!(*charge_point, ChargePoint::FunctionCall);
}

fn completed_integer(evaluation: &Evaluation) -> &Integer {
    let FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) = &evaluation.outcome
    else {
        panic!(
            "expected completed integer evaluation, got {:?}",
            evaluation.outcome
        );
    };
    value
}

/// TC-3124: the root frame is counted once for either an interpreted or host root.
#[trace("TC-3124", "NFR-130-AC-5")]
#[test]
fn live_occurrences_aggregate_across_host_and_interpreted_roots() {
    let three = fixture(true);
    let interpreted_stop = run(&three, false, true, 10);
    let host_stop = run(&three, true, true, 10);
    assert_incomplete(&interpreted_stop, 10, 7, 4);
    assert_incomplete(&host_stop, 10, 7, 4);
    assert_eq!(interpreted_stop.location, host_stop.location);
    assert!(interpreted_stop.location.is_some());
    assert_eq!(interpreted_stop.peak_live_occurrences, 7);
    assert_eq!(host_stop.peak_live_occurrences, 7);

    let two = fixture(false);
    let interpreted_complete = run(&two, false, false, 10);
    let host_complete = run(&two, true, false, 10);
    assert_eq!(
        completed_integer(&interpreted_complete),
        completed_integer(&host_complete)
    );
    assert_eq!(interpreted_complete.peak_live_occurrences, 10);
    assert_eq!(host_complete.peak_live_occurrences, 10);
}

/// TC-3125: the acquired application facade forwards the caller's session limit.
#[trace("TC-3125", "FR-485-AC-19")]
#[test]
fn registry_live_occurrence_limit_is_the_callers() {
    let three = fixture(true);
    let interpreted_stop = run(&three, false, true, 10);
    let host_stop = run(&three, true, true, 10);
    assert_incomplete(&interpreted_stop, 10, 7, 4);
    assert_incomplete(&host_stop, 10, 7, 4);
    assert_eq!(interpreted_stop.location, host_stop.location);
    assert!(interpreted_stop.location.is_some());
    assert_eq!(interpreted_stop.peak_live_occurrences, 7);
    assert_eq!(host_stop.peak_live_occurrences, 7);

    let two = fixture(false);
    let interpreted_complete = run(&two, false, false, 10);
    let host_complete = run(&two, true, false, 10);
    assert_eq!(
        completed_integer(&interpreted_complete),
        completed_integer(&host_complete)
    );
    assert_eq!(interpreted_complete.peak_live_occurrences, 10);
    assert_eq!(host_complete.peak_live_occurrences, 10);

    let limited = run(&three, true, true, 6);
    let FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) = limited.outcome else {
        panic!("caller limit of six must stop before c's frame");
    };
    assert_eq!(incomplete.limit_kind, LimitKind::LiveOccurrences);
    assert_eq!(incomplete.limit, 6);
    assert_eq!(incomplete.consumed, 3);
    assert_eq!(incomplete.next_charge, Integer::from(4_u64));
}
