// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 AC-16/AC-17 seeds on the timed routes. Each seed is a hand-forged,
//! structurally self-consistent certificate for a false claim, and the
//! package entry `check_probability_certificate_package` (which recompiles the
//! model from bytes and takes no producer object) must reject it by a
//! semantic rule. The truth of each claim is computed by the definitional
//! reference from the checked package, never by the analyzer.

use filament_exact::{Cancel, Integer, Rational};
use fsl_analyze::analyze::{
    analyze_items, exact_probabilistic_item, AnalyzeRecord, Engines, SourceProvision,
};
use fsl_analyze::exact_probabilistic::{ExactProbLimits, ProductRequest};
use fsl_eval::model_subject::ModelSubject;
use fsl_foundation::RequestIndex;
use fsl_replay::{
    ExactProbabilityCertificateCheck, ProbabilityCertificateRejection, ProbabilityCertificateRule,
    TerminalValue,
};

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

fn document_limits() -> fsl_replay::ProbabilityDocumentLimits {
    fsl_replay::ProbabilityDocumentLimits {
        bytes: 1_000_000,
        storage_bytes: 8_000_000,
        states: 4096,
        edges: 65_536,
        scalar_bits: 128,
        accounting: fsl_replay::AccountingLimits::default().0,
    }
}

/// The checker's verdict on `certificate` for `claim`, through the package
/// entry: the emitted package bytes, the byte provision of the source and the
/// domain documents, and the claim identity. No analyzer or producer value
/// is passed.
pub(crate) fn package_check(
    world: &World,
    claim: &str,
    certificate: &[u8],
) -> Result<ExactProbabilityCertificateCheck, fsl_replay::ProbabilitySourceError> {
    let emitted = fsl_package::emit_checked(
        &world.package,
        fsl_replay::spine::SpineLimits::default().package.emit(),
    )
    .expect("the checked package emits");
    let simulation = world.request();
    let provision_source = SourceProvision::single(world.package.graph().source(), &world.unit);
    let mut documents = vec![world.document.clone()];
    documents.extend(world.snapshots.values().cloned());
    let provision = fsl_replay::ByteProvision::admit(
        provision_source
            .entries()
            .map(|(authority, identity, bytes)| {
                (authority.to_owned(), identity.to_owned(), bytes.to_vec())
            })
            .collect(),
        documents,
        std::iter::empty(),
    )
    .expect("the provision admits");
    let item = fsl_replay::probabilistic_claim_identity(
        world
            .package
            .graph()
            .probabilistic_model()
            .claim(claim)
            .expect("the claim checks"),
    )
    .expect("the claim has an identity");
    fsl_replay::check_probability_certificate_package(
        fsl_replay::ProbabilityPackageRequest {
            package: emitted.package().bytes(),
            package_name: Some("package"),
            provision: &provision,
            item: &item,
            initial: simulation.initial.clone(),
            universes: simulation.universes.clone(),
            certificate,
            role: fsl_replay::CertificateRole::Claim,
        },
        fsl_replay::ProbabilitySourceLimits {
            spine: fsl_replay::spine::SpineLimits::default(),
            observation: simulation.observation,
            expansion: simulation.expansion,
            scalar: simulation.meter,
            document: document_limits(),
        },
        &Cancel::new(),
    )
}

pub(crate) fn rejection_rule(
    checked: Result<ExactProbabilityCertificateCheck, fsl_replay::ProbabilitySourceError>,
) -> ProbabilityCertificateRule {
    match checked {
        Ok(ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
            rule,
            ..
        })) => rule,
        other => panic!("expected a semantic rejection, got {other:?}"),
    }
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

/// FR-1051-AC-17, seed QSL-3444, the checker half. Expected: the package
/// entry rejects a self-consistent value-0 certificate of the falsity world's
/// graph with a semantic rule. It is ignored because no self-consistent
/// certificate of that graph can be built yet: the checker compares the
/// certificate's `targets` with the target keys of its own graph before it
/// reads a value (`document/timed.rs`, `replay_targets != document.targets`),
/// so the proof of any other model ends in `ObjectiveMismatch`, a document
/// refusal that AC-16 does not accept, and the producer reaches its
/// certificate step only for a claim that holds. Needed: a producer hook that
/// writes the certificate of the falsity world's own graph (its keys and
/// targets) so a test can set the values to 0.
#[test]
#[ignore = "QSL-3550: no certificate of the falsity world's own graph can be built; see the doc comment"]
fn seed_qsl_3444_a_certificate_of_maximum_zero_is_rejected_for_the_strict_two_clock_claim() {
    let falsity = two_clock_world("1 ms");
    let truth = two_clock_world("3 ms");
    let record = settle(&truth, "Max");
    assert!(
        is_proved(&record),
        "a model whose third cannot fire has maximum 0"
    );
    let proof = record
        .terminal()
        .and_then(|terminal| terminal.exact_certificate_document())
        .expect("the proof of the maximum-zero model has a certificate")
        .to_vec();
    // Self-consistent for the falsity world's claim: only the claim identity
    // is retargeted, as a forger would, so the checker reaches its rules.
    let mut candidate =
        fsl_replay::read_probability_certificate(&proof, document_limits(), &Cancel::new())
            .expect("the proof is a canonical certificate document");
    candidate.identity = fsl_replay::probabilistic_claim_identity(
        falsity
            .package
            .graph()
            .probabilistic_model()
            .claim("Max")
            .expect("the claim checks"),
    )
    .expect("the claim has an identity");
    let forged =
        fsl_replay::write_probability_certificate(&candidate, document_limits(), &Cancel::new())
            .expect("the retargeted certificate is well formed");
    assert!(matches!(
        package_check(&truth, "Max", &proof),
        Ok(ExactProbabilityCertificateCheck::Accepted(_))
    ));
    assert_eq!(
        rejection_rule(package_check(&falsity, "Max", &forged)),
        ProbabilityCertificateRule::Malformed,
    );
}
