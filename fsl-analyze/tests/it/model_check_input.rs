// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531, FR-940: the model-check input of a model system, built on the
//! source route from the source package and on the package route once from
//! a `BarePackage`'s admitted nodes and handoff supply.

use fsl_eval::simulation::model::{ModelCheckInput, ModelSystem, PackageModelInput};
use fsl_eval::simulation::TransitionSystem;
use fsl_package::{BarePackage, BareSupply, CheckedPackage, EmitLimits, V2ReadLimits};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use ix_trace_rs::trace;

use crate::counter::{document, package_with_clauses, subject, Documents, CONTRACT, UNLIMITED};

const DIRECT_DENSE: &str =
    "dimension Time;\nunit ms : Time = rational(1, 1);\ntime M dense unit ms;\n";

/// Dense delay tables preserve the original clock identities and exact valuations.
#[trace("FR-940-AC-6", "FR-120")]
#[test]
fn package_direct_dense_clocks_preserve_initial_and_delay_identity() {
    use filament_exact::{Integer, Rational};
    use fsl_eval::simulation::model::TimedSystem;
    let world = crate::timed_world::world(
        &crate::timed_world::document(&["z", "a"], &[]),
        "",
        DIRECT_DENSE,
    );
    let request = world.request();
    let emission = fsl_package::emit_checked(&world.package, EmitLimits::fixture()).expect("emit");
    let admitted = fsl_package::admit_checked_package(
        emission.package().bytes(),
        request.packages,
        V2ReadLimits::fixture(),
    )
    .expect("admit bytes");
    let identities = emission
        .identity_rows(&admitted, &mut V2ReadLimits::fixture().native_join_budget())
        .expect("identities");
    let equality = emission
        .equality_witness(&admitted, &mut V2ReadLimits::fixture().native_join_budget())
        .expect("equality");
    let handoff = Handoff {
        admitted,
        identities,
        equality,
    };
    let bare = bare(&handoff);
    let input = PackageModelInput::of(&bare, request.packages, ModelNormalizationLimits::default())
        .expect("dense tables");
    let source = world.system();
    let mut bytes_request = request.clone();
    bytes_request.input = input.input();
    let bytes = ModelSystem::new(bytes_request).expect("bytes system");
    let source = TimedSystem::new(&source, "M").expect("source timed");
    let bytes = TimedSystem::new(&bytes, "test/timed").expect("bytes timed");
    assert_eq!(source.places().len(), 2);
    let identities = |timed: &TimedSystem<'_, '_>| {
        (0..timed.places().len())
            .map(|place| {
                timed
                    .clock_place_identity(place)
                    .filter(|identity| !identity.is_empty())
                    .expect("each clock place has its original canonical identity")
            })
            .collect::<Vec<_>>()
    };
    let source_identities = identities(&source);
    let bytes_identities = identities(&bytes);
    assert_eq!(source_identities, bytes_identities);
    for identities in [&source_identities, &bytes_identities] {
        assert_eq!(
            identities
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            identities.len(),
            "the two original clock fields have distinct canonical identities"
        );
    }
    assert_eq!(source.model().clocks(), bytes.model().clocks());
    let data_fields = |timed: &TimedSystem<'_, '_>| {
        timed
            .system()
            .input()
            .types
            .attributes(timed.model().clocks()[0].owner())
            .expect("the timed object's complete data declaration")
            .iter()
            .map(|attribute| attribute.field().clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        data_fields(&source).len(),
        4,
        "phase and all three Boolean fields"
    );
    assert_eq!(data_fields(&source), data_fields(&bytes));
    for (source, bytes) in source.places().iter().zip(bytes.places()) {
        assert_eq!(source.object(), bytes.object());
        assert_eq!(source.clock(), bytes.clock());
    }
    let source_initial = source.initial();
    let bytes_initial = bytes.initial();
    assert_eq!(source_initial.len(), 1);
    assert_eq!(source_initial[0].clocks().values().len(), 2);
    assert!(source_initial[0]
        .clocks()
        .values()
        .iter()
        .all(Rational::is_zero));
    assert_eq!(source_initial[0].clocks(), bytes_initial[0].clocks());
    let delay = Rational::new(Integer::from(3_i64), Integer::from(2_i64)).expect("delay");
    let source_delayed = source
        .delay(&source_initial[0], &delay)
        .expect("source delay")
        .expect("admissible");
    let bytes_delayed = bytes
        .delay(&bytes_initial[0], &delay)
        .expect("bytes delay")
        .expect("admissible");
    assert_eq!(source_delayed.clocks().values(), &[delay.clone(), delay]);
    assert_eq!(source_delayed.clocks(), bytes_delayed.clocks());
    assert_eq!(
        input.input().time_sources[0].span,
        world.package.graph().time_sources()[0].span
    );
    let table = input.input();
    let original = &world.package.graph().time_sources()[0].nominal_nodes;
    assert_eq!(table.time_sources[0].nominal_nodes.len(), 2);
    for (read, original) in table.time_sources[0].nominal_nodes.iter().zip(original) {
        assert_eq!(read.key, original.key);
        assert_eq!(read.form, original.form);
        assert_eq!(read.origin, original.origin);
        assert_eq!(read.dependencies, original.dependencies);
        assert!(read.nominal_payload().is_some());
    }
}

/// Unsupported timed definitions do not publish partially reconstructed tables.
#[trace("FR-940-AC-6")]
#[test]
fn package_direct_dense_reader_refuses_derived_units_and_timed_guards() {
    for body in [
        crate::timed_world::DENSE.to_owned(),
        format!("{DIRECT_DENSE}pre Guard using v on M::Call::send {{ self.x >= 1 ms }}\n"),
    ] {
        let world = crate::timed_world::world(
            &crate::timed_world::document(&["x"], &[("send", &[])]),
            "",
            &body,
        );
        let request = world.request();
        let emission =
            fsl_package::emit_checked(&world.package, EmitLimits::fixture()).expect("emit");
        let admitted = fsl_package::admit_checked_package(
            emission.package().bytes(),
            request.packages,
            V2ReadLimits::fixture(),
        )
        .expect("admit bytes");
        let identities = emission
            .identity_rows(&admitted, &mut V2ReadLimits::fixture().native_join_budget())
            .expect("identities");
        let equality = emission
            .equality_witness(&admitted, &mut V2ReadLimits::fixture().native_join_budget())
            .expect("equality");
        let handoff = Handoff {
            admitted,
            identities,
            equality,
        };
        let bare = bare(&handoff);
        assert!(matches!(
            PackageModelInput::of(&bare, request.packages, ModelNormalizationLimits::default()),
            Err(fsl_eval::simulation::model::PendingPackageForm::Form {
                form: fsl_package::UnreadModelForm::TimedModel,
                ..
            })
        ));
        assert!(
            PackageModelInput::of_timed_trace(
                &bare,
                request.packages,
                ModelNormalizationLimits::default()
            )
            .is_ok(),
            "supplied timed traces preserve their wider admitted model boundary"
        );
    }
}

/// Real analyze model items select the timed engine on both intake routes.
#[trace("FR-850", "FR-940-AC-6")]
#[test]
fn package_direct_dense_model_items_match_source_cli_results() {
    use crate::analyze_cli_support::{analyze, label, package_selection, request_over, Workspace};
    use serde_json::json;
    let body =
        format!("{DIRECT_DENSE}possible Yes using v {{ true }}\npossible No using v {{ false }}\n");
    let world = crate::timed_world::world(&crate::timed_world::document(&["x"], &[]), "", &body);
    let ws = Workspace::new("package-direct-dense-items");
    let path = request_over(
        &ws,
        std::str::from_utf8(&world.unit).expect("unit"),
        &world.document,
        &world.snapshots.values().cloned().collect::<Vec<_>>(),
        json!({"items": [{"claim": "Yes"}, {"claim": "No"}]}),
    );
    let source = analyze(&path);
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("JSON");
    let (selected, _) = package_selection(&ws, &document["request"]);
    let package_path = ws.write(
        "package-analyze.json",
        json!({"format": "native-analyze/1", "request": selected})
            .to_string()
            .as_bytes(),
    );
    let bytes = analyze(&package_path);
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    assert_eq!(label(&source.item(1)), "refuted", "{}", source.document());
    for index in 0..2 {
        assert_eq!(
            bytes.item(index)["record"]["backend"],
            "en-6",
            "{}",
            bytes.document()
        );
        assert_eq!(
            label(&bytes.item(index)),
            label(&source.item(index)),
            "{}",
            bytes.document()
        );
        assert_eq!(
            bytes.item(index)["record"]["method"],
            source.item(index)["record"]["method"]
        );
    }
    assert_eq!(bytes.exit, source.exit);
}

/// Missing graph coverage cannot become a false-claim replay decision.
#[trace("FR-940-AC-6", "FR-170-AC-4", "FR-245-AC-7")]
#[test]
fn dense_replay_rejects_incomplete_certificate_coverage() {
    use filament_exact::{Cancel, Integer};
    use fsl_analyze::zone_check::{explore_timed_state_graph, ZoneCheckLimits};
    use fsl_eval::simulation::model::{TimedSystem, DEFAULT_MAX_CLOCKS};
    use fsl_replay::{GraphEvidence, ModelPath, ReplayRefusal, TrapClosure};
    use fsl_route::deadlock::{ItemObligation, ObligationKind};
    let world = crate::timed_world::world(
        &crate::timed_world::document(&["x"], &[("send", &["phase"])]),
        "",
        &format!(
            "{DIRECT_DENSE}\
            pre Enabled using v on M::Call::send {{ self.phase = 0 }}\n\
            post SetPhase using v on M::Call::send {{ self.phase = 1 }}\n\
            possible Reach using v over (c: M::Call) {{ c.phase = 1 }}\n\
            possible Never using v over (c: M::Call) {{ c.phase = 2 }}\n"
        ),
    );
    let request = world.request();
    let emission = fsl_package::emit_checked(&world.package, EmitLimits::fixture()).expect("emit");
    let admitted = fsl_package::admit_checked_package(
        emission.package().bytes(),
        request.packages,
        V2ReadLimits::fixture(),
    )
    .expect("admit bytes");
    let identities = emission
        .identity_rows(&admitted, &mut V2ReadLimits::fixture().native_join_budget())
        .expect("identities");
    let equality = emission
        .equality_witness(&admitted, &mut V2ReadLimits::fixture().native_join_budget())
        .expect("equality");
    let handoff = Handoff {
        admitted,
        identities,
        equality,
    };
    let bare = bare(&handoff);
    let input = PackageModelInput::of(&bare, request.packages, ModelNormalizationLimits::default())
        .expect("dense tables and complete data declarations");
    let reach = world
        .package
        .graph()
        .state_graph_claim("Reach")
        .expect("Reach");
    assert!(
        matches!(
            input.state_graph_claim(reach.occurrence().expect("occurrence"), 1000),
            Err(fsl_eval::simulation::model::PendingClauseForm::Read(
                fsl_package::BareClauseRefusal::Pending {
                    fact: "dense-state-graph-predicate",
                    ..
                }
            ))
        ),
        "the bounded package CLI route keeps data-bearing predicates explicitly pending"
    );
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("timed source");
    let graph = explore_timed_state_graph(&timed, ZoneCheckLimits::default(), &Cancel::new())
        .expect("closed graph");
    assert_eq!(graph.nodes.len(), 2, "phase 0 and phase 1");
    assert_eq!(graph.edges.len(), 1, "the actual phase-changing operation");
    let instance = fsl_eval::model_subject::ModelInstance {
        population: request.universes[0].population.clone(),
        key: "c".to_owned(),
    };
    let nodes = graph
        .nodes
        .iter()
        .map(|node| fsl_replay::CertNode {
            state_key: fsl_eval::simulation::StateKey::from_canonical_bytes(
                system.state_key(&node.state).expect("state key").0,
            ),
            automaton_state: 0,
            zone: fsl_replay::Zone::universe(2),
            locked: false,
            product: None,
        })
        .collect::<Vec<_>>();
    let edges = graph
        .edges
        .iter()
        .map(|edge| fsl_replay::CertEdge {
            from: edge.from,
            to: edge.to,
            transition: Some(edge.transition.clone()),
            part: Some(edge.part_index),
            task_transition: None,
            horizon: None,
            silent: None,
        })
        .collect::<Vec<_>>();
    let certificate =
        |claim: &fsl_semantics::check::CheckedStateGraphClaim| fsl_replay::ZoneCertificate {
            obligation: ItemObligation {
                subject: (),
                kind: ObligationKind::StateGraphAuthored(
                    claim.occurrence().expect("occurrence").clone(),
                ),
                reductions: Default::default(),
            },
            scale: Integer::one(),
            lu_bounds: fsl_replay::LuBounds {
                lower: vec![None],
                upper: vec![None],
            },
            nodes: nodes.clone(),
            edges: edges.clone(),
            initials: graph.initials.clone(),
            components: None,
            non_zeno: None,
            divergence: None,
            admitted: None,
            state_graph: None,
            refinement: None,
            task_product: None,
        };
    let replay = |claim: &fsl_semantics::check::CheckedStateGraphClaim, certificate| {
        let occurrence = claim.occurrence().expect("occurrence").clone();
        let obligation = fsl_package::ObligationIdentity::new(occurrence.node().clone(), None, 0);
        let envelope = fsl_replay::GraphEvidenceEnvelope::new(
            obligation.clone(),
            occurrence.clone(),
            occurrence.node().clone(),
            Vec::new(),
            request.meter,
            Vec::new(),
            fsl_foundation::BackendId::new("en-1"),
            GraphEvidence::Trap {
                stem: ModelPath {
                    initial: 0,
                    steps: Vec::new(),
                },
                closure: TrapClosure::StateGraph,
                over: Some(instance.clone()),
            },
        )
        .with_zone_certificate(certificate);
        let mut wire = crate::model_settle_support::request(
            &world.package,
            &[],
            &world.document,
            world.snapshots.iter(),
            &world.unit,
        );
        wire.replay_target = fsl_replay::RequestTargetWire::default();
        wire.obligation_identity = obligation;
        wire.backend = "en-1".to_owned();
        wire.accounting_limits = request.meter;
        wire.source = fsl_replay::ReplaySource::ModelGraph(Box::new(envelope));
        fsl_replay::replay_model_graph(
            wire,
            &fsl_replay::ModelTraceSubject {
                timed: Some(fsl_replay::TimedReplay {
                    model: "M".to_owned(),
                    exploration: Default::default(),
                    max_clocks: DEFAULT_MAX_CLOCKS,
                    digital: false,
                }),
                initial: request.initial.clone(),
                universes: request.universes.clone(),
                expansion: request.expansion,
                stutter_closure: false,
            },
            fsl_replay::ReplayLimits::default(),
        )
    };
    let reach = world
        .package
        .graph()
        .state_graph_claim("Reach")
        .expect("Reach");
    let valid = certificate(reach);
    let over = system
        .bound_reference(reach.over().expect("over").object.clone(), &instance)
        .expect("bound c");
    assert_eq!(
        fsl_replay::check_zone_certificate(
            fsl_replay::ZoneCertificateRequest {
                simulation: request.clone(),
                model: "M",
                max_clocks: DEFAULT_MAX_CLOCKS,
                item: fsl_replay::CertifiedItem::StateGraph {
                    claim: reach,
                    over: Some(&over)
                },
            },
            &valid.obligation,
            |obligation| match &obligation.kind {
                ObligationKind::StateGraphAuthored(occurrence) =>
                    fsl_replay::CertificateItem::StateGraph(occurrence),
                _ => fsl_replay::CertificateItem::Other,
            },
            &valid,
            fsl_replay::CertificateCheckLimits::default(),
            &Cancel::new()
        )
        .expect("valid certificate checked"),
        fsl_replay::ZoneCertificateVerdict::Accepted
    );
    let fsl_replay::ReplayResult::ModelGraph(actual) =
        replay(reach, valid.clone()).expect("intact coverage admits")
    else {
        panic!("graph result");
    };
    assert_ne!(
        actual.settlement,
        fsl_replay::GraphReplaySettlement::Reproduced,
        "the reachable phase-1 target defeats the forged trap"
    );
    let mut missing_initial = valid.clone();
    missing_initial.initials = vec![graph.edges[0].to];
    let mut missing_successor = valid;
    missing_successor.edges.clear();
    for tampered in [missing_initial, missing_successor] {
        assert!(
            matches!(
                replay(reach, tampered),
                Err(ReplayRefusal::ModelGraphMismatch {
                    member: "zone_certificate"
                })
            ),
            "coverage failure must refuse before deciding the claim"
        );
    }
    let never = world
        .package
        .graph()
        .state_graph_claim("Never")
        .expect("Never");
    let fsl_replay::ReplayResult::ModelGraph(actual) =
        replay(never, certificate(never)).expect("complete false-claim certificate admits")
    else {
        panic!("graph result");
    };
    assert_eq!(
        actual.settlement,
        fsl_replay::GraphReplaySettlement::Reproduced,
        "complete graph coverage still replays a genuine false claim"
    );
}

/// The Counter domain package with no operation.
fn still_document() -> Vec<u8> {
    crate::model_world::still_counter().document
}

/// The admitted bytes `package` was written to, with the identity and
/// equality rows of its handoff.
struct Handoff {
    admitted: fsl_package::CheckedPackageV2,
    identities: fsl_package::ReadIdentityRows,
    equality: fsl_package::ReadEqualityRows,
}

fn handoff(package: &CheckedPackage, documents: &Documents) -> Handoff {
    let emission = fsl_package::emit_checked(package, EmitLimits::fixture()).expect("emits");
    let admitted = fsl_package::admit_checked_package(
        emission.package().bytes(),
        &documents.packages,
        V2ReadLimits::fixture(),
    )
    .expect("the strict reader admits the written bytes");
    let identities = emission
        .identity_rows(&admitted, &mut V2ReadLimits::fixture().native_join_budget())
        .expect("the identity rows");
    let equality = emission
        .equality_witness(&admitted, &mut V2ReadLimits::fixture().native_join_budget())
        .expect("the equality rows");
    Handoff {
        admitted,
        identities,
        equality,
    }
}

fn bare(handoff: &Handoff) -> BarePackage<'_> {
    BarePackage::new(&handoff.admitted)
        .with_supply(
            BareSupply {
                identities: &handoff.identities,
                equality: Some(&handoff.equality),
            },
            &mut V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply")
}

/// The state keys of `system`'s initial states, in order.
fn initial_keys(system: &ModelSystem<'_>) -> Vec<Vec<u8>> {
    system
        .initial()
        .iter()
        .map(|state| system.state_key(state).expect("a state key").0)
        .collect()
}

/// FR-940-AC-6 (model-check input half), FR-531: over a Counter package whose
/// model declares no operation, the package route builds the model-check
/// input once from the admitted bytes and handoff rows, and the model system
/// over it has the source route's populations, requires-bound roots and
/// initial state keys.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn the_package_route_input_builds_the_source_route_model_system() {
    let model = still_document();
    let (package, _) = package_with_clauses(&model, "", &[]);
    let mut documents = Documents::new(model);
    let start = documents.start("start", &[("a", 1), ("b", 2)]);
    let source = subject(&package, &documents, vec![start], &["a", "b"], UNLIMITED);
    let handoff = handoff(&package, &documents);
    let bare = bare(&handoff);
    let input = PackageModelInput::of(
        &bare,
        &documents.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package route reads every form");
    let mut request = source.simulation.clone();
    request.input = input.input();
    assert!(request.source_package().is_none());
    assert!(source.simulation.source_package().is_some());

    let over_source = ModelSystem::new(source.simulation.clone()).expect("the source admits");
    let over_bytes = ModelSystem::new(request).expect("the package route admits");
    assert_eq!(
        format!("{:?}", over_source.populations()),
        format!("{:?}", over_bytes.populations())
    );
    assert_eq!(over_source.rooted_domains(), over_bytes.rooted_domains());
    assert_eq!(initial_keys(&over_source), initial_keys(&over_bytes));
    assert_eq!(initial_keys(&over_bytes).len(), 1);
    assert!(over_bytes.source_package().is_none());
}

/// An operation parameter resolves its unit from authenticated source declarations.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn package_operation_parameters_read_admitted_unit_names() {
    let mut model: serde_json::Value = serde_json::from_slice(&document(true)).expect("model JSON");
    let counter = model["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|node| node["identity"] == "ix://test/counter/Counter")
        .expect("Counter");
    let set_to = counter["operations"]
        .as_array_mut()
        .expect("operations")
        .iter_mut()
        .find(|operation| operation["name"] == "setTo")
        .expect("setTo");
    set_to["params"][0]["typeRef"] = serde_json::json!("ix://filament/unit/ms");
    let model = serde_json::to_vec(&model).expect("model bytes");
    let (package, _) = package_with_clauses(
        &model,
        "dimension Time;\nunit ms : Time = rational(1, 1);\n",
        &[],
    );
    let documents = Documents::new(model);
    let handoff = handoff(&package, &documents);
    let bare = bare(&handoff);
    let input = PackageModelInput::of(
        &bare,
        &documents.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the admitted unit names rebuild quantity-parameter operations");
    let source = ModelCheckInput::source(&package);
    assert_eq!(input.input().operations, source.operations);
    let parameter = source
        .types
        .object_types()
        .flat_map(|object| source.operations.declared_by(object.key()).iter())
        .find(|operation| operation.name() == "setTo")
        .expect("setTo remains declared");
    assert!(
        matches!(
            &parameter.parameters()[0].1,
            filament_exact::ValueType::Quantity(_)
        ),
        "{parameter:?}"
    );
}

/// The class of each state `system` reaches from its initial states, in
/// breadth-first order.
fn terminal_classes(
    system: &ModelSystem<'_>,
) -> Vec<(Vec<u8>, fsl_eval::simulation::model::TerminalClass)> {
    let mut queue: std::collections::VecDeque<_> = system.initial().into_iter().collect();
    let mut seen: Vec<Vec<u8>> = Vec::new();
    let mut classes = Vec::new();
    while let Some(state) = queue.pop_front() {
        let key = system.state_key(&state).expect("a state key").0;
        if seen.contains(&key) {
            continue;
        }
        seen.push(key.clone());
        let expansion = system.successors(&state).expect("expands");
        let class = system
            .terminal_class(&state, !expansion.successors.is_empty())
            .expect("the class reads");
        classes.push((key, class));
        queue.extend(expansion.successors.into_iter().map(|(_, next)| next));
    }
    classes
}

#[trace("TC-519", "FR-124-AC-5", "FR-124-AC-8", "TC-1076", "FR-391-AC-9")]
#[test]
fn terminal_when_population_and_self_classify_equally_on_both_routes() {
    use fsl_eval::simulation::model::TerminalClass;
    for (predicate, empty) in [
        ("self.value = 3", TerminalClass::Intended),
        (
            "forall(c in M::counters : c.value = 3)",
            TerminalClass::Intended,
        ),
        (
            "exists(c in M::counters : c.value = 3)",
            TerminalClass::Deadlocked,
        ),
    ] {
        for (objects, expected) in [
            (vec![("a", 3), ("b", 3)], TerminalClass::Intended),
            (vec![("a", 1), ("b", 2)], TerminalClass::Deadlocked),
            (vec![], empty),
        ] {
            let model = still_document();
            let body = format!("terminal Done using v on M when {{ {predicate} }};\n");
            let (package, _) = package_with_clauses(&model, &body, &[]);
            let mut documents = Documents::new(model);
            let start = documents.start("start", &objects);
            let keys: Vec<_> = objects.iter().map(|(key, _)| *key).collect();
            let source = subject(&package, &documents, vec![start], &keys, UNLIMITED);
            let handoff = handoff(&package, &documents);
            let bare = bare(&handoff);
            let input = PackageModelInput::of(
                &bare,
                &documents.packages,
                ModelNormalizationLimits::default(),
            )
            .expect("package input");
            let mut request = source.simulation.clone();
            request.input = input.input();
            for system in [
                ModelSystem::new(source.simulation).expect("source system"),
                ModelSystem::new(request).expect("package system"),
            ] {
                let states = terminal_classes(&system);
                assert_eq!(states.len(), 1);
                assert_eq!(states[0].1, expected, "{predicate}, {objects:?}");
            }
        }
    }
}

/// FR-411, FR-940, FR-124: a package holding a `terminal` declaration builds
/// the package-route input, which reads the declaration from its admitted
/// terminal row and classifies every reachable state as the source route does
/// for `terminal any`, `terminal when` over `self`, and no declaration.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_package_route_input_reads_a_terminal_declaration_as_the_source_does() {
    use fsl_eval::simulation::model::TerminalClass;
    for (declaration, expect_intended) in [
        ("", false),
        ("terminal Halt using v on M any;\n", true),
        (
            "terminal Done using v on M when { self.value = 3 };\n",
            true,
        ),
        (
            "terminal Done using v on M when { self.value = 2 };\n",
            false,
        ),
    ] {
        let model = document(false);
        let body = format!("{CONTRACT}{declaration}");
        let (package, _) = package_with_clauses(&model, &body, &[]);
        let mut documents = Documents::new(model);
        let start = documents.start("start", &[("a", 0)]);
        let source = subject(&package, &documents, vec![start], &["a"], UNLIMITED);
        let handoff = handoff(&package, &documents);
        let bare = bare(&handoff);
        let input = PackageModelInput::of(
            &bare,
            &documents.packages,
            ModelNormalizationLimits::default(),
        )
        .expect("the package route reads the terminal declaration");
        let mut request = source.simulation.clone();
        request.input = input.input();
        let over_bytes = ModelSystem::new(request).expect("the package route admits");
        let over_source = ModelSystem::new(source.simulation).expect("the source admits");
        let (from_bytes, from_source) = (
            terminal_classes(&over_bytes),
            terminal_classes(&over_source),
        );
        assert_eq!(from_bytes, from_source, "{declaration}");
        let source_keys = ModelCheckInput::source(&package).terminal_keys();
        let package_keys = input.input().terminal_keys();
        assert_eq!(
            source_keys.mismatch(&package_keys),
            None,
            "{declaration}: the declarations join by selection identity"
        );
        assert_eq!(source_keys, package_keys, "{declaration}");
        assert_eq!(
            from_source
                .iter()
                .any(|(_, class)| *class == TerminalClass::Intended),
            expect_intended,
            "{declaration}"
        );
    }
}

/// The canonical keys of every state `system` reaches from its initial
/// states, in breadth-first order, with each state's successor keys.
fn reachable(system: &ModelSystem<'_>) -> Vec<(Vec<u8>, Vec<Vec<u8>>)> {
    let key = |state: &<ModelSystem<'_> as TransitionSystem>::State| {
        system.state_key(state).expect("a state key").0
    };
    let mut queue: std::collections::VecDeque<_> = system.initial().into_iter().collect();
    let mut seen = Vec::new();
    while let Some(state) = queue.pop_front() {
        let at = key(&state);
        if seen.iter().any(|(known, _)| *known == at) {
            continue;
        }
        let expansion = system.successors(&state).expect("expands");
        let next: Vec<_> = expansion
            .successors
            .into_iter()
            .map(|(_, next)| next)
            .collect();
        seen.push((at, next.iter().map(key).collect()));
        queue.extend(next);
    }
    seen
}

/// FR-940, FR-850: a model whose domain package declares operations is built
/// over a package-route input with the operation table S3 declares for its
/// selected domain packages, so its steps reach the source route's states
/// with the source route's successors, rather than being refused.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_package_route_system_steps_the_operations_of_its_domain_packages() {
    let model = document(false);
    let (package, _) = package_with_clauses(&model, "", &[]);
    let mut documents = Documents::new(model);
    let start = documents.start("start", &[("a", 1)]);
    let source = subject(&package, &documents, vec![start], &["a"], UNLIMITED);
    let handoff = handoff(&package, &documents);
    let bare = bare(&handoff);
    let input = PackageModelInput::of(
        &bare,
        &documents.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package holds no unread node");
    assert_eq!(
        input.input().operations,
        package.graph().scope().operations(),
        "the package route declares the source route's operations"
    );
    assert_ne!(
        *package.graph().scope().operations(),
        fsl_semantics::model::operation::OperationTable::default(),
        "the Counter declares operations"
    );
    let mut request = source.simulation.clone();
    request.input = input.input();
    let over_bytes = ModelSystem::new(request).expect("the package route admits operations");
    let over_source = ModelSystem::new(source.simulation).expect("the source admits");
    let (from_bytes, from_source) = (reachable(&over_bytes), reachable(&over_source));
    assert_eq!(from_bytes, from_source);
    assert!(
        from_source.iter().any(|(_, next)| !next.is_empty()),
        "the operations step the counter"
    );
}

/// FR-531: the source route's input reads the source package's tables and
/// evaluates through the source package.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn the_source_route_input_reads_the_source_package() {
    let model = document(false);
    let (package, _) = package_with_clauses(&model, CONTRACT, &[]);
    let input = ModelCheckInput::source(&package);
    let graph = package.graph();
    assert!(std::ptr::eq(input.types, graph.scope().types()));
    assert!(std::ptr::eq(input.operations, graph.scope().operations()));
    let names: Vec<&str> = input
        .state_clauses
        .iter()
        .map(|clause| clause.name.as_str())
        .collect();
    let checked: Vec<&str> = graph
        .state_clauses()
        .iter()
        .map(|clause| clause.name())
        .collect();
    assert_eq!(names, checked);
    assert_eq!(names.len(), 2);
    assert!(input
        .source_package()
        .is_some_and(|source| std::ptr::eq(source, &package)));
}

/// FR-940, FR-850 (QSL-2425): over a Counter package whose domain package
/// declares its operations, EN-1 checks the package form of each temporal
/// clause, read from the package route's admitted bytes by its node key,
/// through letters that evaluate its `holds` atoms keyed by node, and gives
/// the source route's outcome: `Bounded` proved and `AtMostTwo` refuted with
/// the same counterexample.
#[trace("TC-2958", "FR-940-AC-6")]
#[trace("TC-2410", "FR-850-AC-2")]
#[test]
fn the_model_checker_decides_a_package_form_clause_as_its_source_form() {
    use fsl_analyze::model_check::{check_model, ModelCheckRequest};
    use fsl_replay::{ModelCheckItem, ModelCheckLimits};
    let model = document(false);
    let (package, _) = package_with_clauses(
        &model,
        "",
        &[
            ("Bounded", "", "always holds(c.value <= 3)"),
            ("AtMostTwo", "", "always holds(c.value <= 2)"),
        ],
    );
    let mut documents = Documents::new(model);
    let start = documents.start("start", &[("a", 0)]);
    let source = subject(&package, &documents, vec![start], &["a"], UNLIMITED);
    let handoff = handoff(&package, &documents);
    let bare = bare(&handoff);
    let input = PackageModelInput::of(
        &bare,
        &documents.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package holds no unread node");
    let mut over_bytes = source.simulation.clone();
    over_bytes.input = input.input();
    let source_system = ModelSystem::new(source.simulation.clone()).expect("the source admits");
    let package_system = ModelSystem::new(over_bytes.clone()).expect("the package route admits");
    fn run(
        simulation: &fsl_eval::simulation::model::ModelSimulationRequest<'_>,
        item: ModelCheckItem<'_>,
    ) -> Result<String, String> {
        check_model(
            ModelCheckRequest {
                workers: 1,
                temporal_clock_selections: &[],
                subject: fsl_eval::model_subject::ModelSubject {
                    simulation: simulation.clone(),
                    proof_bounds: Vec::new(),
                },
                item,
                max_depth: None,
                limits: ModelCheckLimits::default(),
                smt_encoding: Default::default(),
                smt_run: Default::default(),
                reductions: Default::default(),
            },
            &filament_exact::Cancel::new(),
        )
        .map(|outcome| format!("{outcome:?}"))
        .map_err(|refusal| format!("{refusal:?}"))
    }
    let mut verdicts = Vec::new();
    for node in package.graph().temporal_clauses() {
        let key = node.identity();
        let clause = input
            .temporal_clause(
                &fsl_foundation::source::provenance::OccurrenceKey::new(
                    key.clone(),
                    node.claim().clone(),
                ),
                filament_semantic_value::checking::CheckingLimits::default().nodes(),
            )
            .unwrap_or_else(|refusal| panic!("{}: {refusal}", node.clause().name()));
        let source_letters = source_system.clause_letters(node).expect("source letters");
        let package_letters = package_system
            .package_clause_letters(&clause, &key)
            .expect("package letters");
        let from_source = run(
            &source.simulation,
            ModelCheckItem::Clause {
                clause: node.clause(),
                letters: &source_letters,
            },
        );
        let from_package = run(
            &over_bytes,
            ModelCheckItem::Clause {
                clause: &clause,
                letters: &package_letters,
            },
        );
        assert_eq!(from_package, from_source, "{}", node.clause().name());
        verdicts.push(from_package.expect("checked").starts_with("Holds"));
    }
    assert_eq!(verdicts, [true, false], "Bounded holds, AtMostTwo does not");
}

/// FR-940-AC-6 (deadlock-freedom, QSL-2403's engine half): EN-1 decides the
/// derived deadlock-freedom item over the package route's model system with
/// the source route's outcome, for a Counter whose operations stay enabled
/// and for one whose model declares none, which deadlocks at its start.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn the_model_checker_decides_deadlock_freedom_over_the_package_route_as_over_source() {
    use fsl_analyze::model_check::{check_model, ModelCheckRequest};
    use fsl_replay::{ModelCheckItem, ModelCheckLimits};
    let mut verdicts = Vec::new();
    for model in [document(false), still_document()] {
        let (package, _) = package_with_clauses(&model, "", &[]);
        let mut documents = Documents::new(model);
        let start = documents.start("start", &[("a", 0)]);
        let source = subject(&package, &documents, vec![start], &["a"], UNLIMITED);
        let handoff = handoff(&package, &documents);
        let bare = bare(&handoff);
        let input = PackageModelInput::of(
            &bare,
            &documents.packages,
            ModelNormalizationLimits::default(),
        )
        .expect("the package holds no unread node");
        let mut over_bytes = source.simulation.clone();
        over_bytes.input = input.input();
        let run = |simulation: fsl_eval::simulation::model::ModelSimulationRequest<'_>| {
            check_model(
                ModelCheckRequest {
                    workers: 1,
                    temporal_clock_selections: &[],
                    subject: fsl_eval::model_subject::ModelSubject {
                        simulation,
                        proof_bounds: Vec::new(),
                    },
                    item: ModelCheckItem::DeadlockFreedom,
                    max_depth: None,
                    limits: ModelCheckLimits::default(),
                    smt_encoding: Default::default(),
                    smt_run: Default::default(),
                    reductions: Default::default(),
                },
                &filament_exact::Cancel::new(),
            )
            .map(|outcome| format!("{outcome:?}"))
            .map_err(|refusal| format!("{refusal:?}"))
        };
        let from_source = run(source.simulation.clone());
        let from_package = run(over_bytes);
        assert_eq!(from_package, from_source);
        verdicts.push(from_package.expect("checked").starts_with("Holds"));
    }
    assert_eq!(verdicts, [true, false], "the still Counter deadlocks");
}

/// FR-850-AC-2, FR-940-AC-6 (deadlock-freedom, QSL-2403): over a Counter
/// world whose model declares no operation, so it deadlocks at its start,
/// the derived deadlock-freedom item settles through `derived_model_item`
/// with the same record on the package route as on the source route, its
/// anchor and terminal declarations joined against the admitted package;
/// and the counterexample replays from the package bytes and the handoff
/// files with the source replay's settlement.
#[trace("TC-2410", "FR-850-AC-2")]
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_package_route_deadlock_settles_and_replays_as_the_source_deadlock() {
    use crate::model_world::{still_counter, Clause, World, INFINITE};
    let world = World::new(
        still_counter(),
        "",
        INFINITE,
        &[Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(c.value <= 3)",
        }],
        &[("c", serde_json::json!({"value": {"integer": "0"}}))],
    );
    let replayed = deadlock_parity(&world, "Refuted");
    assert!(!replayed.contains("Undefined"), "{replayed}");
}

/// FR-411, FR-940, FR-128-AC-6: under `terminal when q(3 - self.value) = 0`
/// (`q` divides 6 by its argument, a Kernel function), the deadlock item is
/// refuted by an undefined evaluation at the terminal predicate, and the
/// package route settles it, and replays it through the terminal locus its
/// admitted node names, as the source route does.
#[trace("TC-2958", "FR-940-AC-6")]
#[trace("TC-539", "FR-128-AC-6")]
#[test]
fn a_package_route_undefined_terminal_replays_as_the_source_undefined_terminal() {
    use crate::model_world::{counter, Clause, World, INFINITE};
    let world = World::with_kernel_functions(
        counter(),
        &format!("{CONTRACT}terminal Done using v on M when {{ q(3 - self.value) = 0 }};\n"),
        "function q using v(x: Integer): Integer pure { 6 mod x }\n",
        INFINITE,
        &[Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(c.value <= 3)",
        }],
        &[("c", serde_json::json!({"value": {"integer": "0"}}))],
    );
    let replayed = deadlock_parity(&world, "Refuted");
    assert!(replayed.contains("Undefined"), "{replayed}");
}

/// The deadlock-freedom item over `world`, settled and replayed on both
/// routes, which must agree; the source replay's settlement.
fn deadlock_parity(world: &crate::model_world::World, settles: &str) -> String {
    use filament_exact::Cancel;
    use fsl_analyze::analyze::{
        analyze_items, deadlock_model_item, derived_model_item, AnalyzeLimits, Engine, Engines,
        PackageDeadlockSubject, RunInput, TemporalItem,
    };
    use fsl_route::request::{DerivedLimits, ItemKind};
    let written = crate::derived_request_support::written(
        &world.subject(&["c"], UNLIMITED),
        &DerivedLimits::default(),
    );
    let derived = &written.items()[1];
    assert_eq!(derived.item_kind(), Some(ItemKind::DeadlockFreedom));
    let obligation = written.obligation(derived.index()).expect("its obligation");
    let form = TemporalItem {
        item_kind: ItemKind::DeadlockFreedom,
        ..TemporalItem::default()
    };
    let run = RunInput::source(&world.package);
    let source_item = || {
        deadlock_model_item(
            &run,
            &world.binding(&["c"]),
            AnalyzeLimits::unbounded(),
            &world.provision,
            derived.occurrence(),
            &form,
        )
        .expect("the derived target is formed")
    };
    let handoff = world.handoff();
    let bare = handoff.bare();
    let input = PackageModelInput::of(
        &bare,
        world.binding(&["c"]).packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package route reads every form");
    let package_item = || {
        PackageDeadlockSubject::new(
            &input,
            &handoff.bytes,
            handoff.supply(),
            world.binding(&["c"]),
            AnalyzeLimits::unbounded(),
            derived.occurrence(),
            &form,
        )
        .model_item()
    };

    let mut engines = Engines::default();
    engines
        .with(Engine::model_check().expect("EN-1's own manifest is admitted"))
        .expect("one engine");
    let settled = |item: fsl_analyze::analyze::ModelItem<'_>| {
        let item = derived_model_item(derived, obligation, item).expect("the join admits");
        let mut records = analyze_items(&engines, vec![item], &Cancel::new());
        format!("{:?}", records.remove(0))
    };
    let from_source = settled(source_item());
    assert_eq!(settled(package_item()), from_source);
    assert!(from_source.contains(settles), "{from_source}");

    let outcome = fsl_analyze::model_check::check_model(source_item().request, &Cancel::new())
        .expect("checked");
    let fsl_replay::ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        assert_eq!(settles, "Proved", "a violation is expected: {outcome:?}");
        return String::new();
    };
    let source_replay = (source_item().replay)(&counterexample).expect("the source replays");
    let package_replay = (package_item().replay)(&counterexample).expect("the bytes replay");
    assert_eq!(
        format!("{:?}", package_replay.settlement),
        format!("{:?}", source_replay.settlement)
    );
    assert_eq!(package_replay.hints, source_replay.hints);
    assert_eq!(
        source_replay.terminals.mismatch(&package_replay.terminals),
        None
    );
    format!("{:?}", source_replay.settlement)
}

/// The Counter world under `body`, with one clause and one counter at 0.
fn counter_with(body: &str) -> crate::model_world::World {
    use crate::model_world::{counter, Clause, World, INFINITE};
    World::new(
        counter(),
        body,
        INFINITE,
        &[Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(c.value <= 3)",
        }],
        &[("c", serde_json::json!({"value": {"integer": "0"}}))],
    )
}

/// FR-411 "Deadlock-freedom verdict join", FR-940: a derived deadlock-freedom
/// item over the Counter unit with `terminal when { self.value = 3 }` is not
/// refused `TerminalDeclaration` on the package route: the declarations are
/// keyed by their model's selection identity on both routes, and the item
/// settles, and replays, equal on both.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_package_route_deadlock_item_joins_a_terminal_declaration_as_the_source_does() {
    // With `terminal any` there is no derived item (FR-411), so the item is
    // written over the `when` declaration; `terminal any` joins by its keys
    // (the terminal-class parity test above).
    let world = counter_with(&format!(
        "{CONTRACT}terminal Done using v on M when {{ self.value = 3 }};\n"
    ));
    deadlock_parity(&world, "Proved");
}

/// FR-411-AC-5 (package route): for a derived deadlock-freedom item written
/// over the `when` package, an EN-1 run over the package route's no-declaration
/// package is refused `TerminalDeclaration` (`stale_dependency`/
/// `content-mismatch`), naming the model's selection identity, the `when`
/// terminal node key and none; over the `when` package itself the join admits.
#[trace("TC-1142", "FR-411-AC-5")]
#[test]
fn a_package_route_run_over_another_terminal_declaration_is_refused_as_the_source_is() {
    use fsl_analyze::analyze::DerivedItemMismatch;
    use fsl_analyze::analyze::{
        derived_model_item, AnalyzeLimits, PackageDeadlockSubject, TemporalItem,
    };
    use fsl_route::request::{DerivedLimits, ItemKind};

    let when = counter_with(&format!(
        "{CONTRACT}terminal Done using v on M when {{ self.value = 3 }};\n"
    ));
    let none = counter_with(CONTRACT);
    let written = crate::derived_request_support::written(
        &when.subject(&["c"], UNLIMITED),
        &DerivedLimits::default(),
    );
    let derived = &written.items()[1];
    assert_eq!(derived.item_kind(), Some(ItemKind::DeadlockFreedom));
    let obligation = written.obligation(derived.index()).expect("its obligation");
    let form = TemporalItem {
        item_kind: ItemKind::DeadlockFreedom,
        ..TemporalItem::default()
    };
    let join = |world: &crate::model_world::World| {
        let handoff = world.handoff();
        let bare = handoff.bare();
        let input = PackageModelInput::of(
            &bare,
            world.binding(&["c"]).packages,
            ModelNormalizationLimits::default(),
        )
        .expect("the package route reads the unit");
        let item = PackageDeadlockSubject::new(
            &input,
            &handoff.bytes,
            handoff.supply(),
            world.binding(&["c"]),
            AnalyzeLimits::unbounded(),
            derived.occurrence(),
            &form,
        )
        .model_item();
        derived_model_item(derived, obligation, item).err()
    };
    assert_eq!(join(&when), None, "the written package joins");
    let when_keys = when.package.graph().terminal_keys();
    let refused = join(&none);
    let Some(DerivedItemMismatch::TerminalDeclaration { mismatch, .. }) = refused else {
        panic!("a run over the no-declaration package is refused: {refused:?}");
    };
    assert_eq!(mismatch.model, "test/counter");
    assert_eq!(mismatch.recompiled, None);
    assert_eq!(
        when_keys.mismatch(&none.package.graph().terminal_keys()),
        Some(mismatch),
        "the package route names what the source route names"
    );
}

/// FR-411, FR-940: the package route finds the `terminal when` declaration
/// whose predicate is at a locus, as the source route does: the locus of the
/// source predicate's body names the same model, by selection identity, on
/// both routes, and a locus of no terminal names none. This is the lookup the
/// state-graph replay of an undefined terminal predicate resolves its locus
/// through.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_package_route_finds_the_terminal_whose_predicate_is_at_a_locus() {
    let model = document(false);
    let body = format!("{CONTRACT}terminal Done using v on M when {{ self.value = 3 }};\n");
    let (package, _) = package_with_clauses(&model, &body, &[]);
    let documents = Documents::new(model);
    let handoff = handoff(&package, &documents);
    let bare = bare(&handoff);
    let input = PackageModelInput::of(
        &bare,
        &documents.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package route reads the terminal");
    let terminals = fsl_package::resolve_terminals(&package).expect("the terminals resolve");
    let [terminal] = terminals.as_slice() else {
        panic!("one terminal: {terminals:?}");
    };
    let fsl_package::PackageTerminalDeclaration::When(predicate) = terminal.declaration else {
        panic!("a `when` declaration");
    };
    let locus = predicate.body().location().clone();
    let source = ModelCheckInput::source(&package);
    let over_bytes = input.input();
    let from_source = source.terminal_at(&locus).expect("resolves");
    let from_bytes = over_bytes.terminal_at(&locus).expect("resolves");
    assert_eq!(from_bytes, from_source);
    let found = from_bytes.expect("the locus names the terminal");
    assert_eq!(found.identity(), "test/counter");
    assert_eq!(
        found.label(),
        "test/counter",
        "the package route has no alias"
    );
    assert_eq!(source.terminal_at(&locus).unwrap().unwrap().label(), "M");
    let elsewhere = filament_semantic_value::location::ExpressionLocation::root(
        filament_semantic_value::location::ExpressionOwner::Expression,
    );
    assert_eq!(over_bytes.terminal_at(&elsewhere).expect("resolves"), None);
    assert_eq!(source.terminal_at(&elsewhere).expect("resolves"), None);
}

/// FR-124, FR-411, FR-940: on a model of several object types, `self` of a
/// `terminal when` ranges over the one canonical member type of the model's
/// populations, on the package route as on the source route. Here `Aaa`,
/// `Counter` and `Zeta` are object types and the one population holds
/// `Counter` and `Zeta`: its canonical member `Counter` (the least) is the
/// terminal object, not the least object type `Aaa`. A predicate that reads `self`
/// is refused at assembly unless the populations cover one object type, so
/// the predicate here reads nothing; the context type is still the one the
/// source lowering names.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_package_route_terminal_self_is_the_populations_canonical_member() {
    use fsl_eval::simulation::model::TerminalClass;
    use fsl_package::BareTerminalDeclaration;
    let mut model: serde_json::Value =
        serde_json::from_slice(&still_document()).expect("model JSON");
    let counter = model["types"]
        .as_array()
        .expect("types")
        .iter()
        .find(|node| node["identity"] == "ix://test/counter/Counter")
        .expect("Counter")
        .to_string();
    for name in ["Aaa", "Zeta"] {
        let node: serde_json::Value =
            serde_json::from_str(&counter.replace("Counter", name)).expect("the renamed type");
        model["types"].as_array_mut().expect("types").push(node);
    }
    model["populations"][0]["members"] =
        serde_json::json!(["ix://test/counter/Counter", "ix://test/counter/Zeta"]);
    let model = serde_json::to_vec(&model).expect("model bytes");

    let body = "terminal Done using v on M when { true };\n";
    let (package, _) = package_with_clauses(&model, body, &[]);
    let mut documents = Documents::new(model);
    let start = documents.start("start", &[("a", 3), ("b", 1)]);
    let source = subject(&package, &documents, vec![start], &["a", "b"], UNLIMITED);
    let handoff = handoff(&package, &documents);
    let bare = bare(&handoff);

    let terminals = bare
        .terminal_declarations()
        .expect("the terminal rows read");
    let [terminal] = terminals.as_slice() else {
        panic!("one terminal declaration");
    };
    let BareTerminalDeclaration::When(predicate) = &terminal.declaration else {
        panic!("a `when` declaration");
    };
    let context = format!("{:?}", predicate.context());
    assert!(context.contains("/Counter"), "{context}");

    let input = PackageModelInput::of(
        &bare,
        &documents.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package route reads the terminal");
    let mut request = source.simulation.clone();
    request.input = input.input();
    let over_bytes = ModelSystem::new(request).expect("the package route admits");
    let over_source = ModelSystem::new(source.simulation).expect("the source admits");
    let classes = |system: &ModelSystem<'_>| {
        terminal_classes(system)
            .into_iter()
            .map(|(_, class)| class)
            .collect::<Vec<_>>()
    };
    assert_eq!(classes(&over_bytes), classes(&over_source));
    assert!(classes(&over_source).contains(&TerminalClass::Intended));
}
