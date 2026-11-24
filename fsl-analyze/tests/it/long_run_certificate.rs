// SPDX-License-Identifier: AGPL-3.0-or-later
//! End-to-end exact long-run candidates checked against an independent graph.

use crate::prob_world::{self, Model, Operation, World};
use filament_exact::{Cancel, CancelCause, Integer, Rational};
use fsl_analyze::analyze::{analyze_items, exact_probabilistic_item, Engines};
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::TransitionSystem;
use fsl_foundation::RequestIndex;
use fsl_replay::{
    check_probability_certificate_document, read_long_run_probability_certificate,
    read_weighted_long_run_probability_certificate, write_long_run_probability_certificate,
    ExactProbabilityCertificateCheck, IncompleteCause, LongRunProbabilityDocument, ModelCheckLimit,
    ModelCheckLimits, ProbabilityDocumentError, ProbabilityDocumentLimits, TerminalValue,
};
use ix_trace_rs::trace;

fn ratio(n: i64, d: i64) -> Rational {
    Rational::new(Integer::from(n), Integer::from(d)).unwrap()
}
pub(super) fn limits() -> ProbabilityDocumentLimits {
    ProbabilityDocumentLimits {
        bytes: 1_000_000,
        storage_bytes: 16_000_000,
        states: 128,
        edges: 1024,
        scalar_bits: 128,
        accounting: fsl_replay::AccountingLimits::default().0,
    }
}
fn claim(name: &str, predicate: &str, bound: &str) -> String {
    format!("probabilistic {name} using inf on M over (s: M::Server) under Steady on origin {{ long_run_fraction holds({predicate}) {bound} }}\n")
}

/// The workload settles the bound exceeded by its stationary distribution.
#[trace("TC-638", "FR-203-AC-28")]
#[test]
fn workload_long_run_dense_working_bytes_limit_settles_incomplete() {
    assert_long_run_dense_limit("Workload");
}

/// The workload's two-state recurrent component has a stationary system
/// whose elimination stop is settled by the long-run adapter.
#[trace("TC-638", "FR-203-AC-28")]
#[test]
fn workload_long_run_dense_elimination_cancellation_settles_incomplete() {
    let world = finite(true);
    prob_world::assert_dense_elimination_cancelled(
        world.request("Fraction", ModelCheckLimits::default()),
    );
}

/// The scheduler route settles the bound exceeded during policy evaluation.
#[trace("TC-638", "FR-203-AC-28")]
#[test]
fn scheduler_long_run_dense_working_bytes_limit_settles_incomplete() {
    assert_long_run_dense_limit("Schedulers");
}

fn assert_long_run_dense_limit(name: &str) {
    use fsl_analyze::exact_probabilistic::{
        build_product_with_key_budget, decide_long_run_mdp, decide_long_run_workload, ProbBuild,
        UnboundedError, UnboundedObjective,
    };
    use fsl_foundation::IdentityLimits;
    use fsl_replay::ProbabilityKeyAdmission;

    const PACKAGE: &str = "test/longrun-dense";
    let bits = [
        "b0", "b1", "b2", "b3", "b4", "b5", "b6", "b7", "b8", "b9", "b10", "b11",
    ];
    let flips = [
        "flip0", "flip1", "flip2", "flip3", "flip4", "flip5", "flip6", "flip7", "flip8", "flip9",
        "flip10", "flip11",
    ];
    // A reflected Gray-code cycle visits all bit patterns, changing one bit
    // per step. Each frame enumerates two candidates for the bit it writes.
    let odd = bits[1..].iter().fold("self.b0".to_owned(), |parity, bit| {
        format!("not ({parity} = self.{bit})")
    });
    let declarations: String = flips
        .iter()
        .zip(bits)
        .enumerate()
        .map(|(index, (name, bit))| {
            let guard = if index == 0 {
                format!("not ({odd})")
            } else {
                let lower = bits[index - 1];
                let selected = if index + 1 == bits.len() {
                    format!("(self.{lower} or self.{bit})")
                } else {
                    format!("self.{lower}")
                };
                std::iter::once(format!("({odd})"))
                    .chain(
                        bits[..index - 1]
                            .iter()
                            .map(|bit| format!("not self.{bit}")),
                    )
                    .chain([selected])
                    .collect::<Vec<_>>()
                    .join(" and ")
            };
            format!(
            "pre Guard{index} using v on M::Server::{name} {{ {guard} }}\n\
             post Step{index} using v on M::Server::{name} {{ self.{bit} = not pre(self.{bit}) }}\n"
        )
        })
        .collect();
    let weights: String = flips
        .iter()
        .map(|name| format!("weight Server::{name} = 1; "))
        .collect();
    let world = World::new(&Model {
        package: PACKAGE,
        object: "Server",
        population: "servers",
        key: "s",
        integers: Vec::new(),
        fields: bits
            .into_iter()
            .map(|bit| (bit, prob_world::BOOLEAN.to_owned()))
            .collect(),
        operations: flips
            .into_iter()
            .zip(bits)
            .map(|(name, bit)| Operation {
                name,
                parameters: Vec::new(),
                modifies: vec![bit],
                deletes: false,
            })
            .collect(),
        body: format!("{declarations}workload Steady on M {{ {weights}}}\n")
            + &claim("Workload", "s.b11", ">= rational(1,2)")
            + &every_claim("Schedulers", "s.b11", ">= rational(1,2)"),
        initial: serde_json::Value::Object(
            bits.into_iter()
                .map(|bit| (bit.to_owned(), serde_json::json!({"boolean": false})))
                .collect(),
        ),
    });
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let cancel = Cancel::new();
    let product_request = world.request(name, ModelCheckLimits::default());
    let mut limits = product_request.limits;
    let mut working_bytes = 0;
    let mut meter = filament_exact::Meter::new(product_request.subject.simulation.meter)
        .with_cancel(cancel.clone());
    let mut admission =
        ProbabilityKeyAdmission::new(&mut meter, &mut working_bytes, u64::MAX, &cancel);
    let ProbBuild::Built(product) = build_product_with_key_budget(
        product_request,
        IdentityLimits::default(),
        &mut admission,
        &cancel,
    )
    .expect("the product fits the working storage bound") else {
        panic!("the long-run product must build");
    };
    assert_eq!(product.states.len(), 1 << bits.len());
    let rows = product.states.len() as u64;
    let cell_bytes =
        (std::mem::size_of::<Rational>() + std::mem::size_of::<num_bigint::BigInt>()) as u64;
    let dense_bytes = rows * (rows + 1) * cell_bytes;
    let bound = working_bytes
        .checked_add(1)
        .expect("the measured key storage fits");
    assert!(
        bound < dense_bytes,
        "{name}: key admission needs {working_bytes} bytes and the dense system needs {dense_bytes}",
    );
    eprintln!("{name}: key admission {working_bytes} bytes; request bound {bound} bytes; dense system {dense_bytes} bytes");
    limits.document.storage_bytes = bound;
    let predicate: Vec<_> = product.labels.iter().map(|labels| labels[0]).collect();
    let solve = if name == "Workload" {
        decide_long_run_workload(&product, &predicate, None, &limits, &cancel).map(|_| ())
    } else {
        decide_long_run_mdp(
            &product,
            &predicate,
            None,
            UnboundedObjective::Min,
            &limits,
            &cancel,
        )
        .map(|_| ())
    };
    assert!(
        matches!(solve, Err(UnboundedError::MaxWorkingBytes)),
        "{name} needs more than the {working_bytes} bytes used by key admission: {solve:?}",
    );

    let mut request = world.request(name, ModelCheckLimits::default());
    request.limits = limits;
    let records = analyze_items(
        &engines,
        vec![exact_probabilistic_item(RequestIndex::new(0), request)],
        &Cancel::new(),
    );
    let terminal = records[0]
        .terminal()
        .unwrap_or_else(|| panic!("{name} must settle incomplete: {:?}", records[0].settlement));
    assert_eq!(
        terminal.value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::CertificateMaxWorkingBytes,
            value: bound,
            required: None,
        }),
        "{name}",
    );
}
pub(super) fn availability() -> World {
    let claims = [
        claim("Lower", "s.phase = 0", ">= rational(1800,1801)"),
        claim("Upper", "s.phase = 0", "<= rational(1800,1801)"),
        claim("TooHigh", "s.phase = 0", ">= rational(1801,1802)"),
        claim("TooLow", "s.phase = 0", "<= rational(999,1000)"),
    ]
    .concat();
    availability_with_claims(claims, "")
}

pub(super) fn availability_with_claims(claims: String, extra: &str) -> World {
    let package = "test/longrun-availability";
    World::new(&Model {
        package, object: "Server", population: "servers", key: "s", integers: vec![("Phase", 0, 1)], fields: vec![("phase", format!("ix://{package}/Phase"))],
        operations: vec![Operation { name: "tick", parameters: vec![("to_down", prob_world::BOOLEAN.to_owned())], modifies: vec!["phase"], deletes: false }, Operation { name: "repair", parameters: vec![("ok", prob_world::BOOLEAN.to_owned())], modifies: vec!["phase"], deletes: false }],
        body: "pre Up using v on M::Server::tick { self.phase = 0 }\npost Ticked using v on M::Server::tick { (to_down and self.phase = 1) or (not to_down and self.phase = 0) }\npre Down using v on M::Server::repair { self.phase = 1 }\npost Repaired using v on M::Server::repair { (ok and self.phase = 0) or (not ok and self.phase = 1) }\nrandom Server::tick(to_down) ~ { true: 1, false: 1999 };\nrandom Server::repair(ok) ~ { true: 9, false: 1 };\nworkload Steady on M { weight Server::tick = 1; weight Server::repair = 1; }\n".to_owned() + extra + &claims,
        initial: serde_json::json!({"phase": {"integer": "0"}}),
    })
}

pub(super) fn produce(world: &World, name: &str) -> Vec<u8> {
    let records = analyze_items(
        &Engines::fsl().unwrap(),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request(name, ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let terminal = records[0].terminal().unwrap();
    terminal
        .exact_certificate_document()
        .unwrap_or_else(|| panic!("expected proof: {:?}", terminal.value()))
        .to_vec()
}
fn check(
    world: &World,
    name: &str,
    bytes: &[u8],
    control: &Cancel,
    limits: ProbabilityDocumentLimits,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
    check_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        bytes,
        limits,
        control,
    )
}
fn accepted(world: &World, name: &str, doc: &LongRunProbabilityDocument) -> bool {
    let bytes = write_long_run_probability_certificate(doc, limits(), &Cancel::new()).unwrap();
    matches!(
        check(world, name, &bytes, &Cancel::new(), limits()).unwrap(),
        ExactProbabilityCertificateCheck::Accepted(_)
    )
}
#[trace("TC-1401", "FR-201-AC-27", "FR-199-AC-1")]
#[test]
fn availability_proofs_preserve_original_transitions_and_gain_bias() {
    let world = availability();
    for name in ["Lower", "Upper"] {
        let bytes = produce(&world, name);
        let doc = read_long_run_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap();
        assert_eq!(doc.components.len(), 1);
        assert_eq!(doc.components[0].value, ratio(1800, 1801));
        assert_eq!(doc.components[0].gain, ratio(1800, 1801));
        assert_eq!(doc.components[0].bias[0].value, ratio(0, 1));
        assert_eq!(doc.components[0].bias[1].value, ratio(-2000, 1801));
        assert!(doc.values.iter().all(|row| row.value == ratio(1800, 1801)));
        assert_eq!(
            write_long_run_probability_certificate(&doc, limits(), &Cancel::new()).unwrap(),
            bytes
        );
        assert!(accepted(&world, name, &doc));
        for mutation in 0..4 {
            let mut bad = doc.clone();
            match mutation {
                0 => bad.components[0].bias[1].value = ratio(2001, 1801),
                1 => bad.components[0].gain = ratio(0, 1),
                2 => {
                    for row in &mut bad.components[0].bias {
                        row.value = row.value.add(&ratio(1, 1));
                    }
                }
                _ => {
                    for row in &mut bad.values {
                        row.value = ratio(1, 1);
                    }
                    bad.components[0].value = ratio(1, 1);
                    bad.components[0].gain = ratio(1, 1);
                }
            }
            assert!(!accepted(&world, name, &bad), "mutation {mutation}");
        }
    }
}

/// FSpec FR-412-AC-29/30: fair long-run outcomes carry
/// independently checked gain-bias and randomized recurrent evidence.
#[trace("FSpec-FR-412-AC-29", "FSpec-FR-412-AC-30")]
#[test]
fn fair_long_run_holds_with_an_independently_checked_component_document() {
    let claim = "probabilistic FairLongRun using inf on M over (p: M::Proc) \
        under every scheduler fair strong Proc::flip on origin \
        { long_run_fraction holds(p.done) >= rational(99,100) }\n";
    let world = World::new(&prob_world::process(claim));
    let records = analyze_items(
        &Engines::fsl().expect("FSL engines"),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("FairLongRun", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let terminal = records[0].terminal().unwrap_or_else(|| {
        panic!(
            "fair long-run claim did not settle: {:?}",
            records[0].settlement
        )
    });
    assert!(
        matches!(terminal.value(), fsl_replay::TerminalValue::Proved { .. }),
        "terminal: {terminal:?}"
    );
    let bytes = terminal
        .exact_certificate_document()
        .expect("the fair long-run proof retains its checked document");
    let document = read_long_run_probability_certificate(bytes, limits(), &Cancel::new())
        .expect("strict long-run certificate");
    assert_eq!(document.values.len(), 2);
    assert!(document.values.iter().all(|row| row.value == ratio(1, 1)));
    assert!(!document.components.is_empty());
    assert!(document
        .components
        .iter()
        .all(|component| component.value == ratio(1, 1)));
    assert!(matches!(
        check(&world, "FairLongRun", bytes, &Cancel::new(), limits()).unwrap(),
        ExactProbabilityCertificateCheck::Accepted(_)
    ));
    let mut duplicate: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let components = duplicate["components"].as_array_mut().unwrap();
    components.push(components[0].clone());
    let forged = serde_json::to_vec(&duplicate).unwrap();
    assert!(matches!(
        check(&world, "FairLongRun", &forged, &Cancel::new(), limits()).unwrap(),
        ExactProbabilityCertificateCheck::Rejected(_)
    ));
}

/// FR-199-AC-4 (TC-2177): when every transition has zero reward weight, the
/// engine uses the exact unweighted recurrent fraction and certifies it.
#[trace("TC-2177", "FR-199-AC-4")]
#[test]
fn long_run_zero_weight_component() {
    let claims = "probabilistic Zero using inf on M over (s: M::Server) under Steady on origin { long_run_fraction holds(s.phase = 0) weighted by duration >= rational(1800,1801) }\n";
    let world = availability_with_claims(
        claims.to_owned(),
        "reward duration on Server::tick = 0;\nreward duration on Server::repair = 0;\n",
    );
    let records = analyze_items(
        &Engines::fsl().unwrap(),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("Zero", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let terminal = records[0]
        .terminal()
        .unwrap_or_else(|| panic!("the zero-weight component must settle: {:?}", records[0]));
    assert!(matches!(
        terminal.value(),
        fsl_replay::TerminalValue::Proved { .. }
    ));
    let bytes = terminal
        .exact_certificate_document()
        .expect("the settled result carries its exact certificate");
    let document =
        read_weighted_long_run_probability_certificate(bytes, limits(), &Cancel::new()).unwrap();
    assert!(document
        .certificate
        .components
        .iter()
        .all(|component| component.value == ratio(1800, 1801)));

    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("Zero")).unwrap();
    assert!(matches!(
        check_probability_certificate_document(
            &system,
            world.package.graph().probabilistic_model().claims(),
            &item,
            bytes,
            limits(),
            &Cancel::new(),
        ),
        Ok(ExactProbabilityCertificateCheck::Accepted(_))
    ));
}

fn finite(periodic: bool) -> World {
    let package = "test/longrun-finite";
    let body = if periodic {
        "post Next using v on M::Server::step { self.phase = 1-pre(self.phase) }\nworkload Steady on M { weight Server::step = 1; }\n".to_owned()+&claim("Fraction","s.phase = 1", ">= rational(1,2)")
    } else {
        "pre Begin using v on M::Server::step { self.phase = 0 }\npost Next using v on M::Server::step { (heads and self.phase = 1) or (not heads and self.phase = 2) }\nrandom Server::step(heads) ~ { true: 1, false: 2 };\nterminal Done using v on M when { self.phase > 0 };\nworkload Steady on M { weight Server::step = 1; }\n".to_owned()+&claim("Fraction","s.phase = 1", ">= rational(1,3)")
    };
    World::new(&Model {
        package,
        object: "Server",
        population: "servers",
        key: "s",
        integers: vec![("Phase", 0, if periodic { 1 } else { 2 })],
        fields: vec![("phase", format!("ix://{package}/Phase"))],
        operations: vec![Operation {
            name: "step",
            parameters: if periodic {
                vec![]
            } else {
                vec![("heads", prob_world::BOOLEAN.to_owned())]
            },
            modifies: vec!["phase"],
            deletes: false,
        }],
        body,
        initial: serde_json::json!({"phase":{"integer":"0"}}),
    })
}
#[trace("TC-1401", "FR-201-AC-28", "FR-201-AC-29")]
#[test]
fn independent_components_fix_terminal_values_transients_and_periodic_classes() {
    for periodic in [false, true] {
        let world = finite(periodic);
        let bytes = produce(&world, "Fraction");
        let doc = read_long_run_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap();
        assert_eq!(doc.components.len(), if periodic { 1 } else { 2 });
        assert!(accepted(&world, "Fraction", &doc));
        if periodic {
            assert!(doc.values.iter().all(|r| r.value == ratio(1, 2)));
        } else {
            assert_eq!(doc.values.len(), 3);
            for v in [ratio(0, 1), ratio(1, 3), ratio(1, 1)] {
                assert!(doc.values.iter().any(|r| r.value == v));
            }
        }
        let mut missing = doc.clone();
        missing.components.pop();
        assert!(!accepted(&world, "Fraction", &missing));
        if !periodic {
            let mut merged = doc.clone();
            let extra = merged.components.pop().unwrap();
            merged.components[0].bias.extend(extra.bias);
            merged.components[0]
                .bias
                .sort_by(|a, b| a.state.cmp(&b.state));
            assert!(!accepted(&world, "Fraction", &merged));
            let mut bad = doc.clone();
            bad.values
                .iter_mut()
                .find(|r| r.value == ratio(1, 3))
                .unwrap()
                .value = ratio(1, 2);
            assert!(!accepted(&world, "Fraction", &bad));
        }
    }
}
#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn long_run_control_stops_preserve_actual_causes_without_acceptance() {
    let world = availability();
    let bytes = produce(&world, "Lower");
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let cancel = Cancel::new();
        cancel.cancel(cause);
        assert_eq!(
            check(&world, "Lower", &bytes, &cancel, limits()),
            Err(ProbabilityDocumentError::Cancelled(cause))
        );
    }
    let mut capped = limits();
    capped.accounting.work_units = 0;
    assert!(matches!(
        check(&world, "Lower", &bytes, &Cancel::new(), capped),
        Err(ProbabilityDocumentError::Accounting(_))
    ));
    let mut capped = limits();
    capped.states = 1;
    assert!(matches!(
        check(&world, "Lower", &bytes, &Cancel::new(), capped),
        Err(ProbabilityDocumentError::Limit {
            limit: fsl_replay::ProbabilityDocumentLimit::States,
            ..
        })
    ));
    let mut capped = limits();
    capped.storage_bytes = 0;
    assert!(matches!(
        check(&world, "Lower", &bytes, &Cancel::new(), capped),
        Err(ProbabilityDocumentError::Limit {
            limit: fsl_replay::ProbabilityDocumentLimit::StorageBytes,
            ..
        })
    ));
}

#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn phase_observed_cancellation_uses_the_actual_checker_control() {
    use fsl_replay::{
        check_long_run_probability_certificate_observing, LongRunProbabilityPhase as Phase,
    };
    let world = availability();
    let bytes = produce(&world, "Lower");
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("Lower")).unwrap();
    let claims = world.package.graph().probabilistic_model().claims();
    for phase in [
        Phase::Admission,
        Phase::Key,
        Phase::Predicate,
        Phase::Workload,
        Phase::Components,
        Phase::Membership,
        Phase::Arithmetic,
    ] {
        let mut reached = false;
        let control = check_long_run_probability_certificate_observing(
            &system,
            claims,
            &item,
            &bytes,
            limits(),
            &Cancel::new(),
            &mut |actual| {
                if actual == phase {
                    reached = true;
                }
            },
        )
        .unwrap();
        assert!(reached, "phase {phase:?}");
        assert!(matches!(
            control,
            ExactProbabilityCertificateCheck::Accepted(_)
        ));
        for cause in [CancelCause::Requested, CancelCause::Deadline] {
            let cancel = Cancel::new();
            let mut reached = false;
            let stopped = check_long_run_probability_certificate_observing(
                &system,
                claims,
                &item,
                &bytes,
                limits(),
                &cancel,
                &mut |actual| {
                    if actual == phase {
                        reached = true;
                        cancel.cancel(cause);
                    }
                },
            );
            assert!(reached, "phase {phase:?}");
            assert_eq!(stopped, Err(ProbabilityDocumentError::Cancelled(cause)));
        }
    }
}

#[trace("TC-1401", "FR-201-AC-27")]
#[test]
fn valid_long_run_equations_still_require_the_checked_initial_threshold() {
    let world = availability();
    let bytes = produce(&world, "Lower");
    let original = read_long_run_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap();
    for name in ["TooHigh", "TooLow"] {
        let mut document = original.clone();
        document.identity = fsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
        let bytes =
            write_long_run_probability_certificate(&document, limits(), &Cancel::new()).unwrap();
        let result = check(&world, name, &bytes, &Cancel::new(), limits()).unwrap();
        let ExactProbabilityCertificateCheck::Rejected(rejection) = result else {
            panic!("threshold must reject")
        };
        assert_eq!(
            rejection.rule,
            fsl_replay::ProbabilityCertificateRule::ThresholdMissed
        );
    }
}

#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn reachable_undefined_predicate_retains_its_family_cause() {
    use fsl_replay::long_run_probability_certificate_document;
    let source="probabilistic Dropped using inf on M over (c: M::Coin) under Even on origin { long_run_fraction holds(c.v = 0) >= rational(1,2) }\n";
    let world = World::new(&prob_world::coin(source));
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("Dropped")).unwrap();
    let result = long_run_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        vec![],
        vec![],
        limits(),
        &Cancel::new(),
    );
    assert!(
        matches!(result,Err(ProbabilityDocumentError::UndefinedPredicate(fsl_eval::temporal::UndefinedCause::Family(ref cause))) if cause.reason==filament_semantic_value::catalog::UndefinedReason::AbsentKey)
    );
}

#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn finite_profile_claims_issue_no_long_run_document() {
    {
        let source = "probabilistic Unsupported using ev on M over (s: M::Server) under Steady on origin { long_run_fraction holds(s.phase = 0) >= rational(1,2) }";
        let world = World::new(&prob_world::service(source));
        let records = analyze_items(
            &Engines::fsl().unwrap(),
            vec![exact_probabilistic_item(
                RequestIndex::new(0),
                world.request("Unsupported", ModelCheckLimits::default()),
            )],
            &Cancel::new(),
        );
        let fsl_analyze::analyze::Settlement::Disposed(disposed) = &records[0].settlement else {
            panic!("expected unsupported disposition: {:?}", records[0]);
        };
        assert_eq!(
            disposed.kind,
            fsl_analyze::analyze::DispositionKind::Unsupported
        );
        assert!(records[0].terminal().is_none());
        assert!(records[0].exact_probabilistic.is_none());
    }
}

/// FR-411-AC-18: no `Every`-scheduler long-run request settles unsupported for
/// naming `Every`; a violated bound settles refuted by its scheduler witness
/// rather than issuing a proof document.
#[trace("FSpec-TC-362", "FSpec-FR-411-AC-18")]
#[test]
fn every_scheduler_long_run_claims_settle_instead_of_unsupported() {
    let source = "probabilistic Settles using inf on M over (s: M::Server) under every scheduler on origin { long_run_fraction holds(s.phase = 0) >= rational(1,2) }";
    let world = World::new(&prob_world::service(source));
    let records = analyze_items(
        &Engines::fsl().unwrap(),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("Settles", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let terminal = records[0]
        .terminal()
        .unwrap_or_else(|| panic!("expected a settlement: {:?}", records[0].settlement));
    assert_eq!(terminal.value(), &fsl_replay::TerminalValue::Refuted);
    assert!(terminal.exact_certificate_document().is_none());
    assert!(records[0].probabilistic_counterexample.is_some());
}

#[trace("TC-1401", "FR-201-AC-30")]
#[test]
fn no_bound_instance_produces_no_document_or_fabricated_probability() {
    let source="probabilistic Empty using inf on M over (s: M::Server) under Steady on origin { long_run_fraction holds(s.phase = 0) >= rational(1,2) }";
    let world = World::with_objects(&prob_world::service(source), &[]);
    let records = analyze_items(
        &Engines::fsl().unwrap(),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request("Empty", ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    let fsl_analyze::analyze::Settlement::Disposed(disposed) = &records[0].settlement else {
        panic!("expected unsupported disposition: {:?}", records[0]);
    };
    assert_eq!(
        disposed.kind,
        fsl_analyze::analyze::DispositionKind::Unsupported
    );
    assert!(records[0].terminal().is_none());
    assert!(records[0].exact_probabilistic.is_none());
    assert!(records[0].counterexample().is_none());
}

#[trace("TC-1401", "FR-201-AC-29")]
#[test]
fn changed_optimization_refuses_the_checked_objective() {
    let world = availability();
    let bytes = produce(&world, "Lower");
    let mut document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    document["objective"]["optimization"] = serde_json::json!("minimum");
    let bytes = serde_json::to_vec(&document).unwrap();
    assert_eq!(
        check(&world, "Lower", &bytes, &Cancel::new(), limits()),
        Err(ProbabilityDocumentError::ObjectiveMismatch)
    );
}

#[trace("TC-1401", "FR-201-AC-29")]
#[test]
fn missing_long_run_evidence_members_reject_at_their_public_member_locus() {
    use fsl_replay::{
        ProbabilityCertificateLocus, ProbabilityCertificateRejection, ProbabilityCertificateRule,
    };
    let world = availability();
    let bytes = produce(&world, "Lower");
    let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    for member in ["values", "bias", "gain", "value"] {
        let mut document = original.clone();
        let locus = if member == "values" {
            document.as_object_mut().unwrap().remove(member);
            "values"
        } else {
            document["components"][0]
                .as_object_mut()
                .unwrap()
                .remove(member);
            "components"
        };
        let bytes = serde_json::to_vec(&document).unwrap();
        assert_eq!(
            check(&world, "Lower", &bytes, &Cancel::new(), limits()),
            Ok(ExactProbabilityCertificateCheck::Rejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::Malformed,
                    at: ProbabilityCertificateLocus::Member(locus)
                }
            )),
            "missing {member}"
        );
    }
    assert!(matches!(
        check(&world, "Lower", &bytes, &Cancel::new(), limits()).unwrap(),
        ExactProbabilityCertificateCheck::Accepted(_)
    ));
}

#[trace("TC-1401", "FR-201-AC-29")]
#[test]
fn conflicting_long_run_identity_refuses_before_key_enumeration() {
    use fsl_replay::{
        check_long_run_probability_certificate_observing, LongRunProbabilityPhase as Phase,
    };
    let world = availability();
    let bytes = produce(&world, "Lower");
    let mut document =
        read_long_run_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap();
    document.identity = fsl_replay::probabilistic_claim_identity(world.claim("Upper")).unwrap();
    let foreign =
        write_long_run_probability_certificate(&document, limits(), &Cancel::new()).unwrap();
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("Lower")).unwrap();
    let mut entered = false;
    assert_eq!(
        check_long_run_probability_certificate_observing(
            &system,
            world.package.graph().probabilistic_model().claims(),
            &item,
            &foreign,
            limits(),
            &Cancel::new(),
            &mut |phase| {
                if phase == Phase::Key {
                    entered = true;
                }
            }
        ),
        Err(ProbabilityDocumentError::IdentityMismatch)
    );
    assert!(
        !entered,
        "foreign identity must stop before the first key is enumerated"
    );
    let mut entered = false;
    let control = check_long_run_probability_certificate_observing(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        &bytes,
        limits(),
        &Cancel::new(),
        &mut |phase| {
            if phase == Phase::Key {
                entered = true;
            }
        },
    )
    .unwrap();
    assert!(entered);
    assert!(matches!(
        control,
        ExactProbabilityCertificateCheck::Accepted(_)
    ));
}

#[trace("TC-1401", "FR-201-AC-29")]
#[test]
fn conflicting_identity_precedes_source_provision_and_compilation() {
    use fsl_replay::{
        check_probability_certificate_source, ByteProvision, DefinitionSourceRef,
        ProbabilitySourceError, ProbabilitySourceLimits, ProbabilitySourceRequest,
    };
    let world = availability();
    let bytes = produce(&world, "Lower");
    let mut document =
        read_long_run_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap();
    document.identity = fsl_replay::probabilistic_claim_identity(world.claim("Upper")).unwrap();
    let foreign =
        write_long_run_probability_certificate(&document, limits(), &Cancel::new()).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("Lower")).unwrap();
    let source = DefinitionSourceRef::from_wire(("test".to_owned(), "unprovided".to_owned()));
    let provision = ByteProvision::admit(
        vec![],
        vec![],
        std::iter::empty::<(String, &DefinitionSourceRef)>(),
    )
    .unwrap();
    let expected = fsl_replay::spine::SourceComponent {
        authority: "test".to_owned(),
        identity: "unprovided".to_owned(),
        content: "invalid source".to_owned(),
    };
    let check = |certificate: &[u8]| {
        check_probability_certificate_source(
            ProbabilitySourceRequest {
                source: &source,
                expected_source: &expected,
                package_name: "never-compiled",
                provision: &provision,
                item: &item,
                initial: vec![fsl_semantics::model::observation::DocumentRef {
                    authority: "test".to_owned(),
                    identity: "absent-state".to_owned(),
                }],
                universes: vec![],
                certificate,
                role: fsl_replay::CertificateRole::Claim,
            },
            ProbabilitySourceLimits {
                spine: fsl_replay::spine::SpineLimits::default(),
                observation: fsl_semantics::model::observation::ObservationLimits::default(),
                expansion: fsl_eval::simulation::model::ExpansionLimits {
                    max_candidates: 100_000,
                },
                scalar: fsl_replay::AccountingLimits::default().0,
                document: limits(),
            },
            &Cancel::new(),
        )
    };
    assert!(matches!(
        check(&foreign),
        Err(ProbabilitySourceError::Document(
            ProbabilityDocumentError::IdentityMismatch
        ))
    ));
    assert!(
        matches!(check(&bytes), Err(ProbabilitySourceError::SourceMismatch)),
        "matching identity must reach the missing provision control"
    );
}

fn every_claim(name: &str, predicate: &str, bound: &str) -> String {
    format!("probabilistic {name} using inf on M over (s: M::Server) under every scheduler on origin {{ long_run_fraction holds({predicate}) {bound} }}\n")
}

/// `go` leaves the initial phase 0 for phase 1, where `stay` loops: phase 0 is
/// transient, phase 1 the only end component.
fn drift() -> World {
    let package = "test/longrun-drift";
    let claims = [
        every_claim("Min", "s.phase = 1", ">= rational(1,2)"),
        every_claim("Max", "s.phase = 0", "<= rational(1,2)"),
    ]
    .concat();
    World::new(&Model {
        package,
        object: "Server",
        population: "servers",
        key: "s",
        integers: vec![("Phase", 0, 1)],
        fields: vec![("phase", format!("ix://{package}/Phase"))],
        operations: vec![
            Operation {
                name: "go",
                parameters: vec![],
                modifies: vec!["phase"],
                deletes: false,
            },
            Operation {
                name: "stay",
                parameters: vec![],
                modifies: vec!["phase"],
                deletes: false,
            },
        ],
        body: format!(
            "pre Start using v on M::Server::go {{ self.phase = 0 }}\n\
             post Went using v on M::Server::go {{ self.phase = 1 }}\n\
             pre Rest using v on M::Server::stay {{ self.phase = 1 }}\n\
             post Stayed using v on M::Server::stay {{ self.phase = 1 }}\n{claims}"
        ),
        initial: serde_json::json!({"phase": {"integer": "0"}}),
    })
}

fn rejection(
    world: &World,
    name: &str,
    document: &serde_json::Value,
) -> ExactProbabilityCertificateCheck {
    let bytes = serde_json::to_vec(document).unwrap();
    check(world, name, &bytes, &Cancel::new(), limits()).unwrap()
}

fn malformed_policy() -> ExactProbabilityCertificateCheck {
    use fsl_replay::{
        ProbabilityCertificateLocus, ProbabilityCertificateRejection, ProbabilityCertificateRule,
    };
    ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
        rule: ProbabilityCertificateRule::Malformed,
        at: ProbabilityCertificateLocus::Member("policy"),
    })
}

#[trace("FSpec-TC-362", "FSpec-FR-412-AC-26", "FSpec-FR-411-AC-18")]
#[test]
fn an_every_scheduler_long_run_proof_carries_per_component_policy_rows() {
    let world = drift();
    for (name, optimization) in [("Min", "minimum"), ("Max", "maximum")] {
        let bytes = produce(&world, name);
        let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(original["objective"]["optimization"], optimization);
        assert_eq!(original["components"].as_array().unwrap().len(), 1);
        let policy = original["components"][0]["policy"].as_array().unwrap();
        assert_eq!(policy.len(), 1, "one row per state of the component");
        assert!(matches!(
            check(&world, name, &bytes, &Cancel::new(), limits()).unwrap(),
            ExactProbabilityCertificateCheck::Accepted(_)
        ));
        let row = policy[0].clone();
        // The policy removed, a row for a state outside the component, a
        // duplicated row, and a choice that is no action of its state.
        let mut removed = original.clone();
        removed["components"][0]
            .as_object_mut()
            .unwrap()
            .remove("policy");
        assert_eq!(rejection(&world, name, &removed), malformed_policy());
        let outside = original["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value["state"].clone())
            .find(|state| *state != row["state"])
            .expect("the transient state");
        let mut foreign = original.clone();
        foreign["components"][0]["policy"] = serde_json::json!([
            {"state": outside, "choice": row["choice"]},
            row
        ]);
        let mut foreign_rows = foreign["components"][0]["policy"]
            .as_array()
            .unwrap()
            .clone();
        foreign_rows.sort_by(|a, b| a["state"].as_str().cmp(&b["state"].as_str()));
        foreign["components"][0]["policy"] = serde_json::Value::Array(foreign_rows);
        assert_eq!(rejection(&world, name, &foreign), malformed_policy());
        let mut duplicated = original.clone();
        duplicated["components"][0]["policy"] = serde_json::json!([row.clone(), row.clone()]);
        assert_eq!(rejection(&world, name, &duplicated), malformed_policy());
        let mut unnamed = original.clone();
        unnamed["components"][0]["policy"][0]["choice"] = serde_json::json!(["terminal-stutter"]);
        assert_eq!(rejection(&world, name, &unnamed), malformed_policy());
    }
}

#[trace("FSpec-TC-362", "FSpec-FR-412-AC-26")]
#[test]
fn an_every_scheduler_long_run_proof_checks_the_gain_bias_inequalities_and_workload_has_no_policy()
{
    use fsl_replay::{
        ProbabilityCertificateLocus, ProbabilityCertificateRejection, ProbabilityCertificateRule,
    };
    let world = availability_with_claims(
        [
            every_claim("EveryLower", "s.phase = 0", ">= rational(1800,1801)"),
            every_claim("EveryUpper", "s.phase = 0", "<= rational(1800,1801)"),
            claim("Lower", "s.phase = 0", ">= rational(1800,1801)"),
        ]
        .concat(),
        "",
    );
    for name in ["EveryLower", "EveryUpper"] {
        let bytes = produce(&world, name);
        let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            original["components"][0]["policy"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(matches!(
            check(&world, name, &bytes, &Cancel::new(), limits()).unwrap(),
            ExactProbabilityCertificateCheck::Accepted(_)
        ));
        // One bias moved by 1/1000 fails its equality at that state.
        let mut changed = original.clone();
        changed["components"][0]["bias"][1]["value"] =
            serde_json::json!({"numerator": "-2000001", "denominator": "1801000"});
        assert!(
            matches!(
                rejection(&world, name, &changed),
                ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::InequalityFails,
                    at: ProbabilityCertificateLocus::ProductState(_),
                })
            ),
            "{name}: a moved bias fails an equality at a state"
        );
    }
    // Under a workload a policy is rejected, and without one the document is
    // accepted.
    let bytes = produce(&world, "Lower");
    assert!(matches!(
        check(&world, "Lower", &bytes, &Cancel::new(), limits()).unwrap(),
        ExactProbabilityCertificateCheck::Accepted(_)
    ));
    let mut workload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    workload["components"][0]["policy"] = serde_json::json!([]);
    assert_eq!(rejection(&world, "Lower", &workload), malformed_policy());
}

/// A one-object automaton over `phase` in `0..phases`: operation `name` moves
/// phase `from` to phase `to`, deterministically. The model starts at phase 0.
fn automaton(
    package: &'static str,
    phases: i64,
    ops: &[(&'static str, i64, i64)],
    claims: String,
) -> World {
    let mut body = String::new();
    for (name, from, to) in ops {
        body.push_str(&format!(
            "pre Pre_{name} using v on M::Server::{name} {{ self.phase = {from} }}\n\
             post Post_{name} using v on M::Server::{name} {{ self.phase = {to} }}\n"
        ));
    }
    body.push_str(&claims);
    World::new(&Model {
        package,
        object: "Server",
        population: "servers",
        key: "s",
        integers: vec![("Phase", 0, phases - 1)],
        fields: vec![("phase", format!("ix://{package}/Phase"))],
        operations: ops
            .iter()
            .map(|(name, _, _)| Operation {
                name,
                parameters: vec![],
                modifies: vec!["phase"],
                deletes: false,
            })
            .collect(),
        body,
        initial: serde_json::json!({"phase": {"integer": "0"}}),
    })
}

/// The state operation `op` reaches from `state`, with its scheduled action.
fn apply(
    world: &World,
    state: &fsl_eval::simulation::model::ModelSimState,
    op: &str,
) -> (
    fsl_eval::simulation::model::ModelSimState,
    fsl_replay::ProductAction,
) {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let action = system
        .actions(state)
        .unwrap()
        .unwrap()
        .into_iter()
        .find(|action| action.identity.operation.contains(op))
        .unwrap_or_else(|| panic!("{op} is enabled"));
    let post = action.draws[0].successors[0].1.clone();
    (
        post,
        fsl_replay::ProductAction::Identity(
            fsl_replay::Schedule::of(&system, &action.identity).unwrap(),
        ),
    )
}

fn product_key(
    world: &World,
    name: &str,
    state: &fsl_eval::simulation::model::ModelSimState,
) -> fsl_replay::ProbabilityProductKey {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let binding = fsl_replay::claim_binding(&system, world.claim(name))
        .unwrap()
        .unwrap();
    let raw = system.state_key(state).unwrap().0;
    fsl_replay::ProbabilityProductKey::for_model_state(Some(&binding.reference), &raw).unwrap()
}

/// One claimed component: its states, value, bias per state and policy.
struct Claimed {
    rho: Rational,
    rows: Vec<(
        fsl_replay::ProbabilityProductKey,
        Rational,
        fsl_replay::ProductAction,
    )>,
}

/// The document a producer that pinned component states would write, written
/// as a forged proof for `name`, and its check.
fn forged(
    world: &World,
    name: &str,
    values: Vec<(fsl_replay::ProbabilityProductKey, Rational)>,
    components: Vec<Claimed>,
) -> ExactProbabilityCertificateCheck {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
    let claims = world.package.graph().probabilistic_model().claims();
    let mut values = values
        .into_iter()
        .map(|(state, value)| fsl_replay::ExactProbabilityValue { state, value })
        .collect::<Vec<_>>();
    values.sort_by(|a, b| a.state.cmp(&b.state));
    let mut components = components
        .into_iter()
        .map(|claimed| {
            let mut rows = claimed.rows;
            rows.sort_by(|a, b| a.0.cmp(&b.0));
            fsl_replay::LongRunProbabilityComponent {
                value: claimed.rho.clone(),
                gain: claimed.rho,
                bias: rows
                    .iter()
                    .map(|(state, bias, _)| fsl_replay::ExactProbabilityValue {
                        state: state.clone(),
                        value: bias.clone(),
                    })
                    .collect(),
                policy: Some(
                    rows.into_iter()
                        .map(|(state, _, choice)| fsl_replay::LongRunPolicyRow {
                            state,
                            choice: fsl_replay::LongRunPolicyChoice::Deterministic(choice),
                        })
                        .collect(),
                ),
            }
        })
        .collect::<Vec<_>>();
    components.sort_by(|a, b| a.bias[0].state.cmp(&b.bias[0].state));
    let document = fsl_replay::long_run_probability_certificate_document(
        &system,
        claims,
        &item,
        values,
        components,
        limits(),
        &Cancel::new(),
    )
    .unwrap();
    let bytes =
        write_long_run_probability_certificate(&document, limits(), &Cancel::new()).unwrap();
    check(world, name, &bytes, &Cancel::new(), limits()).unwrap()
}

fn failed_at(check: &ExactProbabilityCertificateCheck) -> bool {
    use fsl_replay::{
        ProbabilityCertificateLocus, ProbabilityCertificateRejection, ProbabilityCertificateRule,
    };
    matches!(
        check,
        ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
            rule: ProbabilityCertificateRule::InequalityFails,
            at: ProbabilityCertificateLocus::ProductState(_),
        })
    )
}

fn refuted_by_analysis(world: &World, name: &str) -> bool {
    let records = analyze_items(
        &Engines::fsl().unwrap(),
        vec![exact_probabilistic_item(
            RequestIndex::new(0),
            world.request(name, ModelCheckLimits::default()),
        )],
        &Cancel::new(),
    );
    records[0].terminal().map(|t| t.value()) == Some(&fsl_replay::TerminalValue::Refuted)
}

/// A component state may leave for a better component: phase 0 loops on
/// `stay0` or leaves by `go` for phase 1, which loops on `stay`.
#[trace("FSpec-TC-362", "FSpec-FR-412-AC-26", "FSpec-FR-411-AC-18")]
#[test]
fn a_component_state_may_leave_for_a_better_component_and_a_pinned_value_is_rejected() {
    let claims = [
        every_claim("Max", "s.phase = 1", "<= rational(1,2)"),
        every_claim("Min", "s.phase = 1", ">= rational(1,2)"),
    ]
    .concat();
    let world = automaton(
        "test/longrun-leave",
        2,
        &[("idle0", 0, 0), ("go", 0, 1), ("park", 1, 1)],
        claims,
    );
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let zero_state = system.initial()[0].clone();
    let (one_state, go) = apply(&world, &zero_state, "go");
    let (_, stay0) = apply(&world, &zero_state, "idle0");
    let (_, stay) = apply(&world, &one_state, "park");
    let key = |name: &str, state| product_key(&world, name, state);
    let zero = ratio(0, 1);
    let one = ratio(1, 1);
    // The true maximum is 1: phase 0 goes to phase 1. A certificate that pins
    // phase 0 to its own component's value 0 would prove `<= 1/2`.
    let pinned = forged(
        &world,
        "Max",
        vec![
            (key("Max", &zero_state), zero.clone()),
            (key("Max", &one_state), one.clone()),
        ],
        vec![
            Claimed {
                rho: zero.clone(),
                rows: vec![(key("Max", &zero_state), zero.clone(), stay0.clone())],
            },
            Claimed {
                rho: one.clone(),
                rows: vec![(key("Max", &one_state), zero.clone(), stay.clone())],
            },
        ],
    );
    assert!(failed_at(&pinned), "{pinned:?}");
    // Likewise the true minimum is 0 (phase 0 stays), not 1: pinning phase 0
    // to the better component's value is rejected under `minimum`.
    let min_pinned = forged(
        &world,
        "Min",
        vec![
            (key("Min", &zero_state), one.clone()),
            (key("Min", &one_state), one.clone()),
        ],
        vec![
            Claimed {
                rho: zero.clone(),
                rows: vec![(key("Min", &zero_state), zero.clone(), stay0)],
            },
            Claimed {
                rho: one.clone(),
                rows: vec![(key("Min", &one_state), zero.clone(), stay)],
            },
        ],
    );
    assert!(failed_at(&min_pinned), "{min_pinned:?}");
    // The analyzer solves the quotient: both claims are refuted, by the
    // scheduler that goes and the one that stays.
    assert!(refuted_by_analysis(&world, "Max"));
    assert!(refuted_by_analysis(&world, "Min"));
    let _ = go;
}

/// Under `minimum` a component state may leave for a WORSE component: phase 0
/// (value 1 inside its own component) can go to phase 1 (value 0), so the true
/// minimum from phase 0 is 0 and a pinned v(0) = 1 must be rejected.
#[trace("FSpec-TC-362", "FSpec-FR-412-AC-26", "FSpec-FR-411-AC-18")]
#[test]
fn under_minimum_a_component_state_may_leave_for_a_worse_component_and_a_pinned_value_is_rejected()
{
    let world = automaton(
        "test/longrun-leave-worse",
        2,
        &[("idle0", 0, 0), ("go", 0, 1), ("park", 1, 1)],
        every_claim("MinW", "s.phase = 0", ">= rational(1,2)"),
    );
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let zero_state = system.initial()[0].clone();
    let (one_state, _) = apply(&world, &zero_state, "go");
    let (_, stay0) = apply(&world, &zero_state, "idle0");
    let (_, stay) = apply(&world, &one_state, "park");
    let key = |state| product_key(&world, "MinW", state);
    let pinned = forged(
        &world,
        "MinW",
        vec![
            (key(&zero_state), ratio(1, 1)),
            (key(&one_state), ratio(0, 1)),
        ],
        vec![
            Claimed {
                rho: ratio(1, 1),
                rows: vec![(key(&zero_state), ratio(0, 1), stay0)],
            },
            Claimed {
                rho: ratio(0, 1),
                rows: vec![(key(&one_state), ratio(0, 1), stay)],
            },
        ],
    );
    assert!(failed_at(&pinned), "{pinned:?}");
    assert!(refuted_by_analysis(&world, "MinW"));
}

/// FR-412-AC-26's component `C` of two states `u`, `v`: `u` has actions `a`
/// (to `v`) and `b` (to `u`), `v` returns to `u` by `c`. The maximum fraction
/// of `v` is 1/2, by `a`.
fn uv(claims: String) -> World {
    automaton(
        "test/longrun-uv",
        2,
        &[("toV", 0, 1), ("keepU", 0, 0), ("toU", 1, 0)],
        claims,
    )
}

#[trace("FSpec-TC-362", "FSpec-FR-412-AC-26")]
#[test]
fn equalities_under_the_policy_do_not_suffice_when_another_action_breaks_an_inequality() {
    let world = uv([
        every_claim("Holds", "s.phase = 1", "<= rational(1,2)"),
        every_claim("Fails", "s.phase = 1", "<= rational(1,4)"),
    ]
    .concat());
    // The analyzer's proof of the maximum 1/2 is accepted.
    let bytes = produce(&world, "Holds");
    assert!(matches!(
        check(&world, "Holds", &bytes, &Cancel::new(), limits()).unwrap(),
        ExactProbabilityCertificateCheck::Accepted(_)
    ));
    assert!(refuted_by_analysis(&world, "Fails"));
    // Gain 0 with the policy `b` at `u` and `c` at `v` satisfies every equality
    // (u: h = 0 + h, v: h = 1 + h(u)), yet action `a` at `u` breaks the
    // inequality h(u) >= h(v): the pair is not the maximum, and a document
    // claiming `<= 1/4` from it is rejected.
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let u = system.initial()[0].clone();
    let (v, _) = apply(&world, &u, "toV");
    let (_, b) = apply(&world, &u, "keepU");
    let (_, c) = apply(&world, &v, "toU");
    let key = |state| product_key(&world, "Fails", state);
    let (ku, kv) = (key(&u), key(&v));
    let (bias_u, bias_v) = if ku < kv {
        (ratio(0, 1), ratio(1, 1))
    } else {
        (ratio(-1, 1), ratio(0, 1))
    };
    let broken = forged(
        &world,
        "Fails",
        vec![(ku.clone(), ratio(0, 1)), (kv.clone(), ratio(0, 1))],
        vec![Claimed {
            rho: ratio(0, 1),
            rows: vec![(ku, bias_u, b), (kv, bias_v, c)],
        }],
    );
    assert!(failed_at(&broken), "{broken:?}");
}

/// FR-411-AC-18's `Pair`: an initial choice between a component `C1` (phases
/// 1, 2; fraction 1/2) and `C2` (phases 3 to 6; fraction 3/4), one action per
/// state in each. `>= 3/4` is refuted, `<= 3/4` proved.
#[trace("FSpec-TC-362", "FSpec-FR-411-AC-18")]
#[test]
fn pair_settles_ge_three_quarters_refuted_and_le_three_quarters_proved() {
    let claims = [
        every_claim("Ge", "s.phase = 2 or s.phase >= 4", ">= rational(3,4)"),
        every_claim("Le", "s.phase = 2 or s.phase >= 4", "<= rational(3,4)"),
    ]
    .concat();
    let world = automaton(
        "test/longrun-pair",
        7,
        &[
            ("pick1", 0, 1),
            ("pick2", 0, 3),
            ("c1a", 1, 2),
            ("c1b", 2, 1),
            ("c2a", 3, 4),
            ("c2b", 4, 5),
            ("c2c", 5, 6),
            ("c2d", 6, 3),
        ],
        claims,
    );
    let bytes = produce(&world, "Le");
    let proof = read_long_run_probability_certificate(&bytes, limits(), &Cancel::new()).unwrap();
    assert_eq!(proof.components.len(), 2);
    let mut values = proof
        .components
        .iter()
        .map(|component| component.value.clone())
        .collect::<Vec<_>>();
    values.sort();
    assert_eq!(values, vec![ratio(1, 2), ratio(3, 4)]);
    assert!(matches!(
        check(&world, "Le", &bytes, &Cancel::new(), limits()).unwrap(),
        ExactProbabilityCertificateCheck::Accepted(_)
    ));
    assert!(refuted_by_analysis(&world, "Ge"));
}
