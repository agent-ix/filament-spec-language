// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-3219: a hyper clause's instances are the admitted keys of every
//! population that overlaps its parameter's type (FR-125 "Instancing"), and an
//! item over several instances reports the first refuted one (FR-621 "Items
//! over several claim instances"). Every expected value is read from those
//! requirements and worked by hand over the fixtures here.

use std::collections::BTreeSet;

use filament_exact::{Cancel, ValueType};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_analyze::model_check::{
    check_hyper, check_single_existential, HyperCheckRequest, ModelCheckRefusal,
    SingleExistentialRequest, SingleExistentialRun,
};
use fsl_eval::hyper::{CheckedHyperLetters, CheckedStepMatch, ParameterBinding};
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::TransitionSystem;
use fsl_foundation::diagnostic::Code;
use fsl_package::CheckedPackage;
use fsl_replay::{
    replay_hyper_trace_tuple, settle_existential, DisagreementCause, ExistentialOutcome,
    HyperCounterexampleKind, HyperOutcome, HyperReplay, HyperStep, InconclusiveCause,
    ModelCheckLimits, ModelStep, ProductClosureClause, ProductClosureRefusal,
    ProductClosureRequest, ResultLabel, StateGraphMethod, TerminalValue, UndecidedCause,
};
use fsl_route::reductions::Reductions;
use fsl_semantics::model::observation::AdmissionFailure;
use ix_trace_rs::trace;
use serde_json::{json, Value as Json};

use crate::analyze_cli_support::{
    analyze, counter_snapshot, label, package_selection, request_over, Run, Workspace,
};
use crate::counter::{declarations_in, unit_text_in, Documents, UNLIMITED};
use crate::model_instances::{ident, the_admitted_lamp, universes, world, Held, World, PACKAGE};

/// Claim bodies true of every admitted instance of `v: M::Thing`.
const GUARD: &str = "present(lookup<M::Thing>(M::pets, v) absent empty)";
const READ: &str = "(v = v)";

const EVENT_POSITION: &str = "filament.temporal.event-position.false-extension/v1";

/// Every state is intended to be terminal: the fixtures have no operation, so
/// without this the subject's derived deadlock-freedom item would refute and
/// set the request's exit.
const TERMINAL: &str = "terminal Done using v on M when { true };\n";

/// The binding `reference` (`UniverseId`, `object_type`, `key`) of a
/// parameter of type `Thing` (FR-125 "Instancing").
fn thing_reference(key: &str, object_type: &str) -> Json {
    let mut written = the_admitted_lamp();
    written["identity"] = json!(key);
    written["object_type"]["original"]["node"] = json!(ident(object_type));
    written
}

/// The evidence instance of `v` bound to `reference`.
fn bound_v(reference: Json) -> Json {
    json!({"type": "instance", "parameter": "v", "key": reference})
}

/// The keys a hyper counterexample's `instance` binds.
fn counterexample_keys(item: &Json) -> Vec<(String, String)> {
    item["counterexample"]["instance"]
        .as_array()
        .unwrap_or_else(|| panic!("a hyper counterexample instance: {item}"))
        .iter()
        .map(|bound| {
            (
                bound["parameter"].as_str().expect("a parameter").to_owned(),
                bound["key"]["value"].as_str().expect("a key").to_owned(),
            )
        })
        .collect()
}

/// The pairs `bindings` as [`counterexample_keys`] returns them.
fn keys(bindings: &[(&str, &str)]) -> Vec<(String, String)> {
    bindings
        .iter()
        .map(|(parameter, key)| ((*parameter).to_owned(), (*key).to_owned()))
        .collect()
}

/// A `native-analyze/1` request over a `test/things` world: the initial state
/// holds `held`, the universes list `listed`.
struct Pets<'a> {
    populations: &'a [(&'a str, &'a [&'a str])],
    held: &'a [Held<'a>],
    listed: &'a [Held<'a>],
}

impl Pets<'_> {
    /// `pets` declares `Thing`, universe `[l, m]`; the one initial state
    /// holds a `Lamp` under `l`.
    const LAMP: Pets<'static> = Pets {
        populations: &[("pets", &["Thing"])],
        held: &[("pets", &[("l", "Lamp")])],
        listed: &[("pets", &[("l", "Lamp"), ("m", "Lamp")])],
    };

    /// `P1`, universe `[d, u]`, and `P2`, universe `[u, a, d]`; the initial
    /// state holds a `Lamp` under `d` in `P1` and a `Switch` under `a` in
    /// `P2`. Both populations declare `Thing`, so that `lookup<Thing>` over
    /// each is covered (FR-084).
    const TWO: Pets<'static> = Pets {
        populations: &[("P1", &["Thing"]), ("P2", &["Thing"])],
        held: &[("P1", &[("d", "Lamp")]), ("P2", &[("a", "Switch")])],
        listed: &[
            ("P1", &[("d", "Lamp"), ("u", "Lamp")]),
            ("P2", &[("u", "Switch"), ("a", "Switch"), ("d", "Switch")]),
        ],
    };

    fn initial(&self) -> World {
        world(self.populations, self.held)
    }

    /// The run of the item of the claim `H` that `declaration` declares,
    /// through the source route or, with `packaged`, the package selection.
    fn analyze(&self, name: &str, declaration: &str, item: Json, packaged: bool) -> Run {
        let initial = self.initial();
        let listed = world(self.populations, self.listed);
        let universe = String::from_utf8(
            listed
                .snapshots
                .values()
                .next()
                .expect("one snapshot")
                .clone(),
        )
        .expect("snapshot text")
        .replace("\"identity\":\"s0\"", "\"identity\":\"u0\"")
        .into_bytes();
        let held = initial.snapshots.values().next().expect("one snapshot");
        let unit = unit_text_in(PACKAGE, &format!("{TERMINAL}{declaration}")).replace(
            "profile t0 = \"filament.temporal.infinite-trace/v1\";",
            &format!(
                "profile t0 = \"filament.temporal.infinite-trace/v1\";\n\
                 profile ep = \"{EVENT_POSITION}\";"
            ),
        );
        let workspace = Workspace::new(name);
        let path = request_over(
            &workspace,
            &unit,
            &initial.document,
            &[held.clone(), universe],
            json!({
                "items": [item],
                "subject": {
                    "initial_states": [{"authority": "test", "identity": "s0"}],
                    "universes": [{"authority": "test", "identity": "u0"}],
                    "proof_bounds": []
                }
            }),
        );
        if !packaged {
            return analyze(&path);
        }
        let document: Json = serde_json::from_slice(&std::fs::read(&path).expect("the request"))
            .expect("request JSON");
        let (selected, _) = package_selection(&workspace, &document["request"]);
        let selected = json!({"format": "native-analyze/1", "request": selected});
        analyze(&workspace.write("analyze-package.json", selected.to_string().as_bytes()))
    }
}

/// The single-existential claim `H` of `v: M::<object>` over `body`.
fn exists(object: &str, body: &str) -> String {
    format!(
        "hyper H using t0 over behaviours (v: M::{object}) \
         exists trace a of M {{ {body} }}\n"
    )
}

/// The two-universal claim `H` of `v: M::<object>` over `body`.
fn universal(object: &str, body: &str) -> String {
    format!(
        "hyper H using t0 over behaviours (v: M::{object}) \
         forall trace a of M forall trace b of M {{ {body} }}\n"
    )
}

fn always(body: &str) -> String {
    format!("always holds({body} @ a)")
}

fn claim() -> Json {
    json!({"claim": "H"})
}

/// FR-181-AC-8: the engine lists the one admitted key `l`, settlement proves
/// it and refuses an outcome at `m`, and `m` is the excluded key.
#[trace("TC-3219", "FR-181-AC-8", "FR-125-AC-19")]
#[test]
fn the_single_existential_instances_are_the_admitted_keys() {
    let held = Pets::LAMP.initial();
    for body in [GUARD, READ] {
        let declaration = exists("Thing", &always(body));
        let graph = declarations_in(PACKAGE, &held.document, &declaration)
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .unwrap_or_else(|refusals| panic!("{declaration}: {refusals:?}"));
        let package = CheckedPackage::link(graph);
        let [clause] = package.graph().hyper_clauses() else {
            panic!("one hyper clause");
        };
        let both = || universes(&[("pets", &["l", "m"])]);
        let system = ModelSystem::new(held.subject(&package, both()).simulation).expect("admitted");
        let letters = CheckedHyperLetters::new(&package, clause, vec![&system]).expect("letters");
        let run = check_single_existential(
            SingleExistentialRequest {
                clause,
                subject: held.subject(&package, both()),
                letters: &letters,
                limits: ModelCheckLimits::default(),
                max_depth: None,
                seed: None,
                meter: UNLIMITED,
            },
            &Cancel::new(),
        )
        .expect("the run is not refused");
        let [decided] = run.instances.as_slice() else {
            panic!("one instance: {:?}", run.instances.len());
        };
        assert_eq!(decided.instance, [("v".to_owned(), "l".to_owned())]);
        assert!(
            matches!(decided.outcome, ExistentialOutcome::Witnessed { .. }),
            "{body}: {:?}",
            decided.outcome
        );
        let subjects = [held.subject(&package, both())];
        let request = ProductClosureRequest {
            package: &package,
            clause: ProductClosureClause::Hyper(clause),
            subjects: &subjects,
            meter: UNLIMITED,
        };
        let method = StateGraphMethod {
            limits: ModelCheckLimits::default(),
            max_depth: None,
            seed: run.seed,
        };
        let settled = settle_existential(&request, decided, &method, &Cancel::new())
            .expect("the admitted instance settles");
        assert_eq!(settled.record.label, ResultLabel::Proved, "{body}");
        let mut unadmitted = decided.clone();
        unadmitted.instance = vec![("v".to_owned(), "m".to_owned())];
        assert!(matches!(
            settle_existential(&request, &unadmitted, &method, &Cancel::new()),
            Err(ProductClosureRefusal::Input("HP-5-instance"))
        ));
        assert_eq!(
            system
                .excluded_keys(&ValueType::Reference(clause.parameters()[0].object.clone()))
                .expect("excluded keys"),
            ["m"]
        );
    }
}

/// FR-181-AC-8: the source item is proved at `l`, binding the admitted
/// `Lamp`, and a body false everywhere is refuted at `l`, never at the
/// unadmitted `k` that is listed first.
#[trace("TC-3219", "FR-181-AC-8")]
#[test]
fn the_source_item_binds_only_an_admitted_key() {
    for (name, body) in [("guard", GUARD), ("read", READ)] {
        let run = Pets::LAMP.analyze(
            &format!("hyper-admitted-source-{name}"),
            &exists("Thing", &always(body)),
            claim(),
            false,
        );
        let item = run.item(0);
        assert_eq!(label(&item), "proved", "{}", run.document());
        assert_eq!(run.exit, 0, "{}", run.document());
        assert_eq!(
            item["record"]["evidence"]["instance"],
            bound_v(thing_reference("l", "Lamp")),
            "{}",
            run.document()
        );
        assert!(
            !item.to_string().contains("\"identity\":\"m\""),
            "{}",
            run.document()
        );
    }
    let control = Pets {
        listed: &[("pets", &[("k", "Lamp"), ("l", "Lamp")])],
        ..Pets::LAMP
    };
    let run = control.analyze(
        "hyper-admitted-source-control",
        &exists("Thing", &always("(v != v)")),
        claim(),
        false,
    );
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{}", run.document());
    assert_eq!(run.exit, 10, "{}", run.document());
    assert_eq!(
        item["counterexample"]["instance"],
        bound_v(thing_reference("l", "Lamp")),
        "{}",
        run.document()
    );
}

/// FR-181-AC-9: the event-position form has the same one instance.
#[trace("TC-3219", "FR-181-AC-9")]
#[test]
fn the_event_position_item_has_the_same_instance() {
    for (name, body) in [("guard", GUARD), ("read", READ)] {
        let declaration = format!(
            "hyper H using ep over behaviours (v: M::Thing) \
             exists trace a of M {{ always [0, 0] holds({body} @ a) }}\n"
        );
        let run = Pets::LAMP.analyze(
            &format!("hyper-admitted-event-{name}"),
            &declaration,
            claim(),
            false,
        );
        assert_eq!(label(&run.item(0)), "proved", "{}", run.document());
        assert!(
            !run.item(0).to_string().contains("\"identity\":\"m\""),
            "{}",
            run.document()
        );
    }
}

/// FR-181-AC-10: a package selection of an HP-5 clause with an object
/// parameter is not yet implemented, with no verdict and no instance.
#[trace("TC-3219", "FR-181-AC-10")]
#[test]
fn a_package_selection_of_an_object_parameter_hp5_clause_is_not_implemented() {
    for (name, body) in [("guard", GUARD), ("read", READ)] {
        let run = Pets::LAMP.analyze(
            &format!("hyper-admitted-package-hp5-{name}"),
            &exists("Thing", &always(body)),
            claim(),
            true,
        );
        let text = run.document().to_string();
        assert_eq!(run.exit, 21, "{text}");
        assert!(text.contains("unsupported_construct"), "{text}");
        assert!(text.contains("not-yet-implemented"), "{text}");
        assert!(run.item(0)["record"]["result"].is_null(), "{text}");
        assert!(!text.contains("\"instance\""), "{text}");
    }
}

/// FR-176-AC-6 and AC-7: a two-universal clause has the same instances, on
/// both routes.
#[trace("TC-3219", "FR-176-AC-6", "FR-176-AC-7")]
#[test]
fn a_two_universal_item_ranges_over_the_admitted_keys_alone() {
    for packaged in [false, true] {
        let run = Pets::LAMP.analyze(
            &format!("hyper-admitted-universal-{packaged}"),
            &universal("Thing", &always(GUARD)),
            claim(),
            packaged,
        );
        assert_eq!(label(&run.item(0)), "proved", "{}", run.document());
        assert_eq!(run.exit, 0, "{}", run.document());
    }
}

/// FR-176-AC-7 and FR-125-AC-19: over two overlapping populations the
/// instances are `d` then `a`; the body that holds only of `P1`'s keys is
/// refuted at `a`, a `Switch`.
#[trace("TC-3219", "FR-125-AC-19", "FR-176-AC-7")]
#[test]
fn every_overlapping_population_gives_instances() {
    let in_p1 = "present(lookup<M::Thing>(M::P1, v) absent empty)";
    let either = "(present(lookup<M::Thing>(M::P1, v) absent empty) \
                  or present(lookup<M::Thing>(M::P2, v) absent empty))";
    for packaged in [false, true] {
        let run = Pets::TWO.analyze(
            &format!("hyper-two-populations-universal-{packaged}"),
            &universal("Thing", &always(in_p1)),
            claim(),
            packaged,
        );
        assert_eq!(label(&run.item(0)), "refuted", "{}", run.document());
        assert_eq!(run.exit, 10, "{}", run.document());
        assert_eq!(
            counterexample_keys(&run.item(0)),
            keys(&[("v", "a")]),
            "{}",
            run.document()
        );
    }
    let run = Pets::TWO.analyze(
        "hyper-two-populations-exists",
        &exists("Thing", &always(in_p1)),
        claim(),
        false,
    );
    assert_eq!(label(&run.item(0)), "refuted", "{}", run.document());
    assert_eq!(
        run.item(0)["counterexample"]["instance"],
        bound_v(thing_reference("a", "Switch")),
        "{}",
        run.document()
    );
    let run = Pets::TWO.analyze(
        "hyper-two-populations-either",
        &exists("Thing", &always(either)),
        claim(),
        false,
    );
    assert_eq!(label(&run.item(0)), "proved", "{}", run.document());
}

/// FR-172-AC-11 and FR-176-AC-7: an object type no population overlaps checks
/// and gives no instance, on both routes.
#[trace("TC-3219", "FR-172-AC-11", "FR-176-AC-7")]
#[test]
fn a_parameter_with_no_overlapping_population_settles_no_instance() {
    for (name, declaration, packaged) in [
        ("exists", exists("Other", &always(READ)), false),
        ("universal", universal("Other", &always(READ)), false),
        ("package", universal("Other", &always(READ)), true),
    ] {
        let run = Pets::LAMP.analyze(
            &format!("hyper-no-population-{name}"),
            &declaration,
            claim(),
            packaged,
        );
        let item = run.item(0);
        assert_eq!(label(&item), "inconclusive", "{}", run.document());
        assert_eq!(
            item["record"]["cause"]["kind"],
            "no-instance",
            "{}",
            run.document()
        );
    }
}

/// FR-125-AC-21: the instancing outcomes of both forms, none a fault.
#[trace("TC-3219", "FR-125-AC-21", "FR-181-AC-11")]
#[test]
fn the_instancing_outcomes_are_never_a_fault() {
    let unadmitted = Pets {
        populations: &[("pets", &["Thing"])],
        held: &[("pets", &[])],
        listed: &[("pets", &[("m", "Lamp")])],
    };
    for (name, declaration) in [
        ("exists", exists("Thing", &always(READ))),
        ("universal", universal("Thing", &always(READ))),
    ] {
        let run = unadmitted.analyze(
            &format!("hyper-only-unadmitted-{name}"),
            &declaration,
            claim(),
            false,
        );
        let item = run.item(0);
        assert_eq!(label(&item), "inconclusive", "{}", run.document());
        assert_eq!(
            item["record"]["cause"]["kind"],
            "no-instance",
            "{}",
            run.document()
        );
    }
}

/// FR-125-AC-22: with two parameters the first varies slowest, so the first
/// violating pair is `(l, k)`.
#[trace("TC-3219", "FR-125-AC-22")]
#[test]
fn the_first_parameter_varies_slowest() {
    let pair = Pets {
        populations: &[("pets", &["Thing"])],
        held: &[("pets", &[("l", "Lamp"), ("k", "Lamp")])],
        listed: &[("pets", &[("l", "Lamp"), ("k", "Lamp")])],
    };
    let declaration = "hyper H using t0 over behaviours (v: M::Thing, w: M::Thing) \
        forall trace a of M forall trace b of M { always holds((v = w) @ a) }\n";
    let run = pair.analyze("hyper-two-parameters", declaration, claim(), false);
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{}", run.document());
    assert_eq!(item["counterexample"]["trace_position"], "0", "{item}");
    assert_eq!(
        counterexample_keys(&item),
        keys(&[("v", "l"), ("w", "k")]),
        "{}",
        run.document()
    );
}

/// The `Counter` of FR-176-AC-8: `value: Int[0, 5]`, `inc` adding one while
/// below 3 and `idle` always enabled, from one initial state `held`, over the
/// universe `listed`.
fn counter_analyze(
    name: &str,
    declaration: &str,
    held: &[(&str, i64)],
    listed: &[&str],
    item: Json,
) -> Run {
    ranged_counter_analyze(name, declaration, held, listed, item, true)
}

/// A `native-analyze/1` request over the receiver-framed Counter with `value`
/// in `Int[0, 5]` and `inc` enabled below 3; `idle` adds an always enabled
/// operation that changes nothing.
fn ranged_counter_analyze(
    name: &str,
    declaration: &str,
    held: &[(&str, i64)],
    listed: &[&str],
    item: Json,
    idle: bool,
) -> Run {
    let mut model: Json =
        serde_json::from_slice(&crate::analyze_reach_rulings::receiver_framed_document())
            .expect("counter JSON");
    let types = model["types"].as_array_mut().expect("types");
    types[0]["constraints"][1]["operands"]["value"] = json!("5");
    if idle {
        let operations = types[1]["operations"].as_array_mut().expect("operations");
        let mut stay = operations[0].clone();
        stay["identity"] = json!("ix://test/counter/Counter/idle");
        stay["name"] = json!("idle");
        stay["frame"] = json!({"modifies": [], "creates": [], "deletes": []});
        operations.push(stay);
    }
    let universe: Vec<(&str, i64)> = listed.iter().map(|key| (*key, 0)).collect();
    let universe = String::from_utf8(counter_snapshot(&universe))
        .expect("snapshot text")
        .replace("\"identity\":\"start\"", "\"identity\":\"u0\"")
        .into_bytes();
    let unit = unit_text_in(
        crate::counter::PACKAGE,
        &format!("{}{declaration}", crate::counter::CONTRACT),
    );
    let workspace = Workspace::new(name);
    analyze(&request_over(
        &workspace,
        &unit,
        model.to_string().as_bytes(),
        &[counter_snapshot(held), universe],
        json!({
            "items": [item],
            "subject": {
                "initial_states": [{"authority": "test", "identity": "start"}],
                "universes": [{"authority": "test", "identity": "u0"}],
                "proof_bounds": []
            }
        }),
    ))
}

fn counter_universal(bound: u8) -> String {
    format!(
        "hyper H using t0 over behaviours (v: M::Counter) \
         forall trace a of M forall trace b of M {{ always holds((v.value <= {bound}) @ a) }}\n"
    )
}

fn counter_exists(bound: u8) -> String {
    format!(
        "hyper H using t0 over behaviours (v: M::Counter) \
         exists trace a of M {{ always holds((v.value <= {bound}) @ a) }}\n"
    )
}

/// FR-176-AC-8: the first refuted instance in order decides the item, whether
/// or not a later one is refuted at a shorter position, and an earlier
/// instance stopped at its bound does not drop it.
#[trace("TC-3219", "FR-176-AC-8")]
#[test]
fn a_universal_item_takes_the_first_refuted_instance() {
    let run = counter_analyze(
        "hyper-first-refuted",
        &counter_universal(1),
        &[("p", 0), ("q", 2)],
        &["p", "q"],
        claim(),
    );
    assert_eq!(label(&run.item(0)), "refuted", "{}", run.document());
    assert_eq!(
        counterexample_keys(&run.item(0)),
        keys(&[("v", "p")]),
        "{}",
        run.document()
    );
    let run = counter_analyze(
        "hyper-refuted-after-bound",
        &counter_universal(3),
        &[("p", 0), ("q", 4)],
        &["p", "q"],
        json!({"claim": "H", "temporal": {"max_depth": 1}}),
    );
    assert_eq!(label(&run.item(0)), "refuted", "{}", run.document());
    assert_eq!(
        counterexample_keys(&run.item(0)),
        keys(&[("v", "q")]),
        "{}",
        run.document()
    );
}

/// FR-181-AC-11: `p` is not decided within the bound and `q` is trapped, so
/// the item is refuted at `q`; with `p` alone it is inconclusive, and with no
/// instance it is inconclusive `no-instance`.
#[trace("TC-3219", "FR-181-AC-11")]
#[test]
fn a_single_existential_item_takes_the_refuted_instance_past_an_undecided_one() {
    let item = || json!({"claim": "H", "temporal": {"max_depth": 1, "witness_samples": 0}});
    let run = ranged_counter_analyze(
        "hyper-exists-refuted-later",
        &counter_exists(3),
        &[("p", 0), ("q", 4)],
        &["p", "q"],
        item(),
        false,
    );
    assert_eq!(label(&run.item(0)), "refuted", "{}", run.document());
    assert_eq!(run.exit, 10, "{}", run.document());
    assert_eq!(
        run.item(0)["counterexample"]["instance"]["key"]["identity"],
        "q",
        "{}",
        run.document()
    );
    let run = ranged_counter_analyze(
        "hyper-exists-undecided",
        &counter_exists(3),
        &[("p", 0)],
        &["p"],
        item(),
        false,
    );
    assert_eq!(label(&run.item(0)), "inconclusive", "{}", run.document());
    assert_eq!(
        run.item(0)["record"]["cause"]["kind"],
        "bound-reached",
        "{}",
        run.document()
    );
    let run = ranged_counter_analyze(
        "hyper-exists-no-instance",
        &counter_exists(3),
        &[],
        &["r"],
        item(),
        false,
    );
    assert_eq!(label(&run.item(0)), "inconclusive", "{}", run.document());
    assert_eq!(
        run.item(0)["record"]["cause"]["kind"],
        "no-instance",
        "{}",
        run.document()
    );
}

/// `declaration` over the `test/things` document of `held`, checked and
/// linked.
fn checked(held: &World, declaration: &str) -> CheckedPackage {
    let graph = declarations_in(PACKAGE, &held.document, declaration)
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{declaration}: {refusals:?}"));
    CheckedPackage::link(graph)
}

/// The engine's run of the two-universal clause of `package` over `held`.
fn run_universal(
    held: &World,
    package: &CheckedPackage,
    listed: &[(&str, &[&str])],
) -> Result<HyperOutcome, ModelCheckRefusal> {
    let [clause] = package.graph().hyper_clauses() else {
        panic!("one hyper clause");
    };
    let subject = || held.subject(package, universes(listed));
    let system = ModelSystem::new(subject().simulation)
        .map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    let letters =
        CheckedHyperLetters::new(package, clause, vec![&system, &system]).expect("letters");
    let matcher = CheckedStepMatch::new(package, clause);
    check_hyper(
        HyperCheckRequest {
            clause,
            subjects: vec![subject()],
            letters: &letters,
            matcher: &matcher,
            limits: ModelCheckLimits::default(),
            meter: UNLIMITED,
            reductions: Default::default(),
            max_depth: None,
        },
        &Cancel::new(),
    )
}

/// The engine's run of the single-existential clause of `package` over
/// `held`.
fn run_single(
    held: &World,
    package: &CheckedPackage,
    listed: &[(&str, &[&str])],
) -> Result<SingleExistentialRun, ModelCheckRefusal> {
    let [clause] = package.graph().hyper_clauses() else {
        panic!("one hyper clause");
    };
    let subject = || held.subject(package, universes(listed));
    let system = ModelSystem::new(subject().simulation)
        .map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    let letters = CheckedHyperLetters::new(package, clause, vec![&system]).expect("letters");
    check_single_existential(
        SingleExistentialRequest {
            clause,
            subject: subject(),
            letters: &letters,
            limits: ModelCheckLimits::default(),
            max_depth: None,
            seed: None,
            meter: UNLIMITED,
        },
        &Cancel::new(),
    )
}

/// FR-176-AC-6: the engine holds with a certificate for the instance `l` and
/// none for `m`.
#[trace("TC-3219", "FR-176-AC-6")]
#[test]
fn a_two_universal_run_certifies_the_admitted_instance_alone() {
    let held = Pets::LAMP.initial();
    let package = checked(&held, &universal("Thing", &always(GUARD)));
    let outcome =
        run_universal(&held, &package, &[("pets", &["l", "m"])]).expect("the run is not refused");
    let HyperOutcome::Holds { certificates, .. } = outcome else {
        panic!("the guard holds of the admitted key: {outcome:?}");
    };
    let certified: Vec<_> = certificates
        .iter()
        .map(|certificate| certificate.instance.clone())
        .collect();
    assert_eq!(certified, [[("v".to_owned(), "l".to_owned())].to_vec()]);
}

/// FR-125-AC-21: the instancing outcomes the engine reports for both forms
/// before it explores, none a fault; the two-key types refuse at admission,
/// which both forms pass through.
#[trace("TC-3219", "FR-125-AC-21")]
#[test]
fn the_engine_reports_each_instancing_outcome_for_both_forms() {
    let one = Pets::LAMP.initial();
    let single = checked(&one, &exists("Thing", &always(READ)));
    let pair = checked(&one, &universal("Thing", &always(READ)));
    let listed: &[(&str, &[&str])] = &[("pets", &["l"])];

    for refusal in [
        run_single(&one, &single, &[]).map(|_| ()),
        run_universal(&one, &pair, &[]).map(|_| ()),
    ] {
        let Err(ModelCheckRefusal::RequiresBound(bound)) = refusal else {
            panic!("`pets` has no universe: {refusal:?}");
        };
        assert!(format!("{bound:?}").contains("pets"), "{bound:?}");
    }

    let mut empty = Pets::LAMP.initial();
    empty.initial.clear();
    let run = run_single(&empty, &single, listed).expect("not a fault");
    assert!(run.instances.is_empty(), "{:?}", run.instances.len());
    assert!(matches!(
        run_universal(&empty, &pair, listed).expect("not a fault"),
        HyperOutcome::Undecided {
            cause: UndecidedCause::NoInitialState,
            ..
        }
    ));

    let unadmitted = world(&[("pets", &["Thing"])], &[("pets", &[])]);
    let run = run_single(&unadmitted, &single, &[("pets", &["m"])]).expect("not a fault");
    assert!(run.instances.is_empty(), "{:?}", run.instances.len());
    assert!(matches!(
        run_universal(&unadmitted, &pair, &[("pets", &["m"])]).expect("not a fault"),
        HyperOutcome::Undecided {
            cause: UndecidedCause::NoInstance,
            ..
        }
    ));

    let two = world(
        Pets::TWO.populations,
        &[("P1", &[("l", "Lamp")]), ("P2", &[("l", "Switch")])],
    );
    let package = checked(&two, &exists("Thing", &always(READ)));
    let refusal = run_single(&two, &package, &[("P1", &["l"]), ("P2", &["l"])])
        .expect_err("one key has two types");
    let ModelCheckRefusal::Admission(failure) = refusal else {
        panic!("an admission refusal: {refusal:?}");
    };
    let AdmissionFailure::Refused(record) = *failure else {
        panic!("a typed refusal: {failure:?}");
    };
    assert_eq!(record.catalog.code(), Code::InvalidRuntimeInput.as_str());
    assert_eq!(record.catalog.cause(), "conflicting-identity");
}

/// FR-125-AC-21's last case: over two aliases, one parameter has an admitted
/// key and the other none.
#[trace("TC-3219", "FR-125-AC-21")]
#[test]
fn a_parameter_of_a_second_alias_with_no_admitted_key_gives_no_instance() {
    let world = crate::hyper_world::World::with_second_vaults(
        crate::hyper_world::Vault::Secure,
        &[("saving", (0, 1))],
        &[],
        "hyper Saves using inf over behaviours (v: V::Vault, w: N::Vault) \
         forall trace s of V forall trace n of N { always holds(v.l @ s <= w.l @ n) }",
    );
    assert!(matches!(
        world
            .run_selecting(Reductions::default())
            .expect("not a fault"),
        HyperOutcome::Undecided {
            cause: UndecidedCause::NoInstance,
            ..
        }
    ));
}

/// FR-172-AC-10: two populations overlap `Thing`, and the checked parameter
/// holds both; a population of an unrelated type is not among them.
#[trace("TC-3219", "FR-172-AC-10")]
#[test]
fn a_parameter_holds_every_overlapping_population() {
    let held = world(
        &[("P1", &["Lamp"]), ("P2", &["Switch"]), ("Q", &["Other"])],
        &[],
    );
    let package = checked(
        &held,
        "hyper H using t0 over behaviours (v: M::Thing, l: M::Lamp, s: M::Switch) \
         exists trace a of M { always holds((v = v) @ a) }\n",
    );
    let [clause] = package.graph().hyper_clauses() else {
        panic!("one hyper clause");
    };
    let [thing, lamp, switch] = clause.parameters() else {
        panic!("three parameters");
    };
    let members: BTreeSet<_> = thing
        .populations
        .iter()
        .map(|population| population.object())
        .collect();
    assert_eq!(thing.populations.len(), 2);
    assert_eq!(
        members,
        BTreeSet::from([lamp.object.clone(), switch.object.clone()])
    );
}

/// FR-125-AC-14's subject as AC-20 reads it: `inc` disabled and `make`,
/// which creates a `Counter`.
fn counter_with_make() -> Vec<u8> {
    let mut model: Json =
        serde_json::from_slice(&crate::counter::document(false)).expect("counter JSON");
    let operations = model["types"][1]["operations"]
        .as_array_mut()
        .expect("operations");
    let mut make = operations[0].clone();
    make["identity"] = json!("ix://test/counter/Counter/make");
    make["name"] = json!("make");
    make["frame"] = json!({
        "modifies": [], "creates": ["ix://test/counter/Counter"], "deletes": []
    });
    operations.push(make);
    model.to_string().into_bytes()
}

const MAKE_UNIT: &str = "pre Disabled using v on M::Counter::inc { false }\n";

/// FR-125-AC-20: a key only an operation creates is never an instance; the
/// instances are `c` and `d`, each witnessed, and both forms are proved.
#[trace("TC-3219", "FR-125-AC-20")]
#[test]
fn a_key_only_an_operation_creates_is_no_hyper_instance() {
    let document = counter_with_make();
    let existential = "hyper H using t0 over behaviours (v: M::Counter) \
        exists trace a of M { always holds((v.value = 0) @ a) }\n";
    let universal = "hyper H using t0 over behaviours (v: M::Counter) \
        forall trace a of M { always holds(v.value @ a = 0) }\n";
    for (name, declaration) in [("exists", existential), ("universal", universal)] {
        let universe = String::from_utf8(counter_snapshot(&[("c", 0), ("d", 0), ("z", 0)]))
            .expect("snapshot text")
            .replace("\"identity\":\"start\"", "\"identity\":\"u0\"")
            .into_bytes();
        let unit = unit_text_in(
            crate::counter::PACKAGE,
            &format!("{MAKE_UNIT}{TERMINAL}{declaration}"),
        );
        let workspace = Workspace::new(&format!("hyper-created-key-{name}"));
        let run = analyze(&request_over(
            &workspace,
            &unit,
            &document,
            &[counter_snapshot(&[("c", 0), ("d", 0)]), universe],
            json!({
                "items": [claim()],
                "subject": {
                    "initial_states": [{"authority": "test", "identity": "start"}],
                    "universes": [{"authority": "test", "identity": "u0"}],
                    "proof_bounds": []
                }
            }),
        ));
        assert_eq!(label(&run.item(0)), "proved", "{}", run.document());
        assert_eq!(run.exit, 0, "{}", run.document());
    }

    let package = crate::counter::package(&document, &format!("{MAKE_UNIT}{existential}"));
    let mut documents = Documents::new(document);
    let start = documents.start("start", &[("c", 0), ("d", 0)]);
    let [clause] = package.graph().hyper_clauses() else {
        panic!("one hyper clause");
    };
    let subject = || {
        crate::counter::subject(
            &package,
            &documents,
            vec![start.clone()],
            &["c", "d", "z"],
            UNLIMITED,
        )
    };
    let system = ModelSystem::new(subject().simulation).expect("the subject admits");
    let letters = CheckedHyperLetters::new(&package, clause, vec![&system]).expect("letters");
    let run = check_single_existential(
        SingleExistentialRequest {
            clause,
            subject: subject(),
            letters: &letters,
            limits: ModelCheckLimits::default(),
            max_depth: None,
            seed: None,
            meter: UNLIMITED,
        },
        &Cancel::new(),
    )
    .expect("the run is not refused");
    let witnessed: Vec<_> = run
        .instances
        .iter()
        .map(|decided| {
            assert!(
                matches!(decided.outcome, ExistentialOutcome::Witnessed { .. }),
                "{:?}",
                decided.outcome
            );
            decided.instance[0].1.as_str()
        })
        .collect();
    assert_eq!(witnessed, ["c", "d"]);
}

/// FR-125-AC-21 through `native-analyze/1`: a subject with an empty `initial`
/// list reports `no-initial-state`, never `no-instance`, on both forms.
#[trace("TC-3219", "FR-125-AC-21")]
#[test]
fn an_empty_initial_list_reports_no_initial_state() {
    let held = Pets::LAMP.initial();
    let universe = String::from_utf8(
        held.snapshots
            .values()
            .next()
            .expect("one snapshot")
            .clone(),
    )
    .expect("snapshot text")
    .replace("\"identity\":\"s0\"", "\"identity\":\"u0\"")
    .into_bytes();
    for (name, declaration) in [
        ("exists", exists("Thing", &always(READ))),
        ("universal", universal("Thing", &always(READ))),
    ] {
        let workspace = Workspace::new(&format!("hyper-empty-initial-{name}"));
        let run = analyze(&request_over(
            &workspace,
            &unit_text_in(PACKAGE, &format!("{TERMINAL}{declaration}")),
            &held.document,
            std::slice::from_ref(&universe),
            json!({
                "items": [claim()],
                "subject": {
                    "initial_states": [],
                    "universes": [{"authority": "test", "identity": "u0"}],
                    "proof_bounds": []
                }
            }),
        ));
        let item = run.item(0);
        assert_eq!(label(&item), "inconclusive", "{}", run.document());
        assert_eq!(
            item["record"]["cause"]["kind"],
            "no-initial-state",
            "{}",
            run.document()
        );
    }
}

/// A temporal item over several instances reports the first refuted one in
/// the instance order, which follows the universe list and not key order.
#[trace("TC-3219", "FR-621-AC-9", "FR-621-AC-23")]
#[test]
fn a_temporal_item_reports_the_first_refuted_instance_in_universe_order() {
    let clause = "temporal Q using t0 over (c: M::Counter) clock \"model-steps\" on origin \
        { always holds(c.value >= 1) }\n";
    for (name, held, listed) in [
        (
            "first-refuted",
            &[("a", 1), ("b", 0), ("d", 0)][..],
            &["a", "b", "d"][..],
        ),
        ("universe-order", &[("a", 0), ("b", 0)][..], &["b", "a"][..]),
    ] {
        let run = counter_analyze(
            &format!("temporal-{name}"),
            clause,
            held,
            listed,
            json!({"claim": "Q"}),
        );
        let item = run.item(0);
        assert_eq!(label(&item), "refuted", "{}", run.document());
        assert_eq!(
            item["counterexample"]["over_binding"]["identity"],
            "b",
            "{}",
            run.document()
        );
    }
}

/// FR-181-AC-12: the source item of a clause with two object parameters has no
/// evidence shape, so it is not yet implemented, with no verdict and no
/// instance.
#[trace("TC-3219", "FR-181-AC-12")]
#[test]
fn a_two_parameter_hp5_item_is_not_implemented() {
    let pair = Pets {
        populations: &[("pets", &["Thing"])],
        held: &[("pets", &[("l", "Lamp"), ("k", "Lamp")])],
        listed: &[("pets", &[("l", "Lamp"), ("k", "Lamp")])],
    };
    let declaration = "hyper H using t0 over behaviours (v: M::Thing, w: M::Thing) \
        exists trace a of M { always holds((v = w) @ a) }\n";
    let run = pair.analyze("hyper-two-parameters-exists", declaration, claim(), false);
    let text = run.document().to_string();
    assert_eq!(run.exit, 21, "{text}");
    assert!(text.contains("unsupported_construct"), "{text}");
    assert!(text.contains("not-yet-implemented"), "{text}");
    let item = run.item(0);
    assert_eq!(item["record"]["result"], "unsupported", "{text}");
    assert!(item["axes"]["truth"].is_null(), "{text}");
    assert!(!text.contains("\"instance\""), "{text}");
}

/// The `Counter` of FR-176-AC-8 from `p` at 0 and `q` at 2, with `idle`
/// always enabled.
fn counter_with_idle(claim: &str) -> crate::graph_world::World {
    use crate::graph_world::{Field, Model, Operation};
    let model = Model {
        package: "test/counter",
        alias: "M",
        object: "Counter",
        population: "counters",
        symmetric: false,
        fields: vec![Field {
            name: "value",
            range: Some((0, 5)),
        }],
        operations: vec![
            Operation {
                name: "inc",
                modifies: &["value"],
            },
            Operation {
                name: "idle",
                modifies: &[],
            },
        ],
    };
    crate::graph_world::World::with_profiles(
        model,
        "profile t0 = \"filament.temporal.infinite-trace/v1\";\n",
        &format!("{}{claim}", crate::counter::CONTRACT),
        &[&[("p", &[("value", 0)]), ("q", &[("value", 2)])]],
    )
}

/// FR-621-AC-26, the universal half: `p` is violated at position 2 and `q` at
/// position 0. With `p`'s counterexample replaced by the lasso that takes
/// `idle` at every step, on which `p` stays at 0, the item is
/// `replay-parity`, not refuted by `q`.
#[trace("TC-3219", "FR-621-AC-26")]
#[test]
fn a_universal_counterexample_that_does_not_replay_is_not_displaced_by_a_later_instance() {
    let world = counter_with_idle(&counter_universal(1));
    let [clause] = world.package.graph().hyper_clauses() else {
        panic!("one hyper clause");
    };
    let subject = || world.subject(&["p", "q"], UNLIMITED);
    let system = ModelSystem::new(subject().simulation).expect("admitted");
    let letters =
        CheckedHyperLetters::new(&world.package, clause, vec![&system, &system]).expect("letters");
    let matcher = CheckedStepMatch::new(&world.package, clause);
    let limits = ModelCheckLimits::default();
    let mut outcome = check_hyper(
        HyperCheckRequest {
            clause,
            subjects: vec![subject()],
            letters: &letters,
            matcher: &matcher,
            limits,
            meter: UNLIMITED,
            reductions: Default::default(),
            max_depth: None,
        },
        &Cancel::new(),
    )
    .expect("the run is not refused");
    let HyperOutcome::Violated { counterexample, .. } = &mut outcome else {
        panic!("a violation: {outcome:?}");
    };
    assert_eq!(counterexample.instance, [("v".to_owned(), "p".to_owned())]);

    let initial = system.initial()[0].clone();
    let (transition, next) = system
        .successors(&initial)
        .expect("the counter expands")
        .successors
        .into_iter()
        .find(|(transition, _)| {
            transition.operation().contains("/idle\"")
                && transition
                    .receiver()
                    .is_some_and(|key| key.contains("\"p\""))
        })
        .expect("idle on p is enabled");
    let idle = HyperStep::Model(ModelStep {
        transition,
        post_state: system
            .state_key(&next, fsl_foundation::IdentityLimits::default())
            .expect("a key")
            .0
            .into(),
        delay: None,
    });
    for trace in &mut counterexample.traces {
        trace.initial = 0;
        trace.steps = vec![idle.clone()];
        trace.results = Vec::new();
    }
    counterexample.kind = HyperCounterexampleKind::Lockstep { loop_entry: 0 };
    counterexample.trace_position = 0;
    let record = fsl_replay::settle_hyper_check(
        &outcome,
        &limits,
        &[],
        None,
        &Cancel::new(),
        |counterexample| {
            let [(parameter, key)] = counterexample.instance.as_slice() else {
                panic!("one binding: {:?}", counterexample.instance);
            };
            let bound = system
                .instances(&ValueType::Reference(clause.parameters()[0].object.clone()))
                .expect("instances")
                .into_iter()
                .find(|instance| &instance.key == key)
                .expect("an instance");
            replay_hyper_trace_tuple(
                &HyperReplay {
                    clause,
                    systems: vec![&system, &system],
                    instance: &[ParameterBinding {
                        parameter: parameter.clone(),
                        instance: bound,
                    }],
                    letters: &letters,
                    matcher: &matcher,
                    meter: UNLIMITED,
                    max_witness_set: limits.max_witness_set,
                    max_states: u64::try_from(limits.limits.max_states).expect("small"),
                    cancel: &Cancel::new(),
                },
                counterexample,
            )
        },
    )
    .expect("the outcome settles");
    assert!(
        matches!(
            record.value,
            TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(
                DisagreementCause::Verdicts { .. }
            ))
        ),
        "{:?}",
        record.value
    );
}

/// FR-125-AC-14's subject with `inc` disabled and `drop`, which may delete a
/// `Counter`.
fn counter_with_drop() -> Vec<u8> {
    let mut model: Json =
        serde_json::from_slice(&crate::counter::document(false)).expect("counter JSON");
    let operations = model["types"][1]["operations"]
        .as_array_mut()
        .expect("operations");
    let mut delete = operations[0].clone();
    delete["identity"] = json!("ix://test/counter/Counter/drop");
    delete["name"] = json!("drop");
    delete["frame"] = json!({
        "modifies": [], "creates": [], "deletes": ["ix://test/counter/Counter"]
    });
    operations.push(delete);
    model.to_string().into_bytes()
}

/// FR-181-AC-5: once `drop` has deleted the instance's object, its read has
/// no value, so the source item is refuted with the `absent-key` evaluation
/// at the position after that step.
#[trace("TC-3219", "FR-181-AC-5")]
#[test]
fn an_hp5_item_with_an_undefined_read_is_refuted_with_its_evaluation() {
    let declaration = "hyper H using t0 over behaviours (v: M::Counter) \
        exists trace a of M { always holds((v.value = 0) @ a) }\n";
    let unit = unit_text_in(
        crate::counter::PACKAGE,
        &format!("{MAKE_UNIT}{TERMINAL}{declaration}"),
    );
    let workspace = Workspace::new("hyper-dropped-key");
    let universe = String::from_utf8(counter_snapshot(&[("c", 0)]))
        .expect("snapshot text")
        .replace("\"identity\":\"start\"", "\"identity\":\"u0\"")
        .into_bytes();
    let run = analyze(&request_over(
        &workspace,
        &unit,
        &counter_with_drop(),
        &[counter_snapshot(&[("c", 0)]), universe],
        json!({
            "items": [claim()],
            "subject": {
                "initial_states": [{"authority": "test", "identity": "start"}],
                "universes": [{"authority": "test", "identity": "u0"}],
                "proof_bounds": []
            }
        }),
    ));
    let item = run.item(0);
    assert_eq!(label(&item), "refuted", "{}", run.document());
    assert_eq!(run.exit, 10, "{}", run.document());
    let counterexample = &item["counterexample"];
    assert_eq!(counterexample["arm"], "undefined", "{item}");
    assert_eq!(counterexample["undefined"]["cause"], "absent-key", "{item}");
    assert_eq!(
        counterexample["undefined"]["where"]["position"], 1,
        "{item}"
    );
}
