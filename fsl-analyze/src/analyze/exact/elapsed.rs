// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-877: an elapsed-quantile claim over a timed subject, decided by ADR-028
//! XF-2's transform over the product with the measure's elapsed clock and
//! certified in `timed-product/v1` with `form: "elapsed-quantile"`.

use std::collections::BTreeMap;

use super::*;
use crate::exact_probabilistic::{MeasurePhase, MonitorState, MonitorVerdict};
use ix_trace_rs::implements;

/// The transform's value at every state of an acyclic product: a decided
/// measure's terminal reward `[M op c] - q`, otherwise the minimum over the
/// state's choices of the expected successor value. `None` when the product
/// has a cycle, a pending dead end or a decided state that is no measure's end.
fn transform_values(
    product: &ProbProduct,
    q: &Rational,
    maximum: bool,
    cancel: &Cancel,
    tripped: &mut Option<filament_exact::CancelCause>,
) -> Option<Vec<Rational>> {
    let count = product.states.len();
    let decided: BTreeMap<usize, MonitorVerdict> = product
        .decided
        .iter()
        .map(|(state, verdict)| (state.index(), *verdict))
        .collect();
    let terminal = |index: usize| -> Option<Option<Rational>> {
        match product.states[index].monitor {
            Some(MonitorState::Measure {
                phase: MeasurePhase::Complete | MeasurePhase::Censored,
                ..
            }) => {
                let hit = match decided.get(&index)? {
                    MonitorVerdict::Accept => Rational::from_integer(1_i64.into()),
                    MonitorVerdict::Reject => Rational::from_integer(0_i64.into()),
                };
                Some(Some(hit.sub(q)))
            }
            // A time-lock or any other decided pending state has no reward.
            _ if decided.contains_key(&index) => None,
            _ => Some(None),
        }
    };
    let ids: Vec<_> = product.ids().collect();
    let mut values: Vec<Option<Rational>> = vec![None; count];
    // Depth-first postorder with a colour per state: a back edge is a cycle.
    let mut colour = vec![0_u8; count];
    for root in 0..count {
        if colour[root] != 0 {
            continue;
        }
        let mut stack = vec![(root, false)];
        while let Some((state, done)) = stack.pop() {
            if let Some(cause) = cancel.tripped() {
                *tripped = Some(cause);
                return None;
            }
            if done {
                colour[state] = 2;
                let value = match terminal(state)? {
                    Some(reward) => reward,
                    None => {
                        let mut choices: Vec<(
                            &crate::exact_probabilistic::ProductAction,
                            Rational,
                        )> = Vec::new();
                        for edge in product.outgoing(ids[state]) {
                            if edge.probability == Rational::from_integer(0_i64.into()) {
                                continue;
                            }
                            let successor = values[edge.to.index()].clone()?;
                            let weighted = edge.probability.mul(&successor);
                            match choices
                                .iter_mut()
                                .find(|(action, _)| same_mean_time_choice(action, &edge.choice))
                            {
                                Some((_, total)) => *total = total.add(&weighted),
                                None => choices.push((&edge.choice, weighted)),
                            }
                        }
                        {
                            let totals = choices.into_iter().map(|(_, total)| total);
                            if maximum {
                                totals.max()?
                            } else {
                                totals.min()?
                            }
                        }
                    }
                };
                values[state] = Some(value);
                continue;
            }
            match colour[state] {
                2 => continue,
                1 => return None,
                _ => {}
            }
            colour[state] = 1;
            stack.push((state, true));
            if terminal(state)?.is_none() {
                for edge in product.outgoing(ids[state]) {
                    match colour[edge.to.index()] {
                        1 => return None,
                        0 => stack.push((edge.to.index(), false)),
                        _ => {}
                    }
                }
            }
        }
    }
    values.into_iter().collect()
}

/// Whether the actions that attain each state's value, followed from `start`,
/// reach a state whose measure is activated (active, complete or censored).
fn tight_actions_activate(product: &ProbProduct, values: &[Rational], start: usize) -> bool {
    let ids: Vec<_> = product.ids().collect();
    let mut seen = vec![false; product.states.len()];
    let mut layer = vec![start];
    seen[start] = true;
    while !layer.is_empty() {
        let mut next = Vec::new();
        for state in layer {
            if matches!(
                product.states[state].monitor,
                Some(MonitorState::Measure {
                    phase: MeasurePhase::Active | MeasurePhase::Complete | MeasurePhase::Censored,
                    ..
                })
            ) {
                return true;
            }
            let mut choices: Vec<(
                &crate::exact_probabilistic::ProductAction,
                Rational,
                Vec<usize>,
            )> = Vec::new();
            for edge in product.outgoing(ids[state]) {
                if edge.probability == Rational::from_integer(0_i64.into()) {
                    continue;
                }
                let weighted = edge.probability.mul(&values[edge.to.index()]);
                match choices
                    .iter_mut()
                    .find(|(action, _, _)| same_mean_time_choice(action, &edge.choice))
                {
                    Some((_, total, targets)) => {
                        *total = total.add(&weighted);
                        targets.push(edge.to.index());
                    }
                    None => choices.push((&edge.choice, weighted, vec![edge.to.index()])),
                }
            }
            for (_, total, targets) in choices {
                if total == values[state] {
                    for target in targets {
                        if !seen[target] {
                            seen[target] = true;
                            next.push(target);
                        }
                    }
                }
            }
        }
        layer = next;
    }
    false
}

/// The policy that takes, at every state, the first choice attaining the
/// state's transform value (the maximum for `>=`, the minimum for `<=`).
fn tight_policy(product: &ProbProduct, values: &[Rational]) -> Option<Vec<Option<usize>>> {
    let ids: Vec<_> = product.ids().collect();
    let mut policy = vec![None; product.states.len()];
    for (state, entry) in policy.iter_mut().enumerate() {
        let mut choices: Vec<(&crate::exact_probabilistic::ProductAction, Rational)> = Vec::new();
        for edge in product.outgoing(ids[state]) {
            if edge.probability == Rational::from_integer(0_i64.into()) {
                continue;
            }
            let weighted = edge.probability.mul(&values[edge.to.index()]);
            match choices
                .iter_mut()
                .find(|(action, _)| same_mean_time_choice(action, &edge.choice))
            {
                Some((_, total)) => *total = total.add(&weighted),
                None => choices.push((&edge.choice, weighted)),
            }
        }
        if let Some((action, _)) = choices.iter().find(|(_, total)| *total == values[state]) {
            *entry = Some(crate::exact_probabilistic::choice_index(
                product, ids[state], action,
            )?);
        }
    }
    Some(policy)
}

/// FR-877: refute an elapsed-quantile claim with a path set, under a scheduler
/// attaining the transform's extremum, over the decided measures the refutation
/// counts (`M > c` or censored for `<= c`, `M < c` for `>= c`). The checker's
/// own replay of the same product must reproduce it.
/// What `refute` reads: the claim over the built product and its transform values.
struct Refutation<'a> {
    system: &'a ModelSystem<'a>,
    claim: &'a fsl_semantics::check::CheckedProbabilisticClaim,
    product: &'a ProbProduct,
    values: &'a [Rational],
    start: crate::exact_probabilistic::ProbStateId,
    q: &'a Rational,
    at_least: bool,
    binding: Option<&'a filament_exact::ObjectReference>,
    limits: &'a crate::exact_probabilistic::ExactProbLimits,
    cancel: &'a Cancel,
    subject: SubjectLimits<'a>,
    tamper_witness: Option<&'a dyn Fn(&mut ProbabilisticCounterexample)>,
    work: &'a filament_exact::WorkScope,
}

fn refute(request: Refutation<'_>) -> EngineReport {
    use crate::exact_probabilistic::{WitnessError, WitnessScope, produce_quantile_witness};
    let Refutation {
        system,
        claim,
        product,
        values,
        start,
        q,
        at_least,
        binding,
        limits,
        cancel,
        subject,
        tamper_witness,
        work,
    } = request;
    let Some(policy) = tight_policy(product, values) else {
        return not_yet();
    };
    let decided: BTreeMap<usize, MonitorVerdict> = product
        .decided
        .iter()
        .map(|(state, verdict)| (state.index(), *verdict))
        .collect();
    // The counted measures: `M < c` accepts a `>=` claim's refutation, and
    // `M > c` (a rejected or censored measure) a `<=` claim's.
    let counted = if at_least {
        MonitorVerdict::Accept
    } else {
        MonitorVerdict::Reject
    };
    let measure = |state: &crate::exact_probabilistic::ProbKey| {
        matches!(
            state.monitor,
            Some(MonitorState::Measure {
                phase: MeasurePhase::Complete | MeasurePhase::Censored,
                ..
            })
        )
    };
    // The replay prices against the mass of decided measures, so a run that
    // ends in a decided state that is no measure's end is a nonactivation the
    // producer must leave out of the mass too.
    let never = product
        .states
        .iter()
        .enumerate()
        .map(|(index, state)| decided.contains_key(&index) && !measure(state))
        .collect::<Vec<_>>();
    let accepting = product
        .states
        .iter()
        .enumerate()
        .map(|(index, state)| {
            never[index] || (measure(state) && decided.get(&index) == Some(&counted))
        })
        .collect::<Vec<_>>();
    let produced = match produce_quantile_witness(
        product,
        &policy,
        WitnessScope::EveryScheduler,
        start,
        &accepting,
        &never,
        q,
        at_least,
        limits,
        &mut |_| Ok(()),
    ) {
        Ok(produced) => produced,
        Err(WitnessError::Limit { limit, value }) => {
            return settled(&ExactProbBuildIssue::Limit { limit, value }, subject);
        }
        Err(_) => return broken(),
    };
    let produced = match require_witness(produced) {
        Ok(produced) => produced,
        Err(report) => return *report,
    };
    replay_timed_paths(
        system,
        claim,
        product,
        produced,
        start,
        binding,
        limits,
        cancel,
        subject,
        tamper_witness,
        work,
    )
}

#[implements("FR-203-AC-29", "FR-203-AC-30", "FR-203-AC-31", "FR-203-AC-32")]
pub(super) fn run(
    request: ProductRequest<'_>,
    cancel: &Cancel,
    tamper: Option<&dyn Fn(&mut ExactProbabilityCertificate)>,
    tamper_witness: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    let claim = request.claim;
    let ProbForm::Quantile { q, threshold, .. } = &claim.form else {
        return not_yet();
    };
    // `<= c` is decided by the transform's minimum over the digital product;
    // `>= c` by its maximum over the region product, whose `M < c` is strict.
    let at_least = matches!(threshold, Bound::AtLeast(_));
    let optimization = if at_least {
        ProbabilityObjective::Maximum
    } else {
        ProbabilityObjective::Minimum
    };
    if !matches!(claim.scheduler, ProbScheduler::Every) || !claim.fairness.is_empty() {
        return not_yet();
    }
    let simulation = request.subject.simulation.clone();
    let Some(source_package) = simulation.source_package() else {
        return not_yet();
    };
    let work = WorkScope::new(simulation.meter);
    let limits = request.limits;
    let meter = simulation.meter;
    let subject = SubjectLimits {
        expansion: simulation.expansion,
        meter: &meter,
    };
    let document_limits = ProbabilityDocumentLimits {
        bytes: limits.document.bytes,
        storage_bytes: limits.document.storage_bytes,
        states: limits.model.limits.max_states,
        edges: limits.model.limits.max_transitions,
        scalar_bits: limits.max_rational_bits,
        accounting: meter,
    };
    let admitted = ModelSystem::new_with_cancel(simulation.clone(), cancel);
    if let Some(cause) = cancel.tripped() {
        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
    }
    let system = match admitted {
        Ok(system) => system,
        Err(failure) => return build_refusal(ProductRefusal::Admission(failure), subject),
    };
    let timed = match fsl_eval::simulation::model::TimedSystem::new(
        &system,
        claim.subject.alias.as_str(),
    ) {
        Ok(timed) => timed,
        Err(error) => return EngineReport::Refused(subject_refusal(&error)),
    };
    if crate::exact_probabilistic::timed_region_route(&timed, claim) != at_least {
        return not_yet();
    }
    let certificate_scale =
        match crate::exact_probabilistic::timed_certificate_scale(&system, &timed, claim) {
            Ok(scale) => scale,
            Err(refusal) => return build_refusal(refusal, subject),
        };
    let product_request = ProductRequest {
        subject: request.subject,
        claim,
        limits,
    };
    let (built, _) = match build_retaining_monitor_in_scope(product_request, work.clone(), cancel) {
        Ok(built) => built,
        Err(refusal) => return build_refusal(refusal, subject),
    };
    let product = match timed_product(built, &limits, subject) {
        Ok(product) => product,
        Err(report) => return *report,
    };
    let [(_, _, start)] = product.initial.as_slice() else {
        return not_yet();
    };
    let mut tripped = None;
    let values = transform_values(&product, q, at_least, cancel, &mut tripped);
    if let Some(cause) = tripped {
        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
    }
    let Some(values) = values else {
        return not_yet();
    };
    let Some(value) = values.get(start.index()) else {
        return broken();
    };
    // `quantile q of M <= c` holds exactly when the minimum is not negative and
    // `>= c` when the maximum is negative; the converse is a refutation, read
    // as a witness path set under a scheduler attaining the extremum.
    let zero = Rational::from_integer(0_i64.into());
    // A maximum of exactly 0 proves the claim only when no tight action leads
    // to an activated state (FR-201, ADR-028 CE-4).
    if (at_least && *value > zero)
        || (at_least && *value == zero && tight_actions_activate(&product, &values, start.index()))
        || (!at_least && *value < zero)
    {
        let binding = match claim_binding(&system, claim) {
            Ok(binding) => binding,
            Err(error) => return graph_failure(error, &limits, subject),
        };
        return refute(Refutation {
            system: &system,
            claim,
            product: &product,
            values: &values,
            start: *start,
            q,
            at_least,
            binding: binding.as_ref().map(|binding| &binding.reference),
            limits: &limits,
            cancel,
            subject,
            tamper_witness,
            work: &work,
        });
    }
    let values = values
        .into_iter()
        .map(ExtRational::Finite)
        .collect::<Vec<_>>();
    let identity = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(identity) => identity,
        Err(error) => return graph_failure(error, &limits, subject),
    };
    let binding = match claim_binding(&system, claim) {
        Ok(binding) => binding,
        Err(error) => return graph_failure(error, &limits, subject),
    };
    let mut certificate = match certificate_of(
        &product,
        None,
        &values,
        fsl_replay::ProbabilityDocumentIoLimits {
            bytes: document_limits.bytes,
            storage_bytes: document_limits.storage_bytes,
        },
        cancel,
    ) {
        Ok(certificate) => certificate,
        Err(error) => return document_failure(error, subject),
    };
    if let Some(tamper) = tamper {
        tamper(&mut certificate);
    }
    let mut targets = product
        .decided
        .iter()
        .filter(|(_, verdict)| *verdict == MonitorVerdict::Accept)
        .filter_map(|(state, _)| product.states[state.index()].certificate_state.clone())
        .collect::<Vec<_>>();
    targets.sort();
    targets.dedup();
    let document = ProbabilityCertificateDocument {
        cyclic_dtmc: false,
        identity: identity.clone(),
        targets,
        event: None,
        failures: Vec::new(),
        reward: None,
        optimization,
        timed: Some(TimedProbabilityObjective {
            route: if at_least {
                TimedProbabilityRoute::Region
            } else {
                TimedProbabilityRoute::Digital
            },
            unit: timed.source().unit_node.clone(),
            scale: certificate_scale,
            form: TimedProbabilityForm::ElapsedQuantile,
        }),
        values: certificate.values.clone(),
        ranking: Vec::new(),
        policy: None,
    };
    let checked = (|| {
        let bytes = write_probability_certificate(&document, document_limits, cancel)?;
        fsl_replay::check_probability_certificate_document_as_with_work_scope(
            &system,
            source_package.graph().probabilistic_model().claims(),
            &identity,
            &bytes,
            fsl_replay::CertificateRole::Claim,
            document_limits,
            cancel,
            &work,
        )
    })();
    let extremum = if at_least {
        Extremum::Max
    } else {
        Extremum::Min
    };
    checked_report(checked, certificate, binding, Some(extremum), subject)
}
