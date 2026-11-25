// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{ffi::OsString, num::NonZeroUsize, path::Path, time::Duration};

use filament_contract_codegen::smt::temporal::{
    run_and_settle_temporal_smt_proof, TemporalSmtProofResult,
};
use filament_contract_codegen::smt::ProcessLimits;
use filament_exact::Cancel;
use fsl_replay::{CertificateCheckLimits, ModelCheckItem, SmtProofBasis, SmtQueryKind};
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
    let clause = &package.graph().temporal_clauses()[0];
    let letters = crate::counter::Letters {
        atom: |value| Some(value <= 3),
    };
    let request = world.request_in(
        &package,
        ModelCheckItem::Clause {
            clause: clause.clause(),
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
    assert_eq!(
        settlement.value(),
        &fsl_replay::TerminalValue::Proved {
            basis: fsl_replay::ProofBasis::BoundedComplete { depth: 3 },
            certification: fsl_replay::Certification::Certified,
        }
    );
    assert_eq!(
        settlement.checker(),
        Some(fsl_replay::CertificateChecker::SmtProof)
    );
    let certificate = fsl_analyze::analyze::document::smt_proof_certificate_member(
        serde_json::json!("test/counter/Bounded"),
        &settlement,
    )
    .expect("the production FR-621 projection writes verified SMT evidence");
    assert_eq!(certificate["checker"], "smt-proof");
    assert_eq!(certificate["accepted"], true);
}
