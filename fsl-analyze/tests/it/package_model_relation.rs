// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: HP-1 analysis and independent checking from package bytes alone.

use filament_exact::Cancel;
use fsl_analyze::model_check::{
    ModelRelationInput, ModelRelationOutcome, check_model_relation_input,
};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelRelationDescriptor, ModelSimulationRequest, PackageModelInput,
    PopulationUniverse,
};
use fsl_package::{AnalysisView, BarePackage, BareSupply, ReadEqualityRows, ReadIdentityRows};
use fsl_replay::{
    ModelCheckLimits, ModelRelationClosureRequest, ProductClosure, ProductClosureCheck,
    ProductClosureRefusal, check_model_relation_closure,
};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::observation::ObservationLimits;
use ix_trace_rs::trace;
use serde_json::{Value, json};

use crate::analyze_cli_support::{
    HANDOFF, Workspace, analyze, label, package_selection, request_members, request_over,
};
use crate::counter::UNLIMITED;
use crate::hyper_world::{self, STARTS, Vault, request_files_over_starts};
use crate::relation_claims::{DETERMINISTIC, initial_world, step_with_result_document};

fn analyze_package(ws: &Workspace, name: &str, members: &Value) -> crate::analyze_cli_support::Run {
    analyze(
        &ws.write(
            name,
            json!({"format": "native-analyze/1", "request": members})
                .to_string()
                .as_bytes(),
        ),
    )
}

fn forget_source(ws: &Workspace) {
    ws.write("program.fsl", b"not a native unit");
    std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
        .expect("delete retained sources");
}

/// Compare the two real evaluator ports on labelled reachable edges, retaining
/// every admitted charge rather than only comparing the terminal verdict.
pub(super) fn assert_tuple_evaluator_parity(
    ws: &Workspace,
    bytes: &[u8],
    swap_observations: bool,
    expect_no_charges: bool,
) -> [bool; 3] {
    use filament_exact::Meter;
    use fsl_eval::simulation::TransitionSystem;
    use fsl_eval::simulation::model::{ModelEvaluator, ModelRelationTuple, ModelSystem};
    use fsl_eval::value::ModelRelationEvaluation;

    let compiled = fsl_command::checked_handoff::compile(&ws.path(HANDOFF))
        .expect("source handoff compiles for evaluator comparison");
    let request: Value =
        serde_json::from_slice(&std::fs::read(ws.path("analyze.json")).expect("fixture request"))
            .expect("request JSON");
    let model_documents: Vec<_> = request["request"]["models"]
        .as_array()
        .expect("models")
        .iter()
        .map(|model| {
            std::fs::read(ws.path(model["source"]["file"].as_str().expect("model file")))
                .expect("model fixture")
        })
        .collect();
    let packages =
        fsl_semantics::model::intake::package_input(model_documents.iter().map(Vec::as_slice))
            .expect("model fixtures admit");
    let snapshots: fsl_semantics::model::observation::DocumentProvision =
        request["request"]["documents"]
            .as_array()
            .expect("documents")
            .iter()
            .map(|document| {
                let bytes =
                    std::fs::read(ws.path(document["file"].as_str().expect("snapshot file")))
                        .expect("snapshot fixture");
                (
                    fsl_semantics::model::observation::document_reference(&bytes)
                        .expect("snapshot identity"),
                    bytes,
                )
            })
            .collect();
    let limits = fsl_package::V2ReadLimits::bounded(1 << 24);
    let admitted = fsl_package::admit_checked_package(bytes, &packages, limits)
        .expect("the evaluator package admits");
    let mut budget = limits.native_join_limits().budget();
    let identities = ReadIdentityRows::read(
        &std::fs::read(ws.path(&format!("{HANDOFF}/identities.json"))).expect("identities"),
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
    let source = compiled.checked();
    let relation = &source.graph().step_relations()[0];
    let descriptor =
        ModelRelationDescriptor::of_source(source, relation).expect("source descriptor");
    let form = bare
        .model_relation_form(&bare.claim_name(relation.name()).expect("claim"))
        .expect("package relation form");
    let package_descriptor =
        ModelRelationDescriptor::of_package(&bare, &form).expect("package descriptor");
    let selection = fsl_analyze::analyze::document::SubjectSelection::default_over(&snapshots);
    let binding = selection.binding(&packages, &snapshots);
    let simulation = ModelSimulationRequest {
        input: fsl_eval::simulation::model::ModelCheckInput::source(source),
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
    let systems: Vec<_> = descriptor
        .execution_aliases()
        .iter()
        .map(|alias| ModelSystem::for_alias(simulation.clone(), alias).expect("execution system"))
        .collect();
    assert_eq!(systems.len(), 2, "the fixture has two labelled executions");
    let edges: Vec<Vec<_>> = systems
        .iter()
        .zip(descriptor.executions())
        .map(|(system, (_, operation))| {
            let operation = fsl_semantics::check::operation_text(operation);
            let operation = operation.as_str();
            system
                .initial()
                .into_iter()
                .flat_map(|pre| {
                    system
                        .relation_successors(&pre, &Cancel::new())
                        .expect("reachable edges")
                        .successors
                        .into_iter()
                        // Each slot binds only its declared operation, as EN-1 does.
                        .filter(move |(edge, _)| edge.transition.operation() == operation)
                        .map(move |(edge, post)| (pre.clone(), edge, post))
                })
                .collect()
        })
        .collect();
    assert!(edges.iter().all(|edges| !edges.is_empty()));
    let mut observed = [false; 3];
    let mut swapped_differs = false;
    for left in &edges[0] {
        for right in &edges[1] {
            let chosen = [left, right];
            let pre: Vec<_> = chosen
                .iter()
                .zip(&systems)
                .map(|(edge, system)| system.observe(edge.0.state()).expect("pre observation"))
                .collect();
            let post: Vec<_> = chosen
                .iter()
                .zip(&systems)
                .map(|(edge, system)| system.observe(edge.2.state()).expect("post observation"))
                .collect();
            let transitions: Vec<_> = chosen.iter().map(|edge| &edge.1.transition).collect();
            let results: Vec<_> = chosen.iter().map(|edge| edge.1.result.as_deref()).collect();
            let before: Vec<_> = pre.iter().map(|value| value.environment()).collect();
            let after: Vec<_> = post.iter().map(|value| value.environment()).collect();
            let mut source_meter = Meter::new(UNLIMITED);
            let expected = source
                .evaluate_model_relation(
                    &descriptor,
                    ModelRelationTuple {
                        transitions: &transitions,
                        results: &results,
                        pre: &before,
                        post: &after,
                    },
                    &mut source_meter,
                )
                .unwrap_or_else(|failure| {
                    panic!(
                        "source tuple evaluates for {}: executions {:?}, transitions {:?}: {failure:?}",
                        ws.path("program.fsl").display(),
                        descriptor.executions(),
                        transitions.iter().map(|transition| transition.operation()).collect::<Vec<_>>()
                    )
                });
            let mut package_meter = Meter::new(UNLIMITED);
            let actual = bare
                .evaluate_model_relation(
                    &package_descriptor,
                    ModelRelationTuple {
                        transitions: &transitions,
                        results: &results,
                        pre: &before,
                        post: &after,
                    },
                    &mut package_meter,
                )
                .expect("package tuple evaluates");
            assert_eq!(actual, expected);
            assert!(!source_meter.charge_log_truncated());
            assert!(!package_meter.charge_log_truncated());
            assert_eq!(
                source_meter.admitted_charges().is_empty(),
                expect_no_charges
            );
            assert_eq!(
                package_meter.admitted_charges(),
                source_meter.admitted_charges()
            );
            match expected {
                ModelRelationEvaluation::Holds => observed[0] = true,
                ModelRelationEvaluation::Refuted => observed[1] = true,
                ModelRelationEvaluation::Undefined(_) => observed[2] = true,
                ModelRelationEvaluation::Undetermined(_) => panic!("unlimited tuple undecided"),
            }
            if swap_observations {
                let mut swapped_pre = before.clone();
                let mut swapped_post = after.clone();
                // Swap one execution's pre/post meanings, leaving its labelled
                // transition and the other execution unchanged.
                swapped_pre[0] = after[0];
                swapped_post[0] = before[0];
                swapped_pre[1] = before[1];
                swapped_post[1] = after[1];
                let swapped = bare
                    .evaluate_model_relation(
                        &package_descriptor,
                        ModelRelationTuple {
                            transitions: &transitions,
                            results: &results,
                            pre: &swapped_pre,
                            post: &swapped_post,
                        },
                        &mut Meter::new(UNLIMITED),
                    )
                    .expect("swapped observation projection evaluates");
                swapped_differs |= swapped != actual;
            }
        }
    }
    if swap_observations {
        assert!(
            swapped_differs,
            "a real changing field detects swapped pre/post"
        );
    }
    observed
}

/// Includes a real post-state violation, runtime undefined body and counted stops.
#[trace("FR-940-AC-24", "FR-940-AC-25")]
#[test]
fn hp1_package_analyze_matches_source_after_source_deletion() {
    const NONDETERMINISTIC: &str =
        "post Step using v on V::Vault::step { self.l = i or self.l = 1 - i }\n";
    for (case, vault, relation, temporal, expected) in [
        ("holds", Vault::Secure, DETERMINISTIC, json!({}), "proved"),
        (
            "results-holds",
            Vault::Custom(
                "post Step using v on V::Vault::step { (result = 0 or result = 1) and self.l = 0 }\n",
            ),
            "relation Det using v over (x: V::Vault::step, y: V::Vault::step) { true }",
            json!({}),
            "proved",
        ),
        (
            "results-refuted",
            Vault::Custom(
                "post Step using v on V::Vault::step { (result = 0 or result = 1) and self.l = 0 }\n",
            ),
            "relation Det using v over (x: V::Vault::step, y: V::Vault::step) { x.result = y.result }",
            json!({}),
            "refuted",
        ),
        (
            "refuted",
            Vault::Custom(NONDETERMINISTIC),
            DETERMINISTIC,
            json!({}),
            "refuted",
        ),
        (
            "undefined",
            Vault::Secure,
            "relation Det using v over (x: V::Vault::step, y: V::Vault::step) { 1 mod x.i >= y.i }",
            json!({}),
            "refuted",
        ),
        (
            "tuple-stop",
            Vault::Secure,
            DETERMINISTIC,
            json!({"limits": {"max_relation_tuples": 63}}),
            "incomplete",
        ),
        (
            "tuple-paid",
            Vault::Secure,
            DETERMINISTIC,
            json!({"limits": {"max_relation_tuples": 64}}),
            "proved",
        ),
        (
            "depth",
            Vault::Secure,
            DETERMINISTIC,
            json!({"max_depth": 1}),
            "inconclusive",
        ),
    ] {
        let ws = Workspace::new(&format!("hp1-package-{case}"));
        let (unit, mut model, snapshots) =
            request_files_over_starts(vault, false, &STARTS, relation);
        if case.starts_with("results-") {
            model = step_with_result_document();
        }
        let path = request_over(
            &ws,
            &unit,
            &model,
            &snapshots,
            json!({"items": [{"claim": "Det", "temporal": temporal}]}),
        );
        let source = analyze(&path);
        assert!(!source.item(0).is_null(), "{case}: {}", source.document());
        assert_eq!(label(&source.item(0)), expected, "{}", source.document());
        let request: Value = serde_json::from_slice(&std::fs::read(path).expect("request file"))
            .expect("request JSON");
        let (members, bytes) = package_selection(&ws, &request["request"]);
        let tuple_outcomes =
            assert_tuple_evaluator_parity(&ws, &bytes, false, case == "results-holds");
        let expected_tuple = if case == "undefined" {
            2
        } else if expected == "refuted" {
            1
        } else {
            0
        };
        assert!(
            tuple_outcomes[expected_tuple],
            "{case}: expected actual tuple outcome"
        );
        forget_source(&ws);
        let package = analyze_package(&ws, "package.json", &members);
        assert_eq!(package.exit, source.exit, "{}", package.document());
        for member in ["record", "counterexample", "replay", "certificate"] {
            assert_eq!(
                package.item(0)[member],
                source.item(0)[member],
                "{case}: {member}: {}",
                package.document()
            );
        }
        if expected == "proved" {
            assert_eq!(package.item(0)["record"]["certification"], "certified");
        }
        if expected == "refuted" {
            assert_eq!(package.item(0)["replay"]["reproduced"], true);
        }
    }
}

/// Neither the engine nor its independent checker receives a checked source package.
#[trace("FR-940-AC-25", "FR-184-AC-4")]
#[test]
fn hp1_package_checker_accepts_and_refuses_tampering_without_source() {
    let ws = Workspace::new("hp1-package-independent");
    let (unit, model, snapshots) =
        request_files_over_starts(Vault::Secure, false, &STARTS, DETERMINISTIC);
    let path = request_over(
        &ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Det"}]}),
    );
    let request: Value =
        serde_json::from_slice(&std::fs::read(path).expect("request file")).expect("request JSON");
    let (_, bytes) = package_selection(&ws, &request["request"]);
    forget_source(&ws);
    let (packages, snapshots, initial) = initial_world(&model);
    let limits = fsl_package::V2ReadLimits::bounded(1 << 24);
    let admitted = fsl_package::admit_checked_package(&bytes, &packages, limits)
        .expect("the package admits without source");
    let mut budget = limits.native_join_limits().budget();
    let identities = ReadIdentityRows::read(
        &std::fs::read(ws.path(&format!("{HANDOFF}/identities.json"))).expect("identities"),
        &admitted,
        &mut budget,
    )
    .expect("identity witness admits");
    let equality = std::fs::read(ws.path(&format!("{HANDOFF}/equality.json")))
        .ok()
        .map(|bytes| ReadEqualityRows::read(&bytes, &admitted).expect("equality witness admits"));
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: equality.as_ref(),
            },
            &mut budget,
        )
        .expect("supply joins");
    let selected = bare.claim_name("Det").expect("the admitted claim name");
    let form = bare
        .model_relation_form(&selected)
        .expect("HP-1 form selects");
    let relation =
        ModelRelationDescriptor::of_package(&bare, &form).expect("execution descriptor projects");
    let input = PackageModelInput::of(&bare, &packages, ModelNormalizationLimits::default())
        .expect("package model input");
    let subject = ModelSubject {
        simulation: ModelSimulationRequest {
            input: input.input(),
            packages: &packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &snapshots,
            initial,
            universes: vec![PopulationUniverse {
                population: hyper_world::key("vaults"),
                keys: vec!["v".to_owned()],
            }],
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: UNLIMITED,
            emit: fsl_package::EmitLimits::fixture(),
        },
        proof_bounds: Vec::new(),
    };
    let subjects = vec![subject];
    let outcome = check_model_relation_input(
        ModelRelationInput {
            relation: &relation,
            evaluator: &bare,
            subjects: subjects
                .iter()
                .map(|subject| ModelSubject {
                    simulation: subject.simulation.clone(),
                    proof_bounds: subject.proof_bounds.clone(),
                })
                .collect(),
            limits: ModelCheckLimits::default(),
            max_depth: None,
            meter: UNLIMITED,
        },
        &Cancel::new(),
    )
    .expect("package EN-1 admits");
    let ModelRelationOutcome::Holds { certificate, stats } = outcome else {
        panic!("holding package relation: {outcome:?}");
    };
    assert_eq!(stats.relation_tuples, 64);
    let request = ModelRelationClosureRequest {
        relation: &relation,
        evaluator: &bare,
        subjects: &subjects,
        meter: UNLIMITED,
    };
    for allowance in [64, 63, 64] {
        let limits = ModelCheckLimits {
            max_relation_tuples: allowance,
            ..ModelCheckLimits::default()
        };
        let result = check_model_relation_closure(&request, &certificate, &limits, &Cancel::new())
            .expect("checker reconstructs from package inputs");
        if allowance == 64 {
            assert_eq!(result, ProductClosureCheck::Accepted);
        } else {
            let ProductClosureCheck::Stopped {
                tuple: Some(tuple), ..
            } = result
            else {
                panic!("first unpaid tuple: {result:?}");
            };
            assert_eq!(tuple.count, 64);
            let ProductClosure::Relation(closure) = &certificate.closure else {
                panic!("HP-1 closure");
            };
            assert_eq!(
                tuple.first_unpaid.transitions,
                [
                    closure[0].executions[0].transitions[7].clone(),
                    closure[0].executions[1].transitions[7].clone()
                ]
            );
        }
    }
    for defect in 0..3 {
        let mut tampered = certificate.clone();
        let ProductClosure::Relation(subjects) = &mut tampered.closure else {
            panic!("HP-1 closure");
        };
        match defect {
            0 => {
                subjects[0].states.remove(0);
            }
            1 => {
                subjects[0].executions[0].transitions.remove(0);
            }
            2 => {
                subjects[0].executions[0].transitions[0].result = Some("\"wrong-type\"".to_owned());
            }
            _ => unreachable!(),
        }
        let checked = check_model_relation_closure(
            &request,
            &tampered,
            &ModelCheckLimits::default(),
            &Cancel::new(),
        );
        if defect == 0 {
            assert!(matches!(checked, Ok(ProductClosureCheck::Rejected(_))));
        } else {
            assert_eq!(
                checked,
                Err(ProductClosureRefusal::Input("execution-edges"))
            );
        }
    }
}

fn ordered_handoff(
    ws: &Workspace,
    separate_models: bool,
) -> (Value, Vec<u8>, fsl_semantics::model::intake::PackageInput) {
    ordered_handoff_with_body(
        ws,
        separate_models,
        "pre(x.self.h) = pre(x.self.h) and pre(y.self.h) = pre(y.self.h) and x.self.l = x.i and y.self.l = y.i",
    )
}

fn ordered_handoff_with_body(
    ws: &Workspace,
    separate_models: bool,
    body: &str,
) -> (Value, Vec<u8>, fsl_semantics::model::intake::PackageInput) {
    let declarations = format!(
        "post OtherStep using v on N::Vault::step {{ self.l = i }}\nrelation Ordered using v over (y: N::Vault::step, x: V::Vault::step) {{ {body} }}\ninvariant Sibling using v on V::Vault at current {{ true }}"
    );
    let (unit, model, other, snapshots, extra) = if separate_models {
        let (unit, model, other, snapshot) =
            hyper_world::two_model_request_files((0, 0), &declarations);
        ws.write("other.json", &other);
        let mut models = request_members(None)["models"].clone();
        models.as_array_mut().expect("model selections").push(json!({"format": "semantic-ir/2.0.0", "source": {"file": "other.json", "authority": "test", "identity": "vault-other-model", "document": "VaultOther", "formal_revision": 1}}));
        (
            unit,
            model,
            Some(other),
            vec![snapshot],
            json!({"models": models}),
        )
    } else {
        let (unit, model, snapshots) =
            request_files_over_starts(Vault::Secure, false, &STARTS, &declarations);
        let unit = unit.replace(
            "function Noop",
            &format!("model N = \"{}\";\nfunction Noop", hyper_world::PACKAGE),
        );
        (unit, model, None, snapshots, json!({}))
    };
    let mut extra = extra;
    extra["items"] =
        json!([{"claim": "Ordered"}, {"claim": "Sibling", "kind": "operation-contract"}]);
    let path = request_over(ws, &unit, &model, &snapshots, extra);
    let request: Value =
        serde_json::from_slice(&std::fs::read(path).expect("request file")).expect("request JSON");
    let mut documents = vec![model.as_slice()];
    if let Some(other) = &other {
        documents.push(other);
    }
    let packages =
        fsl_semantics::model::intake::package_input(documents).expect("domain packages admit");
    let (members, bytes) = package_selection(ws, &request["request"]);
    (members, bytes, packages)
}

#[trace("FR-940-AC-23", "FR-940-AC-27")]
#[test]
fn hp1_package_forms_preserve_declaration_order_and_distinct_aliases() {
    assert_declaration_order_and_distinct_aliases(false);
}

#[trace("FR-940-AC-23", "FR-940-AC-27")]
#[test]
fn hp1_package_forms_preserve_declaration_order_across_separate_models() {
    assert_declaration_order_and_distinct_aliases(true);
}

#[trace("FR-940-AC-24", "FR-179-AC-2")]
#[test]
fn hp1_package_separate_models_counterexample_replays() {
    let ws = Workspace::new("hp1-package-model-scope-replay");
    let (members, bytes, _) = ordered_handoff_with_body(&ws, true, "x.i = y.i");
    let source = analyze(&ws.path("analyze.json"));
    assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
    assert_eq!(
        source.item(0)["replay"]["reproduced"],
        true,
        "{}",
        source.document()
    );
    let outcomes = assert_tuple_evaluator_parity(&ws, &bytes, false, false);
    assert!(
        outcomes[0] && outcomes[1],
        "distinct models retain both matching and conflicting result tuples"
    );
    forget_source(&ws);
    let package = analyze_package(&ws, "model-scope-replay.json", &members);
    for member in ["record", "counterexample", "replay"] {
        assert_eq!(
            package.item(0)[member],
            source.item(0)[member],
            "{member}: {}",
            package.document()
        );
    }
}

/// Scoping may exclude another selected model's populations, never an
/// undeclared population, even when it shares a selected model's prefix.
#[trace("FR-940-AC-23", "FR-940-AC-24", "FR-179-AC-1")]
#[test]
fn hp1_package_model_scope_keeps_unselected_population_refusals() {
    for foreign in ["ix://outside/vaults", "ix://test/vault/not-declared"] {
        let ws = Workspace::new("hp1-package-model-scope-refusal");
        let (mut members, _, _) = ordered_handoff(&ws, true);
        let mut request: Value = serde_json::from_slice(
            &std::fs::read(ws.path("analyze.json")).expect("source request"),
        )
        .expect("request JSON");
        let mut initial: Value =
            serde_json::from_slice(&std::fs::read(ws.path("snapshot-0.json")).expect("snapshot"))
                .expect("snapshot JSON");
        // Keep the universe clean so the foreign population is checked by
        // snapshot admission, rather than rejected as a universe mapping.
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
        let subject = json!({
            "initial_states": [{"authority": "test", "identity": "s0"}],
            "universes": [{"authority": "test", "identity": "scope-universe"}],
            "proof_bounds": []
        });
        request["request"]["documents"]
            .as_array_mut()
            .expect("documents")
            .push(json!({"file": "scope-universe.json"}));
        request["request"]["subject"] = subject.clone();
        request["request"]["items"] = json!([{"claim": "Ordered"}]);
        members["documents"] = request["request"]["documents"].clone();
        members["subject"] = subject;
        members["items"] = request["request"]["items"].clone();
        let source = analyze(&ws.write("foreign-source.json", request.to_string().as_bytes()));
        forget_source(&ws);
        let package = analyze_package(&ws, "foreign-package.json", &members);
        for run in [&source, &package] {
            let document = run.document();
            assert_eq!(document["category"], "refusal", "{document}");
            // Source preflight can refuse before any items are created;
            // an item-local refusal uses FR-621's singular `detail` member.
            // Require exactly the same native refusal at either boundary.
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
            let diagnostic = diagnostics[0];
            assert_eq!(
                diagnostic["code"],
                "invalid_runtime_input",
                "{}",
                run.document()
            );
            assert_eq!(
                diagnostic["cause"],
                "wrong-role-mapping",
                "{}",
                run.document()
            );
            assert_eq!(
                diagnostic
                    .get("detail")
                    .or_else(|| diagnostic.get("details"))
                    .expect("native diagnostic payload")["population"],
                foreign,
                "{}",
                run.document()
            );
        }
    }
}

/// Projection must preserve the native reader's refusal on original bytes,
/// including duplicate members and work hidden in another model's portion.
#[trace("FR-940-AC-23", "FR-179-AC-1", "FR-106-AC-3")]
#[test]
fn hp1_package_model_scope_preserves_original_document_refusals() {
    use fsl_eval::simulation::model::{ModelCheckInput, ModelSystem};
    use fsl_semantics::model::observation::{AdmissionFailure, DocumentProvision};
    use fsl_semantics::model::state::StateModel;

    let ws = Workspace::new("hp1-package-original-read-refusals");
    let (_, _, packages) = ordered_handoff(&ws, true);
    let compiled =
        fsl_command::checked_handoff::compile(&ws.path(HANDOFF)).expect("source compiles");
    let input = ModelCheckInput::source(compiled.checked());
    let whole = StateModel::new(
        input.model_inputs(),
        input.types,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("selected models normalize");
    let original = std::fs::read(ws.path("snapshot-0.json")).expect("original snapshot");
    let reference =
        fsl_semantics::model::observation::document_reference(&original).expect("identity");
    let clean = DocumentProvision::from([(reference.clone(), original.clone())]);
    let selection = fsl_analyze::analyze::document::SubjectSelection::default_over(&clean);
    let binding = selection.binding(&packages, &clean);
    let text = std::str::from_utf8(&original).expect("fixture UTF-8");
    let duplicate = text
        .replacen(
            "\"observation\":\"current\"",
            "\"observation\":\"pre\",\"observation\":\"current\"",
            1,
        )
        .into_bytes();
    assert_ne!(
        duplicate, original,
        "fixture really contains a duplicate member"
    );
    let mut padded = original.clone();
    padded.extend(std::iter::repeat_n(b' ', 4096));
    let mut cases = vec![
        (
            "duplicate",
            duplicate,
            ObservationLimits::default(),
            "invalid-value",
        ),
        (
            "bytes",
            padded,
            ObservationLimits {
                document_bytes: u64::try_from(original.len()).expect("fixture length") + 2048,
                ..ObservationLimits::default()
            },
            "input-bytes-exceeded",
        ),
        (
            "objects",
            original.clone(),
            ObservationLimits {
                objects_per_document: 1,
                ..ObservationLimits::default()
            },
            "node-count-exceeded",
        ),
    ];
    for foreign in ["ix://outside/vaults", "ix://test/vault/not-declared"] {
        let mut snapshot: Value = serde_json::from_slice(&original).expect("fixture snapshot");
        let mut population = snapshot["populations"][0].clone();
        population["population"] = json!(foreign);
        snapshot["populations"]
            .as_array_mut()
            .expect("populations")
            .push(population);
        cases.push((
            foreign,
            serde_json::to_vec(&snapshot).expect("foreign fixture"),
            ObservationLimits::default(),
            "wrong-role-mapping",
        ));
    }
    for (case, bytes, limits, cause) in cases {
        let provision = DocumentProvision::from([(reference.clone(), bytes)]);
        let expected = whole
            .admit_initial(&provision, &reference, limits)
            .expect_err("original document must refuse");
        let AdmissionFailure::Refused(record) = &expected else {
            panic!("{case}: expected refusal: {expected:?}");
        };
        assert_eq!(
            record.catalog.cause(),
            cause,
            "{case}: native oracle reaches the intended refusal"
        );
        assert_eq!(input.model_selections.len(), 2);
        for identity in input
            .model_selections
            .iter()
            .map(|model| model.identity.as_str())
        {
            let actual = ModelSystem::for_model_identity(
                ModelSimulationRequest {
                    input: input.clone(),
                    packages: &packages,
                    normalization: ModelNormalizationLimits::default(),
                    snapshots: &provision,
                    initial: vec![reference.clone()],
                    universes: binding.universes.clone(),
                    observation: limits,
                    expansion: ExpansionLimits {
                        max_candidates: 1_000_000,
                    },
                    meter: UNLIMITED,
                    emit: fsl_package::EmitLimits::fixture(),
                },
                identity,
            )
            .err()
            .expect("scoped admission must refuse");
            assert_eq!(
                actual, expected,
                "{case}, {identity}: preserve the full native refusal"
            );
        }
    }
}

fn assert_declaration_order_and_distinct_aliases(separate_models: bool) {
    let ws = Workspace::new(&format!("hp1-package-form-{separate_models}"));
    let (members, bytes, packages) = ordered_handoff(&ws, separate_models);
    let source_run = analyze(&ws.path("analyze.json"));
    assert_eq!(
        label(&source_run.item(0)),
        "proved",
        "{}",
        source_run.document()
    );
    let compiled = fsl_command::checked_handoff::compile(&ws.path(HANDOFF))
        .expect("the written source handoff compiles");
    let limits = fsl_package::V2ReadLimits::bounded(1 << 24);
    let source_read = fsl_package::read_evaluation_package(
        compiled.checked(),
        fsl_semantics::library::LibraryName::new(compiled.checked().graph().package_name())
            .expect("package name"),
        &packages,
        limits,
        fsl_package::EmitLimits::fixture(),
    )
    .expect("the source emission admits through the same reader");
    let admitted =
        fsl_package::admit_checked_package(&bytes, &packages, limits).expect("written bytes admit");
    let bare = BarePackage::new(&admitted);
    let source = source_read
        .model_relation_form(
            &source_read
                .claim_name("Ordered")
                .expect("source occurrence"),
        )
        .expect("source form");
    let package = bare
        .model_relation_form(&bare.claim_name("Ordered").expect("package occurrence"))
        .expect("package form");
    assert_eq!(source, package);
    use fsl_semantics::model::clause_observation::Observation;
    assert_eq!(
        package
            .reads()
            .iter()
            .map(|read| (read.execution, read.observation))
            .collect::<std::collections::BTreeSet<_>>(),
        [
            (0, Observation::Pre),
            (1, Observation::Pre),
            (0, Observation::Post),
            (1, Observation::Post)
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        package.span(),
        compiled.checked().graph().step_relations()[0].span()
    );
    assert_eq!(
        package
            .subjects()
            .iter()
            .map(|subject| subject.alias.as_str())
            .collect::<Vec<_>>(),
        ["V", "N"]
    );
    assert_eq!(
        package
            .body()
            .subjects
            .iter()
            .map(|(alias, _)| alias.as_ref())
            .collect::<Vec<_>>(),
        ["N", "V"]
    );
    assert_eq!(
        package
            .body()
            .executions
            .iter()
            .map(|execution| execution.variable.as_ref())
            .collect::<Vec<_>>(),
        ["y", "x"]
    );
    assert_eq!(
        package.subjects()[0].root == package.subjects()[1].root,
        !separate_models
    );
    assert_eq!(
        package.subjects()[0].identity == package.subjects()[1].identity,
        !separate_models
    );
    assert_tuple_evaluator_parity(&ws, &bytes, true, false);
    drop(source_read);
    drop(compiled);
    forget_source(&ws);
    let package_run = analyze_package(&ws, "ordered-package.json", &members);
    assert_eq!(
        package_run.exit,
        source_run.exit,
        "{}",
        package_run.document()
    );
    for member in ["record", "certificate", "counterexample", "replay"] {
        assert_eq!(
            package_run.item(0)[member],
            source_run.item(0)[member],
            "{member}: {}",
            package_run.document()
        );
    }
}

#[trace("FR-940-AC-26", "FR-940-AC-27")]
#[test]
fn hp1_package_bad_order_evidence_refuses_but_legacy_absence_is_item_local() {
    let ws = Workspace::new("hp1-package-order-admission");
    let (members, bytes, packages) = ordered_handoff(&ws, true);
    let original: Value = serde_json::from_slice(&bytes).expect("checked package JSON");
    let nodes = original["semantic_graph"]["nodes"]
        .as_array()
        .expect("graph nodes");
    let position = nodes
        .iter()
        .position(|node| node["semantic_form"] == "step_relation")
        .expect("relation node");
    let prefix = format!("/semantic_graph/nodes/{position}/body/members/4");
    for (defect, suffix) in [
        (0, "/value/members/1/name"),
        (1, "/value/members/0/name"),
        (2, "/value/members/1"),
        (3, "/value/members/0/value/target"),
        (4, ""),
        (5, "/value"),
        (6, "/value/members/0"),
    ] {
        let mut altered = original.clone();
        let member = &mut altered["semantic_graph"]["nodes"][position]["body"]["members"][4];
        match defect {
            0 => {
                member["value"]["members"][1]["name"] =
                    member["value"]["members"][0]["name"].clone();
            }
            1 => {
                member["value"]["members"][0]["name"] = json!("Unknown");
            }
            2 => {
                member["value"]["members"]
                    .as_array_mut()
                    .expect("order Group")
                    .pop();
            }
            3 => {
                member["value"]["members"][0]["value"] =
                    member["value"]["members"][1]["value"].clone();
            }
            4 => {
                member["name"] = json!("wrong-member");
            }
            5 => {
                member["value"] = member["value"]["members"][0]["value"].clone();
            }
            6 => {
                member["value"]["members"][0] = member["value"]["members"][0]["value"].clone();
            }
            _ => unreachable!(),
        }
        let refused = fsl_package::admit_checked_package(
            &serde_json::to_vec(&altered).expect("altered JSON"),
            &packages,
            fsl_package::V2ReadLimits::bounded(1 << 24),
        )
        .expect_err("present malformed evidence refuses globally");
        assert_eq!(refused.code, fsl_foundation::Code::InvalidPackage);
        assert_eq!(refused.cause, None);
        assert_eq!(
            refused.pointer.as_deref(),
            Some(format!("{prefix}{suffix}").as_str()),
            "defect {defect}"
        );
    }
    // The consumer must retain the strict reader's first refusal at the
    // whole-request boundary, including integrity checks that precede joins.
    let execution =
        &original["semantic_graph"]["nodes"][position]["body"]["members"][1]["value"]["members"][1];
    assert_eq!(execution["name"], "x");
    assert_eq!(
        execution["value"]["members"][2]["value"]["members"]
            .as_array()
            .expect("execution binders")
            .len(),
        2
    );
    // In this two-execution fixture x follows y, so its receiver and
    // argument have global levels 2 and 3. Select their unique literal
    // bodies: operand-bearing wire key spellings are not an ID oracle.
    let parameter_position = |name: &str, level: &str| {
        let positions: Vec<_> = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                node["node_tag"] == "value"
                    && node["semantic_form"] == "parameter"
                    && node["body"]["members"][0]["value"]["value"] == name
                    && node["body"]["members"][1]["value"]["value"] == level
            })
            .map(|(position, _)| position)
            .collect();
        let [position] = positions.as_slice() else {
            panic!("exactly one {name} parameter at level {level}: {positions:?}");
        };
        *position
    };
    let receiver = parameter_position("self", "2");
    let argument = parameter_position("i", "3");
    forget_source(&ws);
    for defect in 0..4 {
        let mut altered = original.clone();
        let graph = altered["semantic_graph"]["nodes"]
            .as_array_mut()
            .expect("nodes");
        match defect {
            0 => {
                let receiver_level = graph[receiver]["body"]["members"][1]["value"].clone();
                assert_ne!(
                    graph[argument]["body"]["members"][1]["value"],
                    receiver_level
                );
                graph[argument]["body"]["members"][1]["value"] = receiver_level;
            }
            1 => {
                graph[argument]["body"]["members"]
                    .as_array_mut()
                    .expect("parameter body")
                    .pop();
            }
            2 => {
                assert_ne!(
                    graph[argument]["semantic_type"],
                    graph[receiver]["semantic_type"]
                );
                graph[argument]["semantic_type"] = graph[receiver]["semantic_type"].clone();
            }
            3 => {
                let root = graph
                    .iter()
                    .find(|node| {
                        node["node_tag"] == "model" && node["semantic_form"] == "model_root"
                    })
                    .expect("selected root")["node_id"]
                    .clone();
                graph[position]["body"]["members"][3]["value"]["target"] = root;
            }
            _ => unreachable!(),
        }
        let bytes = serde_json::to_vec(&altered).expect("malformed package JSON");
        let refused = fsl_package::admit_checked_package(
            &bytes,
            &packages,
            fsl_package::V2ReadLimits::bounded(1 << 24),
        )
        .expect_err("malformed execution evidence refuses before evaluation");
        assert!(
            refused
                .pointer
                .as_deref()
                .is_some_and(|path| path.starts_with("/semantic_graph/nodes/"))
        );
        ws.write(&format!("{HANDOFF}/checked-package.json"), &bytes);
        let run = analyze_package(&ws, &format!("malformed-{defect}.json"), &members);
        let document = run.document();
        assert_eq!(run.exit, 20, "defect {defect}: {document}");
        assert_eq!(
            document["items"].as_array().map(Vec::len),
            Some(0),
            "{document}"
        );
        let diagnostic = &document["diagnostics"][0];
        assert_eq!(
            diagnostic["code"],
            refused.code.as_str(),
            "defect {defect}: {document}"
        );
        assert_eq!(
            diagnostic["cause"],
            json!(refused.cause),
            "defect {defect}: {document}"
        );
        assert_eq!(
            diagnostic["details"]["path"],
            json!(refused.pointer),
            "defect {defect}: {document}"
        );
    }
    let mut legacy = original;
    legacy["semantic_graph"]["nodes"][position]["body"]["members"]
        .as_array_mut()
        .expect("body bindings")
        .pop();
    let bytes = serde_json::to_vec(&legacy).expect("legacy JSON");
    let admitted = fsl_package::admit_checked_package(
        &bytes,
        &packages,
        fsl_package::V2ReadLimits::bounded(1 << 24),
    )
    .expect("legacy four-member package still admits");
    let bare = BarePackage::new(&admitted);
    assert!(matches!(
        bare.model_relation_form(&bare.claim_name("Ordered").expect("legacy claim")),
        Err(fsl_package::ModelRelationFormRefusal::MissingSubjectDeclarationOrder { .. })
    ));
    ws.write(&format!("{HANDOFF}/checked-package.json"), &bytes);
    let run = analyze_package(&ws, "legacy.json", &members);
    assert_eq!(label(&run.item(0)), "unsupported", "{}", run.document());
    assert_eq!(
        run.item(0)["record"]["diagnostic"]["code"],
        "unknown_required_feature"
    );
    assert_eq!(
        run.item(0)["record"]["diagnostic"]["cause"],
        "unsupported-feature"
    );
    assert_eq!(
        run.item(0)["record"]["diagnostic"]["detail"]["reason"],
        "missing-subject-declaration-order"
    );
    assert_eq!(label(&run.item(1)), "proved", "{}", run.document());
}

#[trace("FR-940-AC-26")]
#[test]
fn hp1_package_ambiguous_model_read_fails_only_its_item() {
    let ws = Workspace::new("hp1-package-ambiguous-read");
    let declarations = "relation Ambiguous using v over (x: V::Vault::step, y: V::Vault::step) { (if x.i = 0 then x.self else y.self).l = 0 }\ninvariant Sibling using v on V::Vault at current { true }";
    let (unit, model, snapshots) =
        request_files_over_starts(Vault::Secure, false, &STARTS, declarations);
    let path = request_over(
        &ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Ambiguous"}, {"claim": "Sibling", "kind": "operation-contract"}]}),
    );
    let request: Value =
        serde_json::from_slice(&std::fs::read(path).expect("request file")).expect("request JSON");
    let (members, bytes) = package_selection(&ws, &request["request"]);
    let (packages, _, _) = initial_world(&model);
    let admitted = fsl_package::admit_checked_package(
        &bytes,
        &packages,
        fsl_package::V2ReadLimits::bounded(1 << 24),
    )
    .expect("the typed reader admits the expression and its execution binders");
    let bare = BarePackage::new(&admitted);
    let selected = bare
        .claim_name("Ambiguous")
        .expect("the ambiguous claim is named");
    assert!(
        matches!(
            bare.model_relation_form(&selected),
            Err(fsl_package::ModelRelationFormRefusal::MalformedNode { .. })
        ),
        "the admitted expression has no unique execution owner"
    );
    forget_source(&ws);
    let run = analyze_package(&ws, "ambiguous.json", &members);
    assert_eq!(label(&run.item(0)), "failed", "{}", run.document());
    assert_eq!(
        run.item(0)["record"]["diagnostic"]["detail"]["selection"],
        "malformed-node"
    );
    assert!(run.item(0)["counterexample"].is_null());
    assert_eq!(label(&run.item(1)), "proved", "{}", run.document());
}

/// The source producer must carry every clause of the selected anchor, even
/// when its profile alias differs from the model alias. This seam emits real
/// bytes without asking the still-pinned reader to accept its known defect.
#[trace("FR-940-AC-23")]
#[test]
fn relation_emission_retains_exact_execution_clause_joins() {
    use fsl_forms::StateClauseKind;
    use fsl_semantics::check::{
        AggregateTerm, BindingValue, BodyTerm, GroupMember, LeafTerm, MemberTerm,
    };
    use fsl_semantics::value::Member;

    fn binding<'a>(aggregate: &'a AggregateTerm, name: &str) -> &'a BindingValue {
        aggregate
            .members
            .iter()
            .find_map(|member| match member {
                MemberTerm::Binding(binding) if binding.name == name => Some(&binding.value),
                _ => None,
            })
            .expect("named metadata binding")
    }
    fn reference(value: &BindingValue) -> filament_exact::NodeKey {
        let BindingValue::Leaf(LeafTerm::Reference { target }) = value else {
            panic!("metadata reference");
        };
        target.admitted().expect("admitted source key").clone()
    }
    const CLAUSES: &str = "pre First using v on V::Vault::step { i >= 0 }\n\
        pre Second using v on V::Vault::step { self.h >= 0 }\n\
        post Step using v on V::Vault::step { self.l = i }\n\
        post Keep using v on V::Vault::step { self.h = pre(self.h) }\n\
        post Other using v on V::Vault::reset { self.l = 0 }\n\
        invariant LocalH using v on V::Vault at current { self.h >= 0 }\n\
        invariant LocalL using v on V::Vault at current { self.l >= 0 }\n\
        invariant Foreign using v on N::Vault at current { self.l >= 0 }\n\
        constraint NotInvariant using v on V::Vault at current { true }\n";
    let document = hyper_world::document(false, true);
    let other = hyper_world::other_document(&document);
    let unit = hyper_world::unit_text(Some(&other), Vault::Custom(CLAUSES), false, DETERMINISTIC);
    let source = fsl_package::CheckedPackage::link(
        hyper_world::declarations(&[document.as_slice(), other.as_slice()], &unit)
            .check(
                filament_semantic_value::checking::CheckMode::Linked,
                filament_semantic_value::checking::CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .expect("both models and their independent invariants check"),
    );
    let graph = source.graph().semantic_graph();
    let relation = graph
        .nodes()
        .find(|node| node.semantic_form() == "step_relation")
        .expect("lowered relation");
    let BodyTerm::Aggregate(body) = relation.body() else {
        panic!("relation body");
    };
    let BindingValue::Aggregate(executions) = binding(body, "executions") else {
        panic!("executions aggregate");
    };
    assert_eq!(executions.members.len(), 2);
    for execution in &executions.members {
        let MemberTerm::Binding(execution) = execution else {
            panic!("execution binding");
        };
        let BindingValue::Aggregate(execution) = &execution.value else {
            panic!("execution facts");
        };
        let anchor = reference(binding(execution, "operation"));
        for (name, kind) in [
            ("preconditions", StateClauseKind::Precondition),
            ("postconditions", StateClauseKind::Postcondition),
        ] {
            let mut expected: Vec<_> = graph
                .nodes()
                .filter_map(|node| {
                    if node.semantic_form() != "state_clause" {
                        return None;
                    }
                    let BodyTerm::Application(application) = node.body() else {
                        return None;
                    };
                    let Some(Member::StateClause { clause, .. }) = application.operation.member()
                    else {
                        return None;
                    };
                    let Some(MemberTerm::Leaf(LeafTerm::Reference { target })) =
                        application.arguments.get(1)
                    else {
                        return None;
                    };
                    (*clause == kind && target.admitted() == Some(&anchor)).then(|| node.key())
                })
                .collect();
            expected.sort();
            expected.dedup();
            assert_eq!(
                expected.len(),
                2,
                "both clauses of this anchor, excluding reset"
            );
            let BindingValue::Group(group) = binding(execution, name) else {
                panic!("clause group");
            };
            let actual: Vec<_> = group
                .members
                .iter()
                .map(|member| {
                    let GroupMember::Leaf(LeafTerm::Reference { target }) = member else {
                        panic!("clause reference");
                    };
                    target.admitted().expect("admitted clause key").clone()
                })
                .collect();
            assert_eq!(
                actual, expected,
                "{name}: exact sorted and deduplicated anchor clauses"
            );
        }
    }
    let BindingValue::Group(subjects) = binding(body, "subjects") else {
        panic!("subject group");
    };
    let [GroupMember::Binding(subject)] = subjects.members.as_slice() else {
        panic!("one selected model");
    };
    let LeafTerm::Reference { target: root } = &subject.value else {
        panic!("model root reference");
    };
    let identity = root.admitted().expect("selected model root").package();
    let mut expected = Vec::new();
    let mut foreign = Vec::new();
    let mut constraints = Vec::new();
    for node in graph
        .nodes()
        .filter(|node| node.semantic_form() == "state_clause")
    {
        let BodyTerm::Application(application) = node.body() else {
            panic!("clause application");
        };
        let Some(Member::StateClause { clause, .. }) = application.operation.member() else {
            panic!("clause kind");
        };
        let Some(MemberTerm::Leaf(LeafTerm::Reference { target })) = application.arguments.get(1)
        else {
            panic!("clause anchor");
        };
        if *clause == StateClauseKind::Constraint {
            constraints.push(node.key());
        }
        if *clause != StateClauseKind::Invariant {
            continue;
        }
        let anchor = graph
            .node(target.admitted().expect("anchor key").clone())
            .expect("context object");
        let Some(fsl_semantics::check::Owner::Model(owner)) = anchor.owner() else {
            panic!("model-owned context");
        };
        if owner.identity() == identity {
            expected.push(node.key());
        } else {
            foreign.push(node.key());
        }
    }
    expected.sort();
    expected.dedup();
    assert_eq!(expected.len(), 2);
    assert_eq!(foreign.len(), 1, "the unrelated model invariant exists");
    assert_eq!(constraints.len(), 1, "the non-invariant control exists");
    let BindingValue::Aggregate(invariants) = binding(body, "invariants") else {
        panic!("invariant entries");
    };
    assert_eq!(invariants.members.len(), 1);
    let BindingValue::Group(group) = binding(invariants, &subject.name) else {
        panic!("selected invariant group");
    };
    let actual: Vec<_> = group
        .members
        .iter()
        .map(|member| {
            let GroupMember::Leaf(LeafTerm::Reference { target }) = member else {
                panic!("invariant reference");
            };
            target.admitted().expect("invariant key").clone()
        })
        .collect();
    assert_eq!(actual, expected, "exact sorted model-owned invariants");
    assert!(
        foreign
            .iter()
            .chain(&constraints)
            .all(|key| !actual.contains(key))
    );
    let emission = fsl_package::emit_checked(&source, fsl_package::EmitLimits::fixture())
        .expect("genuine source emits");
    let wire: Value = serde_json::from_slice(emission.package().bytes()).expect("emitted JSON");
    let relation = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("wire nodes")
        .iter()
        .find(|node| node["semantic_form"] == "step_relation")
        .expect("wire relation");
    assert_eq!(
        relation["body"]["members"][2]["value"]["members"][0]["value"]["members"]
            .as_array()
            .expect("wire model invariants")
            .len(),
        2
    );
    for execution in relation["body"]["members"][1]["value"]["members"]
        .as_array()
        .expect("wire executions")
    {
        for slot in [3, 4] {
            assert_eq!(
                execution["value"]["members"][slot]["value"]["members"]
                    .as_array()
                    .expect("wire clause group")
                    .len(),
                2
            );
        }
    }
}
