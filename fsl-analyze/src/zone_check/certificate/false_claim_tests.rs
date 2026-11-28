// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 seed QSL-3478 (R-27 `always possible`, R-28 `unique path`): over
//! two clocks in dense time, `first` at 1/2 ms leaves a state from which no
//! run reaches phase 2, so both claims are false. The definitional
//! reference decides that by its own region quotient, the analysis does not
//! settle either claim `proved`, and a certificate that says the trapped
//! state reaches phase 2 is not accepted by the check entry.
//!
//! The model is data-free in its clock clauses (the reference reads no data
//! predicate): `first` runs while `y < 1 ms` and resets `x`; `zero` needs
//! `y >= 2 ms` and `x >= 2 ms`; `one` needs `y >= 2 ms` and `x <= 1 ms`;
//! `y <= 2 ms` always. After `first` at `t`, `x = 2 - t` when `y = 2`, so for
//! `t` in (0, 1) neither `zero` (needs `t = 0`) nor `one` (needs `t >= 1`)
//! is enabled, and a later `first` at `t'` in [`t`, 1) leaves the same trap.

use filament_exact::{Cancel, Integer, Rational, ValueType};
use fsl_eval::simulation::model::{TimedSystem, DEFAULT_MAX_CLOCKS};
use fsl_replay::{
    check_state_graph_certificate, CertificateCheckLimits, CertifiedItem,
    StateGraphCertificateDecision, StateGraphCertificateForm, ZoneCertificateRequest,
    ZoneCertificateVerdict,
};
use fsl_route::deadlock::{ItemObligation, ObligationKind};
use fsl_semantics::check::StateGraphForm;

const TIMED: &str = "profile tt = \"filament.temporal.timed/v1\";\n";
const CLAIMS: &str = "always possible Done using v over (c: M::Call) { c.phase >= 2 }\n\
    unique path OnlyWay using v over (c: M::Call) from (c.phase = 1) to (c.phase >= 2);\n";

fn rational(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator))
        .expect("nonzero rational denominator")
}

fn world() -> crate::timed_world::World {
    let body = format!(
        "{}\
         pre First using v on M::Call::first {{ self.y < 1 ms }}\n\
         post FirstDone using v on M::Call::first {{ self.phase = 1 and self.x = 0 ms }}\n\
         pre Zero using v on M::Call::zero {{ self.y >= 2 ms and self.x >= 2 ms }}\n\
         post ZeroDone using v on M::Call::zero {{ self.phase = 2 }}\n\
         pre One using v on M::Call::one {{ self.y >= 2 ms and self.x <= 1 ms }}\n\
         post OneDone using v on M::Call::one {{ self.phase = 3 }}\n\
         time invariant Cap when {{ true }} {{ self.y <= 2 ms }}\n\
         {CLAIMS}",
        crate::timed_world::DENSE
    );
    let document = crate::timed_world::document(
        &["x", "y"],
        &[
            ("first", &["phase", "x"]),
            ("zero", &["phase"]),
            ("one", &["phase"]),
        ],
    );
    crate::timed_world::world(&document, TIMED, &body)
}

/// Seed QSL-3478, reference half: from both clocks at zero
/// the reference's region quotient reaches an enabled `zero`, and after
/// `first` at 1/2 ms (clocks `x = 0`, `y = 1/2`) it reaches none.
#[test]
fn seed_qsl_3478_the_reference_region_quotient_finds_the_trap() {
    let world = world();
    let timed = &world.package.graph().timed_models()[0];
    let start = fsl_definitional_reference::ReferenceClockState::new(
        timed,
        vec![rational(0, 1), rational(0, 1)],
    )
    .expect("both clocks start at zero");
    let operations = ["first", "zero", "one"];
    let targets = ["zero", "one"];
    assert!(
        start
            .can_enable_eventually(timed, &operations, &targets, 2)
            .expect("the reference decides the initial state"),
        "waiting to y = 2 with x = 2 enables zero"
    );
    let trapped = start
        .fire(timed, &rational(1, 2), "first")
        .expect("the reference fires first")
        .expect("first is enabled at 1/2 ms");
    assert_eq!(trapped.values(), &[rational(0, 1), rational(1, 2)]);
    assert!(
        !trapped
            .can_enable_eventually(timed, &operations, &targets, 2)
            .expect("the reference decides the trapped state"),
        "from x = 0, y = 1/2 no delay or later first enables zero or one"
    );
}

/// The analysis decision for `claim_name`, the forged certificate's check and
/// the honest certificate's check, for `claim_name`'s form.
struct Settled {
    refuted: bool,
    forged: (
        ZoneCertificateVerdict,
        Option<StateGraphCertificateDecision>,
    ),
    honest: (
        ZoneCertificateVerdict,
        Option<StateGraphCertificateDecision>,
    ),
}

fn settle(claim_name: &str) -> Settled {
    let world = world();
    let system = world.system();
    let timed = TimedSystem::with_max_clocks(&system, "M", DEFAULT_MAX_CLOCKS)
        .expect("the fixture admits the timed model");
    let claim = world
        .package
        .graph()
        .state_graph_claim(claim_name)
        .expect("the named authored state-graph claim");
    let occurrence = claim
        .occurrence()
        .expect("authored state-graph occurrence")
        .clone();
    let parameter = claim.over().expect("the claim binds its call object");
    let instance = system
        .instances(&ValueType::Reference(parameter.object.clone()))
        .expect("the fixture's call instance")
        .into_iter()
        .next()
        .expect("one call instance");
    let over = system
        .written_reference(parameter.object.clone(), &instance)
        .expect("the written call reference");
    let graph = crate::zone_check::explore_timed_state_graph(
        &timed,
        crate::zone_check::ZoneCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the timed state graph is explored");
    let decision = crate::zone_check::decide_timed_state_graph(
        &timed,
        &graph,
        claim,
        Some(&over),
        crate::zone_check::ZoneCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the timed state graph claim is decided");
    let (refuted, honest_nodes) = match decision {
        crate::zone_check::TimedStateGraphDecision::Refuted { ef_nodes, .. } => (true, ef_nodes),
        _ => (false, Vec::new()),
    };
    let form = match claim.form() {
        StateGraphForm::AlwaysPossible { .. } => StateGraphCertificateForm::AlwaysPossible,
        StateGraphForm::UniquePath { .. } => StateGraphCertificateForm::UniquePath,
        StateGraphForm::Possible { .. } => StateGraphCertificateForm::Possible,
    };
    let obligation = ItemObligation {
        subject: (),
        kind: ObligationKind::StateGraphAuthored(occurrence),
        reductions: Default::default(),
    };
    let check = |ef_nodes: &[usize]| {
        let certificate = crate::zone_check::state_graph_certificate(
            &timed,
            &graph,
            obligation.clone(),
            form,
            ef_nodes,
        )
        .expect("the typed zone certificate is built");
        check_state_graph_certificate(
            ZoneCertificateRequest {
                simulation: world.request(),
                model: "M",
                max_clocks: DEFAULT_MAX_CLOCKS,
                item: CertifiedItem::StateGraph {
                    claim,
                    over: Some(&over),
                },
            },
            &obligation,
            crate::zone_check::certificate_item,
            &certificate,
            CertificateCheckLimits::default(),
            &Cancel::new(),
        )
        .unwrap_or_else(|_| panic!("the check entry decides the certificate"))
    };
    // The forgery: every reachable node is claimed to reach phase 2.
    let every_node: Vec<usize> = (0..graph.nodes.len()).collect();
    Settled {
        refuted,
        forged: check(&every_node),
        honest: check(&honest_nodes),
    }
}

/// Seed QSL-3478, R-27 checker half: the analysis refutes
/// `always possible Done`, a certificate whose `ef_nodes` name every node
/// (the trapped one included) is rejected by the check entry, and the honest
/// certificate decides the claim violated, never holds.
#[test]
fn seed_qsl_3478_always_possible_forgery_is_rejected() {
    let settled = settle("Done");
    assert!(settled.refuted, "the analysis refutes the claim");
    let (verdict, _) = &settled.forged;
    let ZoneCertificateVerdict::Rejected(rejection) = verdict else {
        panic!("the forged certificate was not rejected: {verdict:?}");
    };
    assert_eq!(
        rejection.rule,
        fsl_replay::CertificateRule::RankBroken,
        "{rejection:?}"
    );
    assert_eq!(
        settled.honest.1,
        Some(StateGraphCertificateDecision::Violated),
        "{:?}",
        settled.honest
    );
}

/// Seed QSL-3478, R-28 checker half: the analysis refutes
/// `unique path OnlyWay`, and no certificate makes the check entry decide it
/// holds: it decides violated.
#[test]
fn seed_qsl_3478_unique_path_is_decided_violated() {
    let settled = settle("OnlyWay");
    assert!(settled.refuted, "the analysis refutes the claim");
    assert_eq!(
        settled.forged.1,
        Some(StateGraphCertificateDecision::Violated),
        "{:?}",
        settled.forged
    );
}
