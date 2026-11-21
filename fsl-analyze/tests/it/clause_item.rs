// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-281-AC-2 (TC-762) over a real authored clause, with supplementary EN-1
//! state-budget and refusal cases (FR-281-AC-6 and TC-763 are the EN-5
//! item's, tagged in `exact_probabilistic_item.rs`): the
//! EN-1 clause item (`ClauseSubject`) is built from the compiled unit's
//! bytes, the subject binding and the clause's occurrence, and runs through
//! `analyze_items`. No closure, letters or verdict is supplied by the test:
//! the counterexample replays through the production `replay_model_trace`.

use filament_exact::{Cancel, Origin, Role};
use filament_semantic_value::catalog::Category;
use fsl_analyze::analyze::{
    analyze_items, AnalyzeLimits, ClauseItemRefusal, ClauseLimits, ClauseSubject, Engine, Engines,
    SourceProvision,
};
use fsl_eval::simulation::Limits;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, RequestIndex};
use fsl_replay::{
    CounterexampleKind, IncompleteCause, ModelCheckLimit, ModelTraceSettlement, ModelTraceValue,
    TerminalValue,
};
use ix_trace_rs::trace;
use serde_json::json;

use crate::counter::CONTRACT;
use crate::model_world::{self, Clause, World, INFINITE};

/// The Counter world with `always holds(c.value <= 1)`: `inc` breaks it at
/// value 2.
pub(crate) fn world() -> World {
    World::new(
        model_world::counter(),
        CONTRACT,
        INFINITE,
        &[Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(c.value <= 1)",
        }],
        &[("c", json!({"value": {"integer": "0"}}))],
    )
}

pub(crate) fn subject(world: &World, limits: ClauseLimits) -> ClauseSubject<'_> {
    let node = &world.package.graph().temporal_clauses()[0];
    let key = OccurrenceKey::new(node.identity(), node.claim().clone());
    ClauseSubject::new(
        &fsl_analyze::analyze::RunInput::source(&world.package),
        world.binding(&["c"]),
        AnalyzeLimits::unbounded(),
        &world.provision,
        &key,
        limits,
    )
    .expect("the package holds the clause")
}

fn run(world: &World, limits: ClauseLimits) -> fsl_analyze::analyze::AnalyzeRecord {
    let subject = subject(world, limits);
    let mut engines = Engines::default();
    engines
        .with(Engine::model_check().expect("EN-1's own manifest is admitted"))
        .expect("one engine");
    let mut records = analyze_items(
        &engines,
        vec![subject.item(RequestIndex::new(0))],
        &Cancel::new(),
    );
    assert_eq!(records.len(), 1);
    records.remove(0)
}

/// FR-281-AC-2: the invariant a transition breaks settles `refuted`, the
/// record carries the counterexample, and the production replay reproduced
/// it before the item settled (a replay that disagreed or refused would not
/// settle `refuted`).
#[trace("TC-762", "FR-281-AC-2")]
#[test]
fn a_clause_a_transition_breaks_settles_refuted_with_a_replayed_counterexample() {
    let world = world();
    let record = run(&world, ClauseLimits::default());
    let terminal = record.terminal().expect("a terminal record");
    assert_eq!(terminal.value(), &TerminalValue::Refuted);
    assert_eq!(terminal.value().category(), Category::Violation);
    assert_eq!(terminal.backend().map(BackendId::as_str), Some("en-1"));
    let counterexample = record
        .model_check
        .as_ref()
        .and_then(|model| model.counterexample.clone())
        .expect("the record carries the counterexample");
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    let found = {
        let mut engines = Engines::default();
        engines
            .with(Engine::model_check().expect("EN-1's own manifest is admitted"))
            .expect("one engine");
        let records = analyze_items(
            &engines,
            vec![subject(&world, ClauseLimits::default()).item(RequestIndex::new(0))],
            &Cancel::new(),
        );
        fsl_analyze::analyze::split_records(records, &world.package).1
    };
    let [found] = found.as_slice() else {
        panic!("one counterexample")
    };
    let spelled = found.wire.as_ref().expect("spelled");
    assert_eq!(spelled["kind"], "formula");
    assert_eq!(spelled["over_binding"]["type"], "reference");
    assert!(spelled["interval"].is_null());
    let replay = subject(&world, ClauseLimits::default())
        .replay_counterexample(&counterexample)
        .expect("the counterexample replays");
    assert!(
        matches!(
            replay.settlement,
            ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Truth(false),
                ..
            }
        ),
        "{:?}",
        replay.settlement
    );
}

/// FR-281-AC-7 over a real authored clause: a claim whose evaluation divides
/// by zero at the one initial state settles `refuted`; its counterexample is
/// the `undefined-evaluation` envelope naming that state and the zero modulus,
/// the production replay reproduces the undefined value there, and the
/// record is no `undefined` label.
#[trace("TC-763", "FR-281-AC-7")]
#[test]
fn a_claim_that_takes_a_modulus_by_zero_settles_refuted_with_a_replayed_undefined_counterexample() {
    let world = World::with_kernel_functions(
        model_world::counter(),
        CONTRACT,
        "function q using v(x: Integer): Integer pure { 6 mod x }\n",
        INFINITE,
        &[Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(q(2 - c.value) >= 0)",
        }],
        &[("c", json!({"value": {"integer": "2"}}))],
    );
    let record = run(&world, ClauseLimits::default());
    let terminal = record.terminal().expect("a terminal record");
    assert_eq!(terminal.value(), &TerminalValue::Refuted);
    assert_eq!(terminal.value().category(), Category::Violation);
    let counterexample = record
        .model_check
        .as_ref()
        .and_then(|model| model.counterexample.clone())
        .expect("the record carries the counterexample");
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind else {
        panic!("an undefined evaluation: {:?}", counterexample.kind);
    };
    assert_eq!(evaluation.site.position, 0);
    let replay = subject(&world, ClauseLimits::default())
        .replay_counterexample(&counterexample)
        .expect("the counterexample replays");
    assert!(
        matches!(
            &replay.settlement,
            ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Undefined(reproduced),
                ..
            } if reproduced.cause == evaluation.cause && reproduced.site == evaluation.site
        ),
        "{:?}",
        replay.settlement
    );
    let (terminals, found) = {
        let subject = subject(&world, ClauseLimits::default());
        let mut engines = Engines::default();
        engines
            .with(Engine::model_check().expect("EN-1's own manifest is admitted"))
            .expect("one engine");
        let records = analyze_items(
            &engines,
            vec![subject.item(RequestIndex::new(0))],
            &Cancel::new(),
        );
        fsl_analyze::analyze::split_records(records, &world.package)
    };
    let [found_one] = found.as_slice() else {
        panic!("one counterexample")
    };
    let wire = found_one.wire.clone().expect("the envelope is spelled");
    assert_eq!(wire["kind"]["type"], "undefined-evaluation");
    assert_eq!(wire["kind"]["cause"], "division-by-zero");
    assert_eq!(wire["kind"]["where"]["position"], 0);
    assert!(wire["kind"]["where"]["state_key"]["key"].is_string());
    assert!(wire["kind"]["where"]["locus"]["node"].is_object());
    assert_eq!(
        wire["kind"]["where"]["locus"]["occurrence"]["role"],
        "claim"
    );
    assert_eq!(wire["trace_position"], 0);
    // The item the document writes keeps the cause beside the counterexample:
    // `refuted`, never `undefined`, and FR-285 maps it to exit 10.
    let item = fsl_analyze::analyze::analyze_item(&terminals[0], &found).expect("an item");
    assert_eq!(item.category(), Category::Violation);
    assert_eq!(item.category().exit_code(), 10);
    let written = serde_json::to_value(&item).expect("an item is JSON");
    assert_eq!(written["result"], "refuted");
    assert_eq!(written["cause"]["kind"], "undefined-evaluation");
    assert_eq!(written["cause"]["code"], "division-by-zero");
    assert!(
        written["cause"]["where"]
            .as_str()
            .is_some_and(|at| at.starts_with("position 0, state ")),
        "{written}"
    );
    assert_eq!(written["counterexample"], wire);
}

/// FR-281 and FR-286 on the real path: a refuted record whose counterexample
/// was not kept is no item (`MissingCounterexample`), and one whose
/// counterexample has no wire spelling settles `failed`, so one such item
/// leaves the rest of the document intact.
#[trace("TC-770", "FR-286-AC-14")]
#[test]
fn an_unspelled_counterexample_fails_its_item_and_not_the_document() {
    let world = World::with_kernel_functions(
        model_world::counter(),
        CONTRACT,
        "function q using v(x: Integer): Integer pure { 6 mod x }\n",
        INFINITE,
        &[Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(q(2 - c.value) >= 0)",
        }],
        &[("c", json!({"value": {"integer": "2"}}))],
    );
    let item = |world: &World| {
        let mut engines = Engines::default();
        engines
            .with(Engine::model_check().expect("EN-1's own manifest is admitted"))
            .expect("one engine");
        analyze_items(
            &engines,
            vec![subject(world, ClauseLimits::default()).item(RequestIndex::new(0))],
            &Cancel::new(),
        )
    };
    // The real record, its counterexample not handed on.
    let records = item(&world);
    let terminals = fsl_analyze::analyze::terminal_records(records);
    assert_eq!(terminals[0].value(), &TerminalValue::Refuted);
    assert_eq!(
        fsl_analyze::analyze::analyze_item(&terminals[0], &[]).unwrap_err(),
        fsl_replay::OutcomeItemError::MissingCounterexample
    );
    assert_eq!(
        fsl_analyze::analyze::analyze_document(&terminals, &[]).unwrap_err(),
        fsl_replay::OutcomeItemError::MissingCounterexample
    );
    // The same record with no claim to locate its undefined evaluation.
    let mut records = item(&world);
    records[0].claim = None;
    let (terminals, found) = fsl_analyze::analyze::split_records(records, &world.package);
    assert!(found.is_empty());
    assert_eq!(terminals[0].value(), &TerminalValue::Failed);
    let document = fsl_analyze::analyze::analyze_document(&terminals, &found)
        .expect("a failed item is an item");
    let written = serde_json::to_value(&document).expect("a document is JSON");
    assert_eq!(written["items"][0]["result"], "failed");
    assert!(written["items"][0].get("counterexample").is_none());
}

/// Supplementary EN-1 case (FR-281-AC-6's budget row is the EN-5 item's): the
/// same clause under `max_states` 1 settles `incomplete` with the
/// state-budget cause, not a verdict.
#[test]
fn the_same_clause_under_one_state_settles_incomplete_naming_the_state_budget() {
    let record = run(
        &world(),
        ClauseLimits {
            limits: Limits {
                max_states: 1,
                ..Limits::default()
            },
            ..ClauseLimits::default()
        },
    );
    let terminal = record.terminal().expect("a terminal record");
    assert_eq!(
        terminal.value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 1,
        })
    );
}

fn refused(world: &World, key: &OccurrenceKey) -> ClauseItemRefusal {
    ClauseSubject::new(
        &fsl_analyze::analyze::RunInput::source(&world.package),
        world.binding(&["c"]),
        AnalyzeLimits::unbounded(),
        &world.provision,
        key,
        ClauseLimits::default(),
    )
    .err()
    .expect("the subject is refused")
}

/// A key naming the clause's node under a claim occurrence it does not have
/// is no clause of the package.
#[test]
fn a_key_with_another_claim_occurrence_is_an_unknown_clause() {
    let world = world();
    let node = &world.package.graph().temporal_clauses()[0];
    let key = OccurrenceKey::new(node.identity(), Origin::new(Role::new("claim"), 99));
    assert_eq!(refused(&world, &key), ClauseItemRefusal::UnknownClause);
}

/// A complete source provision preserves real replay for a package assembled
/// from two source units; no single-unit restriction discards the extra unit.
#[test]
fn a_package_of_two_source_units_replays_from_its_complete_provision() {
    use fsl_replay::spine::{
        admit_parsed_unit_list, check_units, parse, select_units, DependencyInput, ParseRequest,
        SpineLimits,
    };
    use fsl_semantics::model::accounting::ModelNormalizationLimits;
    use fsl_semantics::model::intake::package_input;

    let world = world();
    let text = String::from_utf8(world.unit.clone()).expect("a text unit");
    let extra = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
                 profile v = \"filament.value.complete/v1\";\n\
                 profile t = \"{INFINITE}\";\n\
                 function Extra using v(): Boolean pure {{ true }}\n"
    );
    let units: Vec<_> = [("safety", text.as_str()), ("extra", extra.as_str())]
        .into_iter()
        .map(|(identity, source)| {
            let identity = fsl_foundation::SourceIdentity::new("agent-ix", identity);
            parse(
                &ParseRequest {
                    source: &identity,
                    path: "unit.native",
                    bytes: source.as_bytes(),
                },
                fsl_cst::Limits::default(),
                &Cancel::new(),
            )
            .expect("S1 and S2 parse the unit")
            .into_value()
        })
        .collect();
    let dependencies = DependencyInput::default();
    let units =
        admit_parsed_unit_list(units, &dependencies).expect("both headers and source labels admit");
    let packages = package_input([world.domain.document.as_slice()]).expect("admits");
    let selected = select_units(
        &units,
        &packages,
        ModelNormalizationLimits::default(),
        fsl_cst::Limits::default(),
        &Cancel::new(),
    )
    .expect("the model selection admits")
    .into_value();
    let checked = check_units(
        &units,
        "package".to_owned(),
        &selected,
        &dependencies,
        SpineLimits::default(),
        &Cancel::new(),
    )
    .expect("the qualified frontend checks both source units")
    .into_value();
    let package = checked.first().expect("the checked aggregate").package();
    assert_eq!(package.graph().sources().len(), 2);
    let node = &package.graph().temporal_clauses()[0];
    let key = OccurrenceKey::new(node.identity(), node.claim().clone());
    let mut provision = SourceProvision::default();
    for source in package.graph().sources() {
        provision.insert(
            source,
            match source.identity() {
                "safety" => text.as_bytes().to_vec(),
                "extra" => extra.as_bytes().to_vec(),
                other => panic!("unexpected source {other}"),
            },
        );
    }
    let subject = ClauseSubject::new(
        &fsl_analyze::analyze::RunInput::source(package),
        world.binding(&["c"]),
        AnalyzeLimits::unbounded(),
        &provision,
        &key,
        ClauseLimits::default(),
    )
    .expect("the complete multi-unit subject is accepted");
    let mut raw_subject = world.subject(&["c"], crate::counter::UNLIMITED);
    raw_subject.simulation.input = fsl_eval::simulation::model::ModelCheckInput::source(package);
    let system = fsl_eval::simulation::model::ModelSystem::new(raw_subject.simulation.clone())
        .expect("the multi-unit system admits");
    let letters = system
        .clause_letters(node)
        .expect("the multi-unit clause letters");
    let raw = fsl_analyze::model_check::check_model(
        fsl_analyze::model_check::ModelCheckRequest {
            subject: raw_subject,
            temporal_clock_selections: &[],
            item: fsl_replay::ModelCheckItem::Clause {
                clause: node.clause(),
                letters: &letters,
            },
            max_depth: None,
            limits: fsl_replay::ModelCheckLimits::default(),
            reductions: Default::default(),
        },
        &Cancel::new(),
    )
    .expect("the model checker produces a real candidate");
    let fsl_replay::ModelCheckOutcome::Violated { counterexample, .. } = raw else {
        panic!("the same counterexample exists")
    };
    subject
        .replay_counterexample(&counterexample)
        .expect("the operation-built multi-unit request replays the native candidate");
    let records = analyze_items(
        &Engines::fsl().expect("the engines register"),
        vec![subject.item(RequestIndex::new(0))],
        &Cancel::new(),
    );
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].terminal().expect("terminal record").value(),
        &TerminalValue::Refuted
    );
    let counterexample = records[0]
        .model_check
        .as_ref()
        .and_then(|model| model.counterexample.as_ref())
        .expect("a real counterexample");
    let replayed = subject
        .replay_counterexample(counterexample)
        .expect("the supplied sources replay");
    assert!(matches!(
        replayed.settlement,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            ..
        }
    ));
}

/// A counterexample outside the subject, or replayed over other source bytes
/// than the package was checked from, is refused by the production replay:
/// no `Reproduced` result exists for `settle_model_check` to settle `refuted`
/// from. This drives `replay_counterexample`, not the item's own closure
/// (the item settles through the same replay). `UnnamedClause` has no test:
/// `Identifier` is ASCII, and whether S1 admits a non-ASCII clause name is
/// not checked here.
#[test]
fn a_tampered_counterexample_or_another_unit_is_refused_by_the_replay() {
    let world = world();
    let record = run(&world, ClauseLimits::default());
    let counterexample = record
        .model_check
        .as_ref()
        .and_then(|model| model.counterexample.clone())
        .expect("the record carries the counterexample");

    let mut outside = counterexample.clone();
    outside.initial_state = 1;
    assert!(subject(&world, ClauseLimits::default())
        .replay_counterexample(&outside)
        .is_err());

    let node = &world.package.graph().temporal_clauses()[0];
    let key = OccurrenceKey::new(node.identity(), node.claim().clone());
    let other_unit = b"language \"ix:native\" edition \"1-draft\";\n";
    let other_provision = SourceProvision::single(world.package.graph().source(), other_unit);
    let wrong = ClauseSubject::new(
        &fsl_analyze::analyze::RunInput::source(&world.package),
        world.binding(&["c"]),
        AnalyzeLimits::unbounded(),
        &other_provision,
        &key,
        ClauseLimits::default(),
    )
    .expect("construction does not compare the bytes");
    assert!(wrong.replay_counterexample(&counterexample).is_err());
}

/// FR-850-AC-2, FR-940-AC-5 (QSL-2406 bytes-only replay): over the package
/// a Counter unit with domain operations is written to, the package route's
/// clause item refutes the clause with the source route's record, and its
/// counterexample replays from the package bytes and the handoff's identity
/// and equality files alone: the replay request carries no source entry.
#[trace("TC-2410", "FR-850-AC-2")]
#[trace("TC-2958", "FR-940-AC-5")]
#[test]
fn a_package_route_refutation_replays_from_the_handoff_as_the_source_refutation() {
    use fsl_package::{BarePackage, BareSupply, V2ReadLimits};
    let world = World::new(
        model_world::counter(),
        "",
        INFINITE,
        &[Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(c.value <= 1)",
        }],
        &[("c", json!({"value": {"integer": "0"}}))],
    );
    let source = run(&world, ClauseLimits::default());
    assert_eq!(
        source.terminal().expect("a terminal record").value(),
        &TerminalValue::Refuted
    );
    let binding = world.binding(&["c"]);
    let emission = fsl_package::emit_checked(&world.package, fsl_package::EmitLimits::fixture())
        .expect("the package emits");
    let bytes = emission.package().bytes().to_vec();
    let admitted =
        fsl_package::admit_checked_package(&bytes, binding.packages, V2ReadLimits::fixture())
            .expect("the strict reader admits the bytes");
    let mut budget = V2ReadLimits::fixture().native_join_budget();
    let identities = emission
        .identity_rows(&admitted, &mut budget)
        .expect("the identity rows");
    let equality = emission
        .equality_witness(&admitted, &mut budget)
        .expect("the equality rows");
    let identity_bytes = identities.to_bytes(&admitted).expect("identities.json");
    let equality_bytes = equality.to_bytes(&admitted).expect("equality.json");
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut budget,
        )
        .expect("the supply joins");
    let input = fsl_eval::simulation::model::PackageModelInput::of(
        &bare,
        binding.packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the package route reads every form");
    let clause = &world.package.graph().temporal_clauses()[0];
    let key = fsl_foundation::source::provenance::OccurrenceKey::new(
        clause.identity(),
        clause.claim().clone(),
    );
    let subject = fsl_analyze::analyze::PackageClauseSubject::new(
        &input,
        &bytes,
        fsl_replay::HandoffSupply {
            dependencies: &[],
            identities: &identity_bytes,
            equality: Some(&equality_bytes),
        },
        world.binding(&["c"]),
        AnalyzeLimits::unbounded(),
        &key,
        ClauseLimits::default(),
    )
    .expect("the package holds the clause");
    let mut engines = Engines::default();
    engines
        .with(Engine::model_check().expect("EN-1's own manifest is admitted"))
        .expect("one engine");
    let mut records = analyze_items(
        &engines,
        vec![subject.item(RequestIndex::new(0))],
        &Cancel::new(),
    );
    let package = records.remove(0);
    assert_eq!(format!("{package:?}"), format!("{source:?}"));
}

/// Run `check` over the package route's model-check input of `world`'s
/// emitted package, its bytes and its handoff supply.
fn over_package_route(
    world: &World,
    check: impl FnOnce(
        &fsl_eval::simulation::model::PackageModelInput<'_>,
        &[u8],
        fsl_replay::HandoffSupply<'_>,
    ),
) {
    use fsl_package::{BarePackage, BareSupply, V2ReadLimits};
    let binding = world.binding(&["c"]);
    let emission = fsl_package::emit_checked(&world.package, fsl_package::EmitLimits::fixture())
        .expect("the package emits");
    let bytes = emission.package().bytes().to_vec();
    let admitted =
        fsl_package::admit_checked_package(&bytes, binding.packages, V2ReadLimits::fixture())
            .expect("the strict reader admits the bytes");
    let mut budget = V2ReadLimits::fixture().native_join_budget();
    let identities = emission
        .identity_rows(&admitted, &mut budget)
        .expect("the identity rows");
    let equality = emission
        .equality_witness(&admitted, &mut budget)
        .expect("the equality rows");
    let identity_bytes = identities.to_bytes(&admitted).expect("identities.json");
    let equality_bytes = equality.to_bytes(&admitted).expect("equality.json");
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut budget,
        )
        .expect("the supply joins");
    let input = fsl_eval::simulation::model::PackageModelInput::of(
        &bare,
        binding.packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the package route reads every form");
    check(
        &input,
        &bytes,
        fsl_replay::HandoffSupply {
            dependencies: &[],
            identities: &identity_bytes,
            equality: Some(&equality_bytes),
        },
    );
}

/// QSL-2432 (FR-940 "Package forms of the source-checked evaluators"): a
/// clause whose package form reads a fact the package route does not read
/// yet is refused by the package route's clause subject with that fact and
/// the clause's node, never as an unknown clause: a fairness set, and a
/// capture, which the package route's letters do not evaluate yet, so no
/// engine reaches a capture fault.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_pending_package_clause_fact_is_refused_typed_naming_it() {
    for (formula, expected) in [
        (
            "fair weak M::Counter::inc; eventually holds(c.value = 3)",
            "fairness",
        ),
        (
            "capture n: Int[0, 3] = c.value; always holds(c.value >= n)",
            "capture",
        ),
    ] {
        let world = World::new(
            model_world::counter(),
            "",
            INFINITE,
            &[Clause {
                activation: "on each (q: M::Counter) when (true)",
                formula,
            }],
            &[("c", json!({"value": {"integer": "0"}}))],
        );
        let clause = &world.package.graph().temporal_clauses()[0];
        let key = fsl_foundation::source::provenance::OccurrenceKey::new(
            clause.identity(),
            clause.claim().clone(),
        );
        over_package_route(&world, |input, bytes, supply| {
            let refused = fsl_analyze::analyze::PackageClauseSubject::new(
                input,
                bytes,
                supply,
                world.binding(&["c"]),
                AnalyzeLimits::unbounded(),
                &key,
                ClauseLimits::default(),
            )
            .err()
            .expect("the pending clause is refused");
            let fsl_analyze::analyze::ClauseItemRefusal::Pending { fact, .. } = refused else {
                panic!("{formula}: a typed pending refusal, not {refused:?}");
            };
            assert_eq!(fact, expected, "{formula}");
        });
    }
}

/// FR-940-AC-22 (QSL-2698): a failed emission of an admitted package at the
/// replay settles the replaying item `failed`, the internal fault
/// `analyze`/`package-emission`, never a replay refusal. The run's
/// `emit.output_bytes` is the test-only emitter: the checker admitted the
/// package, and the replay's own emission is held below its length. The
/// record shows an internal fault as `runtime_invariant`; the cause name
/// `package-emission` stays on the `ReplayRefusal::Fault` and is not on the record.
#[trace("TC-2958", "FR-940-AC-22")]
#[test]
fn a_failed_replay_emission_settles_failed_with_the_package_emission_fault() {
    let world = world();
    let node = &world.package.graph().temporal_clauses()[0];
    let key = OccurrenceKey::new(node.identity(), node.claim().clone());
    let limits = AnalyzeLimits {
        emit: fsl_package::EmitLimits::new(1),
        ..AnalyzeLimits::unbounded()
    };
    let subject = ClauseSubject::new(
        &fsl_analyze::analyze::RunInput::source(&world.package),
        world.binding(&["c"]),
        limits,
        &world.provision,
        &key,
        ClauseLimits::default(),
    )
    .expect("the package holds the clause");
    let mut engines = Engines::default();
    engines
        .with(Engine::model_check().expect("EN-1's own manifest is admitted"))
        .expect("one engine");
    let records = analyze_items(
        &engines,
        vec![subject.item(RequestIndex::new(0))],
        &Cancel::new(),
    );
    let terminal = records[0].terminal().expect("a terminal record");
    assert_eq!(terminal.value(), &TerminalValue::Failed, "{terminal:?}");
    assert_eq!(
        terminal
            .diagnostic()
            .map(|code| (code.code(), code.cause())),
        Some(("runtime_invariant", "established-invariant-broken")),
        "{terminal:?}"
    );
}

/// FR-621-AC-17: a run whose package has no admitted emission (the emit limit
/// is reached) cannot write the node ids of an item it settled in the held
/// form; the run reports the typed cause, never a result with a nested node
/// id and never an untyped fault.
#[trace("TC-3154", "FR-621-AC-17")]
#[test]
fn a_package_over_the_emit_limit_reports_the_typed_cause_not_a_nested_id() {
    use fsl_analyze::analyze::document::{
        settle_run, AnalyzeDocumentError, AnalyzeRun, HeldIdentityError, ResolvedClaim,
        ResolvedItem, SubjectSelection,
    };
    use fsl_analyze::analyze::{ClaimItem, GraphUnavailable, ItemOptions};
    use fsl_route::request::ExtentClassification;
    use fsl_semantics::check::Capability;

    let world = world();
    let node = &world.package.graph().temporal_clauses()[0];
    let claim = OccurrenceKey::new(node.identity(), node.claim().clone());
    let engines = Engines::fsl().expect("the manifests register");
    let mut registry = fsl_route::Registry::new();
    for descriptor in engines.registrations() {
        registry
            .register(descriptor.clone())
            .expect("a unique backend");
    }
    let candidates = registry.candidates_for_evidence(
        Capability::TemporalSatisfaction,
        &fsl_route::ItemEvidence::Absent,
        None,
    );
    let fsl_route::CandidateOutcome::Candidates(set) = &candidates else {
        panic!("temporal satisfaction has a candidate");
    };
    let binding = world.binding(&["c"]);
    let subject = SubjectSelection::default_over(binding.snapshots);
    let run = |limits: fsl_analyze::analyze::AnalyzeLimits| {
        settle_run(
            AnalyzeRun {
                package: &world.package,
                binding: world.binding(&["c"]),
                provision: world.provision.clone(),
                items: vec![ResolvedItem::Claim(Box::new(ResolvedClaim {
                    item: ClaimItem {
                        request_index: RequestIndex::new(0),
                        claim: claim.clone(),
                        kind: Capability::TemporalSatisfaction,
                        extent: ExtentClassification::Bounded,
                        backend: None,
                        candidates: set.candidates().to_vec(),
                        options: ItemOptions {
                            required: true,
                            ..ItemOptions::default()
                        },
                        form: None,
                    },
                    claim_name: "Bounded".to_owned(),
                    candidates: candidates.clone(),
                    subject: subject.clone(),
                    unrouted: false,
                }))],
                engines: &engines,
                limits,
            },
            &crate::analyze_cli_support::Fr290,
            &Cancel::new(),
        )
    };
    let settled = run(fsl_analyze::analyze::AnalyzeLimits::unbounded()).expect("a settled run");
    crate::analyze_cli_support::assert_no_nested_node_ids(&settled.document);
    let limited = fsl_analyze::analyze::AnalyzeLimits {
        emit: fsl_package::EmitLimits::new(1),
        ..fsl_analyze::analyze::AnalyzeLimits::unbounded()
    };
    match run(limited) {
        Err(AnalyzeDocumentError::Identity(HeldIdentityError::NoGraph(
            GraphUnavailable::EmitRefused(refusal),
        ))) => {
            // The emitter's own typed code and the limit kind's cause, never
            // the `insufficient-next-charge` default.
            assert_eq!(refusal.code(), fsl_foundation::Code::StageLimitExceeded);
            assert!(
                refusal
                    .cause()
                    .is_some_and(|cause| cause.ends_with("-exceeded")),
                "{refusal:?}"
            );
        }
        Err(AnalyzeDocumentError::Refused(_)) => {}
        other => panic!(
            "the typed cause is reported: {:?}",
            other.map(|run| run.document)
        ),
    }
}

/// FR-128-AC-15: over the Counter world, the closure lasso at the initial state
/// (empty stem) replays to `reproduced-with-evaluated-witness` from the source
/// route and from the package route alike, for an item whose
/// `temporal.stutter_closure` is true, and both refuse the closure step at
/// `/loop/0/transition/type` for the same item without the flag. The package
/// route replays the lasso; a refusal there would not satisfy the criterion.
#[trace("TC-523", "FR-128-AC-15")]
#[test]
fn a_closure_lasso_replays_alike_on_the_source_and_the_package_routes() {
    use fsl_analyze::analyze::{PackageClauseSubject, TemporalItem};
    use fsl_eval::simulation::TransitionSystem;
    use fsl_replay::{ClosureStutter, Lasso, ModelTraceRefusal, ReplayRefusal, StepSite};

    let world = World::new(
        model_world::counter(),
        CONTRACT,
        INFINITE,
        &[Clause {
            activation: "on origin",
            formula: "always eventually holds(c.value = 3)",
        }],
        &[("c", json!({"value": {"integer": "0"}}))],
    );
    let node = &world.package.graph().temporal_clauses()[0];
    let key = OccurrenceKey::new(node.identity(), node.claim().clone());
    let temporal = |selected: bool| TemporalItem {
        stutter_closure: Some(selected),
        fairness: node.clause().fairness().to_vec(),
        ..TemporalItem::default()
    };

    // The key of the initial state, the loop's entry.
    let binding = world.binding(&["c"]);
    let system = fsl_eval::simulation::model::ModelSystem::new(
        fsl_eval::simulation::model::ModelSimulationRequest {
            input: fsl_eval::simulation::model::ModelCheckInput::source(&world.package),
            packages: binding.packages,
            normalization: fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
            snapshots: binding.snapshots,
            initial: binding.initial.clone(),
            universes: binding.universes.clone(),
            observation: fsl_semantics::model::observation::ObservationLimits::default(),
            expansion: fsl_eval::simulation::model::ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: fsl_analyze::analyze::AnalyzeLimits::unbounded().accounting,
            emit: fsl_package::EmitLimits::fixture(),
        },
    )
    .expect("the subject admits");
    let initial = system.initial()[0].clone();
    let entry = fsl_eval::simulation::StateKey::from_canonical_bytes(
        system.state_key(&initial).expect("the state encodes").0,
    );
    let payload = fsl_replay::TemporalCounterexample {
        initial_state: 0,
        prefix: Vec::new(),
        over_binding: Some(fsl_eval::model_subject::ModelInstance {
            population: world.domain.key(world.domain.population),
            key: "c".to_owned(),
        }),
        fairness: node.clause().fairness().to_vec(),
        kind: CounterexampleKind::Formula,
        lasso: Some(Lasso {
            entry: 0,
            terminal_stutter: false,
            terminal_stutter_delay: None,
        }),
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
        closure_stutters: vec![ClosureStutter {
            site: StepSite::Loop(0),
            post_state: entry,
            delay: None,
        }],
    };

    let source = |selected: bool| {
        subject(&world, ClauseLimits::default())
            .configured(temporal(selected))
            .replay_counterexample(&payload)
    };
    let package = |selected: bool| {
        let mut replayed = None;
        over_package_route(&world, |input, bytes, supply| {
            replayed = Some(
                PackageClauseSubject::new(
                    input,
                    bytes,
                    supply,
                    world.binding(&["c"]),
                    AnalyzeLimits::unbounded(),
                    &key,
                    ClauseLimits::default(),
                )
                .expect("the package holds the clause")
                .configured(temporal(selected))
                .replay_counterexample(&payload),
            );
        });
        replayed.expect("the package route ran")
    };

    let reproduced = ModelTraceSettlement::Reproduced {
        value: ModelTraceValue::Truth(false),
        trace_position: Some(0),
    };
    let from_source = source(true).expect("the source route replays the closure lasso");
    let from_package = package(true).expect("the package route replays the closure lasso");
    assert_eq!(from_source.settlement, reproduced);
    assert_eq!(from_source, from_package);

    for refused in [source(false), package(false)] {
        let Err(ReplayRefusal::ModelTrace(refusal)) = refused else {
            panic!("an item that does not select the flag refuses the step: {refused:?}");
        };
        assert!(
            matches!(
                refusal.as_ref(),
                ModelTraceRefusal::ClosureStutter { pointer, .. }
                    if pointer == "/loop/0/transition/type"
            ),
            "{refusal:?}"
        );
    }
}

/// FR-128-AC-14, AC-15 with FR-1030-AC-6's mechanism: over the Counter world
/// no model step repeats, so the only loops that violate `always eventually
/// holds(c.value = 3)` are closure stutters. For an item that selects
/// `stutter_closure` both routes settle it `refuted`, the engine's lasso is the
/// closure-only form, it is written with the closure tag, read back by the
/// auditor's reader, and replays to a reproduction before the item settles.
#[trace("TC-523", "FR-128-AC-14", "FR-128-AC-15")]
#[test]
fn a_closure_violation_settles_refuted_through_its_written_and_replayed_lasso() {
    use fsl_analyze::analyze::{PackageClauseSubject, TemporalItem};

    let world = World::new(
        model_world::counter(),
        CONTRACT,
        INFINITE,
        &[Clause {
            activation: "on origin",
            formula: "always eventually holds(c.value = 3)",
        }],
        &[("c", json!({"value": {"integer": "0"}}))],
    );
    let node = &world.package.graph().temporal_clauses()[0];
    let key = OccurrenceKey::new(node.identity(), node.claim().clone());
    let temporal = || TemporalItem {
        stutter_closure: Some(true),
        fairness: node.clause().fairness().to_vec(),
        ..TemporalItem::default()
    };
    fn settle(item: fsl_analyze::analyze::AnalyzeItem<'_>) -> fsl_analyze::analyze::AnalyzeRecord {
        let mut engines = Engines::default();
        engines
            .with(Engine::model_check().expect("EN-1's own manifest is admitted"))
            .expect("one engine");
        analyze_items(&engines, vec![item], &Cancel::new()).remove(0)
    }

    let source_subject = subject(&world, ClauseLimits::default()).configured(temporal());
    let source = settle(source_subject.item(RequestIndex::new(0)));
    assert_eq!(
        source.terminal().expect("a terminal record").value(),
        &TerminalValue::Refuted,
        "{source:?}"
    );
    let found = source
        .model_check
        .as_ref()
        .and_then(|model| model.counterexample.as_ref())
        .expect("the record carries the counterexample");
    assert!(found.closure_loop().is_some(), "{found:?}");

    let mut unbound = found.clone();
    unbound.over_binding = None;
    let wire = fsl_replay::temporal_wire(&unbound, None, None).expect("the lasso is spelled");
    assert_eq!(wire["loop"][0]["transition"]["type"], "closure-stutter");
    assert_eq!(wire["loop"].as_array().map(Vec::len), Some(1));
    let read = fsl_replay::read_temporal_counterexample::<
        filament_semantic_value::model_transition::ModelTransition,
    >(&wire)
    .expect("the auditor's reader reads the written lasso");
    assert_eq!(read.prefix, found.prefix);
    assert_eq!(read.lasso, found.lasso);
    assert_eq!(read.closure_stutters, found.closure_stutters);
    let replayed = source_subject
        .replay_counterexample(found)
        .expect("the written lasso replays");
    assert!(matches!(
        replayed.settlement,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            ..
        }
    ));

    over_package_route(&world, |input, bytes, supply| {
        let package = PackageClauseSubject::new(
            input,
            bytes,
            supply,
            world.binding(&["c"]),
            AnalyzeLimits::unbounded(),
            &key,
            ClauseLimits::default(),
        )
        .expect("the package holds the clause")
        .configured(temporal());
        // The source engine's lasso replays over the package bytes before the
        // package item settles, so a mismatch names its stage.
        let replayed = package
            .replay_counterexample(found)
            .expect("the engine's lasso replays over the package route");
        assert!(matches!(
            replayed.settlement,
            ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Truth(false),
                ..
            }
        ));
        let record = settle(package.item(RequestIndex::new(0)));
        // A package item that fails after its engine refuted names where: the
        // engine's value and the `over` reference its wire is spelled with.
        assert_eq!(
            record.terminal().expect("a terminal record").value(),
            &TerminalValue::Refuted,
            "engine {:?}, over {:?}",
            record.model_check.as_ref().map(|model| &model.value),
            record.over_reference
        );
        assert_eq!(format!("{record:?}"), format!("{source:?}"));
    });
}
