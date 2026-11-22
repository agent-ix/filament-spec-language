// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: per-model subjects in universal package hyper analysis.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, label, package_selection, request_members, request_over, Workspace, HANDOFF,
};

fn two_model_parity(equal: bool) {
    let (expected, exit, operator) = if equal {
        ("proved", 0, "=")
    } else {
        ("refuted", 10, "!=")
    };
    let declaration = format!(
        "hyper Same using inf over behaviours (v: V::Vault, w: N::Vault) \
        forall trace a of V forall trace b of N \
        {{ always holds(v.h @ a {operator} w.h @ b) }}"
    );
    let (unit, model, other, snapshot) =
        crate::hyper_world::two_model_request_files((1, 0), &declaration);
    let ws = Workspace::new(&format!("package-hyper-two-model-{expected}"));
    ws.write("other.json", &other);
    let mut models = request_members(None)["models"].clone();
    models.as_array_mut().expect("models").push(json!({
        "format": "semantic-ir/2.0.0", "source": {"file": "other.json",
            "authority": "test", "identity": "vault-other-model", "document": "VaultOther",
            "formal_revision": 1}
    }));
    let source_path = request_over(
        &ws,
        &unit,
        &model,
        &[snapshot],
        json!({"models": models, "items": [{"claim": "Same"}]}),
    );
    let source = analyze(&source_path);
    assert_eq!(source.exit, exit, "{}", source.document());
    assert_eq!(label(&source.item(0)), expected, "{}", source.document());
    let request: Value =
        serde_json::from_slice(&std::fs::read(&source_path).expect("request bytes"))
            .expect("request JSON");
    let (members, bytes) = package_selection(&ws, &request["request"]);
    check_two_model_atoms_and_certificate(&ws, &bytes, equal);
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
    assert_eq!(package.exit, exit, "{}", package.document());
    assert!(
        source.item(0)["exploration"].is_object(),
        "source exploration is reported"
    );
    for member in [
        "record",
        "certificate",
        "counterexample",
        "replay",
        "exploration",
    ] {
        assert_eq!(
            package.item(0)[member],
            source.item(0)[member],
            "{member}: {}",
            package.document()
        );
    }
    if equal {
        assert_eq!(package.item(0)["certificate"]["accepted"], true);
    } else {
        assert_eq!(package.item(0)["replay"]["reproduced"], true);
    }
}

fn check_two_model_atoms_and_certificate(ws: &Workspace, bytes: &[u8], holds: bool) {
    use crate::counter::UNLIMITED;
    use filament_exact::{Cancel, Meter, ValueType};
    use fsl_analyze::model_check::{check_hyper, HyperCheckRequest};
    use fsl_eval::hyper::{CheckedHyperLetters, CheckedStepMatch, HyperLetters, ParameterBinding};
    use fsl_eval::model_subject::ModelSubject;
    use fsl_eval::simulation::model::{
        ExpansionLimits, ModelCheckInput, ModelSimState, ModelSimulationRequest, ModelSystem,
        PackageModelInput,
    };
    use fsl_eval::simulation::{SubjectSystem, TransitionSystem};
    use fsl_foundation::source::provenance::OccurrenceKey;
    use fsl_package::{AnalysisView, BarePackage, BareSupply, ReadEqualityRows, ReadIdentityRows};
    use fsl_replay::{
        check_package_hyper_closure, HyperOutcome, ModelCheckLimits, PackageHyperClosureRequest,
        ProductClosure, ProductClosureCheck,
    };
    use fsl_semantics::model::accounting::ModelNormalizationLimits;
    use fsl_semantics::model::observation::ObservationLimits;

    let compiled = fsl_command::checked_handoff::compile(&ws.path(HANDOFF))
        .expect("source oracle before deletion");
    let source = compiled.checked();
    let source_clause = &source.graph().hyper_clauses()[0];
    let models = [
        std::fs::read(ws.path("model.json")).expect("model"),
        std::fs::read(ws.path("other.json")).expect("other model"),
    ];
    let packages = fsl_semantics::model::intake::package_input(models.iter().map(Vec::as_slice))
        .expect("models admit");
    let snapshot = std::fs::read(ws.path("snapshot-0.json")).expect("initial snapshot");
    let snapshots = [(
        fsl_semantics::model::observation::document_reference(&snapshot)
            .expect("snapshot identity"),
        snapshot,
    )]
    .into_iter()
    .collect();
    let limits = fsl_package::V2ReadLimits::bounded(1 << 24);
    let admitted = fsl_package::admit_checked_package(bytes, &packages, limits).expect("package");
    let mut budget = limits.native_join_limits().budget();
    let identities = ReadIdentityRows::read(
        &std::fs::read(ws.path(&format!("{HANDOFF}/identities.json"))).expect("identity rows"),
        &admitted,
        &mut budget,
    )
    .expect("identity join");
    let equality = std::fs::read(ws.path(&format!("{HANDOFF}/equality.json")))
        .ok()
        .map(|bytes| ReadEqualityRows::read(&bytes, &admitted).expect("equality join"));
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: equality.as_ref(),
            },
            &mut budget,
        )
        .expect("supply joins");
    let row = bare.claim_name("Same").expect("claim name");
    let occurrence = OccurrenceKey::new(
        bare.node_key(&row.declaration).expect("claim key"),
        filament_exact::Origin::new(filament_exact::Role::new("claim"), row.occurrence.ordinal),
    );
    let form = bare.hyper_clause_form(&occurrence).expect("two-model form");
    let clause = fsl_semantics::check::CheckedHyperClause::of_package_form(
        form,
        bare.world().expect("package world").types(),
        10_000,
    )
    .expect("checked package form");
    let selection = fsl_analyze::analyze::document::SubjectSelection::default_over(&snapshots);
    let binding = selection.binding(&packages, &snapshots);
    let input = PackageModelInput::of(&bare, &packages, ModelNormalizationLimits::default())
        .expect("package input");
    let simulation = ModelSimulationRequest {
        input: input.input(),
        packages: &packages,
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
    let mut source_simulation = simulation.clone();
    source_simulation.input = ModelCheckInput::source(source);
    let package_systems: Vec<_> = clause
        .aliases()
        .iter()
        .map(|alias| {
            ModelSystem::for_hyper_alias(simulation.clone(), &clause, alias.as_str())
                .expect("package scoped model")
        })
        .collect();
    let source_systems: Vec<_> = source_clause
        .aliases()
        .iter()
        .map(|alias| {
            ModelSystem::for_hyper_alias(source_simulation.clone(), source_clause, alias.as_str())
                .expect("source scoped model")
        })
        .collect();
    assert_eq!(package_systems.len(), 2);
    assert_eq!(package_systems[0].model(), Some("test/vault"));
    assert_eq!(package_systems[1].model(), Some("test/vault-other"));
    for (actual, expected) in package_systems.iter().zip(&source_systems) {
        assert_eq!(
            actual.model(),
            expected.model(),
            "every alias has its own model"
        );
    }
    let mut instance = Vec::new();
    for parameter in clause.parameters() {
        let slot = clause
            .aliases()
            .iter()
            .position(|alias| alias == &parameter.alias)
            .expect("parameter alias");
        let binding = package_systems[slot]
            .instances(&ValueType::Reference(parameter.population.object()))
            .expect("parameter universe")
            .into_iter()
            .next()
            .expect("one Vault");
        instance.push(ParameterBinding {
            parameter: parameter.name.clone(),
            instance: binding,
        });
    }
    let letters =
        CheckedHyperLetters::for_evaluator(&bare, &clause, package_systems.iter().collect())
            .expect("package letters");
    let source_letters =
        CheckedHyperLetters::new(source, source_clause, source_systems.iter().collect())
            .expect("source letters");
    let mut reachable: Vec<Vec<ModelSimState>> = Vec::new();
    for system in &package_systems {
        let initial = system.initial().into_iter().next().expect("initial Vault");
        let mut seen = std::collections::BTreeSet::from([system
            .canonical_state_key(&initial)
            .expect("initial key")]);
        let mut states = vec![initial];
        let mut cursor = 0;
        while let Some(state) = states.get(cursor).cloned() {
            for (_, next) in system
                .relation_successors(&state, &Cancel::new())
                .expect("reachable successors")
                .successors
            {
                if seen.insert(system.canonical_state_key(&next).expect("state key")) {
                    assert!(
                        states.len() < 2,
                        "the fixture has two reachable public values"
                    );
                    states.push(next);
                }
            }
            cursor += 1;
        }
        assert_eq!(states.len(), 2);
        reachable.push(states);
    }
    let mut readings = 0;
    for left in &reachable[0] {
        for right in &reachable[1] {
            for (index, formula) in clause.body().nodes().iter().enumerate() {
                if !matches!(formula, fsl_semantics::check::FormulaNode::Holds { .. }) {
                    continue;
                }
                let atom = fsl_semantics::check::FormulaNodeId::from_arena_index(index);
                let mut source_meter = Meter::new(UNLIMITED);
                let mut package_meter = Meter::new(UNLIMITED);
                assert_eq!(
                    letters
                        .atom(atom, &instance, &[left, right], &mut package_meter)
                        .expect("package atom"),
                    source_letters
                        .atom(atom, &instance, &[left, right], &mut source_meter)
                        .expect("source atom")
                );
                assert!(!source_meter.admitted_charges().is_empty());
                assert!(!source_meter.charge_log_truncated());
                assert!(!package_meter.charge_log_truncated());
                assert_eq!(
                    package_meter.admitted_charges(),
                    source_meter.admitted_charges()
                );
                readings += 1;
            }
        }
    }
    assert!(readings >= 4);
    if !holds {
        return;
    }
    let subjects: Vec<_> = clause
        .aliases()
        .iter()
        .map(|_| ModelSubject {
            simulation: simulation.clone(),
            proof_bounds: Vec::new(),
        })
        .collect();
    let originals =
        PackageHyperClosureRequest::new(&bare, &occurrence, &subjects, UNLIMITED, 10_000)
            .expect("independently rebuilt model roots");
    let matcher = CheckedStepMatch::for_evaluator(&bare, &clause);
    let outcome = check_hyper(
        HyperCheckRequest {
            clause: &clause,
            subjects: subjects
                .iter()
                .map(|subject| ModelSubject {
                    simulation: subject.simulation.clone(),
                    proof_bounds: subject.proof_bounds.clone(),
                })
                .collect(),
            letters: &letters,
            matcher: &matcher,
            limits: ModelCheckLimits::default(),
            max_depth: None,
            meter: UNLIMITED,
            reductions: fsl_route::reductions::Reductions::default(),
        },
        &Cancel::new(),
    )
    .expect("two-model product");
    let certificates = match outcome {
        HyperOutcome::Holds { certificates, .. }
        | HyperOutcome::HoldsReduced { certificates, .. } => certificates,
        outcome => panic!("holding two-model product: {outcome:?}"),
    };
    assert!(!certificates.is_empty());
    for certificate in certificates {
        assert_eq!(
            check_package_hyper_closure(
                &originals,
                &certificate,
                &ModelCheckLimits::default(),
                &Cancel::new()
            )
            .expect("independent certificate check"),
            ProductClosureCheck::Accepted
        );
        let mut tampered = certificate;
        let ProductClosure::Product(states) = &mut tampered.closure else {
            panic!("product closure");
        };
        assert!(!states.is_empty());
        states.remove(0);
        assert!(matches!(
            check_package_hyper_closure(
                &originals,
                &tampered,
                &ModelCheckLimits::default(),
                &Cancel::new()
            ),
            Ok(ProductClosureCheck::Rejected(_))
        ));
    }
}

#[trace("FR-940-AC-17", "FR-940-AC-18", "FR-176-AC-1", "FR-163-AC-1")]
#[test]
fn two_model_package_hyper_proof_matches_source_without_source() {
    two_model_parity(true);
}

#[trace("FR-940-AC-17", "FR-940-AC-18", "FR-176-AC-2", "FR-183-AC-1")]
#[test]
fn two_model_package_hyper_refutation_replays_without_source() {
    two_model_parity(false);
}

/// A population belonging to neither selected model cannot disappear while
/// projecting a hyper subject. The universe stays clean so snapshot admission
/// must inspect the foreign population in the original initial bytes.
#[trace("FR-940-AC-17", "FR-179-AC-1", "FR-106-AC-3")]
#[test]
fn two_model_hyper_foreign_population_is_refused() {
    for foreign in ["ix://outside/vaults", "ix://test/vault/not-declared"] {
        let declaration = "hyper Same using inf over behaviours (v: V::Vault, w: N::Vault) \
            forall trace a of V forall trace b of N { always holds(v.h @ a = w.h @ b) }";
        let (unit, model, other, snapshot) =
            crate::hyper_world::two_model_request_files((1, 0), declaration);
        let ws = Workspace::new("package-hyper-two-model-foreign");
        ws.write("other.json", &other);
        let mut models = request_members(None)["models"].clone();
        models.as_array_mut().expect("models").push(json!({
            "format": "semantic-ir/2.0.0", "source": {"file": "other.json",
                "authority": "test", "identity": "vault-other-model", "document": "VaultOther",
                "formal_revision": 1}
        }));
        let path = request_over(
            &ws,
            &unit,
            &model,
            &[snapshot.clone()],
            json!({"models": models, "items": [{"claim": "Same"}]}),
        );
        let mut request: Value =
            serde_json::from_slice(&std::fs::read(path).expect("request bytes"))
                .expect("request JSON");
        let mut initial: Value = serde_json::from_slice(&snapshot).expect("snapshot JSON");
        let mut universe = initial.clone();
        universe["identity"]["identity"] = json!("scope-universe");
        ws.write("scope-universe.json", universe.to_string().as_bytes());
        let mut extra = initial["populations"][0].clone();
        extra["population"] = json!(foreign);
        initial["populations"]
            .as_array_mut()
            .expect("populations")
            .push(extra);
        ws.write("snapshot-0.json", initial.to_string().as_bytes());
        request["request"]["documents"]
            .as_array_mut()
            .expect("documents")
            .push(json!({"file": "scope-universe.json"}));
        request["request"]["subject"] = json!({
            "initial_states": [{"authority": "test", "identity": "s0"}],
            "universes": [{"authority": "test", "identity": "scope-universe"}],
            "proof_bounds": []
        });
        let (members, _) = package_selection(&ws, &request["request"]);
        let source = analyze(&ws.write("foreign-source.json", request.to_string().as_bytes()));
        ws.write("program.fsl", b"not a native unit");
        std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
            .expect("remove retained sources");
        let package = analyze(
            &ws.write(
                "foreign-package.json",
                json!({"format": "native-analyze/1", "request": members})
                    .to_string()
                    .as_bytes(),
            ),
        );
        for run in [&source, &package] {
            let document = run.document();
            assert_eq!(document["category"], "refusal", "{document}");
            let diagnostics: Vec<_> = document["diagnostics"]
                .as_array()
                .expect("diagnostics")
                .iter()
                .chain(
                    document["items"]
                        .as_array()
                        .expect("items")
                        .iter()
                        .filter_map(|item| item["record"].get("diagnostic")),
                )
                .collect();
            assert_eq!(diagnostics.len(), 1, "{document}");
            assert_eq!(
                diagnostics[0]["code"], "invalid_runtime_input",
                "{document}"
            );
            assert_eq!(diagnostics[0]["cause"], "wrong-role-mapping", "{document}");
            let detail = diagnostics[0]
                .get("detail")
                .or_else(|| diagnostics[0].get("details"))
                .expect("native refusal details");
            assert_eq!(detail["population"], foreign, "{document}");
        }
    }
}
