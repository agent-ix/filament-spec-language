// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-800 to FR-803: the goal, objective and policy rules of a plan
//! synthesis written in FSL source, evaluated over the checked package.
//!
//! [`SourcePlan`] is the [`PlanRules`] of one checked `synthesis` declaration
//! whose grammar is a model object type. The declaration's `satisfies` block
//! and objective were typed over the plan value (`fsl_semantics`'s
//! `CheckedPlan`); here the plan the search is judging is written as that
//! value (`length`, `final`, `initial`, `steps`) and the checked bodies are
//! evaluated over it under one evaluation meter. An evaluation that is
//! undefined does not reach the goal and ranks no plan (FSpec FR-800, FR-802);
//! one the meter denies stops the search `incomplete` (FSpec FR-801).

use std::collections::BTreeMap;

use filament_checked::node::{Arithmetic, NodeKind, Visit};
use filament_exact::{
    from_admitted, from_admitted_slots, CollectionKind, CollectionType, EnumMember, FieldValue,
    Integer, Outcome, Rational, ScalarLimits, UnionValue, Value, ValueType,
};
use filament_semantic_value::declaration::CompositeShape;
use filament_semantic_value::model_transition::ModelTransition;
use filament_semantic_value::stop::Stop as ValueStop;
use fsl_eval::automaton::{PolicyContext, PolicyState, PolicyStateError, StepPolicy};
use fsl_eval::model_subject::{AtomStop, ModelLetters};
use fsl_eval::simulation::model::{read_clause, ClauseReading};
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::temporal::AtomReading;
use fsl_package::CheckedPackage;
use fsl_semantics::check::{
    CheckedBody, CheckedNode, CheckedPlan, CheckedSynthesis, PlanOperation, TemporalClauseNode,
};
use fsl_semantics::model::state::StateObject;

use crate::plan_search::{
    Direction, MonitorStep, ObjectiveKind, PlanRules, PlanStep, PlanView, RuleStop, StepTotal,
};

/// A numeric objective family whose planning ordering and certificate
/// implementation is not available in this source adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedObjectiveFamily {
    /// IEEE floating-point objectives, including NaNs.
    Float,
    /// Unit-bearing quantity objectives.
    Quantity,
}

impl std::fmt::Display for UnsupportedObjectiveFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Float => "Float",
            Self::Quantity => "Quantity",
        })
    }
}

/// Why a checked synthesis cannot be judged as a plan.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PlanSourceError {
    /// The expression is well typed, but its numeric ordering and
    /// certificate family is not implemented by the source planner.
    #[error("plan objectives of numeric family {family} are not yet implemented")]
    UnsupportedObjective {
        /// The unsupported numeric family.
        family: UnsupportedObjectiveFamily,
        /// The checked objective expression's exact source location.
        location: filament_semantic_value::location::ExpressionLocation,
    },
    /// An admitted source form whose plan adapter is not implemented yet.
    #[error("source planning of {form} is not yet implemented")]
    UnsupportedForm {
        /// The specific unsupported form.
        form: String,
        /// The authored expression that selects it.
        location: filament_semantic_value::location::ExpressionLocation,
    },
    /// The declaration's grammar is not a model object type.
    #[error("the synthesis `{0}` is not a plan synthesis")]
    NotAPlan(String),
    /// The model has an operation the plan value's operation enum does not
    /// list.
    #[error("the model's operation `{0}` is not an operation of the plan value")]
    UnknownOperation(String),
}

/// One step argument in its canonical value order (FSpec FR-804).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ArgumentKey {
    /// A Boolean argument.
    Boolean(bool),
    /// An integer argument.
    Integer(Integer),
    /// A reference in the canonical order of its complete identity.
    Reference(filament_exact::ObjectReference),
    /// An admitted enum member's declaration rank. The operation fixes its enum type.
    Enum(u32),
}

impl ArgumentKey {
    fn of(value: &Value) -> Option<Self> {
        Some(match value {
            Value::Boolean(value) => Self::Boolean(*value),
            Value::Integer(value) => Self::Integer(value.clone()),
            Value::Reference(value) => Self::Reference(value.clone()),
            Value::Enum(value) => Self::Enum(value.rank()),
            _ => return None,
        })
    }
}

/// What one checked body evaluated to.
enum Evaluated {
    Value(Value),
    Undefined,
}

/// The `PlanRules` of one checked plan synthesis.
pub struct SourcePlan<'a> {
    package: &'a CheckedPackage,
    synthesis: &'a CheckedSynthesis,
    plan: &'a CheckedPlan,
    system: &'a ModelSystem<'a>,
    limits: ScalarLimits,
    /// The index in the plan value's operation list of each operation text a
    /// transition carries.
    operations: BTreeMap<String, usize>,
    native_operations: std::sync::Arc<[(filament_exact::EffectiveId, String)]>,
    direction: Direction,
    kind: ObjectiveKind,
    merges: bool,
    goal_reads_final_attributes: bool,
    objective_reads_final_attributes: bool,
    /// The temporal clauses the plan must satisfy, in the request's order
    /// (FSpec FR-803).
    policies: Vec<&'a TemporalClauseNode>,
    /// The states of each policy monitor's prefix, by monitor id: a policy
    /// monitor id is used only for zero-length closed source-policy reads.
    /// Positive-length source policies refuse until canonical monitors exist.
    traces: Vec<Vec<ModelSimState>>,
    positive_policies: bool,
    policy_cache: Vec<StepPolicy>,
    trace_ids: BTreeMap<(usize, Vec<Vec<u8>>), usize>,
    fault: Option<String>,
}

impl<'a> SourcePlan<'a> {
    /// Whether a checked policy has a canonical positive-length source adapter.
    /// Model compatibility is admitted separately against the plan grammar.
    pub fn supports_positive_policy(node: &TemporalClauseNode) -> bool {
        node.clause().clock() == "model-steps" && StepPolicy::new(node.clause()).is_some()
    }

    /// The rules of `synthesis` over `system`, evaluating under `limits`.
    ///
    /// # Errors
    /// [`PlanSourceError`] when the declaration is not a plan synthesis or
    /// the model has an operation its plan value does not list.
    pub fn new(
        package: &'a CheckedPackage,
        synthesis: &'a CheckedSynthesis,
        system: &'a ModelSystem<'a>,
        policies: Vec<&'a TemporalClauseNode>,
        limits: ScalarLimits,
    ) -> Result<Self, PlanSourceError> {
        let plan = synthesis
            .plan()
            .ok_or_else(|| PlanSourceError::NotAPlan(synthesis.name().to_owned()))?;
        if let Some(body) = synthesis.objective() {
            let family = match body.root().value_type() {
                ValueType::Float(_) => Some(UnsupportedObjectiveFamily::Float),
                ValueType::Quantity(_)
                | ValueType::BoundedQuantity(_)
                | ValueType::CarriedQuantity(_) => Some(UnsupportedObjectiveFamily::Quantity),
                _ => None,
            };
            if let Some(family) = family {
                return Err(PlanSourceError::UnsupportedObjective {
                    family,
                    location: body.root().location().clone(),
                });
            }
        }
        let positive_policies =
            !matches!(synthesis.domain(), ValueType::Int(domain) if domain.upper().is_zero());
        if positive_policies
            && policies
                .iter()
                .any(|node| !Self::supports_positive_policy(node))
        {
            return Err(PlanSourceError::UnsupportedForm {
                form: "positive-length source policy outside capture-free origin safety and bounded F/G[0,k]"
                    .to_owned(),
                location: synthesis.satisfies().root().location().clone(),
            });
        }
        system.share_evaluation_meter(limits);
        let shape = plan.shape();
        let mut operations = BTreeMap::new();
        for (declaring, name, text) in system.operations() {
            let index = shape
                .operations
                .iter()
                .position(|operation| operation.declaring == declaring && operation.name == name)
                .ok_or_else(|| PlanSourceError::UnknownOperation(name.to_owned()))?;
            operations.insert(text.to_owned(), index);
        }
        for operation in &shape.operations {
            let declaration = plan
                .scope()
                .operations()
                .declared_by(operation.declaring.clone())
                .iter()
                .find(|declaration| declaration.name() == operation.name)
                .ok_or_else(|| PlanSourceError::UnknownOperation(operation.name.clone()))?;
            if let Some(result) = declaration.result() {
                if matches!(
                    result,
                    ValueType::Float(_) | ValueType::Rational(_) | ValueType::Decimal(_)
                ) && !system.has_native_result_domain(result)
                {
                    return Err(PlanSourceError::UnsupportedForm {
                        form: format!(
                            "operation `{}` result type {result:?} (native result domain is not enumerable)",
                            operation.name
                        ),
                        location: synthesis.satisfies().root().location().clone(),
                    });
                }
            }
        }
        let native_operations = shape
            .operations
            .iter()
            .map(|operation| (operation.declaring.clone(), operation.name.clone()))
            .collect::<Vec<_>>()
            .into();
        let direction = match synthesis.objective_direction() {
            Some("maximize") => Direction::Maximize,
            _ => Direction::Minimize,
        };
        let plan_key = shape.plan.clone();
        let kind = match synthesis.objective() {
            None => ObjectiveKind::StepSum,
            Some(body) if is_step_sum(body, &plan_key) => ObjectiveKind::StepSum,
            Some(_) => ObjectiveKind::General,
        };
        // FSpec FR-801: prefixes with equal nodes merge unless the goal, or
        // an objective that is no per-step sum, reads `steps`.
        let merges = !reads_field(synthesis.satisfies(), &plan_key, "steps")
            && (kind == ObjectiveKind::StepSum
                || synthesis
                    .objective()
                    .is_none_or(|body| !reads_field(body, &plan_key, "steps")));
        let attributes = |body: &CheckedBody| {
            shape
                .inline
                .iter()
                .any(|name| reads_field(body, &shape.state, name))
        };
        let goal_reads_final_attributes = attributes(synthesis.satisfies());
        let objective_reads_final_attributes = synthesis.objective().is_some_and(attributes);
        if goal_reads_final_attributes || objective_reads_final_attributes {
            use fsl_eval::simulation::TransitionSystem;
            let unique = system.initial().iter().all(|state| {
                state
                    .state()
                    .populations
                    .values()
                    .flat_map(|objects| objects.values())
                    .filter(|object| {
                        system
                            .type_identity(&object.type_identity)
                            .is_some_and(|identity| {
                                plan.scope()
                                    .types()
                                    .conforms(identity, shape.grammar.clone())
                            })
                    })
                    .count()
                    == 1
            });
            let deletes = shape.operations.iter().any(|operation| {
                plan.scope()
                    .operations()
                    .declared_by(operation.declaring.clone())
                    .iter()
                    .find(|declaration| declaration.name() == operation.name)
                    .is_some_and(|declaration| {
                        let effect = declaration.effect();
                        !effect.deletes.is_empty()
                            || effect.deletes_receiver
                            || !effect.deletes_arguments.is_empty()
                    })
            });
            if !unique || deletes {
                let body = if goal_reads_final_attributes {
                    synthesis.satisfies()
                } else {
                    synthesis.objective().expect("objective attribute read")
                };
                return Err(PlanSourceError::UnsupportedForm {
                    form: "ambiguous grammar-object state-field projection (requires exactly one stable grammar object)".to_owned(),
                    location: body.root().location().clone(),
                });
            }
        }

        Ok(Self {
            package,
            synthesis,
            plan,
            system,
            limits,
            operations,
            native_operations,
            direction,
            kind,
            merges,
            goal_reads_final_attributes,
            objective_reads_final_attributes,
            policies,
            traces: Vec::new(),
            positive_policies,
            policy_cache: Vec::new(),
            trace_ids: BTreeMap::new(),
            fault: None,
        })
    }

    /// The first internal fault an evaluation raised, if any: a plan the
    /// rules could not judge is a method failure, never an answer.
    pub fn fault(&self) -> Option<&str> {
        self.fault.as_deref()
    }

    /// The operation of `step` in the plan value's operation list.
    pub fn operation(&self, step: &ModelTransition) -> Option<&PlanOperation> {
        self.operations
            .get(step.operation())
            .and_then(|index| self.plan.shape().operations.get(*index))
    }

    /// The name of the operation `step` invokes.
    pub fn operation_name(&self, step: &ModelTransition) -> Option<&str> {
        self.operation(step)
            .map(|operation| operation.name.as_str())
    }

    /// The plan value of `view`, its steps numbered from `first`.
    fn plan_value(&mut self, view: &PlanView<'_>, first: usize) -> Option<Value> {
        let built = self.build_plan(view, first);
        if built.is_none() {
            self.fault
                .get_or_insert_with(|| "a plan value could not be built".to_owned());
        }
        built
    }

    fn build_plan(&self, view: &PlanView<'_>, first: usize) -> Option<Value> {
        let shape = self.plan.shape();
        let states = view
            .states
            .iter()
            .map(|state| self.state_value(state))
            .collect::<Option<Vec<_>>>()?;
        let mut steps = Vec::with_capacity(view.steps.len());
        for (position, step) in view.steps.iter().enumerate() {
            steps.push(self.step_value(
                step,
                first + position,
                states.get(position)?.clone(),
                states.get(position + 1)?.clone(),
            )?);
        }
        let steps = from_admitted(
            CollectionType::new(
                CollectionKind::Sequence,
                ValueType::Composite(shape.step.clone()),
                None,
            ),
            steps,
        );
        let length = Value::Integer(Integer::from(i64::try_from(view.steps.len()).ok()?));
        Some(from_admitted_slots(
            shape.plan.clone(),
            vec![
                FieldValue::Present(length),
                FieldValue::Present(states.last()?.clone()),
                FieldValue::Present(states.first()?.clone()),
                FieldValue::Present(steps),
            ]
            .into_boxed_slice(),
        ))
    }

    fn step_value(
        &self,
        step: &PlanStep,
        index: usize,
        before: Value,
        after: Value,
    ) -> Option<Value> {
        let shape = self.plan.shape();
        let operation = self.operation(step)?;
        let (native_receiver, native_arguments) = self.invocation_values(step)?;
        let member = shape
            .operation_enum
            .members
            .iter()
            .find(|member| member.case() == operation.name)?;
        let rank = u32::try_from(member.position()).ok()?;
        let carried = shape.argument_fields();
        let slots = carried
            .iter()
            .map(|(name, _)| {
                match operation
                    .parameters
                    .iter()
                    .position(|(parameter, _)| parameter == name)
                {
                    Some(position) => {
                        Some(FieldValue::Present(native_arguments.get(position)?.clone()))
                    }
                    None => Some(FieldValue::Absent),
                }
            })
            .collect::<Option<Vec<_>>>()?;
        let arguments = from_admitted_slots(shape.arguments.clone(), slots.into_boxed_slice());
        let receiver = if shape.receiver_type.is_some() {
            let reference = native_receiver;
            if !self
                .plan
                .scope()
                .types()
                .conforms(reference.object_type().clone(), operation.declaring.clone())
            {
                return None;
            }
            let value = Value::Reference(reference.clone());
            if let Some(handle) = &shape.receiver_union {
                let (_, case) = shape
                    .receiver_cases
                    .iter()
                    .find(|(ty, _)| ty == &operation.declaring)?;
                let CompositeShape::Union(members) =
                    self.plan.scope().types().composite(handle.clone())?.shape()
                else {
                    return None;
                };
                let member = members
                    .iter()
                    .find(|member| member.identifier() == case.as_str())?
                    .member()?
                    .clone();
                Some(UnionValue::from_admitted(member, vec![value]))
            } else {
                return None;
            }
        } else {
            if step.receiver().is_some() {
                return None;
            }
            None
        };
        let mut step_slots = vec![
            FieldValue::Present(Value::Integer(Integer::from(i64::try_from(index).ok()?))),
            FieldValue::Present(Value::Enum(EnumMember::new(member.variant(), rank))),
            step.outcome_ordinal()
                .map_or(FieldValue::Absent, |ordinal| {
                    FieldValue::Present(Value::Integer(Integer::from(ordinal)))
                }),
            FieldValue::Present(arguments),
            FieldValue::Present(before),
            FieldValue::Present(after),
        ];
        if let Some(receiver) = receiver {
            step_slots.insert(2, FieldValue::Present(receiver));
        }
        Some(from_admitted_slots(
            shape.step.clone(),
            step_slots.into_boxed_slice(),
        ))
    }

    /// A state as the plan value's state record: each population's objects,
    /// then the attributes of the grammar's object, read when the state has
    /// exactly one.
    fn state_value(&self, state: &ModelSimState) -> Option<Value> {
        let shape = self.plan.shape();
        let types = self.plan.scope().types();
        let declaration = types.composite(shape.state.clone())?;
        let CompositeShape::Record(fields) = declaration.shape() else {
            return None;
        };
        let mut grammar_objects = Vec::new();
        for objects in state.state().populations.values() {
            for object in objects.values() {
                let identity = self.system.type_identity(&object.type_identity)?;
                if types.conforms(identity, shape.grammar.clone()) {
                    grammar_objects.push(object);
                }
            }
        }
        let own = match grammar_objects.as_slice() {
            [object] => Some(*object),
            _ => None,
        };
        let mut slots = Vec::with_capacity(fields.len());
        for field in fields {
            let population = shape
                .populations
                .iter()
                .find(|population| population.field == field.name());
            let slot = match population {
                Some(population) => {
                    let record = shape.objects.get(&population.object)?;
                    let elements = state
                        .state()
                        .populations
                        .get(&population.key)
                        .map(|objects| {
                            objects
                                .values()
                                .map(|object| self.object_value(object, record))
                                .collect::<Option<Vec<_>>>()
                        })
                        .unwrap_or_else(|| Some(Vec::new()))?;
                    FieldValue::Present(from_admitted(
                        CollectionType::new(
                            CollectionKind::Sequence,
                            ValueType::Composite(record.clone()),
                            None,
                        ),
                        elements,
                    ))
                }
                None => own
                    .and_then(|object| self.attribute(object, field.name()))
                    .map_or(FieldValue::Absent, FieldValue::Present),
            };
            slots.push(slot);
        }
        Some(from_admitted_slots(
            shape.state.clone(),
            slots.into_boxed_slice(),
        ))
    }

    fn object_value(
        &self,
        object: &StateObject,
        record: &filament_exact::NodeKey,
    ) -> Option<Value> {
        let declaration = self.plan.scope().types().composite(record.clone())?;
        let CompositeShape::Record(fields) = declaration.shape() else {
            return None;
        };
        let slots = fields
            .iter()
            .map(|field| {
                self.attribute(object, field.name())
                    .map_or(FieldValue::Absent, FieldValue::Present)
            })
            .collect::<Vec<_>>();
        Some(from_admitted_slots(
            record.clone(),
            slots.into_boxed_slice(),
        ))
    }

    /// The value `object` holds for its attribute `name`.
    fn attribute(&self, object: &StateObject, name: &str) -> Option<Value> {
        let declared = self.system.fields_of(&object.type_identity)?;
        let field = declared.iter().find(|field| field.name == name)?;
        object.fields.get(&field.key).cloned()
    }

    fn evaluation_stop(&self, counter: &'static str, value: u64) -> RuleStop {
        match self.system.plan_cancel_cause() {
            Some(cause) => RuleStop::Cancelled(cause),
            None => RuleStop::Accounting { counter, value },
        }
    }

    /// Literal and implicit objective values can return without a charge.
    /// Observe the shared meter's cancellation without consuming its budget.
    fn poll_cancellation(&self) -> Result<(), RuleStop> {
        match self
            .system
            .evaluation_meter()
            .cancel()
            .and_then(filament_exact::Cancel::cause)
        {
            Some(cause) => Err(RuleStop::Cancelled(cause)),
            None => Ok(()),
        }
    }

    fn evaluate(&mut self, body: &CheckedBody, plan: Value) -> Result<Evaluated, RuleStop> {
        self.evaluate_node(body.root(), plan)
    }

    fn evaluate_node(&mut self, node: CheckedNode<'_>, plan: Value) -> Result<Evaluated, RuleStop> {
        self.poll_cancellation()?;
        let evaluation = fsl_eval::value::evaluate_plan_node(
            self.package,
            self.plan.scope(),
            node,
            vec![Some(plan)],
            &mut self.system.evaluation_meter(),
        );
        match evaluation {
            Ok(evaluation) => match evaluation.outcome {
                filament_eval::FamilyOutcome::Evaluated(Outcome::Completed(value)) => {
                    Ok(Evaluated::Value(value))
                }
                filament_eval::FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
                    Err(self.evaluation_stop(incomplete.limit_kind.as_str(), incomplete.limit))
                }
                filament_eval::FamilyOutcome::Evaluated(Outcome::Undefined(_)) => {
                    Ok(Evaluated::Undefined)
                }
                filament_eval::FamilyOutcome::Evaluated(Outcome::Refused(_)) => {
                    self.fault.get_or_insert_with(|| {
                        "a checked plan expression refused evaluation".to_owned()
                    });
                    Err(RuleStop::Cancelled(filament_exact::CancelCause::Requested))
                }
                filament_eval::FamilyOutcome::FamilyEvaluated(
                    filament_eval::FamilyResult::Undefined(_),
                ) => Ok(Evaluated::Undefined),
                filament_eval::FamilyOutcome::FamilyEvaluated(
                    filament_eval::FamilyResult::Refused(_),
                ) => {
                    self.fault.get_or_insert_with(|| "a checked plan expression returned a family-owned result unsupported by this adapter".to_owned());
                    Err(RuleStop::Cancelled(filament_exact::CancelCause::Requested))
                }
            },
            Err(_fault) => {
                self.fault.get_or_insert_with(|| {
                    "a checked plan expression violated its evaluation contract".to_owned()
                });
                Err(RuleStop::Cancelled(filament_exact::CancelCause::Requested))
            }
        }
    }

    fn policy_stop(&mut self, error: PolicyStateError) -> RuleStop {
        match error {
            PolicyStateError::Accounting(incomplete) => {
                self.evaluation_stop(incomplete.limit_kind.as_str(), incomplete.limit)
            }
            PolicyStateError::IdentityLimit { value } => RuleStop::IdentityLimit { value },
            PolicyStateError::Cancelled(cause) => RuleStop::Cancelled(cause),
            other => {
                self.fault.get_or_insert_with(|| other.to_string());
                RuleStop::Cancelled(filament_exact::CancelCause::Requested)
            }
        }
    }

    fn policy_charge(&self, charge: filament_exact::Charge) -> Result<(), RuleStop> {
        let mut meter = self.system.evaluation_meter();
        if let Some(cause) = meter.cancel().and_then(filament_exact::Cancel::cause) {
            return Err(RuleStop::Cancelled(cause));
        }
        meter.charge(charge).map_err(|incomplete| {
            match meter.cancel().and_then(filament_exact::Cancel::cause) {
                Some(cause) => RuleStop::Cancelled(cause),
                None => self.evaluation_stop(incomplete.limit_kind.as_str(), incomplete.limit),
            }
        })?;
        if let Some(cause) = meter.cancel().and_then(filament_exact::Cancel::cause) {
            return Err(RuleStop::Cancelled(cause));
        }
        Ok(())
    }

    fn policy_capacity<T>(&self, values: &mut Vec<T>, extra: usize) -> Result<(), RuleStop> {
        use filament_exact::{Charge, ChargePoint};
        let needed = values
            .len()
            .checked_add(extra)
            .ok_or(RuleStop::Cancelled(filament_exact::CancelCause::Requested))?;
        if needed > values.capacity() {
            let bytes = needed
                .checked_mul(std::mem::size_of::<T>())
                .ok_or(RuleStop::Cancelled(filament_exact::CancelCause::Requested))?;
            self.policy_charge(
                Charge::new(ChargePoint::ModelCandidate)
                    .work(Integer::zero())
                    .results(u64::try_from(bytes).map_err(|_| {
                        RuleStop::Cancelled(filament_exact::CancelCause::Requested)
                    })?),
            )?;
            values
                .try_reserve_exact(needed - values.len())
                .map_err(|_| RuleStop::Cancelled(filament_exact::CancelCause::Requested))?;
        }
        Ok(())
    }

    fn compiled_policy(&mut self, index: usize) -> Result<StepPolicy, RuleStop> {
        while self.policy_cache.len() <= index {
            let policy = self
                .policies
                .get(self.policy_cache.len())
                .and_then(|node| StepPolicy::new(node.clause()))
                .ok_or(RuleStop::Cancelled(filament_exact::CancelCause::Requested))?;
            let mut cache = std::mem::take(&mut self.policy_cache);
            self.policy_capacity(&mut cache, 1)?;
            cache.push(policy);
            self.policy_cache = cache;
        }
        Ok(self.policy_cache[index])
    }

    fn canonical_policy(
        &mut self,
        index: usize,
        previous: Option<&[u8]>,
        state: &ModelSimState,
        close: bool,
    ) -> Result<MonitorStep, RuleStop> {
        let policy = self.compiled_policy(index)?;
        let node = self.policies[index];
        let instances = self
            .system
            .instances_with_meter(&node.clause().over().1, &mut self.system.evaluation_meter())
            .map_err(|error| self.policy_stop(error))?;
        let identity = node.identity();
        let parameter = node.over();
        let mut contexts = Vec::new();
        self.policy_capacity(&mut contexts, instances.len())?;
        contexts.extend(instances.iter().map(|instance| PolicyContext {
            node: &identity,
            claim: node.claim(),
            parameter: &parameter,
            instance: &instance.key,
        }));
        let before = if let Some(bytes) = previous {
            policy
                .admit(&contexts, bytes, &mut self.system.evaluation_meter())
                .map_err(|error| self.policy_stop(error))?
        } else {
            Vec::new()
        };
        let letters = match self
            .system
            .clause_letters_with_meter(node, &mut self.system.evaluation_meter())
        {
            Ok(letters) => letters,
            Err(fsl_eval::simulation::model::LettersPreparationError::Policy(error)) => {
                return Err(self.policy_stop(error))
            }
            Err(fsl_eval::simulation::model::LettersPreparationError::Admission(error)) => {
                self.fault.get_or_insert_with(|| error.to_string());
                return Ok(MonitorStep::Reject);
            }
        };
        let mut after = Vec::new();
        self.policy_capacity(&mut after, instances.len())?;
        for (position, instance) in instances.iter().enumerate() {
            let old = before.get(position).copied();
            let needs_atom = if close {
                matches!(
                    old,
                    Some(PolicyState::Bounded(
                        fsl_eval::automaton::BoundedState::Open { remaining: 1.. }
                    ))
                )
            } else {
                policy.reads(old)
            };
            let holds = if needs_atom {
                match letters.atom(
                    policy.atom(),
                    instance,
                    state,
                    &mut self.system.evaluation_meter(),
                ) {
                    Ok(AtomReading::True) => true,
                    Ok(AtomReading::False) => false,
                    Ok(AtomReading::Undefined(_)) => return Ok(MonitorStep::Reject),
                    Err(AtomStop::Incomplete(incomplete)) => {
                        return Err(
                            self.evaluation_stop(incomplete.limit_kind.as_str(), incomplete.limit)
                        )
                    }
                    Err(error) => {
                        self.fault.get_or_insert_with(|| format!("{error:?}"));
                        return Ok(MonitorStep::Reject);
                    }
                }
            } else {
                false
            };
            if close {
                let Some(old) = old else {
                    self.fault
                        .get_or_insert_with(|| "a closed policy has an initial state".to_owned());
                    return Ok(MonitorStep::Reject);
                };
                if !policy
                    .terminal(old, holds, &mut self.system.evaluation_meter())
                    .map_err(|error| self.policy_stop(error))?
                {
                    return Ok(MonitorStep::Reject);
                }
            } else {
                let next = policy
                    .advance(old, holds, &mut self.system.evaluation_meter())
                    .map_err(|error| self.policy_stop(error))?;
                if policy.rejects(next) {
                    return Ok(MonitorStep::Reject);
                }
                after.push(next);
            }
        }
        if close {
            return Ok(MonitorStep::Continue(Vec::new()));
        }
        let encoded = policy
            .encode(&contexts, &after, &mut self.system.evaluation_meter())
            .map_err(|error| self.policy_stop(error))?;
        Ok(MonitorStep::Continue(encoded))
    }

    /// The states a policy monitor has read.
    fn prefix(&mut self, monitor: &[u8]) -> Result<Vec<ModelSimState>, RuleStop> {
        <[u8; 8]>::try_from(monitor)
            .ok()
            .and_then(|id| {
                self.traces
                    .get(usize::try_from(u64::from_le_bytes(id)).ok()?)
            })
            .cloned()
            .ok_or_else(|| {
                self.fault
                    .get_or_insert_with(|| "a policy monitor id is unknown".to_owned());
                RuleStop::Cancelled(filament_exact::CancelCause::Requested)
            })
    }

    /// Policy `policy` after reading `state` following `prefix`: rejected
    /// when every continuation violates the clause (FSpec FR-803), else the
    /// id of the prefix it has now read.
    fn advance(
        &mut self,
        policy: usize,
        mut prefix: Vec<ModelSimState>,
        state: &ModelSimState,
    ) -> Result<MonitorStep, RuleStop> {
        prefix.push(state.clone());
        let Some(&node) = self.policies.get(policy) else {
            return Ok(MonitorStep::Reject);
        };
        match read_clause(
            self.system,
            node,
            &prefix,
            false,
            &mut self.system.evaluation_meter(),
        ) {
            Ok(ClauseReading::Violated) => return Ok(MonitorStep::Reject),
            Ok(ClauseReading::Incomplete { counter, value }) => {
                return Err(self.evaluation_stop(counter, value));
            }
            Ok(ClauseReading::Holds | ClauseReading::Undecided) => {}
            Err(error) => {
                self.fault.get_or_insert_with(|| error.to_string());
                return Ok(MonitorStep::Reject);
            }
        }
        let keys = prefix
            .iter()
            .map(|read| self.system.state_key(read, fsl_foundation::IdentityLimits::default()).map(|(key, _)| key))
            .collect::<Result<Vec<_>, _>>();
        let Ok(keys) = keys else {
            self.fault
                .get_or_insert_with(|| "a policy prefix has no state key".to_owned());
            return Ok(MonitorStep::Reject);
        };
        let next = self.traces.len();
        let id = *self.trace_ids.entry((policy, keys)).or_insert(next);
        if id == next {
            self.traces.push(prefix);
        }
        Ok(MonitorStep::Continue(
            u64::try_from(id).unwrap_or(u64::MAX).to_le_bytes().to_vec(),
        ))
    }

    fn step_cost_value(
        &mut self,
        plan: &PlanView<'_>,
        index: usize,
    ) -> Result<Option<Value>, RuleStop> {
        self.poll_cancellation()?;
        let Some(body) = self.synthesis.objective() else {
            return Ok(Some(Value::Integer(Integer::one())));
        };
        let (Some(states), Some(steps)) = (
            plan.states.get(index..=index + 1),
            plan.steps.get(index..=index),
        ) else {
            return Ok(None);
        };
        let one = PlanView { states, steps };
        if !self.readable(self.objective_reads_final_attributes, &one) {
            return Ok(None);
        }
        let Some(value) = self.plan_value(&one, index) else {
            return Ok(None);
        };
        Ok(
            match self.evaluate_node(step_sum_node(body.root()), value)? {
                Evaluated::Value(value) => Some(value),
                Evaluated::Undefined => None,
            },
        )
    }

    /// The exact value of an objective evaluation.
    fn exact(value: &Value) -> Option<Rational> {
        match value {
            Value::Integer(value) => Some(Rational::from_integer(value.clone())),
            Value::Rational(value) => Some(value.clone()),
            Value::Decimal(value) => Some(value.representation().to_rational()),
            _ => None,
        }
    }

    fn invocation_values<'s>(
        &self,
        step: &'s PlanStep,
    ) -> Option<(&'s filament_exact::ObjectReference, &'s [Value])> {
        let operation = self.operation(step)?;
        let (receiver, arguments) = step.native_values()?;
        let types = self.plan.scope().types();
        if !types.conforms(receiver.object_type().clone(), operation.declaring.clone())
            || arguments.len() != operation.parameters.len()
            || !operation
                .parameters
                .iter()
                .zip(arguments)
                .all(|((_, declared), value)| match (declared, value) {
                    (ValueType::Reference(target), Value::Reference(reference)) => {
                        types.conforms(reference.object_type().clone(), target.clone())
                    }
                    _ => declared.admits(value),
                })
        {
            return None;
        }
        Some((receiver, arguments))
    }

    /// The state-attribute reads need exactly one object of the grammar's
    /// type in the state they read; without one, the evaluation is undefined.
    fn readable(&self, reads_attributes: bool, view: &PlanView<'_>) -> bool {
        if !reads_attributes {
            return true;
        }
        let types = self.plan.scope().types();
        let grammar = self.plan.shape().grammar.clone();
        view.states.iter().all(|state| {
            let mut count = 0_usize;
            for objects in state.state().populations.values() {
                for object in objects.values() {
                    if self
                        .system
                        .type_identity(&object.type_identity)
                        .is_some_and(|identity| types.conforms(identity, grammar.clone()))
                    {
                        count += 1;
                    }
                }
            }
            count == 1
        })
    }
}

/// Whether `body` reads the field `field` of the record `record`.
fn reads_field(body: &CheckedBody, record: &filament_exact::NodeKey, field: &str) -> bool {
    let mut pending = vec![body.root()];
    while let Some(node) = pending.pop() {
        if matches!(
            node.kind(),
            NodeKind::Field { field: read, .. }
                if read.name() == field && &read.declaration() == record
        ) {
            return true;
        }
        pending.extend(node.children());
    }
    false
}

/// Whether `body` is a sum over the plan's steps whose term reads only the
/// step it is evaluated for (FSpec FR-802 "Classification").
fn is_step_sum(body: &CheckedBody, plan: &filament_exact::NodeKey) -> bool {
    let root = step_sum_node(body.root());
    if !matches!(
        root.value_type(),
        ValueType::Integer | ValueType::Int(_) | ValueType::Rational(_) | ValueType::Decimal(_)
    ) {
        return false;
    }
    if matches!(
        root.kind(),
        NodeKind::Field { field, .. } if field.name() == "length" && &field.declaration() == plan
    ) {
        return true;
    }
    let NodeKind::Query {
        visit: Visit::Sum,
        source,
        body: term,
        ..
    } = root.kind()
    else {
        return false;
    };
    let source = root.at(*source);
    let reads_steps = matches!(
        source.kind(),
        NodeKind::Field { field, .. } if field.name() == "steps" && &field.declaration() == plan
    );
    if !reads_steps {
        return false;
    }
    let mut pending = vec![root.at(*term)];
    while let Some(node) = pending.pop() {
        let reads_plan = match node.kind() {
            NodeKind::Local(_) => node.value_type() == &ValueType::Composite(plan.clone()),
            NodeKind::Field { field, .. } => &field.declaration() == plan,
            _ => false,
        };
        if reads_plan {
            return false;
        }
        pending.extend(node.children());
    }
    true
}

/// Split only a root sum plus constant. The explicit `sum<N>` expression
/// grammar also reaches this classifier and admits nonconstant additions.
fn step_sum_node(root: CheckedNode<'_>) -> CheckedNode<'_> {
    let (left, right) = match root.kind() {
        NodeKind::Arithmetic(Arithmetic::Add, left, right)
        | NodeKind::Rational {
            operator: filament_exact::ArithmeticOperator::Add,
            left,
            right,
            ..
        }
        | NodeKind::Decimal {
            operator: filament_exact::ArithmeticOperator::Add,
            left,
            right,
            ..
        } => (root.at(*left), root.at(*right)),
        _ => return root,
    };
    if matches!(
        left.kind(),
        NodeKind::Query {
            visit: Visit::Sum,
            ..
        }
    ) && constant_offset(right)
    {
        left
    } else {
        root
    }
}

fn constant_offset(mut node: CheckedNode<'_>) -> bool {
    loop {
        match node.kind() {
            NodeKind::Literal(literal) => {
                return matches!(
                    &literal.0,
                    Value::Integer(_) | Value::Rational(_) | Value::Decimal(_)
                );
            }
            NodeKind::Negate(inner)
            | NodeKind::RationalNegate(inner, _)
            | NodeKind::DecimalNegate(inner, _)
            | NodeKind::Coerce(inner, _) => node = node.at(*inner),
            _ => return false,
        }
    }
}

impl PlanRules for SourcePlan<'_> {
    type StepKey = (
        usize,
        Option<filament_exact::ObjectReference>,
        Vec<ArgumentKey>,
    );

    fn direction(&self) -> Direction {
        self.direction
    }

    fn objective_kind(&self) -> ObjectiveKind {
        self.kind
    }

    fn merges_prefixes(&self) -> bool {
        self.merges
    }

    fn step_key(&mut self, step: &PlanStep) -> Result<Self::StepKey, RuleStop> {
        let key = self
            .operations
            .get(step.operation())
            .copied()
            .zip(self.invocation_values(step))
            .and_then(|(operation, (receiver, arguments))| {
                Some((
                    operation,
                    Some(receiver.clone()),
                    arguments
                        .iter()
                        .map(ArgumentKey::of)
                        .collect::<Option<Vec<_>>>()?,
                ))
            });
        key.ok_or_else(|| {
            self.fault.get_or_insert_with(|| {
                "a native plan step has no admitted typed invocation values".to_owned()
            });
            RuleStop::Cancelled(filament_exact::CancelCause::Requested)
        })
    }

    fn native_operations(&self) -> Option<std::sync::Arc<[(filament_exact::EffectiveId, String)]>> {
        Some(self.native_operations.clone())
    }

    fn policy_states(&mut self) -> Result<Vec<Vec<u8>>, RuleStop> {
        let mut states = Vec::new();
        if self.positive_policies {
            self.policy_capacity(&mut states, self.policies.len())?;
        } else {
            states.reserve_exact(self.policies.len());
        }
        Ok(states)
    }

    fn copy_policy_states(&mut self, monitors: &[Vec<u8>]) -> Result<Vec<Vec<u8>>, RuleStop> {
        if !self.positive_policies || monitors.is_empty() {
            return Ok(monitors.to_vec());
        }
        StepPolicy::copy_states(monitors, &mut self.system.evaluation_meter())
            .map_err(|error| self.policy_stop(error))
    }

    fn compare_policy_states(
        &mut self,
        left: &[Vec<u8>],
        right: &[Vec<u8>],
    ) -> Result<std::cmp::Ordering, RuleStop> {
        if !self.positive_policies || self.policies.is_empty() {
            return Ok(left.cmp(right));
        }
        StepPolicy::compare_states(left, right, &mut self.system.evaluation_meter())
            .map_err(|error| self.policy_stop(error))
    }

    fn push_policy_index(&mut self, bucket: &mut Vec<usize>, node: usize) -> Result<(), RuleStop> {
        if self.positive_policies && !self.policies.is_empty() {
            self.policy_charge(
                filament_exact::Charge::new(filament_exact::ChargePoint::ModelCandidate)
                    .work(Integer::from(1_i64)),
            )?;
            self.policy_capacity(bucket, 1)?;
        }
        bucket.push(node);
        Ok(())
    }

    fn policy_count(&self) -> usize {
        self.policies.len()
    }

    fn policy_start(
        &mut self,
        policy: usize,
        state: &ModelSimState,
    ) -> Result<MonitorStep, RuleStop> {
        if self.positive_policies {
            self.canonical_policy(policy, None, state, false)
        } else {
            self.advance(policy, Vec::new(), state)
        }
    }

    fn policy_advance(
        &mut self,
        policy: usize,
        monitor: &[u8],
        state: &ModelSimState,
    ) -> Result<MonitorStep, RuleStop> {
        if self.positive_policies {
            return self.canonical_policy(policy, Some(monitor), state, false);
        }
        let prefix = self.prefix(monitor)?;
        self.advance(policy, prefix, state)
    }

    fn policy_accepts(
        &mut self,
        policy: usize,
        monitor: &[u8],
        last: &ModelSimState,
    ) -> Result<bool, RuleStop> {
        if self.positive_policies {
            return Ok(matches!(
                self.canonical_policy(policy, Some(monitor), last, true)?,
                MonitorStep::Continue(_)
            ));
        }
        let states = self.prefix(monitor)?;
        let Some(&node) = self.policies.get(policy) else {
            return Ok(false);
        };
        Ok(
            match read_clause(
                self.system,
                node,
                &states,
                true,
                &mut self.system.evaluation_meter(),
            ) {
                Ok(ClauseReading::Holds) => true,
                Ok(ClauseReading::Incomplete { counter, value }) => {
                    return Err(self.evaluation_stop(counter, value));
                }
                Ok(ClauseReading::Violated | ClauseReading::Undecided) => false,
                Err(error) => {
                    self.fault.get_or_insert_with(|| error.to_string());
                    false
                }
            },
        )
    }

    fn goal(&mut self, plan: &PlanView<'_>) -> Result<bool, RuleStop> {
        if !self.readable(self.goal_reads_final_attributes, plan) {
            return Ok(false);
        }
        let Some(value) = self.plan_value(plan, 0) else {
            return Ok(false);
        };
        let body = self.synthesis.satisfies();
        Ok(matches!(
            self.evaluate(body, value)?,
            Evaluated::Value(Value::Boolean(true))
        ))
    }

    fn step_cost(
        &mut self,
        plan: &PlanView<'_>,
        index: usize,
    ) -> Result<Option<Rational>, RuleStop> {
        Ok(self
            .step_cost_value(plan, index)?
            .as_ref()
            .and_then(Self::exact))
    }

    fn step_cost_typed(
        &mut self,
        plan: &PlanView<'_>,
        index: usize,
    ) -> Result<Option<StepTotal>, RuleStop> {
        Ok(self.step_cost_value(plan, index)?.and_then(|value| {
            Self::exact(&value).map(|exact| StepTotal {
                value: exact,
                carried: Some(value),
            })
        }))
    }

    fn extend_step_sum(
        &mut self,
        plan: &PlanView<'_>,
        total: &StepTotal,
        cost: &StepTotal,
    ) -> Result<Option<StepTotal>, RuleStop> {
        use fsl_eval::value::collections::{self, Halt};
        let Some(index) = plan.steps.len().checked_sub(1) else {
            return Ok(None);
        };
        let Some(summand) = cost.carried.clone() else {
            self.fault
                .get_or_insert_with(|| "an exact step cost lost its admitted payload".to_owned());
            return Err(RuleStop::Cancelled(filament_exact::CancelCause::Requested));
        };
        if index > 0 && total.carried.is_none() {
            self.fault
                .get_or_insert_with(|| "a typed step total lost its admitted payload".to_owned());
            return Err(RuleStop::Cancelled(filament_exact::CancelCause::Requested));
        }
        let value_type = self
            .synthesis
            .objective()
            .map(|body| step_sum_node(body.root()).value_type().clone())
            .unwrap_or(ValueType::Integer);
        let added = collections::sum_exact(
            &value_type,
            total.carried.clone(),
            summand,
            &mut self.system.evaluation_meter(),
        );
        let retained = match added {
            Some(Ok((value, _loss))) => {
                collections::retain_accumulator(value, &mut self.system.evaluation_meter())
                    .map_err(Halt::from)
            }
            Some(Err(stop)) => Err(stop),
            None => Err(Halt::Fault("exact-step-total-has-an-exact-family")),
        };
        match retained {
            Ok(value) => {
                let Some(exact) = Self::exact(&value) else {
                    self.fault.get_or_insert_with(|| {
                        "a retained step total has no numeric value".to_owned()
                    });
                    return Err(RuleStop::Cancelled(filament_exact::CancelCause::Requested));
                };
                Ok(Some(StepTotal {
                    value: exact,
                    carried: Some(value),
                }))
            }
            Err(Halt::Stop(ValueStop::Incomplete(incomplete))) => {
                Err(self.evaluation_stop(incomplete.limit_kind.as_str(), incomplete.limit))
            }
            Err(Halt::Stop(ValueStop::Undefined(_)) | Halt::SumSeedOutOfDomain) => Ok(None),
            Err(Halt::Stop(ValueStop::Refused(_)) | Halt::Fault(_) | Halt::OnlyNotOne { .. }) => {
                self.fault.get_or_insert_with(|| {
                    "a checked objective addition refused its evaluation contract".to_owned()
                });
                Err(RuleStop::Cancelled(filament_exact::CancelCause::Requested))
            }
        }
    }

    fn add_objective_values(
        &mut self,
        left: &Rational,
        right: &Rational,
    ) -> Result<Option<Rational>, RuleStop> {
        match filament_exact::evaluate_rational_arithmetic(
            filament_exact::RationalArithmetic::Add(left, right),
            None,
            &mut self.system.evaluation_meter(),
        ) {
            Outcome::Completed(value) => Ok(Some(value)),
            Outcome::Incomplete(incomplete) => {
                Err(self.evaluation_stop(incomplete.limit_kind.as_str(), incomplete.limit))
            }
            Outcome::Undefined(_) => Ok(None),
            Outcome::Refused(_) => {
                self.fault.get_or_insert_with(|| {
                    "a checked objective addition refused evaluation".to_owned()
                });
                Err(RuleStop::Cancelled(filament_exact::CancelCause::Requested))
            }
        }
    }

    fn objective_offset(&mut self, plan: &PlanView<'_>) -> Result<Option<Rational>, RuleStop> {
        self.poll_cancellation()?;
        let Some(body) = self.synthesis.objective() else {
            return Ok(Some(Rational::from_integer(Integer::zero())));
        };
        let root = body.root();
        if step_sum_node(root).id() == root.id() {
            return Ok(Some(Rational::from_integer(Integer::zero())));
        }
        let right = match root.kind() {
            NodeKind::Arithmetic(Arithmetic::Add, _, right)
            | NodeKind::Rational {
                operator: filament_exact::ArithmeticOperator::Add,
                right,
                ..
            }
            | NodeKind::Decimal {
                operator: filament_exact::ArithmeticOperator::Add,
                right,
                ..
            } => *right,
            _ => return Ok(None),
        };
        let Some(value) = self.plan_value(plan, 0) else {
            return Ok(None);
        };
        Ok(match self.evaluate_node(root.at(right), value)? {
            Evaluated::Value(value) => Self::exact(&value),
            Evaluated::Undefined => None,
        })
    }

    fn step_sum_objective(
        &mut self,
        plan: &PlanView<'_>,
        _total: &Rational,
    ) -> Result<Option<Rational>, RuleStop> {
        self.objective(plan)
    }

    fn objective(&mut self, plan: &PlanView<'_>) -> Result<Option<Rational>, RuleStop> {
        self.poll_cancellation()?;
        let Some(body) = self.synthesis.objective() else {
            return Ok(Some(Rational::from_integer(Integer::from(
                i64::try_from(plan.length()).unwrap_or(i64::MAX),
            ))));
        };
        if !self.readable(self.objective_reads_final_attributes, plan) {
            return Ok(None);
        }
        let Some(value) = self.plan_value(plan, 0) else {
            return Ok(None);
        };
        Ok(match self.evaluate(body, value)? {
            Evaluated::Value(value) => Self::exact(&value),
            Evaluated::Undefined => None,
        })
    }

    fn begin_replay(&mut self) {
        self.system.begin_plan_replay(self.limits);
    }
}
