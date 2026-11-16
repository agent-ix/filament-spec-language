// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1040 steps 1 to 4 (FR-107-AC-7 to AC-11): a clause evaluation's receiver
//! path and the runtime value path of each field read, over FR-108's `Config`
//! unit. A path is `(observation, subject, steps)`.

use super::clause_set::{admit, documents, Documents};
use super::frame_replay::{config_version_unit, unit_for, Unit};
use super::*;
use crate::spine::{CallOutcome, CallRefusal, ClauseRunReport};
use fsl_eval::value::ValuePathSubject;
use fsl_eval::value::{ClauseEvaluation, ObservationIdentity, RuntimeValuePath, ValuePathStep};

const UNLIMITED: u64 = 1_000_000;

/// The identity of the document `name` (`config_version_document_ref`).
fn identity(name: &str) -> ObservationIdentity {
    ObservationIdentity {
        authority: "test".to_owned(),
        identity: name.to_owned(),
    }
}

fn path(
    observation: &str,
    subject: &ValuePathSubject,
    steps: Vec<ValuePathStep>,
) -> RuntimeValuePath {
    RuntimeValuePath {
        observation: identity(observation),
        subject: subject.clone(),
        steps,
    }
}

fn member(name: &str) -> ValuePathStep {
    ValuePathStep::Member(name.to_owned())
}

fn evaluate(
    unit: &Unit,
    clause: &str,
    observations: &AdmittedObservations,
    work_units: u64,
) -> ClauseEvaluation {
    unit.compiled
        .package
        .evaluate_clause(
            &QualifiedName::unqualified(clause).unwrap(),
            observations,
            &mut Meter::new(default_accounting(work_units)),
        )
        .expect("the clause evaluates")
}

fn read_paths(evaluation: &ClauseEvaluation) -> Vec<Option<RuntimeValuePath>> {
    evaluation
        .reads()
        .iter()
        .map(|read| read.path.clone())
        .collect()
}

fn object(observations: &AdmittedObservations) -> ValuePathSubject {
    ValuePathSubject::Object(observations.self_object.clone())
}

/// TC-1040 steps 1 and 2: `ParentOrder` over `H` (root 1, child 2 naming
/// root) and `A` (root alone).
#[trace("TC-1040", "FR-107-AC-7", "FR-107-AC-8", "FR-107-AC-9")]
#[test]
fn an_object_receivers_reads_are_located_at_its_declared_identity() {
    let unit = config_version_unit();
    let mut docs = documents();
    let h = docs.current_snapshot("H", 1, Some(2));
    let a = docs.current_snapshot("A", 1, None);
    let over = |docs: &Documents, snapshot: &DocumentRef, key: &str| {
        admit(
            &unit,
            docs,
            "ParentOrder",
            config_version_current_selection("ParentOrder", snapshot.clone(), key),
        )
    };
    let child = over(&docs, &h, "child");
    let root = over(&docs, &h, "root");
    let (child, root) = (object(&child), object(&root));

    let evaluation = evaluate(&unit, "ParentOrder", &over(&docs, &h, "child"), UNLIMITED);
    assert_eq!(evaluation.receiver_path(), Some(&path("H", &child, vec![])));
    let parent = path("H", &child, vec![member("parent")]);
    assert_eq!(
        read_paths(&evaluation),
        vec![
            Some(parent.clone()),
            Some(parent),
            Some(path(
                "H",
                &child,
                vec![
                    member("parent"),
                    ValuePathStep::OptionValue,
                    ValuePathStep::Reference(match &root {
                        ValuePathSubject::Object(reference) => reference.clone(),
                        other => panic!("root is an object subject: {other:?}"),
                    }),
                    member("versionNumber"),
                ],
            )),
            Some(path("H", &child, vec![member("versionNumber")])),
        ]
    );
    let locations: Vec<_> = evaluation.reads().iter().map(|r| &r.location).collect();
    assert_ne!(locations[0], locations[1], "two different selection nodes");

    // Over `A`, `implies` skips its right operand.
    let absent = over(&docs, &a, "root");
    let evaluation = evaluate(&unit, "ParentOrder", &absent, UNLIMITED);
    assert_eq!(
        read_paths(&evaluation),
        vec![Some(path("A", &object(&absent), vec![member("parent")]))]
    );

    // A zero work budget denies a charge before any read.
    let evaluation = evaluate(&unit, "ParentOrder", &over(&docs, &h, "child"), 0);
    assert!(evaluation.reads().is_empty());
}

/// TC-1040 step 3: `VersionUnchanged` over changed-version.
#[trace("TC-1040", "FR-107-AC-9")]
#[test]
fn a_read_under_pre_is_located_in_the_pre_observation() {
    let unit = config_version_unit();
    let mut docs = documents();
    let invocation = docs.invocation("changed", 2, 3);
    let observations = admit(
        &unit,
        &docs,
        "VersionUnchanged",
        config_version_invocation_selection("VersionUnchanged", invocation),
    );
    let child = object(&observations);
    let evaluation = evaluate(&unit, "VersionUnchanged", &observations, UNLIMITED);
    assert_eq!(
        read_paths(&evaluation),
        vec![
            Some(path("changed-post", &child, vec![member("versionNumber")])),
            Some(path("changed-pre", &child, vec![member("versionNumber")])),
        ]
    );
}

/// TC-1040 step 4: `ProbeOrdered` over an invocation and over a pre-call
/// observation.
#[trace("TC-1040", "FR-107-AC-7", "FR-107-AC-10")]
#[test]
fn a_parameter_read_is_rooted_at_the_invocation_and_absent_without_one() {
    let (base, _) = config_version_step3_unit_and_packages();
    let unit = unit_for(
        format!(
            "{base}pre ProbeOrdered using v on Config::ConfigVersion::probe {{ \
             self.versionNumber < target.versionNumber }}\n"
        ),
        config_version_step3_domain_document(),
    );
    let version = |key: &str, version: &str| {
        let mut object = config_version_step3_object(key, None);
        object["fields"]["versionNumber"] = json!({"integer": version});
        object
    };
    let objects = [version("a", "1"), version("b", "2")];
    let (w, w_bytes) = document_ref_and_bytes("W", |label| {
        config_version_step3_snapshot(label, "pre", &objects)
    });
    let (post, post_bytes) = document_ref_and_bytes("W-post", |label| {
        config_version_step3_snapshot(label, "post", &objects)
    });
    let (i1, i1_bytes) = document_ref_and_bytes("i1", |label| {
        config_version_probe_invocation_bytes(label, &w, &post, "a", "b")
    });
    let mut docs = documents();
    docs.snapshots.insert(w.clone(), w_bytes);
    docs.snapshots.insert(post.clone(), post_bytes);
    docs.invocations.insert(i1.clone(), i1_bytes);

    let by_invocation = admit(
        &unit,
        &docs,
        "ProbeOrdered",
        config_version_invocation_selection("ProbeOrdered", i1),
    );
    let by_pre_call = admit(
        &unit,
        &docs,
        "ProbeOrdered",
        ClauseSelection {
            name: "ProbeOrdered".to_owned(),
            input: ClauseSelectionInput::PreCall {
                snapshot: w,
                self_object: Some(SelectedObject {
                    population: config_version_population_identity(),
                    key: "a".to_owned(),
                }),
                parameters: BTreeMap::from([(
                    filament_exact::Identifier::new("target").unwrap(),
                    fsl_semantics::model::observation::SnapshotValue::reference(SelectedObject {
                        population: config_version_population_identity(),
                        key: "b".to_owned(),
                    }),
                )]),
            },
        },
    );
    let a = object(&by_invocation);
    let Some((_, Value::Reference(b))) = by_invocation.parameters.first() else {
        panic!("target is a reference parameter");
    };
    let own = path("W", &a, vec![member("versionNumber")]);

    let evaluation = evaluate(&unit, "ProbeOrdered", &by_invocation, UNLIMITED);
    assert_eq!(evaluation.receiver_path(), Some(&path("W", &a, vec![])));
    assert_eq!(
        read_paths(&evaluation),
        vec![
            Some(own.clone()),
            Some(path(
                "W",
                &ValuePathSubject::Invocation {
                    invocation: identity("i1"),
                    binding: fsl_eval::value::InvocationBinding::Parameter("target".to_owned()),
                },
                vec![ValuePathStep::Reference(b.clone()), member("versionNumber")],
            )),
        ]
    );

    let evaluation = evaluate(&unit, "ProbeOrdered", &by_pre_call, UNLIMITED);
    assert_eq!(evaluation.receiver_path(), Some(&path("W", &a, vec![])));
    assert_eq!(read_paths(&evaluation), vec![Some(own), None]);
}

/// FR-766-AC-5 (TC-2195): the `AtMostOne` unit, whose `count<One>` over a
/// two-element sequence can reach 2, with `One` spelled `one`.
fn at_most_one(one: &str) -> (String, fsl_semantics::model::intake::PackageInput) {
    let (base, packages) = config_version_unit_and_packages();
    let unit = format!(
        "{base}type One = {one};\n\
         function pair using v(a: Int[0, 1000]): Sequence<Int[0, 1000]>[0, 2] pure {{ sequence[a, a] }}\n\
         invariant AtMostOne using v on Config::ConfigVersion at current \
         {{ count<One>(m in pair(self.versionNumber): m >= 0) <= 1 }}\n"
    );
    (unit, packages)
}

/// The run item a clause run of `clause` builds from `report`, as JSON.
fn run_item(clause: &str, report: &ClauseRunReport) -> serde_json::Value {
    let item = crate::OutcomeItem::from_clause_run(crate::RequestIndex::new(0), clause, report)
        .expect("a refusal is an item");
    serde_json::to_value(&item).expect("the item serializes")
}

/// FR-766-AC-5 (TC-2195): a clause run compiles its unit under
/// `CheckMode::Linked`, so an invariant whose `count<One>` can exceed `One`
/// is refused at compile with a named cause, never run to a null-cause
/// refusal; with a domain that holds the count it compiles.
#[trace("TC-2195", "FR-766-AC-5")]
#[test]
#[ignore = "QSL-2003: a linked count<N> result range is not proved at check yet; when it is, move the temporal-atom count<Zero> run-time-refusal fixtures to a package checked under CheckMode::Kernel (FR-766 Notes): fsl-replay/src/spine/clause/tests/temporal.rs:35-141,363-473, fsl-analyze/tests/it/monitor_inputs.rs:45-55,478; the invariant RefusedDomain fixture (witness.rs:84,1736, tc_743_the_separation_check_names_its_failing_step) moves its count<Zero> into a called function z (FR-268-AC-3) under CheckMode::Kernel and keeps its step-2 refusal record"]
fn count_out_of_domain_names_its_cause() {
    let compile = |one: &str| {
        let (unit, packages) = at_most_one(one);
        compose(
            source(),
            super::frame_replay::IDENTITY,
            unit.as_bytes(),
            &packages,
            &DependencyInput::default(),
            SpineLimits::default(),
        )
    };
    let refusal = compile("Int[0, 1]").expect_err("the count can reach 2, outside Int[0, 1]");
    assert_eq!(refusal.code().as_str(), "undefined_expression");
    assert_eq!(refusal.cause(), Some("unproved-range"));
    assert_eq!(refusal.code().category().exit_code(), 20);
    assert!(
        compile("Int[0, 2]").is_ok(),
        "a domain holding [0, 2] proves the count"
    );
}

/// FR-766-AC-6 (TC-2195): the clause run of that unit reports a compile
/// disposition whose run item names `unproved-range`.
#[trace("TC-2195", "FR-766-AC-6")]
#[test]
#[ignore = "QSL-2003: a linked count<N> result range is not proved at check yet; when it is, move the temporal-atom count<Zero> run-time-refusal fixtures to a package checked under CheckMode::Kernel (FR-766 Notes): fsl-replay/src/spine/clause/tests/temporal.rs:35-141,363-473, fsl-analyze/tests/it/monitor_inputs.rs:45-55,478; the invariant RefusedDomain fixture (witness.rs:84,1736, tc_743_the_separation_check_names_its_failing_step) moves its count<Zero> into a called function z (FR-268-AC-3) under CheckMode::Kernel and keeps its step-2 refusal record"]
fn a_count_refused_at_compile_is_a_declined_run_item_with_its_cause() {
    let (unit, packages) = at_most_one("Int[0, 1]");
    let selection = ClauseSelection {
        name: "AtMostOne".to_owned(),
        input: ClauseSelectionInput::Current {
            snapshot: config_version_document_ref("snap"),
            anchor: SelectedAnchor {
                kind: fsl_semantics::model::observation::AnchorKind::Initialization,
                name: "init".to_owned(),
            },
            self_object: None,
        },
    };
    let request = config_version_request_for(
        unit,
        packages,
        "at-most-one.native",
        ClauseRunSelection::Clause(selection),
    );
    let report = run_clause(request).expect("a report");
    assert_eq!(report.disposition.stage(), ClauseRunStage::Compile);
    let item = run_item("AtMostOne", &report);
    assert_eq!(item["category"], "refusal", "{item}");
    assert_eq!(item["result"], "declined", "{item}");
    assert_eq!(item["cause"]["code"], "undefined_expression", "{item}");
    assert_eq!(item["cause"]["kind"], "unproved-range", "{item}");
}

/// FR-766-AC-7 (TC-2195): a clause-run report whose S6a outcome is a kernel
/// refusal record projects that record's code and cause onto its run item.
#[trace("TC-2195", "FR-766-AC-7")]
#[test]
#[ignore = "QSL-2003: a clause-run item over a kernel refusal record does not carry its cause yet"]
fn a_kernel_refusal_record_names_its_cause_on_the_run_item() {
    use filament_semantic_value::catalog::CatalogCode;
    let report = ClauseRunReport {
        usage: Default::default(),
        basis: crate::proof_result::SettlementBasis::Unavailable,
        witness: None,
        consequent_source_events: Default::default(),
        finite_window: None,
        counterexample: None,
        decisive: None,
        authenticated_temporal_selection: None,
        refused_at: None,
        ledger: None,
        disposition: ClauseDisposition::Evaluate(CallOutcome::Refused(CallRefusal::Record {
            code: CatalogCode::new("integer_out_of_domain", "outside-domain"),
            fields: [("expected", "Int[0, 1]".to_owned())].into_iter().collect(),
            locus: None,
            location: None,
        })),
    };
    let item = run_item("AtMostOne", &report);
    assert_eq!(item["category"], "refusal", "{item}");
    assert_eq!(item["result"], "declined", "{item}");
    assert_eq!(item["cause"]["code"], "integer_out_of_domain", "{item}");
    assert_eq!(item["cause"]["kind"], "outside-domain", "{item}");
}
