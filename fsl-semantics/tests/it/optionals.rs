// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-025 to TC-029 and TC-895: optional values, presence facts, guarded
//! bounded addition, ambiguous scalar inference, Boolean clause roots and
//! optionality read from a field's own presence.
//!
//! Every case is a pair: the adverse form is refused with its typed code and
//! cause, and a near-identical control is accepted, so a checker that refused
//! (or accepted) everything fails one side. The ConfigVersion package of
//! TC-458 supplies `versionNumber: Int[0, 1000]` and the optional `parent`.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_foundation::Code;
use fsl_semantics::check::CheckedGraph;
use fsl_semantics::model::intake::package_input;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::imported_call::{compile, HEADER};
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

#[trace("TC-027")]
#[test]
fn guarded_addition_proves_the_bound_and_weakened_guards_do_not() {
    let version = |body: &str| {
        compile(
            &format!(
                "{HEADER}type Version = Int[0, 1000];\n\
                 function next using v(x: Version): Version pure {{ {body} }}\n"
            ),
            Vec::new(),
        )
    };
    version("if x < 1000 then x + 1 else x").expect("the strict guard proves x + 1 <= 1000");
    for (label, body) in [
        ("no guard", "x + 1"),
        (
            "guard at the boundary still permits overflow",
            "if x <= 1000 then x + 1 else x",
        ),
        ("weaker guard", "if x < 1001 then x + 1 else x"),
    ] {
        let refusals = version(body).expect_err(label);
        assert_eq!(refusals.len(), 1, "{label}: {refusals:?}");
        assert_eq!(
            (refusals[0].cause.code(), refusals[0].cause.cause()),
            (Code::UndefinedExpression, Some("unproved-range")),
            "{label}"
        );
    }
}

#[trace("TC-028")]
#[test]
fn a_literal_without_a_context_is_refused_and_one_with_a_context_checks() {
    let amount = "type Amount = Decimal[-10000, 10000; 3, 3; exact];\n";
    // A `let` does not borrow the context of its later use.
    let ambiguous = compile(
        &format!(
            "{HEADER}{amount}function f using v(): Amount pure \
             {{ let total = decimal(125, 2) + decimal(375, 2) in total }}\n"
        ),
        Vec::new(),
    )
    .expect_err("context-free decimal literals are ambiguous");
    assert_eq!(ambiguous.len(), 1, "{ambiguous:?}");
    assert_eq!(ambiguous[0].cause.code(), Code::IllTyped);
    assert_eq!(ambiguous[0].cause.cause(), Some("ambiguous-literal"));
    // The same sum under the result type is contextual and checks.
    compile(
        &format!(
            "{HEADER}{amount}function f using v(): Amount pure \
             {{ decimal(125, 2) + decimal(375, 2) }}\n"
        ),
        Vec::new(),
    )
    .expect("the declared result type supplies the context");
    // An explicit Count context for a collection size.
    compile(
        &format!(
            "{HEADER}type Count = Int[0, 1000];\n\
             function f using v(c: Sequence<Integer>[0, 4]): Boolean pure {{ size<Count>(c) = 2 }}\n"
        ),
        Vec::new(),
    )
    .expect("explicit Count context");
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

// ---- TC-895: optionality is read from presence, never from multiplicity ----

#[path = "../support/tc235_k2.rs"]
#[allow(clippy::duplicate_mod, reason = "shared K2 builder")]
mod carrier;
#[path = "../support/variant_fields.rs"]
#[allow(clippy::duplicate_mod, reason = "shared K2 builder")]
mod fields;

fn multiplicity(mut field: Value, lower: u64, upper: u64) -> Value {
    field["multiplicity"] =
        json!({"lower": lower, "upper": upper, "ordered": false, "unique": true});
    field
}

fn object_with(presence: &str, lower: u64, upper: u64) -> Value {
    let boolean = "ix://filament/native/Boolean";
    let mut f = fields::field("T", "f", boolean);
    f["presence"] = json!(presence);
    let t = carrier::declaration(
        "T",
        "thing",
        json!({"identityFields":[carrier::id("T/id")],"fields":[
            fields::field("T", "id", "ix://filament/native/Integer"),
            multiplicity(f, lower, upper)]}),
    );
    let mut document = fields::package(vec![t], true);
    document["populations"] = json!([{
        "identity":carrier::id("ts"),"displayName":"ts",
        "kind":{"module":carrier::BUSINESS,"name":"population"},
        "origin":carrier::origin("ts"),"extent":"closed","members":[carrier::id("T")]
    }]);
    document
}

fn check_field(
    presence: &str,
    lower: u64,
    upper: u64,
    expression: &str,
) -> Result<CheckedGraph, Vec<(Code, Option<&'static str>)>> {
    let bytes = serde_json::to_vec(&object_with(presence, lower, upper)).unwrap();
    let packages = package_input([bytes.as_slice()]).unwrap();
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model M = {:?};\n\
         invariant Clause using v on M::T at current {{ {expression} }}\n",
        carrier::PACKAGE
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "optionals"),
        "optionals.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2");
    let selected = fsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the package admits");
    let assembled = fsl_semantics::check::PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("assembly: {refusal:?}"));
    assembled
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .map_err(|refusals| {
            refusals
                .iter()
                .map(|refusal| (refusal.cause.code(), refusal.cause.cause()))
                .collect()
        })
}

#[trace("TC-895")]
#[test]
fn a_field_is_optional_by_its_presence_and_not_by_a_zero_lower_bound() {
    // Step 1: required [0, 1] is a sequence, so `present` is ill typed.
    let refusals = check_field("required", 0, 1, "present(self.f)").expect_err("step 1");
    assert_eq!(
        refusals.first().map(|r| r.0),
        Some(Code::IllTyped),
        "{refusals:?}"
    );
    // Step 2: optional [1, 1] is an option of Boolean.
    assert!(
        check_field("optional", 1, 1, "present(self.f) implies value(self.f)").is_ok(),
        "guarded unwrap checks"
    );
    let refusals = check_field("optional", 1, 1, "value(self.f)").expect_err("unguarded");
    assert_eq!(refusals, [UNPROVED_FIELD], "{refusals:?}");
    // Step 3: optional [0, 1] is an option of a sequence; `present` checks.
    assert!(check_field("optional", 0, 1, "present(self.f)").is_ok());
}

const UNPROVED_FIELD: (Code, Option<&str>) = (Code::UndefinedExpression, Some("unproved-presence"));
