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

const PROFILES: &str = "profile inf = \"filament.temporal.infinite-trace/v1\";\n\
                        profile ev = \"filament.temporal.event-position.false-extension/v1\";\n";

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
    let document = document();
    let (unit, _) = config_unit_with_profiles(&document, PROFILES, body);
    let declarations = admit_and_assemble_with_profiles(&document, PROFILES, body)
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
    let Err(mut refusals) = check(body) else {
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
#[ignore = "QSL-2141: a clock name other than model-steps or model-time over a model subject is not refused yet"]
fn a_model_subject_refuses_a_clock_name_other_than_the_two_bindings() {
    for profile in ["inf", "ev"] {
        for clock in ["ms", "wall", "steps"] {
            let (code, cause, named) = sole_refusal(&clocked(profile, clock));
            assert_eq!(
                (code, cause),
                (Code::UnsupportedConstruct, Some("declaration-form")),
                "{profile} {clock}"
            );
            assert_eq!(named.as_deref(), Some(format!("\"{clock}\"").as_str()));
        }
        check(&clocked(profile, "model-steps"))
            .unwrap_or_else(|refusals| panic!("{profile}: model-steps checks: {refusals:?}"));
    }
}

/// FR-165-AC-7: a state-graph claim `using` a temporal profile refuses
/// `unknown_profile`/`wrong-selection-role` at the alias; `using v` checks.
#[trace("TC-1545", "FR-165-AC-7")]
#[test]
#[ignore = "QSL-2150: a temporal profile as a state-graph claim's profile is not refused yet"]
fn a_state_graph_claim_refuses_a_temporal_profile() {
    for claim in [
        "possible Bad using inf over (c: Config::ConfigVersion) { c.versionNumber = 2 }",
        "always possible Bad using inf over (c: Config::ConfigVersion) { c.versionNumber = 2 }",
        "unique path Bad using inf over (c: Config::ConfigVersion) \
         from (c.versionNumber = 0) to (c.versionNumber = 2)",
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

/// The S2 outcome of the fixture unit with `body`: the forms refusal and the
/// text its span covers.
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
    let Err(refusal) = fsl_forms::build_unit(&parsed) else {
        panic!("{body}: S2 refuses it");
    };
    let span = refusal.span.expect("a located refusal");
    (
        refusal.catalog_code(),
        unit[span.start..span.end].to_owned(),
    )
}

/// FR-612-AC-1, FR-612-AC-2: an indexed read or a step label in a non-hyper
/// clause refuses `unsupported_construct` at the form, in the formula and in
/// the `on each ... when` guard alike, with no S1 syntax error.
#[trace("TC-1543", "FR-612-AC-1", "FR-612-AC-2")]
#[test]
#[ignore = "QSL-2143: hyper-only forms in a single-trace clause do not refuse uniformly yet"]
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
