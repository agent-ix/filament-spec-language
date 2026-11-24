// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-025, TC-026 and TC-029: optional values, presence facts and Boolean
//! clause roots.
//!
//! Every case is a pair: the adverse form is refused with its typed code and
//! cause, and a near-identical control is accepted, so a checker that refused
//! (or accepted) everything fails one side. The ConfigVersion package of
//! TC-458 supplies `versionNumber: Int[0, 1000]` and the optional `parent`.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_foundation::Code;
use ix_trace_rs::trace;
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_body, attempt_update_modifies_version_and_parent,
    config_version_document,
};

type Verdict = Result<(), Vec<(Code, Option<&'static str>)>>;

/// Checks `body` against the ConfigVersion package and reduces the outcome to
/// catalog codes and causes.
fn config(body: &str) -> Verdict {
    let document = config_version_document(
        attempt_update_modifies_version_and_parent(),
        Vec::new(),
        Vec::new(),
        json!([]),
    );
    let declarations = match admit_and_assemble_with_body(&document, body) {
        Ok(declarations) => declarations,
        Err(refusal) => {
            return Err(refusal
                .errors
                .iter()
                .map(|error| (error.cause.code(), Some(error.cause.catalog_code().cause())))
                .collect())
        }
    };
    declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .map(|_| ())
        .map_err(|refusals| {
            refusals
                .iter()
                .map(|refusal| (refusal.cause.code(), refusal.cause.cause()))
                .collect()
        })
}
fn assert_accepts(label: &str, verdict: Verdict) {
    if let Err(refusals) = verdict {
        panic!("{label} must check: {refusals:?}");
    }
}

fn assert_refuses(label: &str, verdict: Verdict, code: Code, cause: &str) {
    let refusals = verdict.expect_err(label);
    assert_eq!(refusals, [(code, Some(cause))], "{label}");
}

fn invariant(name: &str, expression: &str) -> String {
    format!("invariant {name} using v on Config::ConfigVersion at current {{ {expression} }}\n")
}

const UNPROVED: (Code, &str) = (Code::UndefinedExpression, "unproved-presence");

#[trace("TC-025")]
#[test]
fn an_unwrap_is_defined_only_where_its_own_guard_holds() {
    let unwrap = "deref(value(self.parent)).versionNumber < self.versionNumber";
    // Adverse: no guard, a guard written to the right, and a join of
    // alternatives that does not union their facts.
    for (label, expression) in [
        ("unguarded", unwrap.to_owned()),
        (
            "right guard cannot protect the left",
            format!("({unwrap}) and present(self.parent)"),
        ),
        (
            "alternative join does not union facts",
            format!("(present(self.parent) or self.versionNumber = 0) implies ({unwrap})"),
        ),
    ] {
        assert_refuses(
            label,
            config(&invariant("Adverse", &expression)),
            UNPROVED.0,
            UNPROVED.1,
        );
    }
    // Controls under the same model: implication and conditional guards.
    for (label, expression) in [
        (
            "implication",
            format!("present(self.parent) implies ({unwrap})"),
        ),
        (
            "conditional",
            format!("if present(self.parent) then {unwrap} else true"),
        ),
        (
            "absent-parent implication",
            format!("(not present(self.parent)) or ({unwrap})"),
        ),
    ] {
        assert_accepts(label, config(&invariant("Control", &expression)));
    }
}

#[trace("TC-026")]
#[test]
fn a_presence_fact_stays_with_the_observation_it_was_proved_on() {
    let post = |expression: &str| {
        format!("post P using v on Config::ConfigVersion::attemptUpdate {{ {expression} }}\n")
    };
    // Adverse: a `pre` presence fact never discharges a `post` unwrap.
    assert_refuses(
        "pre presence cannot protect a post unwrap",
        config(&post(
            "present(pre(self.parent)) implies deref(value(self.parent)).versionNumber > 0",
        )),
        UNPROVED.0,
        UNPROVED.1,
    );
    // Controls: the same observation on both sides, `post` and `pre`.
    assert_accepts(
        "post presence protects a post unwrap",
        config(&post(
            "present(self.parent) implies deref(value(self.parent)).versionNumber > 0",
        )),
    );
    assert_accepts(
        "pre presence protects a pre unwrap",
        config(&post(
            "pre(present(self.parent) implies deref(value(self.parent)).versionNumber > 0)",
        )),
    );
}

#[trace("TC-029")]
#[test]
fn a_clause_root_must_be_boolean() {
    assert_refuses(
        "numeric invariant root",
        config(&invariant("NumericRoot", "self.versionNumber")),
        Code::IllTyped,
        "non-boolean-root",
    );
    assert_accepts(
        "Boolean comparison root",
        config(&invariant(
            "BooleanRoot",
            "self.versionNumber = self.versionNumber",
        )),
    );
    assert_refuses(
        "numeric postcondition root",
        config("post N using v on Config::ConfigVersion::attemptUpdate { self.versionNumber }\n"),
        Code::IllTyped,
        "non-boolean-root",
    );
    assert_accepts(
        "Boolean operation-result postcondition",
        config("post R using v on Config::ConfigVersion::attemptUpdate { result = result }\n"),
    );
}
