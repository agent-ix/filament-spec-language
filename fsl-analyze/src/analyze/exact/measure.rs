// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared-control finite-horizon induction and independently accepted proofs.
use super::*;
use crate::exact_probabilistic::{
    backward_induction, decide_measure_exact, Dyadic, FhObjective, QualifiedFhError,
};
use fsl_replay::{
    conditional_measure_probability_certificate, measure_probability_certificate_document,
    measure_probability_objective, measure_probability_product_key,
    partial_lower_measure_probability_certificate_document, write_measure_probability_certificate,
    MeasurePayoffKind,
};

pub(super) fn run(
    request: ProductRequest<'_>,
    cancel: &Cancel,
    tamper: Option<&dyn Fn(&mut ExactProbabilityCertificate)>,
    tamper_witness: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    let claim = request.claim;
    let simulation = request.subject.simulation.clone();
    let limits = request.limits;
    let scalar = simulation.meter;
    let subject = SubjectLimits {
        expansion: simulation.expansion,
        meter: &scalar,
    };
    let admitted = ModelSystem::new_with_cancel(simulation.clone(), cancel);
    if let Some(cause) = cancel.tripped() {
        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
    }
    let system = match admitted {
        Ok(system) => system,
        Err(error) => return build_refusal(ProductRefusal::Admission(error), subject),
    };
    let Some(package) = system.source_package() else {
        return not_yet();
    };
    let claims = package.graph().probabilistic_model().claims();
    let item = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(item) => item,
        Err(error) => return graph_failure(error, &limits, subject),
    };
    let binding = match claim_binding(&system, claim) {
        Ok(binding) => binding,
        Err(error) => return document_failure(error.into(), subject),
    };
    let reference = binding.as_ref().map(|binding| &binding.reference);
    let document_limits = ProbabilityDocumentLimits::for_exact(&limits, scalar);
    let mut meter = filament_exact::Meter::new(scalar).with_cancel(cancel.clone());
    let mut storage = 0;
    let kind = match &claim.form {
        ProbForm::Probability {
            event: Event::Compare { .. },
            ..
        } => MeasurePayoffKind::Event,
        ProbForm::MeanOfFraction { .. } => MeasurePayoffKind::Fraction,
        ProbForm::Quantile { .. } if matches!(claim.scheduler, ProbScheduler::Workload(_)) => {
            MeasurePayoffKind::Satisfied
        }
        ProbForm::Quantile { .. } => MeasurePayoffKind::Shifted,
        _ => return not_yet(),
    };
    let (payoff, reward, optimization) = match measure_probability_objective(
        &system,
        claim,
        kind,
        document_limits,
        cancel,
        &mut meter,
        &mut storage,
    ) {
        Ok(selected) => selected,
        Err(error) => return document_failure(error, subject),
    };
    let (mut product, lower_product) = match build_retaining_monitor(request, cancel) {
        Ok((ProbBuild::Built(product), _)) => (product, None),
        Ok((
            ProbBuild::Partial {
                product,
                lower_product,
            },
            _,
        )) => {
            if matches!(claim.form, ProbForm::MeanOfFraction { .. })
                && matches!(claim.scheduler, ProbScheduler::Workload(_))
            {
                return max_states_limit(&limits, subject);
            }
            (product, Some(lower_product))
        }
        Ok((ProbBuild::Undefined(path), monitor)) => {
            return undefined::run(
                simulation,
                claim,
                path,
                monitor,
                &limits,
                cancel,
                subject,
                tamper_witness,
            );
        }
        Err(error) => return build_refusal(error, subject),
    };
    let Some((_, _, initial)) = product.initial.first() else {
        return broken();
    };
    let initial = *initial;
    if matches!(claim.form, ProbForm::Quantile { .. })
        && !product.states.iter().any(|state| {
            matches!(
                state.accumulator.as_ref(),
                Some(
                    crate::exact_probabilistic::Accumulator::Steps {
                        activated: true,
                        ..
                    } | crate::exact_probabilistic::Accumulator::Reward {
                        activated: true,
                        ..
                    }
                )
            )
        })
    {
        let empty = ExactProbabilityCertificate { values: Vec::new() };
        let checked = (|| {
            let document = fsl_replay::vacuous_measure_probability_certificate_document(
                &system,
                claims,
                &item,
                document_limits,
                cancel,
            )?;
            let bytes = write_measure_probability_certificate(&document, document_limits, cancel)?;
            check_probability_certificate_document(
                &system,
                claims,
                &item,
                &bytes,
                document_limits,
                cancel,
            )
        })();
        return checked_report(checked, empty, binding, None, subject);
    }
    let terminal = match product
        .states
        .iter()
        .map(|state| {
            let control = state.monitor.ok_or(ProbabilityDocumentError::Malformed)?;
            let sums = state
                .accumulator
                .as_ref()
                .ok_or(ProbabilityDocumentError::Malformed)?;
            payoff
                .terminal(control, sums, &mut meter)
                .map_err(ProbabilityDocumentError::from)
        })
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(terminal) => terminal,
        Err(error) => return document_failure(error, subject),
    };
    let objective = match optimization {
        ProbabilityObjective::Maximum => FhObjective::Max,
        _ => FhObjective::Min,
    };
    let mut lower_terminal = terminal.clone();
    if lower_product.is_some() {
        let least = if kind == MeasurePayoffKind::Shifted {
            match &claim.form {
                ProbForm::Quantile { q, .. } => q.neg(),
                _ => return not_yet(),
            }
        } else {
            Rational::from_integer(0_i64.into())
        };
        lower_terminal.push(Some(least));
    }
    let solve_product = lower_product.as_ref().unwrap_or(&product);
    let mut interval = None;
    let result = decide_measure_exact(
        solve_product,
        &lower_terminal,
        objective,
        limits.max_rational_bits,
        &mut meter,
        &mut storage,
        document_limits.storage_bytes,
    );
    let (values, policy) = match result {
        Ok(FhResult::Exact { values, policy }) => (values, policy),
        Ok(FhResult::Bounds {
            lower,
            upper,
            precision_bits,
            policy,
        }) if lower_product.is_none()
            && matches!(
                kind,
                MeasurePayoffKind::Fraction | MeasurePayoffKind::Shifted
            ) =>
        {
            let values = lower.iter().map(Dyadic::rational).collect();
            interval = Some((lower, upper, precision_bits));
            (values, policy)
        }
        Err(QualifiedFhError::RationalBits { .. })
            if lower_product.is_none()
                && matches!(
                    kind,
                    MeasurePayoffKind::Fraction | MeasurePayoffKind::Shifted
                ) =>
        {
            let threshold = match (&claim.form, kind) {
                (
                    ProbForm::MeanOfFraction {
                        bound: Bound::AtLeast(value) | Bound::AtMost(value),
                        ..
                    },
                    MeasurePayoffKind::Fraction,
                ) => value.clone(),
                (ProbForm::Quantile { .. }, MeasurePayoffKind::Shifted) => {
                    Rational::from_integer(0_i64.into())
                }
                _ => return not_yet(),
            };
            match backward_induction(
                solve_product,
                &lower_terminal,
                objective,
                &threshold,
                limits.max_rational_bits,
                limits.precision_bits,
                limits.max_precision_bits,
            ) {
                Ok(FhResult::Bounds {
                    lower,
                    upper,
                    precision_bits,
                    policy,
                }) => {
                    let values = lower.iter().map(Dyadic::rational).collect();
                    interval = Some((lower, upper, precision_bits));
                    (values, policy)
                }
                Ok(FhResult::Exact { values, policy }) => (values, policy),
                Ok(FhResult::PrecisionBudget { .. }) => {
                    return settled(
                        &ExactProbBuildIssue::Limit {
                            limit: ModelCheckLimit::ExactProbMaxPrecisionBits,
                            value: u64::from(limits.max_precision_bits),
                        },
                        subject,
                    );
                }
                Err(error) => {
                    return document_failure(
                        induction_failure(QualifiedFhError::Graph(error), cancel),
                        subject,
                    );
                }
            }
        }
        Err(error) => {
            return document_failure(induction_failure(error, cancel), subject);
        }
        Ok(_other) => {
            return broken();
        }
    };
    let values = values
        .into_iter()
        .take(product.states.len())
        .collect::<Vec<_>>();
    let policy = policy
        .into_iter()
        .take(product.states.len())
        .collect::<Vec<_>>();
    for state in &mut product.states {
        let Some(control) = state.monitor else {
            return broken();
        };
        let Some(sums) = state.accumulator.as_ref() else {
            return broken();
        };
        let pending = state
            .pending
            .as_ref()
            .map(|pending| {
                let mut admission = fsl_replay::ProbabilityKeyAdmission::new(
                    &mut meter,
                    &mut storage,
                    document_limits.storage_bytes,
                    cancel,
                );
                system
                    .pending_key_bytes_with_budget(
                        &pending.identity,
                        &pending.random,
                        fsl_foundation::IdentityLimits::default(),
                        &mut admission,
                        cancel,
                    )
                    .map_err(|error| admission.refusal(error))
            })
            .transpose();
        let pending = match pending {
            Ok(pending) => pending,
            Err(error) => return document_failure(error, subject),
        };
        let key = match measure_probability_product_key(
            reference,
            &state.model,
            control,
            sums,
            pending.as_deref(),
            reward.as_ref(),
            document_limits,
            cancel,
            &mut meter,
            &mut storage,
        ) {
            Ok(key) => key,
            Err(error) => return document_failure(error, subject),
        };
        state.certificate_state = Some(key);
    }
    let rows =
        |values: &[Rational]| -> Result<ExactProbabilityCertificate, ProbabilityDocumentError> {
            let mut rows = product
                .states
                .iter()
                .zip(values)
                .map(|(state, value)| {
                    Ok(ExactProbabilityValue {
                        state: state
                            .certificate_state
                            .clone()
                            .ok_or(ProbabilityDocumentError::Malformed)?,
                        value: value.clone(),
                    })
                })
                .collect::<Result<Vec<_>, ProbabilityDocumentError>>()?;
            rows.sort_by(|a, b| a.state.cmp(&b.state));
            Ok(ExactProbabilityCertificate { values: rows })
        };
    let mut certificate = match rows(&values) {
        Ok(certificate) => certificate,
        Err(error) => return document_failure(error, subject),
    };
    if let Some(tamper) = tamper {
        tamper(&mut certificate);
    }
    if let Some((lower, upper, precision_bits)) = interval {
        if lower_product.is_some() {
            return not_yet();
        }
        return finish_measure_interval(
            &system,
            claims,
            &item,
            &product,
            kind,
            lower,
            upper,
            precision_bits,
            &limits,
            document_limits,
            cancel,
            optimization,
            binding.as_ref(),
            subject,
        );
    }
    if lower_product.is_some() {
        // A partial product shows only a `lower`: the claim side of `>= θ`
        // and of a `<= c` quantile.
        let lower_side = match &claim.form {
            ProbForm::Probability { bound, .. } | ProbForm::MeanOfFraction { bound, .. } => {
                matches!(bound, Bound::AtLeast(_))
            }
            ProbForm::Quantile { threshold, .. } => matches!(threshold, Bound::AtMost(_)),
            _ => false,
        };
        if !lower_side {
            return max_states_limit(&limits, subject);
        }
        let checked = (|| {
            let document = partial_lower_measure_probability_certificate_document(
                &system,
                claims,
                &item,
                &certificate,
                kind,
                document_limits,
                cancel,
            )?;
            let bytes = fsl_replay::write_value_bounds_measure_certificate(
                &document,
                fsl_replay::SubsystemBoundKind::Lower,
                document_limits,
                cancel,
            )?;
            fsl_replay::check_probability_certificate_document_as(
                &system,
                claims,
                &item,
                &bytes,
                fsl_replay::CertificateRole::Claim,
                document_limits,
                cancel,
            )
        })();
        let check = match checked {
            Ok(check) => check,
            Err(error) => return document_failure(error, subject),
        };
        // A prefix whose bound misses the threshold settles nothing (FR-196).
        if matches!(&check, ExactProbabilityCertificateCheck::Rejected(rejection)
            if rejection.rule == ProbabilityCertificateRule::ThresholdMissed)
        {
            return max_states_limit(&limits, subject);
        }
        let ExactProbabilityCertificateCheck::Accepted(accepted) = &check else {
            return checked_report(Ok(check), certificate, binding, None, subject);
        };
        let upper = if kind == MeasurePayoffKind::Shifted {
            let ProbForm::Quantile { q, .. } = &claim.form else {
                return broken();
            };
            fsl_replay::ExactBound::Finite(fsl_replay::ExactRational::new(
                Rational::from_integer(1_i64.into()).sub(q),
            ))
        } else {
            fsl_replay::ExactBound::Finite(fsl_replay::ExactRational::new(Rational::from_integer(
                1_i64.into(),
            )))
        };
        let evidence = Box::new(ExactProbabilityEvidence {
            certificate,
            value: accepted.value().clone(),
            side: accepted.side(),
        });
        let binding = binding.map(|bound| ExactBinding {
            population: bound.instance.population.node,
            key: bound.instance.key,
        });
        let extremum = match optimization {
            ProbabilityObjective::Maximum => Some(Extremum::Max),
            ProbabilityObjective::Minimum => Some(Extremum::Min),
            _ => None,
        };
        return EngineReport::Exact(Box::new(ExactReport::ProvedPartialBounds {
            claim: check,
            upper,
            // The values are exact; FSpec's method still states the
            // run's positive starting precision.
            method: fsl_replay::BoundsMethod::BackwardInduction {
                precision_bits: limits.precision_bits,
            },
            extremum,
            binding,
            evidence,
        }));
    }
    let checked = (|| {
        let numerator = measure_probability_certificate_document(
            &system,
            claims,
            &item,
            &certificate,
            kind,
            document_limits,
            cancel,
        )?;
        let document = if kind == MeasurePayoffKind::Satisfied {
            let (activated, _, _) = measure_probability_objective(
                &system,
                claim,
                MeasurePayoffKind::Activated,
                document_limits,
                cancel,
                &mut meter,
                &mut storage,
            )?;
            let terminal = product
                .states
                .iter()
                .map(|state| {
                    activated
                        .terminal(
                            state.monitor.ok_or(ProbabilityDocumentError::Malformed)?,
                            state
                                .accumulator
                                .as_ref()
                                .ok_or(ProbabilityDocumentError::Malformed)?,
                            &mut meter,
                        )
                        .map_err(ProbabilityDocumentError::from)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let denominator = match decide_measure_exact(
                &product,
                &terminal,
                objective,
                limits.max_rational_bits,
                &mut meter,
                &mut storage,
                document_limits.storage_bytes,
            )
            .map_err(|error| induction_failure(error, cancel))?
            {
                FhResult::Exact { values, .. } => values,
                _ => return Err(ProbabilityDocumentError::Malformed),
            };
            let denominator = measure_probability_certificate_document(
                &system,
                claims,
                &item,
                &rows(&denominator)?,
                MeasurePayoffKind::Activated,
                document_limits,
                cancel,
            )?;
            conditional_measure_probability_certificate(numerator, denominator)?
        } else {
            numerator
        };
        let bytes = write_measure_probability_certificate(&document, document_limits, cancel)?;
        check_probability_certificate_document(
            &system,
            claims,
            &item,
            &bytes,
            document_limits,
            cancel,
        )
    })();
    let failed_bound = matches!(&checked, Ok(ExactProbabilityCertificateCheck::Rejected(rejection)) if rejection.rule == ProbabilityCertificateRule::ThresholdMissed || matches!(&claim.form,ProbForm::Quantile { threshold:Bound::AtLeast(_),.. }) && values.get(initial.index()).is_some_and(Rational::is_zero));
    if failed_bound && tamper.is_none() {
        if matches!(claim.form, ProbForm::MeanOfFraction { .. }) {
            return refute_mean(
                &system,
                claim,
                &product,
                &policy,
                initial,
                reference,
                &certificate,
                &limits,
                cancel,
                subject,
                tamper_witness,
            );
        }
        return refute(
            &system,
            claim,
            &product,
            RefutationContext {
                policy: &policy,
                initial,
                binding: reference,
                limits: &limits,
                cancel,
                subject,
                tamper: tamper_witness,
                meter,
                storage,
            },
        );
    }
    let extremum = match optimization {
        ProbabilityObjective::Minimum => Some(Extremum::Min),
        ProbabilityObjective::Maximum => Some(Extremum::Max),
        ProbabilityObjective::Workload
        | ProbabilityObjective::WorkloadMinimum
        | ProbabilityObjective::WorkloadMaximum => None,
    };
    checked_report(checked, certificate, binding, extremum, subject)
}
#[allow(clippy::too_many_arguments)]
fn finish_measure_interval(
    system: &ModelSystem<'_>,
    claims: &[fsl_semantics::check::CheckedProbabilisticClaim],
    item: &fsl_package::ObligationIdentity,
    product: &ProbProduct,
    kind: MeasurePayoffKind,
    lower: Vec<Dyadic>,
    upper: Vec<Dyadic>,
    precision_bits: u32,
    exact_limits: &crate::exact_probabilistic::ExactProbLimits,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    optimization: ProbabilityObjective,
    binding: Option<&fsl_replay::ClaimBinding>,
    subject: SubjectLimits<'_>,
) -> EngineReport {
    let interval_limits = ProbabilityDocumentLimits::for_interval(exact_limits, limits.accounting);
    let certificate = |ends: Vec<Dyadic>| {
        let mut values = product
            .states
            .iter()
            .zip(ends)
            .map(|(state, value)| {
                Ok(ExactProbabilityValue {
                    state: state
                        .certificate_state
                        .clone()
                        .ok_or(ProbabilityDocumentError::Malformed)?,
                    value: value.rational(),
                })
            })
            .collect::<Result<Vec<_>, ProbabilityDocumentError>>()?;
        values.sort_by(|left, right| left.state.cmp(&right.state));
        Ok::<_, ProbabilityDocumentError>(ExactProbabilityCertificate { values })
    };
    let lower_certificate = match certificate(lower) {
        Ok(certificate) => certificate,
        Err(error) => return document_failure(error, subject),
    };
    let upper_certificate = match certificate(upper) {
        Ok(certificate) => certificate,
        Err(error) => return document_failure(error, subject),
    };
    let check = |certificate: &ExactProbabilityCertificate,
                 side: fsl_replay::SubsystemBoundKind,
                 role: fsl_replay::CertificateRole| {
        let document = measure_probability_certificate_document(
            system,
            claims,
            item,
            certificate,
            kind,
            interval_limits,
            cancel,
        )?;
        let bytes = fsl_replay::write_value_bounds_measure_certificate(
            &document,
            side,
            interval_limits,
            cancel,
        )?;
        fsl_replay::check_probability_certificate_document_as(
            system,
            claims,
            item,
            &bytes,
            role,
            interval_limits,
            cancel,
        )
    };
    let checked = (
        check(
            &lower_certificate,
            fsl_replay::SubsystemBoundKind::Lower,
            fsl_replay::CertificateRole::Claim,
        ),
        check(
            &upper_certificate,
            fsl_replay::SubsystemBoundKind::Upper,
            fsl_replay::CertificateRole::Counterpart,
        ),
    );
    let (lower_check, upper_check) = match checked {
        (Ok(lower), Ok(upper)) => (lower, upper),
        (Err(error), _) | (_, Err(error)) => {
            return document_failure(error, subject);
        }
    };
    if matches!(
        &lower_check,
        fsl_replay::ExactProbabilityCertificateCheck::Rejected(rejection)
            if rejection.rule == ProbabilityCertificateRule::ThresholdMissed
    ) {
        return settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::ExactProbMaxRationalBits,
                value: exact_limits.max_rational_bits,
            },
            subject,
        );
    }
    let (
        fsl_replay::ExactProbabilityCertificateCheck::Accepted(accepted),
        fsl_replay::ExactProbabilityCertificateCheck::Accepted(_),
    ) = (&lower_check, &upper_check)
    else {
        let check = match lower_check {
            fsl_replay::ExactProbabilityCertificateCheck::Rejected(_) => lower_check,
            _ => upper_check,
        };
        return checked_report(
            Ok(check),
            lower_certificate,
            binding.cloned(),
            None,
            subject,
        );
    };
    let evidence = Box::new(ExactProbabilityEvidence {
        certificate: lower_certificate,
        value: accepted.value().clone(),
        side: accepted.side(),
    });
    let binding = binding.map(|bound| ExactBinding {
        population: bound.instance.population.node.clone(),
        key: bound.instance.key.clone(),
    });
    let extremum = match optimization {
        ProbabilityObjective::Maximum => Some(Extremum::Max),
        ProbabilityObjective::Minimum => Some(Extremum::Min),
        _ => None,
    };
    EngineReport::Exact(Box::new(ExactReport::ProvedBounds {
        claim: lower_check,
        counterpart: Box::new(upper_check),
        method: fsl_replay::BoundsMethod::BackwardInduction { precision_bits },
        extremum,
        binding,
        evidence,
    }))
}

struct RefutationContext<'a> {
    policy: &'a [Option<usize>],
    initial: crate::exact_probabilistic::ProbStateId,
    binding: Option<&'a ObjectReference>,
    limits: &'a crate::exact_probabilistic::ExactProbLimits,
    cancel: &'a Cancel,
    subject: SubjectLimits<'a>,
    tamper: Option<&'a dyn Fn(&mut ProbabilisticCounterexample)>,
    meter: filament_exact::Meter,
    storage: u64,
}

fn refute(
    system: &ModelSystem<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    product: &ProbProduct,
    context: RefutationContext<'_>,
) -> EngineReport {
    let RefutationContext {
        policy,
        initial,
        binding,
        limits,
        cancel,
        subject,
        tamper,
        mut meter,
        mut storage,
    } = context;
    use crate::exact_probabilistic::{
        produce_quantile_witness, produce_witness, WitnessError, WitnessScope, WitnessWork,
    };
    let document_limits = ProbabilityDocumentLimits::for_exact(limits, system.meter_limits());
    let kind = if matches!(claim.form, ProbForm::Quantile { .. }) {
        MeasurePayoffKind::Satisfied
    } else {
        MeasurePayoffKind::Event
    };
    let (payoff, _, _) = match measure_probability_objective(
        system,
        claim,
        kind,
        document_limits,
        cancel,
        &mut meter,
        &mut storage,
    ) {
        Ok(selected) => selected,
        Err(error) => return document_failure(error, subject),
    };
    let terminal = match product
        .states
        .iter()
        .map(|state| {
            payoff
                .terminal(
                    state.monitor.ok_or(ProbabilityDocumentError::Malformed)?,
                    state
                        .accumulator
                        .as_ref()
                        .ok_or(ProbabilityDocumentError::Malformed)?,
                    &mut meter,
                )
                .map_err(ProbabilityDocumentError::from)
        })
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(terminal) => terminal,
        Err(error) => return document_failure(error, subject),
    };
    let scope = match claim.scheduler {
        ProbScheduler::Every => WitnessScope::EveryScheduler,
        ProbScheduler::Workload(_) => WitnessScope::Workload,
    };
    let mut issue = None;
    let mut control = |work: WitnessWork<'_>| {
        witness_payload_bytes(work, system, |target| {
            for state in &product.states {
                witness_charge(&mut meter, &mut storage, 0, limits, cancel)?;
                if state == target && state.pending.is_none() {
                    let key = state
                        .certificate_state
                        .as_ref()
                        .ok_or_else(|| Box::new(ExactProbBuildIssue::BrokenInvariant))?;
                    return Ok(std::mem::size_of_val(state).saturating_add(key.as_bytes().len()));
                }
            }
            Err(Box::new(ExactProbBuildIssue::BrokenInvariant))
        })
        .and_then(|bytes| witness_charge(&mut meter, &mut storage, bytes, limits, cancel))
        .map_err(|failure| {
            issue = Some(failure);
            WitnessError::InvalidState
        })
    };
    let produced = match &claim.form {
        ProbForm::Quantile { q, threshold, .. } => {
            let strict = matches!(threshold, Bound::AtLeast(_));
            let never = product
                .states
                .iter()
                .map(|state| {
                    matches!(
                        state.accumulator,
                        Some(
                            fsl_eval::probabilistic::Accumulator::Steps {
                                activated: false,
                                ..
                            } | fsl_eval::probabilistic::Accumulator::Reward {
                                activated: false,
                                ..
                            }
                        )
                    ) && state
                        .monitor
                        .is_some_and(fsl_eval::probabilistic::MonitorState::is_decided)
                })
                .collect::<Vec<_>>();
            let accepting = terminal
                .iter()
                .enumerate()
                .map(|(i, value)| {
                    never[i]
                        || value.as_ref().is_some_and(|value| {
                            if strict {
                                value == &Rational::from_integer(1_i64.into())
                            } else {
                                value.is_zero() && !never[i]
                            }
                        })
                })
                .collect::<Vec<_>>();
            produce_quantile_witness(
                product,
                policy,
                scope,
                initial,
                &accepting,
                &never,
                q,
                strict,
                limits,
                &mut control,
            )
        }
        ProbForm::Probability { bound, .. } => {
            let accepting = terminal
                .iter()
                .map(|value| {
                    value.as_ref().is_some_and(|value| match bound {
                        Bound::AtLeast(_) => value.is_zero(),
                        Bound::AtMost(_) => value == &Rational::from_integer(1_i64.into()),
                    })
                })
                .collect::<Vec<_>>();
            let past = match bound {
                Bound::AtLeast(value) => Rational::from_integer(1_i64.into()).sub(value),
                Bound::AtMost(value) => value.clone(),
            };
            produce_witness(product, policy, scope, initial, &accepting, &past, limits)
        }
        _ => return not_yet(),
    };
    if let Some(issue) = issue {
        return settled(&issue, subject);
    }
    let produced = match produced {
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
    replay_produced_witness(
        system, claim, product, produced, initial, binding, None, limits, cancel, subject, tamper,
        meter, storage,
    )
}

fn induction_failure(error: QualifiedFhError, cancel: &Cancel) -> ProbabilityDocumentError {
    if let Some(cause) = cancel.tripped() {
        return ProbabilityDocumentError::Cancelled(cause);
    }
    match error {
        QualifiedFhError::Accounting(error) => ProbabilityDocumentError::Accounting(error),
        QualifiedFhError::Storage { bound, actual } => ProbabilityDocumentError::Limit {
            limit: fsl_replay::ProbabilityDocumentLimit::StorageBytes,
            bound,
            actual,
        },
        QualifiedFhError::RationalBits { bound, actual } => ProbabilityDocumentError::Limit {
            limit: fsl_replay::ProbabilityDocumentLimit::RationalBits,
            bound,
            actual,
        },
        QualifiedFhError::Graph(error) => match error {
            FhError::Cycle => ProbabilityDocumentError::Graph(ExactProbabilityGraphError::Cycle),
            FhError::InvalidState
            | FhError::MissingAction
            | FhError::InvalidProbability
            | FhError::InvalidPrecision
            | FhError::PolicyIterationLimit
            | FhError::MaxWorkingBytes
            | FhError::Cancelled(_) => ProbabilityDocumentError::Malformed,
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn refute_mean(
    system: &ModelSystem<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    product: &ProbProduct,
    policy: &[Option<usize>],
    initial: crate::exact_probabilistic::ProbStateId,
    binding: Option<&ObjectReference>,
    certificate: &ExactProbabilityCertificate,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
    tamper: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    let item = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(item) => item,
        Err(_) => return broken(),
    };
    let Some(package) = system.source_package() else {
        return not_yet();
    };
    let document_limits = ProbabilityDocumentLimits::for_exact(limits, system.meter_limits());
    let document = match measure_probability_certificate_document(
        system,
        package.graph().probabilistic_model().claims(),
        &item,
        certificate,
        MeasurePayoffKind::Fraction,
        document_limits,
        cancel,
    ) {
        Ok(document) => document,
        Err(error) => return document_failure(error, subject),
    };
    let subsystem = match fsl_replay::ProbabilitySubsystemCertificate::from_document(
        &document,
        document_limits,
        cancel,
    ) {
        Ok(subsystem) => subsystem,
        Err(error) => return document_failure(error, subject),
    };
    let key_of = |state: &crate::exact_probabilistic::ProbKey| {
        state
            .certificate_state
            .clone()
            .ok_or(ProbabilityDocumentError::Malformed)
    };
    let mut scheduler = Vec::new();
    if matches!(claim.scheduler, ProbScheduler::Every) {
        for state in product.ids() {
            let Some(action) = crate::exact_probabilistic::policy_action(product, state, policy)
            else {
                continue;
            };
            let Some(node) = product.states.get(state.index()) else {
                return broken();
            };
            let key = match key_of(node) {
                Ok(key) => key,
                Err(error) => return document_failure(error, subject),
            };
            let action = match canonical_action(system, product, &action, &key_of) {
                Ok(action) => action,
                Err(error) => return document_failure(error, subject),
            };
            scheduler.push((key, action));
        }
    }
    scheduler.sort_by(|a, b| a.0.cmp(&b.0));
    let initials = system.initial();
    let [first] = initials.as_slice() else {
        return not_yet();
    };
    let initial_key = match product
        .states
        .get(initial.index())
        .and_then(|state| state.certificate_state.clone())
    {
        Some(key) => key,
        None => return broken(),
    };
    let mut witness = ProbabilisticCounterexample {
        item: item.clone(),
        binding: binding.cloned(),
        instance: None,
        initial: first.clone(),
        initial_key,
        scheduler: scheduler
            .into_iter()
            .map(|(key, action)| {
                (
                    key,
                    fsl_replay::ProbabilisticSchedulerChoice::Deterministic(action),
                )
            })
            .collect(),
        paths: Vec::new(),
        subsystem: Some(Box::new(subsystem)),
    };
    if let Some(tamper) = tamper {
        tamper(&mut witness);
    }
    match replay_probabilistic_witness(package, system, &item, &witness, limits, cancel) {
        Ok(ProbabilityWitnessReplay::Reproduced(token)) => {
            let ProbForm::MeanOfFraction { bound, .. } = &claim.form else {
                return broken();
            };
            let mut spelled = Vec::new();
            for (key, action) in &token.evidence().scheduler {
                let fsl_replay::ProbabilisticSchedulerChoice::Deterministic(
                    fsl_replay::ProductAction::Identity(schedule),
                ) = action
                else {
                    continue;
                };
                let Some(state) = product.ids().find(|state| {
                    product
                        .states
                        .get(state.index())
                        .and_then(|state| state.certificate_state.as_ref())
                        == Some(key)
                }) else {
                    return broken();
                };
                let identity = product.outgoing(state).iter().find_map(|edge| {
                    let crate::exact_probabilistic::ProductAction::Identity(identity) =
                        &edge.choice
                    else {
                        return None;
                    };
                    (fsl_replay::Schedule::of(system, identity).as_ref() == Ok(schedule))
                        .then_some(identity)
                });
                let Some(identity) = identity else {
                    return broken();
                };
                spelled.push((
                    fsl_eval::simulation::StateKey::from_canonical_bytes(key.as_bytes().to_vec()),
                    identity.clone(),
                ));
            }
            let scope = package.graph().scope();
            let Some(claim_type) = super::probabilistic_bound_type(&claim.form) else {
                return broken();
            };
            let wire = fsl_replay::probabilistic_wire(
                token.evidence(),
                bound,
                claim_type,
                &fsl_replay::ProbabilisticValueSchema::new(
                    scope.types(),
                    scope.variant_types(),
                    scope.enums(),
                ),
                &|state| system.state_key(state).ok().map(|(_, key)| key),
                &spelled,
            );
            EngineReport::Exact(Box::new(ExactReport::Refuted(token, Box::new(wire))))
        }
        Ok(ProbabilityWitnessReplay::Parity) => {
            EngineReport::Exact(Box::new(ExactReport::ReplayParity))
        }
        Err(error) => undefined::failure(error, subject, limits),
    }
}
