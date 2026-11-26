// SPDX-License-Identifier: AGPL-3.0-or-later
//! Probabilistic satisfaction retains its source outcome and evaluator charges from bytes.

use filament_exact::{Meter, Value};
use fsl_eval::simulation::model::ModelEvaluator;
use fsl_package::{BarePackage, BareSupply, ReadEqualityRows, ReadIdentityRows};
use fsl_semantics::check::{ProbForm, ReachTarget};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::observation::{
    admit_trace_snapshot, document_reference, DocumentProvision, ObservationLimits,
};
use ix_trace_rs::trace;
use serde_json::json;

use crate::analyze_cli_support::{
    analyze, label, package_selection, request_over, Workspace, HANDOFF,
};
use crate::counter::UNLIMITED;

const LINK_UNIT: &str =
    include_str!("../../../tools/fsl-probability-check/tests/fixtures/link.native");
const LINK_MODEL: &[u8] =
    include_bytes!("../../../tools/fsl-probability-check/tests/fixtures/link.semantic-ir.json");
const LINK_START: &[u8] =
    include_bytes!("../../../tools/fsl-probability-check/tests/fixtures/link.snapshot.json");
const EVENTUALLY: &str =
    "probabilistic Eventually using inf on M over (m: M::Msg) under Even on origin \
     { probability >= rational(9, 10) [ eventually holds(m.delivered) ] }";

/// A finite-window probability form is not admitted by the package
/// eventuality projection, and cannot yield a certificate or verdict.
#[trace("FR-940-AC-6", "FR-741-AC-2")]
#[test]
fn an_unimplemented_probability_package_form_keeps_its_typed_refusal() {
    let ws = Workspace::new("probabilistic-package-bounded-refusal");
    let unit = LINK_UNIT.replace(
        "profile inf = \"filament.temporal.infinite-trace/v1\";",
        "profile inf = \"filament.temporal.infinite-trace/v1\";\n\
         profile ev = \"filament.temporal.event-position.false-extension/v1\";",
    );
    let path = request_over(
        &ws,
        &format!(
            "{unit}\nprobabilistic Bounded using ev on M over (m: M::Msg) under Even on origin \
            {{ probability >= rational(9, 10) [ eventually[0,2] holds(m.delivered) ] }}\n"
        ),
        LINK_MODEL,
        &[LINK_START.to_vec()],
        json!({"items": [{"claim": "Bounded", "kind": "probabilistic-satisfaction",
            "probabilistic": {"evidence": "exact"}}]}),
    );
    let source = analyze(&path);
    assert_eq!(source.exit, 0, "{}", source.document());
    assert_eq!(source.item(0)["record"]["backend"], "en-5");
    let request: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("source request"))
            .expect("request JSON");
    let (members, _) = package_selection(&ws, &request["request"]);
    std::fs::remove_file(ws.path("program.fsl")).expect("delete source program");
    std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
        .expect("delete retained sources");
    let package = analyze(
        &ws.write(
            "bounded-package.json",
            json!({"format": "native-analyze/1", "request": members})
                .to_string()
                .as_bytes(),
        ),
    );
    let document = package.document();
    assert_eq!(package.exit, 21, "{document}");
    assert_eq!(document["diagnostics"][0]["code"], "unsupported_construct");
    assert_eq!(document["diagnostics"][0]["cause"], "not-yet-implemented");
    assert!(
        document["diagnostics"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("missing-claim-profile")),
        "{document}"
    );
    assert_eq!(document["items"], json!([]));
    assert!(document.get("certificate").is_none());
    assert!(document.get("counterexample").is_none());
}

fn assert_missing_profile_without_sources(ws: &Workspace, members: &serde_json::Value) {
    let (members, _) = package_selection(ws, members);
    std::fs::remove_file(ws.path("program.fsl")).expect("delete source program");
    std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
        .expect("delete retained sources");
    let package = analyze(
        &ws.write(
            "excluded-package.json",
            json!({"format": "native-analyze/1", "request": members})
                .to_string()
                .as_bytes(),
        ),
    );
    let document = package.document();
    assert_eq!(package.exit, 21, "{document}");
    assert_eq!(document["diagnostics"][0]["code"], "unsupported_construct");
    assert_eq!(document["diagnostics"][0]["cause"], "not-yet-implemented");
    assert!(
        document["diagnostics"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("missing-claim-profile")),
        "{document}"
    );
    assert_eq!(document["items"], json!([]));
    assert!(document.get("certificate").is_none());
    assert!(document.get("counterexample").is_none());
}

/// The unbounded exact-form derivation never authorizes a statistical item.
#[trace("FR-940-AC-6", "FR-741-AC-4", "FSpec-FR-641-AC-3")]
#[test]
fn a_statistical_package_item_refuses_the_missing_claim_profile() {
    let ws = Workspace::new("probabilistic-package-statistical-refusal");
    let path = request_over(
        &ws,
        &format!("{LINK_UNIT}\n{EVENTUALLY}\n"),
        LINK_MODEL,
        &[LINK_START.to_vec()],
        json!({"items": [{"claim": "Eventually", "kind": "probabilistic-satisfaction",
            "probabilistic": {"evidence": "statistical", "method": {"type": "okamoto"},
                "seed": "0", "max_witnesses": 0}}]}),
    );
    let source = analyze(&path);
    assert_eq!(source.exit, 21, "{}", source.document());
    assert_eq!(label(&source.item(0)), "unsupported");
    assert!(source.item(0).to_string().contains("exact-only-form"));
    assert!(source.item(0).get("measurement").is_none());
    let request: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("source request"))
            .expect("request JSON");
    assert_missing_profile_without_sources(&ws, &request["request"]);
}

/// Every other profile-dependent projection category is refused even when
/// the source engine proves it: a bounded comparison event, a quantile,
/// a mean fraction and a long-run fraction. The bounded formula is the
/// separate finite-window control above.
#[trace("FR-940-AC-6", "FR-741-AC-2")]
#[test]
fn other_probability_forms_refuse_the_missing_claim_profile() {
    for (name, profile, body) in [
        (
            "Compare",
            "ev",
            "probability >= 0.5 [ steps from holds(true) until holds(m.delivered) within 2 <= 2 ]",
        ),
        (
            "Quantile",
            "ev",
            "quantile 0.4 of (steps from holds(true) until holds(m.delivered) within 4) <= 3",
        ),
        (
            "Mean",
            "ev",
            "mean of (fraction holds(m.delivered) over [0, 2]) >= 0.1",
        ),
        (
            "LongRun",
            "inf",
            "long_run_fraction holds(m.delivered) >= rational(391, 400)",
        ),
    ] {
        let ws = Workspace::new(&format!("probabilistic-package-excluded-{name}"));
        let unit = LINK_UNIT.replace(
            "profile inf = \"filament.temporal.infinite-trace/v1\";",
            "profile inf = \"filament.temporal.infinite-trace/v1\";\n\
             profile ev = \"filament.temporal.event-position.false-extension/v1\";",
        );
        let path = request_over(
            &ws,
            &format!("{unit}\nprobabilistic {name} using {profile} on M over (m: M::Msg) under Even on origin {{ {body} }}\n"),
            LINK_MODEL, &[LINK_START.to_vec()],
            json!({"items": [{"claim": name, "kind": "probabilistic-satisfaction",
                "probabilistic": {"evidence": "exact"}}]}),
        );
        let source = analyze(&path);
        assert_eq!(source.exit, 0, "{name}: {}", source.document());
        assert_eq!(
            label(&source.item(0)),
            "proved",
            "{name}: {}",
            source.document()
        );
        assert_eq!(source.item(0)["record"]["backend"], "en-5", "{name}");
        assert_eq!(source.item(0)["certificate"]["accepted"], true, "{name}");
        let request: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).expect("source request"))
                .expect("request JSON");
        assert_missing_profile_without_sources(&ws, &request["request"]);
    }
}

/// EN-5's exact probability is 391/400, with a checked certificate. The
/// package request has only the compiled handoff, domain model and snapshot.
#[trace("FR-940-AC-6", "FR-741-AC-2")]
#[test]
fn probabilistic_package_preserves_whole_result_exit_and_predicate_charges() {
    let ws = Workspace::new("probabilistic-package-whole-parity");
    let source_path = request_over(
        &ws,
        &format!("{LINK_UNIT}\n{EVENTUALLY}\n"),
        LINK_MODEL,
        &[LINK_START.to_vec()],
        json!({"items": [{"claim": "Eventually", "kind": "probabilistic-satisfaction",
            "probabilistic": {"evidence": "exact"}}]}),
    );
    let source = analyze(&source_path);
    assert_eq!(source.exit, 0, "{}", source.document());
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    assert_eq!(source.item(0)["record"]["backend"], "en-5");
    assert_eq!(source.item(0)["certificate"]["accepted"], true);
    assert_eq!(source.item(0)["certificate"]["kind"], "exact");
    let request: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&source_path).expect("source request"))
            .expect("request JSON");
    let (members, bytes) = package_selection(&ws, &request["request"]);
    let compiled = fsl_command::checked_handoff::compile(&ws.path(HANDOFF))
        .expect("source handoff compiles for evaluator comparison");
    let packages =
        fsl_semantics::model::intake::package_input([LINK_MODEL]).expect("the Link model admits");
    let limits = fsl_package::V2ReadLimits::bounded(1 << 24);
    let admitted = fsl_package::admit_checked_package(&bytes, &packages, limits)
        .expect("the actual producer package admits");
    let mut budget = limits.native_join_limits().budget();
    let identities = ReadIdentityRows::read(
        &std::fs::read(ws.path(&format!("{HANDOFF}/identities.json"))).expect("handoff identities"),
        &admitted,
        &mut budget,
    )
    .expect("identity rows admit");
    let equality = std::fs::read(ws.path(&format!("{HANDOFF}/equality.json")))
        .ok()
        .map(|bytes| ReadEqualityRows::read(&bytes, &admitted).expect("equality admits"));
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the handoff supply joins");
    let selected = fsl_package::AnalysisView::claim_name(&bare, "Eventually")
        .expect("the package names its probabilistic claim");
    let occurrence = fsl_foundation::source::provenance::OccurrenceKey::new(
        bare.node_key(&selected.declaration)
            .expect("the claim has a structural key"),
        filament_exact::Origin::new(
            filament_exact::Role::new("claim"),
            selected.occurrence.ordinal,
        ),
    );
    let package_form = bare
        .probability_claim_form(&occurrence)
        .expect("the exact eventuality form reads from the package");

    std::fs::remove_file(ws.path("program.fsl")).expect("delete the source program");
    std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
        .expect("delete retained source copies");
    assert!(!ws.path("program.fsl").exists());
    assert!(!ws.path(&format!("{HANDOFF}/sources")).exists());
    let package = analyze(
        &ws.write(
            "analyze-package.json",
            json!({"format": "native-analyze/1", "request": members})
                .to_string()
                .as_bytes(),
        ),
    );
    assert_eq!(package.exit, source.exit, "{}", package.document());
    assert_eq!(package.stdout, source.stdout, "whole serialized result");

    let checked = compiled.checked();
    let claim = checked
        .graph()
        .probabilistic_model()
        .claim("Eventually")
        .expect("the exact claim is checked");
    let ProbForm::Reach {
        target: ReachTarget::Eventually(predicate),
        ..
    } = &claim.form
    else {
        panic!("the fixture is an exact eventuality");
    };
    let selected = document_reference(LINK_START).expect("snapshot identity");
    let provision = DocumentProvision::from([(selected.clone(), LINK_START.to_vec())]);
    let (objects, _) = admit_trace_snapshot(
        checked.graph().model_selections(),
        checked.graph().scope().types(),
        compiled.packages(),
        ModelNormalizationLimits::default(),
        &provision,
        &selected,
        ObservationLimits::default(),
        &[],
    )
    .expect("the Link snapshot admits");
    let over = objects
        .references()
        .iter()
        .find(|reference| reference.object().as_str() == "m")
        .expect("the message is present")
        .clone();
    let (mut source_meter, mut package_meter) = (Meter::new(UNLIMITED), Meter::new(UNLIMITED));
    let expected = checked
        .evaluate_state_predicate(
            predicate,
            &objects,
            Some(over.clone()),
            Vec::new(),
            &mut source_meter,
        )
        .expect("the source probability predicate evaluates");
    let actual = bare
        .evaluate_probability_predicate(
            &package_form.occurrence,
            &objects,
            Some(over),
            &mut package_meter,
        )
        .expect("the package probability predicate evaluates");
    assert!(matches!(
        &expected.outcome,
        filament_eval::FamilyOutcome::Evaluated(filament_exact::Outcome::Completed(
            Value::Boolean(false)
        ))
    ));
    assert_eq!(
        format!("{:?}", actual.outcome),
        format!("{:?}", expected.outcome)
    );
    assert_eq!(actual.location, expected.location);
    assert!(!source_meter.admitted_charges().is_empty());
    assert!(!source_meter.charge_log_truncated());
    assert!(!package_meter.charge_log_truncated());
    assert_eq!(
        package_meter.admitted_charges(),
        source_meter.admitted_charges()
    );
}

/// The caller's `exact_prob.max_states` bounds an exact probabilistic item
/// on the source route; an item that states the same bound in its own
/// `limits` member is refused whichever is larger (FR-255 "Lifecycle
/// caller-limit coverage").
#[trace("TC-721", "FR-255-AC-24", "FR-255-AC-25")]
#[test]
fn caller_exact_prob_max_states_bounds_the_item_and_refuses_a_duplicate() {
    use fsl_foundation::Setting;

    let ws = Workspace::new("caller-exact-prob-max-states");
    let path = request_over(
        &ws,
        &format!("{LINK_UNIT}\n{EVENTUALLY}\n"),
        LINK_MODEL,
        &[LINK_START.to_vec()],
        json!({"items": [{"claim": "Eventually", "kind": "probabilistic-satisfaction",
            "probabilistic": {"evidence": "exact"}}]}),
    );
    let run = crate::analyze_cli_support::analyze_under(&path, &[(Setting::ExactProbMaxStates, 1)]);
    let text = run.document().to_string();
    assert_ne!(run.exit, 0, "{text}");
    assert!(text.contains("exact_prob.max_states"), "{text}");

    let stated = Workspace::new("caller-exact-prob-max-states-stated");
    let stated_path = request_over(
        &stated,
        &format!("{LINK_UNIT}\n{EVENTUALLY}\n"),
        LINK_MODEL,
        &[LINK_START.to_vec()],
        json!({"items": [{"claim": "Eventually", "kind": "probabilistic-satisfaction",
            "probabilistic": {"evidence": "exact", "limits": {"max_states": 100000}}}]}),
    );
    let refused = crate::analyze_cli_support::analyze_under(
        &stated_path,
        &[(Setting::ExactProbMaxStates, 1)],
    );
    assert_eq!(refused.exit, 20, "{}", refused.document());
    assert!(refused.document().to_string().contains("exact_prob.max_states"));
}

/// FR-255-AC-25 on the package route: the caller's `exact_prob.max_states`
/// bounds the item over the compiled handoff alone, and a stated duplicate
/// is refused there too.
#[trace("TC-721", "FR-255-AC-25")]
#[test]
fn caller_exact_prob_max_states_bounds_the_package_route_too() {
    use fsl_foundation::Setting;

    for (name, members, refused) in [
        (
            "silent",
            json!({"items": [{"claim": "Eventually", "kind": "probabilistic-satisfaction",
                "probabilistic": {"evidence": "exact"}}]}),
            false,
        ),
        (
            "stated",
            json!({"items": [{"claim": "Eventually", "kind": "probabilistic-satisfaction",
                "probabilistic": {"evidence": "exact", "limits": {"max_states": 100000}}}]}),
            true,
        ),
    ] {
        let ws = Workspace::new(&format!("caller-exact-prob-package-{name}"));
        let path = request_over(
            &ws,
            &format!("{LINK_UNIT}\n{EVENTUALLY}\n"),
            LINK_MODEL,
            &[LINK_START.to_vec()],
            members,
        );
        let request: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).expect("source request"))
                .expect("request JSON");
        let (selection, _) = package_selection(&ws, &request["request"]);
        std::fs::remove_file(ws.path("program.fsl")).expect("delete source program");
        std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
            .expect("delete retained sources");
        let package_path = ws.write(
            "package.json",
            json!({"format": "native-analyze/1", "request": selection})
                .to_string()
                .as_bytes(),
        );
        let run = crate::analyze_cli_support::analyze_under(
            &package_path,
            &[(Setting::ExactProbMaxStates, 1)],
        );
        let text = run.document().to_string();
        if refused {
            assert_eq!(run.exit, 20, "{name}: {text}");
        } else {
            assert_ne!(run.exit, 0, "{name}: {text}");
            assert_ne!(run.exit, 20, "{name}: {text}");
        }
        assert!(text.contains("exact_prob.max_states"), "{name}: {text}");
    }
}

/// FR-255-AC-25: a statistical item that states a limit in its own member
/// while the caller passes the same setting is refused `invalid-request`
/// naming the setting; this covers `limits.max_samples`, `max_draws`,
/// `max_cycle_steps` and `timed_sampling.max_sample_steps`.
#[trace("TC-721", "FR-255-AC-25")]
#[test]
fn a_statistical_item_and_the_caller_may_not_both_state_a_limit() {
    use fsl_foundation::Setting;

    for (member, value, setting) in [
        ("limits", json!({"max_samples": 100}), Setting::StatisticalMaxSamples),
        ("limits", json!({"max_draws": 100}), Setting::StatisticalMaxDraws),
        ("limits", json!({"max_cycle_steps": 100}), Setting::StatisticalMaxCycleSteps),
        (
            "timed_sampling",
            json!({"q": 8, "max_sample_steps": 100}),
            Setting::StatisticalMaxSampleSteps,
        ),
    ] {
        let ws = Workspace::new(&format!("caller-statistical-stated-{setting}"));
        let mut probabilistic = json!({"evidence": "statistical",
            "method": {"type": "okamoto"}, "seed": "0", "max_witnesses": 0});
        probabilistic[member] = value;
        let path = request_over(
            &ws,
            &format!("{LINK_UNIT}\n{EVENTUALLY}\n"),
            LINK_MODEL,
            &[LINK_START.to_vec()],
            json!({"items": [{"claim": "Eventually", "kind": "probabilistic-satisfaction",
                "probabilistic": probabilistic}]}),
        );
        let refused = crate::analyze_cli_support::analyze_under(&path, &[(setting, 3)]);
        let text = refused.document().to_string();
        assert_eq!(refused.exit, 20, "{setting}: {text}");
        assert!(text.contains(&setting.to_string()), "{setting}: {text}");
    }
}
