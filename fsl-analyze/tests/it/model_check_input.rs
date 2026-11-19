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

/// FR-411, FR-940, FR-124: a package holding a `terminal` declaration builds
/// the package-route input, which reads the declaration from its admitted
/// terminal row and classifies every reachable state as the source route does
/// for `terminal any`, `terminal when` over `self`, and no declaration.
#[trace("TC-2958", "FR-940-AC-6")]
#[trace("TC-487", "FR-060-AC-6")]
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
                temporal_clock_selections: &[],
                subject: fsl_eval::model_subject::ModelSubject {
                    simulation: simulation.clone(),
                    proof_bounds: Vec::new(),
                },
                item,
                max_depth: None,
                limits: ModelCheckLimits::default(),
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
                    temporal_clock_selections: &[],
                    subject: fsl_eval::model_subject::ModelSubject {
                        simulation,
                        proof_bounds: Vec::new(),
                    },
                    item: ModelCheckItem::DeadlockFreedom,
                    max_depth: None,
                    limits: ModelCheckLimits::default(),
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
    use filament_exact::Cancel;
    use fsl_analyze::analyze::{
        analyze_items, deadlock_model_item, derived_model_item, AnalyzeLimits, Engine, Engines,
        PackageDeadlockSubject, RunInput, TemporalItem,
    };
    use fsl_route::request::{DerivedLimits, ItemKind};
    use serde_json::json;

    use crate::model_world::{still_counter, Clause, World, INFINITE};

    let world = World::new(
        still_counter(),
        "",
        INFINITE,
        &[Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(c.value <= 3)",
        }],
        &[("c", json!({"value": {"integer": "0"}}))],
    );
    let written = crate::derived_request::written(
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
    assert!(from_source.contains("Refuted"), "{from_source}");

    let outcome = fsl_analyze::model_check::check_model(source_item().request, &Cancel::new())
        .expect("checked");
    let fsl_replay::ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("the still Counter deadlocks: {outcome:?}");
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
}
