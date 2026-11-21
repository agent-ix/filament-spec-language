// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lower and upper fraction expectations in the mean-refutation role.
use super::*;

const PROFILE: &str = "filament.probability-objective/value-bounds/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireBoundObjective {
    profile: String,
    base: WireObjective,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WirePolicy {
    state: String,
    choice: Box<serde_json::value::RawValue>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireBound {
    identity: WireIdentity,
    objective: WireBoundObjective,
    kind: String,
    values: Vec<WireValue>,
    ranking: Option<Vec<WireRank>>,
    policy: Option<Vec<WirePolicy>>,
}

pub(super) fn is_bound(bytes: &[u8]) -> Result<bool, ProbabilityDocumentError> {
    #[derive(Deserialize)]
    struct Dispatch<'a> {
        #[serde(borrow)]
        kind: &'a str,
    }
    let dispatch: Dispatch<'_> =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    Ok(matches!(dispatch.kind, "lower" | "upper"))
}

pub(super) fn admit(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    storage: &mut u64,
) -> Result<(), ProbabilityDocumentError> {
    poll(cancel)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::Bytes,
            limits.bytes,
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        ));
    }
    let admission = RefCell::new(writer::Admission::with_used(
        limits.storage_bytes,
        cancel,
        *storage,
    ));
    let parsed = quire_canonical::read_with_reservation(
        bytes,
        limits.bytes,
        &mut writer::Reserve(&admission),
    );
    *storage = admission.borrow().used();
    if let Some(error) = admission.borrow_mut().take_stop() {
        return Err(error);
    }
    let parsed = parsed.map_err(|_| ProbabilityDocumentError::Malformed)?;
    let canonical = writer::encode_reserved(&parsed, limits.bytes, &admission)?;
    *storage = admission.borrow().used();
    if canonical != bytes {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    Ok(())
}

pub(in crate::probability_certificate) fn admit_subsystem(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    poll(cancel)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::Bytes,
            limits.bytes,
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        ));
    }
    if !is_bound(bytes)? {
        read_measure_probability_certificate(bytes, limits, cancel)?;
        return Ok(());
    }
    let mut storage = 0;
    admit(bytes, limits, cancel, &mut storage)?;
    let wire: WireBound =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    if wire.objective.profile != PROFILE {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let document = read_wire(
        WireDocument {
            identity: wire.identity,
            objective: wire.objective.base,
            kind: "exact".into(),
            values: wire.values,
            ranking: wire.ranking.unwrap_or_default(),
            conditional: None,
        },
        limits,
        cancel,
        &mut meter,
        &mut storage,
        0,
    )?;
    validate_ranges(&document, limits, cancel, &mut meter)
}

struct Transition<'a> {
    schedule: quire_canonical::NodeRef<'a>,
    operation: &'a str,
}
impl Encode for Transition<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        let Node::Array(parts) = self.schedule.node() else {
            return Err(Error::ReservationDenied { requested: 0 });
        };
        let parts = parts.collect::<Vec<_>>();
        let [_operation, receiver, arguments] = parts.as_slice() else {
            return Err(Error::ReservationDenied { requested: 0 });
        };
        writer.begin_object()?;
        writer.name("type")?;
        writer.string("transition")?;
        writer.name("operation")?;
        writer.string(self.operation)?;
        writer.name("receiver")?;
        receiver.encode_into(writer)?;
        writer.name("arguments")?;
        arguments.encode_into(writer)?;
        writer.end_object()
    }
}

fn policy_action(
    choice: &str,
    state: &State,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    storage: &mut u64,
) -> Result<Option<usize>, ProbabilityDocumentError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct TransitionShape {
        #[serde(rename = "type")]
        kind: String,
        operation: String,
        receiver: Box<serde_json::value::RawValue>,
        arguments: Vec<Box<serde_json::value::RawValue>>,
    }
    let shape: TransitionShape = serde_json::from_str(choice)
        .map_err(|_| ProbabilityDocumentError::MalformedMember("policy"))?;
    if shape.kind != "transition"
        || shape.operation.is_empty()
        || !(shape.receiver.get() == "null" || shape.receiver.get().starts_with('{'))
        || shape
            .arguments
            .iter()
            .any(|value| !value.get().starts_with('{'))
    {
        return Err(ProbabilityDocumentError::MalformedMember("policy"));
    }
    let admission = RefCell::new(writer::Admission::with_used(
        limits.storage_bytes,
        cancel,
        *storage,
    ));
    for (index, action) in state.actions.iter().enumerate() {
        poll(cancel)?;
        let Some(crate::ProductAction::Identity(schedule)) = action else {
            continue;
        };
        let parsed = quire_canonical::read_with_reservation(
            schedule.as_bytes(),
            limits.bytes,
            &mut writer::Reserve(&admission),
        );
        *storage = admission.borrow().used();
        if let Some(error) = admission.borrow_mut().take_stop() {
            return Err(error);
        }
        let parsed = parsed.map_err(|_| ProbabilityDocumentError::MalformedMember("policy"))?;
        let Node::Array(parts) = parsed.root().node() else {
            return Err(ProbabilityDocumentError::MalformedMember("policy"));
        };
        let operation = parts
            .into_iter()
            .next()
            .ok_or(ProbabilityDocumentError::MalformedMember("policy"))?;
        let operation = writer::encode_reserved(&operation, limits.bytes, &admission)?;
        let operation = std::str::from_utf8(&operation)
            .map_err(|_| ProbabilityDocumentError::MalformedMember("policy"))?;
        let expected = writer::encode_reserved(
            &Transition {
                schedule: parsed.root(),
                operation,
            },
            limits.bytes,
            &admission,
        )?;
        *storage = admission.borrow().used();
        if expected == choice.as_bytes() {
            return Ok(Some(index));
        }
    }
    *storage = admission.borrow().used();
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn check(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    item: &ObligationIdentity,
    bytes: &[u8],
    scheduler: &[(ProbabilityProductKey, crate::ProductAction)],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<MeanRefutationCheck, ProbabilityDocumentError> {
    let mut storage = 0;
    admit(bytes, limits, cancel, &mut storage)?;
    charge(
        &mut storage,
        limits.storage_bytes,
        u64::try_from(bytes.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(4),
    )?;
    let wire: WireBound =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    if wire.objective.profile != PROFILE {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let lower = match wire.kind.as_str() {
        "lower" => true,
        "upper" => false,
        _ => return Err(ProbabilityDocumentError::MalformedMember("kind")),
    };
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let selected = plan(
        system,
        claim,
        MeasurePayoffKind::Fraction,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    // A refutation bound has the opposite direction to a proof of the claim.
    if !matches!(
        (&selected.bound, lower),
        (Bound::AtLeast(_), false) | (Bound::AtMost(_), true)
    ) {
        return Err(ProbabilityDocumentError::MalformedMember("kind"));
    }
    let needs_policy = matches!(
        (selected.objective.optimization, lower),
        (ProbabilityObjective::Minimum, false) | (ProbabilityObjective::Maximum, true)
    );
    if wire.ranking.is_some() != lower || (wire.policy.is_some() && !needs_policy) {
        return Err(ProbabilityDocumentError::MalformedMember(
            if wire.ranking.is_some() != lower {
                "ranking"
            } else {
                "policy"
            },
        ));
    }
    let document = read_wire(
        WireDocument {
            identity: wire.identity,
            objective: wire.objective.base,
            kind: "exact".into(),
            values: wire.values,
            ranking: wire.ranking.unwrap_or_default(),
            conditional: None,
        },
        limits,
        cancel,
        &mut meter,
        &mut storage,
        0,
    )?;
    if document.identity != *item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    if document.payoff != MeasurePayoffKind::Fraction {
        return Err(ProbabilityDocumentError::MalformedMember("kind"));
    }
    if document.objective != selected.objective {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    validate_ranges(&document, limits, cancel, &mut meter)?;
    let graph = enumerate(
        system,
        claim,
        &selected,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    let rejection = |rule, key| {
        MeanRefutationCheck::Rejected(ProbabilityCertificateRejection {
            rule,
            at: ProbabilityCertificateLocus::ProductState(key),
        })
    };
    for row in &document.values {
        charge(
            &mut storage,
            limits.storage_bytes,
            u64::try_from(
                std::mem::size_of::<ExactProbabilityValue>()
                    .saturating_add(row.state.as_bytes().len()),
            )
            .unwrap_or(u64::MAX),
        )?;
    }
    let values = document
        .values
        .iter()
        .map(|row| (row.state.clone(), row.value.clone()))
        .collect::<BTreeMap<_, _>>();
    if !values.contains_key(&graph.initial)
        || values.keys().any(|key| !graph.states.contains_key(key))
        || (!lower && values.len() != graph.states.len())
    {
        return Err(ProbabilityDocumentError::MalformedMember("values"));
    }
    if lower && document.ranking.iter().map(|row| &row.0).ne(values.keys()) {
        return Err(ProbabilityDocumentError::MalformedMember("ranking"));
    }
    let mut policy = BTreeMap::new();
    for row in wire.policy.unwrap_or_default() {
        let key = decode_state(&row.state, limits, cancel, &mut storage)?;
        if policy
            .last_key_value()
            .is_some_and(|(last, _)| last >= &key)
            || !values.contains_key(&key)
        {
            return Err(ProbabilityDocumentError::MalformedMember("policy"));
        }
        let state = graph
            .states
            .get(&key)
            .ok_or(ProbabilityDocumentError::MalformedMember("policy"))?;
        if state.terminal.is_some() {
            return Err(ProbabilityDocumentError::MalformedMember("policy"));
        }
        let Some(index) = policy_action(row.choice.get(), state, limits, cancel, &mut storage)?
        else {
            return Ok(rejection(ProbabilityCertificateRule::InequalityFails, key));
        };
        policy.insert(key, index);
    }
    let zero = super::super::rational(0);
    let mut selected_policy = Vec::new();
    for (key, value) in &values {
        poll(cancel)?;
        let state = graph
            .states
            .get(key)
            .ok_or(ProbabilityDocumentError::MalformedMember("values"))?;
        if lower
            && document
                .ranking
                .iter()
                .find(|row| &row.0 == key)
                .map(|row| row.1)
                != Some(state.rank)
        {
            return Ok(rejection(
                ProbabilityCertificateRule::RankBroken,
                key.clone(),
            ));
        }
        if let Some(terminal) = &state.terminal {
            if value != terminal {
                return Ok(rejection(
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                ));
            }
            continue;
        }
        charge(
            &mut storage,
            limits.storage_bytes,
            u64::try_from(state.actions.len().saturating_mul(std::mem::size_of::<(
                &str,
                &[u8],
                usize,
            )>()))
            .unwrap_or(u64::MAX),
        )?;
        let mut choices = Vec::new();
        for (index, action) in state.actions.iter().enumerate() {
            let (tag, bytes): (&str, &[u8]) = match action {
                None => ("", &[]),
                Some(crate::ProductAction::Identity(schedule)) => ("identity", schedule.as_bytes()),
                Some(crate::ProductAction::PostState(target)) => ("post-state", target.as_bytes()),
                Some(crate::ProductAction::TerminalStutter) => ("terminal-stutter", &[]),
                Some(_) => return Err(ProbabilityDocumentError::Unsupported),
            };
            choices.push((tag, bytes, index));
        }
        choices.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
        let mut satisfying = None;
        for (_, _, index) in choices {
            if policy.get(key).is_some_and(|hint| *hint != index) {
                continue;
            }
            let choice = state
                .choices
                .get(index)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let mut sum = super::super::rational(0);
            for edge in choice {
                let target = values.get(&edge.target).unwrap_or(&zero);
                let term = arithmetic(
                    filament_exact::RationalArithmetic::Multiply(&edge.probability, target),
                    &mut meter,
                    cancel,
                )?;
                sum = arithmetic(
                    filament_exact::RationalArithmetic::Add(&sum, &term),
                    &mut meter,
                    cancel,
                )?;
            }
            let holds = if lower { value <= &sum } else { &sum <= value };
            if !holds && (!needs_policy || policy.contains_key(key)) {
                return Ok(rejection(
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                ));
            }
            if holds && needs_policy {
                satisfying = Some(index);
                break;
            }
        }
        if needs_policy {
            let Some(index) = satisfying else {
                return Ok(rejection(
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                ));
            };
            let action = state
                .actions
                .get(index)
                .and_then(Option::as_ref)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let action_bytes = match action {
                crate::ProductAction::Identity(schedule) => schedule.as_bytes().len(),
                crate::ProductAction::PostState(target) => target.as_bytes().len(),
                crate::ProductAction::TerminalStutter => 0,
                _ => return Err(ProbabilityDocumentError::Unsupported),
            };
            charge(
                &mut storage,
                limits.storage_bytes,
                u64::try_from(
                    key.as_bytes()
                        .len()
                        .saturating_add(action_bytes)
                        .saturating_add(std::mem::size_of::<(
                            ProbabilityProductKey,
                            crate::ProductAction,
                        )>()),
                )
                .unwrap_or(u64::MAX),
            )?;
            selected_policy.push((key.clone(), action.clone()));
        }
    }
    // The witness and certificate must describe the same scheduler wherever
    // the certificate chooses a semantic action.
    if selected.objective.optimization == ProbabilityObjective::Workload {
        if !scheduler.is_empty() {
            return Ok(MeanRefutationCheck::PolicyParity);
        }
    } else {
        let resolved = selected_policy
            .iter()
            .map(|(key, action)| (key, action))
            .collect::<BTreeMap<_, _>>();
        for (key, action) in scheduler {
            let Some(state) = graph.states.get(key) else {
                return Ok(MeanRefutationCheck::PolicyParity);
            };
            if !state
                .actions
                .iter()
                .any(|candidate| candidate.as_ref() == Some(action))
            {
                return Ok(MeanRefutationCheck::PolicyParity);
            }
            if resolved
                .get(key)
                .is_some_and(|selected| *selected != action)
            {
                return Ok(MeanRefutationCheck::PolicyParity);
            }
        }
    }
    let value = values
        .get(&graph.initial)
        .ok_or(ProbabilityDocumentError::MalformedMember("values"))?;
    if !match &selected.bound {
        Bound::AtLeast(threshold) => value < threshold,
        Bound::AtMost(threshold) => value > threshold,
    } {
        return Ok(rejection(
            ProbabilityCertificateRule::ThresholdMissed,
            graph.initial,
        ));
    }
    poll(cancel)?;
    Ok(MeanRefutationCheck::AcceptedBounds(selected_policy))
}
