// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact refutations retain their negative witness, replay and charges from bytes.

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
const TOO_LIKELY: &str =
    "probabilistic TooLikely using inf on M over (m: M::Msg) under Even on origin \
     { probability >= rational(99, 100) [ eventually holds(m.delivered) ] }";

/// EN-5 refutes 99/100 because the exact probability is 391/400. Its
/// independent negative-witness replay must reproduce without source files.
#[trace("FR-940-AC-6", "FR-741-AC-3", "FR-203-AC-8")]
#[test]
fn a_package_refutation_preserves_the_negative_witness_replay_and_charges() {
    let ws = Workspace::new("probabilistic-package-refutation-parity");
    let source_path = request_over(
        &ws,
        &format!("{LINK_UNIT}\n{TOO_LIKELY}\n"),
        LINK_MODEL,
        &[LINK_START.to_vec()],
        json!({"items": [{"claim": "TooLikely", "kind": "probabilistic-satisfaction",
            "probabilistic": {"evidence": "exact"}}]}),
    );
    let source = analyze(&source_path);
    let source_item = source.item(0);
    assert_eq!(source.exit, 10, "{}", source.document());
    assert_eq!(label(&source_item), "refuted", "{source_item}");
    assert_eq!(source_item["record"]["backend"], "en-5");
    assert_eq!(source_item["counterexample"]["type"], "probabilistic");
    assert_eq!(
        source_item["counterexample"]["evidence"]["type"],
        "path-set"
    );
    assert!(
        source_item["counterexample"]["evidence"]["paths"]
            .as_array()
            .is_some_and(|paths| !paths.is_empty()),
        "{source_item}"
    );
    assert_eq!(source_item["replay"]["reproduced"], true, "{source_item}");
    assert!(
        source_item
            .get("certificate")
            .is_none_or(|value| value.is_null()),
        "{source_item}"
    );

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
    let selected = fsl_package::AnalysisView::claim_name(&bare, "TooLikely")
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

    let checked = compiled.checked();
    let claim = checked
        .graph()
        .probabilistic_model()
        .claim("TooLikely")
        .expect("the refuted claim is checked");
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

    let package = analyze(
        &ws.write(
            "refutation-package.json",
            json!({"format": "native-analyze/1", "request": members})
                .to_string()
                .as_bytes(),
        ),
    );
    assert_eq!(package.exit, source.exit, "{}", package.document());
    assert_eq!(
        package.item(0)["counterexample"],
        source_item["counterexample"]
    );
    assert_eq!(package.item(0)["replay"], source_item["replay"]);
    assert_eq!(package.item(0)["replay"]["reproduced"], true);
    assert_eq!(package.stdout, source.stdout, "whole serialized result");
}
