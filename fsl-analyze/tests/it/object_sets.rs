// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cross-route comparison of the object set a claim ranges over.

use std::collections::BTreeMap;

use filament_exact::{Cancel, Meter};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_analyze::model_check::{
    check_model, check_state_graph, ModelCheckRequest, StateGraphItem, StateGraphRequest,
};
use fsl_analyze::plan_search::{search, PlanLengths, PlanSearchLimits, PlanSearchRequest};
use fsl_analyze::plan_source::SourcePlan;
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::ModelSystem;
use fsl_package::CheckedPackage;
use fsl_replay::{
    check_closure, check_components, check_product_closure, encode_hyper_query,
    encode_temporal_smt_query, CertificateRequest, ClosureCertificate, HyperQueryRequest,
    ModelCheckItem, ModelCheckLimits, ModelCheckOutcome, ProductClosure, ProductClosureCertificate,
    ProductClosureClause, ProductClosureRequest, ProofCertificate, SmtEncodingLimits, SmtQueryKind,
    StateGraphCertificate, StateGraphCertificateRequest, StateGraphOutcome,
};
use fsl_semantics::check::HyperForm;
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::observation::{document_reference, DocumentRef};
use serde_json::{json, Value as Json};

use crate::analyze_cli_support::{analyze, request_over, Workspace};
use crate::counter::{declarations_in, unit_text_in, UNLIMITED};
use crate::model_instances::{document, ident, universes, world, Held, World, PACKAGE};

type Named = &'static [(&'static str, &'static [&'static str])];

struct Case {
    name: &'static str,
    populations: Named,
    held: &'static [Held<'static>],
    listed: Named,
    initial: bool,
    make: bool,
    over: &'static str,
    instances: &'static [&'static str],
}

const ONE: Named = &[("animals", &["Animal"])];
const TWO: Named = &[("P1", &["Animal"]), ("P2", &["Animal"])];

const BASE: Case = Case {
    name: "base",
    populations: ONE,
    held: &[("animals", &[("a", "Animal")])],
    listed: &[("animals", &["a"])],
    initial: true,
    make: false,
    over: "Animal",
    instances: &[],
};

const CASES: &[Case] = &[
    Case {
        name: "empty-universe",
        held: &[("animals", &[])],
        listed: &[("animals", &[])],
        ..BASE
    },
    Case {
        name: "no-initial",
        initial: false,
        ..BASE
    },
    Case {
        name: "no-universe",
        listed: &[],
        initial: false,
        ..BASE
    },
    Case {
        name: "only-unadmitted",
        held: &[("animals", &[])],
        listed: &[("animals", &["u"])],
        ..BASE
    },
    Case {
        name: "unadmitted-key",
        listed: &[("animals", &["u", "a", "v"])],
        instances: &["a"],
        ..BASE
    },
    Case {
        name: "unadmitted-across-populations",
        populations: TWO,
        held: &[("P1", &[("d", "Animal")]), ("P2", &[("a", "Animal")])],
        listed: &[("P1", &["d", "u"]), ("P2", &["u", "a"])],
        instances: &["d", "a"],
        ..BASE
    },
    Case {
        name: "subtype-only",
        populations: &[("dogs", &["Dog"])],
        held: &[("dogs", &[("d", "Dog"), ("e", "Dog")])],
        listed: &[("dogs", &["e", "d"])],
        instances: &["e", "d"],
        ..BASE
    },
    Case {
        name: "two-populations",
        populations: TWO,
        held: &[
            ("P1", &[("d", "Animal"), ("c", "Animal")]),
            ("P2", &[("c", "Animal"), ("a", "Animal")]),
        ],
        listed: &[("P1", &["d", "c"]), ("P2", &["c", "a"])],
        instances: &["d", "c", "a"],
        ..BASE
    },
    Case {
        name: "created",
        held: &[("animals", &[("c", "Animal"), ("d", "Animal")])],
        listed: &[("animals", &["c", "d", "z"])],
        make: true,
        instances: &["c", "d"],
        ..BASE
    },
    Case {
        name: "orphan",
        over: "Thing",
        ..BASE
    },
    Case {
        name: "conflict",
        populations: TWO,
        held: &[("P1", &[("c", "Dog")]), ("P2", &[("c", "Cat")])],
        listed: &[("P1", &["c"]), ("P2", &["c"])],
        ..BASE
    },
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
    fn populations(&self) -> Vec<(&str, &[&str])> {
        let mut populations = self.populations.to_vec();
        populations.push(("actors", &["Other"]));
        populations
    }

    fn model(&self) -> Vec<u8> {
        let mut model: Json = serde_json::from_slice(&document(&self.populations())).expect("JSON");
        for ty in model["types"].as_array_mut().expect("types") {
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
        self.instances
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
            .listed
            .iter()
            .map(|(population, keys)| {
                let member = self
                    .populations
                    .iter()
                    .find(|(name, _)| name == population)
                    .expect("a declared population")
                    .1[0];
                (*population, keys.iter().map(|key| (*key, member)).collect())
            })
            .collect();
        let mut held: Vec<Held<'_>> = objects
            .iter()
            .map(|(population, objects)| (*population, objects.as_slice()))
            .collect();
        held.push(("actors", &[("p", "Other")]));
        self.snapshot("u0", &held)
    }

    fn run(&self, header: &str, body: &str, items: Json) -> crate::analyze_cli_support::Run {
        let mut held = self.held.to_vec();
        held.push(("actors", &[("p", "Other")]));
        let unit = unit_text_in(
            PACKAGE,
            &format!("{header}terminal Done using v on M when {{ true }};\n{body}"),
        );
        let initial = if self.initial {
            json!([{"authority": "test", "identity": "s0"}])
        } else {
            json!([])
        };
        let ws = Workspace::new(&format!("object-sets-{}", unique()));
        analyze(&request_over(
            &ws,
            &unit,
            &self.model(),
            &[self.snapshot("s0", &held), self.universe_snapshot()],
            json!({"items": items, "subject": {
                "initial_states": initial,
                "universes": [{"authority": "test", "identity": "u0"}],
                "proof_bounds": []}}),
        ))
    }
}

fn unique() -> usize {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[test]
fn zz_explore() {
    for route in [
        "temporal",
        "liveness",
        "graph",
        "hp5",
        "hpaa",
        "hpae",
        "exact",
        "statistical",
        "zone",
        "digital",
        "tgraph",
        "conformance",
    ] {
        for case in CASES {
            let over = case.over;
            let pred = |x: &str| {
                if case.instances.is_empty() {
                    format!("{x} = {x}")
                } else {
                    format!("{x}.age <= 1")
                }
            };
            let (header, body, item) = match route {
            "temporal" => (
                "",
                format!("temporal C using t0 over (x: M::{over}) clock \"model-steps\" on origin {{ always holds({}) }}\n", pred("x")),
                json!({"claim": "C"}),
            ),
            "liveness" => (
                "",
                format!("temporal C using t0 over (x: M::{over}) clock \"model-steps\" on origin {{ eventually holds({}) }}\n", pred("x")),
                json!({"claim": "C"}),
            ),
            "graph" => (
                "",
                format!("possible C using v over (x: M::{over}) {{ {} }}\n", pred("x")),
                json!({"claim": "C"}),
            ),
            "hp5" => (
                "",
                format!("hyper C using t0 over behaviours (x: M::{over}) exists trace a of M {{ always holds(({}) @ a) }}\n", pred("x")),
                json!({"claim": "C"}),
            ),
            "hpaa" => (
                "",
                format!("hyper C using t0 over behaviours (x: M::{over}) forall trace a of M forall trace b of M {{ always holds(({}) @ a) }}\n", pred("x")),
                json!({"claim": "C"}),
            ),
            "hpae" => (
                "",
                format!("hyper C using t0 over behaviours (x: M::{over}) forall trace a of M exists trace b of M {{ always holds(({}) @ b) }}\n", pred("x")),
                json!({"claim": "C"}),
            ),
            "exact" => (
                "",
                format!("probabilistic C using t0 on M over (x: M::{over}) under every scheduler on origin {{ probability >= rational(1, 1) [ eventually holds({}) ] }}\n", pred("x")),
                json!({"claim": "C", "probabilistic": {"evidence": "exact"}}),
            ),
            "statistical" => (
                "",
                format!("workload Even on M {{ weight Animal::idle = 1; }}\nprofile_placeholder\nprobabilistic C using ev on M over (x: M::{over}) under Even on origin with alpha 0.1, beta 0.1, indifference 0.05 {{ probability >= 0.9 [ eventually[0,1] holds({}) ] }}\n", pred("x")),
                json!({"claim": "C", "probabilistic": {"evidence": "statistical", "seed": "7"}}),
            ),
            "zone" | "digital" => (
                "profile tt = \"filament.temporal.timed/v1\";\ndimension Time;\nunit ms : Time = rational(1, 1);\ntime M dense unit ms;\n",
                format!("temporal C using tt over (x: M::{over}) clock \"model-time\" on origin {{ always holds({}) }}\n", pred("x")),
                if route == "zone" { json!({"claim": "C"}) } else { json!({"claim": "C", "options": {"method": "digital-clocks"}}) },
            ),
            "tgraph" => (
                "dimension Time;\nunit ms : Time = rational(1, 1);\ntime M dense unit ms;\n",
                format!("possible C using v over (x: M::{over}) {{ {} }}\n", pred("x")),
                json!({"claim": "C"}),
            ),
            "conformance" => (
                "profile pp = \"filament.protocol.finite-global/v1\";\n",
                format!("record Ack {{ ok: Boolean; }}\nprotocol Flow using pp over (k: M::{over}) on origin {{\nrole Buyer on M::Other;\nrole Seller on M::Other;\nchannel Order from Buyer to Seller carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\nrun sequence Main {{\nsend Placed via Order as (placed: Ack) {{ true }};\nreceive Got via Order of Placed as (got: Ack) {{ true }};\n}}\nfinish Done2 as (zz: Boolean) {{ true }};\n}}\n"),
                json!({"claim": "Flow", "kind": "global-conformance"}),
            ),
            _ => continue,
        };
            let body = body.replace("profile_placeholder\n", "");
            let run = case.run(header, &body, json!([item]));
            let document = run.document();
            let item = &document["items"][0];
            if item.is_null() {
                println!("{route} {}: exit {} {document}", case.name, run.exit);
            } else {
                println!(
                    "{route} {}: exit {} record={} cx={} diag={}",
                    case.name, run.exit, item["record"], item["counterexample"], item["diagnostic"],
                );
            }
        }
    }
}

struct Library {
    world: World,
    package: CheckedPackage,
}

impl Case {
    fn library(&self, header: &str, body: &str) -> Library {
        let document = self.model();
        let body = format!("{header}terminal Done using v on M when {{ true }};\n{body}");
        let graph = declarations_in(PACKAGE, &document, &body)
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_eval::value::S6aRewardEvaluator,
            )
            .unwrap_or_else(|refusals| panic!("{body}: the unit checks: {refusals:?}"));
        let mut held = self.held.to_vec();
        held.push(("actors", &[("p", "Other")]));
        let bytes = self.snapshot("s0", &held);
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
            document,
            package: CheckedPackage::link(
                declarations_in(PACKAGE, &self.model(), "")
                    .check(
                        CheckMode::Linked,
                        CheckingLimits::default(),
                        &fsl_eval::value::S6aRewardEvaluator,
                    )
                    .expect("the empty unit checks"),
            ),
        };
        Library {
            world,
            package: CheckedPackage::link(graph),
        }
    }

    fn universes(&self) -> Vec<fsl_eval::simulation::model::PopulationUniverse> {
        let mut listed = self.listed.to_vec();
        listed.push(("actors", &["p"]));
        universes(&listed)
    }
}

impl Library {
    fn subject(&self, case: &Case) -> ModelSubject<'_> {
        self.world.subject(&self.package, case.universes())
    }
}

#[test]
fn zz_explore_library() {
    for case in CASES {
        let over = case.over;
        let bound = case.instances.len();
        let pred = |i: usize| {
            if bound == 0 {
                "x = x".to_owned()
            } else {
                format!("x.age <= {i}")
            }
        };
        for (route, formula) in [("closure", "always"), ("components", "eventually")] {
            let library = case.library(
                "",
                &format!("temporal C using t0 over (x: M::{over}) clock \"model-steps\" on origin {{ {formula} holds({}) }}\n", pred(bound)),
            );
            let node = &library.package.graph().temporal_clauses()[0];
            let system = match ModelSystem::new(library.subject(case).simulation) {
                Ok(system) => system,
                Err(error) => {
                    println!("{route} {}: system {error:?}", case.name);
                    continue;
                }
            };
            let letters = match system.clause_letters(node) {
                Ok(letters) => letters,
                Err(error) => {
                    println!("{route} {}: letters {error:?}", case.name);
                    continue;
                }
            };
            let item = || ModelCheckItem::Clause {
                clause: node.clause(),
                letters: &letters,
            };
            let outcome = check_model(
                ModelCheckRequest {
                    workers: 1,
                    temporal_clock_selections: &[],
                    subject: library.subject(case),
                    item: item(),
                    max_depth: None,
                    limits: ModelCheckLimits::default(),
                    smt_encoding: Default::default(),
                    smt_run: Default::default(),
                    reductions: Default::default(),
                },
                &Cancel::new(),
            );
            let certificate = match &outcome {
                Ok(ModelCheckOutcome::Holds { certificate, .. }) => Some(certificate.clone()),
                other => {
                    println!("{route} {}: engine {other:?}", case.name);
                    None
                }
            };
            let request = CertificateRequest {
                temporal_clock_selections: &[],
                subject: library.subject(case),
                item: item(),
                smt_encoding: Default::default(),
            };
            let limits = ModelCheckLimits::default();
            let checked = match certificate {
                Some(ProofCertificate::Closure(proof)) => {
                    check_closure(&request, &proof, &limits, &Cancel::new())
                }
                Some(ProofCertificate::Component(proof)) => {
                    check_components(&request, &proof, &limits, &Cancel::new())
                }
                None => check_closure(
                    &request,
                    &ClosureCertificate { states: Vec::new() },
                    &limits,
                    &Cancel::new(),
                ),
            };
            println!(
                "{route} {}: checker {}",
                case.name,
                format!("{checked:?}").chars().take(300).collect::<String>()
            );
        }

        let library = case.library(
            "",
            &format!("possible C using v over (x: M::{over}) {{ {} }}\n", pred(1)),
        );
        let claim = &library.package.graph().state_graph_claims()[0];
        let run = check_state_graph(
            StateGraphRequest {
                subject: library.subject(case),
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
        match run {
            Ok(run) => {
                for instance in &run.items[0] {
                    println!(
                        "graphlib {}: {} {}",
                        case.name,
                        instance.over.as_ref().map_or("-", |o| o.object().as_str()),
                        format!("{:?}", instance.outcome)
                            .chars()
                            .take(80)
                            .collect::<String>()
                    );
                    let certificate = match &instance.outcome {
                        StateGraphOutcome::Witnessed { certificate, .. } => certificate.clone(),
                        _ => StateGraphCertificate {
                            closure: Vec::new(),
                            ranks: Vec::new(),
                            counts: Vec::new(),
                        },
                    };
                    let checked = fsl_replay::check_state_graph(
                        &StateGraphCertificateRequest {
                            subject: library.subject(case),
                            claim,
                            over: instance.over.clone(),
                        },
                        &certificate,
                        &ModelCheckLimits::default(),
                        &Cancel::new(),
                    );
                    println!(
                        "graphcheck {}: {}",
                        case.name,
                        format!("{checked:?}").chars().take(200).collect::<String>()
                    );
                }
            }
            Err(error) => println!("graphlib {}: {error:?}", case.name),
        }

        let library = case.library(
            "",
            &format!("type Lengths = Int[0,1];\nsynthesis Reach using t0 grammar M::Animal domain Lengths satisfies {{ true }};\ntemporal C using t0 over (x: M::{over}) clock \"model-steps\" on origin {{ always holds({}) }}\n", pred(bound)),
        );
        match ModelSystem::over_model(library.subject(case).simulation, "M") {
            Ok(system) => {
                let node = &library.package.graph().temporal_clauses()[0];
                let synthesis = &library.package.graph().syntheses()[0];
                match SourcePlan::new(&library.package, synthesis, &system, vec![node], UNLIMITED) {
                    Ok(mut rules) => {
                        let result = search(
                            &system,
                            &mut rules,
                            PlanSearchRequest {
                                lengths: PlanLengths::new(0, 1).expect("ordered"),
                                limits: PlanSearchLimits::default(),
                                cancel: &Cancel::new(),
                                fault: None,
                            },
                        );
                        match result {
                            Ok(result) => println!("plan {}: {:?}", case.name, result.status),
                            Err(error) => println!("plan {}: {error:?}", case.name),
                        }
                    }
                    Err(error) => println!("plan {}: source {error:?}", case.name),
                }
            }
            Err(error) => println!("plan {}: system {error:?}", case.name),
        }

        let library = case.library(
            "",
            &format!("temporal C using t0 over (x: M::{over}) clock \"model-steps\" on origin {{ always holds({}) }}\n", pred(bound)),
        );
        let smt = encode_temporal_smt_query(
            &library.subject(case),
            library.package.graph().temporal_clauses()[0].clause(),
            SmtQueryKind::InductionBase { depth: 1 },
            &SmtEncodingLimits::default(),
        );
        match smt {
            Ok(script) => println!(
                "smt {}: {}",
                case.name,
                String::from_utf8_lossy(&script)
                    .chars()
                    .take(900)
                    .collect::<String>()
                    .replace('\n', " | ")
            ),
            Err(error) => println!("smt {}: {error:?}", case.name),
        }

        let library = case.library(
            "",
            &format!("hyper C using t0 over behaviours (x: M::{over}) forall trace a of M forall trace b of M {{ always holds(({}) @ a) }}\n", pred(bound)),
        );
        let clause = &library.package.graph().hyper_clauses()[0];
        let subjects = vec![library.subject(case), library.subject(case)];
        let key = case.instances.first().copied().unwrap_or("a");
        let mut meter = Meter::new(UNLIMITED);
        let query = encode_hyper_query(
            HyperQueryRequest {
                clause,
                subjects: &subjects,
                instance: &[("x".to_owned(), key.to_owned())],
                meter: &mut meter,
                encoding: SmtEncodingLimits::default(),
            },
            SmtQueryKind::InductionBase { depth: 1 },
        );
        match query {
            Ok(_) => println!("smthyper {}: encoded {key}", case.name),
            Err(error) => println!("smthyper {}: {error:?}", case.name),
        }
        let product = check_product_closure(
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
        println!(
            "product {}: {}",
            case.name,
            format!("{product:?}").chars().take(200).collect::<String>()
        );
    }
}
