// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cross-route comparison of the object set a claim ranges over (FR-125
//! "Instancing"): every route and checker is given the same populations and
//! asked which keys are instances, or why there are none. Expected values
//! are written from the criteria, never from a route's output.
//!
//! An ordered instance list is read from a route by probing: instance `i`
//! of the expected list is given `age` `i + 1`, and the claim `x.age <= i`
//! is refuted at the `i`-th instance for each `i < n` and proved at `i = n`.
//! An excluded key holds no object, so an extra instance changes a verdict.
//! Liveness is probed with the refuted claims only: it reads the instances in
//! order and does not show that no further one follows.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use filament_exact::{Cancel, Meter};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_analyze::model_check::{
    check_model, check_state_graph, ModelCheckRefusal, ModelCheckRequest, StateGraphItem,
    StateGraphRequest,
};
use fsl_analyze::plan_search::{
    search, PlanLengths, PlanSearchLimits, PlanSearchRequest, PlanStatus,
};
use fsl_analyze::plan_source::SourcePlan;
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{ModelSystem, PopulationUniverse};
use fsl_package::CheckedPackage;
use fsl_replay::{
    check_closure, check_components, check_product_closure, encode_hyper_query,
    encode_temporal_smt_query, CertificateRequest, ClosureCertificate, HyperQueryRequest,
    ModelCheckItem, ModelCheckLimits, ModelCheckOutcome, ProductClosure, ProductClosureCertificate,
    ProductClosureClause, ProductClosureRequest, ProofCertificate, SmtEncodingLimits, SmtQueryKind,
    StateGraphCertificateRequest, StateGraphOutcome,
};
use fsl_semantics::check::HyperForm;
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::observation::{document_reference, DocumentRef};
use ix_trace_rs::trace;
use serde_json::{json, Value as Json};

use crate::analyze_cli_support::{analyze, request_over, Run, Workspace};
use crate::counter::{declarations_in, UNLIMITED};
use crate::model_instances::{document, ident, universes, world, Held, World, PACKAGE};
use crate::timed_world::{unit_text, DENSE};

type Named = &'static [(&'static str, &'static [&'static str])];

/// The object set a claim over the case's subject ranges over, or why it has
/// none, in FR-125's order of causes.
#[derive(Clone, Copy, Debug)]
enum Expect {
    Instances(&'static [&'static str]),
    NoInstance,
    NoInitialState,
    Conflict,
    RequiresBound,
}

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    populations: Named,
    held: &'static [Held<'static>],
    listed: Named,
    initial: bool,
    make: bool,
    /// The model declares `Animal` as its only object type, which a timed
    /// model needs (FR-230-AC-38).
    single_type: bool,
    over: &'static str,
    expect: Expect,
}

const ONE: Named = &[("animals", &["Animal"])];
const TWO: Named = &[("P1", &["Animal"]), ("P2", &["Dog"])];

const BASE: Case = Case {
    name: "base",
    populations: ONE,
    held: &[("animals", &[("a", "Animal")])],
    listed: &[("animals", &["a"])],
    initial: true,
    make: false,
    single_type: false,
    over: "Animal",
    expect: Expect::NoInstance,
};

const EMPTY_UNIVERSE: Case = Case {
    name: "empty-universe",
    held: &[("animals", &[])],
    listed: &[("animals", &[])],
    ..BASE
};
const ONLY_UNADMITTED: Case = Case {
    name: "only-unadmitted",
    held: &[("animals", &[])],
    listed: &[("animals", &["u"])],
    ..BASE
};
const ORPHAN: Case = Case {
    name: "orphan",
    over: "Thing",
    ..BASE
};
const NO_INITIAL: Case = Case {
    name: "no-initial",
    initial: false,
    expect: Expect::NoInitialState,
    ..BASE
};
const EMPTY_UNIVERSE_NO_INITIAL: Case = Case {
    name: "empty-universe-no-initial",
    held: &[("animals", &[])],
    listed: &[("animals", &[])],
    initial: false,
    expect: Expect::NoInitialState,
    ..BASE
};
const ORPHAN_NO_INITIAL: Case = Case {
    name: "orphan-no-initial",
    over: "Thing",
    initial: false,
    expect: Expect::NoInitialState,
    ..BASE
};
const NO_UNIVERSE: Case = Case {
    name: "no-universe-no-initial",
    listed: &[],
    initial: false,
    expect: Expect::RequiresBound,
    ..BASE
};
const CONFLICT: Case = Case {
    name: "conflicting-identity",
    populations: TWO,
    held: &[("P1", &[("c", "Cat")]), ("P2", &[("c", "Dog")])],
    listed: &[("P1", &["c"]), ("P2", &["c"])],
    expect: Expect::Conflict,
    ..BASE
};
const UNADMITTED_KEY: Case = Case {
    name: "unadmitted-key",
    listed: &[("animals", &["u", "a", "v"])],
    expect: Expect::Instances(&["a"]),
    ..BASE
};
const SUBTYPE_ONLY: Case = Case {
    name: "subtype-only",
    populations: &[("dogs", &["Dog"])],
    held: &[("dogs", &[("d", "Dog"), ("e", "Dog")])],
    listed: &[("dogs", &["e", "d"])],
    expect: Expect::Instances(&["e", "d"]),
    ..BASE
};
const CREATED: Case = Case {
    name: "created",
    held: &[("animals", &[("c", "Animal"), ("d", "Animal")])],
    listed: &[("animals", &["c", "d", "z"])],
    make: true,
    expect: Expect::Instances(&["c", "d"]),
    ..BASE
};
const UNADMITTED_ACROSS: Case = Case {
    name: "unadmitted-across-populations",
    populations: TWO,
    held: &[("P1", &[("d", "Animal")]), ("P2", &[("a", "Dog")])],
    listed: &[("P1", &["d", "u"]), ("P2", &["u", "a"])],
    expect: Expect::Instances(&["d", "a"]),
    ..BASE
};
const TWO_POPULATIONS: Case = Case {
    name: "two-populations",
    populations: TWO,
    held: &[
        ("P1", &[("d", "Animal"), ("c", "Dog")]),
        ("P2", &[("c", "Dog"), ("a", "Dog")]),
    ],
    listed: &[("P1", &["d", "c"]), ("P2", &["c", "a"])],
    expect: Expect::Instances(&["d", "c", "a"]),
    ..BASE
};

const fn single(case: Case) -> Case {
    Case {
        single_type: true,
        ..case
    }
}

/// Two populations of the one object type, listing `c` in both.
const TIMED_TWO_POPULATIONS: Case = single(Case {
    populations: &[("P1", &["Animal"]), ("P2", &["Animal"])],
    held: &[
        ("P1", &[("d", "Animal"), ("c", "Animal")]),
        ("P2", &[("c", "Animal"), ("a", "Animal")]),
    ],
    ..TWO_POPULATIONS
});

/// Every case with at least one instance.
const INSTANCES: &[Case] = &[
    UNADMITTED_KEY,
    SUBTYPE_ONLY,
    CREATED,
    UNADMITTED_ACROSS,
    TWO_POPULATIONS,
];
/// Every case with none, by the cause FR-125's order reports.
const CAUSES: &[Case] = &[
    EMPTY_UNIVERSE,
    ONLY_UNADMITTED,
    ORPHAN,
    NO_INITIAL,
    EMPTY_UNIVERSE_NO_INITIAL,
    ORPHAN_NO_INITIAL,
    NO_UNIVERSE,
    CONFLICT,
];

/// The cases a single-object-type model can state, for the timed routes.
const TIMED_INSTANCES: &[Case] = &[single(UNADMITTED_KEY), single(CREATED)];
const TIMED_NO_INSTANCE: &[Case] = &[single(EMPTY_UNIVERSE), single(ONLY_UNADMITTED)];
const TIMED_EARLIER_CAUSES: &[Case] = &[
    single(NO_INITIAL),
    single(EMPTY_UNIVERSE_NO_INITIAL),
    single(NO_UNIVERSE),
];

fn operation(name: &str, creates: &[String]) -> Json {
    let identity = ident(&format!("Animal/{name}"));
    json!({
        "identity": identity, "name": name, "params": [], "pre": [], "post": [],
        "origin": {"source": {"sourceIdentity": ident("spec"), "path": "spec.fspec",
            "startLine": 1, "startColumn": 1}},
        "frame": {"modifies": [], "creates": creates, "deletes": []},
    })
}

impl Case {
    fn instances(&self) -> &'static [&'static str] {
        match self.expect {
            Expect::Instances(instances) => instances,
            _ => &[],
        }
    }

    /// The case's populations, and the `actors` population a protocol's
    /// roles range over.
    fn populations(&self) -> Vec<(&str, &[&str])> {
        let mut populations = self.populations.to_vec();
        if !self.single_type {
            populations.push(("actors", &["Other"]));
        }
        populations
    }

    fn held(&self) -> Vec<Held<'static>> {
        let mut held = self.held.to_vec();
        if !self.single_type {
            held.push(("actors", &[("p", "Other")]));
        }
        held
    }

    fn listed(&self) -> Vec<(&str, &[&str])> {
        let mut listed = self.listed.to_vec();
        if !self.single_type {
            listed.push(("actors", &["p"]));
        }
        listed
    }

    fn model(&self) -> Vec<u8> {
        let mut model: Json = serde_json::from_slice(&document(&self.populations())).expect("JSON");
        let types = model["types"].as_array_mut().expect("types");
        if self.single_type {
            types.retain(|ty| {
                ty["kind"]["name"] != "object_type" || ty["identity"] == json!(ident("Animal"))
            });
        }
        for ty in types {
            if ty["identity"] == json!(ident("Animal")) {
                ty["fields"].as_array_mut().expect("fields").truncate(1);
                let mut operations = vec![operation("idle", &[])];
                if self.make {
                    operations.push(operation("make", &[ident("Animal")]));
                }
                ty["operations"] = Json::Array(operations);
            }
        }
        model.to_string().into_bytes()
    }

    fn age(&self, key: &str) -> usize {
        self.instances()
            .iter()
            .position(|instance| *instance == key)
            .map_or(1, |at| at + 1)
    }

    fn snapshot(&self, name: &str, held: &[Held<'_>]) -> Vec<u8> {
        let built = world(&self.populations(), held);
        let bytes = built.snapshots.values().next().expect("one snapshot");
        let mut snapshot: Json = serde_json::from_slice(bytes).expect("JSON");
        snapshot["identity"]["identity"] = json!(name);
        for population in snapshot["populations"].as_array_mut().expect("populations") {
            for object in population["objects"].as_array_mut().expect("objects") {
                if object["fields"].get("age").is_some() {
                    let age = self.age(object["key"].as_str().expect("a key"));
                    object["fields"] = json!({"age": {"integer": age.to_string()}});
                }
            }
        }
        snapshot.to_string().into_bytes()
    }

    fn universe_snapshot(&self) -> Vec<u8> {
        let objects: Vec<(&str, Vec<(&str, &str)>)> = self
            .listed()
            .into_iter()
            .map(|(population, keys)| {
                let member = self
                    .populations()
                    .into_iter()
                    .find(|(name, _)| *name == population)
                    .expect("a declared population")
                    .1[0];
                (population, keys.iter().map(|key| (*key, member)).collect())
            })
            .collect();
        let held: Vec<Held<'_>> = objects
            .iter()
            .map(|(population, objects)| (*population, objects.as_slice()))
            .collect();
        self.snapshot("u0", &held)
    }

    /// `native-analyze/1` over `unit` with this case's subject.
    fn analyze(&self, unit: &str, items: Vec<Json>) -> Run {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let initial = if self.initial {
            json!([{"authority": "test", "identity": "s0"}])
        } else {
            json!([])
        };
        let ws = Workspace::new(&format!(
            "object-sets-{}",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        analyze(&request_over(
            &ws,
            unit,
            &self.model(),
            &[self.snapshot("s0", &self.held()), self.universe_snapshot()],
            json!({"items": items, "subject": {
                "initial_states": initial,
                "universes": [{"authority": "test", "identity": "u0"}],
                "proof_bounds": []}}),
        ))
    }

    /// The probes of this case: one per bound `0..=n` over a non-empty set,
    /// one that reads no field otherwise.
    fn probes(&self) -> Vec<(usize, String)> {
        let n = self.instances().len();
        if n == 0 {
            return vec![(0, "true".to_owned())];
        }
        (0..=n).map(|i| (i, format!("x.age <= {i}"))).collect()
    }
}

#[derive(Clone, Copy)]
enum Route {
    Temporal,
    Liveness,
    StateGraph,
    SingleExistential,
    ForallForall,
    ForallExists,
    Zone,
    Digital,
    TimedStateGraph,
}

impl Route {
    fn claim(self, name: &str, over: &str, p: &str) -> String {
        match self {
            Self::Temporal => format!(
                "temporal {name} using t0 over (x: M::{over}) clock \"model-steps\" on origin {{ always holds({p}) }}\n"
            ),
            Self::Liveness => format!(
                "temporal {name} using t0 over (x: M::{over}) clock \"model-steps\" on origin {{ eventually holds({p}) }}\n"
            ),
            Self::StateGraph | Self::TimedStateGraph => {
                format!("possible {name} using v over (x: M::{over}) {{ {p} }}\n")
            }
            Self::SingleExistential => format!(
                "hyper {name} using t0 over behaviours (x: M::{over}) exists trace a of M {{ always holds(({p}) @ a) }}\n"
            ),
            Self::ForallForall => format!(
                "hyper {name} using t0 over behaviours (x: M::{over}) forall trace a of M forall trace b of M {{ always holds(({p}) @ a) }}\n"
            ),
            Self::ForallExists => format!(
                "hyper {name} using t0 over behaviours (x: M::{over}) forall trace a of M exists trace b of M {{ always holds(({p}) @ b) }}\n"
            ),
            Self::Zone | Self::Digital => format!(
                "temporal {name} using tt over (x: M::{over}) clock \"model-time\" on origin {{ always holds({p}) }}\n"
            ),
        }
    }

    fn unit(self, claims: &str) -> String {
        let terminal = "terminal Done using v on M when { true };\n";
        match self {
            Self::Zone | Self::Digital | Self::TimedStateGraph => unit_text(
                PACKAGE,
                "profile tt = \"filament.temporal.timed/v1\";\n",
                &format!("{DENSE}{terminal}{claims}"),
            ),
            _ => untimed_unit("", &format!("{terminal}{claims}")),
        }
    }

    fn item(self, name: &str) -> Json {
        match self {
            Self::Digital => json!({"claim": name, "options": {"method": "digital-clocks"}}),
            _ => json!({"claim": name}),
        }
    }
}

fn untimed_unit(profiles: &str, body: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile t0 = \"filament.temporal.infinite-trace/v1\";\n{profiles}\
         model M = \"{PACKAGE}\";\n\
         function Noop using v(): Boolean pure {{ true }}\n{body}"
    )
}

/// The key the reported binding names, in whichever form the route writes it.
fn binding(item: &Json) -> Option<&str> {
    let cx = &item["counterexample"];
    [
        &cx["over_binding"]["identity"],
        &cx["instance"]["key"]["identity"],
        &cx["instance"][0]["key"]["value"],
        &cx["instance"]["identity"],
    ]
    .into_iter()
    .find_map(Json::as_str)
}

fn short(value: &Json) -> String {
    value.to_string().chars().take(300).collect()
}

/// The `conflicting-identity` diagnostic naming `c`, at the request or at
/// each item.
fn is_conflict(diagnostic: &Json) -> bool {
    diagnostic["code"] == "invalid_runtime_input" && diagnostic["cause"] == "conflicting-identity"
}

/// What is wrong with `item` against a cause or a probe's verdict.
fn judge(case: &Case, i: usize, document: &Json, at: usize) -> Option<String> {
    let item = &document["items"][at];
    let record = &item["record"];
    let right = match case.expect {
        Expect::Instances(instances) => match instances.get(i) {
            Some(key) => record["result"] == "refuted" && binding(item) == Some(*key),
            None => record["result"] == "proved",
        },
        Expect::NoInstance => {
            record["result"] == "inconclusive" && record["cause"]["kind"] == "no-instance"
        }
        Expect::NoInitialState => {
            record["result"] == "inconclusive" && record["cause"]["kind"] == "no-initial-state"
        }
        // FR-281 "RequiresBound".
        Expect::RequiresBound => {
            record["result"] == "unsupported"
                && record["diagnostic"]["code"] == "unsupported_projection"
                && record["diagnostic"]["cause"] == "unbounded-extent"
        }
        Expect::Conflict => {
            is_conflict(&document["diagnostics"][0]) || is_conflict(&record["diagnostic"])
        }
    };
    if right {
        return None;
    }
    let seen = if item.is_null() {
        short(&document["diagnostics"])
    } else {
        format!("{} binding={:?}", short(record), binding(item))
    };
    Some(format!(
        "{} probe {i}: expected {:?}, got {seen}",
        case.name, case.expect
    ))
}

/// The exact route over no instance: unsupported, with no verdict,
/// certificate or counterexample (FR-201).
fn check_exact_unsupported(cases: &[Case]) {
    let mut wrong = Vec::new();
    for case in cases {
        let over = case.over;
        let unit = untimed_unit(
            "",
            &format!(
                "terminal Done using v on M when {{ true }};\nprobabilistic C0 using t0 on M over (x: M::{over}) under every scheduler on origin {{ probability >= 0.5 [ eventually holds(true) ] }}\n"
            ),
        );
        let item = json!({"claim": "C0", "probabilistic": {"evidence": "exact"}});
        let document = case.analyze(&unit, vec![item]).document();
        let item = &document["items"][0];
        if item["record"]["result"] != "unsupported" || !item["counterexample"].is_null() {
            wrong.push(format!("{}: {}", case.name, short(item)));
        }
    }
    assert_clean(wrong);
}

fn assert_clean(wrong: Vec<String>) {
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

fn check_route(route: Route, cases: &[Case]) {
    let mut wrong = Vec::new();
    for case in cases {
        let mut probes = case.probes();
        if case.instances().is_empty() {
            // A hyper body names its trace, and `Orphan` has no field.
            if matches!(
                route,
                Route::SingleExistential | Route::ForallForall | Route::ForallExists
            ) {
                probes = vec![(0, "x = x".to_owned())];
            }
        } else if matches!(route, Route::Liveness) {
            // Only the refuted probes: see the module note on liveness.
            probes.pop();
        }
        let claims: String = probes
            .iter()
            .map(|(i, p)| route.claim(&format!("C{i}"), case.over, p))
            .collect();
        let items = probes
            .iter()
            .map(|(i, _)| route.item(&format!("C{i}")))
            .collect();
        let document = case.analyze(&route.unit(&claims), items).document();
        wrong.extend(
            probes
                .iter()
                .enumerate()
                .filter_map(|(at, (i, _))| judge(case, *i, &document, at)),
        );
    }
    assert_clean(wrong);
}

#[derive(Clone, Copy)]
enum Probabilistic {
    Exact,
    Statistical,
}

/// The probabilistic routes list every binding they decided: the exact
/// route in its basis, the statistical one in its tests.
fn check_probabilistic(route: Probabilistic, cases: &[Case]) {
    let mut wrong = Vec::new();
    for case in cases {
        let n = case.instances().len();
        let over = case.over;
        let p = if n == 0 {
            "true".to_owned()
        } else {
            format!("x.age <= {n}")
        };
        let (unit, item) = match route {
            Probabilistic::Exact => (
                untimed_unit(
                    "",
                    &format!(
                        "terminal Done using v on M when {{ true }};\nprobabilistic C0 using t0 on M over (x: M::{over}) under every scheduler on origin {{ probability >= 0.5 [ eventually holds({p}) ] }}\n"
                    ),
                ),
                json!({"claim": "C0", "probabilistic": {"evidence": "exact"}}),
            ),
            Probabilistic::Statistical => {
                let make = if case.make {
                    " weight Animal::make = 1;"
                } else {
                    ""
                };
                (
                    untimed_unit(
                        "profile ev = \"filament.temporal.event-position.false-extension/v1\";\n",
                        &format!(
                            "terminal Done using v on M when {{ true }};\nworkload Even on M {{ weight Animal::idle = 1;{make} }}\nprobabilistic C0 using ev on M over (x: M::{over}) under Even on origin with alpha 0.1, beta 0.1, indifference 0.05 {{ probability >= 0.9 [ eventually[0,1] holds({p}) ] }}\n"
                        ),
                    ),
                    json!({"claim": "C0", "probabilistic": {"evidence": "statistical", "seed": "7"}}),
                )
            }
        };
        let document = case.analyze(&unit, vec![item]).document();
        let Expect::Instances(expected) = case.expect else {
            wrong.extend(judge(case, 0, &document, 0));
            continue;
        };
        let item = &document["items"][0];
        let entries = match route {
            Probabilistic::Exact => &item["record"]["basis"]["entries"],
            Probabilistic::Statistical => &item["verdict"]["provenance"]["tests"],
        };
        let listed: Vec<&str> = entries
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(|entry| {
                entry["binding"]["identity"]
                    .as_str()
                    .or_else(|| entry["binding"]["key"]["identity"].as_str())
            })
            .collect();
        if listed != expected {
            wrong.push(format!(
                "{}: expected {expected:?}, listed {listed:?}, item {}",
                case.name,
                short(item)
            ));
        }
    }
    assert_clean(wrong);
}

/// Global protocol conformance over a protocol whose first control checks
/// the bound object's age (FR-896 "The objects checked over every
/// behaviour").
fn check_conformance(cases: &[Case]) {
    let mut wrong = Vec::new();
    for case in cases {
        let over = case.over;
        for (i, p) in case.probes() {
            let p = p.replace("x.", "k.");
            let unit = untimed_unit(
                "profile pp = \"filament.protocol.finite-global/v1\";\n",
                &format!(
                    "record Ack {{ ok: Boolean; }}\n\
                     protocol Flow using pp over (k: M::{over}) on origin {{\n\
                     role Buyer on M::Other;\nrole Seller on M::Other;\n\
                     channel Order from Buyer to Seller carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\n\
                     run sequence Main {{\n\
                     check Ready using v {{ {p} }};\n\
                     send Placed via Order as (placed: Ack) {{ true }};\n\
                     receive Got via Order of Placed as (got: Ack) {{ true }};\n\
                     }}\n\
                     finish Done2 as (zz: Boolean) {{ true }};\n}}\n"
                ),
            );
            let item = json!({"claim": "Flow", "kind": "global-conformance"});
            let document = case.analyze(&unit, vec![item]).document();
            if matches!(case.expect, Expect::Instances(_)) {
                let item = &document["items"][0];
                let result = &item["record"]["result"];
                let right = match case.instances().get(i) {
                    // FR-896-AC-7: the counterexample's protocol subject
                    // names the blocked key.
                    Some(key) => {
                        result == "refuted"
                            && item["counterexample"]
                                .to_string()
                                .contains(&format!("\"identity\":\"{key}\""))
                    }
                    None => result == "proved",
                };
                if !right {
                    wrong.push(format!(
                        "{} probe {i}: {} {}",
                        case.name,
                        short(item),
                        short(&document["diagnostics"])
                    ));
                }
            } else {
                wrong.extend(judge(case, i, &document, 0));
            }
        }
    }
    assert_clean(wrong);
}

/// A checked package of `body` with the case's model, snapshots and
/// universes, for the routes and checkers driven as a library.
struct Library {
    world: World,
    package: CheckedPackage,
    universes: Vec<(String, Vec<String>)>,
}

impl Case {
    fn library(&self, body: &str) -> Library {
        let document = self.model();
        let body = format!("terminal Done using v on M when {{ true }};\n{body}");
        let check = |body: &str| {
            CheckedPackage::link(
                declarations_in(PACKAGE, &document, body)
                    .check(
                        CheckMode::Linked,
                        CheckingLimits::default(),
                        &fsl_eval::value::S6aRewardEvaluator,
                    )
                    .unwrap_or_else(|refusals| panic!("{body}: the unit checks: {refusals:?}")),
            )
        };
        let package = check(&body);
        let bytes = self.snapshot("s0", &self.held());
        let initial = self.initial.then(|| DocumentRef {
            authority: "test".to_owned(),
            identity: "s0".to_owned(),
        });
        let world = World {
            packages: package_input([document.as_slice()]).expect("a package input"),
            snapshots: BTreeMap::from([(
                document_reference(&bytes).expect("a document names itself"),
                bytes,
            )]),
            initial: initial.into_iter().collect(),
            package: check(""),
            document: document.clone(),
        };
        Library {
            world,
            package,
            universes: self
                .listed()
                .into_iter()
                .map(|(population, keys)| {
                    (
                        population.to_owned(),
                        keys.iter().map(|key| (*key).to_owned()).collect(),
                    )
                })
                .collect(),
        }
    }
}

impl Library {
    fn universes(&self) -> Vec<PopulationUniverse> {
        self.universes
            .iter()
            .flat_map(|(population, keys)| {
                let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
                universes(&[(population.as_str(), keys.as_slice())])
            })
            .collect()
    }

    fn subject(&self) -> ModelSubject<'_> {
        self.world.subject(&self.package, self.universes())
    }
}

/// Whether a library refusal, or an outcome, spells the case's cause.
fn spells(expect: Expect, seen: &str) -> bool {
    match expect {
        Expect::Instances(_) => false,
        Expect::NoInstance => seen.contains("NoInstance"),
        Expect::NoInitialState => seen.contains("NoInitialState"),
        Expect::Conflict => seen.contains("conflicting-identity"),
        Expect::RequiresBound => seen.contains("RequiresBound"),
    }
}

/// EN-1 as a library, and the core's closure or component check of the
/// certificate it writes.
fn check_certificates(formula: &str, cases: &[Case]) {
    let mut wrong = Vec::new();
    for case in cases {
        let n = case.instances().len();
        let over = case.over;
        let p = if n == 0 {
            "true".to_owned()
        } else {
            format!("x.age <= {n}")
        };
        let library = case.library(&format!(
            "temporal C using t0 over (x: M::{over}) clock \"model-steps\" on origin {{ {formula} holds({p}) }}\n"
        ));
        let node = &library.package.graph().temporal_clauses()[0];
        let system = match ModelSystem::new(library.subject().simulation) {
            Ok(system) => system,
            Err(failure) => {
                let seen = format!("{failure:?}");
                if !spells(case.expect, &seen) {
                    wrong.push(format!("{} admission: {seen}", case.name));
                }
                continue;
            }
        };
        let letters = system.clause_letters(node).expect("an unguarded clause");
        let item = || ModelCheckItem::Clause {
            clause: node.clause(),
            letters: &letters,
        };
        let outcome: Result<ModelCheckOutcome, ModelCheckRefusal> = check_model(
            ModelCheckRequest {
                workers: 1,
                temporal_clock_selections: &[],
                subject: library.subject(),
                item: item(),
                max_depth: None,
                limits: ModelCheckLimits::default(),
                smt_encoding: Default::default(),
                smt_run: Default::default(),
                reductions: Default::default(),
            },
            &Cancel::new(),
        );
        let request = CertificateRequest {
            temporal_clock_selections: &[],
            subject: library.subject(),
            item: item(),
            smt_encoding: Default::default(),
        };
        let limits = ModelCheckLimits::default();
        let (certificate, engine) = match outcome {
            Ok(ModelCheckOutcome::Holds { certificate, .. }) => (Some(certificate), String::new()),
            other => (None, format!("{other:?}")),
        };
        let checked = match &certificate {
            Some(ProofCertificate::Component(proof)) => {
                check_components(&request, proof, &limits, &Cancel::new())
            }
            Some(ProofCertificate::Closure(proof)) => {
                check_closure(&request, proof, &limits, &Cancel::new())
            }
            None => check_closure(
                &request,
                &ClosureCertificate { states: Vec::new() },
                &limits,
                &Cancel::new(),
            ),
        };
        let checked = format!("{checked:?}");
        let right = match case.expect {
            Expect::Instances(_) => certificate.is_some() && checked == "Ok(Accepted)",
            _ => spells(case.expect, &engine) && spells(case.expect, &checked),
        };
        if !right {
            wrong.push(format!(
                "{}: engine {} checker {}",
                case.name,
                engine.chars().take(200).collect::<String>(),
                checked.chars().take(200).collect::<String>()
            ));
        }
    }
    assert_clean(wrong);
}

/// The state-graph engine as a library lists each instance, and the core
/// checks the certificate of each.
fn check_state_graph_library(cases: &[Case]) {
    let mut wrong = Vec::new();
    for case in cases {
        let n = case.instances().len();
        let over = case.over;
        let p = if n == 0 {
            "true".to_owned()
        } else {
            format!("x.age <= {n}")
        };
        let library = case.library(&format!(
            "possible C using v over (x: M::{over}) {{ {p} }}\n"
        ));
        let claim = &library.package.graph().state_graph_claims()[0];
        let run = check_state_graph(
            StateGraphRequest {
                subject: library.subject(),
                temporal_clock_selections: Vec::new(),
                items: vec![StateGraphItem::new(claim)],
                reductions: Default::default(),
                deadlock_freedom: false,
                max_depth: None,
                limits: ModelCheckLimits::default(),
                seed: None,
            },
            &Cancel::new(),
        );
        let run = match run {
            Ok(run) => run,
            Err(refusal) => {
                let seen = format!("{refusal:?}");
                if !spells(case.expect, &seen) {
                    wrong.push(format!("{}: {seen}", case.name));
                }
                continue;
            }
        };
        let instances = &run.items[0];
        let Expect::Instances(expected) = case.expect else {
            let seen = format!(
                "{:?}",
                instances.iter().map(|i| &i.outcome).collect::<Vec<_>>()
            );
            if instances.len() != 1 || !spells(case.expect, &seen) {
                wrong.push(format!("{}: {seen}", case.name));
            }
            continue;
        };
        let listed: Vec<&str> = instances
            .iter()
            .filter_map(|instance| instance.over.as_ref())
            .map(|reference| reference.object().as_str())
            .collect();
        if listed != expected {
            wrong.push(format!("{}: listed {listed:?}", case.name));
        }
        for instance in instances {
            let StateGraphOutcome::Witnessed { certificate, .. } = &instance.outcome else {
                wrong.push(format!("{}: {:?}", case.name, instance.outcome));
                continue;
            };
            let checked = fsl_replay::check_state_graph(
                &StateGraphCertificateRequest {
                    subject: library.subject(),
                    claim,
                    over: instance.over.clone(),
                },
                certificate,
                &ModelCheckLimits::default(),
                &Cancel::new(),
            );
            let checked = format!("{checked:?}");
            if checked != "Ok(Accepted)" {
                wrong.push(format!("{}: checker {checked}", case.name));
            }
        }
    }
    assert_clean(wrong);
}

/// FR-1012 plan policies: one monitor per admitted key, and a plan keeps the
/// policy only when every monitor accepts it.
fn check_plan_policies(cases: &[Case]) {
    let mut wrong = Vec::new();
    for case in cases {
        let over = case.over;
        for (i, p) in case.probes() {
            let library = case.library(&format!(
                "type Lengths = Int[0,1];\n\
                 synthesis Reach using t0 grammar M::Animal domain Lengths satisfies {{ true }};\n\
                 temporal C using t0 over (x: M::{over}) clock \"model-steps\" on origin {{ always holds({p}) }}\n"
            ));
            let seen = (|| {
                let system = ModelSystem::over_model(library.subject().simulation, "M")
                    .map_err(|failure| format!("{failure:?}"))?;
                let node = &library.package.graph().temporal_clauses()[0];
                let synthesis = &library.package.graph().syntheses()[0];
                let mut rules =
                    SourcePlan::new(&library.package, synthesis, &system, vec![node], UNLIMITED)
                        .map_err(|error| format!("{error:?}"))?;
                search(
                    &system,
                    &mut rules,
                    PlanSearchRequest {
                        lengths: PlanLengths::new(0, 1).expect("ordered"),
                        limits: PlanSearchLimits::default(),
                        cancel: &Cancel::new(),
                        fault: None,
                    },
                )
                .map(|result| result.status)
                .map_err(|error| format!("{error:?}"))
            })();
            let wanted = if i < case.instances().len() {
                PlanStatus::Infeasible
            } else {
                PlanStatus::Optimal
            };
            if seen != Ok(wanted) {
                wrong.push(format!(
                    "{} probe {i}: expected {wanted:?}, got {seen:?}",
                    case.name
                ));
            }
        }
    }
    assert_clean(wrong);
}

/// The SMT temporal encoder: each case with the refusal FR-315's ordered
/// table gives it, or `None` where it writes a script. No solver is run, so
/// the instance list itself is not read back.
fn check_smt_temporal(cases: &[(Case, Option<&str>)]) {
    let mut wrong = Vec::new();
    for (case, refusal) in cases {
        let over = case.over;
        let p = if case.instances().is_empty() {
            "true"
        } else {
            "x.age <= 9"
        };
        let library = case.library(&format!(
            "temporal C using t0 over (x: M::{over}) clock \"model-steps\" on origin {{ always holds({p}) }}\n"
        ));
        let encoded = encode_temporal_smt_query(
            &library.subject(),
            library.package.graph().temporal_clauses()[0].clause(),
            SmtQueryKind::InductionBase { depth: 1 },
            &SmtEncodingLimits::default(),
        );
        let seen = format!("{:?}", encoded.map(|script| script.len()));
        let right = match refusal {
            Some(name) => seen.starts_with("Err(") && seen.contains(name),
            None => seen.starts_with("Ok("),
        };
        if !right {
            wrong.push(format!("{}: expected {refusal:?}, got {seen}", case.name));
        }
    }
    assert_clean(wrong);
}

/// The SMT hyper encoder takes one instance: each admitted key encodes and
/// each excluded key is refused. The product-closure checker refuses an
/// empty set.
fn check_hyper_library(cases: &[Case]) {
    let mut wrong = Vec::new();
    for case in cases {
        let over = case.over;
        let p = if case.instances().is_empty() {
            "x = x".to_owned()
        } else {
            "x.age <= 9".to_owned()
        };
        let library = case.library(&Route::ForallForall.claim("C", over, &p));
        let clause = &library.package.graph().hyper_clauses()[0];
        let subjects = vec![library.subject(), library.subject()];
        let mut keys: Vec<&str> = case
            .listed
            .iter()
            .flat_map(|(_, keys)| keys.iter().copied())
            .collect();
        keys.sort_unstable();
        keys.dedup();
        for key in keys {
            let mut meter = Meter::new(UNLIMITED);
            let encoded = encode_hyper_query(
                HyperQueryRequest {
                    clause,
                    subjects: &subjects,
                    instance: &[("x".to_owned(), key.to_owned())],
                    meter: &mut meter,
                    encoding: SmtEncodingLimits::default(),
                },
                SmtQueryKind::InductionBase { depth: 1 },
            );
            if encoded.is_ok() != case.instances().contains(&key) {
                wrong.push(format!(
                    "{} smt key {key}: {:?}",
                    case.name,
                    encoded.map(|query| query.script.len())
                ));
            }
        }
        if case.instances().is_empty() {
            let checked = check_product_closure(
                &ProductClosureRequest {
                    package: &library.package,
                    clause: ProductClosureClause::Hyper(clause),
                    subjects: &subjects,
                    meter: UNLIMITED,
                },
                &ProductClosureCertificate {
                    form: HyperForm::Universal,
                    instance: Vec::new(),
                    reduction: None,
                    closure: ProductClosure::Product(Vec::new()),
                    components: None,
                },
                &ModelCheckLimits::default(),
                &Cancel::new(),
            );
            let seen = format!("{checked:?}");
            if !spells(case.expect, &seen) {
                wrong.push(format!(
                    "{} product checker: {}",
                    case.name,
                    seen.chars().take(200).collect::<String>()
                ));
            }
        }
    }
    assert_clean(wrong);
}

#[trace("TC-520", "FR-125-AC-8", "FR-125-AC-10", "FR-125-AC-14")]
#[test]
fn temporal_ranges_over_the_admitted_keys() {
    check_route(Route::Temporal, INSTANCES);
}

#[trace("TC-520", "FR-125-AC-7", "FR-125-AC-13", "FSpec-FR-361-AC-8")]
#[trace("TC-520", "FSpec-FR-361-AC-14", "FSpec-FR-361-AC-16")]
#[test]
fn temporal_names_why_there_is_no_instance() {
    check_route(Route::Temporal, CAUSES);
}

#[trace("TC-520", "FR-125-AC-8", "FR-125-AC-10", "FR-125-AC-14")]
#[test]
fn liveness_binds_the_instances_in_order() {
    check_route(Route::Liveness, INSTANCES);
}

#[trace("TC-520", "FR-125-AC-7", "FR-125-AC-13", "FSpec-FR-361-AC-8")]
#[trace("TC-520", "FSpec-FR-361-AC-14", "FSpec-FR-361-AC-16")]
#[test]
fn liveness_names_why_there_is_no_instance() {
    check_route(Route::Liveness, CAUSES);
}

#[trace("TC-520", "FR-125-AC-8", "FR-125-AC-10", "FR-125-AC-14")]
#[test]
fn state_graph_ranges_over_the_admitted_keys() {
    check_route(Route::StateGraph, INSTANCES);
}

#[trace("TC-520", "FR-125-AC-7", "FR-125-AC-13", "FSpec-FR-361-AC-8")]
#[trace("TC-520", "FSpec-FR-361-AC-14", "FSpec-FR-361-AC-16")]
#[test]
fn state_graph_names_why_there_is_no_instance() {
    check_route(Route::StateGraph, CAUSES);
}

#[trace("TC-520", "FR-125-AC-8", "FR-125-AC-10", "FR-125-AC-14")]
#[test]
fn state_graph_library_lists_the_instances_its_checker_accepts() {
    check_state_graph_library(INSTANCES);
}

#[trace("TC-520", "FR-125-AC-7", "FR-125-AC-13")]
#[test]
fn state_graph_library_names_why_there_is_no_instance() {
    check_state_graph_library(CAUSES);
}

#[trace("TC-520", "FR-125-AC-8", "FR-125-AC-10", "FR-125-AC-14")]
#[test]
fn closure_checker_accepts_the_certificate_over_the_instances() {
    check_certificates("always", INSTANCES);
}

#[trace("TC-520", "FR-125-AC-7", "FR-125-AC-13")]
#[test]
fn closure_checker_names_why_there_is_no_instance() {
    check_certificates("always", CAUSES);
}

#[trace("TC-520", "FR-125-AC-7", "FR-125-AC-13")]
#[test]
fn component_checker_names_why_there_is_no_instance() {
    check_certificates("eventually", CAUSES);
}

#[trace("TC-3219", "FR-125-AC-19", "FR-172-AC-10")]
#[test]
fn hp5_ranges_over_a_subtype_population() {
    check_route(Route::SingleExistential, &[SUBTYPE_ONLY]);
}

#[trace("TC-3219", "FR-125-AC-19", "FR-125-AC-20")]
#[test]
#[ignore = "QSL-3408: HP-5 instances universe keys no initial state admits (FR-125-AC-19, AC-20)"]
fn hp5_excludes_unadmitted_keys() {
    check_route(Route::SingleExistential, &[UNADMITTED_KEY, CREATED]);
}

#[trace("TC-3219", "FR-125-AC-19", "FR-172-AC-10")]
#[test]
#[ignore = "QSL-3408: a hyper parameter over several populations is refused ambiguous (FR-172-AC-10)"]
fn hp5_ranges_over_several_populations() {
    check_route(
        Route::SingleExistential,
        &[UNADMITTED_ACROSS, TWO_POPULATIONS],
    );
}

#[trace("TC-3219", "FR-125-AC-21", "FR-172-AC-11")]
#[test]
fn hp5_names_why_there_is_no_instance() {
    check_route(Route::SingleExistential, CAUSES);
}

#[trace("TC-3219", "FR-125-AC-19", "FR-172-AC-10")]
#[test]
fn hyper_ranges_over_a_subtype_population() {
    check_route(Route::ForallForall, &[SUBTYPE_ONLY]);
    check_route(Route::ForallExists, &[SUBTYPE_ONLY]);
}

#[trace("TC-3219", "FR-125-AC-19", "FR-125-AC-20")]
#[test]
#[ignore = "QSL-3408: hyper clauses instance universe keys no initial state admits (FR-125-AC-19, AC-20)"]
fn hyper_excludes_unadmitted_keys() {
    check_route(Route::ForallForall, &[UNADMITTED_KEY, CREATED]);
    check_route(Route::ForallExists, &[UNADMITTED_KEY, CREATED]);
}

#[trace("TC-3219", "FR-125-AC-19", "FR-172-AC-10")]
#[test]
#[ignore = "QSL-3408: a hyper parameter over several populations is refused ambiguous (FR-172-AC-10)"]
fn hyper_ranges_over_several_populations() {
    check_route(Route::ForallForall, &[UNADMITTED_ACROSS, TWO_POPULATIONS]);
    check_route(Route::ForallExists, &[UNADMITTED_ACROSS, TWO_POPULATIONS]);
}

#[trace("TC-3219", "FR-125-AC-21", "FR-172-AC-11")]
#[test]
fn hyper_names_why_there_is_no_instance() {
    check_route(Route::ForallForall, CAUSES);
    check_route(Route::ForallExists, CAUSES);
}

#[trace("TC-3219", "FR-125-AC-19", "FR-125-AC-20", "FR-125-AC-21")]
#[test]
fn hyper_smt_and_product_checker_take_only_instances() {
    check_hyper_library(INSTANCES);
    check_hyper_library(CAUSES);
}

#[trace("TC-520", "FR-125-AC-8", "FR-125-AC-14")]
#[test]
fn zone_ranges_over_the_admitted_keys() {
    check_route(Route::Zone, TIMED_INSTANCES);
    check_route(Route::Zone, &[TIMED_TWO_POPULATIONS]);
}

#[trace("TC-520", "FR-125-AC-13", "FSpec-FR-361-AC-16")]
#[test]
fn zone_names_the_causes_before_instancing() {
    check_route(Route::Zone, TIMED_EARLIER_CAUSES);
}

#[trace("TC-520", "FR-125-AC-7", "FSpec-FR-361-AC-8")]
#[test]
#[ignore = "QSL-3484: en-6 proves a claim over no instance (FR-125-AC-7)"]
fn zone_reports_no_instance() {
    check_route(Route::Zone, TIMED_NO_INSTANCE);
}

#[trace("TC-520", "FR-125-AC-8", "FR-125-AC-14")]
#[test]
fn digital_ranges_over_the_admitted_keys() {
    check_route(Route::Digital, TIMED_INSTANCES);
    check_route(Route::Digital, &[TIMED_TWO_POPULATIONS]);
}

#[trace("TC-520", "FR-125-AC-13", "FSpec-FR-361-AC-16")]
#[test]
fn digital_names_the_causes_before_instancing() {
    check_route(Route::Digital, TIMED_EARLIER_CAUSES);
}

#[trace("TC-520", "FR-125-AC-7", "FSpec-FR-361-AC-8")]
#[test]
#[ignore = "QSL-3484: en-1 digital clocks proves a claim over no instance (FR-125-AC-7)"]
fn digital_reports_no_instance() {
    check_route(Route::Digital, TIMED_NO_INSTANCE);
}

#[trace("TC-520", "FR-125-AC-8", "FR-125-AC-14")]
#[test]
fn timed_state_graph_ranges_over_the_admitted_keys() {
    check_route(Route::TimedStateGraph, TIMED_INSTANCES);
}

#[trace("TC-520", "FR-125-AC-7", "FR-125-AC-13", "FSpec-FR-361-AC-8")]
#[trace("TC-520", "FSpec-FR-361-AC-16")]
#[test]
fn timed_state_graph_names_why_there_is_no_instance() {
    check_route(Route::TimedStateGraph, TIMED_NO_INSTANCE);
    check_route(Route::TimedStateGraph, TIMED_EARLIER_CAUSES);
}

#[trace("TC-520", "FR-125-AC-8")]
#[test]
fn exact_binds_the_one_admitted_key() {
    check_probabilistic(Probabilistic::Exact, &[UNADMITTED_KEY]);
}

#[trace("TC-636", "FR-201-AC-30")]
#[test]
fn exact_is_unsupported_over_no_instance() {
    check_exact_unsupported(&[EMPTY_UNIVERSE, ONLY_UNADMITTED, ORPHAN]);
}

#[trace("TC-520", "FR-125-AC-13", "FSpec-FR-361-AC-14")]
#[test]
fn exact_refuses_before_instancing() {
    check_probabilistic(Probabilistic::Exact, &[NO_UNIVERSE, CONFLICT]);
}

#[trace("TC-520", "FR-125-AC-8", "FR-125-AC-10", "FR-125-AC-14")]
#[test]
fn statistical_tests_each_admitted_key() {
    check_probabilistic(Probabilistic::Statistical, INSTANCES);
}

#[trace("TC-520", "FR-125-AC-13", "FSpec-FR-361-AC-14", "FSpec-FR-361-AC-16")]
#[test]
fn statistical_names_the_causes_before_instancing() {
    check_probabilistic(
        Probabilistic::Statistical,
        &[
            NO_INITIAL,
            EMPTY_UNIVERSE_NO_INITIAL,
            ORPHAN_NO_INITIAL,
            NO_UNIVERSE,
            CONFLICT,
        ],
    );
}

#[trace("TC-2532", "FR-896-AC-7")]
#[test]
fn conformance_checks_each_admitted_key() {
    check_conformance(INSTANCES);
}

#[trace("TC-520", "FR-125-AC-7", "FR-125-AC-13", "FSpec-FR-361-AC-8")]
#[trace("TC-520", "FSpec-FR-361-AC-14", "FSpec-FR-361-AC-16")]
#[test]
#[ignore = "QSL-3507: global conformance fails established-invariant-broken over no instance (FR-125-AC-13)"]
fn conformance_names_why_there_is_no_instance() {
    check_conformance(CAUSES);
}

#[trace("TC-2862", "FR-1012-AC-8")]
#[test]
fn plan_policy_monitors_each_admitted_key() {
    check_plan_policies(&[SUBTYPE_ONLY, TWO_POPULATIONS]);
}

#[trace("TC-2862", "FR-1012-AC-8")]
#[test]
#[ignore = "QSL-3323: plan policy monitors universe keys no initial state admits (FR-1012-AC-8)"]
fn plan_policy_monitors_no_excluded_key() {
    check_plan_policies(&[UNADMITTED_KEY, UNADMITTED_ACROSS]);
}

#[trace("TC-900", "FR-315-AC-21")]
#[test]
fn smt_temporal_refuses_in_its_table_order() {
    check_smt_temporal(&[
        (EMPTY_UNIVERSE, Some("NoInstance")),
        (ORPHAN, Some("NoInstance")),
        (ONLY_UNADMITTED, Some("ObjectLifecycle")),
        (NO_INITIAL, Some("NoInitialState")),
        (EMPTY_UNIVERSE_NO_INITIAL, Some("NoInitialState")),
        (ORPHAN_NO_INITIAL, Some("NoInitialState")),
        (NO_UNIVERSE, Some("NoInitialState")),
        (CONFLICT, Some("conflicting-identity")),
        (UNADMITTED_KEY, Some("ObjectLifecycle")),
        (SUBTYPE_ONLY, None),
        (CREATED, Some("ObjectLifecycle")),
        (UNADMITTED_ACROSS, Some("ObjectLifecycle")),
        (TWO_POPULATIONS, Some("ConstantName")),
    ]);
}
