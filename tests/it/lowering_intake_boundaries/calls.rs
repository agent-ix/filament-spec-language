// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{identity, source};
use filament_exact::{Cancel, Integer, Value};
use fsl_replay::spine::{
    self, Call, CallArgument, CallOutcome, DependencyInput, RunRefusal, SpineLimits,
};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::trace;
use serde_json::json;

const PROGRAM: &str = "function difference using v(left: Int[0, 9], right: Int[0, 9]): Integer pure { left - right }\nfunction negateFlag using v(flag: Boolean): Boolean pure { not flag }";

fn call(
    function: &str,
    arguments: Vec<CallArgument>,
    work: u64,
) -> Result<CallOutcome, Box<RunRefusal>> {
    let text = source(PROGRAM);
    spine::run(
        identity(),
        "boundaries".to_owned(),
        "boundaries.native",
        text.as_bytes(),
        &Default::default(),
        &DependencyInput::default(),
        SpineLimits::default(),
        &Call::new(
            function.to_owned(),
            arguments,
            spine::default_accounting(work),
            Default::default(),
        ),
        &ObjectEnvironment::default(),
        &Cancel::new(),
    )
}

fn integer(parameter: &str, value: i64) -> CallArgument {
    CallArgument {
        parameter: parameter.to_owned(),
        value: Value::Integer(Integer::from(value)),
    }
}

#[test]
#[trace("FR-100-AC-4", "FR-100-AC-7")]
fn the_named_call_binds_by_parameter_identity_instead_of_request_position() {
    for (arguments, expected) in [
        (vec![integer("right", 3), integer("left", 8)], "5"),
        (vec![integer("left", 3), integer("right", 8)], "-5"),
    ] {
        let CallOutcome::Completed(value) = call("difference", arguments, 1000).unwrap() else {
            panic!("call must complete");
        };
        assert_eq!(value.form(), json!({"integer":expected}));
    }
}

#[test]
#[trace("FR-100-AC-4", "FR-100-AC-7")]
fn missing_duplicate_and_foreign_argument_names_keep_the_offending_parameter() {
    let missing = call("difference", vec![integer("right", 3)], 1000).unwrap_err();
    assert!(
        matches!(&*missing, RunRefusal::UnboundParameter { parameter } if parameter == "left"),
        "{missing:?}"
    );
    let duplicate = call(
        "difference",
        vec![integer("left", 3), integer("left", 4), integer("right", 1)],
        1000,
    )
    .unwrap_err();
    assert!(
        matches!(&*duplicate, RunRefusal::DuplicateArgument { parameter } if parameter == "left"),
        "{duplicate:?}"
    );
    let foreign = call(
        "difference",
        vec![integer("left", 3), integer("right", 4), integer("other", 1)],
        1000,
    )
    .unwrap_err();
    assert!(
        matches!(&*foreign, RunRefusal::UnknownParameter { parameter } if parameter == "other"),
        "{foreign:?}"
    );
}

#[test]
#[trace("FR-100-AC-5", "FR-100-AC-7")]
fn malformed_and_unknown_function_selections_are_typed_call_refusals() {
    for function in [
        "missing",
        "module.difference",
        "",
        "difference.",
        "7difference",
    ] {
        let refusal = call(function, vec![], 1000).unwrap_err();
        assert!(
            matches!(&*refusal, RunRefusal::MissingDeclaration { function: found } if found == function),
            "{function}: {refusal:?}"
        );
        assert_eq!(refusal.stage(), Some("call"));
        assert_eq!(refusal.category().exit_code(), 20);
    }
}

#[test]
#[trace("FR-100-AC-6", "FR-100-AC-7")]
fn zero_evaluation_work_is_incomplete_while_the_same_call_with_work_completes() {
    let arguments = || {
        vec![CallArgument {
            parameter: "flag".to_owned(),
            value: Value::Boolean(false),
        }]
    };
    let outcome = call("negateFlag", arguments(), 0).unwrap();
    let CallOutcome::Incomplete(incomplete) = outcome else {
        panic!("zero work must stop: {outcome:?}");
    };
    assert_eq!(incomplete.setting_name(), "work_units");
    assert_eq!(incomplete.category().exit_code(), 22);
    let CallOutcome::Completed(value) = call("negateFlag", arguments(), 1000).unwrap() else {
        panic!("sufficient work must complete");
    };
    assert_eq!(value.form(), json!({"boolean":true}));
}
