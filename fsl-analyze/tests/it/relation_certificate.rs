// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-163: HP-1 certificates checked independently of retained engine graphs.

use filament_exact::Cancel;
use fsl_analyze::model_check::{
    check_model_relation, ModelRelationCheckRequest, ModelRelationOutcome,
};
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::{StateKey, TransitionSystem};
use fsl_package::CheckedPackage;
use fsl_replay::{
    check_product_closure, settle_hyper_check, CertificateLocus, CertificateRule, Certification,
    HyperOutcome, HyperSubjectClosure, IncompleteCause, ModelCheckLimit, ModelCheckLimits,
    ProductClosure, ProductClosureCertificate, ProductClosureCheck, ProductClosureClause,
    ProductClosureRequest, ProofBasis, RunStats, TerminalValue,
};
use fsl_semantics::check::CheckedModelRelation;
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::{DocumentProvision, DocumentRef};
use ix_trace_rs::trace;

use crate::counter::UNLIMITED;
use crate::hyper_world::{self, Vault};
use crate::relation_claims::{
    initial_world, package_and_relation, package_and_relation_with_document,
    step_with_result_document, subjects, DETERMINISTIC,
};

struct Fixture {
    package: CheckedPackage,
    relation: CheckedModelRelation,
    packages: PackageInput,
    snapshots: DocumentProvision,
    initial: Vec<DocumentRef>,
}

impl Fixture {
    fn new(vault: Vault, text: &str) -> Self {
        Self::from_parts(package_and_relation(vault, text))
    }

    fn from_parts(
        (package, relation, document): (CheckedPackage, CheckedModelRelation, Vec<u8>),
    ) -> Self {
        let (packages, snapshots, initial) = initial_world(&document);
        Self {
            package,
            relation,
            packages,
            snapshots,
            initial,
        }
    }

    fn subjects(&self) -> Vec<fsl_eval::model_subject::ModelSubject<'_>> {
        let subject = subjects(
            &self.package,
            &self.packages,
            &self.snapshots,
            &self.initial,
        )
        .remove(0);
        self.relation
            .aliases()
            .iter()
            .map(|_| fsl_eval::model_subject::ModelSubject {
                simulation: subject.simulation.clone(),
                proof_bounds: subject.proof_bounds.clone(),
            })
            .collect()
    }

    fn proof(&self, limits: ModelCheckLimits) -> ProductClosureCertificate {
        let outcome = check_model_relation(
            ModelRelationCheckRequest {
                relation: &self.relation,
                package: &self.package,
                subjects: self.subjects(),
                limits,
                max_depth: None,
                meter: UNLIMITED,
            },
            &Cancel::new(),
        )
        .expect("relation search admits");
        let ModelRelationOutcome::Holds { certificate, .. } = outcome else {
            panic!("expected holding search: {outcome:?}");
        };
        certificate
    }

    fn check(
        &self,
        proof: &ProductClosureCertificate,
        limits: ModelCheckLimits,
    ) -> Result<ProductClosureCheck, fsl_replay::ProductClosureRefusal> {
        let subjects = self.subjects();
        check_product_closure(
            &ProductClosureRequest {
                package: &self.package,
                clause: ProductClosureClause::Relation(&self.relation),
                subjects: &subjects,
                meter: UNLIMITED,
            },
            proof,
            &limits,
            &Cancel::new(),
        )
    }

    fn settle(
        &self,
        proof: ProductClosureCertificate,
        limits: ModelCheckLimits,
    ) -> fsl_replay::HyperRecord {
        let subjects = self.subjects();
        let request = ProductClosureRequest {
            package: &self.package,
            clause: ProductClosureClause::Relation(&self.relation),
            subjects: &subjects,
            meter: UNLIMITED,
        };
        settle_hyper_check(
            &HyperOutcome::Holds {
                certificates: vec![proof],
                stats: RunStats::default(),
            },
            &limits,
            &[ModelCheckLimit::MaxRelationTuples],
            Some(&request),
            &Cancel::new(),
            |_| panic!("a holding certificate never replays a counterexample"),
        )
        .expect("certificate settles")
    }

    fn initial_keys(&self) -> Vec<StateKey> {
        let subjects = self.subjects();
        let system =
            ModelSystem::for_alias(subjects[0].simulation.clone(), &self.relation.aliases()[0])
                .expect("model admits");
        let mut keys: Vec<_> = system
            .initial()
            .iter()
            .map(|state| system.state_key(state).expect("state key").1)
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }
}

fn closure(proof: &ProductClosureCertificate) -> &[HyperSubjectClosure] {
    match &proof.closure {
        ProductClosure::Relation(subjects) => subjects,
        ProductClosure::Product(_) => panic!("expected HP-1 closure"),
    }
}

fn closure_mut(proof: &mut ProductClosureCertificate) -> &mut Vec<HyperSubjectClosure> {
    match &mut proof.closure {
        ProductClosure::Relation(subjects) => subjects,
        ProductClosure::Product(_) => panic!("expected HP-1 closure"),
    }
}

fn assert_state_rejection(
    fixture: &Fixture,
    proof: &ProductClosureCertificate,
    rule: CertificateRule,
    key: &StateKey,
) {
    let result = fixture
        .check(proof, ModelCheckLimits::default())
        .expect("state defect has a certificate verdict");
    assert_eq!(
        result,
        ProductClosureCheck::Rejected(fsl_replay::CertificateRejection {
            rule,
            at: CertificateLocus::HyperState(key.clone())
        })
    );
    let ProductClosureCheck::Rejected(rejection) = result else {
        unreachable!()
    };
    let wire = serde_json::to_value(&rejection.at).expect("state rejection serializes");
    assert_eq!(wire.as_object().expect("state locus object").len(), 1);
    assert_eq!(
        wire["states"].as_array().expect("actual state keys").len(),
        1
    );
}

#[trace("FR-163-AC-1", "FR-163-AC-10", "FR-182-AC-17")]
#[test]
fn hp1_det_certificate_is_deterministic_and_independently_certified() {
    let fixture = Fixture::new(Vault::Secure, DETERMINISTIC);
    let proof = fixture.proof(ModelCheckLimits::default());
    let repeated = fixture.proof(ModelCheckLimits::default());
    assert_eq!(
        proof.relation_bytes().expect("HP-1 JCS"),
        repeated.relation_bytes().expect("repeated HP-1 JCS")
    );
    let [subject] = closure(&proof) else {
        panic!("one shared alias closure");
    };
    assert_eq!(subject.alias, "V");
    assert_eq!(subject.states.len(), 4);
    assert!(subject.states.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        subject
            .executions
            .iter()
            .map(|execution| execution.execution.as_str())
            .collect::<Vec<_>>(),
        ["x", "y"]
    );
    assert_eq!(subject.executions[0].transitions.len(), 8);
    assert_eq!(subject.executions[1].transitions.len(), 8);
    let wire: serde_json::Value =
        serde_json::from_slice(&proof.relation_bytes().expect("certificate bytes"))
            .expect("JSON certificate");
    assert_eq!(
        wire["closure"]
            .as_array()
            .expect("same one-alias sequence")
            .len(),
        1
    );
    assert!(wire["closure"][0].get("automaton_state").is_none());
    assert_eq!(
        fixture
            .check(&proof, ModelCheckLimits::default())
            .expect("independent check"),
        ProductClosureCheck::Accepted
    );
    let record = fixture.settle(proof, ModelCheckLimits::default());
    assert_eq!(
        record.value,
        TerminalValue::Proved {
            basis: ProofBasis::Exhaustive,
            certification: Certification::Certified
        }
    );
    assert!(record.counterexample.is_none());
    assert_eq!(record.certificates.len(), 1);
}

#[trace("FR-163-AC-10")]
#[test]
fn hp1_state_mismatches_name_the_actual_missing_or_extra_key() {
    let fixture = Fixture::new(Vault::Secure, DETERMINISTIC);
    let proof = fixture.proof(ModelCheckLimits::default());
    let initial = fixture.initial_keys();
    let mut missing = proof.clone();
    closure_mut(&mut missing)[0]
        .states
        .retain(|key| key != &initial[0]);
    assert_state_rejection(
        &fixture,
        &missing,
        CertificateRule::InitialMissing,
        &initial[0],
    );
    let noninitial = closure(&proof)[0]
        .states
        .iter()
        .find(|key| !initial.contains(key))
        .expect("reachable noninitial state")
        .clone();
    let mut missing = proof.clone();
    closure_mut(&mut missing)[0]
        .states
        .retain(|key| key != &noninitial);
    assert_state_rejection(
        &fixture,
        &missing,
        CertificateRule::SuccessorMissing,
        &noninitial,
    );
    let extra = StateKey::from_canonical_bytes(b"null".to_vec());
    let mut added = proof.clone();
    closure_mut(&mut added)[0].states.push(extra.clone());
    closure_mut(&mut added)[0].states.sort();
    assert_state_rejection(&fixture, &added, CertificateRule::Malformed, &extra);
    for extra in [
        StateKey::from_canonical_bytes(Vec::new()),
        StateKey::from_canonical_bytes(b"zz".to_vec()),
    ] {
        let mut mixed = missing.clone();
        closure_mut(&mut mixed)[0].states.push(extra.clone());
        closure_mut(&mut mixed)[0].states.sort();
        let (rule, first) = if extra < noninitial {
            (CertificateRule::Malformed, &extra)
        } else {
            (CertificateRule::SuccessorMissing, &noninitial)
        };
        assert_state_rejection(&fixture, &mixed, rule, first);
        closure_mut(&mut mixed)[0]
            .states
            .retain(|key| key != &initial[0]);
        assert_state_rejection(
            &fixture,
            &mixed,
            CertificateRule::InitialMissing,
            &initial[0],
        );
    }
}

#[trace("FR-163-AC-10")]
#[test]
fn hp1_closed_unreachable_superset_is_rejected_at_its_least_extra_state() {
    let reachable = Fixture::new(Vault::Secure, DETERMINISTIC).proof(ModelCheckLimits::default());
    let fixture = Fixture::new(
        Vault::Custom("post Step using v on V::Vault::step { self.l = 0 }\n"),
        DETERMINISTIC,
    );
    let mut proof = fixture.proof(ModelCheckLimits::default());
    let extra = closure(&reachable)[0]
        .states
        .iter()
        .filter(|key| !closure(&proof)[0].states.contains(key))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        extra.len(),
        2,
        "states with l=1 are valid but unreachable when every step writes l=0"
    );
    closure_mut(&mut proof)[0]
        .states
        .extend(extra.iter().cloned());
    closure_mut(&mut proof)[0].states.sort();
    assert_state_rejection(&fixture, &proof, CertificateRule::Malformed, &extra[0]);
}

#[trace("FR-163-AC-10")]
#[test]
fn hp1_unsorted_and_duplicate_states_fail_before_recomputation() {
    let fixture = Fixture::new(Vault::Secure, DETERMINISTIC);
    let proof = fixture.proof(ModelCheckLimits::default());
    for duplicate in [false, true] {
        let mut malformed = proof.clone();
        let states = &mut closure_mut(&mut malformed)[0].states;
        if duplicate {
            states.insert(1, states[0].clone());
        } else {
            states.swap(0, 1);
        }
        let offending = states[1].clone();
        let limits = ModelCheckLimits {
            limits: fsl_eval::simulation::Limits::default().with_max_states(0),
            ..ModelCheckLimits::default()
        };
        assert_eq!(
            fixture
                .check(&malformed, limits)
                .expect("malformed before budget"),
            ProductClosureCheck::Rejected(fsl_replay::CertificateRejection {
                rule: CertificateRule::Malformed,
                at: CertificateLocus::HyperState(offending)
            })
        );
    }
}

#[trace("FR-163-AC-13", "FR-184-AC-3", "FR-184-AC-4", "FR-182-AC-17")]
#[test]
fn hp1_checker_owns_its_tuple_allowance_and_names_first_unpaid_tuple() {
    let fixture = Fixture::new(Vault::Secure, DETERMINISTIC);
    let limits = ModelCheckLimits {
        max_relation_tuples: 64,
        ..ModelCheckLimits::default()
    };
    let proof = fixture.proof(limits);
    assert_eq!(
        fixture.check(&proof, limits).expect("own 64 tuples"),
        ProductClosureCheck::Accepted
    );
    assert_eq!(
        fixture.check(&proof, limits).expect("fresh 64 tuples"),
        ProductClosureCheck::Accepted
    );
    let tight = ModelCheckLimits {
        max_relation_tuples: 63,
        ..limits
    };
    let ProductClosureCheck::Stopped {
        cause,
        tuple: Some(tuple),
    } = fixture.check(&proof, tight).expect("named tuple stop")
    else {
        panic!("expected first unpaid tuple");
    };
    assert_eq!(
        cause,
        IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxRelationTuples,
            value: 63
        }
    );
    assert_eq!(tuple.count, 64);
    let executions = &closure(&proof)[0].executions;
    assert_eq!(
        tuple.first_unpaid.transitions,
        [
            executions[0].transitions[7].clone(),
            executions[1].transitions[7].clone()
        ]
    );
    let record = fixture.settle(proof.clone(), tight);
    assert_eq!(record.value, TerminalValue::Incomplete(cause));
    assert_eq!(record.count, Some(64));
    assert!(record.counterexample.is_none());
    assert!(record.certificates.is_empty());
    let terminal = fsl_replay::TerminalRecord::of_hyper(
        fsl_foundation::RequestIndex::new(0),
        fsl_foundation::BackendId::new("en-1"),
        &record,
    )
    .expect("terminal record");
    let wire = serde_json::to_value(fsl_replay::outcome::OutcomeItem::from_terminal(&terminal))
        .expect("settled cause wire");
    let text = wire.to_string();
    assert!(text.contains("model_check.max_relation_tuples"), "{text}");
    assert!(
        !text.contains("first_unpaid"),
        "checker-only tuple must not alter the cause: {text}"
    );
    let zero = ModelCheckLimits {
        max_relation_tuples: 0,
        ..limits
    };
    let ProductClosureCheck::Stopped {
        tuple: Some(first), ..
    } = fixture.check(&proof, zero).expect("zero allowance")
    else {
        panic!("zero allowance stops");
    };
    assert_eq!(first.count, 1);
    assert_eq!(
        first.first_unpaid.transitions,
        [
            executions[0].transitions[0].clone(),
            executions[1].transitions[0].clone()
        ]
    );
}

#[trace("FR-163-AC-11")]
#[test]
fn hp1_aliases_follow_declarations_and_edge_sequences_are_exact_inputs() {
    use filament_semantic_value::checking::{CheckMode, CheckingLimits};
    let document = hyper_world::document(false, false);
    let text = "relation Ordered using v over (y: N::Vault::step, x: V::Vault::step, z: V::Vault::step) { true }";
    let unit = hyper_world::unit_text(None, Vault::Secure, false, text).replace(
        "function Noop",
        &format!("model N = \"{}\";\nfunction Noop", hyper_world::PACKAGE),
    );
    let graph = hyper_world::declarations(&[&document], &unit)
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .expect("two aliases check");
    let package = CheckedPackage::link(graph);
    let relation = package.graph().step_relations()[0].clone();
    let fixture = Fixture::from_parts((package, relation, document));
    assert_eq!(fixture.relation.aliases(), ["V", "N"]);
    let proof = fixture.proof(ModelCheckLimits::default());
    let entries = closure(&proof);
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.alias.as_str())
            .collect::<Vec<_>>(),
        ["V", "N"]
    );
    assert_eq!(
        entries[0]
            .executions
            .iter()
            .map(|entry| entry.execution.as_str())
            .collect::<Vec<_>>(),
        ["x", "z"]
    );
    assert_eq!(entries[1].executions[0].execution, "y");
    assert_eq!(
        entries[0].states, entries[1].states,
        "equal keys retain two separate alias entries"
    );
    assert_eq!(
        fixture
            .check(&proof, ModelCheckLimits::default())
            .expect("two independent alias closures"),
        ProductClosureCheck::Accepted
    );
    let mut simultaneous = proof.clone();
    let extra = StateKey::from_canonical_bytes(b"zz".to_vec());
    closure_mut(&mut simultaneous)[0].states.push(extra.clone());
    let initials = fixture.initial_keys();
    let missing = closure(&simultaneous)[1]
        .states
        .iter()
        .find(|key| !initials.contains(key))
        .expect("second alias noninitial")
        .clone();
    closure_mut(&mut simultaneous)[1]
        .states
        .retain(|key| key != &missing);
    assert_state_rejection(&fixture, &simultaneous, CertificateRule::Malformed, &extra);
    for defect in 0..11 {
        let mut altered = proof.clone();
        let entries = closure_mut(&mut altered);
        match defect {
            0 => {
                entries.pop();
            }
            1 => {
                entries[1] = entries[0].clone();
            }
            2 => {
                let mut extra = entries[0].clone();
                extra.alias = "Third".to_owned();
                entries.push(extra);
            }
            3 => entries.swap(0, 1),
            4 => {
                entries[0].executions.pop();
            }
            5 => entries[0].executions.swap(0, 1),
            6 => {
                entries[0].executions[1] = entries[0].executions[0].clone();
            }
            7 => {
                entries[0].executions[0].transitions.pop();
            }
            8 => entries[0].executions[0].transitions.swap(0, 1),
            9 => {
                let edge = entries[0].executions[0].transitions[0].clone();
                entries[0].executions[0].transitions.push(edge);
            }
            10 => entries[0].executions[0].transitions[0].post_state = StateKey::default(),
            _ => unreachable!(),
        }
        assert!(
            matches!(
                fixture.check(&altered, ModelCheckLimits::default()),
                Err(fsl_replay::ProductClosureRefusal::Input(_))
            ),
            "input defect {defect} must not produce a certificate verdict"
        );
    }
}

#[trace("FR-163-AC-6", "FR-163-AC-12")]
#[test]
fn hp1_false_and_undefined_rejections_keep_complete_first_tuples() {
    const NONDETERMINISTIC: &str =
        "post Step using v on V::Vault::step { self.l = i or self.l = 1 - i }\n";
    let true_text = "relation Det using v over (x: V::Vault::step, y: V::Vault::step) { true }";
    for (vault, rejected_body, rule) in [
        (
            Vault::Custom(NONDETERMINISTIC),
            DETERMINISTIC,
            CertificateRule::ViolatingMember,
        ),
        (
            Vault::Secure,
            "relation Det using v over (x: V::Vault::step, y: V::Vault::step) { 1 mod x.i >= y.i }",
            CertificateRule::UndefinedMember,
        ),
    ] {
        let proof = Fixture::new(vault, true_text).proof(ModelCheckLimits::default());
        let fixture = Fixture::new(vault, rejected_body);
        let ProductClosureCheck::Rejected(rejection) = fixture
            .check(&proof, ModelCheckLimits::default())
            .expect("body verdict")
        else {
            panic!("bad relation cannot certify");
        };
        assert_eq!(rejection.rule, rule);
        let CertificateLocus::HyperTransitionTuple(tuple) = &rejection.at else {
            panic!("body locus must retain edges");
        };
        let edges = &closure(&proof)[0].executions;
        let second = if rule == CertificateRule::ViolatingMember {
            1
        } else {
            0
        };
        assert_eq!(
            tuple.transitions,
            [
                edges[0].transitions[0].clone(),
                edges[1].transitions[second].clone()
            ]
        );
        if rule == CertificateRule::ViolatingMember {
            assert_eq!(
                tuple.transitions[0].pre_state,
                tuple.transitions[1].pre_state
            );
            assert_eq!(
                tuple.transitions[0].transition,
                tuple.transitions[1].transition
            );
            assert_ne!(
                tuple.transitions[0].post_state,
                tuple.transitions[1].post_state
            );
        }
        let wire = serde_json::to_value(&rejection.at).expect("complete tuple wire");
        assert_eq!(wire.as_object().expect("tuple object").len(), 1);
        for edge in wire["transitions"].as_array().expect("labelled edges") {
            assert_eq!(edge.as_object().expect("edge object").len(), 4);
            assert!(edge["pre_state"].is_string());
            assert!(edge["post_state"].is_string());
            assert_eq!(edge["transition"]["type"], "transition");
            assert!(edge["result"].is_null());
        }
        let record = fixture.settle(proof, ModelCheckLimits::default());
        assert!(matches!(
            record.value,
            TerminalValue::Inconclusive(fsl_replay::InconclusiveCause::CertificateRejected { .. })
        ));
    }
}

#[trace("FR-163-AC-12")]
#[test]
fn hp1_coalesced_results_and_typed_null_remain_distinct() {
    let vault = Vault::Custom("post Step using v on V::Vault::step { self.l = 0 }\n");
    let true_text = "relation Result using v over (x: V::Vault::step, y: V::Vault::step) { true }";
    let false_text = "relation Result using v over (x: V::Vault::step, y: V::Vault::step) { x.result = y.result }";
    let proof = Fixture::from_parts(package_and_relation_with_document(
        step_with_result_document(),
        vault,
        true_text,
    ))
    .proof(ModelCheckLimits::default());
    let fixture = Fixture::from_parts(package_and_relation_with_document(
        step_with_result_document(),
        vault,
        false_text,
    ));
    let ProductClosureCheck::Rejected(rejection) = fixture
        .check(&proof, ModelCheckLimits::default())
        .expect("result check")
    else {
        panic!("unequal results refute");
    };
    assert_eq!(rejection.rule, CertificateRule::ViolatingMember);
    let CertificateLocus::HyperTransitionTuple(tuple) = rejection.at else {
        panic!("complete result tuple");
    };
    let [first, second] = tuple.transitions.as_slice() else {
        panic!("two executions");
    };
    assert_eq!(first.pre_state, second.pre_state);
    assert_eq!(first.transition, second.transition);
    assert_eq!(first.post_state, second.post_state);
    assert_ne!(first.result, second.result);
    let wire = serde_json::to_value(&tuple).expect("typed result tuple");
    assert_eq!(
        wire["transitions"][0]["result"],
        serde_json::json!({"type":"integer","value":"0"})
    );
    assert_eq!(
        wire["transitions"][1]["result"],
        serde_json::json!({"type":"integer","value":"1"})
    );
    let mut absent = first.clone();
    absent.result = None;
    let mut null = absent.clone();
    null.result = Some(r#"{"type":"null"}"#.to_owned());
    assert_ne!(absent, null);
    assert_eq!(
        serde_json::to_value(absent).expect("absent result")["result"],
        serde_json::Value::Null
    );
    assert_eq!(
        serde_json::to_value(null).expect("typed null result")["result"],
        serde_json::json!({"type":"null"})
    );
}

#[trace("FR-163-AC-4", "FR-182-AC-17")]
#[test]
fn hp1_checker_state_transition_and_cancellation_stops_never_certify() {
    let fixture = Fixture::new(Vault::Secure, DETERMINISTIC);
    let proof = fixture.proof(ModelCheckLimits::default());
    for (limit, states, transitions) in [
        (ModelCheckLimit::MaxStates, 1, 100),
        (ModelCheckLimit::MaxTransitions, 100, 1),
    ] {
        let limits = ModelCheckLimits {
            limits: fsl_eval::simulation::Limits {
                max_states: states,
                max_transitions: transitions,
            },
            ..ModelCheckLimits::default()
        };
        assert_eq!(
            fixture.check(&proof, limits).expect("graph limit"),
            ProductClosureCheck::Stopped {
                cause: IncompleteCause::LimitReached { limit, value: 1 },
                tuple: None
            }
        );
        let record = fixture.settle(proof.clone(), limits);
        assert_eq!(
            record.value,
            TerminalValue::Incomplete(IncompleteCause::LimitReached { limit, value: 1 })
        );
        assert_eq!(record.count, None);
        assert!(record.counterexample.is_none());
    }
    let cancel = Cancel::new();
    cancel.cancel(filament_exact::CancelCause::Requested);
    let subjects = fixture.subjects();
    let result = check_product_closure(
        &ProductClosureRequest {
            package: &fixture.package,
            clause: ProductClosureClause::Relation(&fixture.relation),
            subjects: &subjects,
            meter: UNLIMITED,
        },
        &proof,
        &ModelCheckLimits::default(),
        &cancel,
    )
    .expect("cancellation result");
    assert!(matches!(
        result,
        ProductClosureCheck::Stopped {
            cause: IncompleteCause::Cancelled { .. },
            tuple: None
        }
    ));
}
