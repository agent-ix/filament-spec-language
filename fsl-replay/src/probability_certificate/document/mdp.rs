// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201's exact cyclic MDP values, independently checked against all actions.

use super::*;
use ix_trace_rs::implements;

/// Unchecked exact document with its closed wire-profile selection retained.
/// Existing document literals retain their original meaning and encoding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbabilityProofDocument {
    document: ProbabilityCertificateDocument,
    cyclic_mdp: bool,
    fair_mdp: bool,
    fairness: Vec<super::fairness::Record>,
}

impl ProbabilityProofDocument {
    /// A document of a base with no fairness and no every-scheduler cyclic
    /// route.
    pub(super) fn unfair(document: ProbabilityCertificateDocument) -> Self {
        Self {
            document,
            cyclic_mdp: false,
            fair_mdp: false,
            fairness: Vec::new(),
        }
    }

    /// Borrow the untrusted identity, values, ranks and objective data.
    pub fn document(&self) -> &ProbabilityCertificateDocument {
        &self.document
    }

    /// Mutate unchecked identity, values, ranks and objective data for tests.
    /// The retained profile and fairness discriminator are unchanged. Writing
    /// still validates the wire shape; semantic acceptance requires checking.
    #[cfg(feature = "test-support")]
    pub fn document_mut(&mut self) -> &mut ProbabilityCertificateDocument {
        &mut self.document
    }

    /// Whether this document selects a cyclic MDP proof profile.
    pub fn is_cyclic_mdp(&self) -> bool {
        self.cyclic_mdp
    }

    /// Whether this document selects finite cyclic MDP expected reward.
    pub fn is_cyclic_mdp_reward(&self) -> bool {
        self.cyclic_mdp && self.document.reward.is_some()
    }

    /// Whether this unchecked document selects the bounded fair profile.
    pub fn is_bounded_fair(&self) -> bool {
        !self.fairness.is_empty() && !self.fair_mdp
    }

    /// Whether this document selects the unbounded fair reachability profile.
    pub fn is_fair_mdp(&self) -> bool {
        self.fair_mdp
    }
}

/// Read a strict exact profile without discarding its wire discriminator.
pub fn read_probability_proof_document(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ProbabilityProofDocument, ProbabilityDocumentError> {
    let cyclic_mdp = matches!(
        profile_dispatch(bytes, limits, cancel)?,
        ProfileDispatch::CyclicMdp | ProfileDispatch::CyclicMdpReward
    );
    let fair_mdp = matches!(
        profile_dispatch(bytes, limits, cancel)?,
        ProfileDispatch::FairMdp
    );
    let mut meter = limits.accounting_meter(cancel);
    let mut fairness = Vec::new();
    let document = read_document_profile(
        bytes,
        limits,
        cancel,
        &mut meter,
        cyclic_mdp,
        Some(&mut fairness),
        &mut 0,
    )?;
    Ok(ProbabilityProofDocument {
        document,
        cyclic_mdp,
        fair_mdp,
        fairness,
    })
}

/// Write the shared canonical schema with the retained exact profile.
/// This produces unchecked evidence, never an acceptance token.
pub fn write_probability_proof_document(
    document: &ProbabilityProofDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    if !document.fairness.is_empty() {
        super::fairness::admit_encoding(&document.fairness, limits, cancel)?;
    }
    let bytes = writer::encode_fair_profile(
        &document.document,
        limits,
        cancel,
        document.cyclic_mdp,
        document.fair_mdp,
        &document.fairness,
    )?;
    read_probability_proof_document(&bytes, limits, cancel)?;
    Ok(bytes)
}

/// FR-1042: write an untimed proof document whose values are one end of an
/// interval as the proof-role document of `side`.
pub fn write_value_bounds_proof_document(
    document: &ProbabilityProofDocument,
    side: SubsystemBoundKind,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let exact = write_probability_proof_document(document, limits, cancel)?;
    value_bounds::wrap(&exact, side, limits)
}

/// Prefer existing eligible profiles before constructing cyclic MDP evidence.
/// All admission attempts share the same arithmetic and retained-storage scope.
pub fn probability_proof_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ProbabilityProofDocument, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel, None);
    proof_document_in(system, claims, item, certificate, &mut cx)
}

/// [`probability_proof_document`], with the arithmetic and retained-storage
/// scope its attempts spent, for a further attempt to continue.
pub(super) fn probability_proof_document_spending(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> (
    Result<ProbabilityProofDocument, ProbabilityDocumentError>,
    filament_exact::Meter,
    u64,
) {
    let mut cx = Context::new(limits, cancel, None);
    let proof = proof_document_in(system, claims, item, certificate, &mut cx);
    (proof, cx.meter, cx.storage)
}

fn proof_document_in(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    cx: &mut Context<'_, '_>,
) -> Result<ProbabilityProofDocument, ProbabilityDocumentError> {
    let (limits, cancel) = (cx.limits, cx.cancel);
    let claim = bind_claim(claims, item, cancel)?;
    if !claim.fairness.is_empty() {
        super::fairness::admit(claim)?;
        let fair_mdp = matches!(
            &claim.form,
            ProbForm::Reach {
                target: ReachTarget::Eventually(_),
                ..
            }
        );
        let fairness = super::fairness::checked(
            system,
            claim,
            limits,
            cancel,
            &mut cx.meter,
            &mut cx.storage,
        )?;
        if fair_mdp {
            cx.fair_mdp = true;
            let mut proof = construct(system, claims, item, certificate, cx)?;
            proof.document.cyclic_dtmc = false;
            proof.cyclic_mdp = false;
            proof.fair_mdp = true;
            proof.fairness = fairness;
            return Ok(proof);
        }
        let document = construct_document(
            system,
            claims,
            item,
            certificate,
            limits,
            cancel,
            &mut cx.meter,
            &mut cx.storage,
            true,
        )?;
        return Ok(ProbabilityProofDocument {
            document,
            cyclic_mdp: false,
            fair_mdp,
            fairness,
        });
    }
    match construct_document(
        system,
        claims,
        item,
        certificate,
        limits,
        cancel,
        &mut cx.meter,
        &mut cx.storage,
        false,
    ) {
        Ok(document) => Ok(ProbabilityProofDocument {
            document,
            cyclic_mdp: false,
            fair_mdp: false,
            fairness: Vec::new(),
        }),
        Err(error)
            if may_retry(&error)
                || (error == ProbabilityDocumentError::ObjectiveMismatch
                    && super::super::cyclic_state::is_claim(claim)) =>
        {
            construct(system, claims, item, certificate, cx)
        }
        Err(error) => Err(error),
    }
}

fn may_retry(error: &ProbabilityDocumentError) -> bool {
    matches!(
        error,
        ProbabilityDocumentError::Unsupported
            | ProbabilityDocumentError::UndefinedPredicate(_)
            | ProbabilityDocumentError::Graph(ExactProbabilityGraphError::Cycle)
    ) || matches!(error, ProbabilityDocumentError::Build(issue)
            if matches!(issue.as_ref(), crate::ExactProbBuildIssue::Predicate(_)))
}

/// Construct this profile explicitly, including for a one-action DTMC subtype.
/// The independent checker must still verify every supplied value and rank.
pub fn cyclic_mdp_probability_certificate_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ProbabilityProofDocument, ProbabilityDocumentError> {
    if !matches!(
        bind_claim(claims, item, cancel)?.form,
        ProbForm::Reach {
            target: ReachTarget::Eventually(_) | ReachTarget::Until(..) | ReachTarget::Always(_),
            ..
        }
    ) {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    construct(
        system,
        claims,
        item,
        certificate,
        &mut Context::new(limits, cancel, None),
    )
}

/// Construct finite cyclic MDP expected-reward evidence explicitly.
/// Every stopped state must have a finite selected value; supplied values remain unchecked evidence.
pub fn cyclic_mdp_reward_probability_certificate_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ProbabilityProofDocument, ProbabilityDocumentError> {
    if !matches!(
        bind_claim(claims, item, cancel)?.form,
        ProbForm::ExpectedReward { .. }
    ) {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    construct(
        system,
        claims,
        item,
        certificate,
        &mut Context::new(limits, cancel, None),
    )
}

/// Actual stages of the cyclic MDP checker, for default-off test observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CyclicMdpProbabilityPhase {
    /// Complete original model admission without evaluating the query predicate.
    OriginalAdmission,
    /// Initial reachable graph stopped at the first true predicate.
    StoppedGraph,
    /// Retaining offered value and progress rows.
    Rows,
    /// Exact sums over separate, source-reconstructed action distributions.
    ActionSums,
    /// Checking the positive-support closure of zero states.
    ZeroSupport,
    /// Checking every-action or tight-action progress.
    Progress,
    /// Deriving an unchecked producer progress witness.
    Ranks,
    /// Classification graph of an until or always claim: decided states absorb.
    Classification,
    /// Maximal end components of the classification graph.
    Components,
    /// Certificate graph with the violating union not expanded.
    CertificateGraph,
}

struct Context<'a, 'control> {
    limits: ProbabilityDocumentLimits,
    cancel: &'a Cancel,
    fair_mdp: bool,
    bound_side: Option<SubsystemBoundKind>,
    meter: filament_exact::Meter,
    storage: u64,
    reward: bool,
    reward_control: Option<&'a super::super::reward::RewardControl<'control>>,
    work: Option<&'a filament_exact::WorkScope>,
    identity: fsl_foundation::IdentityLimits,
    observe: Option<&'a mut dyn FnMut(CyclicMdpProbabilityPhase)>,
}

struct FairProgress<'a> {
    zero: Option<&'a BTreeSet<ProbabilityProductKey>>,
    selected_policy: Option<&'a BTreeMap<ProbabilityProductKey, usize>>,
}

pub(super) struct ValueBoundControl<'a, 'control> {
    pub(super) limits: ProbabilityDocumentLimits,
    pub(super) cancel: &'a Cancel,
    pub(super) reward: &'a super::super::reward::RewardControl<'control>,
    pub(super) work: Option<&'a filament_exact::WorkScope>,
}

/// Independently reconstruct the unbounded fair eventual MDP and verify that
/// the offered deterministic scheduler has no unfair bottom component.
pub(super) fn fair_scheduler_is_fair(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    rows: &[(ProbabilityProductKey, crate::ProductAction)],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<bool, ProbabilityDocumentError> {
    let claim = bind_claim(claims, item, cancel)?;
    if claim.fairness.is_empty()
        || !matches!(
            &claim.form,
            ProbForm::Reach {
                target: ReachTarget::Eventually(_),
                ..
            } | ProbForm::Probability {
                event: fsl_semantics::check::Event::Formula { .. },
                ..
            }
        )
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let mut cx = Context::new(limits, cancel, None);
    cx.fair_mdp = true;
    let (graph, _, _, _, _) = subject(system, claims, item, &mut cx, None)?;
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|binding| &binding.reference);
    Ok(matches!(
        super::long_run::scheduler_is_fair(
            system,
            graph,
            reference,
            rows,
            &claim.fairness,
            limits,
            cancel,
        )?,
        Some(true)
    ))
}
impl<'a, 'control> Context<'a, 'control> {
    fn new(
        limits: ProbabilityDocumentLimits,
        cancel: &'a Cancel,
        observe: Option<&'a mut dyn FnMut(CyclicMdpProbabilityPhase)>,
    ) -> Self {
        Self {
            limits,
            cancel,
            fair_mdp: false,
            bound_side: None,
            meter: limits.accounting_meter(cancel),
            storage: 0,
            reward: false,
            reward_control: None,
            work: None,
            identity: fsl_foundation::IdentityLimits::default(),
            observe,
        }
    }
    fn tick(&mut self, phase: CyclicMdpProbabilityPhase) -> Result<(), ProbabilityDocumentError> {
        if let Some(observe) = self.observe.as_mut() {
            observe(phase);
        }
        if self.reward {
            if let Some(control) = self.reward_control {
                use super::super::reward::RewardProbabilityPhase as R;
                control.phase(match phase {
                    CyclicMdpProbabilityPhase::OriginalAdmission => R::Original,
                    CyclicMdpProbabilityPhase::StoppedGraph => R::Stopped,
                    CyclicMdpProbabilityPhase::Progress => R::Progress,
                    CyclicMdpProbabilityPhase::Ranks => R::Ranks,
                    _ => R::Arithmetic,
                })?;
            }
        }
        super::super::reward::visit(self.cancel, &mut self.meter)
    }
    fn reserve(&mut self, bytes: usize) -> Result<(), ProbabilityDocumentError> {
        let mut admission = ProbabilityKeyAdmission::new(
            &mut self.meter,
            &mut self.storage,
            self.limits.storage_bytes,
            self.cancel,
        );
        filament_exact::Reservation::reserve(&mut admission, bytes)
            .map_err(|_| admission.refusal(fsl_eval::simulation::model::ModelKeyError::Denied))
    }
}

#[ix_trace_rs::string_edge]
pub(super) fn member_shapes(
    document: &quire_canonical::Document,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<(), ProbabilityDocumentError> {
    let root = document.root();
    for member in ["values", "ranking"] {
        let Some(quire_canonical::Node::Array(rows)) = root.get(member).map(|node| node.node())
        else {
            return Err(ProbabilityDocumentError::MalformedMember(member));
        };
        for row in rows {
            super::super::reward::visit(cancel, meter)?;
            let shape = matches!(row.node(), quire_canonical::Node::Object(fields) if fields.clone().count() == 2)
                && row.get("state").is_some();
            let payload = if member == "values" {
                row.get("value").is_some_and(|value| {
                    matches!(value.node(), quire_canonical::Node::Object(fields) if fields.clone().count() == 2)
                        && value.get("numerator").is_some() && value.get("denominator").is_some()
                })
            } else {
                row.get("rank").is_some()
            };
            if !shape || !payload {
                return Err(ProbabilityDocumentError::MalformedMember(member));
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn admit_reward(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    identity: fsl_foundation::IdentityLimits,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<(), ProbabilityDocumentError> {
    reward_subject(
        system,
        claims,
        item,
        limits,
        cancel,
        identity,
        meter,
        storage,
        &super::super::reward::RewardControl::new(cancel),
    )
    .map(|_| ())
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn reward_subject(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    identity: fsl_foundation::IdentityLimits,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    control: &super::super::reward::RewardControl<'_>,
) -> Result<
    (
        CheckerGraph,
        ProbabilityObjective,
        Bound,
        Option<super::super::reward::RewardGraph>,
    ),
    ProbabilityDocumentError,
> {
    use fsl_eval::simulation::model::{TimedSubjectRefusal, TimedSystem};
    if !std::ptr::eq(system.input().probabilistic.claims(), claims) {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let claim = bind_claim(claims, item, cancel)?;
    let ProbForm::ExpectedReward {
        bound,
        reward,
        until,
    } = &claim.form
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    if !matches!(claim.scheduler, ProbScheduler::Every)
        || !claim.fairness.is_empty()
        || fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
            != Some(fsl_semantics::check::TemporalProfile::InfiniteTrace)
        || !matches!(
            TimedSystem::new(system, &claim.subject.alias),
            Err(TimedSubjectRefusal::NoTimedModel { .. })
        )
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|bound| &bound.reference);
    let (graph, _, reward) = super::super::reward::enumerate(
        system,
        claim,
        reward,
        until,
        reference,
        None,
        limits,
        cancel,
        identity,
        super::super::reward::RewardProfile::CyclicMdp,
        None,
        meter,
        storage,
        control,
    )?;
    Ok((
        graph,
        claim_objective(&claim.scheduler, bound),
        bound.clone(),
        Some(reward),
    ))
}

#[allow(clippy::type_complexity)]
fn subject(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    cx: &mut Context<'_, '_>,
    capture: Option<&super::super::cyclic::CapturedStates>,
) -> Result<
    (
        CheckerGraph,
        ProbabilityObjective,
        Bound,
        Option<super::super::reward::RewardGraph>,
        Option<ProbabilityStateEvent>,
    ),
    ProbabilityDocumentError,
> {
    use fsl_eval::simulation::model::{TimedSubjectRefusal, TimedSystem};
    if !std::ptr::eq(system.input().probabilistic.claims(), claims) {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let claim = bind_claim(claims, item, cx.cancel)?;
    if cx.reward {
        cx.tick(CyclicMdpProbabilityPhase::OriginalAdmission)?;
        let found = if let Some(control) = cx.reward_control {
            reward_subject(
                system,
                claims,
                item,
                cx.limits,
                cx.cancel,
                cx.identity,
                &mut cx.meter,
                &mut cx.storage,
                control,
            )
        } else {
            reward_subject(
                system,
                claims,
                item,
                cx.limits,
                cx.cancel,
                cx.identity,
                &mut cx.meter,
                &mut cx.storage,
                &super::super::reward::RewardControl::new(cx.cancel),
            )
        };
        return found
            .map(|(graph, objective, bound, reward)| (graph, objective, bound, reward, None));
    }
    if super::super::cyclic_state::is_claim(claim) {
        let binding = claim_binding(system, claim)?;
        let reference = binding.as_ref().map(|bound| &bound.reference);
        let workload = claim_workload(system, claim)?;
        let observe = std::cell::RefCell::new(&mut cx.observe);
        let found = super::super::cyclic_state::certificate_graph(
            system,
            claim,
            super::super::cyclic_state::Profile::Mdp,
            reference,
            workload,
            cx.limits,
            cx.cancel,
            cx.identity,
            &mut cx.meter,
            &mut cx.storage,
            &mut |stage| {
                if let Some(observe) = observe.borrow_mut().as_mut() {
                    observe(match stage {
                        super::super::cyclic_state::Stage::Original => {
                            CyclicMdpProbabilityPhase::OriginalAdmission
                        }
                        super::super::cyclic_state::Stage::Classification => {
                            CyclicMdpProbabilityPhase::Classification
                        }
                        super::super::cyclic_state::Stage::Components => {
                            CyclicMdpProbabilityPhase::Components
                        }
                        super::super::cyclic_state::Stage::Certificate => {
                            CyclicMdpProbabilityPhase::CertificateGraph
                        }
                    });
                }
                Ok(())
            },
        )?;
        let violated = super::super::cyclic_state::violated_bound(
            &found.bound,
            cx.limits,
            cx.cancel,
            &mut cx.meter,
        )?;
        return Ok((
            found.graph,
            found.objective,
            violated,
            None,
            Some(found.event),
        ));
    }
    let ProbForm::Reach {
        bound,
        target: ReachTarget::Eventually(predicate),
    } = &claim.form
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    if !matches!(claim.scheduler, ProbScheduler::Every)
        || !claim.fairness.is_empty() && !cx.fair_mdp
        || fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
            != Some(fsl_semantics::check::TemporalProfile::InfiniteTrace)
        || !matches!(
            TimedSystem::new(system, &claim.subject.alias),
            Err(TimedSubjectRefusal::NoTimedModel { .. })
        )
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let (Bound::AtLeast(threshold) | Bound::AtMost(threshold)) = bound;
    if *threshold <= rational(0) || *threshold >= rational(1) {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|bound| &bound.reference);
    cx.tick(CyclicMdpProbabilityPhase::OriginalAdmission)?;
    let original = super::super::cyclic::enumerate_graph(
        system,
        None::<&fsl_semantics::check::CheckedPredicate>,
        reference,
        None,
        cx.limits,
        cx.cancel,
        cx.identity,
        &mut cx.meter,
        &mut cx.storage,
        false,
        false,
    )?;
    for state in original.states.values() {
        for choice in &state.choices {
            let mut mass = rational(0);
            for edge in choice {
                cx.tick(CyclicMdpProbabilityPhase::OriginalAdmission)?;
                if edge.probability < rational(0) {
                    return Err(ProbabilityDocumentError::Malformed);
                }
                check_scalar(&edge.probability, cx.limits)?;
                mass = arithmetic(
                    filament_exact::RationalArithmetic::Add(&mass, &edge.probability),
                    &mut cx.meter,
                    cx.cancel,
                )?;
                check_scalar(&mass, cx.limits)?;
            }
            if mass != rational(1) {
                return Err(ProbabilityDocumentError::Malformed);
            }
        }
    }
    drop(original);
    cx.tick(CyclicMdpProbabilityPhase::StoppedGraph)?;
    let graph = super::super::cyclic::enumerate_graph_capturing(
        system,
        Some(predicate),
        reference,
        None,
        cx.limits,
        cx.cancel,
        cx.identity,
        &mut cx.meter,
        &mut cx.storage,
        true,
        true,
        capture,
    )?;
    Ok((
        graph,
        claim_objective(&claim.scheduler, bound),
        bound.clone(),
        None,
        None,
    ))
}

type Values = BTreeMap<ProbabilityProductKey, Rational>;
type Sums = BTreeMap<ProbabilityProductKey, Vec<Rational>>;

fn rows(
    graph: &CheckerGraph,
    certificate: &ExactProbabilityCertificate,
    cx: &mut Context<'_, '_>,
    allow_partial: bool,
) -> Result<CertificateValues, ProbabilityDocumentError> {
    for row in &certificate.values {
        cx.tick(CyclicMdpProbabilityPhase::Rows)?;
        cx.reserve(
            std::mem::size_of::<(ProbabilityProductKey, Rational)>()
                .saturating_add(row.state.as_bytes().len())
                .saturating_add(
                    usize::try_from(
                        row.value
                            .numerator()
                            .magnitude_bits()
                            .saturating_add(row.value.denominator().magnitude_bits())
                            .div_ceil(8),
                    )
                    .unwrap_or(usize::MAX),
                ),
        )?;
        check_scalar(&row.value, cx.limits)?;
        if row.value < rational(0) || !cx.reward && row.value > rational(1) {
            return Err(ProbabilityDocumentError::Malformed);
        }
    }
    Ok(certificate_values(
        graph,
        certificate,
        cx.cancel,
        allow_partial,
    )?)
}

fn sums(
    graph: &CheckerGraph,
    values: &Values,
    reward: Option<&super::super::reward::RewardGraph>,
    cx: &mut Context<'_, '_>,
) -> Result<Sums, ProbabilityDocumentError> {
    let mut sums = BTreeMap::new();
    for (key, state) in &graph.states {
        cx.tick(CyclicMdpProbabilityPhase::ActionSums)?;
        cx.reserve(
            std::mem::size_of::<(ProbabilityProductKey, Vec<Rational>)>()
                .saturating_add(key.as_bytes().len())
                .saturating_add(
                    state
                        .choices
                        .len()
                        .saturating_mul(std::mem::size_of::<Rational>()),
                ),
        )?;
        let mut actions = Vec::new();
        actions
            .try_reserve_exact(state.choices.len())
            .map_err(|_| {
                ProbabilityDocumentError::KeyEncoding(
                    fsl_eval::simulation::EncodingRefusal::Allocation {
                        requested: state
                            .choices
                            .len()
                            .saturating_mul(std::mem::size_of::<Rational>()),
                    },
                )
            })?;
        for (index, choice) in state.choices.iter().enumerate() {
            cx.tick(CyclicMdpProbabilityPhase::ActionSums)?;
            let sum = action_sum(
                key,
                index,
                choice,
                values,
                reward,
                cx.limits,
                cx.cancel,
                &mut cx.meter,
                &super::super::reward::RewardControl::new(cx.cancel),
            )?;
            cx.reserve(
                usize::try_from(
                    sum.numerator()
                        .magnitude_bits()
                        .saturating_add(sum.denominator().magnitude_bits())
                        .div_ceil(8),
                )
                .unwrap_or(usize::MAX),
            )?;
            actions.push(sum);
        }
        sums.insert(key.clone(), actions);
    }
    Ok(sums)
}

fn equality_failure(
    graph: &CheckerGraph,
    values: &Values,
    sums: &Sums,
    objective: ProbabilityObjective,
    cx: &mut Context<'_, '_>,
) -> Result<Option<ProbabilityProductKey>, ProbabilityDocumentError> {
    for (key, state) in &graph.states {
        cx.tick(CyclicMdpProbabilityPhase::ActionSums)?;
        let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        if cx.fair_mdp
            && cx.bound_side == Some(SubsystemBoundKind::Lower)
            && state.terminal.is_none()
        {
            for choice in sums.get(key).ok_or(ProbabilityDocumentError::Malformed)? {
                if value > choice {
                    return Ok(Some(key.clone()));
                }
            }
            continue;
        }
        let expected = match state.terminal {
            Some(_) if cx.reward => rational(0),
            Some(target) => rational(i64::from(target)),
            None => {
                let choices = sums.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
                let mut expected: Option<Rational> = None;
                for choice in choices {
                    cx.tick(CyclicMdpProbabilityPhase::ActionSums)?;
                    expected = Some(match expected {
                        None => choice.clone(),
                        Some(previous) => match objective {
                            ProbabilityObjective::Minimum => previous.min(choice.clone()),
                            ProbabilityObjective::Maximum => previous.max(choice.clone()),
                            ProbabilityObjective::Workload
                            | ProbabilityObjective::WorkloadMinimum
                            | ProbabilityObjective::WorkloadMaximum => {
                                return Err(ProbabilityDocumentError::Unsupported);
                            }
                        },
                    });
                }
                expected.ok_or(ProbabilityDocumentError::Unsupported)?
            }
        };
        if *value != expected {
            return Ok(Some(key.clone()));
        }
    }
    Ok(None)
}

#[cfg(test)]
fn progress_failure(
    graph: &CheckerGraph,
    values: &Values,
    sums: &Sums,
    ranks: &BTreeMap<ProbabilityProductKey, u64>,
    objective: ProbabilityObjective,
    cx: &mut Context<'_, '_>,
) -> Result<Option<(ProbabilityCertificateRule, ProbabilityProductKey)>, ProbabilityDocumentError> {
    progress_failure_with_fair_zero(
        graph,
        values,
        sums,
        ranks,
        objective,
        FairProgress {
            zero: None,
            selected_policy: None,
        },
        cx,
    )
}

fn progress_failure_with_fair_zero(
    graph: &CheckerGraph,
    values: &Values,
    sums: &Sums,
    ranks: &BTreeMap<ProbabilityProductKey, u64>,
    objective: ProbabilityObjective,
    fair: FairProgress<'_>,
    cx: &mut Context<'_, '_>,
) -> Result<Option<(ProbabilityCertificateRule, ProbabilityProductKey)>, ProbabilityDocumentError> {
    for (key, state) in &graph.states {
        let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let rank = ranks.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        cx.tick(CyclicMdpProbabilityPhase::Progress)?;
        let certified_zero = !cx.reward
            && *value == rational(0)
            && (!cx.fair_mdp || fair.zero.is_some_and(|states| states.contains(key)));
        if state.terminal == Some(true) || certified_zero {
            if *rank != 0 {
                return Ok(Some((ProbabilityCertificateRule::RankBroken, key.clone())));
            }
            if state.terminal.is_none() && *value == rational(0) {
                cx.tick(CyclicMdpProbabilityPhase::ZeroSupport)?;
                let mut closed = false;
                for choice in &state.choices {
                    let mut zero_support = true;
                    for edge in choice {
                        cx.tick(CyclicMdpProbabilityPhase::ZeroSupport)?;
                        if edge.probability > rational(0)
                            && values.get(&edge.target) != Some(&rational(0))
                        {
                            zero_support = false;
                        }
                    }
                    match objective {
                        ProbabilityObjective::Minimum => closed |= zero_support,
                        ProbabilityObjective::Maximum if !zero_support => {
                            return Ok(Some((
                                ProbabilityCertificateRule::InequalityFails,
                                key.clone(),
                            )));
                        }
                        ProbabilityObjective::Maximum => closed = true,
                        ProbabilityObjective::Workload
                        | ProbabilityObjective::WorkloadMinimum
                        | ProbabilityObjective::WorkloadMaximum => {
                            return Err(ProbabilityDocumentError::Unsupported);
                        }
                    }
                }
                if !closed {
                    return Ok(Some((
                        ProbabilityCertificateRule::InequalityFails,
                        key.clone(),
                    )));
                }
            }
            continue;
        }
        if state.terminal == Some(false) || *rank == 0 {
            return Ok(Some((ProbabilityCertificateRule::RankBroken, key.clone())));
        }
        let action_sums = sums.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let mut any = false;
        let mut all = true;
        for (index, choice) in state.choices.iter().enumerate() {
            if fair
                .selected_policy
                .and_then(|policy| policy.get(key))
                .is_some_and(|selected| *selected != index)
            {
                continue;
            }
            let mut lower = false;
            for edge in choice {
                cx.tick(CyclicMdpProbabilityPhase::Progress)?;
                if edge.probability > rational(0)
                    && ranks.get(&edge.target).is_some_and(|next| next < rank)
                {
                    lower = true;
                }
            }
            all &= lower;
            if action_sums.get(index) == Some(value) {
                any |= lower;
            }
        }
        let progress = if let Some(policy) = fair.selected_policy.filter(|_| cx.fair_mdp) {
            let progresses = |index: usize| {
                state.choices.get(index).is_some_and(|choice| {
                    choice.iter().any(|edge| {
                        edge.probability > rational(0)
                            && ranks.get(&edge.target).is_some_and(|next| next < rank)
                    })
                })
            };
            if let Some(index) = policy.get(key) {
                action_sums.get(*index).is_some_and(|sum| sum <= value) && progresses(*index)
            } else {
                state.choices.iter().enumerate().any(|(index, _)| {
                    action_sums.get(index).is_some_and(|sum| sum <= value) && progresses(index)
                })
            }
        } else {
            match objective {
                ProbabilityObjective::Minimum if cx.reward => any,
                ProbabilityObjective::Minimum if cx.fair_mdp => any,
                ProbabilityObjective::Maximum if cx.reward => all && !state.choices.is_empty(),
                ProbabilityObjective::Minimum => all && !state.choices.is_empty(),
                ProbabilityObjective::Maximum => any,
                ProbabilityObjective::Workload
                | ProbabilityObjective::WorkloadMinimum
                | ProbabilityObjective::WorkloadMaximum => {
                    return Err(ProbabilityDocumentError::Unsupported);
                }
            }
        };
        if !progress {
            return Ok(Some((ProbabilityCertificateRule::RankBroken, key.clone())));
        }
    }
    Ok(None)
}

#[cfg(test)]
fn derive_ranks(
    graph: &CheckerGraph,
    values: &Values,
    sums: &Sums,
    objective: ProbabilityObjective,
    cx: &mut Context<'_, '_>,
) -> Result<BTreeMap<ProbabilityProductKey, u64>, ProbabilityDocumentError> {
    derive_ranks_with_fair_zero(graph, values, sums, objective, None, cx)
}

fn derive_ranks_with_fair_zero(
    graph: &CheckerGraph,
    values: &Values,
    sums: &Sums,
    objective: ProbabilityObjective,
    fair_zero: Option<&BTreeSet<ProbabilityProductKey>>,
    cx: &mut Context<'_, '_>,
) -> Result<BTreeMap<ProbabilityProductKey, u64>, ProbabilityDocumentError> {
    let mut ranks = BTreeMap::new();
    for (key, state) in &graph.states {
        cx.tick(CyclicMdpProbabilityPhase::Ranks)?;
        if state.terminal == Some(true)
            || !cx.reward
                && values.get(key) == Some(&rational(0))
                && (!cx.fair_mdp || fair_zero.is_some_and(|states| states.contains(key)))
        {
            cx.reserve(std::mem::size_of::<(ProbabilityProductKey, u64)>() + key.as_bytes().len())?;
            ranks.insert(key.clone(), 0);
        }
    }
    while ranks.len() < graph.states.len() {
        let mut changed = false;
        for (key, state) in &graph.states {
            cx.tick(CyclicMdpProbabilityPhase::Ranks)?;
            if ranks.contains_key(key) {
                continue;
            }
            let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
            let action_sums = sums.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
            let mut candidate = None;
            let mut all = !state.choices.is_empty();
            for (index, choice) in state.choices.iter().enumerate() {
                let mut lower = None;
                for edge in choice {
                    cx.tick(CyclicMdpProbabilityPhase::Ranks)?;
                    if edge.probability > rational(0) {
                        if let Some(rank) = ranks.get(&edge.target) {
                            lower = Some(lower.map_or(*rank, |previous: u64| previous.min(*rank)));
                        }
                    }
                }
                match objective {
                    _ if (objective == ProbabilityObjective::Minimum
                        && !cx.reward
                        && !cx.fair_mdp)
                        || (objective == ProbabilityObjective::Maximum && cx.reward) =>
                    {
                        match lower {
                            Some(rank) => {
                                candidate =
                                    Some(candidate.map_or(rank, |previous: u64| previous.max(rank)))
                            }
                            None => all = false,
                        }
                    }
                    ProbabilityObjective::Minimum | ProbabilityObjective::Maximum
                        if action_sums.get(index) == Some(value) =>
                    {
                        if let Some(rank) = lower {
                            candidate =
                                Some(candidate.map_or(rank, |previous: u64| previous.min(rank)));
                        }
                    }
                    ProbabilityObjective::Minimum | ProbabilityObjective::Maximum => {}
                    ProbabilityObjective::Workload
                    | ProbabilityObjective::WorkloadMinimum
                    | ProbabilityObjective::WorkloadMaximum => {
                        return Err(ProbabilityDocumentError::Unsupported);
                    }
                }
            }
            let universal =
                (objective == ProbabilityObjective::Minimum && !cx.reward && !cx.fair_mdp)
                    || (objective == ProbabilityObjective::Maximum && cx.reward);
            if (!universal || all) && candidate.is_some() {
                let rank = candidate
                    .ok_or(ProbabilityDocumentError::Malformed)?
                    .checked_add(1)
                    .filter(|rank| *rank <= MAX_COUNT)
                    .ok_or(exceeded(
                        ProbabilityDocumentLimit::Rank,
                        MAX_COUNT,
                        u64::MAX,
                    ))?;
                cx.reserve(
                    std::mem::size_of::<(ProbabilityProductKey, u64)>() + key.as_bytes().len(),
                )?;
                ranks.insert(key.clone(), rank);
                changed = true;
            }
        }
        if !changed {
            return Err(ProbabilityDocumentError::Unsupported);
        }
    }
    Ok(ranks)
}

fn admitted_vector<T>(
    count: usize,
    cx: &mut Context<'_, '_>,
) -> Result<Vec<T>, ProbabilityDocumentError> {
    let requested = count
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(ProbabilityDocumentError::KeySizeOverflow)?;
    cx.reserve(requested)?;
    let mut result = Vec::new();
    result.try_reserve_exact(count).map_err(|_| {
        ProbabilityDocumentError::KeyEncoding(fsl_eval::simulation::EncodingRefusal::Allocation {
            requested,
        })
    })?;
    Ok(result)
}

fn construct(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    cx: &mut Context<'_, '_>,
) -> Result<ProbabilityProofDocument, ProbabilityDocumentError> {
    cx.reward = matches!(
        bind_claim(claims, item, cx.cancel)?.form,
        ProbForm::ExpectedReward { .. }
    );
    let (graph, objective, _, reward, event) = subject(system, claims, item, cx, None)?;
    let converted;
    let certificate = if event.is_some() {
        let ProbForm::Reach { bound, .. } = &bind_claim(claims, item, cx.cancel)?.form else {
            return Err(ProbabilityDocumentError::Unsupported);
        };
        converted = super::super::cyclic_state::violated_values(
            &graph,
            certificate,
            bound,
            cx.limits,
            cx.cancel,
            &mut cx.meter,
        )?;
        &converted
    } else {
        certificate
    };
    let values = match rows(&graph, certificate, cx, false)? {
        CertificateValues::Complete(values) => values,
        CertificateValues::Partial { .. } => {
            return Err(ProbabilityDocumentError::MalformedMember("values"));
        }
        CertificateValues::Rejected(_) => {
            return Err(ProbabilityDocumentError::MalformedMember("values"));
        }
    };
    let action_sums = sums(&graph, &values, reward.as_ref(), cx)?;
    if equality_failure(&graph, &values, &action_sums, objective, cx)?.is_some() {
        return Err(ProbabilityDocumentError::MalformedMember("values"));
    }
    let fair_zero = if cx.fair_mdp {
        let claim = bind_claim(claims, item, cx.cancel)?;
        Some(super::fairness::non_target_fair_end_component_states(
            system,
            claim,
            &graph,
            cx.limits,
            cx.cancel,
            cx.identity,
            &mut cx.meter,
            &mut cx.storage,
        )?)
    } else {
        None
    };
    let ranks = derive_ranks_with_fair_zero(
        &graph,
        &values,
        &action_sums,
        objective,
        fair_zero.as_ref(),
        cx,
    )?;
    let mut ranking = admitted_vector(ranks.len(), cx)?;
    for row in ranks {
        cx.tick(CyclicMdpProbabilityPhase::Ranks)?;
        ranking.push(row);
    }
    let mut targets = admitted_vector(graph.states.len(), cx)?;
    for (key, state) in &graph.states {
        cx.tick(CyclicMdpProbabilityPhase::Rows)?;
        if state.terminal == Some(true) {
            cx.reserve(key.as_bytes().len())?;
            targets.push(key.clone());
        }
    }
    let mut offered = admitted_vector(values.len(), cx)?;
    for (state, value) in values {
        cx.tick(CyclicMdpProbabilityPhase::Rows)?;
        offered.push(ExactProbabilityValue { state, value });
    }
    Ok(ProbabilityProofDocument {
        cyclic_mdp: true,
        fair_mdp: false,
        fairness: Vec::new(),
        document: ProbabilityCertificateDocument {
            cyclic_dtmc: false,
            identity: item.clone(),
            targets,
            event,
            failures: Vec::new(),
            reward: reward.map(|reward| reward.objective),
            optimization: objective,
            timed: None,
            values: offered,
            ranking,
            policy: None,
        },
    })
}

#[cfg(feature = "test-support")]
pub(super) fn check<'a>(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &'a Cancel,
    observe: Option<&'a mut dyn FnMut(CyclicMdpProbabilityPhase)>,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    check_with_context(
        system,
        claims,
        item,
        bytes,
        Offer::exact(),
        Context::new(limits, cancel, observe),
    )
}

#[allow(clippy::too_many_arguments)]
#[implements("FR-203-AC-30")]
pub(super) fn check_with_reward_control<'a, 'control>(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    offer: Offer<'_>,
    limits: ProbabilityDocumentLimits,
    cancel: &'a Cancel,
    control: &'a super::super::reward::RewardControl<'control>,
    work: Option<&'a filament_exact::WorkScope>,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel, None);
    cx.meter = limits.accounting_meter_in_scope(cancel, work);
    cx.work = work;
    cx.reward_control = Some(control);
    check_with_context(system, claims, item, bytes, offer, cx)
}

#[implements("FR-203-AC-30")]
pub(super) fn check_with_value_bound_control<'a, 'control>(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    offer: Offer<'_>,
    control: ValueBoundControl<'a, 'control>,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut cx = Context::new(control.limits, control.cancel, None);
    cx.meter = control
        .limits
        .accounting_meter_in_scope(control.cancel, control.work);
    cx.work = control.work;
    cx.reward_control = Some(control.reward);
    cx.bound_side = offer.side;
    check_with_context(system, claims, item, bytes, offer, cx)
}

fn check_with_context(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    offer: Offer<'_>,
    mut cx: Context<'_, '_>,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let limits = cx.limits;
    let cancel = cx.cancel;
    let fair_mdp = matches!(
        profile_dispatch(bytes, limits, cancel)?,
        ProfileDispatch::FairMdp
    );
    let fair_lower_bound = fair_mdp && cx.bound_side == Some(SubsystemBoundKind::Lower);
    cx.fair_mdp = fair_mdp;
    let mut fairness_records = Vec::new();
    let document = match read_document_profile(
        bytes,
        limits,
        cancel,
        &mut cx.meter,
        !fair_mdp,
        Some(&mut fairness_records),
        &mut cx.storage,
    ) {
        Ok(document) => document,
        Err(error) => return document_admission_error(error),
    };
    if &document.identity != item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    if fair_mdp {
        let claim = bind_claim(claims, item, cancel)?;
        let expected = super::fairness::checked(
            system,
            claim,
            limits,
            cancel,
            &mut cx.meter,
            &mut cx.storage,
        )?;
        if !super::fairness::same(
            &expected,
            &fairness_records,
            limits,
            cancel,
            &mut cx.meter,
            &mut cx.storage,
        )? {
            return Err(ProbabilityDocumentError::ObjectiveMismatch);
        }
    } else if !fairness_records.is_empty() {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    cx.reward = document.reward.is_some();
    if cx.reward
        != matches!(
            bind_claim(claims, item, cx.cancel)?.form,
            ProbForm::ExpectedReward { .. }
        )
    {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    // A bound document resolves its existential choices by their transition
    // identities, so it keeps the model state of every key it rebuilds.
    let captured = offer
        .side
        .map(|_| super::super::cyclic::CapturedStates::default());
    let (graph, objective, bound, reward, event) =
        subject(system, claims, item, &mut cx, captured.as_ref())?;
    let mut targets = document.targets.iter();
    let mut target_match = true;
    for (key, state) in &graph.states {
        cx.tick(CyclicMdpProbabilityPhase::Rows)?;
        if state.terminal == Some(true) && targets.next() != Some(key) {
            target_match = false;
        }
    }
    if targets.next().is_some() {
        target_match = false;
    }
    if document.optimization != objective
        || document.event != event
        || !target_match
        || document.reward.as_ref() != reward.as_ref().map(|reward| &reward.objective)
    {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    if let (Some(side), Some(captured)) = (offer.side, captured) {
        // FR-1042 value bounds currently cover cyclic reward only on a
        // structurally valid expected-reward subject; state-event bounds use
        // their separate profile rules.
        if event.is_some() {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        let relation = offer.relation();
        if relation.misplaced(&bound) {
            return Ok(misplaced_kind());
        }
        let allow_partial_reward_lower = cx.reward && side == SubsystemBoundKind::Lower;
        let mut values = match rows(
            &graph,
            &ExactProbabilityCertificate {
                values: document.values,
            },
            &mut cx,
            allow_partial_reward_lower,
        )? {
            CertificateValues::Complete(values) => values,
            CertificateValues::Partial { values, .. } if allow_partial_reward_lower => values,
            CertificateValues::Partial { .. } => {
                return Err(ProbabilityDocumentError::MalformedMember("values"));
            }
            CertificateValues::Rejected(rejection) => {
                return Ok(ExactProbabilityCertificateCheck::Rejected(rejection));
            }
        };
        if cx.reward && side == SubsystemBoundKind::Lower {
            for key in graph.states.keys() {
                values.entry(key.clone()).or_insert_with(|| rational(0));
            }
        }
        let existential = matches!(
            (objective, side),
            (ProbabilityObjective::Minimum, SubsystemBoundKind::Upper)
                | (ProbabilityObjective::Maximum, SubsystemBoundKind::Lower)
        );
        let ranks = if fair_mdp && existential {
            match offered_ranks(&graph, document.ranking, &mut cx)? {
                Some(ranks) => ranks,
                None => return Ok(malformed_ranking()),
            }
        } else if fair_mdp {
            if !document.ranking.is_empty() {
                return Ok(malformed_ranking());
            }
            BTreeMap::new()
        } else if cx.reward {
            match side {
                SubsystemBoundKind::Lower => BTreeMap::new(),
                SubsystemBoundKind::Upper => {
                    match offered_ranks(&graph, document.ranking, &mut cx)? {
                        Some(ranks) => ranks,
                        None => return Ok(malformed_ranking()),
                    }
                }
            }
        } else {
            match side {
                SubsystemBoundKind::Lower => {
                    match offered_ranks(&graph, document.ranking, &mut cx)? {
                        Some(ranks) => ranks,
                        None => return Ok(malformed_ranking()),
                    }
                }
                SubsystemBoundKind::Upper => BTreeMap::new(),
            }
        };
        let hints = match offer.policy {
            None => BTreeMap::new(),
            Some(_) if !existential => {
                return document_admission_error(ProbabilityDocumentError::MalformedMember(
                    "policy",
                ));
            }
            Some(rows) => match hinted_choices(system, &graph, &captured, rows, &mut cx) {
                Ok(hints) => hints,
                Err(error) => return document_admission_error(error),
            },
        };
        let fair_zero = if fair_mdp {
            let claim = bind_claim(claims, item, cancel)?;
            let states = super::fairness::non_target_fair_end_component_states(
                system,
                claim,
                &graph,
                limits,
                cancel,
                cx.identity,
                &mut cx.meter,
                &mut cx.storage,
            )?;
            for key in &states {
                if values.get(key) != Some(&rational(0)) {
                    return Ok(rejected(
                        ProbabilityCertificateRule::InequalityFails,
                        key.clone(),
                    ));
                }
            }
            Some(states)
        } else {
            None
        };
        let action_sums = sums(&graph, &values, reward.as_ref(), &mut cx)?;
        let selected = match bound_failure(
            system,
            &graph,
            &captured,
            &values,
            &action_sums,
            &ranks,
            objective,
            side,
            &hints,
            fair_zero.as_ref(),
            &mut cx,
        )? {
            Ok(selected) => selected,
            Err((rule, key)) => return Ok(rejected(rule, key)),
        };
        let mut result = check_threshold(
            &graph,
            &values,
            &bound,
            relation.tests_threshold(),
            limits,
            cancel,
        )?;
        if let ExactProbabilityCertificateCheck::Accepted(accepted) = &mut result {
            accepted.policy = selected;
        }
        return Ok(result);
    }
    let values = match rows(
        &graph,
        &ExactProbabilityCertificate {
            values: document.values,
        },
        &mut cx,
        false,
    )? {
        CertificateValues::Complete(values) => values,
        CertificateValues::Partial { .. } => {
            return Err(ProbabilityDocumentError::MalformedMember("values"));
        }
        CertificateValues::Rejected(rejection) => {
            return Ok(ExactProbabilityCertificateCheck::Rejected(rejection));
        }
    };
    let policy = policy_indices(document.policy.as_deref(), &graph)?;
    let mut ranks = BTreeMap::new();
    for (key, rank) in document.ranking {
        cx.tick(CyclicMdpProbabilityPhase::Rows)?;
        cx.reserve(std::mem::size_of::<(ProbabilityProductKey, u64)>() + key.as_bytes().len())?;
        if !graph.states.contains_key(&key) {
            return Ok(malformed_ranking());
        }
        ranks.insert(key, rank);
    }
    if fair_lower_bound {
        if !ranks.is_empty() {
            return Ok(malformed_ranking());
        }
    } else if ranks.len() != graph.states.len() {
        return Ok(malformed_ranking());
    }
    let fair_zero = if fair_mdp {
        let claim = bind_claim(claims, item, cancel)?;
        let states = super::fairness::non_target_fair_end_component_states(
            system,
            claim,
            &graph,
            limits,
            cancel,
            cx.identity,
            &mut cx.meter,
            &mut cx.storage,
        )?;
        for key in &states {
            cx.tick(CyclicMdpProbabilityPhase::Rows)?;
            if values.get(key) != Some(&rational(0)) {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                ));
            }
        }
        Some(states)
    } else {
        None
    };
    let action_sums = sums(&graph, &values, reward.as_ref(), &mut cx)?;
    let fair_upper_bound = fair_mdp && cx.bound_side == Some(SubsystemBoundKind::Upper);
    if fair_mdp && cx.bound_side.is_none() {
        for (key, index) in &policy {
            let actual = action_sums
                .get(key)
                .and_then(|actions| actions.get(*index))
                .ok_or(ProbabilityDocumentError::MalformedMember("policy"))?;
            if values.get(key) != Some(actual) {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                ));
            }
        }
    }
    if fair_upper_bound {
        if let Some(key) =
            fair_upper_bound_failure(&graph, &values, &action_sums, &policy, &mut cx)?
        {
            return Ok(rejected(ProbabilityCertificateRule::InequalityFails, key));
        }
    } else if let Some(key) = equality_failure(&graph, &values, &action_sums, objective, &mut cx)? {
        return Ok(rejected(ProbabilityCertificateRule::InequalityFails, key));
    }
    if !fair_lower_bound {
        if let Some((rule, key)) = progress_failure_with_fair_zero(
            &graph,
            &values,
            &action_sums,
            &ranks,
            objective,
            FairProgress {
                zero: fair_zero.as_ref(),
                selected_policy: if fair_upper_bound || (fair_mdp && !policy.is_empty()) {
                    Some(&policy)
                } else {
                    None
                },
            },
            &mut cx,
        )? {
            return Ok(rejected(rule, key));
        }
    }
    let mut result = check_threshold(&graph, &values, &bound, true, limits, cancel)?;
    if event.is_some() {
        let ProbForm::Reach { bound, .. } = &bind_claim(claims, item, cancel)?.form else {
            return Err(ProbabilityDocumentError::Unsupported);
        };
        result = super::super::cyclic_state::claim_acceptance(
            result,
            bound,
            limits,
            cancel,
            &mut cx.meter,
        )?;
    }
    if let ExactProbabilityCertificateCheck::Accepted(accepted) = &mut result {
        accepted.document = Some(bytes.into());
    }
    Ok(result)
}

/// The offered `ranking` as a map over exactly the graph's states; `None`
/// when a row names another state or a state has no row.
fn offered_ranks(
    graph: &CheckerGraph,
    ranking: Vec<(ProbabilityProductKey, u64)>,
    cx: &mut Context<'_, '_>,
) -> Result<Option<BTreeMap<ProbabilityProductKey, u64>>, ProbabilityDocumentError> {
    let mut ranks = BTreeMap::new();
    for (key, rank) in ranking {
        cx.tick(CyclicMdpProbabilityPhase::Rows)?;
        cx.reserve(std::mem::size_of::<(ProbabilityProductKey, u64)>() + key.as_bytes().len())?;
        if !graph.states.contains_key(&key) {
            return Ok(None);
        }
        ranks.insert(key, rank);
    }
    Ok((ranks.len() == graph.states.len()).then_some(ranks))
}

/// The canonical schedule of each enabled action at the undecided `key`, in
/// the order of the rebuilt graph's choices there.
fn schedules_at(
    system: &ModelSystem<'_>,
    captured: &super::super::cyclic::CapturedStates,
    key: &ProbabilityProductKey,
    cx: &mut Context<'_, '_>,
) -> Result<Vec<crate::Schedule>, ProbabilityDocumentError> {
    let state = captured
        .borrow()
        .get(key)
        .cloned()
        .ok_or(ProbabilityDocumentError::Malformed)?;
    let mut admission = ProbabilityKeyAdmission::new(
        &mut cx.meter,
        &mut cx.storage,
        cx.limits.storage_bytes,
        cx.cancel,
    );
    let actions = system
        .actions_with_budget(&state, cx.identity, &mut admission, cx.cancel)
        .map_err(|error| admission.expansion_refusal(error))?
        .map_err(|_| {
            ProbabilityDocumentError::Build(Box::new(
                crate::ExactProbBuildIssue::UndecidedSuccessor,
            ))
        })?;
    let mut schedules = admitted_vector(actions.len(), cx)?;
    for action in &actions {
        cx.tick(CyclicMdpProbabilityPhase::Rows)?;
        schedules.push(
            crate::Schedule::of(system, &action.identity)
                .map_err(|_| ProbabilityDocumentError::Malformed)?,
        );
    }
    Ok(schedules)
}

/// Resolve each `policy` row to the index of the hinted action among the
/// rebuilt choices at its state, `None` when no enabled action there has that
/// identity. Rows must name undecided states in strictly ascending key order
/// and carry an FR-181 transition identity.
fn hinted_choices(
    system: &ModelSystem<'_>,
    graph: &CheckerGraph,
    captured: &super::super::cyclic::CapturedStates,
    rows: &[super::value_bounds::PolicyHint],
    cx: &mut Context<'_, '_>,
) -> Result<BTreeMap<ProbabilityProductKey, Option<usize>>, ProbabilityDocumentError> {
    let malformed = ProbabilityDocumentError::MalformedMember("policy");
    let mut hints = BTreeMap::new();
    for row in rows {
        cx.tick(CyclicMdpProbabilityPhase::Rows)?;
        let key = decode_key(&row.state, cx.limits).map_err(|_| malformed.clone())?;
        if hints.last_key_value().is_some_and(|(last, _)| *last >= key)
            || graph
                .states
                .get(&key)
                .is_none_or(|state| state.terminal.is_some())
        {
            return Err(malformed);
        }
        let hinted = crate::Schedule::read_transition(
            row.choice.as_bytes(),
            quire_canonical::Limits::new(cx.limits.bytes),
        )
        .map_err(|_| malformed.clone())?;
        let schedules = schedules_at(system, captured, &key, cx)?;
        let index = schedules.iter().position(|schedule| *schedule == hinted);
        cx.reserve(
            std::mem::size_of::<(ProbabilityProductKey, Option<usize>)>() + key.as_bytes().len(),
        )?;
        hints.insert(key, index);
    }
    Ok(hints)
}

/// Whether choice `index` at `key` reaches a positive-valued successor of
/// strictly smaller rank with positive probability (FSpec FR-412 progress).
fn progresses(
    state: &CheckerState,
    index: usize,
    rank: u64,
    values: &Values,
    ranks: &BTreeMap<ProbabilityProductKey, u64>,
    cx: &mut Context<'_, '_>,
) -> Result<bool, ProbabilityDocumentError> {
    let choice = state
        .choices
        .get(index)
        .ok_or(ProbabilityDocumentError::Malformed)?;
    for edge in choice {
        cx.tick(CyclicMdpProbabilityPhase::Progress)?;
        if edge.probability > rational(0)
            && (cx.reward
                || values
                    .get(&edge.target)
                    .is_some_and(|value| *value > rational(0)))
            && ranks.get(&edge.target).is_some_and(|next| *next < rank)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// FR-1042's per-state conditions for a `lower` or `upper` document over a
/// cyclic MDP. Decided states carry their boundary value exactly. A
/// universal side (`lower` of a minimum, `upper` of a maximum) holds under
/// every action; an existential side holds under the hinted action, or under
/// the first action in canonical full-product-action order that satisfies
/// its inequality and, for a positive `lower` value, progress. Returns the
/// selected action at each existential state, or the first failing rule and
/// state in key order.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn bound_failure(
    system: &ModelSystem<'_>,
    graph: &CheckerGraph,
    captured: &super::super::cyclic::CapturedStates,
    values: &Values,
    sums: &Sums,
    ranks: &BTreeMap<ProbabilityProductKey, u64>,
    objective: ProbabilityObjective,
    side: SubsystemBoundKind,
    hints: &BTreeMap<ProbabilityProductKey, Option<usize>>,
    fair_zero: Option<&BTreeSet<ProbabilityProductKey>>,
    cx: &mut Context<'_, '_>,
) -> Result<
    Result<
        Vec<(ProbabilityProductKey, crate::ProductAction)>,
        (ProbabilityCertificateRule, ProbabilityProductKey),
    >,
    ProbabilityDocumentError,
> {
    if cx.reward && side == SubsystemBoundKind::Upper && !hints.is_empty() {
        return Err(ProbabilityDocumentError::MalformedMember("policy"));
    }
    let existential = match (objective, side) {
        (ProbabilityObjective::Minimum, SubsystemBoundKind::Upper)
        | (ProbabilityObjective::Maximum, SubsystemBoundKind::Lower) => true,
        (ProbabilityObjective::Minimum, SubsystemBoundKind::Lower)
        | (ProbabilityObjective::Maximum, SubsystemBoundKind::Upper) => false,
        (
            ProbabilityObjective::Workload
            | ProbabilityObjective::WorkloadMinimum
            | ProbabilityObjective::WorkloadMaximum,
            _,
        ) => return Err(ProbabilityDocumentError::Unsupported),
    } && !(cx.reward && side == SubsystemBoundKind::Upper);
    let mut selected = Vec::new();
    for (key, state) in &graph.states {
        cx.tick(CyclicMdpProbabilityPhase::ActionSums)?;
        let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        if let Some(target) = state.terminal {
            let terminal_value = if cx.reward {
                rational(0)
            } else {
                rational(i64::from(target))
            };
            if *value != terminal_value {
                return Ok(Err((
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                )));
            }
            continue;
        }
        if cx.fair_mdp && existential {
            let rank = ranks.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
            if (*value == rational(0)) != (*rank == 0) {
                return Ok(Err((ProbabilityCertificateRule::RankBroken, key.clone())));
            }
            if fair_zero.is_some_and(|states| states.contains(key)) && *value != rational(0) {
                return Ok(Err((
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                )));
            }
        }
        let action_sums = sums.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        if action_sums.is_empty() {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        let inequality = |index: usize| -> Result<bool, ProbabilityDocumentError> {
            let sum = action_sums
                .get(index)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            Ok(match side {
                SubsystemBoundKind::Lower => value <= sum,
                SubsystemBoundKind::Upper => sum <= value,
            })
        };
        // On a fair-MDP existential endpoint, progress proves termination
        // outside the independently reconstructed non-target fair MEC. The
        // ordinary bounded profile uses progress only for lower documents.
        let needs_progress = if cx.fair_mdp {
            existential && *value > rational(0)
        } else {
            cx.reward && side == SubsystemBoundKind::Upper
                || !cx.reward && side == SubsystemBoundKind::Lower && *value > rational(0)
        };
        let rank = if needs_progress {
            *ranks.get(key).ok_or(ProbabilityDocumentError::Malformed)?
        } else {
            0
        };
        if !existential {
            for index in 0..action_sums.len() {
                if !inequality(index)? {
                    return Ok(Err((
                        ProbabilityCertificateRule::InequalityFails,
                        key.clone(),
                    )));
                }
                if needs_progress && !progresses(state, index, rank, values, ranks, cx)? {
                    return Ok(Err((ProbabilityCertificateRule::RankBroken, key.clone())));
                }
            }
            continue;
        }
        let schedules = schedules_at(system, captured, key, cx)?;
        if schedules.len() != action_sums.len() {
            return Err(ProbabilityDocumentError::Malformed);
        }
        let candidates = match hints.get(key) {
            Some(None) => {
                return Ok(Err((
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                )));
            }
            Some(Some(index)) => vec![*index],
            None => {
                let mut order: Vec<usize> = (0..schedules.len()).collect();
                order.sort_by(|left, right| schedules[*left].cmp(&schedules[*right]));
                order
            }
        };
        let mut satisfies_inequality = false;
        let mut choice = None;
        for index in candidates {
            if !inequality(index)? {
                continue;
            }
            satisfies_inequality = true;
            if !needs_progress || progresses(state, index, rank, values, ranks, cx)? {
                choice = Some(index);
                break;
            }
        }
        let Some(index) = choice else {
            let rule = if satisfies_inequality {
                ProbabilityCertificateRule::RankBroken
            } else {
                ProbabilityCertificateRule::InequalityFails
            };
            return Ok(Err((rule, key.clone())));
        };
        let schedule = schedules
            .into_iter()
            .nth(index)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        cx.reserve(
            key.as_bytes()
                .len()
                .saturating_add(schedule.as_bytes().len())
                .saturating_add(std::mem::size_of::<(
                    ProbabilityProductKey,
                    crate::ProductAction,
                )>()),
        )?;
        selected.push((key.clone(), crate::ProductAction::Identity(schedule)));
    }
    Ok(Ok(selected))
}

fn policy_indices(
    rows: Option<&[(ProbabilityProductKey, Vec<u8>)]>,
    graph: &CheckerGraph,
) -> Result<BTreeMap<ProbabilityProductKey, usize>, ProbabilityDocumentError> {
    let mut selected = BTreeMap::new();
    for (state, choice) in rows.unwrap_or_default() {
        let actions = graph
            .actions_by_state
            .as_ref()
            .and_then(|actions| actions.get(state))
            .ok_or(ProbabilityDocumentError::MalformedMember("policy"))?;
        let index = actions
            .iter()
            .position(|action| {
                action
                    .as_ref()
                    .is_some_and(|action| action.as_bytes() == choice)
            })
            .ok_or(ProbabilityDocumentError::MalformedMember("policy"))?;
        if selected.insert(state.clone(), index).is_some() {
            return Err(ProbabilityDocumentError::MalformedMember("policy"));
        }
    }
    Ok(selected)
}

fn fair_upper_bound_failure(
    graph: &CheckerGraph,
    values: &Values,
    sums: &Sums,
    policy: &BTreeMap<ProbabilityProductKey, usize>,
    cx: &mut Context<'_, '_>,
) -> Result<Option<ProbabilityProductKey>, ProbabilityDocumentError> {
    for (key, state) in &graph.states {
        if state.terminal.is_some() {
            continue;
        }
        let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let actions = sums.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let satisfying = |index: usize| actions.get(index).is_some_and(|sum| sum <= value);
        let valid = if let Some(index) = policy.get(key) {
            satisfying(*index)
        } else {
            (0..actions.len()).any(satisfying)
        };
        cx.tick(CyclicMdpProbabilityPhase::ActionSums)?;
        if !valid {
            return Ok(Some(key.clone()));
        }
    }
    Ok(None)
}

/// Observe real MDP action, closure and progress work without replacing cancellation.
#[cfg(feature = "test-support")]
#[allow(clippy::too_many_arguments)]
pub fn check_cyclic_mdp_probability_certificate_observing<'a>(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &'a Cancel,
    observe: &'a mut dyn FnMut(CyclicMdpProbabilityPhase),
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    check(system, claims, item, bytes, limits, cancel, Some(observe))
}

/// Exercise this profile's real owner identity ceiling, without changing wire limits.
#[cfg(feature = "test-support")]
#[allow(clippy::too_many_arguments)]
pub fn check_cyclic_mdp_probability_certificate_with_identity(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    identity: fsl_foundation::IdentityLimits,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel, None);
    cx.identity = identity;
    check_with_context(system, claims, item, bytes, Offer::exact(), cx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    #[trace("TC-1405", "FR-201-AC-45")]
    #[test]
    fn cyclic_mdp_reward_codec_retains_compound_unit_and_large_finite_values() {
        let limits = super::super::tests::limits();
        let mut document = super::super::tests::document();
        document.optimization = ProbabilityObjective::Minimum;
        document.values[0].value = rational(3);
        document.reward = Some(ProbabilityRewardObjective {
            model: document.identity.node().clone(),
            name: filament_exact::Identifier::new("cost").unwrap(),
            unit: Some(filament_exact::UnitId::compound(vec![(
                document.identity.node().clone(),
                2_i64.into(),
            )])),
        });
        let proof = ProbabilityProofDocument {
            document,
            cyclic_mdp: true,
            fair_mdp: false,
            fairness: Vec::new(),
        };
        let cancel = Cancel::new();
        let bytes = write_probability_proof_document(&proof, limits, &cancel).unwrap();
        let read = read_probability_proof_document(&bytes, limits, &cancel).unwrap();
        assert_eq!(read, proof);
        assert!(read.is_cyclic_mdp_reward());
        assert_eq!(
            write_probability_proof_document(&read, limits, &cancel).unwrap(),
            bytes
        );
        assert_eq!(
            read_probability_certificate(&bytes, limits, &cancel),
            Err(ProbabilityDocumentError::Unsupported)
        );
    }

    fn key(n: u8) -> ProbabilityProductKey {
        ProbabilityProductKey(vec![n])
    }
    fn fraction(n: i64, d: i64) -> Rational {
        Rational::new(n.into(), d.into()).unwrap()
    }
    fn fixture(objective: ProbabilityObjective) -> (CheckerGraph, Values) {
        let edge = |state, n, d| ProbabilityEdge {
            target: key(state),
            probability: fraction(n, d),
        };
        let states = BTreeMap::from([
            (
                key(0),
                CheckerState {
                    terminal: None,
                    choices: vec![
                        vec![edge(3, 1, 2), edge(1, 1, 2)],
                        vec![edge(3, 1, 4), edge(1, 3, 4)],
                    ],
                },
            ),
            (
                key(1),
                CheckerState {
                    terminal: None,
                    choices: vec![vec![edge(2, 2, 3), edge(0, 1, 3)]],
                },
            ),
            (
                key(2),
                CheckerState {
                    terminal: Some(true),
                    choices: vec![],
                },
            ),
            (
                key(3),
                CheckerState {
                    terminal: Some(false),
                    choices: vec![],
                },
            ),
        ]);
        let (s, t) = match objective {
            ProbabilityObjective::Minimum => (fraction(2, 5), fraction(4, 5)),
            ProbabilityObjective::Maximum => (fraction(2, 3), fraction(8, 9)),
            _ => unreachable!(),
        };
        (
            CheckerGraph {
                initial: key(0),
                states,
                scheduler_actions: Default::default(),
                actions_by_state: None,
            },
            BTreeMap::from([
                (key(0), s),
                (key(1), t),
                (key(2), rational(1)),
                (key(3), rational(0)),
            ]),
        )
    }
    #[trace("TC-1404", "FR-201-AC-40", "FR-201-AC-41")]
    #[test]
    fn equal_positive_ranks_progress_through_zero_and_attractor_witnesses_hold() {
        for objective in [ProbabilityObjective::Minimum, ProbabilityObjective::Maximum] {
            let (graph, values) = fixture(objective);
            let cancel = Cancel::new();
            let mut cx = Context::new(super::super::tests::limits(), &cancel, None);
            let sums = sums(&graph, &values, None, &mut cx).unwrap();
            assert_eq!(
                equality_failure(&graph, &values, &sums, objective, &mut cx).unwrap(),
                None
            );
            let offered = BTreeMap::from([(key(0), 1), (key(1), 1), (key(2), 0), (key(3), 0)]);
            assert_eq!(
                progress_failure(&graph, &values, &sums, &offered, objective, &mut cx).unwrap(),
                None
            );
            let derived = derive_ranks(&graph, &values, &sums, objective, &mut cx).unwrap();
            assert_eq!(
                progress_failure(&graph, &values, &sums, &derived, objective, &mut cx).unwrap(),
                None
            );
            let broken = BTreeMap::from([(key(0), 0), (key(1), 1), (key(2), 0), (key(3), 0)]);
            assert!(
                progress_failure(&graph, &values, &sums, &broken, objective, &mut cx)
                    .unwrap()
                    .is_some()
            );
        }
    }
    #[trace("TC-1404", "FR-201-AC-40", "FR-201-AC-41")]
    #[test]
    fn probability_zero_edges_do_not_progress_and_non_target_one_needs_positive_rank() {
        let cancel = Cancel::new();
        let mut cx = Context::new(super::super::tests::limits(), &cancel, None);
        let graph = CheckerGraph {
            initial: key(0),
            actions_by_state: None,
            states: BTreeMap::from([
                (
                    key(0),
                    CheckerState {
                        terminal: None,
                        choices: vec![vec![
                            ProbabilityEdge {
                                target: key(0),
                                probability: rational(1),
                            },
                            ProbabilityEdge {
                                target: key(1),
                                probability: rational(0),
                            },
                        ]],
                    },
                ),
                (
                    key(1),
                    CheckerState {
                        terminal: Some(true),
                        choices: vec![],
                    },
                ),
            ]),
            scheduler_actions: Default::default(),
        };
        let values = BTreeMap::from([(key(0), rational(1)), (key(1), rational(1))]);
        let action_sums = sums(&graph, &values, None, &mut cx).unwrap();
        for objective in [ProbabilityObjective::Minimum, ProbabilityObjective::Maximum] {
            assert!(progress_failure(
                &graph,
                &values,
                &action_sums,
                &BTreeMap::from([(key(0), 1), (key(1), 0)]),
                objective,
                &mut cx
            )
            .unwrap()
            .is_some());
        }
        let mut graph = graph;
        graph.states.get_mut(&key(0)).unwrap().choices[0] = vec![ProbabilityEdge {
            target: key(1),
            probability: rational(1),
        }];
        let action_sums = sums(&graph, &values, None, &mut cx).unwrap();
        for objective in [ProbabilityObjective::Minimum, ProbabilityObjective::Maximum] {
            assert_eq!(
                progress_failure(
                    &graph,
                    &values,
                    &action_sums,
                    &BTreeMap::from([(key(0), 1), (key(1), 0)]),
                    objective,
                    &mut cx
                )
                .unwrap(),
                None
            );
            assert!(progress_failure(
                &graph,
                &values,
                &action_sums,
                &BTreeMap::from([(key(0), 0), (key(1), 0)]),
                objective,
                &mut cx
            )
            .unwrap()
            .is_some());
        }
    }
}
