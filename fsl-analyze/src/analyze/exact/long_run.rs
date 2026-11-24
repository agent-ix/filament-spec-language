// SPDX-License-Identifier: AGPL-3.0-or-later
//! Candidate production for the independently checked DTMC long-run profile.

use super::candidate::{rational_bytes, sort, values, CandidateBudget};
use super::*;
use crate::exact_probabilistic::{
    build_retaining_monitor, build_retaining_monitor_with_budget, decide_long_run_workload,
    BudgetedProductError, LongRunResult, ProductKind,
};
use fsl_replay::{
    long_run_probability_certificate_document, read_long_run_probability_certificate,
    weighted_long_run_probability_certificate_document, write_long_run_probability_certificate,
    write_weighted_long_run_probability_certificate, LongRunProbabilityComponent,
    ProbabilityDocumentIoLimits,
};

pub(super) fn run(
    mut request: ProductRequest<'_>,
    cancel: &Cancel,
    tamper: Option<&dyn Fn(&mut ExactProbabilityCertificate)>,
    tamper_witness: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    let claim = request.claim;
    let ProbForm::LongRunFraction { bound, weight, .. } = &claim.form else {
        return not_yet();
    };
    let weight = match weight {
        Some(fsl_semantics::check::Weighting::Reward(name)) => Some(name),
        Some(fsl_semantics::check::Weighting::Delay) => return not_yet(),
        None => None,
    };
    if !matches!(
        fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity()),
        Some(
            fsl_semantics::check::TemporalProfile::InfiniteTrace
                | fsl_semantics::check::TemporalProfile::EventPositionFalseExtension
        )
    ) {
        return not_yet();
    }
    let every = matches!(claim.scheduler, ProbScheduler::Every);
    if every && weight.is_some() {
        // Per-action weights are not part of the MDP long-run profile yet.
        return not_yet();
    }
    let simulation = request.subject.simulation.clone();
    let Some(package) = simulation.source_package() else {
        return not_yet();
    };
    let limits = request.limits;
    let subject = SubjectLimits {
        expansion: simulation.expansion,
        meter: &simulation.meter,
    };
    let system = match ModelSystem::new_with_cancel(simulation.clone(), cancel) {
        Ok(system) => system,
        Err(error) => {
            return match cancel.tripped() {
                Some(cause) => settled(&ExactProbBuildIssue::Cancelled(cause), subject),
                None => EngineReport::Refused(admission(&error)),
            };
        }
    };
    let item = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(item) => item,
        Err(error) => return graph_failure(error, &limits, subject),
    };
    let binding = match claim_binding(&system, claim) {
        Ok(binding) => binding,
        Err(error) => return graph_failure(error, &limits, subject),
    };
    let io = ProbabilityDocumentIoLimits {
        bytes: limits.document.bytes,
        storage_bytes: limits.document.storage_bytes,
    };
    let document_limits = ProbabilityDocumentLimits {
        bytes: io.bytes,
        storage_bytes: io.storage_bytes,
        states: limits.model.limits.max_states,
        edges: limits.model.limits.max_transitions,
        scalar_bits: limits.max_rational_bits,
        accounting: simulation.meter,
    };
    let claims = package.graph().probabilistic_model().claims();
    let mut candidate = CandidateBudget::new(document_limits, cancel);
    let product = {
        let identity = candidate.identity;
        let mut admission = candidate.admission();
        let proof_bounds = std::mem::take(&mut request.subject.proof_bounds);
        match build_retaining_monitor_with_budget(request, identity, &mut admission, cancel) {
            Ok((ProbBuild::Built(product), _)) => product,
            Ok((ProbBuild::Partial { .. }, _)) => return not_yet(),
            Ok((ProbBuild::Undefined(path), monitor)) => {
                return undefined::run(
                    simulation.clone(),
                    claim,
                    path,
                    monitor,
                    &limits,
                    cancel,
                    subject,
                    tamper_witness,
                )
            }
            Err(BudgetedProductError::Product(error)) => return build_refusal(error, subject),
            Err(BudgetedProductError::Key(error)) => {
                return document_failure(admission.refusal(error), subject)
            }
            Err(BudgetedProductError::Undefined(_)) => {
                let undefined_request = ProductRequest {
                    subject: fsl_eval::model_subject::ModelSubject {
                        simulation: simulation.clone(),
                        proof_bounds,
                    },
                    claim,
                    limits,
                };
                match build_retaining_monitor(undefined_request, cancel) {
                    Ok((ProbBuild::Undefined(path), monitor)) => {
                        return undefined::run(
                            simulation.clone(),
                            claim,
                            path,
                            monitor,
                            &limits,
                            cancel,
                            subject,
                            tamper_witness,
                        )
                    }
                    Ok((ProbBuild::Partial { .. }, _)) => return not_yet(),
                    Ok((ProbBuild::Built(_), _)) => return broken(),
                    Err(error) => return build_refusal(error, subject),
                }
            }
        }
    };
    let kind = if every {
        ProductKind::Mdp
    } else {
        ProductKind::Dtmc
    };
    if product.kind != kind
        || product.initial.len() != 1
        || product.states.iter().any(|state| {
            state.monitor.is_some() || state.accumulator.is_some() || state.pending.is_some()
        })
    {
        return not_yet();
    }
    if let Some(weight) = weight {
        if let Err(error) = admit_weights(&product, weight, document_limits, &mut candidate) {
            return document_failure(error, subject);
        }
    }
    let predicate = product
        .labels
        .iter()
        .map(|labels| labels.first().copied().unwrap_or(false))
        .collect::<Vec<_>>();
    if let Some(cause) = cancel.tripped() {
        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
    }
    if every {
        let reference = binding.as_ref().map(|b| &b.reference);
        return every_scheduler(
            EveryScheduler {
                system: &system,
                claim,
                claims,
                item: &item,
                product: &product,
                predicate: &predicate,
                binding: reference,
                claim_binding: binding.clone(),
                limits: &limits,
                document_limits,
                subject,
                cancel,
            },
            &mut candidate,
            tamper_witness,
        );
    }
    let solution = match decide_long_run_workload(
        &product,
        &predicate,
        weight.map(|name| name.as_str()),
        &limits,
    ) {
        Ok(solution) => solution,
        Err(UnboundedError::MaxRationalBits) => {
            return settled(
                &ExactProbBuildIssue::Limit {
                    limit: ModelCheckLimit::ExactProbMaxRationalBits,
                    value: limits.max_rational_bits,
                },
                subject,
            )
        }
        Err(UnboundedError::MaxPolicyIterations) => {
            return settled(
                &ExactProbBuildIssue::Limit {
                    limit: ModelCheckLimit::ExactProbMaxPolicyIterations,
                    value: limits.max_policy_iterations,
                },
                subject,
            )
        }
        Err(UnboundedError::ZeroWeightComponent) => {
            return EngineReport::Refused(Refusal::Unsupported(ZERO_WEIGHT_COMPONENT))
        }
        // `MissingReward` included: `admit_weights` refused every non-numeric or
        // negative reward above, so a solve that still names one is a disagreement
        // between admission and the solver, the engine's own fault.
        Err(_) => return broken(),
    };
    if let Some(cause) = cancel.tripped() {
        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
    }
    let [(_, _, initial)] = product.initial.as_slice() else {
        return not_yet();
    };
    let Some(ExtRational::Finite(value)) = solution.values.get(initial.index()) else {
        return broken();
    };
    let holds = match bound {
        Bound::AtLeast(threshold) => value >= threshold,
        Bound::AtMost(threshold) => value <= threshold,
    };
    let reference = binding.as_ref().map(|b| &b.reference);
    let mut certificate = match values(&product, reference, &solution.values, &mut candidate) {
        Ok(certificate) => certificate,
        Err(error) => return document_failure(error, subject),
    };
    let components = match components(&product, reference, &solution, None, &mut candidate) {
        Ok(components) => components,
        Err(error) => return document_failure(error, subject),
    };
    if !holds {
        // FR-202: the same long-run evidence, checked over the rebuilt
        // workload chain in its counterexample role, settles `refuted` only
        // after replay reproduces the strict comparison.
        let initial_key = match candidate.key(reference, &product.states[initial.index()].model) {
            Ok(key) => key,
            Err(error) => return document_failure(error, subject),
        };
        let subsystem = if weight.is_some() {
            weighted_long_run_probability_certificate_document(
                &system,
                claims,
                &item,
                certificate.values,
                components,
                document_limits,
                cancel,
            )
            .and_then(|document| {
                fsl_replay::ProbabilitySubsystemCertificate::from_weighted_long_run(
                    &document,
                    document_limits,
                    cancel,
                )
            })
        } else {
            long_run_probability_certificate_document(
                &system,
                claims,
                &item,
                certificate.values,
                components,
                document_limits,
                cancel,
            )
            .and_then(|document| {
                fsl_replay::ProbabilitySubsystemCertificate::from_long_run(
                    &document,
                    document_limits,
                    cancel,
                )
            })
        };
        let subsystem = match subsystem {
            Ok(subsystem) => subsystem,
            Err(error) => return document_failure(error, subject),
        };
        let initials = system.initial();
        let [first] = initials.as_slice() else {
            return not_yet();
        };
        let mut witness = ProbabilisticCounterexample {
            item: item.clone(),
            binding: reference.cloned(),
            instance: None,
            initial: first.clone(),
            initial_key,
            scheduler: Vec::new(),
            paths: Vec::new(),
            subsystem: Some(Box::new(subsystem)),
        };
        if let Some(tamper) = tamper_witness {
            tamper(&mut witness);
        }
        return finish_replay(
            replay_witness(&system, &item, &witness, &limits, cancel),
            claim,
            &system,
            &witness,
            &[],
            &limits,
            subject,
        );
    }
    if let Some(tamper) = tamper {
        tamper(&mut certificate);
    }
    for row in &certificate.values {
        if let Err(error) = candidate.retain_row(row) {
            return document_failure(error, subject);
        }
    }
    let bytes = if weight.is_some() {
        weighted_long_run_probability_certificate_document(
            &system,
            claims,
            &item,
            certificate.values.clone(),
            components,
            document_limits,
            cancel,
        )
        .and_then(|document| {
            write_weighted_long_run_probability_certificate(&document, document_limits, cancel)
        })
    } else {
        long_run_probability_certificate_document(
            &system,
            claims,
            &item,
            certificate.values.clone(),
            components,
            document_limits,
            cancel,
        )
        .and_then(|document| {
            write_long_run_probability_certificate(&document, document_limits, cancel)
        })
    };
    let checked = bytes.and_then(|bytes| {
        check_probability_certificate_document(
            &system,
            claims,
            &item,
            &bytes,
            document_limits,
            cancel,
        )
    });
    checked_report(checked, certificate, binding, None, subject)
}

/// What an every-scheduler long-run solve reads (FR-411 "Long-run fractions").
struct EveryScheduler<'a> {
    system: &'a ModelSystem<'a>,
    claim: &'a fsl_semantics::check::CheckedProbabilisticClaim,
    claims: &'a [fsl_semantics::check::CheckedProbabilisticClaim],
    item: &'a fsl_package::ObligationIdentity,
    product: &'a ProbProduct,
    predicate: &'a [bool],
    binding: Option<&'a ObjectReference>,
    claim_binding: Option<fsl_replay::ClaimBinding>,
    limits: &'a crate::exact_probabilistic::ExactProbLimits,
    document_limits: ProbabilityDocumentLimits,
    subject: SubjectLimits<'a>,
    cancel: &'a Cancel,
}

/// Settle `long_run_fraction` over every scheduler (FR-411-AC-18). The
/// extremum comes from `decide_long_run_mdp`, or under a fairness set from the
/// fair-end-component solver. A violated bound is refuted by a deterministic
/// stationary scheduler, written with the long-run evidence of the chain it
/// induces and settled only after FR-202 replay reproduces it (FR-412-AC-27).
fn every_scheduler(
    request: EveryScheduler<'_>,
    candidate: &mut CandidateBudget<'_>,
    tamper_witness: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    use crate::exact_probabilistic::{
        decide_fair_long_run_mdp, decide_long_run_mdp, deterministic_policy_is_fair, induced_chain,
        policy_action, FairProductError, LongRunResult, ProbStateId, ProductAction,
    };
    let EveryScheduler {
        system,
        claim,
        claims,
        item,
        product,
        predicate,
        binding,
        claim_binding,
        limits,
        document_limits,
        subject,
        cancel,
    } = request;
    let ProbForm::LongRunFraction { bound, .. } = &claim.form else {
        return not_yet();
    };
    let objective = match bound {
        Bound::AtLeast(_) => UnboundedObjective::Min,
        Bound::AtMost(_) => UnboundedObjective::Max,
    };
    let stopped = |error: UnboundedError| match error {
        UnboundedError::MissingReward => not_yet(),
        other => unbounded_failure(other, limits, subject),
    };
    let (values, policy, solution, fair_witness) = if claim.fairness.is_empty() {
        match decide_long_run_mdp(product, predicate, None, objective, limits, cancel) {
            Ok(solution) => (
                solution.values.clone(),
                solution.policy.clone(),
                Some(solution),
                None,
            ),
            Err(error) => return stopped(error),
        }
    } else {
        match decide_fair_long_run_mdp(product, predicate, None, bound, limits, cancel) {
            Ok(solution) => {
                let fair_witness = solution.witness.clone();
                let component_solution = LongRunResult {
                    components: solution.components.clone(),
                    values: solution.values.clone(),
                    policy: Some(solution.deterministic.clone()),
                    component_policy: Some(solution.component_policy.clone()),
                    gain_bias: solution.gain_bias.clone(),
                };
                (
                    solution.values,
                    Some(solution.deterministic),
                    Some(component_solution),
                    fair_witness,
                )
            }
            Err(FairProductError::Product(error)) => return stopped(error),
            Err(_) => return not_yet(),
        }
    };
    if let Some(cause) = cancel.tripped() {
        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
    }
    let [(_, _, initial)] = product.initial.as_slice() else {
        return not_yet();
    };
    let Some(ExtRational::Finite(value)) = values.get(initial.index()) else {
        return broken();
    };
    let holds = match bound {
        Bound::AtLeast(threshold) => value >= threshold,
        Bound::AtMost(threshold) => value <= threshold,
    };
    if holds {
        // FR-412-AC-26 and FR-1041: each fair end component carries its own
        // independently checked gain-bias pair and staying policy.
        let Some(solution) = solution else {
            return not_yet();
        };
        let Some(policy) = solution.component_policy.clone() else {
            return broken();
        };
        let certificate =
            match super::candidate::values(product, binding, &solution.values, candidate) {
                Ok(certificate) => certificate,
                Err(error) => return document_failure(error, subject),
            };
        let components = match components(
            product,
            binding,
            &solution,
            Some((system, ComponentPolicy::Deterministic(&policy))),
            candidate,
        ) {
            Ok(components) => components,
            Err(error) => return document_failure(error, subject),
        };
        let bytes = long_run_probability_certificate_document(
            system,
            claims,
            item,
            certificate.values.clone(),
            components,
            document_limits,
            cancel,
        )
        .and_then(|document| {
            write_long_run_probability_certificate(&document, document_limits, cancel)
        });
        let checked = bytes.and_then(|bytes| {
            check_probability_certificate_document(
                system,
                claims,
                item,
                &bytes,
                document_limits,
                cancel,
            )
        });
        return checked_report(checked, certificate, claim_binding, None, subject);
    }
    let Some(policy) = policy else {
        return broken();
    };
    if !claim.fairness.is_empty() {
        if let Some(fair_witness) = fair_witness.as_ref() {
            return fair_randomized_refutation(
                system,
                claim,
                claims,
                item,
                product,
                predicate,
                binding,
                fair_witness,
                document_limits,
                limits,
                cancel,
                subject,
                candidate,
                tamper_witness,
            );
        }
        // A deterministic witness counts only when its induced chain is fair;
        // the fair extremum need not be attained by one (FR-411).
        match deterministic_policy_is_fair(product, &policy, *initial) {
            Ok(true) => {}
            Ok(false) | Err(_) => return not_yet(),
        }
    }
    let chain = induced_chain(product, &policy);
    let solved =
        match crate::exact_probabilistic::decide_long_run_workload(&chain, predicate, None, limits)
        {
            Ok(solved) => solved,
            Err(error) => return stopped(error),
        };
    let Some(ExtRational::Finite(induced)) = solved.values.get(initial.index()) else {
        return broken();
    };
    let refutes = match bound {
        Bound::AtLeast(threshold) => induced < threshold,
        Bound::AtMost(threshold) => induced > threshold,
    };
    if !refutes {
        return not_yet();
    }
    // Only what the scheduler reaches belongs to the evidence.
    let mut reached = vec![false; product.states.len()];
    let mut pending = vec![*initial];
    reached[initial.index()] = true;
    while let Some(state) = pending.pop() {
        for edge in chain.outgoing(state) {
            if !edge.probability.is_zero() && !reached[edge.to.index()] {
                reached[edge.to.index()] = true;
                pending.push(edge.to);
            }
        }
    }
    let mut rows = Vec::new();
    if let Err(error) = candidate.reserve(
        reached
            .iter()
            .filter(|reached| **reached)
            .count()
            .saturating_mul(std::mem::size_of::<ExactProbabilityValue>()),
    ) {
        return document_failure(error, subject);
    }
    for (state, model) in product.states.iter().enumerate() {
        if !reached[state] {
            continue;
        }
        let Some(ExtRational::Finite(value)) = solved.values.get(state) else {
            return broken();
        };
        if let Err(error) = candidate.reserve(rational_bytes(value)) {
            return document_failure(error, subject);
        }
        match candidate.key(binding, &model.model) {
            Ok(state) => rows.push(ExactProbabilityValue {
                state,
                value: value.clone(),
            }),
            Err(error) => return document_failure(error, subject),
        }
    }
    if let Err(error) = sort(
        &mut rows,
        |row| &row.state,
        &mut candidate.meter,
        candidate.cancel,
    ) {
        return document_failure(error, subject);
    }
    let mut evidence = solved.clone();
    let keep = |first: Option<&ProbStateId>| first.is_some_and(|state| reached[state.index()]);
    let (mut kept_components, mut kept_gain_bias) = (Vec::new(), Vec::new());
    for (component, gain_bias) in evidence
        .components
        .drain(..)
        .zip(evidence.gain_bias.drain(..))
    {
        if keep(component.states.first()) {
            kept_components.push(component);
            kept_gain_bias.push(gain_bias);
        }
    }
    evidence.components = kept_components;
    evidence.gain_bias = kept_gain_bias;
    let components = match components(
        product,
        binding,
        &evidence,
        Some((system, ComponentPolicy::Deterministic(&policy))),
        candidate,
    ) {
        Ok(components) => components,
        Err(error) => return document_failure(error, subject),
    };
    let mut scheduler = Vec::new();
    let mut spelled = Vec::new();
    for state in product.ids() {
        if !reached[state.index()] {
            continue;
        }
        let Some(action) = policy_action(product, state, &policy) else {
            continue;
        };
        let ProductAction::Identity(identity) = &action else {
            if matches!(action, ProductAction::TerminalStutter) {
                continue;
            }
            return not_yet();
        };
        let key = match candidate.key(binding, &product.states[state.index()].model) {
            Ok(key) => key,
            Err(error) => return document_failure(error, subject),
        };
        let Ok(schedule) = fsl_replay::Schedule::of(system, identity) else {
            return not_yet();
        };
        spelled.push((
            fsl_eval::simulation::StateKey::from_canonical_bytes(key.as_bytes().to_vec()),
            identity.clone(),
        ));
        scheduler.push((
            key,
            fsl_replay::ProbabilisticSchedulerChoice::Deterministic(
                fsl_replay::ProductAction::Identity(schedule),
            ),
        ));
    }
    scheduler.sort_by(|a, b| a.0.cmp(&b.0));
    let subsystem = long_run_probability_certificate_document(
        system,
        claims,
        item,
        rows,
        components,
        document_limits,
        cancel,
    )
    .and_then(|document| {
        fsl_replay::ProbabilitySubsystemCertificate::from_long_run(
            &document,
            document_limits,
            cancel,
        )
    });
    let subsystem = match subsystem {
        Ok(subsystem) => subsystem,
        Err(error) => return document_failure(error, subject),
    };
    let initial_key = match candidate.key(binding, &product.states[initial.index()].model) {
        Ok(key) => key,
        Err(error) => return document_failure(error, subject),
    };
    let initials = system.initial();
    let [first] = initials.as_slice() else {
        return not_yet();
    };
    let mut witness = ProbabilisticCounterexample {
        item: item.clone(),
        binding: binding.cloned(),
        instance: None,
        initial: first.clone(),
        initial_key,
        scheduler,
        paths: Vec::new(),
        subsystem: Some(Box::new(subsystem)),
    };
    if let Some(tamper) = tamper_witness {
        tamper(&mut witness);
    }
    let replayed = replay_witness(system, item, &witness, limits, cancel);
    finish_replay(replayed, claim, system, &witness, &spelled, limits, subject)
}

#[allow(clippy::too_many_arguments)]
fn fair_randomized_refutation(
    system: &ModelSystem<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    claims: &[fsl_semantics::check::CheckedProbabilisticClaim],
    item: &fsl_package::ObligationIdentity,
    product: &ProbProduct,
    predicate: &[bool],
    binding: Option<&ObjectReference>,
    fair_witness: &crate::exact_probabilistic::FairRandomizedWitness,
    document_limits: ProbabilityDocumentLimits,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
    budget: &mut CandidateBudget<'_>,
    tamper_witness: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    use crate::exact_probabilistic::{
        decide_long_run_workload, induced_fair_randomized_chain, policy_action, ActionChoice,
        ProductAction as AnalyzerAction,
    };
    let chain = match induced_fair_randomized_chain(product, &fair_witness.policy) {
        Ok(chain) => chain,
        Err(_) => return broken(),
    };
    let solution = match decide_long_run_workload(&chain, predicate, None, limits) {
        Ok(solution) => solution,
        Err(error) => return unbounded_failure(error, limits, subject),
    };
    let certificate = match super::candidate::values(&chain, binding, &solution.values, budget) {
        Ok(certificate) => certificate,
        Err(error) => return document_failure(error, subject),
    };
    let components = match components(
        product,
        binding,
        &solution,
        Some((system, ComponentPolicy::Randomized(&fair_witness.policy))),
        budget,
    ) {
        Ok(components) => components,
        Err(error) => return document_failure(error, subject),
    };
    let bytes = long_run_probability_certificate_document(
        system,
        claims,
        item,
        certificate.values,
        components,
        document_limits,
        cancel,
    )
    .and_then(|document| {
        write_long_run_probability_certificate(&document, document_limits, cancel)
    });
    let bytes = match bytes {
        Ok(bytes) => bytes,
        Err(error) => return document_failure(error, subject),
    };
    let subsystem = match read_long_run_probability_certificate(&bytes, document_limits, cancel)
        .and_then(|document| {
            fsl_replay::ProbabilitySubsystemCertificate::from_long_run(
                &document,
                document_limits,
                cancel,
            )
        }) {
        Ok(subsystem) => subsystem,
        Err(error) => return document_failure(error, subject),
    };
    let mut reached = vec![false; chain.states.len()];
    let mut pending = Vec::new();
    let [(_, _, initial)] = chain.initial.as_slice() else {
        return not_yet();
    };
    reached[initial.index()] = true;
    pending.push(*initial);
    while let Some(state) = pending.pop() {
        for edge in chain.outgoing(state) {
            if !edge.probability.is_zero() && !reached[edge.to.index()] {
                reached[edge.to.index()] = true;
                pending.push(edge.to);
            }
        }
    }
    let key = |state: crate::exact_probabilistic::ProbStateId, budget: &mut CandidateBudget<'_>| {
        budget.key(binding, &product.states[state.index()].model)
    };
    let mut scheduler = Vec::new();
    let mut spelled = Vec::new();
    for state in product.ids() {
        if !reached[state.index()] {
            continue;
        }
        let Some(choice) = fair_witness
            .policy
            .get(state.index())
            .and_then(Option::as_ref)
        else {
            continue;
        };
        let alternatives = match choice {
            crate::exact_probabilistic::SchedulerChoice::Deterministic(ActionChoice(index)) => {
                let mut selected = vec![None; product.states.len()];
                selected[state.index()] = Some(*index);
                let Some(AnalyzerAction::Identity(identity)) =
                    policy_action(product, state, &selected)
                else {
                    return broken();
                };
                vec![(
                    AnalyzerAction::Identity(identity),
                    filament_exact::Rational::from_integer(filament_exact::Integer::from(1_i64)),
                )]
            }
            crate::exact_probabilistic::SchedulerChoice::Randomized(distribution) => {
                let mut alternatives = Vec::with_capacity(distribution.len());
                for (ActionChoice(index), probability) in distribution {
                    let mut selected = vec![None; product.states.len()];
                    selected[state.index()] = Some(*index);
                    let resolved = policy_action(product, state, &selected);
                    let Some(AnalyzerAction::Identity(identity)) = resolved else {
                        return broken();
                    };
                    alternatives.push((AnalyzerAction::Identity(identity), probability.clone()));
                }
                alternatives
            }
        };
        let mut distribution = Vec::with_capacity(alternatives.len());
        let state_key = match key(state, budget) {
            Ok(key) => key,
            Err(error) => return document_failure(error, subject),
        };
        let spelled_key =
            fsl_eval::simulation::StateKey::from_canonical_bytes(state_key.as_bytes().to_vec());
        for (action, probability) in alternatives {
            let AnalyzerAction::Identity(identity) = action else {
                return not_yet();
            };
            let schedule = match fsl_replay::Schedule::of(system, &identity) {
                Ok(schedule) => schedule,
                Err(_) => return not_yet(),
            };
            spelled.push((spelled_key.clone(), identity));
            distribution.push((fsl_replay::ProductAction::Identity(schedule), probability));
        }
        let choice = if distribution.len() == 1 {
            fsl_replay::ProbabilisticSchedulerChoice::Deterministic(distribution.remove(0).0)
        } else {
            fsl_replay::ProbabilisticSchedulerChoice::Randomized(distribution)
        };
        scheduler.push((state_key, choice));
    }
    scheduler.sort_by(|left, right| left.0.cmp(&right.0));
    let initial_model = system.initial();
    let [first] = initial_model.as_slice() else {
        return not_yet();
    };
    let initial_key = match key(*initial, budget) {
        Ok(key) => key,
        Err(error) => return document_failure(error, subject),
    };
    let mut witness = ProbabilisticCounterexample {
        item: item.clone(),
        binding: binding.cloned(),
        instance: None,
        initial: first.clone(),
        initial_key,
        scheduler,
        paths: Vec::new(),
        subsystem: Some(Box::new(subsystem)),
    };
    if let Some(tamper) = tamper_witness {
        tamper(&mut witness);
    }
    finish_replay(
        replay_witness(system, item, &witness, limits, cancel),
        claim,
        system,
        &witness,
        &spelled,
        limits,
        subject,
    )
}

fn admit_weights(
    product: &ProbProduct,
    name: &filament_exact::Identifier,
    limits: ProbabilityDocumentLimits,
    budget: &mut CandidateBudget<'_>,
) -> Result<(), ProbabilityDocumentError> {
    use filament_exact::{IntegerDecimalBudget, Value};
    for edge in &product.edges {
        let mut admission = budget.admission();
        for (reward, value) in &edge.rewards {
            IntegerDecimalBudget::work(&mut admission, reward.as_str().len() as u64 + 1).map_err(
                |_| admission.refusal(fsl_eval::simulation::model::ModelKeyError::Denied),
            )?;
            if reward != name {
                continue;
            }
            let (bits, negative) = match value {
                Value::Integer(integer) => {
                    fsl_eval::simulation::model::ModelKeyBudget::integer(&mut admission, integer)
                        .map_err(|_| {
                        admission.refusal(fsl_eval::simulation::model::ModelKeyError::Denied)
                    })?;
                    (integer.magnitude_bits(), integer.is_negative())
                }
                Value::Quantity(quantity) => (
                    quantity
                        .magnitude()
                        .numerator()
                        .magnitude_bits()
                        .max(quantity.magnitude().denominator().magnitude_bits()),
                    quantity.magnitude().numerator().is_negative(),
                ),
                _ => return Err(ProbabilityDocumentError::Unsupported),
            };
            if negative {
                return Err(ProbabilityDocumentError::Unsupported);
            }
            if bits > limits.scalar_bits {
                return Err(ProbabilityDocumentError::Limit {
                    limit: fsl_replay::ProbabilityDocumentLimit::RationalBits,
                    bound: limits.scalar_bits,
                    actual: bits,
                });
            }
        }
    }
    Ok(())
}

enum ComponentPolicy<'a> {
    Deterministic(&'a [Option<usize>]),
    Randomized(&'a [Option<crate::exact_probabilistic::SchedulerChoice>]),
}

fn components(
    product: &ProbProduct,
    binding: Option<&ObjectReference>,
    solution: &LongRunResult,
    // Under every scheduler: the system and the policy each component's rows
    // name the actions of.
    policy: Option<(&ModelSystem<'_>, ComponentPolicy<'_>)>,
    budget: &mut CandidateBudget<'_>,
) -> Result<Vec<LongRunProbabilityComponent>, ProbabilityDocumentError> {
    use crate::exact_probabilistic::{policy_action, ProductAction};
    if solution.components.len() != solution.gain_bias.len() {
        return Err(ProbabilityDocumentError::Malformed);
    }
    budget.reserve(
        solution
            .components
            .len()
            .saturating_mul(std::mem::size_of::<LongRunProbabilityComponent>()),
    )?;
    let mut components = Vec::with_capacity(solution.components.len());
    for (component, evidence) in solution.components.iter().zip(&solution.gain_bias) {
        if let Some(cause) = budget.cancel.tripped() {
            return Err(ProbabilityDocumentError::Cancelled(cause));
        }
        if component.states.len() != evidence.bias.len() {
            return Err(ProbabilityDocumentError::Malformed);
        }
        budget.reserve(
            evidence
                .bias
                .len()
                .saturating_mul(std::mem::size_of::<ExactProbabilityValue>()),
        )?;
        let mut bias = Vec::with_capacity(evidence.bias.len());
        for (state, value) in &evidence.bias {
            let state = product
                .states
                .get(state.index())
                .ok_or(ProbabilityDocumentError::Malformed)?;
            budget.reserve(rational_bytes(value))?;
            bias.push(ExactProbabilityValue {
                state: budget.key(binding, &state.model)?,
                value: value.clone(),
            });
        }
        sort(
            &mut bias,
            |row| &row.state,
            &mut budget.meter,
            budget.cancel,
        )?;
        let offset = bias
            .first()
            .ok_or(ProbabilityDocumentError::Malformed)?
            .value
            .clone();
        for row in &mut bias {
            row.value = match filament_exact::evaluate_rational_arithmetic(
                filament_exact::RationalArithmetic::Subtract(&row.value, &offset),
                None,
                &mut budget.meter,
            ) {
                filament_exact::Outcome::Completed(value) => value,
                filament_exact::Outcome::Incomplete(incomplete) => {
                    return Err(match budget.cancel.tripped() {
                        Some(cause) => ProbabilityDocumentError::Cancelled(cause),
                        None => ProbabilityDocumentError::Accounting(Box::new(incomplete)),
                    })
                }
                _ => return Err(ProbabilityDocumentError::Malformed),
            };
        }
        let policy = match policy {
            None => None,
            Some((system, ComponentPolicy::Deterministic(selected))) => {
                let mut rows = Vec::with_capacity(component.states.len());
                for state in &component.states {
                    let choice = match policy_action(product, *state, selected) {
                        Some(ProductAction::Identity(identity)) => {
                            fsl_replay::ProductAction::Identity(
                                fsl_replay::Schedule::of(system, &identity)
                                    .map_err(|_| ProbabilityDocumentError::Unsupported)?,
                            )
                        }
                        Some(ProductAction::TerminalStutter) | None => {
                            fsl_replay::ProductAction::TerminalStutter
                        }
                        Some(_) => return Err(ProbabilityDocumentError::Unsupported),
                    };
                    let model = product
                        .states
                        .get(state.index())
                        .ok_or(ProbabilityDocumentError::Malformed)?;
                    rows.push(fsl_replay::LongRunPolicyRow {
                        state: budget.key(binding, &model.model)?,
                        choice: fsl_replay::LongRunPolicyChoice::Deterministic(choice),
                    });
                }
                sort(
                    &mut rows,
                    |row| &row.state,
                    &mut budget.meter,
                    budget.cancel,
                )?;
                Some(rows)
            }
            Some((system, ComponentPolicy::Randomized(selected))) => {
                let mut rows = Vec::with_capacity(component.states.len());
                for state in &component.states {
                    let choices = match selected.get(state.index()).and_then(Option::as_ref) {
                        Some(crate::exact_probabilistic::SchedulerChoice::Deterministic(
                            crate::exact_probabilistic::ActionChoice(index),
                        )) => {
                            let mut selected = vec![None; product.states.len()];
                            selected[state.index()] = Some(*index);
                            let action = crate::exact_probabilistic::policy_action(
                                product, *state, &selected,
                            )
                            .ok_or(ProbabilityDocumentError::Malformed)?;
                            let action = match action {
                                ProductAction::Identity(identity) => {
                                    fsl_replay::ProductAction::Identity(
                                        fsl_replay::Schedule::of(system, &identity)
                                            .map_err(|_| ProbabilityDocumentError::Unsupported)?,
                                    )
                                }
                                ProductAction::TerminalStutter => {
                                    fsl_replay::ProductAction::TerminalStutter
                                }
                                _ => return Err(ProbabilityDocumentError::Unsupported),
                            };
                            fsl_replay::LongRunPolicyChoice::Deterministic(action)
                        }
                        Some(crate::exact_probabilistic::SchedulerChoice::Randomized(
                            distribution,
                        )) => {
                            let mut rows = Vec::with_capacity(distribution.len());
                            for (crate::exact_probabilistic::ActionChoice(index), probability) in
                                distribution
                            {
                                let mut selected = vec![None; product.states.len()];
                                selected[state.index()] = Some(*index);
                                let action = crate::exact_probabilistic::policy_action(
                                    product, *state, &selected,
                                )
                                .ok_or(ProbabilityDocumentError::Malformed)?;
                                let action = match action {
                                    ProductAction::Identity(identity) => {
                                        fsl_replay::ProductAction::Identity(
                                            fsl_replay::Schedule::of(system, &identity).map_err(
                                                |_| ProbabilityDocumentError::Unsupported,
                                            )?,
                                        )
                                    }
                                    ProductAction::TerminalStutter => {
                                        fsl_replay::ProductAction::TerminalStutter
                                    }
                                    _ => return Err(ProbabilityDocumentError::Unsupported),
                                };
                                rows.push((action, probability.clone()));
                            }
                            fsl_replay::LongRunPolicyChoice::Randomized(rows)
                        }
                        None => fsl_replay::LongRunPolicyChoice::Deterministic(
                            fsl_replay::ProductAction::TerminalStutter,
                        ),
                    };
                    let model = product
                        .states
                        .get(state.index())
                        .ok_or(ProbabilityDocumentError::Malformed)?;
                    rows.push(fsl_replay::LongRunPolicyRow {
                        state: budget.key(binding, &model.model)?,
                        choice: choices,
                    });
                }
                sort(
                    &mut rows,
                    |row| &row.state,
                    &mut budget.meter,
                    budget.cancel,
                )?;
                Some(rows)
            }
        };
        components.push(LongRunProbabilityComponent {
            value: component.rho.clone(),
            gain: evidence.gain.clone(),
            bias,
            policy,
        });
    }
    sort(
        &mut components,
        |component| &component.bias[0].state,
        &mut budget.meter,
        budget.cancel,
    )?;
    Ok(components)
}
