// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent certificate checks starting from real EN-5 proof evidence.

use super::*;
use fsl_replay::{
    check_probability_certificate_document, read_probability_certificate,
    write_probability_certificate, ProbabilityDocumentError, ProbabilityDocumentLimit,
    ProbabilityDocumentLimits,
};

fn limits() -> ProbabilityDocumentLimits {
    ProbabilityDocumentLimits {
        bytes: 1_000_000,
        storage_bytes: 8_000_000,
        states: 128,
        edges: 1024,
        scalar_bits: 128,
        accounting: fsl_replay::AccountingLimits::default().0,
    }
}

#[trace("FR-201-AC-1", "FR-201-AC-6", "FR-201-AC-8", "TC-1197")]
#[test]
fn changing_the_one_loss_value_rejects_at_the_first_failed_bellman_state() {
    let world = World::new(&prob_world::link(CLAIMS));
    let original = settle(&world, "Likely", ModelCheckLimits::default());
    let evidence = original
        .exact_probabilistic
        .as_ref()
        .expect("real accepted evidence");
    let expected = evidence
        .certificate
        .values
        .iter()
        .filter(|row| row.value == ratio(4, 5) || row.value == ratio(24, 25))
        .map(|row| row.state.clone())
        .min()
        .expect("the one-loss state and its predecessor");
    let engines = Engines::fsl().expect("linked engines");
    let item = exact_probabilistic_item_tampered(
        RequestIndex::new(0),
        world.request("Likely", ModelCheckLimits::default()),
        |certificate| {
            let row = certificate
                .values
                .iter_mut()
                .find(|row| row.value == ratio(4, 5))
                .expect("one loss leaves delivery probability four fifths");
            row.value = ratio(9, 10);
        },
    );
    let record = analyze_items(&engines, vec![item], &Cancel::new(), fsl_replay::CertificationPolicy::AdmitAny).remove(0);
    assert_eq!(
        record.terminal().expect("terminal").value(),
        &TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
            rule: CertificateRule::InequalityFails,
            at: CertificateLocus::ProbabilityProductState(expected),
        })
    );
    assert!(record.exact_probabilistic.is_none());
    assert!(record.probabilistic_counterexample.is_none());
}

#[trace("FR-201-AC-9")]
#[test]
fn engine_certificate_transport_preserves_all_rows_targets_ranks_and_identity() {
    let world = World::new(&prob_world::link(CLAIMS));
    let record = settle(&world, "Likely", ModelCheckLimits::default());
    let bytes = record
        .terminal()
        .expect("proved terminal")
        .exact_certificate_document()
        .expect("actual produced document");
    let parsed =
        read_probability_certificate(bytes, limits(), &Cancel::new()).expect("admitted document");
    let evidence = record
        .exact_probabilistic
        .as_ref()
        .expect("real certificate");
    assert_eq!(parsed.values, evidence.certificate.values);
    assert_eq!(
        parsed.identity,
        fsl_replay::probabilistic_claim_identity(world.claim("Likely")).unwrap()
    );
    assert!(!parsed.targets.is_empty());
    assert_eq!(parsed.ranking.len(), parsed.values.len());
    let written = write_probability_certificate(&parsed, limits(), &Cancel::new()).expect("writes");
    assert_eq!(written, bytes);
    let system = ModelSystem::new(world.subject().simulation).expect("subject admits");
    let checked = check_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &parsed.identity,
        &written,
        limits(),
        &Cancel::new(),
    )
    .expect("independent checker");
    let ExactProbabilityCertificateCheck::Accepted(accepted) = checked else {
        panic!("transported genuine evidence must be accepted: {checked:?}");
    };
    assert_eq!(accepted.value(), &ratio(24, 25));
    assert_eq!(accepted.side(), ProbabilityBoundSide::AtLeast);
}

#[trace("FR-201-AC-10", "FR-201-AC-6")]
#[test]
fn correct_bellman_values_cannot_hide_broken_positive_progress_ranks() {
    let world = World::new(&prob_world::link(CLAIMS));
    let record = settle(&world, "Likely", ModelCheckLimits::default());
    let bytes = record
        .terminal()
        .expect("terminal")
        .exact_certificate_document()
        .expect("document");
    let mut document =
        read_probability_certificate(bytes, limits(), &Cancel::new()).expect("reads");
    let original_values = document.values.clone();
    let expected = document
        .ranking
        .iter()
        .filter(|(_, rank)| *rank > 0)
        .map(|(state, _)| state.clone())
        .min()
        .expect("live positive-probability states");
    for (_, rank) in &mut document.ranking {
        *rank = 0;
    }
    assert_eq!(
        document.values, original_values,
        "only the progress evidence changed"
    );
    let bytes = write_probability_certificate(&document, limits(), &Cancel::new())
        .expect("canonical writes");
    let system = ModelSystem::new(world.subject().simulation).expect("subject admits");
    assert_eq!(
        check_probability_certificate_document(
            &system,
            world.package.graph().probabilistic_model().claims(),
            &document.identity,
            &bytes,
            limits(),
            &Cancel::new()
        )
        .expect("typed rejection"),
        ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
            rule: ProbabilityCertificateRule::RankBroken,
            at: ProbabilityCertificateLocus::ProductState(expected),
        })
    );
}

#[trace("FR-201-AC-11")]
#[test]
fn a_real_certificate_obeys_its_exact_byte_ceiling_without_truncation() {
    let world = World::new(&prob_world::link(CLAIMS));
    let record = settle(&world, "Likely", ModelCheckLimits::default());
    let bytes = record
        .terminal()
        .expect("terminal")
        .exact_certificate_document()
        .expect("document");
    let required = u64::try_from(bytes.len()).expect("document size fits u64");
    let exact = ProbabilityDocumentLimits {
        bytes: required,
        ..limits()
    };
    let document =
        read_probability_certificate(bytes, exact, &Cancel::new()).expect("exact ceiling admits");
    assert_eq!(
        write_probability_certificate(&document, exact, &Cancel::new()).expect("exact writes"),
        bytes
    );
    assert_eq!(
        read_probability_certificate(
            bytes,
            ProbabilityDocumentLimits {
                bytes: required - 1,
                ..limits()
            },
            &Cancel::new()
        ),
        Err(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::Bytes,
            bound: required - 1,
            actual: required
        })
    );
}
