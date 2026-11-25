// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{ffi::OsString, num::NonZeroUsize, path::Path, time::Duration};

use filament_contract_codegen::smt::temporal::{
    run_and_settle_temporal_smt_proof, TemporalSmtProofResult,
};
use filament_contract_codegen::smt::ProcessLimits;
use filament_exact::Cancel;
use fsl_analyze::analyze::document::{
    settle_run_with_smt_proofs, AnalyzeRun, ResolvedClaim, ResolvedItem, SmtProofEvidence,
    SubjectSelection,
};
use fsl_analyze::analyze::{
    AnalyzeLimits, ClaimItem, Engines, ItemOptions, SourceProvision, SubjectBinding,
};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, RequestIndex};
use fsl_replay::{CertificateCheckLimits, ModelCheckItem, SmtProofBasis, SmtQueryKind};
use fsl_route::request::ExtentClassification;
use fsl_semantics::check::Capability;
use ix_trace_rs::trace;

use super::*;

fn counter_refutation(query: &[u8]) -> String {
    let query = String::from_utf8(query.to_vec()).expect("the canonical SMT query is UTF-8");
    let assertions = query
        .lines()
        .filter_map(|line| {
            line.strip_prefix("(assert ")?
                .strip_suffix(')')
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    let final_assertion = assertions
        .last()
        .expect("the bounded query ends with its counterexample assertion");
    let mut proof = String::from("(assume negative-0 (not h0))\n");
    let mut resolution = vec!["split".to_owned(), "negative-0".to_owned()];
    for position in 0..=3 {
        let domain = format!("(and (<= 0 s{position}.c.value) (<= s{position}.c.value 3))");
        assert!(
            assertions.contains(&domain),
            "query contains Counter domain at {position}"
        );
        let range = format!("range-{position}");
        let upper = format!("upper-{position}");
        proof.push_str(&format!(
            "(assume {range} {domain})\n(step {upper} (cl (<= s{position}.c.value 3)) :rule and :premises ({range}) :args (1))\n"
        ));
        resolution.push(upper);
        if position > 0 {
            let transition = format!("(= h{position} h{})", position - 1);
            assert!(
                assertions.contains(&transition),
                "query contains {transition}"
            );
            let equality = format!("equality-{position}");
            let negative = format!("negative-{position}");
            let previous = format!("negative-{}", position - 1);
            proof.push_str(&format!(
                "(assume transition-{position} {transition})\n(step {equality} (cl h{} (not h{position})) :rule equiv2 :premises (transition-{position}))\n(step {negative} (cl (not h{position})) :rule resolution :premises ({equality} {previous}))\n",
                position - 1
            ));
            resolution.push(negative);
        }
    }
    proof.push_str(&format!(
        "(assume final {final_assertion})\n(step split (cl {}) :rule or :premises (final))\n",
        final_assertion
            .strip_prefix("(or ")
            .and_then(|terms| terms.strip_suffix(')'))
            .expect("the final query assertion is a disjunction")
    ));
    proof.push_str(&format!(
        "(step empty (cl) :rule resolution :premises ({}))\n",
        resolution.join(" ")
    ));
    proof
}

#[trace("FR-314-AC-23")]
#[test]
fn counter_smt_proof_settlement_emits_the_fr621_checker_member() {
    let world = world(CONTRACT, &[("c", 0)]);
    let package = crate::counter::bounded_package_with_clauses(
        &world.documents.document,
        &[("Bounded", "", "always[0,3] holds(c.value <= 3)")],
    );
    let clause_node = &package.graph().temporal_clauses()[0];
    let claim = OccurrenceKey::new(clause_node.identity(), clause_node.claim().clone());
    let clause = clause_node.clause();
    let letters = crate::counter::Letters {
        atom: |value| Some(value <= 3),
    };
    let request = world.request_in(
        &package,
        ModelCheckItem::Clause {
            clause,
            letters: &letters,
        },
        UNLIMITED,
    );
    let expected_query = fsl_replay::encode_smt_query(
        &request.subject,
        request.item,
        SmtQueryKind::Unrolling { depth: 3 },
        &request.smt_encoding,
    )
    .expect("the Counter TP-2 query encodes");
    let proof = counter_refutation(&expected_query);
    let solver = format!("cat >/dev/null; printf '%s\\n' 'unsat' '({proof})'");

    let cancel = Cancel::new();
    let result = run_and_settle_temporal_smt_proof(
        &request,
        SmtProofBasis::BoundedComplete { depth: 3 },
        Path::new("/bin/sh"),
        &[
            OsString::from("-c"),
            OsString::from(solver),
            OsString::from("fake-cvc5"),
            OsString::from("--dump-proofs"),
            OsString::from("--proof-format-mode=alethe"),
        ],
        ProcessLimits {
            input_bytes: NonZeroUsize::new(1 << 20).expect("positive input limit"),
            wall_clock: Duration::from_secs(2),
            output_bytes: NonZeroUsize::new(1 << 20).expect("positive output limit"),
        },
        &CertificateCheckLimits::default(),
        &cancel,
    )
    .expect("CG's temporal runner returns typed solver evidence");
    let TemporalSmtProofResult::Settled(cg_settlement) = result else {
        panic!("the solver answered unsat: {result:?}");
    };
    assert_eq!(cg_settlement.queries[0].query_bytes(), expected_query);
    assert!(cg_settlement.queries[0].proof_bytes().is_some());
    let settlement = fsl_analyze::model_check::smt::settle_temporal_smt_proof(
        &request,
        &cg_settlement.outcome,
        &CertificateCheckLimits::default(),
        &cancel,
    );
    let backend = BackendId::new("en-1");
    let evidence = SmtProofEvidence::new(backend.clone(), settlement)
        .expect("the production API admits only FR-314-verified SMT evidence");
    let evidence = [(RequestIndex::new(0), evidence)].into();

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
        Some(&backend),
    );
    let fsl_route::CandidateOutcome::Candidates(set) = &candidates else {
        panic!("the temporal satisfaction item has an engine candidate");
    };
    let subject = SubjectSelection::default_over(&world.documents.snapshots);
    let source = crate::counter::unit_text("")
        + &crate::counter::clause_text("Bounded", "", "always[0,3] holds(c.value <= 3)");
    let run = AnalyzeRun {
        package: &package,
        binding: SubjectBinding {
            packages: &world.documents.packages,
            snapshots: &world.documents.snapshots,
            initial: world.initial.clone(),
            universes: request.subject.simulation.universes.clone(),
        },
        provision: SourceProvision::single(package.graph().source(), source.as_bytes()),
        items: vec![ResolvedItem::Claim(Box::new(ResolvedClaim {
            item: ClaimItem {
                request_index: RequestIndex::new(0),
                claim,
                kind: Capability::TemporalSatisfaction,
                extent: ExtentClassification::Bounded,
                backend: Some(backend.clone()),
                candidates: set.candidates().to_vec(),
                options: ItemOptions {
                    required: true,
                    ..ItemOptions::default()
                },
                form: None,
            },
            claim_name: "Bounded".to_owned(),
            candidates: candidates.clone(),
            subject,
            unrouted: false,
        }))],
        engines: &engines,
        limits: AnalyzeLimits::unbounded(),
    };
    let settled =
        settle_run_with_smt_proofs(run, &crate::analyze_cli_support::Fr290, &cancel, &evidence)
            .expect("the full FR-621 writer settles the SMT evidence");
    let serialized = settled.to_bytes().expect("the FR-621 document serializes");
    let document: serde_json::Value =
        serde_json::from_slice(&serialized).expect("the serialized document parses");
    assert_eq!(document["format"], "filament-outcome/1");
    assert_eq!(document["items"][0]["record"]["result"], "proved");
    assert_eq!(document["items"][0]["certificate"]["checker"], "smt-proof");
    assert_eq!(document["items"][0]["certificate"]["accepted"], true);
}
