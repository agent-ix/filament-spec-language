// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 AC-16/AC-17 seeds on the timed routes. Each seed is a forged,
//! structurally self-consistent certificate for a false claim that the
//! checker must reject by a semantic rule. The truth of each claim is
//! computed by the definitional reference from the checked package, never by
//! the analyzer.

use filament_exact::{Cancel, Integer, Rational};
use fsl_analyze::analyze::{
    analyze_items, exact_probabilistic_item, exact_probabilistic_item_tampered, AnalyzeRecord,
    Engines,
};
use fsl_analyze::exact_probabilistic::{ExactProbLimits, ProductRequest};
use fsl_eval::model_subject::ModelSubject;
use fsl_foundation::RequestIndex;
use fsl_replay::{CertificateRule, ExactProbabilityCertificate, InconclusiveCause, TerminalValue};

use crate::timed_world::{self, World};

const PROFILES: &str = "profile ev = \"filament.temporal.timed/v1\";\n";

pub(crate) fn rational(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn product_request<'a>(world: &'a World, claim: &str) -> ProductRequest<'a> {
    ProductRequest {
        subject: ModelSubject {
            simulation: world.request(),
            proof_bounds: Vec::new(),
        },
        claim: world
            .package
            .graph()
            .probabilistic_model()
            .claim(claim)
            .expect("the claim checks"),
        limits: ExactProbLimits::default(),
    }
}

/// What analysis settles for `claim`.
pub(crate) fn settle(world: &World, claim: &str) -> AnalyzeRecord {
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let item = exact_probabilistic_item(RequestIndex::new(0), product_request(world, claim));
    analyze_items(
        &engines,
        vec![item],
        &Cancel::new(),
        fsl_replay::CertificationPolicy::AdmitAny,
    )
    .remove(0)
}

pub(crate) fn is_proved(record: &AnalyzeRecord) -> bool {
    matches!(
        record.terminal().map(|terminal| terminal.value()),
        Some(TerminalValue::Proved { .. })
    )
}

/// QSL-3444's model: `first` is guarded by `x in (0, 1) ms` and resets `x`;
/// `third` sets `late` when `x < 1 ms and y > third_floor`; `y <= 3 ms` is
/// an invariant. The claim is `probability <= 1/2` of `late` within 4 ms over
/// every scheduler.
fn two_clock_world(third_floor: &str) -> World {
    let body = format!(
        "{}pre First using v on M::Call::first {{ self.x > 0 ms and self.x < 1 ms }}\n\
         post FirstDone using v on M::Call::first {{ self.x = 0 ms }}\n\
         pre Third using v on M::Call::third {{ self.x < 1 ms and self.y > {third_floor} }}\n\
         post ThirdDone using v on M::Call::third {{ self.late = true }}\n\
         time invariant Cap when {{ true }} {{ self.y <= 3 ms }}\n\
         probabilistic Max using ev on M over (c: M::Call) under every scheduler on origin \
         {{ probability <= 0.5 [ eventually[0 ms, 4 ms] holds(c.late) ] }}\n",
        timed_world::DENSE,
    );
    timed_world::world(
        &timed_world::document(&["x", "y"], &[("first", &["x"]), ("third", &["late"])]),
        PROFILES,
        &body,
    )
}

/// FR-1051-AC-17, seed QSL-3444 (R-18, region): the claim
/// `probability <= 1/2` of `eventually[0 ms, 4 ms] holds(c.late)` over every
/// scheduler is false, because `first` at 1/2 ms and `third` at 6/5 ms meet
/// both strict guards, which gives value 1. The reference fires that witness
/// here, and analysis does not settle the
/// claim `proved`. The checker half of the seed is the ignored test below.
#[test]
fn seed_qsl_3444_the_reference_fires_the_strict_witness_and_analysis_does_not_prove_the_claim() {
    let falsity = two_clock_world("1 ms");
    let timed = &falsity.package.graph().timed_models()[0];
    let start = fsl_definitional_reference::ReferenceClockState::new(
        timed,
        vec![rational(0, 1), rational(0, 1)],
    )
    .expect("both clocks start at zero");
    let after_first = start
        .fire(timed, &rational(1, 2), "first")
        .expect("the reference fires first")
        .expect("first is enabled at 1/2 ms");
    // Hand derivation: x was reset at t = 1/2, so at t = 6/5 x = 7/10 < 1 and
    // y = 6/5 > 1, and 6/5 <= 3 meets the invariant: third is enabled.
    let after_third = after_first
        .fire(timed, &rational(7, 10), "third")
        .expect("the reference fires third")
        .expect("third is enabled 7/10 ms later");
    assert_eq!(after_third.values(), &[rational(7, 10), rational(6, 5)]);
    assert!(!is_proved(&settle(&falsity, "Max")), "value 1 exceeds 1/2");
}

/// The settlement of `claim` where the certificate of the claim's own graph
/// is built even though the bound fails and `tamper` alters it.
fn settle_tampered(
    world: &World,
    claim: &str,
    tamper: impl Fn(&mut ExactProbabilityCertificate),
) -> AnalyzeRecord {
    let engines = Engines::fsl().expect("the linked engines are admitted");
    let item = exact_probabilistic_item_tampered(
        RequestIndex::new(0),
        product_request(world, claim),
        tamper,
    );
    analyze_items(
        &engines,
        vec![item],
        &Cancel::new(),
        fsl_replay::CertificationPolicy::AdmitAny,
    )
    .remove(0)
}

/// FR-1051-AC-17, seed QSL-3444, the checker half. The falsity world's claim
/// is false (value 1, above). The certificate of that world's own graph, with
/// every value set to 0 as the wrong maximum the bug produced, is
/// structurally self-consistent: the same states, targets and objective. The
/// checker rejects it by `InequalityFails`, a one-step equation that does not
/// hold: a target state must be worth 1, and the claim does not settle
/// `proved`.
#[test]
fn seed_qsl_3444_a_certificate_of_maximum_zero_is_rejected_for_the_strict_two_clock_claim() {
    let falsity = two_clock_world("1 ms");
    let record = settle_tampered(&falsity, "Max", |certificate| {
        for entry in &mut certificate.values {
            entry.value = rational(0, 1);
        }
    });
    let terminal = record.terminal().expect("the claim settles");
    assert!(
        matches!(
            terminal.value(),
            TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
                rule: CertificateRule::InequalityFails,
                ..
            })
        ),
        "the value-0 certificate of the falsity world is rejected by a semantic rule: {:?}",
        terminal.value()
    );
}
