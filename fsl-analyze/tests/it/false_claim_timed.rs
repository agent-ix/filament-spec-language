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

fn two_clock_world(bound: &str) -> World {
    let body = format!(
        "{}pre First using v on M::Call::first {{ self.x > 0 ms and self.x < 1 ms }}\n\
         post FirstDone using v on M::Call::first {{ self.x = 0 ms }}\n\
         pre Third using v on M::Call::third {{ self.x < 1 ms and self.y > 1 ms }}\n\
         post ThirdDone using v on M::Call::third {{ self.late = true }}\n\
         time invariant Cap when {{ true }} {{ self.y <= 3 ms }}\n\
         probabilistic Max using ev on M over (c: M::Call) under every scheduler on origin \
         {{ probability {bound} [ eventually[0 ms, 4 ms] holds(c.late) ] }}\n",
        timed_world::DENSE,
    );
    timed_world::world(
        &timed_world::document(
            &["x", "y"],
            &[("first", &["x"]), ("third", &["late"])],
        ),
        PROFILES,
        &body,
    )
}

#[test]
fn explore_3444() {
    let truth = two_clock_world("<= 1");
    let record = settle(&truth, "Max");
    eprintln!("TRUE-CLAIM settle: {:?}", record.terminal().map(|t| t.value().clone()));
    let bytes = record
        .terminal()
        .and_then(|terminal| terminal.exact_certificate_document())
        .expect("a proof")
        .to_vec();
    eprintln!("TRUE-DOC {}", String::from_utf8_lossy(&bytes));
    let falsity = two_clock_world("<= 0.999");
    let record = settle(&falsity, "Max");
    eprintln!("FALSE-CLAIM settle: {:?}", record.terminal().map(|t| t.value().clone()));
    eprintln!("CHECK-TRUE {:?}", package_check(&truth, "Max", &bytes));
    eprintln!("CHECK-FALSE-WITH-TRUE-DOC {:?}", package_check(&falsity, "Max", &bytes));
}
