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

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use filament_exact::{Cancel, Meter, ObjectReference};
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
    encode_temporal_smt_query, CertificateRequest, ClosureCertificate, ComponentCertificate,
    HyperQueryRequest, ModelCheckItem, ModelCheckLimits, ModelCheckOutcome, ProductClosure,
    ProductClosureCertificate, ProductClosureClause, ProductClosureRequest, ProofCertificate,
    SmtEncodingLimits, SmtQueryKind, StateGraphCertificate, StateGraphCertificateRequest,
    StateGraphOutcome,
};
use fsl_semantics::check::{CheckedStateGraphClaim, HyperForm};
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
/// `CONFLICT` with a third overlapping population that has no universe: the
/// missing universe is reported before the conflicting records.
const CONFLICT_NO_UNIVERSE: Case = Case {
    name: "missing-universe-before-conflict",
    populations: &[("P1", &["Animal"]), ("P2", &["Dog"]), ("P3", &["Cat"])],
    held: &[
        ("P1", &[("c", "Cat")]),
        ("P2", &[("c", "Dog")]),
        ("P3", &[]),
    ],
    expect: Expect::RequiresBound,
    ..CONFLICT
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
    /// Liveness probed with the refuted claims only.
    LivenessOrder,
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
            Self::Liveness | Self::LivenessOrder => format!(
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
        } else if matches!(route, Route::LivenessOrder) {
            // The proved probe is `liveness_ranges_over_the_admitted_keys`.
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

/// Global protocol conformance over a protocol whose first send admits a
/// message only while the bound object's age passes the probe, so the
/// blocked key is read from the counterexample (FR-896 "The objects checked
/// over every behaviour"). A `check` condition cannot be used: FR-218
/// "Constructs" refuses one that reads model state.
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
                     send Placed via Order as (placed: Ack) {{ {p} }};\n\
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
                    // The counterexample's protocol subject
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

    fn state_graph<'a>(&'a self, claim: &'a CheckedStateGraphClaim) -> StateGraphRequest<'a> {
        StateGraphRequest {
            subject: self.subject(),
            temporal_clock_selections: Vec::new(),
            items: vec![StateGraphItem::new(claim)],
            reductions: Default::default(),
            deadlock_freedom: false,
            max_depth: None,
            limits: ModelCheckLimits::default(),
            seed: None,
        }
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

/// The records of the conflicting cases with `c` typed once, so that the
/// subject admits and the letters a checker request carries can be built.
const ONE_TYPE_HELD: &[Held<'static>] = &[("P1", &[("c", "Dog")]), ("P2", &[("c", "Dog")])];

impl Case {
    fn admitting(&self) -> Case {
        let mut admitting = *self;
        if matches!(self.expect, Expect::Conflict) {
            admitting.held = ONE_TYPE_HELD;
        }
        if self.name == CONFLICT_NO_UNIVERSE.name {
            admitting.held = &[
                ("P1", &[("c", "Dog")]),
                ("P2", &[("c", "Dog")]),
                ("P3", &[]),
            ];
            admitting.listed = &[("P1", &["c"]), ("P2", &["c"]), ("P3", &[])];
        }
        if self.listed.is_empty() {
            admitting.listed = BASE.listed;
        }
        admitting
    }

    /// The temporal clause `formula holds(p)` over this case, as a library.
    fn clause(&self, formula: &str, p: &str) -> Library {
        let over = self.over;
        self.library(&format!(
            "temporal C using t0 over (x: M::{over}) clock \"model-steps\" on origin {{ {formula} holds({p}) }}\n"
        ))
    }
}

/// EN-1 over `subject`, and the core's check of `offered`: of the component
/// checker for a liveness formula, of the closure checker otherwise. With
/// nothing offered the checker gets the empty certificate of its kind.
fn prove_and_check(
    subject: &Library,
    letters_from: &Library,
    liveness: bool,
    offered: Option<&ProofCertificate>,
) -> (Result<ModelCheckOutcome, ModelCheckRefusal>, String) {
    let node = &letters_from.package.graph().temporal_clauses()[0];
    let system = ModelSystem::new(letters_from.subject().simulation).expect("the letters admit");
    let letters = system.clause_letters(node).expect("an unguarded clause");
    let item = || ModelCheckItem::Clause {
        clause: node.clause(),
        letters: &letters,
    };
    let limits = ModelCheckLimits::default();
    let outcome = check_model(
        ModelCheckRequest {
            workers: 1,
            temporal_clock_selections: &[],
            subject: subject.subject(),
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
        subject: subject.subject(),
        item: item(),
        smt_encoding: Default::default(),
    };
    let checked = match offered {
        Some(ProofCertificate::Component(proof)) => {
            check_components(&request, proof, &limits, &Cancel::new())
        }
        Some(ProofCertificate::Closure(proof)) => {
            check_closure(&request, proof, &limits, &Cancel::new())
        }
        None if liveness => check_components(
            &request,
            &ComponentCertificate {
                closure: ClosureCertificate { states: Vec::new() },
                components: Vec::new(),
            },
            &limits,
            &Cancel::new(),
        ),
        None => check_closure(
            &request,
            &ClosureCertificate { states: Vec::new() },
            &limits,
            &Cancel::new(),
        ),
    };
    (outcome, format!("{checked:?}"))
}

/// Over a case with no instance, both the engine and the checker spell why.
fn check_certificate_causes(formula: &str, cases: &[Case]) {
    let liveness = formula != "always";
    let mut wrong = Vec::new();
    for case in cases {
        let subject = case.clause(formula, "true");
        let control = case.admitting().clause(formula, "true");
        let (engine, checked) = prove_and_check(&subject, &control, liveness, None);
        let engine = format!("{engine:?}");
        if !(spells(case.expect, &engine) && spells(case.expect, &checked)) {
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

/// Over a case with instances: the claim every instance meets is proved with
/// a certificate the checker accepts; each claim the `i`-th instance is the
/// first to break is refuted at it, and the checker does not accept that
/// certificate for a claim the `i`-th instance alone breaks.
fn check_certificate_instances(formula: &str, cases: &[Case]) {
    let liveness = formula != "always";
    let mut wrong = Vec::new();
    for case in cases {
        let instances = case.instances();
        let all = case.clause(formula, &format!("x.age <= {}", instances.len()));
        let (outcome, _) = prove_and_check(&all, &all, liveness, None);
        let Ok(ModelCheckOutcome::Holds { certificate, .. }) = outcome else {
            wrong.push(format!("{}: not proved: {outcome:?}", case.name));
            continue;
        };
        let (_, checked) = prove_and_check(&all, &all, liveness, Some(&certificate));
        if checked != "Ok(Accepted)" {
            wrong.push(format!("{}: checker {checked}", case.name));
        }
        for (i, key) in instances.iter().enumerate() {
            let ordered = case.clause(formula, &format!("x.age <= {i}"));
            let (outcome, _) = prove_and_check(&ordered, &ordered, liveness, None);
            // A claim only the `i`-th instance breaks, so a checker that does
            // not read it accepts.
            let only = case.clause(formula, &format!("x.age != {}", i + 1));
            let (_, checked) = prove_and_check(&only, &only, liveness, Some(&certificate));
            let bound = match &outcome {
                Ok(ModelCheckOutcome::Violated { counterexample, .. }) => counterexample
                    .over_binding
                    .as_ref()
                    .map(|binding| binding.key.as_str()),
                _ => None,
            };
            if bound != Some(*key) {
                wrong.push(format!("{} probe {i}: bound {bound:?}", case.name));
            }
            if checked == "Ok(Accepted)" {
                wrong.push(format!("{} probe {i}: the checker accepted", case.name));
            }
        }
    }
    assert_clean(wrong);
}

/// The state-graph engine as a library lists each instance, and the core
/// checks the certificate of each.
/// The reference the engine writes for the instance `a`, which a checker
/// request over a case with no instance names.
fn a_reference() -> ObjectReference {
    let library = BASE.library("possible C using v over (x: M::Animal) { true }\n");
    let run = check_state_graph(
        library.state_graph(&library.package.graph().state_graph_claims()[0]),
        &Cancel::new(),
    )
    .expect("the base case runs");
    run.items[0][0].over.clone().expect("a bound instance")
}

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
        let run = check_state_graph(library.state_graph(claim), &Cancel::new());
        if !matches!(case.expect, Expect::Instances(_)) {
            let checked = fsl_replay::check_state_graph(
                &StateGraphCertificateRequest {
                    subject: library.subject(),
                    claim,
                    over: Some(a_reference()),
                },
                &StateGraphCertificate {
                    closure: Vec::new(),
                    ranks: Vec::new(),
                    counts: Vec::new(),
                },
                &ModelCheckLimits::default(),
                &Cancel::new(),
            );
            let checked = format!("{checked:?}");
            if !spells(case.expect, &checked) {
                wrong.push(format!("{} checker: {checked}", case.name));
            }
        }
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
/// policy only when every monitor accepts it. Probe `i` is broken by the
/// `i`-th instance alone, so a monitor missing for it leaves the search
/// optimal; the last probe is broken by no instance.
fn check_plan_policies(cases: &[Case]) {
    let mut wrong = Vec::new();
    for case in cases {
        let n = case.instances().len();
        for i in 0..=n {
            let (p, wanted) = if i < n {
                (format!("x.age != {}", i + 1), PlanStatus::Infeasible)
            } else {
                ("x.age != 0".to_owned(), PlanStatus::Optimal)
            };
            let over = case.over;
            let library = case.library(&format!(
                "type Lengths = Int[0,1];\n\
                 synthesis Reach using t0 grammar M::Animal domain Lengths satisfies {{ true }};\n\
                 temporal C using t0 over (x: M::{over}) clock \"model-steps\" on origin {{ always holds({p}) }}\n"
            ));
            let seen = (|| {
                let system = ModelSystem::new(library.subject().simulation)
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

impl Case {
    /// A two-universal clause over this case, and its listed keys.
    fn hyper(&self) -> (Library, Vec<&'static str>) {
        let p = if self.instances().is_empty() {
            "x = x"
        } else {
            "x.age <= 9"
        };
        let library = self.library(&Route::ForallForall.claim("C", self.over, p));
        let mut keys: Vec<&str> = self
            .listed
            .iter()
            .flat_map(|(_, keys)| keys.iter().copied())
            .collect();
        keys.sort_unstable();
        keys.dedup();
        (library, keys)
    }
}

impl Library {
    /// The SMT hyper encoder's answer for the instance `key`.
    fn encode_hyper(&self, key: &str) -> String {
        // One subject for the one model alias, `M`.
        let subjects = vec![self.subject()];
        let mut meter = Meter::new(UNLIMITED);
        let encoded = encode_hyper_query(
            HyperQueryRequest {
                clause: &self.package.graph().hyper_clauses()[0],
                subjects: &subjects,
                instance: &[("x".to_owned(), key.to_owned())],
                meter: &mut meter,
                encoding: SmtEncodingLimits::default(),
            },
            SmtQueryKind::InductionBase { depth: 1 },
        );
        format!("{:?}", encoded.map(|query| query.script.len()))
    }
}

/// The SMT hyper encoder takes each admitted key as an instance. FR-315's
/// `object-lifecycle` row then refuses a universe holding an object that no
/// initial state holds, and the `constant-name` row two objects under one
/// key.
fn check_hyper_admitted_keys(cases: &[Case]) {
    let mut wrong = Vec::new();
    for case in cases {
        let (library, keys) = case.hyper();
        let all_held = keys.iter().all(|key| case.instances().contains(key));
        for &key in case.instances() {
            let seen = library.encode_hyper(key);
            let right = if case.name == "two-populations" {
                // `c` is held by both populations (the `constant-name` row).
                seen.contains("ConstantName")
            } else if all_held {
                seen.starts_with("Ok(")
            } else {
                seen.contains("ObjectLifecycle")
            };
            if !right {
                wrong.push(format!("{} smt key {key}: {seen}", case.name));
            }
        }
    }
    assert_clean(wrong);
}

/// A key that is not an instance is `instance-binding`, which precedes the
/// ordered table (FR-315-AC-59).
fn check_hyper_excluded_keys(cases: &[Case]) {
    let mut wrong = Vec::new();
    for case in cases {
        let (library, keys) = case.hyper();
        for key in keys
            .into_iter()
            .filter(|key| !case.instances().contains(key))
        {
            let seen = library.encode_hyper(key);
            if !seen.contains("InstanceBinding") {
                wrong.push(format!("{} smt key {key}: {seen}", case.name));
            }
        }
    }
    assert_clean(wrong);
}

impl Library {
    /// The product-closure checker's answer to an empty closure certificate
    /// that names the instance `key`.
    fn check_product(&self, key: &str) -> String {
        let subjects = vec![self.subject()];
        let checked = check_product_closure(
            &ProductClosureRequest {
                package: &self.package,
                clause: ProductClosureClause::Hyper(&self.package.graph().hyper_clauses()[0]),
                subjects: &subjects,
                meter: UNLIMITED,
            },
            &ProductClosureCertificate {
                form: HyperForm::Universal,
                instance: vec![("x".to_owned(), key.to_owned())],
                reduction: None,
                closure: ProductClosure::Product(Vec::new()),
                components: Some(Vec::new()),
            },
            &ModelCheckLimits::default(),
            &Cancel::new(),
        );
        format!("{checked:?}")
    }
}

/// A certificate naming a key that is not an instance is refused: for the key,
/// or over an empty `initial` list for that (FR-125 "Instancing").
fn check_product_refuses_non_instances(cases: &[Case]) {
    let mut wrong = Vec::new();
    for case in cases {
        let (library, mut keys) = case.hyper();
        if keys.is_empty() {
            keys.push("a");
        }
        let refusal = if matches!(case.expect, Expect::NoInitialState) {
            "Input(\"no-initial-state\")"
        } else {
            "Input(\"instance-key\")"
        };
        for key in keys
            .into_iter()
            .filter(|key| !case.instances().contains(key))
        {
            let seen = library.check_product(key);
            if !seen.contains(refusal) {
                wrong.push(format!("{} product key {key}: {seen}", case.name));
            }
        }
    }
    assert_clean(wrong);
}

/// A certificate naming an instance gets past the checker's instance checks.
fn check_product_takes_instances(cases: &[Case]) {
    let mut wrong = Vec::new();
    for case in cases {
        let (library, _) = case.hyper();
        for &key in case.instances() {
            let seen = library.check_product(key);
            if seen.starts_with("Err(Input(") {
                wrong.push(format!("{} product key {key}: {seen}", case.name));
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

#[trace("TC-520", "FR-125-AC-13", "FR-125-AC-23")]
#[test]
#[ignore = "QSL-3514: admission refuses conflicting-identity before the missing universe (FR-125-AC-23)"]
fn temporal_names_a_missing_universe_before_a_conflict() {
    check_route(Route::Temporal, &[CONFLICT_NO_UNIVERSE]);
}

// The refuted probes only. No criterion is tagged: that no further instance
// follows is not shown here, but in the row below.
#[trace("TC-520")]
#[test]
fn liveness_binds_the_instances_in_order() {
    check_route(Route::LivenessOrder, INSTANCES);
}

#[trace("TC-520", "FR-125-AC-8", "FR-125-AC-10", "FR-125-AC-14")]
#[test]
#[ignore = "QSL-3529: a liveness claim every instance meets fails instead of being proved (FR-125-AC-8)"]
fn liveness_ranges_over_the_admitted_keys() {
    check_route(Route::Liveness, INSTANCES);
}

#[trace("TC-520", "FR-125-AC-7", "FR-125-AC-13", "FSpec-FR-361-AC-8")]
#[trace("TC-520", "FSpec-FR-361-AC-14", "FSpec-FR-361-AC-16")]
#[test]
fn liveness_names_why_there_is_no_instance() {
    check_route(Route::LivenessOrder, CAUSES);
}

#[trace("TC-520", "FR-125-AC-13", "FR-125-AC-23")]
#[test]
#[ignore = "QSL-3514: admission refuses conflicting-identity before the missing universe (FR-125-AC-23)"]
fn liveness_names_a_missing_universe_before_a_conflict() {
    check_route(Route::LivenessOrder, &[CONFLICT_NO_UNIVERSE]);
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

#[trace("TC-520", "FR-125-AC-13", "FR-125-AC-23")]
#[test]
#[ignore = "QSL-3514: admission refuses conflicting-identity before the missing universe (FR-125-AC-23)"]
fn state_graph_names_a_missing_universe_before_a_conflict() {
    check_route(Route::StateGraph, &[CONFLICT_NO_UNIVERSE]);
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

#[trace("TC-520", "FR-125-AC-13", "FR-125-AC-23")]
#[test]
#[ignore = "QSL-3514: admission refuses conflicting-identity before the missing universe (FR-125-AC-23)"]
fn state_graph_library_names_a_missing_universe_before_a_conflict() {
    check_state_graph_library(&[CONFLICT_NO_UNIVERSE]);
}

#[trace("TC-520", "FR-125-AC-8", "FR-125-AC-10", "FR-125-AC-14")]
#[test]
fn closure_checker_reads_every_instance() {
    check_certificate_instances("always", INSTANCES);
}

#[trace("TC-520", "FR-125-AC-7", "FR-125-AC-13")]
#[test]
fn closure_checker_names_why_there_is_no_instance() {
    check_certificate_causes("always", CAUSES);
}

#[trace("TC-520", "FR-125-AC-13", "FR-125-AC-23")]
#[test]
#[ignore = "QSL-3514: admission refuses conflicting-identity before the missing universe (FR-125-AC-23)"]
fn closure_checker_names_a_missing_universe_before_a_conflict() {
    check_certificate_causes("always", &[CONFLICT_NO_UNIVERSE]);
}

#[trace("TC-520", "FR-125-AC-8", "FR-125-AC-10", "FR-125-AC-14")]
#[test]
#[ignore = "QSL-3529: the component checker faults on a certificate over the instances (FR-125-AC-8)"]
fn component_checker_reads_every_instance() {
    check_certificate_instances("eventually", INSTANCES);
}

#[trace("TC-520", "FR-125-AC-7", "FR-125-AC-13")]
#[test]
fn component_checker_names_why_there_is_no_instance() {
    check_certificate_causes("eventually", CAUSES);
}

#[trace("TC-520", "FR-125-AC-13", "FR-125-AC-23")]
#[test]
#[ignore = "QSL-3514: admission refuses conflicting-identity before the missing universe (FR-125-AC-23)"]
fn component_checker_names_a_missing_universe_before_a_conflict() {
    check_certificate_causes("eventually", &[CONFLICT_NO_UNIVERSE]);
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

#[trace("TC-3219", "FR-125-AC-21")]
#[test]
fn hp5_names_the_causes_before_instancing() {
    check_route(Route::SingleExistential, &[NO_INITIAL, NO_UNIVERSE]);
}

#[trace("TC-3219", "FR-125-AC-21")]
#[test]
#[ignore = "QSL-3366: a single-existential clause over no instance fails established-invariant-broken (FR-125-AC-21)"]
fn hp5_reports_an_empty_universe() {
    check_route(
        Route::SingleExistential,
        &[EMPTY_UNIVERSE, EMPTY_UNIVERSE_NO_INITIAL],
    );
}

#[trace("TC-3219", "FR-125-AC-21")]
#[test]
#[ignore = "QSL-3408: HP-5 instances a universe key no initial state admits instead of NoInstance (FR-125-AC-21)"]
fn hp5_reports_no_instance_over_unadmitted_keys() {
    check_route(Route::SingleExistential, &[ONLY_UNADMITTED]);
}

#[trace("TC-3219", "FR-172-AC-11")]
#[test]
#[ignore = "QSL-3408: a hyper parameter of a type no population overlaps is refused missing-name (FR-172-AC-11)"]
fn hp5_checks_a_type_no_population_overlaps() {
    check_route(Route::SingleExistential, &[ORPHAN, ORPHAN_NO_INITIAL]);
}

#[trace("TC-3219", "FR-125-AC-21")]
#[test]
#[ignore = "QSL-3408: conflicting member records are refused ambiguous before conflicting-identity (FR-125-AC-21)"]
fn hp5_refuses_conflicting_identity() {
    check_route(Route::SingleExistential, &[CONFLICT]);
}

#[trace("TC-520", "FR-125-AC-13")]
#[test]
#[ignore = "QSL-3408: overlapping hyper populations are refused ambiguous before the missing universe (FR-125-AC-13)"]
fn hyper_names_a_missing_universe_before_a_conflict() {
    check_route(Route::SingleExistential, &[CONFLICT_NO_UNIVERSE]);
    check_route(Route::ForallForall, &[CONFLICT_NO_UNIVERSE]);
    check_route(Route::ForallExists, &[CONFLICT_NO_UNIVERSE]);
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

#[trace("TC-3219", "FR-125-AC-21")]
#[test]
fn hyper_names_the_causes_before_instancing() {
    let cases = &[
        EMPTY_UNIVERSE,
        NO_INITIAL,
        EMPTY_UNIVERSE_NO_INITIAL,
        NO_UNIVERSE,
    ];
    check_route(Route::ForallForall, cases);
    check_route(Route::ForallExists, cases);
}

#[trace("TC-3219", "FR-125-AC-21")]
#[test]
#[ignore = "QSL-3408: hyper clauses instance a universe key no initial state admits instead of NoInstance (FR-125-AC-21)"]
fn hyper_reports_no_instance_over_unadmitted_keys() {
    check_route(Route::ForallForall, &[ONLY_UNADMITTED]);
    check_route(Route::ForallExists, &[ONLY_UNADMITTED]);
}

#[trace("TC-3219", "FR-172-AC-11")]
#[test]
#[ignore = "QSL-3408: a hyper parameter of a type no population overlaps is refused missing-name (FR-172-AC-11)"]
fn hyper_checks_a_type_no_population_overlaps() {
    check_route(Route::ForallForall, &[ORPHAN, ORPHAN_NO_INITIAL]);
    check_route(Route::ForallExists, &[ORPHAN, ORPHAN_NO_INITIAL]);
}

#[trace("TC-3219", "FR-125-AC-21")]
#[test]
#[ignore = "QSL-3408: conflicting member records are refused ambiguous before conflicting-identity (FR-125-AC-21)"]
fn hyper_refuses_conflicting_identity() {
    check_route(Route::ForallForall, &[CONFLICT]);
    check_route(Route::ForallExists, &[CONFLICT]);
}

#[trace("TC-3219", "FR-125-AC-19", "FR-125-AC-20", "FR-315-AC-59")]
#[test]
fn hyper_smt_takes_the_admitted_keys() {
    check_hyper_admitted_keys(&[UNADMITTED_KEY, SUBTYPE_ONLY, CREATED]);
}

#[trace("TC-3219", "FR-125-AC-19", "FR-125-AC-20", "FR-315-AC-59")]
#[test]
#[ignore = "QSL-3408: the hyper SMT encoder binds a universe key no initial state admits (FR-315-AC-59)"]
fn hyper_smt_refuses_excluded_keys() {
    check_hyper_excluded_keys(&[ONLY_UNADMITTED, UNADMITTED_KEY, CREATED]);
}

#[trace("TC-3219", "FR-125-AC-19", "FR-315-AC-59")]
#[test]
#[ignore = "QSL-3408: a hyper parameter over several populations is refused AmbiguousPopulation (FR-125-AC-19)"]
fn hyper_smt_takes_only_instances_of_several_populations() {
    check_hyper_admitted_keys(&[UNADMITTED_ACROSS, TWO_POPULATIONS]);
    check_hyper_excluded_keys(&[UNADMITTED_ACROSS]);
}

#[trace("TC-3219", "FR-125-AC-21")]
#[test]
fn hyper_product_checker_refuses_a_key_where_there_is_no_instance() {
    check_product_refuses_non_instances(&[EMPTY_UNIVERSE, NO_INITIAL]);
}

#[trace("TC-3219", "FR-125-AC-21")]
#[test]
#[ignore = "QSL-3408: the product checker refuses the key before the empty initial list (FR-125-AC-21)"]
fn hyper_product_checker_names_no_initial_state_over_an_empty_universe() {
    check_product_refuses_non_instances(&[EMPTY_UNIVERSE_NO_INITIAL]);
}

#[trace("TC-3219", "FR-125-AC-21")]
#[test]
#[ignore = "QSL-3408: the product checker takes a universe key no initial state admits (FR-125-AC-21)"]
fn hyper_product_checker_refuses_excluded_keys() {
    check_product_refuses_non_instances(&[ONLY_UNADMITTED, UNADMITTED_KEY, CREATED]);
}

#[trace("TC-3219", "FR-125-AC-19")]
#[test]
fn hyper_product_checker_takes_instances() {
    check_product_takes_instances(&[SUBTYPE_ONLY]);
}

#[trace("TC-3219", "FR-125-AC-19")]
#[test]
#[ignore = "QSL-3408: the product checker refuses an instance of a parameter over several populations (FR-125-AC-19)"]
fn hyper_product_checker_takes_instances_of_several_populations() {
    check_product_takes_instances(&[UNADMITTED_ACROSS, TWO_POPULATIONS]);
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
#[ignore = "QSL-3484: en-1 digital clocks proves a claim over no instance when no state is initial (FR-125-AC-13)"]
fn digital_names_no_initial_state() {
    check_route(
        Route::Digital,
        &[single(NO_INITIAL), single(EMPTY_UNIVERSE_NO_INITIAL)],
    );
}

#[trace("TC-520", "FR-125-AC-13")]
#[test]
#[ignore = "QSL-3492: en-1 digital clocks fails established-invariant-broken for a missing universe (FR-125-AC-13)"]
fn digital_names_a_missing_universe() {
    check_route(Route::Digital, &[single(NO_UNIVERSE)]);
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

#[trace("TC-520", "FR-125-AC-8")]
#[test]
#[ignore = "QSL-3497: a timed state-graph claim over two populations of one type is refused ambiguous (FR-125-AC-8)"]
fn timed_state_graph_ranges_over_two_populations() {
    check_route(Route::TimedStateGraph, &[TIMED_TWO_POPULATIONS]);
}

#[trace("TC-520", "FR-125-AC-7", "FR-125-AC-13", "FSpec-FR-361-AC-8")]
#[trace("TC-520", "FSpec-FR-361-AC-16")]
#[test]
fn timed_state_graph_names_why_there_is_no_instance() {
    check_route(Route::TimedStateGraph, TIMED_NO_INSTANCE);
    check_route(
        Route::TimedStateGraph,
        &[single(NO_INITIAL), single(EMPTY_UNIVERSE_NO_INITIAL)],
    );
}

#[trace("TC-520", "FR-125-AC-13")]
#[test]
#[ignore = "QSL-3492: the timed state graph fails established-invariant-broken for a missing universe (FR-125-AC-13)"]
fn timed_state_graph_names_a_missing_universe() {
    check_route(Route::TimedStateGraph, &[single(NO_UNIVERSE)]);
}

#[trace("TC-520", "FR-125-AC-8")]
#[test]
fn exact_binds_the_one_admitted_key() {
    check_probabilistic(Probabilistic::Exact, &[UNADMITTED_KEY]);
}

#[trace("TC-3241", "FR-203-AC-34")]
#[test]
#[ignore = "QSL-3514: exact settles unsupported over no instance instead of no-instance (FR-203-AC-34)"]
fn exact_reports_no_instance() {
    check_probabilistic(
        Probabilistic::Exact,
        &[EMPTY_UNIVERSE, ONLY_UNADMITTED, ORPHAN],
    );
}

#[trace("TC-3241", "FR-203-AC-20")]
#[test]
#[ignore = "QSL-3514: exact settles unsupported over an empty initial list (FR-203-AC-20)"]
fn exact_names_no_initial_state() {
    check_probabilistic(
        Probabilistic::Exact,
        &[NO_INITIAL, EMPTY_UNIVERSE_NO_INITIAL, ORPHAN_NO_INITIAL],
    );
}

#[trace("TC-3241", "FR-203-AC-36")]
#[test]
fn exact_refuses_before_instancing() {
    check_probabilistic(Probabilistic::Exact, &[NO_UNIVERSE, CONFLICT]);
}

#[trace("TC-3241", "FR-203-AC-36")]
#[test]
#[ignore = "QSL-3514: exact refuses conflicting-identity before the missing universe (FR-203-AC-36)"]
fn exact_names_a_missing_universe_before_a_conflict() {
    check_probabilistic(Probabilistic::Exact, &[CONFLICT_NO_UNIVERSE]);
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

#[trace("TC-520", "FR-125-AC-13", "FR-125-AC-23")]
#[test]
#[ignore = "QSL-3514: admission refuses conflicting-identity before the missing universe (FR-125-AC-23)"]
fn statistical_names_a_missing_universe_before_a_conflict() {
    check_probabilistic(Probabilistic::Statistical, &[CONFLICT_NO_UNIVERSE]);
}

#[trace("TC-520", "FR-125-AC-7", "FSpec-FR-361-AC-8")]
#[test]
#[ignore = "QSL-3514: statistical reports no-initial-state over an initial state with no instance (FR-125-AC-7)"]
fn statistical_reports_no_instance() {
    check_probabilistic(
        Probabilistic::Statistical,
        &[EMPTY_UNIVERSE, ONLY_UNADMITTED, ORPHAN],
    );
}

#[trace("TC-520", "FR-125-AC-13", "FSpec-FR-361-AC-14")]
#[test]
fn conformance_refuses_conflicting_identity() {
    check_conformance(&[CONFLICT]);
}

#[trace("TC-520", "FR-125-AC-13", "FR-125-AC-23")]
#[test]
#[ignore = "QSL-3514: admission refuses conflicting-identity before the missing universe (FR-125-AC-23)"]
fn conformance_names_a_missing_universe_before_a_conflict() {
    check_conformance(&[CONFLICT_NO_UNIVERSE]);
}

#[trace("TC-520", "FR-125-AC-8")]
#[test]
fn conformance_checks_the_one_admitted_key() {
    check_conformance(&[UNADMITTED_KEY]);
}

#[trace("TC-2532", "FR-896-AC-7")]
#[test]
#[ignore = "QSL-3507: global conformance checks only the first admitted key (FR-896-AC-7)"]
fn conformance_checks_every_admitted_key() {
    check_conformance(&[SUBTYPE_ONLY, CREATED, UNADMITTED_ACROSS, TWO_POPULATIONS]);
}

#[trace("TC-520", "FR-125-AC-7", "FR-125-AC-13", "FSpec-FR-361-AC-8")]
#[trace("TC-520", "FSpec-FR-361-AC-16")]
#[test]
#[ignore = "QSL-3507: global conformance fails established-invariant-broken over no instance (FR-125-AC-13)"]
fn conformance_names_why_there_is_no_instance() {
    check_conformance(&[
        EMPTY_UNIVERSE,
        ONLY_UNADMITTED,
        ORPHAN,
        NO_INITIAL,
        EMPTY_UNIVERSE_NO_INITIAL,
        ORPHAN_NO_INITIAL,
        NO_UNIVERSE,
    ]);
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

#[trace("TC-900", "FR-315-AC-21", "FR-315-AC-62")]
#[test]
fn smt_temporal_refuses_in_its_table_order() {
    check_smt_temporal(&[
        (EMPTY_UNIVERSE, Some("NoInstance")),
        (ORPHAN, Some("NoInstance")),
        (NO_INITIAL, Some("NoInitialState")),
        (EMPTY_UNIVERSE_NO_INITIAL, Some("NoInitialState")),
        (ORPHAN_NO_INITIAL, Some("NoInitialState")),
        (CONFLICT, Some("conflicting-identity")),
        (UNADMITTED_KEY, Some("ObjectLifecycle")),
        (SUBTYPE_ONLY, None),
        (CREATED, Some("ObjectLifecycle")),
        (UNADMITTED_ACROSS, Some("ObjectLifecycle")),
        (TWO_POPULATIONS, Some("ConstantName")),
    ]);
}

#[trace("TC-900", "FR-315-AC-62")]
#[test]
#[ignore = "QSL-3514: SMT refuses object-lifecycle before no-instance (FR-315-AC-62)"]
fn smt_temporal_refuses_no_instance_before_object_lifecycle() {
    check_smt_temporal(&[(ONLY_UNADMITTED, Some("NoInstance"))]);
}

#[trace("TC-900", "FR-315-AC-62")]
#[test]
#[ignore = "QSL-3514: SMT refuses no-initial-state before requires-bound (FR-315-AC-62)"]
fn smt_temporal_refuses_requires_bound_before_no_initial_state() {
    check_smt_temporal(&[(NO_UNIVERSE, Some("RequiresBound"))]);
}

#[trace("TC-900", "FR-315-AC-62")]
#[test]
#[ignore = "QSL-3514: SMT refuses conflicting-identity before the missing universe (FR-315-AC-62)"]
fn smt_temporal_names_a_missing_universe_before_a_conflict() {
    check_smt_temporal(&[(CONFLICT_NO_UNIVERSE, Some("RequiresBound"))]);
}
