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
    refutation_parity(
        "probabilistic-package-refutation-parity",
        LINK_UNIT,
        LINK_MODEL,
        TOO_LIKELY,
    );
}

/// Every-scheduler evidence retains enum-valued draws and non-random
/// arguments after the source files have been removed.
#[trace("FR-940-AC-6", "FR-741-AC-3", "FR-203-AC-8")]
#[test]
fn enum_draws_and_arguments_preserve_package_refutation_witness_parity() {
    let mut model: serde_json::Value = serde_json::from_slice(LINK_MODEL).expect("Link model JSON");
    let enum_source = b"language \"ix:native\" edition \"1-draft\";\n\
                        domain \"test/link\";\n\
                        enum Loss { Lost, Delivered }\n";
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "link/spec"),
        "spec.fspec",
        enum_source,
        fsl_cst::Limits::default(),
    )
    .expect("the enum model source parses");
    let compiled = fsl_semantics::model::intake::compile_model_unit(
        &parsed,
        ModelNormalizationLimits::default(),
    )
    .expect("the enum model unit compiles");
    let enum_document: serde_json::Value =
        serde_json::from_slice(compiled.bytes()).expect("compiled enum model JSON");
    let enum_declaration = enum_document["types"]
        .as_array()
        .expect("compiled types")
        .iter()
        .find(|declaration| declaration["displayName"] == "Loss")
        .expect("the actual enum declaration")
        .clone();
    let enum_type = enum_declaration["identity"]
        .as_str()
        .expect("the compiled enum identity")
        .to_owned();
    let enum_construct = enum_document["constructs"]
        .as_array()
        .expect("compiled constructs")
        .iter()
        .find(|construct| construct["kind"]["name"] == "variant_type")
        .expect("the actual enum construct")
        .clone();
    model["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(enum_construct);
    model["types"]
        .as_array_mut()
        .expect("types")
        .push(enum_declaration);
    let origin = |identity: &str| {
        json!({"generated": {
            "generatorIdentity": identity, "generatorVersion": "1.0.0",
            "inputIdentities": [identity]
        }})
    };
    for operation in model["types"][1]["operations"]
        .as_array_mut()
        .expect("message operations")
    {
        operation["params"][0]["typeRef"] = json!(enum_type);
        let mut argument = operation["params"][0].clone();
        let identity = format!(
            "{}/tag",
            operation["identity"].as_str().expect("operation identity")
        );
        argument["identity"] = json!(identity);
        argument["name"] = json!("tag");
        argument["origin"] = origin(&identity);
        operation["params"]
            .as_array_mut()
            .expect("parameters")
            .push(argument);
    }
    let unit = LINK_UNIT
        .replace(
            "(not lost)",
            "(lost = M::Loss::Delivered) and tag = M::Loss::Delivered",
        )
        .replace("true: 1, false: 9", "M::Loss::Lost: 1, M::Loss::Delivered: 9")
        .replace("true: 1, false: 4", "M::Loss::Lost: 1, M::Loss::Delivered: 4");
    let item = refutation_parity(
        "probabilistic-package-enum-refutation-parity",
        &unit,
        &serde_json::to_vec(&model).expect("enum model JSON"),
        &TOO_LIKELY.replace("under Even", "under every scheduler"),
    );
    let witness = &item["counterexample"];
    let paths = witness["evidence"]["paths"].as_array().expect("path set");
    assert!(
        paths.iter().any(|path| path.as_array().is_some_and(|steps| {
            steps.iter().any(|step| step["random"]
                .as_array()
                .is_some_and(|draw| draw.iter().any(|value| value["type"] == "enum")))
        })),
        "{witness}"
    );
    let entries = witness["scheduler"]["entries"]
        .as_array()
        .expect("scheduler entries");
    assert!(
        entries.iter().any(|entry| entry["choice"]["arguments"]
            .as_array()
            .is_some_and(|arguments| arguments.iter().any(|value| value["type"] == "enum"))),
        "{witness}"
    );
}

fn refutation_parity(name: &str, unit: &str, model: &[u8], claim: &str) -> serde_json::Value {
    let ws = Workspace::new(name);
    let source_path = request_over(
        &ws,
        &format!("{unit}\n{claim}\n"),
        model,
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
        fsl_semantics::model::intake::package_input([model]).expect("the Link model admits");
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
    source_item
}

/// Replaying the real negative witness requires its actual bare execution
/// owner. Equal package bytes do not authorize mixed tables or another owner.
#[trace("FR-940-AC-6", "FR-202-AC-10", "FR-203-AC-8")]
#[test]
fn package_negative_replay_authenticates_the_owner_and_complete_claim_occurrence() {
    use filament_exact::Cancel;
    use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
    use fsl_eval::simulation::model::{ModelCheckInput, ModelSystem, PackageModelInput};
    use fsl_foundation::RequestIndex;
    use fsl_replay::{
        replay_probabilistic_witness, replay_probabilistic_witness_from_package,
        ModelCheckLimits, ProbabilityWitnessError, ProbabilityWitnessReplay, TerminalValue,
    };

    let world = crate::prob_world::World::new(&crate::prob_world::link(TOO_LIKELY));
    let record = analyze_items(
        &Engines::fsl().expect("linked engines"),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("TooLikely", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    )
    .remove(0);
    assert_eq!(
        record.terminal().expect("the negative source settles").value(),
        &TerminalValue::Refuted
    );
    let found = record
        .probabilistic_counterexample
        .expect("the negative source carries independently replayed evidence");
    let witness = found.evidence();
    assert!(!witness.paths.is_empty());
    let simulation = world.subject().simulation;
    let emission =
        fsl_package::emit_checked(&world.package, fsl_package::EmitLimits::fixture())
            .expect("the same checked owner emits");
    let limits = fsl_package::V2ReadLimits::bounded(1 << 24);
    let admitted = fsl_package::admit_checked_package(
        emission.package().bytes(),
        simulation.packages,
        limits,
    )
    .expect("the real bytes admit");
    let mut budget = limits.native_join_limits().budget();
    let identities = emission
        .identity_rows(&admitted, &mut budget)
        .expect("the actual identity rows join");
    let equality = emission
        .equality_witness(&admitted, &mut budget)
        .expect("the actual equality rows join");
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut budget,
        )
        .expect("the real supply joins");
    let owner = PackageModelInput::of(&bare, simulation.packages, simulation.normalization)
        .expect("the real owner reads all eventuality facts");
    let other_owner = PackageModelInput::of(&bare, simulation.packages, simulation.normalization)
        .expect("a distinct reader of the same bytes");
    let system = ModelSystem::new(fsl_eval::simulation::model::ModelSimulationRequest {
        input: owner.input(),
        ..simulation.clone()
    })
    .expect("the package system admits");
    let replay_limits = fsl_analyze::exact_probabilistic::ExactProbLimits::default();
    let replay = |owner, item, witness| {
        replay_probabilistic_witness_from_package(
            owner,
            &system,
            item,
            witness,
            &replay_limits,
            &Cancel::new(),
        )
    };
    let ProbabilityWitnessReplay::Reproduced(reproduced) =
        replay(&owner, &witness.item, witness).expect("the actual bare owner replays")
    else {
        panic!("the same negative evidence must reproduce");
    };
    assert_eq!(reproduced.mass(), found.mass());
    assert!(matches!(
        replay(&other_owner, &witness.item, witness),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut mixed = owner.input();
    mixed.model_selections = other_owner.input().model_selections;
    let mixed_system = ModelSystem::new(fsl_eval::simulation::model::ModelSimulationRequest {
        input: mixed,
        ..simulation.clone()
    })
    .expect("semantically equal model selections still admit");
    assert!(matches!(
        replay_probabilistic_witness_from_package(
            &owner,
            &mixed_system,
            &witness.item,
            witness,
            &replay_limits,
            &Cancel::new(),
        ),
        Err(ProbabilityWitnessError::Refused)
    ));
    assert!(matches!(
        replay_probabilistic_witness(
            &world.package,
            &system,
            &witness.item,
            witness,
            &replay_limits,
            &Cancel::new(),
        ),
        Err(ProbabilityWitnessError::Refused)
    ));
    let source_system = ModelSystem::new(simulation.clone()).expect("the source system admits");
    let foreign = crate::prob_world::World::new(&crate::prob_world::link(TOO_LIKELY));
    assert!(matches!(
        replay_probabilistic_witness(
            &foreign.package,
            &source_system,
            &witness.item,
            witness,
            &replay_limits,
            &Cancel::new(),
        ),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut wrong_ordinal = witness.clone();
    wrong_ordinal.item = fsl_package::ObligationIdentity::new(
        witness.item.node().clone(),
        witness.item.model().cloned(),
        witness.item.ordinal().checked_add(1).expect("fixture ordinal"),
    );
    assert!(matches!(
        replay(&owner, &wrong_ordinal.item, &wrong_ordinal),
        Err(ProbabilityWitnessError::Refused)
    ));
    let mut wrong_model = witness.clone();
    wrong_model.item = fsl_package::ObligationIdentity::new(
        witness.item.node().clone(),
        Some(fsl_package::ModelIdentity::new("foreign/model")),
        witness.item.ordinal(),
    );
    assert!(matches!(
        replay(&owner, &wrong_model.item, &wrong_model),
        Err(ProbabilityWitnessError::Refused)
    ));

    let source = ModelCheckInput::source(&world.package);
    for tamper in 0..10 {
        let mut mixed = owner.input();
        match tamper {
            0 => mixed.probabilistic = source.probabilistic,
            1 => mixed.types = source.types,
            2 => mixed.operations = source.operations,
            3 => mixed.model_selections = source.model_selections,
            4 => mixed.read_rewards = None,
            5 => mixed.evaluator = source.evaluator,
            6 => mixed.admitted = None,
            7 => mixed.source = Some(&world.package),
            8 => mixed.enum_members = None,
            9 => mixed.terminals = Ok(Vec::new()),
            _ => unreachable!("the controls are exhaustive"),
        }
        assert!(
            mixed.package_owner().is_none(),
            "mixed execution input {tamper} must have no authenticated owner"
        );
    }
    assert!(std::ptr::eq(
        owner.input().package_owner().expect("intact owner"),
        &owner
    ));
}
