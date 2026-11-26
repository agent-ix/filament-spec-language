// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent checked-subject replay of finite probabilistic witnesses.

use super::*;
use crate::ProductAction;
use crate::{ExactProbLimits, ModelCheckLimit};
use filament_exact::Value as SemanticValue;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{
    same_values, PackageModelInput, PredicateReading, ScheduledIdentity, TimedSubjectRefusal,
    TimedSystem, TransitionId,
};
use fsl_package::CheckedPackage;

mod bounded;
mod measure;
mod step;
mod subsystem;
mod undefined;
pub use undefined::{replay_probabilistic_undefined, ProbabilityUndefinedWitness};

/// A semantic operation, draw and resulting state in a finite witness.
#[derive(Clone, Debug)]
pub struct ProbabilityWitnessStep {
    /// The complete FR-202 choice, including digital delay and idle-tail
    /// actions. Identity choices carry the semantic operation below.
    pub choice: ProductAction,
    /// The scheduled operation, receiver and non-random arguments, when this
    /// step is an identity choice.
    pub action: Option<ScheduledIdentity>,
    /// The actual resolved transition, including its random arguments, when
    /// this step is an identity choice.
    pub transition: Option<ModelTransition>,
    /// The random argument vector in declaration order, when this step is an
    /// identity choice.
    pub draw: Option<Vec<SemanticValue>>,
    /// The actual post-state.
    pub post: ModelSimState,
    /// The complete canonical post-product key.
    pub post_key: ProbabilityProductKey,
    /// Exact time elapsed in the subject's declared time unit. Delay actions
    /// carry their positive duration; non-delay actions carry zero.
    pub elapsed: Rational,
}

fn same_draw(left: &Option<Vec<SemanticValue>>, right: &Option<Vec<SemanticValue>>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => same_values(left, right),
        (None, None) => true,
        _ => false,
    }
}

/// One finite path from the witness's initial state.
#[derive(Clone, Debug)]
pub struct ProbabilityWitnessPath {
    /// The semantic steps in execution order.
    pub steps: Vec<ProbabilityWitnessStep>,
}

/// A bounded, canonical subsystem certificate carried as unchecked evidence.
#[derive(Clone, Debug)]
pub struct ProbabilitySubsystemCertificate {
    bytes: Box<[u8]>,
}
impl ProbabilitySubsystemCertificate {
    /// Admit canonical mean, long-run or counterexample-role value-bound
    /// evidence without accepting its claim.
    ///
    /// # Errors
    /// Closed document syntax, caller limits or cancellation.
    pub fn from_bytes(
        bytes: &[u8],
        limits: ProbabilityDocumentLimits,
        cancel: &Cancel,
    ) -> Result<Self, ProbabilityDocumentError> {
        super::document::refutation::admit_subsystem(bytes, limits, cancel)?;
        Ok(Self {
            bytes: bytes.into(),
        })
    }
    /// Encode an unweighted long-run certificate without accepting it.
    ///
    /// # Errors
    /// Strict certificate admission, caller limits or cancellation.
    pub fn from_long_run(
        document: &LongRunProbabilityDocument,
        limits: ProbabilityDocumentLimits,
        cancel: &Cancel,
    ) -> Result<Self, ProbabilityDocumentError> {
        Ok(Self {
            bytes: write_long_run_probability_certificate(document, limits, cancel)?
                .into_boxed_slice(),
        })
    }
    /// Encode a weighted long-run certificate without accepting it.
    ///
    /// # Errors
    /// Strict certificate admission, caller limits or cancellation.
    pub fn from_weighted_long_run(
        document: &WeightedLongRunProbabilityDocument,
        limits: ProbabilityDocumentLimits,
        cancel: &Cancel,
    ) -> Result<Self, ProbabilityDocumentError> {
        Ok(Self {
            bytes: write_weighted_long_run_probability_certificate(document, limits, cancel)?
                .into_boxed_slice(),
        })
    }
    /// Encode a `lower` or `upper` certificate for an unwindowed `eventually`
    /// or expected-reward claim. The base objective is derived from the
    /// checked subject; the values and ranks are unchecked evidence.
    ///
    /// # Errors
    /// Subject admission, caller limits or cancellation.
    #[allow(clippy::too_many_arguments)]
    pub fn value_bound(
        system: &ModelSystem<'_>,
        claims: &[CheckedProbabilisticClaim],
        item: &ObligationIdentity,
        kind: SubsystemBoundKind,
        values: &[ExactProbabilityValue],
        ranking: Option<&[(ProbabilityProductKey, u64)]>,
        limits: ProbabilityDocumentLimits,
        cancel: &Cancel,
    ) -> Result<Self, ProbabilityDocumentError> {
        Ok(Self {
            bytes: super::document::refutation::write(
                system, claims, item, kind, values, ranking, limits, cancel,
            )?
            .into_boxed_slice(),
        })
    }
    /// Encode a typed finite-horizon measure certificate without accepting it.
    ///
    /// # Errors
    /// Strict certificate admission, caller limits or cancellation.
    pub fn from_document(
        document: &MeasureProbabilityDocument,
        limits: ProbabilityDocumentLimits,
        cancel: &Cancel,
    ) -> Result<Self, ProbabilityDocumentError> {
        Ok(Self {
            bytes: write_measure_probability_certificate(document, limits, cancel)?
                .into_boxed_slice(),
        })
    }
    /// Canonical certificate bytes, independently checked again by native replay.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// One scheduler choice at a product state.
#[derive(Clone, Debug, PartialEq)]
pub enum ProbabilisticSchedulerChoice {
    /// A deterministic product action.
    Deterministic(ProductAction),
    /// An exact-rational distribution over product actions.
    Randomized(Vec<(ProductAction, Rational)>),
}

pub(crate) fn deterministic_scheduler_rows(
    rows: &[(ProbabilityProductKey, ProbabilisticSchedulerChoice)],
) -> Result<Vec<(ProbabilityProductKey, ProductAction)>, ProbabilityWitnessError> {
    rows.iter()
        .map(|(key, choice)| match choice {
            ProbabilisticSchedulerChoice::Deterministic(action) => {
                Ok((key.clone(), action.clone()))
            }
            ProbabilisticSchedulerChoice::Randomized(_) => Err(dbg_refused(line!())),
        })
        .collect()
}

/// Typed path-set evidence for one exact checked claim and binding.
#[derive(Clone, Debug)]
pub struct ProbabilisticCounterexample {
    /// The checked claim occurrence, including its owning package identity.
    pub item: ObligationIdentity,
    /// The claim's object binding, absent for an unbound claim.
    pub binding: Option<ObjectReference>,
    /// The `on each` instance the evidence refutes, absent for any other claim.
    pub instance: Option<ObjectReference>,
    /// Actual initial state content.
    pub initial: ModelSimState,
    /// Complete initial product key.
    pub initial_key: ProbabilityProductKey,
    /// Ordered deterministic or randomized scheduler entries in full-key order.
    pub scheduler: Vec<(ProbabilityProductKey, ProbabilisticSchedulerChoice)>,
    /// Prefix-free finite paths.
    pub paths: Vec<ProbabilityWitnessPath>,
    /// A mean, long-run, expected-reward or event-avoidance subsystem
    /// certificate; mutually exclusive with paths.
    pub subsystem: Option<Box<ProbabilitySubsystemCertificate>>,
}

impl ReproducedProbabilityWitness {
    /// The reproduction of a path-set refutation whose mass the checker
    /// recomputed itself.
    pub(super) fn reproduced(evidence: ProbabilisticCounterexample, mass: Rational) -> Self {
        Self {
            evidence,
            mass,
            undefined: None,
        }
    }
}

/// A refutation reproduced by the qualified core. No caller can construct it.
///
/// ```compile_fail,E0451
/// use fsl_replay::{ReproducedProbabilityWitness, ProbabilisticCounterexample};
/// use filament_exact::Rational;
/// fn forge(evidence: ProbabilisticCounterexample, mass: Rational) {
///     let _ = ReproducedProbabilityWitness { evidence, mass };
/// }
/// ```
#[derive(Clone, Debug)]
pub struct ReproducedProbabilityWitness {
    evidence: ProbabilisticCounterexample,
    mass: Rational,
    undefined: Option<Box<ProbabilityUndefinedWitness>>,
}

impl ReproducedProbabilityWitness {
    /// The independently reproduced undefined evaluation, when this is an undefined path.
    pub fn undefined(&self) -> Option<&ProbabilityUndefinedWitness> {
        self.undefined.as_deref()
    }
    /// The independently recomputed path-set mass. A finite subsystem covers
    /// the complete probability space and reports mass one.
    pub fn mass(&self) -> &Rational {
        &self.mass
    }
    /// The evidence bound to this reproduction.
    pub fn evidence(&self) -> &ProbabilisticCounterexample {
        &self.evidence
    }
}

/// A completed replay either reproduces a refutation or disagrees with it.
#[derive(Clone, Debug)]
pub enum ProbabilityWitnessReplay {
    /// All semantic and threshold checks passed.
    Reproduced(Box<ReproducedProbabilityWitness>),
    /// The path set does not establish a strict refutation.
    Parity,
}

fn validate_randomized_entry(
    distribution: &[(ProductAction, Rational)],
    enabled: &[ProductAction],
    max_rational_bits: u64,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<(), ProbabilityWitnessError> {
    // One encoding per choice: a single certain choice is a deterministic entry.
    if distribution.len() < 2 {
        return Err(dbg_refused(line!()));
    }
    let mut total = rational(0);
    let mut seen = Vec::new();
    for (action, weight) in distribution {
        check_bits(weight, max_rational_bits)?;
        if weight <= &rational(0) || seen.contains(&action) || !enabled.contains(action) {
            return Err(dbg_refused(line!()));
        }
        seen.push(action);
        total = bounded::arithmetic(
            filament_exact::RationalArithmetic::Add(&total, weight),
            meter,
            cancel,
        )?;
        check_bits(&total, max_rational_bits)?;
    }
    if total != rational(1) {
        return Err(dbg_refused(line!()));
    }
    Ok(())
}

#[cfg(test)]
mod randomized_entry_tests {
    use super::*;

    fn q(n: i64, d: i64) -> Rational {
        Rational::new(n.into(), d.into()).expect("positive denominator")
    }

    const BITS: u64 = 64;

    fn check(
        distribution: &[(ProductAction, Rational)],
        enabled: &[ProductAction],
        max_bits: u64,
    ) -> Result<(), ProbabilityWitnessError> {
        let mut meter = filament_exact::Meter::new(crate::spine::default_accounting(1_000_000));
        validate_randomized_entry(distribution, enabled, max_bits, &mut meter, &Cancel::new())
    }

    fn both() -> Vec<ProductAction> {
        vec![ProductAction::TerminalStutter, ProductAction::UnitDelay]
    }

    fn refused(result: Result<(), ProbabilityWitnessError>) -> bool {
        matches!(result, Err(dbg_refused(line!())))
    }

    #[test]
    fn randomized_entry_accepts_enabled_positive_unit_distribution() {
        let distribution = vec![
            (ProductAction::TerminalStutter, q(1, 3)),
            (ProductAction::UnitDelay, q(2, 3)),
        ];
        assert!(check(&distribution, &both(), BITS).is_ok());
    }

    #[test]
    fn randomized_entry_rejects_empty_distribution() {
        assert!(refused(check(&[], &both(), BITS)));
    }

    #[test]
    fn randomized_entry_rejects_a_single_certain_choice() {
        let distribution = vec![(ProductAction::TerminalStutter, q(1, 1))];
        assert!(refused(check(&distribution, &both(), BITS)));
    }

    #[test]
    fn randomized_entry_rejects_duplicate_action() {
        let distribution = vec![
            (ProductAction::TerminalStutter, q(1, 2)),
            (ProductAction::TerminalStutter, q(1, 2)),
        ];
        assert!(refused(check(&distribution, &both(), BITS)));
    }

    #[test]
    fn randomized_entry_rejects_sum_above_one() {
        let distribution = vec![
            (ProductAction::TerminalStutter, q(2, 3)),
            (ProductAction::UnitDelay, q(2, 3)),
        ];
        assert!(refused(check(&distribution, &both(), BITS)));
    }

    #[test]
    fn randomized_entry_rejects_sum_below_one() {
        let distribution = vec![
            (ProductAction::TerminalStutter, q(1, 3)),
            (ProductAction::UnitDelay, q(1, 3)),
        ];
        assert!(refused(check(&distribution, &both(), BITS)));
    }

    #[test]
    fn randomized_entry_rejects_non_enabled_action() {
        let distribution = vec![
            (ProductAction::TerminalStutter, q(1, 2)),
            (ProductAction::UnitDelay, q(1, 2)),
        ];
        assert!(refused(check(
            &distribution,
            &[ProductAction::TerminalStutter],
            BITS
        )));
    }

    #[test]
    fn randomized_entry_rejects_zero_weight() {
        let distribution = vec![
            (ProductAction::TerminalStutter, q(0, 1)),
            (ProductAction::UnitDelay, q(1, 1)),
        ];
        assert!(refused(check(&distribution, &both(), BITS)));
    }

    #[test]
    fn randomized_entry_rejects_negative_weight() {
        let distribution = vec![
            (ProductAction::TerminalStutter, q(-1, 2)),
            (ProductAction::UnitDelay, q(3, 2)),
        ];
        assert!(refused(check(&distribution, &both(), BITS)));
    }

    #[test]
    fn randomized_entry_refuses_an_over_limit_weight_by_the_bit_limit() {
        let huge = Rational::new(
            filament_exact::Integer::from(1_i64),
            filament_exact::Integer::from(10_i128.pow(30)),
        )
        .expect("positive denominator");
        let distribution = vec![
            (ProductAction::TerminalStutter, huge),
            (ProductAction::UnitDelay, q(1, 2)),
        ];
        assert!(matches!(
            check(&distribution, &both(), BITS),
            Err(ProbabilityWitnessError::Limit {
                limit: ModelCheckLimit::ExactProbMaxRationalBits,
                ..
            })
        ));
    }
}

/// A replay could not complete; none of these outcomes contains a token.
#[derive(Clone, Debug)]
pub enum ProbabilityWitnessError {
    /// A subject expansion stopped with its original accounting or failure cause.
    Build(Box<crate::ExactProbBuildIssue>),
    /// Model admission, graph enumeration, unsupported shape or cancellation.
    Graph(ExactProbabilityGraphError),
    /// The witness names an invalid semantic choice, state or claim binding.
    Refused,
    /// The independent subsystem checker rejected a certificate condition.
    CertificateRejected(ProbabilityCertificateRejection),
    /// A witness storage or arithmetic budget stopped replay.
    Limit {
        /// The exhausted caller limit.
        limit: ModelCheckLimit,
        /// Its configured bound.
        value: u64,
    },
}

impl From<ExactProbabilityGraphError> for ProbabilityWitnessError {
    fn from(value: ExactProbabilityGraphError) -> Self {
        Self::Graph(value)
    }
}

impl From<fsl_eval::automaton::QualifiedBoundedError> for ProbabilityWitnessError {
    fn from(error: fsl_eval::automaton::QualifiedBoundedError) -> Self {
        bounded::monitor_error(error)
    }
}

/// Observation points of actual semantic work, for cancellation regression tests.
#[cfg(feature = "test-support")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilityWitnessSemanticPhase {
    /// The original graph is about to call the semantic operation/workload owner.
    ExpansionEntered,
    /// That semantic call returned without observing cancellation.
    ExpansionReturned,
    /// A direct state predicate is about to enter its qualified evaluator.
    PredicateEntered,
    /// That predicate call returned without observing cancellation.
    PredicateReturned,
    /// Independent path replay is entering its actual operation/workload call.
    StepEntered,
    /// That path-step call returned without observing cancellation.
    StepReturned,
}

/// Replay a typed path set against the exact package and admitted system.
/// The claim, binding, predicates, probabilities and terminal leaves are all
/// independently derived. This entry accepts no analyzer graph or wire data.
/// It admits acyclic unbounded reachability, direct infinite-trace `until`
/// and `always` state events, and origin event-position
/// `eventually[0,h] holds(P)`. The latter uses qualified monitor keys and
/// terminal false extension and permits raw model cycles within its horizon.
#[allow(clippy::too_many_arguments)]
pub fn replay_probabilistic_witness(
    package: &CheckedPackage,
    system: &ModelSystem<'_>,
    item: &ObligationIdentity,
    witness: &ProbabilisticCounterexample,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    authenticate_source(package, system, cancel)?;
    replay_probabilistic_witness_impl(
        package.graph().probabilistic_model(),
        system,
        item,
        witness,
        limits,
        cancel,
        #[cfg(feature = "test-support")]
        &mut |_| {},
    )
}

/// Replay through the ordinary semantic path with a test-only phase observer.
#[cfg(feature = "test-support")]
#[allow(clippy::too_many_arguments)]
pub fn replay_probabilistic_witness_observing(
    package: &CheckedPackage,
    system: &ModelSystem<'_>,
    item: &ObligationIdentity,
    witness: &ProbabilisticCounterexample,
    limits: &ExactProbLimits,
    cancel: &Cancel,
    observe: &mut impl FnMut(ProbabilityWitnessSemanticPhase),
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    authenticate_source(package, system, cancel)?;
    replay_probabilistic_witness_impl(
        package.graph().probabilistic_model(),
        system,
        item,
        witness,
        limits,
        cancel,
        observe,
    )
}

fn authenticate_source(
    package: &CheckedPackage,
    system: &ModelSystem<'_>,
    cancel: &Cancel,
) -> Result<(), ProbabilityWitnessError> {
    poll_cancel(cancel)?;
    if !system
        .source_package()
        .is_some_and(|own| std::ptr::eq(package, own))
    {
        return Err(dbg_refused(line!()));
    }
    Ok(())
}

/// Replay a negative witness through the exact bare owner of the admitted
/// system. All execution tables and the evaluator must still belong to this
/// owner. Claims bind by their complete package occurrence and ordinal;
/// predicates and workloads are read through that same owner's tables.
///
/// # Errors
/// Refuses a foreign or mixed owner and otherwise preserves the source
/// entry's semantic, cancellation, resource and parity results.
#[allow(clippy::too_many_arguments)]
pub fn replay_probabilistic_witness_from_package(
    owner: &PackageModelInput<'_>,
    system: &ModelSystem<'_>,
    item: &ObligationIdentity,
    witness: &ProbabilisticCounterexample,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    poll_cancel(cancel)?;
    if !system
        .input()
        .package_owner()
        .is_some_and(|actual| std::ptr::eq(owner, actual))
    {
        return Err(dbg_refused(line!()));
    }
    replay_probabilistic_witness_impl(
        system.input().probabilistic,
        system,
        item,
        witness,
        limits,
        cancel,
        #[cfg(feature = "test-support")]
        &mut |_| {},
    )
}

#[allow(clippy::too_many_arguments)]
fn replay_probabilistic_witness_impl(
    probabilistic: &fsl_semantics::check::CheckedProbabilisticModel,
    system: &ModelSystem<'_>,
    item: &ObligationIdentity,
    witness: &ProbabilisticCounterexample,
    limits: &ExactProbLimits,
    cancel: &Cancel,
    #[cfg(feature = "test-support")] observe: &mut dyn FnMut(ProbabilityWitnessSemanticPhase),
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    poll_cancel(cancel)?;
    if item != &witness.item {
        return Err(dbg_refused(line!()));
    }
    let claim = bind_claim(probabilistic.claims(), item, cancel).map_err(|error| match error {
        ExactProbabilityGraphError::ClaimNotBound => ProbabilityWitnessError::Refused,
        _ => ProbabilityWitnessError::Graph(error),
    })?;
    if witness.subsystem.is_some() {
        return if matches!(claim.form, ProbForm::MeanOfFraction { .. }) {
            measure::replay_subsystem(system, claim, witness, limits, cancel)
        } else {
            subsystem::replay(system, claim, witness, limits, cancel)
        };
    }
    if super::document::is_timed_witness_claim(system, claim) {
        return super::document::replay_timed_witness(system, claim, witness, limits, cancel);
    }
    if claim.bounded_measure() {
        return measure::replay(system, claim, witness, limits, cancel);
    }
    if matches!(claim.form, ProbForm::Probability { .. }) {
        return bounded::replay(system, claim, witness, limits, cancel);
    }
    let ProbForm::Reach { bound, target } = &claim.form else {
        return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into());
    };
    let fair_unbounded_eventual = !claim.fairness.is_empty()
        && matches!(target, ReachTarget::Eventually(_))
        && fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
            == Some(fsl_semantics::check::TemporalProfile::InfiniteTrace);
    if !matches!(target, ReachTarget::Eventually(_))
        && fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
            != Some(fsl_semantics::check::TemporalProfile::InfiniteTrace)
        || !claim.fairness.is_empty() && !fair_unbounded_eventual
    {
        return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into());
    }
    match bound {
        Bound::AtLeast(threshold) | Bound::AtMost(threshold) => {
            check_bits(threshold, limits.max_rational_bits)?
        }
    }
    if !matches!(
        TimedSystem::new(system, &claim.subject.alias),
        Err(TimedSubjectRefusal::NoTimedModel { .. })
    ) {
        return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into());
    }
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|binding| &binding.reference);
    if reference != witness.binding.as_ref() {
        return Err(dbg_refused(line!()));
    }
    let mut model_states = BTreeMap::new();
    let graph_storage = std::cell::RefCell::new(0u64);
    let mut meter = filament_exact::Meter::new(system.meter_limits()).with_cancel(cancel.clone());
    let key_error = std::cell::RefCell::new(None);
    let mut issue = None;
    let graph = enumerate_model_graph_with_keys_and_reservation(
        system,
        None::<&fsl_semantics::check::CheckedPredicate>,
        reference,
        None,
        limits.model.limits.max_states,
        limits.model.limits.max_transitions,
        cancel,
        false,
        &|state| {
            key_of(system, reference, state, limits, cancel).map_err(|error| {
                *key_error.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })
        },
        &mut |bytes| {
            bounded::retain(bytes, &mut graph_storage.borrow_mut(), limits).map_err(|error| {
                *key_error.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })
        },
        #[cfg(feature = "test-support")]
        observe,
        &mut |key, state| {
            bounded::visit(&mut meter, cancel).map_err(|error| {
                *key_error.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })?;
            bounded::retain(
                key.as_bytes().len().saturating_mul(2),
                &mut graph_storage.borrow_mut(),
                limits,
            )
            .map_err(|error| {
                *key_error.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })?;
            model_states.insert(key.clone(), state.clone());
            Ok(())
        },
        &mut |failure| {
            issue = Some(failure);
        },
    )
    .map_err(|error| {
        if let Some(error) = key_error.borrow_mut().take() {
            return error;
        }
        issue.map_or(ProbabilityWitnessError::Graph(error), |issue| {
            ProbabilityWitnessError::Build(Box::new(issue))
        })
    })?;
    let mut storage = graph_storage.into_inner();
    // Kahn's checker retains an indegree map, ready set, and output keys.
    for key in graph.states.keys() {
        poll_cancel(cancel)?;
        bounded::retain(
            key.as_bytes()
                .len()
                .saturating_mul(3)
                .saturating_add(std::mem::size_of::<ProbabilityProductKey>().saturating_mul(3)),
            &mut storage,
            limits,
        )?;
    }
    // A fair unbounded eventual witness may traverse a cyclic model graph;
    // the independently rebuilt bottom components below establish fairness.
    if !fair_unbounded_eventual {
        topological_order(&graph, cancel)?;
    }
    let initials = system.initial();
    let [initial] = initials.as_slice() else {
        return Err(ExactProbabilityGraphError::UnsupportedInitial.into());
    };
    if key_of(system, reference, initial, limits, cancel)? != witness.initial_key
        || key_of(system, reference, &witness.initial, limits, cancel)? != witness.initial_key
        || initial.anchor() != witness.initial.anchor()
        || initial.reached_by() != witness.initial.reached_by()
    {
        return Err(dbg_refused(line!()));
    }
    if u64::try_from(witness.paths.len()).unwrap_or(u64::MAX) > limits.max_witness_paths {
        return Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::ExactProbMaxWitnessPaths,
            value: limits.max_witness_paths,
        });
    }
    if witness.scheduler.len() > limits.model.limits.max_states {
        return Err(ExactProbabilityGraphError::LimitReached.into());
    }
    let workload = match &claim.scheduler {
        ProbScheduler::Every => None,
        ProbScheduler::Workload(name) => Some(
            probabilistic
                .workload(name)
                .ok_or(dbg_refused(line!()))?,
        ),
    };
    if workload.is_some() && !witness.scheduler.is_empty() {
        return Err(dbg_refused(line!()));
    }
    let mut scheduler = BTreeMap::new();
    let mut previous = None;
    for (key, entry) in &witness.scheduler {
        poll_cancel(cancel)?;
        if previous.is_some_and(|previous| previous >= key) || !graph.states.contains_key(key) {
            return Err(dbg_refused(line!()));
        }
        previous = Some(key);
        let state = model_states
            .get(key)
            .ok_or(dbg_refused(line!()))?;
        let enabled = system
            .actions_with_cancel(state, cancel)
            .map_err(|stop| cancelled_expansion(stop, cancel))?
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
        // An identity with no schedule is not an enabled choice; it never refuses the entry itself.
        let enabled_actions: Vec<ProductAction> = enabled
            .iter()
            .filter_map(|candidate| crate::Schedule::of(system, &candidate.identity).ok())
            .map(ProductAction::Identity)
            .collect();
        match entry {
            ProbabilisticSchedulerChoice::Deterministic(action) => {
                if !enabled_actions.contains(action) {
                    return Err(dbg_refused(line!()));
                }
                let bytes = action
                    .to_bytes()
                    .map_err(|_| dbg_refused(line!()))?;
                bounded::retain(
                    key.as_bytes().len().saturating_add(bytes.len()),
                    &mut storage,
                    limits,
                )?;
            }
            ProbabilisticSchedulerChoice::Randomized(distribution) => {
                // Randomized policies belong to claims with a fairness set (FR-202).
                if claim.fairness.is_empty() {
                    return Err(dbg_refused(line!()));
                }
                validate_randomized_entry(
                    distribution,
                    &enabled_actions,
                    limits.max_rational_bits,
                    &mut meter,
                    cancel,
                )?;
                for (action, _) in distribution {
                    let bytes = action
                        .to_bytes()
                        .map_err(|_| dbg_refused(line!()))?;
                    bounded::retain(
                        key.as_bytes().len().saturating_add(bytes.len()),
                        &mut storage,
                        limits,
                    )?;
                }
            }
        }
        scheduler.insert(key, entry);
    }
    if !claim.fairness.is_empty()
        && !witness_is_fair(
            system,
            claim,
            target,
            reference,
            &graph,
            &model_states,
            &scheduler,
            &witness.initial_key,
            limits,
            cancel,
            &mut meter,
            &mut storage,
        )?
    {
        // FR-202: an unfair witness settles `inconclusive`, `replay-parity`.
        return Ok(ProbabilityWitnessReplay::Parity);
    }
    let mut transitions = 0usize;
    let mut mass = rational(0);
    let mut replayed: Vec<&ProbabilityWitnessPath> = Vec::new();
    for path in &witness.paths {
        poll_cancel(cancel)?;
        let mut state = initial.clone();
        let mut probability = rational(1);

        for step in &path.steps {
            poll_cancel(cancel)?;
            transitions = transitions
                .checked_add(1)
                .ok_or(ExactProbabilityGraphError::TooManyEdges)?;
            if transitions > limits.model.limits.max_transitions {
                return Err(ExactProbabilityGraphError::EdgeLimitReached.into());
            }
            let key = key_of(system, reference, &state, limits, cancel)?;
            let terminal = graph
                .states
                .get(&key)
                .ok_or(dbg_refused(line!()))?
                .choices
                .is_empty();
            if outcome(
                system,
                target,
                reference,
                &state,
                terminal,
                cancel,
                #[cfg(feature = "test-support")]
                observe,
            )?
            .is_some()
            {
                return Ok(ProbabilityWitnessReplay::Parity);
            }
            let ProductAction::Identity(schedule) = &step.choice else {
                return Err(dbg_refused(line!()));
            };
            let (Some(step_action), Some(step_transition), Some(step_draw)) =
                (&step.action, &step.transition, &step.draw)
            else {
                return Err(dbg_refused(line!()));
            };
            let scheduler_weight = match scheduler.get(&key) {
                Some(ProbabilisticSchedulerChoice::Deterministic(action))
                    if action == &step.choice =>
                {
                    Some(rational(1))
                }
                Some(ProbabilisticSchedulerChoice::Randomized(distribution)) => distribution
                    .iter()
                    .find(|(action, _)| action == &step.choice)
                    .map(|(_, weight)| weight.clone()),
                Some(_) => None,
                None if workload.is_some() => Some(rational(1)),
                None => None,
            };
            if step.elapsed != rational(0)
                || crate::Schedule::of(system, step_action).as_ref() != Ok(schedule)
                || scheduler_weight.is_none()
            {
                return Err(dbg_refused(line!()));
            }
            let mut found = None;
            if let Some(workload) = workload {
                #[cfg(feature = "test-support")]
                observe(ProbabilityWitnessSemanticPhase::StepEntered);
                let candidates = system.weighted_steps_with_cancel(&state, workload, cancel);
                #[cfg(feature = "test-support")]
                if cancel.tripped().is_none() {
                    observe(ProbabilityWitnessSemanticPhase::StepReturned);
                }
                for candidate in candidates
                    .map_err(|stop| cancelled_expansion(stop, cancel))?
                    .map_err(|_| ExactProbabilityGraphError::UnsupportedDraw)?
                {
                    poll_cancel(cancel)?;
                    if candidate.identity.as_ref() == Some(step_action)
                        && candidate.transition == TransitionId::Operation(step_transition.clone())
                        && same_values(&candidate.random, step_draw)
                        && key_of(system, reference, &candidate.post, limits, cancel)?
                            == step.post_key
                    {
                        found = Some((candidate.probability, candidate.post));
                        break;
                    }
                }
            } else {
                #[cfg(feature = "test-support")]
                observe(ProbabilityWitnessSemanticPhase::StepEntered);
                let actions = system.actions_with_cancel(&state, cancel);
                #[cfg(feature = "test-support")]
                if cancel.tripped().is_none() {
                    observe(ProbabilityWitnessSemanticPhase::StepReturned);
                }
                for action in actions
                    .map_err(|stop| cancelled_expansion(stop, cancel))?
                    .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?
                {
                    poll_cancel(cancel)?;
                    if action.identity != *step_action {
                        continue;
                    }
                    for draw in action.draws {
                        poll_cancel(cancel)?;
                        if !same_values(&draw.random, step_draw) {
                            continue;
                        }
                        if draw.successors.len() != 1 {
                            return Err(ExactProbabilityGraphError::UnsupportedDraw.into());
                        }
                        for (transition, post) in draw.successors {
                            if transition == *step_transition
                                && key_of(system, reference, &post, limits, cancel)?
                                    == step.post_key
                            {
                                found = Some((draw.probability.clone(), post));
                            }
                        }
                    }
                }
            }
            let Some((step_mass, post)) = found else {
                return Err(dbg_refused(line!()));
            };
            if step_mass <= rational(0)
                || key_of(system, reference, &step.post, limits, cancel)? != step.post_key
                || post.anchor() != step.post.anchor()
                || post.reached_by() != step.post.reached_by()
            {
                return Err(dbg_refused(line!()));
            }
            check_bits(&step_mass, limits.max_rational_bits)?;
            {
                let weight = scheduler_weight
                    .as_ref()
                    .ok_or(dbg_refused(line!()))?;
                probability = bounded::arithmetic(
                    filament_exact::RationalArithmetic::Multiply(&probability, weight),
                    &mut meter,
                    cancel,
                )?;
                check_bits(&probability, limits.max_rational_bits)?;
            }
            probability = bounded::arithmetic(
                filament_exact::RationalArithmetic::Multiply(&probability, &step_mass),
                &mut meter,
                cancel,
            )?;
            check_bits(&probability, limits.max_rational_bits)?;

            state = post;
        }
        let key = key_of(system, reference, &state, limits, cancel)?;
        let terminal = graph
            .states
            .get(&key)
            .ok_or(dbg_refused(line!()))?
            .choices
            .is_empty();
        let accepting = matches!(
            (
                bound,
                outcome(
                    system,
                    target,
                    reference,
                    &state,
                    terminal,
                    cancel,
                    #[cfg(feature = "test-support")]
                    observe
                )?
            ),
            (Bound::AtMost(_), Some(true)) | (Bound::AtLeast(_), Some(false))
        );

        if !accepting {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
        for previous in &replayed {
            poll_cancel(cancel)?;
            let mut prefix = true;
            for (a, b) in previous.steps.iter().zip(&path.steps) {
                bounded::visit(&mut meter, cancel)?;
                if a.action != b.action
                    || a.transition != b.transition
                    || a.post_key != b.post_key
                    || !same_draw(&a.draw, &b.draw)
                {
                    prefix = false;
                    break;
                }
            }
            if prefix {
                return Ok(ProbabilityWitnessReplay::Parity);
            }
        }
        bounded::retain(
            std::mem::size_of::<&ProbabilityWitnessPath>(),
            &mut storage,
            limits,
        )?;
        replayed
            .try_reserve(1)
            .map_err(|_| ProbabilityWitnessError::Limit {
                limit: ModelCheckLimit::CertificateMaxWorkingBytes,
                value: limits.document.storage_bytes,
            })?;
        replayed.push(path);
        mass = bounded::arithmetic(
            filament_exact::RationalArithmetic::Add(&mass, &probability),
            &mut meter,
            cancel,
        )?;
        check_bits(&mass, limits.max_rational_bits)?;
    }
    let threshold = match bound {
        Bound::AtMost(threshold) => threshold.clone(),
        Bound::AtLeast(threshold) => bounded::arithmetic(
            filament_exact::RationalArithmetic::Subtract(&rational(1), threshold),
            &mut meter,
            cancel,
        )?,
    };
    if mass <= threshold {
        return Ok(ProbabilityWitnessReplay::Parity);
    }
    let evidence = witness.clone();
    poll_cancel(cancel)?;
    Ok(ProbabilityWitnessReplay::Reproduced(Box::new(
        ReproducedProbabilityWitness {
            evidence,
            mass,
            undefined: None,
        },
    )))
}

#[cfg(test)]
mod fair_component_tests {
    use super::*;
    use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};
    use ix_trace_rs::trace;

    type Node = FairNode<u8>;

    fn node(enabled: &[(u8, &str)], support: &[u8], successors: &[usize], escapes: bool) -> Node {
        FairNode {
            enabled: enabled
                .iter()
                .map(|(id, op)| (*id, (*op).to_owned()))
                .collect(),
            support: support.to_vec(),
            successors: successors.to_vec(),
            escapes,
        }
    }

    fn fair(
        nodes: &[Node],
        kind: FairnessKind,
        granularity: FairnessGranularity,
        operation: &str,
    ) -> bool {
        bottom_components_are_fair(
            nodes,
            &[(kind, granularity, operation.to_owned())],
            &mut || Ok(()),
        )
        .expect("the fairness walk completes")
    }

    /// A two-state bottom component that cycles on `a` while `b` stays enabled
    /// at both states and is never taken: weak fairness to `b` fails.
    fn a_loop_ignoring_b() -> Vec<Node> {
        vec![
            node(&[(1, "a"), (2, "b")], &[1], &[1], false),
            node(&[(1, "a"), (2, "b")], &[1], &[0], false),
        ]
    }

    #[trace("TC-364", "FR-413-AC-5", "FR-202-AC-3")]
    #[test]
    fn an_unfair_nontrivial_bottom_component_is_not_fair() {
        let nodes = a_loop_ignoring_b();
        assert!(!fair(
            &nodes,
            FairnessKind::Weak,
            FairnessGranularity::Whole,
            "b"
        ));
        assert!(!fair(
            &nodes,
            FairnessKind::Strong,
            FairnessGranularity::Whole,
            "b"
        ));
        assert!(!fair(
            &nodes,
            FairnessKind::Weak,
            FairnessGranularity::Each,
            "b"
        ));
    }

    #[trace("TC-364", "FR-413-AC-5", "FR-200-AC-4")]
    #[test]
    fn a_component_that_takes_the_operation_or_cannot_enable_it_is_fair() {
        let nodes = a_loop_ignoring_b();
        // `a` is taken inside the component.
        assert!(fair(
            &nodes,
            FairnessKind::Weak,
            FairnessGranularity::Whole,
            "a"
        ));
        // An operation enabled nowhere is vacuously fair.
        assert!(fair(
            &nodes,
            FairnessKind::Strong,
            FairnessGranularity::Whole,
            "c"
        ));
        // Weak fairness needs enabledness at every member: `b` disabled at one
        // member of the component leaves a weak constraint satisfied.
        let partly = vec![
            node(&[(1, "a"), (2, "b")], &[1], &[1], false),
            node(&[(1, "a")], &[1], &[0], false),
        ];
        assert!(fair(
            &partly,
            FairnessKind::Weak,
            FairnessGranularity::Whole,
            "b"
        ));
        assert!(!fair(
            &partly,
            FairnessKind::Strong,
            FairnessGranularity::Whole,
            "b"
        ));
    }

    #[trace("TC-364", "FR-413-AC-5")]
    #[test]
    fn a_component_with_an_exit_is_not_bottom_and_is_not_checked() {
        let mut nodes = a_loop_ignoring_b();
        nodes[1].escapes = true;
        assert!(fair(
            &nodes,
            FairnessKind::Weak,
            FairnessGranularity::Whole,
            "b"
        ));
    }

    #[trace("TC-364", "FR-413-AC-5")]
    #[test]
    fn each_granularity_checks_every_enabled_identity() {
        // Identity 2 of `b` is taken, identity 3 of `b` is enabled and never taken.
        let nodes = vec![
            node(&[(1, "a"), (2, "b"), (3, "b")], &[2], &[1], false),
            node(&[(1, "a"), (2, "b"), (3, "b")], &[2], &[0], false),
        ];
        assert!(fair(
            &nodes,
            FairnessKind::Weak,
            FairnessGranularity::Whole,
            "b"
        ));
        assert!(!fair(
            &nodes,
            FairnessKind::Weak,
            FairnessGranularity::Each,
            "b"
        ));
    }
}

/// One state of the chain a randomized witness induces on its evidence.
struct FairNode<S = crate::Schedule> {
    /// Enabled scheduled identities with their operation text.
    enabled: Vec<(S, String)>,
    /// The identities the entry gives positive probability.
    support: Vec<S>,
    /// Successor nodes.
    successors: Vec<usize>,
    /// Whether a positive-probability step leaves the evidence's states.
    escapes: bool,
}

/// FS-9 / FR-202: every bottom strongly connected component of the chain the
/// witness scheduler induces on the evidence's states meets FR-406's
/// fair-end-component condition. A reached state with no entry, or a
/// scheduler outside the enabled identities, is not a fair witness.
#[allow(clippy::too_many_arguments)]
fn witness_is_fair(
    system: &ModelSystem<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    target: &ReachTarget,
    reference: Option<&ObjectReference>,
    graph: &CheckerGraph,
    model_states: &BTreeMap<ProbabilityProductKey, ModelSimState>,
    scheduler: &BTreeMap<&ProbabilityProductKey, &ProbabilisticSchedulerChoice>,
    initial_key: &ProbabilityProductKey,
    limits: &ExactProbLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<bool, ProbabilityWitnessError> {
    let is_leaf = |key: &ProbabilityProductKey| -> Result<bool, ProbabilityWitnessError> {
        let terminal = graph
            .states
            .get(key)
            .ok_or(dbg_refused(line!()))?
            .choices
            .is_empty();
        let state = model_states
            .get(key)
            .ok_or(dbg_refused(line!()))?;
        Ok(outcome(
            system,
            target,
            reference,
            state,
            terminal,
            cancel,
            #[cfg(feature = "test-support")]
            &mut |_| {},
        )?
        .is_some())
    };
    if is_leaf(initial_key)? {
        return Ok(true);
    }
    let mut index: BTreeMap<ProbabilityProductKey, usize> = BTreeMap::new();
    let mut nodes: Vec<FairNode> = Vec::new();
    let mut queue = std::collections::VecDeque::new();
    index.insert(initial_key.clone(), 0);
    queue.push_back(initial_key.clone());
    while let Some(key) = queue.pop_front() {
        poll_cancel(cancel)?;
        bounded::visit(meter, cancel)?;
        bounded::retain(
            key.as_bytes()
                .len()
                .saturating_add(std::mem::size_of::<FairNode>()),
            storage,
            limits,
        )?;
        let Some(entry) = scheduler.get(&key) else {
            return Ok(false);
        };
        let state = model_states
            .get(&key)
            .ok_or(dbg_refused(line!()))?;
        let actions = system
            .actions_with_cancel(state, cancel)
            .map_err(|stop| cancelled_expansion(stop, cancel))?
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
        let support: Vec<crate::Schedule> = match entry {
            ProbabilisticSchedulerChoice::Deterministic(ProductAction::Identity(schedule)) => {
                vec![schedule.clone()]
            }
            ProbabilisticSchedulerChoice::Randomized(distribution) => distribution
                .iter()
                .filter_map(|(action, _)| match action {
                    ProductAction::Identity(schedule) => Some(schedule.clone()),
                    _ => None,
                })
                .collect(),
            ProbabilisticSchedulerChoice::Deterministic(_) => return Ok(false),
        };
        let mut node = FairNode {
            enabled: Vec::new(),
            support: support.clone(),
            successors: Vec::new(),
            escapes: false,
        };
        for action in actions {
            poll_cancel(cancel)?;
            let Ok(schedule) = crate::Schedule::of(system, &action.identity) else {
                continue;
            };
            node.enabled
                .push((schedule.clone(), action.identity.operation.clone()));
            if !support.contains(&schedule) {
                continue;
            }
            for draw in action.draws {
                if draw.probability == rational(0) {
                    continue;
                }
                // A draw with several post-states needs the witness's PostState
                // choice (ADR-028 WS-1), which this replay does not read: refuse
                // rather than over-approximate the chain.
                if draw.successors.len() != 1 {
                    return Err(ExactProbabilityGraphError::UnsupportedDraw.into());
                }
                for (_, post) in draw.successors {
                    let post_key = key_of(system, reference, &post, limits, cancel)?;
                    if is_leaf(&post_key)? {
                        node.escapes = true;
                        continue;
                    }
                    let next = match index.get(&post_key) {
                        Some(next) => *next,
                        None => {
                            let next = index.len();
                            index.insert(post_key.clone(), next);
                            queue.push_back(post_key);
                            next
                        }
                    };
                    if !node.successors.contains(&next) {
                        node.successors.push(next);
                    }
                }
            }
        }
        // Nodes are numbered in discovery order, which is queue order.
        nodes.push(node);
    }
    let mut constraints = Vec::with_capacity(claim.fairness.len());
    for constraint in &claim.fairness {
        let text = system
            .operation_text(
                constraint.operation().declaring_type(),
                constraint.operation().name(),
            )
            .ok_or(dbg_refused(line!()))?;
        constraints.push((constraint.kind(), constraint.granularity(), text.to_owned()));
    }
    bottom_components_are_fair(&nodes, &constraints, &mut || {
        poll_cancel(cancel)?;
        bounded::visit(meter, cancel)
    })
}

/// FR-406's condition on every bottom component of the induced chain: a
/// weak constraint enabled at every member, or a strong one enabled at some
/// member, is taken inside it (per identity for an each constraint).
fn bottom_components_are_fair<S: PartialEq>(
    nodes: &[FairNode<S>],
    constraints: &[(
        filament_semantic_value::fairness::FairnessKind,
        filament_semantic_value::fairness::FairnessGranularity,
        String,
    )],
    tick: &mut impl FnMut() -> Result<(), ProbabilityWitnessError>,
) -> Result<bool, ProbabilityWitnessError> {
    use filament_semantic_value::fairness::{FairnessGranularity, FairnessKind};
    let count = nodes.len();
    let edges: Vec<Vec<usize>> = nodes.iter().map(|node| node.successors.clone()).collect();
    let mut reverse = vec![Vec::new(); count];
    for (from, successors) in edges.iter().enumerate() {
        for to in successors {
            reverse[*to].push(from);
        }
    }
    let mut stopped = None;
    let bottom = bottom_components(&edges, &reverse, &mut |_| {
        tick().map_err(|error| {
            stopped = Some(error);
            ProbabilityDocumentError::Malformed
        })
    });
    if let Some(error) = stopped {
        return Err(error);
    }
    let (_, components, closed) = bottom.map_err(|_| dbg_refused(line!()))?;
    for (component, members) in components.iter().enumerate() {
        tick()?;
        if !closed[component] || members.iter().any(|member| nodes[*member].escapes) {
            continue;
        }
        for (kind, granularity, text) in constraints {
            let operation = |schedule: &(S, String)| schedule.1 == *text;
            let identities: Vec<Option<&S>> = if *granularity == FairnessGranularity::Whole {
                vec![None]
            } else {
                let mut seen: Vec<&S> = Vec::new();
                for member in members {
                    for enabled in nodes[*member].enabled.iter().filter(|e| operation(e)) {
                        if !seen.contains(&&enabled.0) {
                            seen.push(&enabled.0);
                        }
                    }
                }
                seen.into_iter().map(Some).collect()
            };
            for identity in identities {
                let enabled_at = |node: &FairNode<S>| {
                    node.enabled.iter().any(|enabled| {
                        operation(enabled) && identity.is_none_or(|identity| identity == &enabled.0)
                    })
                };
                let taken = members.iter().any(|member| {
                    let node = &nodes[*member];
                    node.support.iter().any(|schedule| {
                        node.enabled.iter().any(|enabled| {
                            &enabled.0 == schedule
                                && operation(enabled)
                                && identity.is_none_or(|identity| identity == schedule)
                        })
                    })
                });
                if taken {
                    continue;
                }
                let unfair = match kind {
                    FairnessKind::Weak => members.iter().all(|member| enabled_at(&nodes[*member])),
                    FairnessKind::Strong => {
                        members.iter().any(|member| enabled_at(&nodes[*member]))
                    }
                };
                if unfair {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

/// First decisive state under infinite stutter at a genuine terminal state.
fn outcome(
    system: &ModelSystem<'_>,
    target: &ReachTarget,
    binding: Option<&ObjectReference>,
    state: &ModelSimState,
    terminal: bool,
    cancel: &Cancel,
    #[cfg(feature = "test-support")] observe: &mut dyn FnMut(ProbabilityWitnessSemanticPhase),
) -> Result<Option<bool>, ProbabilityWitnessError> {
    if let ReachTarget::Eventually(predicate) = target {
        if observes(system, predicate, binding, state, cancel)? {
            return Ok(Some(true));
        }
    }
    match target {
        ReachTarget::Eventually(_) => Ok(terminal.then_some(false)),
        ReachTarget::Until(..) | ReachTarget::Always(_) => state_predicate::outcome(
            system,
            target,
            binding,
            state,
            terminal,
            cancel,
            #[cfg(feature = "test-support")]
            observe,
        )
        .map(|(verdict, _)| verdict)
        // A counterexample replay keeps its refusal for an undefined reading.
        .map_err(|error| match error {
            ProbabilityDocumentError::UndefinedPredicate(_) => {
                ProbabilityDocumentError::Graph(ExactProbabilityGraphError::ExpansionRefused)
            }
            other => other,
        })
        .map_err(bounded::document_error),
    }
}

fn key_of(
    system: &ModelSystem<'_>,
    binding: Option<&ObjectReference>,
    state: &ModelSimState,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityProductKey, ProbabilityWitnessError> {
    let (bytes, _) = system
        .state_key(state, fsl_foundation::IdentityLimits::default())
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    ProbabilityProductKey::for_model_state_with_limits(binding, &bytes, limits.document, cancel)
        .map_err(bounded::document_error)
}
fn observes(
    system: &ModelSystem<'_>,
    predicate: &impl StatePredicate,
    binding: Option<&ObjectReference>,
    state: &ModelSimState,
    cancel: &Cancel,
) -> Result<bool, ProbabilityWitnessError> {
    let readings = system.state_predicates_with_cancel(state, &[(predicate, binding)], cancel);
    poll_cancel(cancel)?;
    match readings.map_err(expansion)?.first() {
        Some(PredicateReading::True) => Ok(true),
        Some(PredicateReading::False) => Ok(false),
        Some(PredicateReading::Refused(outcome)) => Err(ProbabilityWitnessError::Build(Box::new(
            crate::ExactProbBuildIssue::Predicate(outcome.clone()),
        ))),
        _ => Err(ExactProbabilityGraphError::ExpansionRefused.into()),
    }
}
fn cancelled_expansion(
    stop: fsl_eval::simulation::ExpansionStop,
    cancel: &Cancel,
) -> ProbabilityWitnessError {
    match poll_cancel(cancel) {
        Err(error) => error.into(),
        Ok(()) => expansion(stop),
    }
}
fn expansion(stop: fsl_eval::simulation::ExpansionStop) -> ProbabilityWitnessError {
    ProbabilityWitnessError::Build(Box::new(crate::ExactProbBuildIssue::Expansion(stop)))
}
fn check_bits(value: &Rational, max: u64) -> Result<(), ProbabilityWitnessError> {
    if value.numerator().as_big().bits() > max || value.denominator().as_big().bits() > max {
        Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::ExactProbMaxRationalBits,
            value: max,
        })
    } else {
        Ok(())
    }
}

impl From<ProbabilityDocumentError> for ProbabilityWitnessError {
    fn from(error: ProbabilityDocumentError) -> Self {
        match error {
            ProbabilityDocumentError::Limit { limit, bound, .. } => Self::Limit {
                limit: limit.model_limit(),
                value: bound,
            },
            ProbabilityDocumentError::Cancelled(cause) => {
                Self::Graph(ExactProbabilityGraphError::Cancelled(cause))
            }
            ProbabilityDocumentError::Graph(error) => Self::Graph(error),
            ProbabilityDocumentError::Build(issue) => Self::Build(issue),
            ProbabilityDocumentError::Accounting(incomplete) => Self::Limit {
                limit: ModelCheckLimit::EvaluationMeter {
                    counter: incomplete.limit_kind,
                },
                value: incomplete.limit,
            },
            ProbabilityDocumentError::IntegerText { bit_bound, .. } => Self::Limit {
                limit: ModelCheckLimit::ExactProbMaxRationalBits,
                value: bit_bound,
            },
            _ => Self::Refused,
        }
    }
}
fn dbg_refused(line: u32) -> ProbabilityWitnessError {
    eprintln!("DBG refused at {line}");
    ProbabilityWitnessError::Refused
}
