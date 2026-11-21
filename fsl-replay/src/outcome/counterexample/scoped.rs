// SPDX-License-Identifier: AGPL-3.0-or-later
//! Full carrier construction under the original reference/resource scope.

use super::*;
use crate::{ReferenceCodecError, ReferenceScope};

impl ReferenceScope<'_, '_, '_, '_> {
    /// Write a complete protocol carrier from the original private runtime
    /// fields, including send/blocked/fairness data. The optional terminal key
    /// is the genuine retained final key and introduces no wire member.
    ///
    /// # Errors
    /// Preserves typed reference, metadata, cancellation and owner denials.
    pub fn protocol_temporal_wire(
        &mut self,
        counterexample: &TemporalCounterexample<ProtocolTransition>,
        subject: &ProtocolWire,
        claim: Option<&OccurrenceKey>,
        terminal_key: Option<&fsl_eval::simulation::StateKey>,
    ) -> Result<Json, ReferenceCodecError> {
        use fsl_eval::simulation::protocol::ProtocolWireSink;
        let mut out = self.temporal_full(
            counterexample,
            Some(&subject.over),
            claim,
            terminal_key,
            false,
            |s, transition| transition.write_borrowed(s),
            |s, transition| {
                transition
                    .send_record()
                    .map(|record| record.write_borrowed(s))
                    .transpose()
            },
            |s, fairness| fairness.write_borrowed(s),
        )?;
        self.field(&mut out, "subject", |s| {
            let tag = s.text("protocol-subject")?;
            let protocol = s.text(&subject.protocol)?;
            let over = s.at_pointer("/over", |s| s.encode_reference(&subject.over))?;
            let mut roles = s.record([])?;
            for (role, model) in &subject.roles {
                s.field(&mut roles, role, |s| s.text(model))?;
            }
            let mut out = s.record([
                ("type", tag),
                ("protocol", protocol),
                ("over", over),
                ("roles", roles),
            ])?;
            if !subject.capacities.is_empty() {
                s.field(&mut out, "capacities", |s| {
                    let mut out = s.record([])?;
                    for (name, value) in &subject.capacities {
                        s.field(&mut out, name, |s| s.display(value))?;
                    }
                    Ok(out)
                })?;
            }
            Ok(out)
        })?;
        if let Some(bound) = subject.max_live_instances {
            self.put(&mut out, "max_live_instances", Json::from(bound))?;
        }
        if matches!(counterexample.kind, CounterexampleKind::Deadlock) {
            self.field(&mut out, "blocked", |s| {
                let mut out = Json::Array(ReferenceScope::array(s, counterexample.blocked.len())?);
                for (index, blocked) in counterexample.blocked.iter().enumerate() {
                    s.element(&mut out, index, |s| blocked.write_borrowed(s))?;
                }
                Ok(out)
            })?;
        }
        Ok(out)
    }

    /// Write a task-scheduler identity with both source arms converted by the
    /// shared codec and exact scalar metadata charged before materialization.
    ///
    /// # Errors
    /// Preserves typed reference, decimal, resource and cancellation failures.
    pub fn task_transition_wire(
        &mut self,
        identity: &TaskTransitionIdentity,
    ) -> Result<Json, ReferenceCodecError> {
        let tag = self.text("task-transition")?;
        let action = self.text(crate::certificate::task_product::action_name(
            identity.action,
        ))?;
        let task_set = self.text(&identity.task_set)?;
        let task = self.text(&identity.task)?;
        let release =
            self.at_pointer("/release", |s| s.encode_task_transition(&identity.release))?;
        let source = match &identity.source_transition {
            Some(source) => {
                self.at_pointer("/source_transition", |s| s.encode_task_transition(source))?
            }
            None => Json::Null,
        };
        let demand = self.at_pointer("/demand", |s| s.rational(&identity.demand))?;
        let jitter = self.at_pointer("/jitter", |s| s.rational(&identity.jitter))?;
        let deadline = self.at_pointer("/deadline", |s| s.rational(&identity.deadline))?;
        let mut out = self.record([
            ("type", tag),
            ("action", action),
            ("taskset", task_set),
            ("task", task),
            ("job", Json::from(identity.job)),
            ("slot", Json::from(identity.slot)),
            ("generation", Json::from(identity.generation)),
            ("release", release),
            ("source_transition", source),
            ("demand", demand),
            ("jitter", jitter),
            ("deadline", deadline),
        ])?;
        if let Some(priority) = identity.priority {
            self.put(&mut out, "priority", Json::from(priority))?;
        }
        if let Some(delta) = &identity.budget_delta {
            let delta = self.at_pointer("/budget_delta", |s| s.rational(delta))?;
            self.put(&mut out, "budget_delta", delta)?;
        }
        Ok(out)
    }

    /// Write the task-product alphabet with its authoritative source adapter.
    ///
    /// # Errors
    /// Preserves reference and caller resource/cancellation failures.
    pub fn task_product_transition_wire(
        &mut self,
        transition: &TaskProductTransition,
    ) -> Result<Json, ReferenceCodecError> {
        match transition {
            TaskProductTransition::Source(source) => self.encode_task_transition(source),
            TaskProductTransition::Task(task) => self.task_transition_wire(task),
            TaskProductTransition::TerminalStutter => {
                let tag = self.text("terminal-stutter")?;
                self.record([("type", tag)])
            }
        }
    }

    /// Write a full task temporal carrier, retaining a genuine optional final
    /// key for an initial-state terminal-stutter loop.
    ///
    /// # Errors
    /// Refuses absent terminal state, invalid delays and conversion/resource failures.
    pub fn task_temporal_wire(
        &mut self,
        counterexample: &TemporalCounterexample<TaskProductTransition>,
        over: Option<&ObjectReference>,
        claim: Option<&OccurrenceKey>,
        terminal_key: Option<&fsl_eval::simulation::StateKey>,
    ) -> Result<Json, ReferenceCodecError> {
        self.temporal_full(
            counterexample,
            over,
            claim,
            terminal_key,
            true,
            |s, transition| s.task_product_transition_wire(transition),
            |_, _| Ok(None),
            |s, fairness| s.fairness_wire(fairness),
        )
    }

    fn claim_locus(&mut self, claim: Option<&OccurrenceKey>) -> Result<Json, ReferenceCodecError> {
        let claim = claim.ok_or_else(|| self.carrier_refusal("undefined locus"))?;
        let node = self.at_pointer("/node", |s| s.node(claim.node()))?;
        let role = self.display(&claim.origin().role())?;
        let occurrence = self.record([
            ("role", role),
            ("ordinal", Json::from(claim.origin().ordinal())),
        ])?;
        self.record([("node", node), ("occurrence", occurrence)])
    }

    fn mapping_row_text(
        &mut self,
        row: &crate::refinement::MappingRowRef,
    ) -> Result<Json, ReferenceCodecError> {
        use crate::refinement::{MappingRowRef, StepArgument};
        match row {
            MappingRowRef::Field { object_type, field } => {
                let object = self.declaration_text(object_type)?;
                let field = self.declaration_text(field)?;
                self.display(&format_args!("field {object} {field}"))
            }
            MappingRowRef::Object { concrete_type } => {
                let object = self.declaration_text(concrete_type)?;
                self.display(&format_args!("object {object}"))
            }
            MappingRowRef::Argument {
                operation,
                argument,
            } => {
                let operation = self.declaration_text(operation)?;
                match argument {
                    StepArgument::Receiver => {
                        self.display(&format_args!("argument {operation} receiver"))
                    }
                    StepArgument::Parameter(position) => {
                        self.display(&format_args!("argument {operation} parameter {position}"))
                    }
                }
            }
            MappingRowRef::History {
                object_type,
                field,
                operation,
            } => {
                let object = self.declaration_text(object_type)?;
                let operation = self.declaration_text(operation)?;
                self.display(&format_args!("history {object} {field} {operation}"))
            }
        }
    }

    pub(super) fn undefined_full(
        &mut self,
        evaluation: &UndefinedEvaluation,
        claim: Option<&OccurrenceKey>,
    ) -> Result<Json, ReferenceCodecError> {
        let site = &evaluation.site;
        let state = self.at_pointer("/where/state_key", |s| s.state(&site.state_key))?;
        let locus = self.at_pointer("/where/locus", |s| s.claim_locus(claim))?;
        let mut at = self.record([
            ("position", Json::from(site.position)),
            ("state_key", state),
            ("locus", locus),
        ])?;
        if let Some(guard) = &site.transition {
            let guard = self.at_pointer("/where/transition", |s| s.encode_transition(guard))?;
            self.put(&mut at, "transition", guard)?;
        }
        if let UndefinedLocus::Mapping(mapping) = &site.locus {
            let row = self.at_pointer("/where/row", |s| s.mapping_row_text(&mapping.row))?;
            self.put(&mut at, "row", row)?;
        }
        let tag = self.text("undefined-evaluation")?;
        let cause = self.text(undefined_reason(&evaluation.cause))?;
        self.record([("type", tag), ("where", at), ("cause", cause)])
    }

    fn race_full(&mut self, race: &Race) -> Result<Json, ReferenceCodecError> {
        use fsl_eval::simulation::Location;
        let tag = self.text("location")?;
        let location = match &race.location {
            Location::Field { object, field } => {
                self.work(field.node.len())?;
                let name = field
                    .node
                    .rsplit_once('/')
                    .map_or(field.node.as_str(), |(_, name)| name);
                let object = self.at_pointer("/location/object", |s| s.encode_reference(object))?;
                let field = self.text(name)?;
                self.record([("type", tag), ("object", object), ("field", field)])?
            }
            Location::AnyField { field } => {
                self.work(field.node.len())?;
                let (owner, name) = field
                    .node
                    .rsplit_once('/')
                    .unwrap_or((&field.node, &field.node));
                let owner = self.text(owner)?;
                let field = self.text(name)?;
                self.record([("type", tag), ("object_type", owner), ("field", field)])?
            }
            Location::Membership { population, .. } | Location::AnyMembership { population } => {
                let population = self.text(&population.node)?;
                self.record([("type", tag), ("population", population)])?
            }
            Location::Protocol(_) => return Err(self.carrier_refusal("race location")),
        };
        let earlier = self.display(&race.earlier)?;
        let later = self.display(&race.later)?;
        self.record([
            ("location", location),
            ("earlier", earlier),
            ("later", later),
        ])
    }

    fn refinement_full(
        &mut self,
        failure: &RefinementFailure,
    ) -> Result<Json, ReferenceCodecError> {
        use crate::refinement::TerminalSuccessDirection;
        let (kind, mut out) = match failure {
            RefinementFailure::InitialNotAbstract { initial } => (
                "InitialNotAbstract",
                self.record([("initial", Json::from(*initial))])?,
            ),
            RefinementFailure::StutterChanged { position } => (
                "StutterChanged",
                self.record([("position", Json::from(*position))])?,
            ),
            RefinementFailure::NoAbstractMatch { position } => (
                "NoAbstractMatch",
                self.record([("position", Json::from(*position))])?,
            ),
            RefinementFailure::AbstractClockGuard {
                position,
                operation,
            } => {
                let operation = self.text(operation)?;
                (
                    "AbstractClockGuard",
                    self.record([
                        ("position", Json::from(*position)),
                        ("operation", operation),
                    ])?,
                )
            }
            RefinementFailure::AbstractTimeInvariant { position, model } => {
                let model = self.text(model)?;
                (
                    "AbstractTimeInvariant",
                    self.record([("position", Json::from(*position)), ("model", model)])?,
                )
            }
            RefinementFailure::AbstractStepRejected {
                position,
                transition,
                cause,
            } => {
                let transition =
                    self.at_pointer("/transition", |s| s.encode_transition(transition))?;
                let cause = match cause {
                    StepRejection::Precondition { clause } => {
                        let tag = self.text("Precondition")?;
                        let clause = self.text(clause)?;
                        self.record([("type", tag), ("clause", clause)])?
                    }
                    StepRejection::Frame { code } => {
                        let tag = self.text("Frame")?;
                        let code = self.text(code.code())?;
                        self.record([("type", tag), ("code", code)])?
                    }
                    StepRejection::Postcondition => {
                        let tag = self.text("Postcondition")?;
                        self.record([("type", tag)])?
                    }
                };
                (
                    "AbstractStepRejected",
                    self.record([
                        ("position", Json::from(*position)),
                        ("transition", transition),
                        ("cause", cause),
                    ])?,
                )
            }
            RefinementFailure::Divergence { constraint, .. } => {
                let constraint = constraint
                    .as_ref()
                    .ok_or_else(|| self.carrier_refusal("divergence constraint"))?;
                let constraint = self.at_pointer("/constraint", |s| s.fairness_wire(constraint))?;
                ("Divergence", self.record([("constraint", constraint)])?)
            }
            RefinementFailure::AbstractUnfair { constraint }
            | RefinementFailure::HiddenUnfair { constraint } => {
                let kind = if matches!(failure, RefinementFailure::AbstractUnfair { .. }) {
                    "AbstractUnfair"
                } else {
                    "HiddenUnfair"
                };
                let constraint = self.at_pointer("/constraint", |s| s.fairness_wire(constraint))?;
                (kind, self.record([("constraint", constraint)])?)
            }
            RefinementFailure::RefusalNotPermitted { position, label } => {
                let step = self.text(&label.step)?;
                let mut binder = self.array(label.binder.len())?;
                for (index, entry) in label.binder.iter().enumerate() {
                    let index = self.display_text(&index)?;
                    let field = self.text(&entry.field)?;
                    let value = self.at_pointer("/label/binder/", |s| {
                        s.at_pointer(&index, |s| {
                            s.at_pointer("/value", |s| s.typed_value(&entry.value))
                        })
                    })?;
                    binder.push(self.record([("field", field), ("value", value)])?);
                }
                let label = self.record([("step", step), ("binder", Json::Array(binder))])?;
                (
                    "RefusalNotPermitted",
                    self.record([("position", Json::from(*position)), ("label", label)])?,
                )
            }
            RefinementFailure::TerminalSuccessMismatch {
                position,
                direction,
            } => {
                let direction = self.text(match direction {
                    TerminalSuccessDirection::ConcreteOnly => "concrete-only",
                    TerminalSuccessDirection::AbstractOnly => "abstract-only",
                })?;
                (
                    "TerminalSuccessMismatch",
                    self.record([
                        ("position", Json::from(*position)),
                        ("direction", direction),
                    ])?,
                )
            }
            RefinementFailure::Undefined { position, row } => {
                let row = self.at_pointer("/row", |s| s.mapping_row_text(row))?;
                (
                    "Undefined",
                    self.record([("position", Json::from(*position)), ("row", row)])?,
                )
            }
        };
        let kind = self.text(kind)?;
        self.put(&mut out, "kind", kind)?;
        Ok(out)
    }

    /// Write the complete model temporal carrier under this authentic scope.
    /// `terminal_key` must be the genuine retained final key; it is required
    /// for an empty-prefix terminal-stutter loop and adds no wire member.
    ///
    /// # Errors
    /// Retains typed metadata/reference/resource failures and refuses absent
    /// terminal keys or non-positive terminal-stutter delays.
    pub fn temporal_wire(
        &mut self,
        counterexample: &TemporalCounterexample,
        over: Option<&ObjectReference>,
        claim: Option<&OccurrenceKey>,
        terminal_key: Option<&fsl_eval::simulation::StateKey>,
    ) -> Result<Json, ReferenceCodecError> {
        self.temporal_full(
            counterexample,
            over,
            claim,
            terminal_key,
            false,
            |s, transition| s.encode_task_transition(transition),
            |_, _| Ok(None),
            |s, fairness| s.fairness_wire(fairness),
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one carrier builder retains original context and alphabet-specific encoders"
    )]
    fn temporal_full<T: SubjectTransition>(
        &mut self,
        counterexample: &TemporalCounterexample<T>,
        over: Option<&ObjectReference>,
        claim: Option<&OccurrenceKey>,
        terminal_key: Option<&fsl_eval::simulation::StateKey>,
        delay_required: bool,
        mut transition: impl FnMut(&mut Self, &T) -> Result<Json, ReferenceCodecError>,
        mut send_record: impl FnMut(&mut Self, &T) -> Result<Option<Json>, ReferenceCodecError>,
        mut fairness: impl FnMut(&mut Self, &T::Fairness) -> Result<Json, ReferenceCodecError>,
    ) -> Result<Json, ReferenceCodecError> {
        let mut race = None;
        let mut loop_proof = None;
        let kind = match &counterexample.kind {
            CounterexampleKind::Formula => self.text("formula")?,
            CounterexampleKind::Deadlock => self.text("deadlock")?,
            CounterexampleKind::TimeLock => self.text("time-lock")?,
            CounterexampleKind::UndefinedEvaluation(evaluation) => {
                self.at_pointer("/kind", |s| s.undefined_full(evaluation, claim))?
            }
            CounterexampleKind::Race(found) => {
                race = Some(self.at_pointer("/race", |s| s.race_full(found))?);
                self.text("race")?
            }
            CounterexampleKind::LoopProof(found) => {
                let obligation = self.text(match found.obligation {
                    crate::LoopProofObligation::Invariant => "loop-invariant",
                    crate::LoopProofObligation::Termination => "loop-termination",
                })?;
                let failure = self.text(match found.failure {
                    crate::LoopProofFailure::InvariantFalse => "invariant-false",
                    crate::LoopProofFailure::Undefined => "undefined",
                    crate::LoopProofFailure::VariantDidNotDecrease => "variant-did-not-decrease",
                    crate::LoopProofFailure::VariantBelowFloor => "variant-below-floor",
                })?;
                loop_proof = Some(self.record([
                    ("repeat", Json::from(found.repeat)),
                    ("obligation", obligation),
                    ("failure", failure),
                ])?);
                self.text("loop-proof")?
            }
        };
        let len = counterexample.prefix.len();
        let split = counterexample
            .lasso
            .as_ref()
            .map_or(len, |lasso| lasso.entry.min(len));
        let stutter = counterexample
            .lasso
            .as_ref()
            .is_some_and(|lasso| lasso.terminal_stutter);
        let mut stem = self.array(split)?;
        let loop_len = len - split;
        let mut looped = self.array(loop_len)?;
        for (index, step) in counterexample.prefix.iter().enumerate() {
            let (field, at) = if index < split {
                ("/prefix/", index)
            } else {
                ("/loop/", index - split)
            };
            let at = self.display_text(&at)?;
            let written = self.at_pointer(field, |s| {
                s.at_pointer(&at, |s| {
                    let transition =
                        s.at_pointer("/transition", |s| transition(s, &step.transition))?;
                    let state = s.at_pointer("/post_state", |s| s.state(&step.post_state))?;
                    let mut out = s.record([("transition", transition), ("post_state", state)])?;
                    if let Some(record) =
                        s.at_pointer("/send_record", |s| send_record(s, &step.transition))?
                    {
                        s.put(&mut out, "send_record", record)?;
                    }
                    if let Some(delay) = &step.delay {
                        let delay = s.at_pointer("/delay", |s| s.rational(delay))?;
                        s.put(&mut out, "delay", delay)?;
                    }
                    Ok(out)
                })
            })?;
            if index < split {
                stem.push(written);
            } else {
                looped.push(written);
            }
        }
        if stutter {
            let index = self.display_text(&loop_len)?;
            let out = self.at_pointer("/loop/", |s| {
                s.at_pointer(&index, |s| {
                    let terminal = terminal_key
                        .or_else(|| counterexample.prefix.last().map(|last| &last.post_state));
                    let post_state = s.at_pointer("/post_state", |s| {
                        let terminal = terminal
                            .ok_or_else(|| s.carrier_refusal("initial terminal stutter"))?;
                        s.state(terminal)
                    })?;
                    let tag = s.text("terminal-stutter")?;
                    let transition = s.record([("type", tag)])?;
                    let mut out =
                        s.record([("transition", transition), ("post_state", post_state)])?;
                    s.at_pointer("/delay", |s| {
                        match counterexample
                            .lasso
                            .as_ref()
                            .and_then(|lasso| lasso.terminal_stutter_delay.as_ref())
                        {
                            Some(delay) if delay > &zero_rational() => {
                                let delay = s.rational(delay)?;
                                s.put(&mut out, "delay", delay)?;
                            }
                            Some(_) => {
                                return Err(s.carrier_refusal("non-positive terminal stutter delay"))
                            }
                            None if delay_required => {
                                return Err(s.carrier_refusal("missing terminal stutter delay"))
                            }
                            None => {}
                        }
                        Ok(())
                    })?;
                    Ok(out)
                })
            })?;
            // Reserve the additional slot before extending the loop vector.
            self.append(&mut looped, out)?;
        }
        let over = match (&counterexample.over_binding, over) {
            (None, _) => Json::Null,
            (Some(_), Some(over)) => {
                self.at_pointer("/over_binding", |s| s.encode_reference(over))?
            }
            (Some(_), None) => return Err(self.carrier_refusal("over binding")),
        };
        let mut fair = self.array(counterexample.fairness.len())?;
        for (index, member) in counterexample.fairness.iter().enumerate() {
            let index = self.display_text(&index)?;
            fair.push(self.at_pointer("/fairness/", |s| {
                s.at_pointer(&index, |s| fairness(s, member))
            })?);
        }
        let trace = self.text("model")?;
        let mut out = self.record([
            ("trace", trace),
            ("kind", kind),
            ("initial_state", Json::from(counterexample.initial_state)),
            ("prefix", Json::Array(stem)),
            ("loop", Json::Array(looped)),
            ("over_binding", over),
            ("fairness", Json::Array(fair)),
            ("interval", Json::Null),
            (
                "trace_position",
                Json::from(counterexample.trace_position()),
            ),
        ])?;
        if let Some(delay) = &counterexample.final_delay {
            let delay = self.at_pointer("/final_delay", |s| s.rational(delay))?;
            self.put(&mut out, "final_delay", delay)?;
        }
        if let Some(race) = race {
            self.put(&mut out, "race", race)?;
        }
        if let Some(loop_proof) = loop_proof {
            self.put(&mut out, "loop_proof", loop_proof)?;
        }
        if let Some(failure) = &counterexample.refinement {
            let failure = self.at_pointer("/refinement_failure", |s| s.refinement_full(failure))?;
            self.put(&mut out, "refinement_failure", failure)?;
        }
        Ok(out)
    }
}
