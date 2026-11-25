// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201's bounded checker adapter starts from an admitted checked model and
//! reaches it only through the claim binder.

use filament_exact::{Cancel, CancelCause, Integer, NodeKey, Rational};
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::TransitionSystem;
use fsl_package::ObligationIdentity;
use fsl_replay::{
    check_claim_exact_reachability, check_probability_certificate_package,
    ExactProbabilityCertificate, ExactProbabilityCertificateCheck,
    ExactProbabilityGraphError, ExactProbabilityValue, ProbabilityBoundSide,
    ProbabilityCertificateLocus, ProbabilityCertificateRejection, ProbabilityCertificateRule,
    ProbabilityProductKey, ByteProvision, CertificateRole, ProbabilityDocumentError,
    ProbabilityDocumentLimits, ProbabilityPackageRequest, ProbabilitySourceError,
    ProbabilitySourceLimits,
};
use fsl_semantics::check::{Bound, CheckedProbabilisticClaim, ProbForm, ReachTarget};
use ix_trace_rs::trace;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

use crate::vault_world::{with_body, Build, World, SECURE, UNLIMITED};

/// Reaches `holds(true)` at the initial state, so its graph is one state.
const LIKELY: &str = "probabilistic Same using inf on V under every scheduler on origin \
                      { probability >= 0.5 [ eventually holds(true) ] }\n";
/// A distinct claim, never reaching its target from the initial state.
const UNLIKELY: &str = "probabilistic Unlikely using inf on V under every scheduler on origin \
                        { probability >= 0.25 [ eventually holds(false) ] }\n";
/// A false claim: the target is reached with probability 1, not at most 1/2.
const FALSE_AT_MOST: &str = "probabilistic AtMost using inf on V under every scheduler on origin \
                             { probability <= 0.5 [ eventually holds(true) ] }\n";
/// A reach form this slice does not check.
const ALWAYS: &str = "probabilistic Always using inf on V under every scheduler on origin \
                      { probability >= 0.5 [ always holds(true) ] }\n";
/// A claim of another name, whose node no `Same` claim carries.
const OTHER: &str = "probabilistic Other using inf on V under every scheduler on origin \
                     { probability >= 0.5 [ eventually holds(true) ] }\n";

fn world_with(extra: &str) -> World {
    let build = Build { extra, ..SECURE };
    World::build(
        &["v"],
        false,
        &with_body("", "", "always holds(true)"),
        &build,
    )
}

fn claims(world: &World) -> &[CheckedProbabilisticClaim] {
    world.package().graph().probabilistic_model().claims()
}

fn by_form(world: &World, wanted: impl Fn(&ProbForm) -> bool) -> &CheckedProbabilisticClaim {
    claims(world)
        .iter()
        .find(|claim| wanted(&claim.form))
        .expect("the claim with that form")
}

fn threshold(
    world: &World,
    target: fn(&Bound) -> Option<&Rational>,
    numerator: i64,
    denominator: i64,
) -> &CheckedProbabilisticClaim {
    let wanted = Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap();
    by_form(world, |form| match form {
        ProbForm::Reach {
            bound,
            target: ReachTarget::Eventually(_),
        } => target(bound) == Some(&wanted),
        _ => false,
    })
}

fn at_least(bound: &Bound) -> Option<&Rational> {
    match bound {
        Bound::AtLeast(value) => Some(value),
        Bound::AtMost(_) => None,
    }
}

fn at_most(bound: &Bound) -> Option<&Rational> {
    match bound {
        Bound::AtMost(value) => Some(value),
        Bound::AtLeast(_) => None,
    }
}

fn node_of(claim: &CheckedProbabilisticClaim) -> NodeKey {
    claim
        .occurrence()
        .expect("a checked claim has a package identity")
        .node()
        .clone()
}

/// The model system, and its single initial state's FR-196 v1 product key.
fn system_and_root(world: &World) -> (ModelSystem<'_>, ProbabilityProductKey) {
    let mut subject = world.subject();
    subject.simulation.initial.truncate(1);
    let system = ModelSystem::new(subject.simulation.clone()).expect("model admits");
    let initial = system.initial().into_iter().next().expect("one initial");
    let model_key = system
        .state_key(&initial, fsl_foundation::IdentityLimits::default())
        .expect("model key")
        .0;
    let product = ProbabilityProductKey::for_model_state(None, &model_key)
        .expect("the owner form is embedded");
    (system, product)
}

fn one() -> Rational {
    Rational::from_integer(Integer::from(1_i64))
}

fn rooted(root: &ProbabilityProductKey, value: Rational) -> ExactProbabilityCertificate {
    ExactProbabilityCertificate {
        values: vec![ExactProbabilityValue {
            state: root.clone(),
            value,
        }],
    }
}

fn check(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    node: &NodeKey,
    certificate: &ExactProbabilityCertificate,
) -> Result<ExactProbabilityCertificateCheck, ExactProbabilityGraphError> {
    check_claim_exact_reachability(
        system,
        claims,
        &ObligationIdentity::new(node.clone(), None, 0),
        certificate,
        128,
        1024,
        &Cancel::new(),
    )
}

fn rejected(
    result: Result<ExactProbabilityCertificateCheck, ExactProbabilityGraphError>,
) -> ProbabilityCertificateRejection {
    match result {
        Ok(ExactProbabilityCertificateCheck::Rejected(rejection)) => rejection,
        other => panic!("expected a certificate rejection: {other:?}"),
    }
}

#[trace("TC-636", "FSpec-FR-331-AC-25")]
#[test]
fn bound_claim_reconstructs_its_graph_and_reports_the_values_member() {
    let world = world_with(UNLIKELY);
    let claim = threshold(&world, at_least, 1, 4);
    let (system, _) = system_and_root(&world);
    let empty = ExactProbabilityCertificate { values: Vec::new() };

    let rejection = rejected(check(&system, claims(&world), &node_of(claim), &empty));
    assert_eq!(rejection.rule, ProbabilityCertificateRule::Malformed);
    assert_eq!(rejection.at, ProbabilityCertificateLocus::Member("values"));
}

#[trace("TC-636")]
#[test]
fn claim_binding_uses_the_checked_node_not_name_or_source_order() {
    let forward = world_with(&format!("{LIKELY}{UNLIKELY}"));
    let reversed = world_with(&format!("{UNLIKELY}{LIKELY}"));

    // The claims keep their NodeKeys when their order is swapped.
    let likely = node_of(threshold(&forward, at_least, 1, 2));
    let unlikely = node_of(threshold(&forward, at_least, 1, 4));
    assert_ne!(likely, unlikely);
    assert_eq!(likely, node_of(threshold(&reversed, at_least, 1, 2)));
    assert_eq!(unlikely, node_of(threshold(&reversed, at_least, 1, 4)));

    for world in [&forward, &reversed] {
        let (system, root) = system_and_root(world);
        let certificate = rooted(&root, one());

        let Ok(ExactProbabilityCertificateCheck::Accepted(accepted)) =
            check(&system, claims(world), &likely, &certificate)
        else {
            panic!("the genuine checker must accept the matching certificate");
        };
        assert_eq!(accepted.value(), &one());
        assert_eq!(accepted.side(), ProbabilityBoundSide::AtLeast);
        // The other claim's target is never reached, so the same certificate
        // is rejected for it: its graph has states the certificate omits.
        let rejection = rejected(check(&system, claims(world), &unlikely, &certificate));
        assert_eq!(rejection.rule, ProbabilityCertificateRule::Malformed);
        assert_eq!(rejection.at, ProbabilityCertificateLocus::Member("values"));
    }
}

#[trace("TC-636", "FSpec-FR-331-AC-25")]
#[test]
fn typed_value_coverage_and_order_rejections_name_the_values_member() {
    let world = world_with(LIKELY);
    let (system, root) = system_and_root(&world);
    let node = node_of(threshold(&world, at_least, 1, 2));
    let valid = rooted(&root, one());
    assert!(matches!(
        check(&system, claims(&world), &node, &valid),
        Ok(ExactProbabilityCertificateCheck::Accepted(_))
    ));
    let foreign = ProbabilityProductKey::for_model_state(
        None,
        include_bytes!("../../../tools/fsl-probability-check/tests/fixtures/link.model-key-0.json"),
    )
    .expect("a valid key from a different model");
    assert_ne!(foreign, root);
    let mut duplicate = valid.clone();
    duplicate.values.push(duplicate.values[0].clone());
    let mut extra = valid.clone();
    extra.values.push(ExactProbabilityValue {
        state: foreign,
        value: one(),
    });
    extra.values.sort_by(|a, b| a.state.cmp(&b.state));
    let rejections: Vec<_> = [("duplicate", duplicate), ("extra", extra)]
        .into_iter()
        .map(|(name, certificate)| {
            (
                name,
                rejected(check(&system, claims(&world), &node, &certificate)),
            )
        })
        .collect();
    let expected = ProbabilityCertificateRejection {
        rule: ProbabilityCertificateRule::Malformed,
        at: ProbabilityCertificateLocus::Member("values"),
    };
    assert_eq!(
        rejections,
        vec![("duplicate", expected.clone()), ("extra", expected)]
    );
}

/// The claim fixes the scheduler objective: a false `under every scheduler`
/// `<=` claim is rejected, and the `>=` claim over the same graph is not.
#[trace("TC-636")]
#[test]
fn claim_fixes_the_objective_and_a_false_claim_is_rejected() {
    let world = world_with(&format!("{LIKELY}{FALSE_AT_MOST}"));
    let (system, root) = system_and_root(&world);
    let certificate = rooted(&root, one());
    let false_claim = node_of(threshold(&world, at_most, 1, 2));

    let rejection = rejected(check(&system, claims(&world), &false_claim, &certificate));
    assert_eq!(rejection.rule, ProbabilityCertificateRule::ThresholdMissed);
    assert_eq!(
        rejection.at,
        ProbabilityCertificateLocus::ProductState(root)
    );
    let Ok(ExactProbabilityCertificateCheck::Accepted(accepted)) = check(
        &system,
        claims(&world),
        &node_of(threshold(&world, at_least, 1, 2)),
        &certificate,
    ) else {
        panic!("the genuine checker must accept the holding bound");
    };
    assert_eq!(accepted.value(), &one());
    assert_eq!(accepted.side(), ProbabilityBoundSide::AtLeast);
}

#[trace("TC-3154", "FR-621-AC-23")]
#[test]
fn package_route_returns_malformed_for_a_deep_held_id_then_bad_index_on_512_kib_stack() {
    const DEEPEST_DEFAULT_CHAIN_NODES: usize = 10_000;

    use fsl_eval::simulation::model::ExpansionLimits;
    use fsl_replay::SpineLimits;
    use fsl_semantics::model::observation::ObservationLimits;
    use std::fmt::Write as _;

    let limits = ProbabilitySourceLimits {
        spine: SpineLimits::default(),
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits { max_candidates: 1 },
        scalar: UNLIMITED,
        document: ProbabilityDocumentLimits {
            bytes: u64::MAX,
            storage_bytes: u64::MAX,
            states: 1,
            edges: 1,
            scalar_bits: 64,
            accounting: UNLIMITED,
        },
    };
    let package_ceiling = limits
        .spine
        .model
        .intake
        .input_bytes
        .max(limits.observation.document_bytes);
    let mut package = String::from(
        r#"{"name":"probe","lock":{"sources":[{"authority":"probe","identity":"source"}]},"semantic_graph":{"nodes":[{"node_id":{"package":"p","path":[{"name":"root"}],"ordinal":0}}"#,
    );
    let mut depth = 1_usize;
    while depth < DEEPEST_DEFAULT_CHAIN_NODES {
        let mut row = String::new();
        write!(
            &mut row,
            r#",{{"node_id":{{"package":"p","path":[{{"node":{}}}],"ordinal":0}}}}"#,
            depth - 1
        )
        .expect("write an in-memory graph row");
        package.push_str(&row);
        depth += 1;
    }
    package.push_str("]}}");
    assert_eq!(depth, DEEPEST_DEFAULT_CHAIN_NODES);
    assert!(package.len() <= 1 << 20, "the 10,000-node chain fits i2's default 1 MiB byte limit");
    assert!(u64::try_from(package.len()).unwrap_or(u64::MAX) <= package_ceiling);

    let item = ObligationIdentity::new(
        NodeKey::new("p", vec![filament_exact::PathSegment::Text("claim".to_owned())], 0)
            .expect("claim identity"),
        None,
        0,
    );
    let certificate = format!(
        r#"{{"identity":{{"model":"model","node_id":{{"package":"p","path":[{{"name":"identity"}}],"ordinal":0}},"ordinal":0}},"values":[{{"state":{{"package":"p","path":[{{"node":{}}}],"ordinal":0}},"value":0}},{{"state":{{"package":"p","path":[{{"node":{depth}}}],"ordinal":0}},"value":0}}]}}"#,
        depth - 1
    )
    .into_bytes();
    let provision = ByteProvision::admit(
        vec![("probe".to_owned(), "source".to_owned(), Vec::new())],
        Vec::new(),
        std::iter::empty::<(String, &fsl_replay::DefinitionSourceRef)>(),
    )
    .expect("the package source has a provision entry");
    let thread_limits = limits;
    let result = std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(move || {
            check_probability_certificate_package(
                ProbabilityPackageRequest {
                    package: package.as_bytes(),
                    package_name: None,
                    provision: &provision,
                    item: &item,
                    initial: Vec::new(),
                    universes: Vec::new(),
                    certificate: &certificate,
                    role: CertificateRole::Claim,
                },
                thread_limits,
                &Cancel::new(),
            )
        })
        .expect("spawn the bounded-stack checker")
        .join()
        .expect("the certificate checker must not overflow the 512 KiB stack");
    assert!(matches!(
        result,
        Err(ProbabilitySourceError::Document(
            ProbabilityDocumentError::Malformed
        ))
    ), "unexpected checker result: {result:?}");
}

#[trace("TC-636")]
#[test]
fn binder_refuses_unbound_nodes_and_unsupported_forms() {
    let world = world_with(&format!("{LIKELY}{ALWAYS}"));
    let other = world_with(OTHER);
    let (system, root) = system_and_root(&world);
    let certificate = rooted(&root, one());

    // A node no claim of the full list carries binds nothing.
    let foreign = node_of(&claims(&other)[0]);
    assert!(claims(&world).iter().all(|claim| node_of(claim) != foreign));
    assert_eq!(
        check(&system, claims(&world), &foreign, &certificate),
        Err(ExactProbabilityGraphError::ClaimNotBound)
    );

    // A bound claim of a form this slice does not check is refused.
    let always = node_of(by_form(&world, |form| {
        matches!(
            form,
            ProbForm::Reach {
                target: ReachTarget::Always(_),
                ..
            }
        )
    }));
    assert_eq!(
        check(&system, claims(&world), &always, &certificate),
        Err(ExactProbabilityGraphError::UnsupportedClaimForm)
    );
}

#[trace("TC-636", "FR-201", "FR-282")]
#[test]
fn cancelled_checker_preserves_cause_before_and_during_graph_reconstruction() {
    let world = world_with(UNLIKELY);
    let (system, _) = system_and_root(&world);
    let certificate = ExactProbabilityCertificate { values: Vec::new() };
    let item = ObligationIdentity::new(node_of(threshold(&world, at_least, 1, 4)), None, 0);
    let run = |cancel: &Cancel| {
        check_claim_exact_reachability(
            &system,
            claims(&world),
            &item,
            &certificate,
            128,
            1024,
            cancel,
        )
    };
    let live = Cancel::new();
    let _counting = live.count_charges();
    assert!(
        matches!(
            run(&live),
            Ok(ExactProbabilityCertificateCheck::Rejected(_))
        ),
        "the live checker must finish graph reconstruction before rejecting the empty certificate"
    );
    let total = live.charges();
    assert!(total > 0, "graph reconstruction must poll cancellation");

    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let cancelled = Cancel::new();
        cancelled.cancel(cause);
        assert_eq!(
            run(&cancelled),
            Err(ExactProbabilityGraphError::Cancelled(cause))
        );
        for at in 1..=total {
            let slot = Arc::new(OnceLock::<Cancel>::new());
            let weak = Arc::downgrade(&slot);
            let seen = Arc::new(AtomicU32::new(0));
            let count = Arc::clone(&seen);
            let cancel = Cancel::observing(move || {
                if count.fetch_add(1, Ordering::Relaxed) + 1 == at {
                    weak.upgrade()
                        .expect("the run owns the handle slot")
                        .get()
                        .expect("the slot is initialized")
                        .cancel(cause);
                }
            });
            slot.set(cancel.clone())
                .expect("the slot is initialized once");
            assert_eq!(
                run(&cancel),
                Err(ExactProbabilityGraphError::Cancelled(cause)),
                "cancellation at observed checker poll {at}/{total} must stop checking"
            );
            assert_eq!(seen.load(Ordering::Relaxed), at);
        }
    }
}
