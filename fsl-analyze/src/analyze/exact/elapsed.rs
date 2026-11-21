// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-877: an elapsed-quantile claim over a timed subject, decided by ADR-028
//! XF-2's transform over the product with the measure's elapsed clock and
//! certified in `timed-product/v1` with `form: "elapsed-quantile"`.

use std::collections::BTreeMap;

use super::*;
use crate::exact_probabilistic::{MeasurePhase, MonitorState, MonitorVerdict};

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

pub(super) fn run(
    request: ProductRequest<'_>,
    cancel: &Cancel,
    tamper: Option<&dyn Fn(&mut ExactProbabilityCertificate)>,
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
        Err(_) => return not_yet(),
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
    let (built, _) = match build_retaining_monitor(product_request, cancel) {
        Ok(built) => built,
        Err(refusal) => return build_refusal(refusal, subject),
    };
    let ProbBuild::Built(product) = built else {
        return not_yet();
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
    // `>= c` when the maximum is negative; the converse is a refutation, which
    // needs a timed witness replay that does not exist yet.
    let zero = Rational::from_integer(0_i64.into());
    if (at_least && *value >= zero) || (!at_least && *value < zero) {
        return not_yet();
    }
    let values = values
        .into_iter()
        .map(ExtRational::Finite)
        .collect::<Vec<_>>();
    let identity = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(identity) => identity,
        Err(_) => return not_yet(),
    };
    let binding = match claim_binding(&system, claim) {
        Ok(binding) => binding,
        Err(_) => return not_yet(),
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
    };
    let checked = (|| {
        let bytes = write_probability_certificate(&document, document_limits, cancel)?;
        check_probability_certificate_document(
            &system,
            source_package.graph().probabilistic_model().claims(),
            &identity,
            &bytes,
            document_limits,
            cancel,
        )
    })();
    checked_report(checked, certificate, binding, Some(if at_least { Extremum::Max } else { Extremum::Min }), subject)
}
