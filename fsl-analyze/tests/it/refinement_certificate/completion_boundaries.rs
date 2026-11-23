// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact certificate resource boundaries, including terminal stutter edges.

use super::*;

#[trace("FR-149-AC-1")]
#[test]
fn each_listed_position_counts_at_the_exact_state_budget() {
    let world = safety_only();
    let (certificate, _) = certificate(&world);
    let count = certificate.positions.len();
    assert!(count > 1);
    for maximum in [0, 1, count - 1, count, count + 1] {
        let mut limits = ModelCheckLimits::default();
        limits.limits.max_states = maximum;
        let actual = check_with(&world, &certificate, &limits);
        let expected = if maximum < count {
            TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
                limit: ModelCheckLimit::MaxStates,
                value: u64::try_from(maximum).expect("small fixture count"),
                required: None,
            })
        } else {
            TemporalCertificateCheck::Accepted
        };
        assert_eq!(
            actual, expected,
            "listed positions={count}, budget={maximum}"
        );
    }
}

#[trace("FR-149-AC-1", "FR-149-AC-4")]
#[test]
fn the_last_listed_edge_requires_its_own_transition_unit() {
    let world = safety_only();
    let (certificate, _) = certificate(&world);
    let edges: usize = certificate.successors.iter().map(Vec::len).sum();
    assert!(edges > 1);
    for maximum in [0, 1, edges - 1, edges, edges + 1] {
        let mut limits = ModelCheckLimits::default();
        limits.limits.max_transitions = maximum;
        let actual = check_with(&world, &certificate, &limits);
        let expected = if maximum < edges {
            TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
                limit: ModelCheckLimit::MaxTransitions,
                value: u64::try_from(maximum).expect("small fixture count"),
                required: None,
            })
        } else {
            TemporalCertificateCheck::Accepted
        };
        assert_eq!(actual, expected, "listed edges={edges}, budget={maximum}");
    }
}

#[trace("FR-149-AC-7")]
#[test]
fn malformed_successor_shape_is_rejected_before_zero_recompute_budgets() {
    let world = safety_only();
    let (mut certificate, _) = certificate(&world);
    certificate
        .successors
        .pop()
        .expect("a nonempty certificate");
    let first_missing = u64::try_from(certificate.successors.len()).expect("fixture count");
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_states = 0;
    limits.limits.max_transitions = 0;
    assert_eq!(
        rejected(check_with(&world, &certificate, &limits)),
        (CertificateRule::Malformed, step(first_missing, None))
    );
}

#[trace("FR-149-AC-6", "FR-149-AC-4")]
#[test]
fn a_terminal_stutter_edge_is_not_free_at_the_transition_ceiling() {
    let world = terminal_world();
    let (certificate, _) = certificate(&world);
    let edges: usize = certificate.successors.iter().map(Vec::len).sum();
    let stutters = certificate
        .successors
        .iter()
        .flatten()
        .filter(|edge| edge.transition.is_none())
        .count();
    assert_eq!(stutters, 1, "the chain has one terminal state");
    let mut limits = ModelCheckLimits::default();
    limits.limits.max_transitions = edges - stutters;
    assert_eq!(
        check_with(&world, &certificate, &limits),
        TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxTransitions,
            value: u64::try_from(edges - stutters).expect("fixture count"),
            required: None,
        })
    );
    limits.limits.max_transitions = edges;
    assert_eq!(
        check_with(&world, &certificate, &limits),
        TemporalCertificateCheck::Accepted
    );
}
