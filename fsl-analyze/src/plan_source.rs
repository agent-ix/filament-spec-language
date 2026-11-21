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

use filament_checked::node::{NodeKind, Visit};
use filament_exact::{
    from_admitted, from_admitted_slots, CollectionKind, CollectionType, EnumMember, FieldValue,
    Integer, Outcome, Rational, ScalarLimits, Value, ValueType,
};
use filament_semantic_value::declaration::CompositeShape;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{read_clause, ClauseReading};
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_package::CheckedPackage;
use fsl_semantics::check::{
    CheckedBody, CheckedPlan, CheckedSynthesis, PlanOperation, TemporalClauseNode,
};
use fsl_semantics::model::state::StateObject;

use crate::plan_search::{Direction, MonitorStep, ObjectiveKind, PlanRules, PlanView, RuleStop};

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
    /// An invalid producer argument; building/rendering the step fails closed.
    Invalid,
}

impl ArgumentKey {
    fn of(text: &str) -> Self {
        match argument_value(text) {
            Some(Value::Boolean(value)) => Self::Boolean(value),
            Some(Value::Integer(value)) => Self::Integer(value),
            _ => Self::Invalid,
        }
    }
}

/// The value a step argument's canonical typed text (FSpec FR-181) reads
/// back to, for the Boolean and integer arguments a step record carries.
fn argument_value(text: &str) -> Option<Value> {
    let document: serde_json::Value = serde_json::from_str(text).ok()?;
    let object = document.as_object()?;
    match object.get("type")?.as_str()? {
        "integer" => {
            let digits = object.get("value")?.as_str()?;
            Some(Value::Integer(digits.parse::<Integer>().ok()?))
        }
        "boolean" => Some(Value::Boolean(object.get("value")?.as_bool()?)),
        _ => None,
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
    trace_ids: BTreeMap<(usize, Vec<Vec<u8>>), usize>,
    fault: Option<String>,
}

impl<'a> SourcePlan<'a> {
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
        if !policies.is_empty()
            && !matches!(synthesis.domain(), ValueType::Int(domain) if domain.upper().is_zero())
        {
            return Err(PlanSourceError::UnsupportedForm {
                form: "positive-length temporal policy planning with canonical monitor states"
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
            direction,
            kind,
            merges,
            goal_reads_final_attributes,
            objective_reads_final_attributes,
            policies,
            traces: Vec::new(),
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
        step: &ModelTransition,
        index: usize,
        before: Value,
        after: Value,
    ) -> Option<Value> {
        let shape = self.plan.shape();
        let operation = self.operation(step)?;
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
                let value = operation
                    .parameters
                    .iter()
                    .position(|(parameter, _)| parameter == name)
                    .and_then(|position| step.arguments().get(position))
                    .and_then(|text| argument_value(text));
                value.map_or(FieldValue::Absent, FieldValue::Present)
            })
            .collect::<Vec<_>>();
        let arguments = from_admitted_slots(shape.arguments.clone(), slots.into_boxed_slice());
        Some(from_admitted_slots(
            shape.step.clone(),
            vec![
                FieldValue::Present(Value::Integer(Integer::from(i64::try_from(index).ok()?))),
                FieldValue::Present(Value::Enum(EnumMember::new(member.variant(), rank))),
                FieldValue::Present(arguments),
                FieldValue::Present(before),
                FieldValue::Present(after),
            ]
            .into_boxed_slice(),
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

    fn evaluate(&mut self, body: &CheckedBody, plan: Value) -> Result<Evaluated, RuleStop> {
        let evaluation = fsl_eval::value::evaluate_plan_body(
            self.package,
            self.plan.scope(),
            body,
            vec![Some(plan)],
            &mut self.system.evaluation_meter(),
        );
        match evaluation {
            Ok(evaluation) => match evaluation.outcome {
                filament_eval::FamilyOutcome::Evaluated(Outcome::Completed(value)) => {
                    Ok(Evaluated::Value(value))
                }
                filament_eval::FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
                    Err(RuleStop::Accounting {
                        counter: incomplete.limit_kind.as_str(),
                        value: incomplete.limit,
                    })
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
                return Err(RuleStop::Accounting { counter, value });
            }
            Ok(_) => {}
            Err(error) => {
                self.fault.get_or_insert_with(|| error.to_string());
                return Ok(MonitorStep::Reject);
            }
        }
        let keys = prefix
            .iter()
            .map(|read| self.system.state_key(read).map(|(key, _)| key))
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

    /// The exact value of an objective evaluation.
    fn exact(value: &Value) -> Option<Rational> {
        match value {
            Value::Integer(value) => Some(Rational::from_integer(value.clone())),
            Value::Rational(value) => Some(value.clone()),
            Value::Decimal(value) => Some(value.representation().to_rational()),
            _ => None,
        }
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
    let root = body.root();
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

impl PlanRules for SourcePlan<'_> {
    type StepKey = (usize, Vec<ArgumentKey>);

    fn direction(&self) -> Direction {
        self.direction
    }

    fn objective_kind(&self) -> ObjectiveKind {
        self.kind
    }

    fn merges_prefixes(&self) -> bool {
        self.merges
    }

    fn step_key(&self, step: &ModelTransition) -> Self::StepKey {
        (
            self.operations
                .get(step.operation())
                .copied()
                .unwrap_or(usize::MAX),
            step.arguments()
                .iter()
                .map(|text| ArgumentKey::of(text))
                .collect(),
        )
    }

    fn policy_count(&self) -> usize {
        self.policies.len()
    }

    fn policy_start(
        &mut self,
        policy: usize,
        state: &ModelSimState,
    ) -> Result<MonitorStep, RuleStop> {
        self.advance(policy, Vec::new(), state)
    }

    fn policy_advance(
        &mut self,
        policy: usize,
        monitor: &[u8],
        state: &ModelSimState,
    ) -> Result<MonitorStep, RuleStop> {
        let prefix = self.prefix(monitor)?;
        self.advance(policy, prefix, state)
    }

    fn policy_accepts(
        &mut self,
        policy: usize,
        monitor: &[u8],
        _last: &ModelSimState,
    ) -> Result<bool, RuleStop> {
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
                    return Err(RuleStop::Accounting { counter, value });
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
        let Some(body) = self.synthesis.objective() else {
            return Ok(Some(Rational::from_integer(Integer::one())));
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
        Ok(match self.evaluate(body, value)? {
            Evaluated::Value(value) => Self::exact(&value),
            Evaluated::Undefined => None,
        })
    }

    fn objective(&mut self, plan: &PlanView<'_>) -> Result<Option<Rational>, RuleStop> {
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
        self.system.share_evaluation_meter(self.limits);
    }
}
