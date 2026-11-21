// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared-control finite-horizon induction and independently accepted proofs.
use super::*;
use crate::exact_probabilistic::{backward_induction, FhObjective};
use fsl_replay::{
    conditional_measure_probability_certificate, measure_probability_certificate_document,
    measure_probability_objective, measure_probability_product_key,
    write_measure_probability_certificate, MeasurePayoffKind,
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
    let system = match ModelSystem::new_with_cancel(simulation.clone(), cancel) {
        Ok(system) => system,
        Err(error) => return build_refusal(ProductRefusal::Admission(error), subject),
    };
    let Some(package) = system.source_package() else {
        return not_yet();
    };
    let claims = package.graph().probabilistic_model().claims();
    let item = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(item) => item,
        Err(_) => return not_yet(),
    };
    let binding = match claim_binding(&system, claim) {
        Ok(binding) => binding,
        Err(error) => return document_failure(error.into(), subject),
    };
    let reference = binding.as_ref().map(|binding| &binding.reference);
    let document_limits = ProbabilityDocumentLimits {
        bytes: limits.document.bytes,
        storage_bytes: limits.document.storage_bytes,
        states: limits.model.limits.max_states,
        edges: limits.model.limits.max_transitions,
        scalar_bits: limits.max_rational_bits,
        accounting: scalar,
    };
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
    let (mut product, _) = match build_retaining_monitor(request, cancel) {
        Ok((ProbBuild::Built(product), monitor)) => (product, monitor),
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
            )
        }
        Err(error) => return build_refusal(error, subject),
    };
    let [(_, _, initial)] = product.initial.as_slice() else {
        return not_yet();
    };
    let initial = *initial;
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
                .map_err(scalar_failure)
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
    let threshold = match &claim.form {
        ProbForm::Probability {
            bound: Bound::AtLeast(value) | Bound::AtMost(value),
            ..
        }
        | ProbForm::MeanOfFraction {
            bound: Bound::AtLeast(value) | Bound::AtMost(value),
            ..
        } => value.clone(),
        ProbForm::Quantile { .. } => Rational::from_integer(0_i64.into()),
        _ => return not_yet(),
    };
    let (values, policy) = match backward_induction(
        &product,
        &terminal,
        objective,
        &threshold,
        limits.max_rational_bits,
        limits.precision_bits,
        limits.max_precision_bits,
    ) {
        Ok(FhResult::Exact { values, policy }) => (values, policy),
        Ok(FhResult::Bounds { .. } | FhResult::PrecisionBudget { .. }) => return not_yet(),
        Err(_) => return broken(),
    };
    for state in &mut product.states {
        if state.pending.is_some() {
            return not_yet();
        }
        let Some(control) = state.monitor else {
            return broken();
        };
        let Some(sums) = state.accumulator.as_ref() else {
            return broken();
        };
        let key = match measure_probability_product_key(
            reference,
            &state.model,
            control,
            sums,
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
                        .map_err(scalar_failure)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let denominator = match backward_induction(
                &product,
                &terminal,
                objective,
                &threshold,
                limits.max_rational_bits,
                limits.precision_bits,
                limits.max_precision_bits,
            )
            .map_err(|_| ProbabilityDocumentError::Malformed)?
            {
                FhResult::Exact { values, .. } => values,
                _ => return Err(ProbabilityDocumentError::Unsupported),
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
    let _ = (initial, policy);
    let extremum = match optimization {
        ProbabilityObjective::Minimum => Some(Extremum::Min),
        ProbabilityObjective::Maximum => Some(Extremum::Max),
        ProbabilityObjective::Workload => None,
    };
    checked_report(checked, certificate, binding, extremum, subject)
}
fn scalar_failure(error: fsl_eval::probabilistic::MeasureError) -> ProbabilityDocumentError {
    match error {
        fsl_eval::probabilistic::MeasureError::Accounting(error) => {
            ProbabilityDocumentError::Accounting(error)
        }
        fsl_eval::probabilistic::MeasureError::Broken(_) => ProbabilityDocumentError::Malformed,
    }
}
