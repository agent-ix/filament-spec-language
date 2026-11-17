// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-233-AC-4, FR-165-AC-7, FR-612 (TC-1540, TC-1545, TC-1543): over TC-458's
//! `ConfigVersion` fixture package (`crate::model_operations`), a temporal
//! clause over a model subject admits only the two model binding names as its
//! clock, a state-graph claim refuses a temporal profile as its `using`
//! alias, and a single-trace temporal clause refuses the hyper-only forms.

use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::checking::CheckingLimits;
use fsl_foundation::diagnostic::Code;
use fsl_foundation::SourceIdentity;
use fsl_semantics::check::{CheckRefusal, CheckedGraph};
use ix_trace_rs::trace;
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_profiles, attempt_update_modifies_version_number,
    config_unit_with_profiles, config_version_document,
};

const PROFILES: &str = "profile inf = \"filament.temporal.infinite-trace/v1\";\n";

/// The header profile line selecting `identity` under the alias `t`.
fn profile_t(identity: &str) -> String {
    format!("profile t = \"{identity}\";\n")
}

fn document() -> Vec<u8> {
    config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    )
}

/// The checked graph of `body`, or every refusal with the source text each
/// names.
fn check(body: &str) -> Result<CheckedGraph, Vec<(CheckRefusal, Option<String>)>> {
    check_with(PROFILES, body)
}

/// [`check`] with the extra header `profiles`.
fn check_with(
    profiles: &str,
    body: &str,
) -> Result<CheckedGraph, Vec<(CheckRefusal, Option<String>)>> {
    let document = document();
    let (unit, _) = config_unit_with_profiles(&document, profiles, body);
    let declarations = admit_and_assemble_with_profiles(&document, profiles, body)
        .unwrap_or_else(|refusal| panic!("assembly refused: {refusal:?}"));
    let regions = declarations.regions();
    declarations
        .check(CheckMode::Linked, CheckingLimits::default())
        .map_err(|refusals| {
            refusals
                .into_iter()
                .map(|refusal| {
                    let named = regions.refusal_region(&refusal).map(|region| {
                        let (start, end) = (region.start() as usize, region.end() as usize);
                        unit[start..end].to_owned()
                    });
                    (refusal, named)
                })
                .collect()
        })
}

/// The one refusal of `body`: its code, cause and the text it names.
fn sole_refusal(body: &str) -> (Code, Option<&'static str>, Option<String>) {
    sole_refusal_with(PROFILES, body)
}

/// [`sole_refusal`] with the extra header `profiles`.
fn sole_refusal_with(profiles: &str, body: &str) -> (Code, Option<&'static str>, Option<String>) {
    let Err(mut refusals) = check_with(profiles, body) else {
        panic!("`{body}` must not check");
    };
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    let (refusal, named) = refusals.remove(0);
    (refusal.cause.code(), refusal.cause.cause(), named)
}

/// A temporal clause over the model subject with `clock "{clock}"` under the
/// profile alias `profile`.
fn clocked(profile: &str, clock: &str) -> String {
    format!(
        "temporal Settles using {profile} over (c: Config::ConfigVersion) clock \"{clock}\" \
         on origin {{ eventually[0,3] holds(c.versionNumber = 2) }}"
    )
}

/// FR-233-AC-4: over a model subject a clock name other than `model-steps` or
/// `model-time` refuses `unsupported_construct`/`declaration-form` at the
/// clock string, under infinite-trace and event-position; `model-steps`
/// checks.
#[trace("TC-1540", "FR-233-AC-4", "FSpec-TC-1040", "FSpec-FR-090-AC-11")]
#[test]
fn a_model_subject_refuses_a_clock_name_other_than_the_two_bindings() {
    for profile in [
        "filament.temporal.infinite-trace/v1",
        "filament.temporal.event-position.false-extension/v1",
    ] {
        let header = profile_t(profile);
        for clock in ["ms", "wall", "steps"] {
            let (code, cause, named) = sole_refusal_with(&header, &clocked("t", clock));
            assert_eq!(
                (code, cause),
                (Code::UnsupportedConstruct, Some("declaration-form")),
                "{profile} {clock}"
            );
            assert_eq!(named.as_deref(), Some(format!("\"{clock}\"").as_str()));
        }
        check_with(&header, &clocked("t", "model-steps"))
            .unwrap_or_else(|refusals| panic!("{profile}: model-steps checks: {refusals:?}"));
    }
}

/// FR-165-AC-7: a state-graph claim `using` a temporal profile refuses
/// `unknown_profile`/`wrong-selection-role` at the alias; `using v` checks.
#[trace("TC-1545", "FR-165-AC-7")]
#[test]
fn a_state_graph_claim_refuses_a_temporal_profile() {
    for claim in [
        "possible Bad using inf over (c: Config::ConfigVersion) { c.versionNumber = 2 }",
        "always possible Bad using inf over (c: Config::ConfigVersion) { c.versionNumber = 2 }",
        "unique path Bad using inf over (c: Config::ConfigVersion) \
         from (c.versionNumber = 0) to (c.versionNumber = 2);",
    ] {
        let (code, cause, named) = sole_refusal(claim);
        assert_eq!(
            (code, cause),
            (Code::UnknownProfile, Some("wrong-selection-role")),
            "{claim}"
        );
        assert_eq!(named.as_deref(), Some("inf"), "{claim}");
    }
    check("possible Good using v over (c: Config::ConfigVersion) { c.versionNumber = 2 }")
        .unwrap_or_else(|refusals| panic!("`using v` checks: {refusals:?}"));
}

/// FR-165-AC-7, its protocol half: a state-graph claim `using` the protocol
/// profile refuses `unknown_profile`/`wrong-selection-role` at the alias, and
/// `using` the header layer `filament.temporal.complete/v1` checks.
#[trace("TC-1545", "FR-165-AC-7")]
#[test]
#[ignore = "QSL-2150: a state-graph claim using the protocol profile is not refused yet"]
fn a_state_graph_claim_refuses_the_protocol_profile() {
    let profiles = "profile pp = \"filament.protocol.finite-global/v1\";\n\
                    profile tc = \"filament.temporal.complete/v1\";\n";
    for claim in [
        "possible Bad using pp over (c: Config::ConfigVersion) { c.versionNumber = 2 }",
        "always possible Bad using pp over (c: Config::ConfigVersion) { c.versionNumber = 2 }",
        "unique path Bad using pp over (c: Config::ConfigVersion) \
         from (c.versionNumber = 0) to (c.versionNumber = 2);",
    ] {
        let (code, cause, named) = sole_refusal_with(profiles, claim);
        assert_eq!(
            (code, cause),
            (Code::UnknownProfile, Some("wrong-selection-role")),
            "{claim}"
        );
        assert_eq!(named.as_deref(), Some("pp"), "{claim}");
    }
    check_with(
        profiles,
        "possible Good using tc over (c: Config::ConfigVersion) { c.versionNumber = 2 }",
    )
    .unwrap_or_else(|refusals| panic!("`using tc` checks: {refusals:?}"));
}

/// The refusal of the fixture unit with `body` before a checked clause
/// exists, at S2 or S3: its code and the text it names.
fn forms_refusal(body: &str) -> (Code, String) {
    let document = document();
    let (unit, _) = config_unit_with_profiles(&document, PROFILES, body);
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "fr-612"),
        "unit.native",
        unit.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(
        parsed.is_admissible(),
        "{body}: S1 admits it: {:?}",
        parsed.diagnostics()
    );
    if let Err(refusal) = fsl_forms::build_unit(&parsed) {
        let span = refusal.span.expect("a located refusal");
        return (
            refusal.catalog_code(),
            unit[span.start..span.end].to_owned(),
        );
    }
    // S2 built the forms, so the refusal is S3's, before any checked clause.
    let (code, _, named) = sole_refusal(body);
    (code, named.unwrap_or_default())
}

/// FR-612-AC-1, FR-612-AC-2: an indexed read or a step label in a non-hyper
/// clause refuses `unsupported_construct` at the form, in the formula and in
/// the `on each ... when` guard alike, with no S1 syntax error.
#[trace("TC-1543", "FR-612-AC-1", "FR-612-AC-2")]
#[test]
fn hyper_only_forms_refuse_in_a_single_trace_clause() {
    for (body, at) in [
        (
            "temporal R using inf over (v: Config::ConfigVersion) clock \"model-steps\" \
             on origin { always holds(v.versionNumber @ a = 0) }",
            "v.versionNumber @ a",
        ),
        (
            "temporal R using inf over (v: Config::ConfigVersion) clock \"model-steps\" \
             on origin { always holds(a.step.attemptUpdate) }",
            "a.step.attemptUpdate",
        ),
        (
            "temporal R using inf over (v: Config::ConfigVersion) clock \"model-steps\" \
             on each (w: Config::ConfigVersion) when (w.versionNumber @ a = 0) \
             { always holds(w.versionNumber = 0) }",
            "w.versionNumber @ a",
        ),
    ] {
        assert_eq!(
            forms_refusal(body),
            (Code::UnsupportedConstruct, at.to_owned()),
            "{body}"
        );
    }
}

/// FR-326-AC-7 (TC-2180): a unit may select two temporal profiles; each
/// clause is checked under the profile its own `using` alias selects.
#[trace("TC-2180", "FR-326-AC-7")]
#[test]
fn clauses_of_one_unit_use_their_own_temporal_profiles() {
    let profiles = "profile ev = \"filament.temporal.event-position.false-extension/v1\";\n\
         profile inf = \"filament.temporal.infinite-trace/v1\";\n";
    let body = "temporal Bounded using ev over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin { always[0,3] holds(c.versionNumber >= 0) }\n\
         temporal Forever using inf over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin { always holds(c.versionNumber >= 0) }";
    check_with(profiles, body).unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
}

/// FR-233-AC-5: the timed profile reads model time, so `clock "model-steps"`
/// under it is a contradiction refused at the clock string; steps of a
/// timed model are counted under a step profile.
#[trace("TC-2389", "FR-233-AC-5")]
#[test]
fn the_timed_profile_refuses_model_steps() {
    const TIMED: &str =
        "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config dense unit ms;\n";
    let clause = |clock: Option<&str>, formula: &str| {
        let clock = clock
            .map(|name| format!("clock \"{name}\" "))
            .unwrap_or_default();
        format!(
            "{TIMED}temporal Settles using t over (c: Config::ConfigVersion) {clock}on origin {{ {formula} }}"
        )
    };
    // A unit's `time` declaration needs the complete temporal layer.
    let header = |identity: &str| {
        format!(
            "{}profile tc = \"filament.temporal.complete/v1\";\n",
            profile_t(identity)
        )
    };
    let timed = header("filament.temporal.timed/v1");
    let (code, cause, named) = sole_refusal_with(
        &timed,
        &clause(Some("model-steps"), "eventually holds(c.versionNumber = 2)"),
    );
    assert_eq!((code, cause), (Code::IllTyped, Some("type-mismatch")));
    assert_eq!(named.as_deref(), Some("\"model-steps\""));
    check_with(
        &header("filament.temporal.event-position.false-extension/v1"),
        &clause(
            Some("model-steps"),
            "eventually[0,3] holds(c.versionNumber = 2)",
        ),
    )
    .unwrap_or_else(|refusals| panic!("model-steps under event-position checks: {refusals:?}"));
    check_with(
        &timed,
        &clause(None, "eventually[0 ms, 3 ms] holds(c.versionNumber = 2)"),
    )
    .unwrap_or_else(|refusals| panic!("model-time under the timed profile checks: {refusals:?}"));
    let (code, cause, named) = sole_refusal_with(
        &timed,
        &clause(Some("events"), "eventually holds(c.versionNumber = 2)"),
    );
    assert_eq!(
        (code, cause),
        (Code::UnsupportedConstruct, Some("declaration-form"))
    );
    assert_eq!(named.as_deref(), Some("\"events\""));
}

/// FR-233-AC-5's time declaration of the ConfigVersion model.
const TIMED_MODEL: &str =
    "dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config dense unit ms;\n";

/// A clause `Settles` over the model subject under the alias `t`, with no
/// `clock` member, its body `formula`, after the header lines `before`.
fn unclocked(before: &str, formula: &str) -> String {
    format!(
        "{before}temporal Settles using t over (c: Config::ConfigVersion) \
         on origin {{ {formula} }}"
    )
}

/// The clock binding name of the sole clause `Settles` of `graph`.
fn sole_binding(graph: &CheckedGraph) -> String {
    let clauses = graph.temporal_clauses();
    assert_eq!(clauses.len(), 1, "one clause");
    clauses[0].clause().clock().to_owned()
}

/// FR-942-AC-1, FSpec FR-765-AC-1: over a timed model under a step profile,
/// a clause with no `clock` member binds `ModelSteps`, as the written default
/// does, and a time-unit interval then refuses as a type mismatch.
#[trace("TC-2622", "FR-942-AC-1", "FSpec-TC-1840", "FSpec-FR-765-AC-1")]
#[test]
fn an_unwritten_clock_under_a_step_profile_binds_model_steps_over_a_timed_model() {
    // The model's `time` declaration needs the complete temporal layer.
    let steps = format!(
        "{}profile tc = \"filament.temporal.complete/v1\";\n",
        profile_t("filament.temporal.event-position.false-extension/v1")
    );
    let unwritten = check_with(
        &steps,
        &unclocked(TIMED_MODEL, "eventually[0,3] holds(c.versionNumber = 2)"),
    )
    .unwrap_or_else(|refusals| panic!("the default binding checks: {refusals:?}"));
    let written = check_with(
        &steps,
        &format!(
            "{TIMED_MODEL}temporal Settles using t over (c: Config::ConfigVersion) \
             clock \"model-steps\" on origin {{ eventually[0,3] holds(c.versionNumber = 2) }}"
        ),
    )
    .unwrap_or_else(|refusals| panic!("model-steps checks: {refusals:?}"));
    assert_eq!(sole_binding(&unwritten), sole_binding(&written));
    assert_eq!(sole_binding(&unwritten), "model-steps");
    let (code, cause, _) = sole_refusal_with(
        &steps,
        &unclocked(
            TIMED_MODEL,
            "eventually[0 ms, 3 ms] holds(c.versionNumber = 2)",
        ),
    );
    assert_eq!((code, cause), (Code::IllTyped, Some("type-mismatch")));
}

/// FR-942-AC-2, FSpec FR-765-AC-2: under the timed profile over an untimed
/// model a clause with no `clock` member refuses at its `using` alias; under
/// infinite-trace it binds `ModelSteps`.
#[trace("TC-2622", "FR-942-AC-2", "FSpec-TC-1840", "FSpec-FR-765-AC-2")]
#[test]
fn an_unwritten_clock_under_the_timed_profile_needs_a_timed_model() {
    let (code, cause, named) = sole_refusal_with(
        &profile_t("filament.temporal.timed/v1"),
        &unclocked("", "eventually[0 ms, 3 ms] holds(c.versionNumber = 2)"),
    );
    assert_eq!(
        (code, cause),
        (Code::InvalidTimedModel, Some("model-time-on-untimed-model"))
    );
    assert_eq!(named.as_deref(), Some("t"));
    let graph = check_with(
        &profile_t("filament.temporal.infinite-trace/v1"),
        &unclocked("", "eventually holds(c.versionNumber = 2)"),
    )
    .unwrap_or_else(|refusals| panic!("the default binding checks: {refusals:?}"));
    assert_eq!(sole_binding(&graph), "model-steps");
}

/// FR-943-AC-1, FSpec FR-766-AC-1: under the timed profile a lower-bounded
/// interval with no upper bound checks on past and future operators, and a
/// unitless lower bound refuses as a type mismatch.
#[trace("TC-2623", "FR-943-AC-1", "FSpec-TC-1841", "FSpec-FR-766-AC-1")]
#[test]
#[ignore = "QSL-2321: a timed interval [q, *] is refused OpenInterval under the timed profile"]
fn a_timed_interval_with_no_upper_bound_checks() {
    let timed = profile_t("filament.temporal.timed/v1");
    for formula in [
        "once[5 ms, *] holds(c.versionNumber = 2)",
        "eventually[5 ms, *] holds(c.versionNumber = 2)",
        "once(5 ms, *) holds(c.versionNumber = 2)",
    ] {
        check_with(&timed, &unclocked(TIMED_MODEL, formula))
            .unwrap_or_else(|refusals| panic!("`{formula}` checks: {refusals:?}"));
    }
    let (code, cause, _) = sole_refusal_with(
        &timed,
        &unclocked(TIMED_MODEL, "once[5, *] holds(c.versionNumber = 2)"),
    );
    assert_eq!((code, cause), (Code::IllTyped, Some("type-mismatch")));
}
