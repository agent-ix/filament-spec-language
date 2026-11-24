// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: universal infinite-trace analysis from package facts alone.

use fsl_package::{AnalysisView, BarePackage, BareSupply, ReadEqualityRows, ReadIdentityRows};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, counter_request, label, package_selection, request_members, Workspace, HANDOFF,
};

fn universal_parity(bound: u64, expected: &str) {
    let ws = Workspace::new(&format!("package-universal-{expected}"));
    let unit = format!("hyper Equal using t0 over behaviours (c: M::Counter) forall trace a of M forall trace b of M {{ always holds(c.value @ a <= {bound} and c.value @ b <= {bound}) }}");
    let items = json!([{"claim": "Equal"}]);
    let source = analyze(&counter_request(&ws, &unit, items.clone()));
    assert_eq!(label(&source.item(0)), expected, "{}", source.document());
    let (members, bytes) = package_selection(&ws, &request_members(Some(items)));
    let model = std::fs::read(ws.path("model.json")).expect("domain fixture");
    let models =
        fsl_semantics::model::intake::package_input([model.as_slice()]).expect("domain admits");
    let limits = fsl_package::V2ReadLimits::bounded(1 << 24);
    let admitted =
        fsl_package::admit_checked_package(&bytes, &models, limits).expect("package admits");
    let mut budget = limits.native_join_limits().budget();
    let identities = ReadIdentityRows::read(
        &std::fs::read(ws.path(&format!("{HANDOFF}/identities.json"))).expect("identity rows"),
        &admitted,
        &mut budget,
    )
    .expect("identities admit");
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
        .expect("supply joins");
    let selected = bare.claim_name("Equal").expect("named hyper claim");
    let occurrence = fsl_foundation::source::provenance::OccurrenceKey::new(
        bare.node_key(&selected.declaration)
            .expect("structural claim key"),
        filament_exact::Origin::new(
            filament_exact::Role::new("claim"),
            selected.occurrence.ordinal,
        ),
    );
    let form = bare
        .hyper_clause_form(&occurrence)
        .expect("existing universal package form is readable");
    let rebuilt = fsl_semantics::check::CheckedHyperClause::of_package_form(
        form,
        bare.world().expect("package world").types(),
        1_000_000,
    )
    .expect("existing source-free constructor accepts Equal");
    assert_eq!(
        rebuilt.form(),
        Some(&fsl_semantics::check::HyperForm::Universal)
    );
    check_universal_atoms_and_certificate(&ws, &bare, &models, &rebuilt, expected == "proved");
    ws.write("program.fsl", b"not a native unit");
    std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
        .expect("remove retained sources");
    let package = analyze(
        &ws.write(
            "package-analyze.json",
            json!({"format": "native-analyze/1", "request": members})
                .to_string()
                .as_bytes(),
        ),
    );
    let document: Value = package.document();
    assert_eq!(package.exit, source.exit, "{document}");
    assert_eq!(package.item(0)["record"]["result"], expected, "{document}");
    for member in ["record", "certificate", "counterexample", "replay"] {
        assert_eq!(
            package.item(0)[member],
            source.item(0)[member],
            "{member}: {document}"
        );
    }
    if expected == "proved" {
        assert_eq!(
            package.item(0)["certificate"]["accepted"],
            true,
            "{document}"
        );
    } else {
        assert_eq!(package.item(0)["replay"]["reproduced"], true, "{document}");
    }
}

// Both evaluator ports read every pair of reachable Counter values. Equal
// verdicts alone would miss evaluator-family and slot-root charging defects.
fn check_universal_atoms_and_certificate<'a>(
    ws: &Workspace,
    bare: &'a BarePackage<'a>,
    packages: &'a fsl_semantics::model::intake::PackageInput,
    clause: &fsl_semantics::check::CheckedHyperClause,
    holds: bool,
) {
    use crate::counter::UNLIMITED;
    use filament_exact::{Cancel, Meter, ValueType};
    use fsl_analyze::model_check::{check_hyper, HyperCheckRequest};
    use fsl_eval::hyper::{CheckedHyperLetters, CheckedStepMatch, HyperLetters, ParameterBinding};
    use fsl_eval::model_subject::ModelSubject;
    use fsl_eval::simulation::model::{
        ExpansionLimits, ModelCheckInput, ModelSimulationRequest, ModelSystem, PackageModelInput,
    };
    use fsl_eval::simulation::{SubjectSystem, TransitionSystem};
    use fsl_replay::{
        check_package_hyper_closure, HyperOutcome, ModelCheckLimits, PackageHyperClosureRequest,
        ProductClosure, ProductClosureCheck,
    };
    use fsl_semantics::model::accounting::ModelNormalizationLimits;
    use fsl_semantics::model::observation::ObservationLimits;

    let compiled = fsl_command::checked_handoff::compile(&ws.path(HANDOFF))
        .expect("source oracle compiles before source deletion");
    let source = compiled.checked();
    let source_clause = &source.graph().hyper_clauses()[0];
    let snapshot = std::fs::read(ws.path("start.json")).expect("snapshot");
    let snapshots = [(
        fsl_semantics::model::observation::document_reference(&snapshot)
            .expect("snapshot identity"),
        snapshot,
    )]
    .into_iter()
    .collect();
    let selected = fsl_analyze::analyze::document::SubjectSelection::default_over(&snapshots);
    let binding = selected.binding(packages, &snapshots);
    let input = PackageModelInput::of(bare, packages, ModelNormalizationLimits::default())
        .expect("source-free input");
    let simulation = ModelSimulationRequest {
        input: input.input(),
        packages,
        normalization: ModelNormalizationLimits::default(),
        snapshots: &snapshots,
        initial: binding.initial,
        universes: binding.universes,
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: 1_000_000,
        },
        meter: UNLIMITED,
        emit: fsl_package::EmitLimits::fixture(),
    };
    let package_system = ModelSystem::new(simulation.clone()).expect("package system");
    let mut source_simulation = simulation.clone();
    source_simulation.input = ModelCheckInput::source(source);
    let source_system = ModelSystem::new(source_simulation).expect("source system");
    let package_letters =
        CheckedHyperLetters::for_evaluator(bare, clause, vec![&package_system, &package_system])
            .expect("package letters");
    let source_letters =
        CheckedHyperLetters::new(source, source_clause, vec![&source_system, &source_system])
            .expect("source letters");
    let parameter = &clause.parameters()[0];
    let instance = package_system
        .instances(&ValueType::Reference(parameter.population.object()))
        .expect("instances")
        .into_iter()
        .next()
        .expect("one Counter");
    let instance = [ParameterBinding {
        parameter: parameter.name.clone(),
        instance,
    }];
    let initial = package_system
        .initial()
        .into_iter()
        .next()
        .expect("initial state");
    let mut seen = std::collections::BTreeSet::from([package_system
        .canonical_state_key(&initial)
        .expect("initial key")]);
    let mut states = vec![initial];
    let mut cursor = 0;
    while let Some(state) = states.get(cursor).cloned() {
        for (_, next) in package_system
            .relation_successors(&state, &Cancel::new())
            .expect("reachable Counter successors")
            .successors
        {
            if seen.insert(
                package_system
                    .canonical_state_key(&next)
                    .expect("state key"),
            ) {
                assert!(states.len() < 4, "the bounded Counter has only four states");
                states.push(next);
            }
        }
        cursor += 1;
    }
    assert_eq!(states.len(), 4, "all four Counter values are exercised");
    let mut evaluated = 0;
    for left in &states {
        for right in &states {
            for (index, formula) in clause.body().nodes().iter().enumerate() {
                if !matches!(formula, fsl_semantics::check::FormulaNode::Holds { .. }) {
                    continue;
                }
                let atom = fsl_semantics::check::FormulaNodeId::from_arena_index(index);
                let mut source_meter = Meter::new(UNLIMITED);
                let mut package_meter = Meter::new(UNLIMITED);
                let expected = source_letters
                    .atom(atom, &instance, &[left, right], &mut source_meter)
                    .expect("source atom");
                let actual = package_letters
                    .atom(atom, &instance, &[left, right], &mut package_meter)
                    .expect("package atom");
                assert_eq!(actual, expected);
                assert!(!source_meter.charge_log_truncated());
                assert!(!package_meter.charge_log_truncated());
                assert!(!source_meter.admitted_charges().is_empty());
                assert_eq!(
                    package_meter.admitted_charges(),
                    source_meter.admitted_charges()
                );
                evaluated += 1;
            }
        }
    }
    assert!(evaluated >= 16);
    if !holds {
        return;
    }
    let subjects = vec![ModelSubject {
        simulation,
        proof_bounds: Vec::new(),
    }];
    let original = PackageHyperClosureRequest::new(
        bare,
        clause.assembled_occurrence().expect("occurrence"),
        &subjects,
        UNLIMITED,
        10_000,
    )
    .expect("checker independently rebuilds package clause");
    let matcher = CheckedStepMatch::for_evaluator(bare, clause);
    let outcome = check_hyper(
        HyperCheckRequest {
            clause,
            subjects: subjects
                .iter()
                .map(|subject| ModelSubject {
                    simulation: subject.simulation.clone(),
                    proof_bounds: subject.proof_bounds.clone(),
                })
                .collect(),
            letters: &package_letters,
            matcher: &matcher,
            limits: ModelCheckLimits::default(),
            max_depth: None,
            meter: UNLIMITED,
            reductions: fsl_route::reductions::Reductions::default(),
        },
        &Cancel::new(),
    )
    .expect("package product runs");
    let certificates = match outcome {
        HyperOutcome::Holds { certificates, .. }
        | HyperOutcome::HoldsReduced { certificates, .. } => certificates,
        outcome => panic!("holding universal product: {outcome:?}"),
    };
    assert!(!certificates.is_empty());
    for certificate in certificates {
        assert_eq!(
            check_package_hyper_closure(
                &original,
                &certificate,
                &ModelCheckLimits::default(),
                &Cancel::new()
            )
            .expect("independent check"),
            ProductClosureCheck::Accepted
        );
        let mut tampered = certificate.clone();
        let ProductClosure::Product(states) = &mut tampered.closure else {
            panic!("product closure");
        };
        assert!(!states.is_empty());
        states.remove(0);
        assert!(
            matches!(
                check_package_hyper_closure(
                    &original,
                    &tampered,
                    &ModelCheckLimits::default(),
                    &Cancel::new()
                ),
                Ok(ProductClosureCheck::Rejected(_))
            ),
            "removing a retained product state cannot prove the claim"
        );
    }
}

#[trace("FR-940-AC-17", "FR-940-AC-18", "FR-176-AC-1")]
#[test]
fn universal_package_proof_matches_source_without_source() {
    universal_parity(3, "proved");
}

#[trace("FR-940-AC-17", "FR-940-AC-18", "FR-176-AC-2")]
#[test]
fn universal_package_refutation_replays_without_source() {
    universal_parity(2, "refuted");
}

/// Prepare genuine inputs for the separately run production-driver smoke
/// check. The driver must create the handoff itself from these originals.
#[trace("FR-940-AC-24", "FR-631-AC-1")]
#[test]
fn prepare_hp1_production_cli_fixture() {
    let ws = Workspace::new("hp1-production-cli-inputs");
    let (unit, model, snapshots) = crate::hyper_world::request_files_over_starts(
        crate::hyper_world::Vault::Secure,
        false,
        &crate::hyper_world::STARTS,
        crate::relation_claims_support::DETERMINISTIC,
    );
    let path = crate::analyze_cli_support::request_over(
        &ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Det", "kind": "temporal-satisfaction", "backend": "en-1"}]}),
    );
    assert_eq!(
        snapshots.len(),
        2,
        "the CLI command block copies both initial snapshots"
    );
    let source = analyze(&path);
    assert_eq!(source.exit, 0, "{}", source.document());
    assert_eq!(source.item(0)["record"]["result"], "proved");
    assert_eq!(source.item(0)["record"]["certification"], "certified");
    assert_eq!(source.item(0)["certificate"]["accepted"], true);
    let source_request: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("source request"))
            .expect("request JSON");
    ws.write("analyze-source.json", source_request.to_string().as_bytes());
    let request = &source_request["request"];
    ws.write(
        "compile.json",
        json!({"format": "native-compile/1", "request": {
            "models": request["models"], "program": request["program"],
            "handoff": {"directory": "handoff", "package_name": "analyze-unit",
                "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}}
        }})
        .to_string()
        .as_bytes(),
    );
    let mut package = source_request;
    package["request"]
        .as_object_mut()
        .expect("request members")
        .remove("program");
    package["request"]["package"] = json!({"directory": "handoff"});
    ws.write("analyze-package.json", package.to_string().as_bytes());
    assert!(
        !ws.path(HANDOFF).exists(),
        "only the production CLI creates this acceptance handoff"
    );
    println!("HP1_FIXTURE={}", ws.path(".").display());
}
