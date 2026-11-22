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
    let packages = fsl_semantics::model::intake::package_input([LINK_MODEL])
        .expect("the Link model admits");
    let limits = fsl_package::V2ReadLimits::bounded(1 << 24);
    let admitted = fsl_package::admit_checked_package(&bytes, &packages, limits)
        .expect("the actual producer package admits");
    let mut budget = limits.native_join_limits().budget();
    let identities = ReadIdentityRows::read(
        &std::fs::read(ws.path(&format!("{HANDOFF}/identities.json")))
            .expect("handoff identities"),
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

    std::fs::remove_file(ws.path("program.fsl")).expect("delete the source program");
    std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
        .expect("delete retained source copies");
    assert!(!ws.path("program.fsl").exists());
    assert!(!ws.path(&format!("{HANDOFF}/sources")).exists());
    let package = analyze(&ws.write(
        "analyze-package.json",
        json!({"format": "native-analyze/1", "request": members})
            .to_string()
            .as_bytes(),
    ));
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
        .evaluate_state_predicate(
            predicate,
            &objects,
            Some(over),
            Vec::new(),
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
