// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051's checked-expression interpreter for finite untimed state clauses.

use std::collections::BTreeMap;

use filament_checked::node::{Arithmetic, Connective, NodeKind, OrderedKind};
use filament_exact::{Rational, Value};
use fsl_semantics::check::{CheckedGraph, CheckedNode, StatePredicate, TimedModel};
use ix_trace_rs::implements;

use crate::ReferenceRefusal;

const MAX_EXPRESSION_NODES: usize = 100_000;
const MAX_EXPRESSION_DEPTH: usize = 256;
const MAX_FINITE_STATES: usize = 1 << 16;

/// One concrete finite state, indexed by object identity and field name.
#[derive(Clone, Debug, Default)]
pub struct ReferenceFiniteState {
    fields: BTreeMap<(String, String), Value>,
}

/// One exact clock valuation in a checked timed model's clock order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceClockState {
    clocks: Vec<Rational>,
}

impl ReferenceClockState {
    /// Creates a nonnegative rational valuation with one value per clock.
    pub fn new(model: &TimedModel, clocks: Vec<Rational>) -> Result<Self, ReferenceRefusal> {
        let zero = Rational::from_integer(filament_exact::Integer::zero());
        if clocks.len() != model.clocks().len() || clocks.iter().any(|value| value < &zero) {
            return Err(unsupported(
                "clock valuation has wrong dimension or negative value",
            ));
        }
        Ok(Self { clocks })
    }

    /// Returns this state's exact clock values in checked clock order.
    pub fn values(&self) -> &[Rational] {
        &self.clocks
    }

    /// Evaluates the indexed checked guard when it is one clock-only
    /// conjunction. Any other guard feature is refused by name.
    #[implements("FR-1051-AC-17")]
    pub fn satisfies_guard(
        &self,
        model: &TimedModel,
        guard_index: usize,
    ) -> Result<bool, ReferenceRefusal> {
        let guard = model
            .guards()
            .get(guard_index)
            .ok_or_else(|| unsupported("missing checked clock guard"))?;
        if guard.parts().len() != 1 {
            return Err(unsupported("disjunctive timed guard"));
        }
        let part = &guard.parts()[0];
        if part.data().is_some()
            || !part.presences().is_empty()
            || !part.element_presences().is_empty()
            || !part.elements().is_empty()
            || !part.families().is_empty()
        {
            return Err(unsupported(
                "timed guard with data or quantified clock access",
            ));
        }
        let mut satisfied = true;
        for constraint in part.clocks() {
            if !constraint.via().is_empty()
                || !constraint.slots().is_empty()
                || constraint.parameter().is_some()
            {
                return Err(unsupported("indirect or parameterized clock constraint"));
            }
            let value = self
                .clocks
                .get(constraint.clock().index())
                .ok_or_else(|| unsupported("clock constraint outside valuation"))?;
            satisfied &= clock_comparison(value, constraint.op(), constraint.bound());
        }
        Ok(satisfied)
    }

    /// Evaluates one checked conjunction of clock bounds from a time invariant.
    #[implements("FR-1051-AC-17")]
    pub fn satisfies_invariant(
        &self,
        model: &TimedModel,
        invariant_index: usize,
    ) -> Result<bool, ReferenceRefusal> {
        let invariant = model
            .invariants()
            .get(invariant_index)
            .ok_or_else(|| unsupported("missing checked time invariant"))?;
        if invariant.alternatives().iter().any(|alternative| {
            !alternative.presences().is_empty()
                || !matches!(
                    alternative.predicate().body(),
                    Some(node) if matches!(node.kind(), NodeKind::Literal(literal) if matches!(literal.0, Value::Boolean(true)))
                )
        }) {
            return Err(unsupported("time invariant with a non-constant data predicate"));
        }
        let mut satisfied = true;
        for constraint in invariant.bounds() {
            if !constraint.via().is_empty()
                || !constraint.slots().is_empty()
                || constraint.parameter().is_some()
            {
                return Err(unsupported("indirect or parameterized clock invariant"));
            }
            let value = self
                .clocks
                .get(constraint.clock().index())
                .ok_or_else(|| unsupported("clock invariant outside valuation"))?;
            satisfied &= clock_comparison(value, constraint.op(), constraint.bound());
        }
        Ok(satisfied)
    }

    /// Applies the indexed checked reset clause when all its resets are to zero.
    #[implements("FR-1051-AC-17")]
    pub fn apply_zero_resets(
        &self,
        model: &TimedModel,
        reset_index: usize,
    ) -> Result<Self, ReferenceRefusal> {
        let resets = model
            .resets()
            .get(reset_index)
            .ok_or_else(|| unsupported("missing checked clock reset"))?;
        let mut clocks = self.clocks.clone();
        for reset in resets.resets() {
            if !reset.value().is_zero()
                || reset.parameter().is_some()
                || reset.selection().is_some()
            {
                return Err(unsupported("nonzero or selected clock reset"));
            }
            let value = clocks
                .get_mut(reset.clock().index())
                .ok_or_else(|| unsupported("clock reset outside valuation"))?;
            *value = Rational::from_integer(filament_exact::Integer::zero());
        }
        Ok(Self { clocks })
    }

    /// Advances every clock by the same exact nonnegative dense-time delay.
    pub fn delay(&self, delta: &Rational) -> Result<Self, ReferenceRefusal> {
        if delta < &Rational::from_integer(filament_exact::Integer::zero()) {
            return Err(unsupported("negative clock delay"));
        }
        Ok(Self {
            clocks: self.clocks.iter().map(|clock| clock.add(delta)).collect(),
        })
    }
}

fn clock_comparison(
    value: &Rational,
    operator: fsl_semantics::check::ClockOp,
    bound: &Rational,
) -> bool {
    use fsl_semantics::check::ClockOp;
    match operator {
        ClockOp::Less => value < bound,
        ClockOp::LessOrEqual => value <= bound,
        ClockOp::Equal => value == bound,
        ClockOp::GreaterOrEqual => value >= bound,
        ClockOp::Greater => value > bound,
    }
}

impl ReferenceFiniteState {
    /// Creates a state from the supplied `(object, field, value)` entries.
    pub fn new(
        fields: impl IntoIterator<Item = (String, String, Value)>,
    ) -> Result<Self, ReferenceRefusal> {
        let mut state = Self::default();
        for (object, field, value) in fields {
            if !finite_state_value(&value) {
                return Err(unsupported("non-finite or non-admitted state field value"));
            }
            let key = (object, field);
            if state.fields.insert(key, value).is_some() {
                return Err(ReferenceRefusal {
                    construct: "duplicate finite-state field".to_owned(),
                });
            }
        }
        Ok(state)
    }

    /// Reads a field value from this state.
    pub fn field(&self, object: &str, field: &str) -> Option<&Value> {
        self.fields.get(&(object.to_owned(), field.to_owned()))
    }

    /// Enumerates the Cartesian product of finite scalar field domains in
    /// deterministic field order, refusing products larger than 2^16.
    #[implements("FR-1051-AC-17")]
    pub fn enumerate(
        fields: Vec<(String, String, Vec<Value>)>,
    ) -> Result<Vec<Self>, ReferenceRefusal> {
        let mut seen = std::collections::BTreeSet::new();
        let mut valuation_count = 1_usize;
        for (object, field, values) in &fields {
            if !seen.insert((object.clone(), field.clone())) {
                return Err(unsupported("duplicate finite-state field domain"));
            }
            if values.is_empty() || values.iter().any(|value| !finite_state_value(value)) {
                return Err(unsupported("empty or non-finite field domain"));
            }
            valuation_count = valuation_count
                .checked_mul(values.len())
                .filter(|count| *count <= MAX_FINITE_STATES)
                .ok_or_else(|| unsupported("finite state space exceeds 2^16 valuations"))?;
        }
        let mut states = vec![Self::default()];
        for (object, field, values) in fields {
            let mut expanded = Vec::with_capacity(valuation_count);
            for state in states {
                for value in &values {
                    let mut next = state.clone();
                    next.fields
                        .insert((object.clone(), field.clone()), value.clone());
                    expanded.push(next);
                }
            }
            states = expanded;
        }
        Ok(states)
    }
}

/// Interprets a checked Boolean expression over explicit current and pre-state
/// values. The admitted expression fragment is literals, locals, field reads,
/// integer comparisons, equality, conjunction, negation, and integer addition
/// or subtraction by a literal. Every other checked node refuses by name.
#[implements("FR-1051-AC-17")]
pub fn evaluate_boolean(
    graph: &CheckedGraph,
    expression: CheckedNode<'_>,
    locals: &[Value],
    pre_state: &ReferenceFiniteState,
    current_state: &ReferenceFiniteState,
) -> Result<bool, ReferenceRefusal> {
    let mut budget = MAX_EXPRESSION_NODES;
    let value = evaluate_value(
        graph,
        expression,
        locals,
        pre_state,
        current_state,
        0,
        &mut budget,
    )?;
    match value {
        Value::Boolean(value) => Ok(value),
        _ => Err(ReferenceRefusal {
            construct: "checked claim expression is not Boolean".to_owned(),
        }),
    }
}

fn evaluate_value(
    graph: &CheckedGraph,
    expression: CheckedNode<'_>,
    locals: &[Value],
    pre_state: &ReferenceFiniteState,
    active_state: &ReferenceFiniteState,
    depth: usize,
    budget: &mut usize,
) -> Result<Value, ReferenceRefusal> {
    if depth > MAX_EXPRESSION_DEPTH {
        return Err(ReferenceRefusal {
            construct: "checked expression exceeds the reference depth bound".to_owned(),
        });
    }
    *budget = budget.checked_sub(1).ok_or_else(|| ReferenceRefusal {
        construct: "checked expression exceeds the reference node bound".to_owned(),
    })?;
    let child = |node, state: &ReferenceFiniteState, budget: &mut usize| {
        evaluate_value(
            graph,
            expression.at(node),
            locals,
            pre_state,
            state,
            depth + 1,
            budget,
        )
    };
    match expression.kind() {
        NodeKind::Literal(literal) => match &literal.0 {
            Value::Boolean(_) | Value::Integer(_) | Value::Rational(_) | Value::Enum(_) => {
                Ok(literal.0.clone())
            }
            _ => Err(unsupported("non-finite literal value")),
        },
        NodeKind::Coerce(operand, interval) => {
            let value = child(*operand, active_state, budget)?;
            let Value::Integer(integer) = &value else {
                return Err(unsupported("non-integer finite coercion"));
            };
            if integer < interval.lower() || integer > interval.upper() {
                return Err(unsupported("integer outside checked finite interval"));
            }
            Ok(value)
        }
        NodeKind::Local(slot) => locals
            .get(*slot)
            .cloned()
            .ok_or_else(|| unsupported("local slot outside checked frame")),
        NodeKind::Attribute {
            reference, field, ..
        } => {
            let reference = child(*reference, active_state, budget)?;
            let Value::Reference(reference) = reference else {
                return Err(unsupported("attribute read without an object reference"));
            };
            active_state
                .field(reference.object().as_str(), &field.name)
                .cloned()
                .ok_or_else(|| unsupported("field absent from finite state"))
        }
        NodeKind::Order(operator, kind, left, right) => {
            let left = child(*left, active_state, budget)?;
            let right = child(*right, active_state, budget)?;
            let ordering = match (kind, &left, &right) {
                (OrderedKind::Integers, Value::Integer(left), Value::Integer(right)) => {
                    left.cmp(right)
                }
                (OrderedKind::Rationals, Value::Rational(left), Value::Rational(right)) => {
                    left.cmp(right)
                }
                (OrderedKind::Enums, Value::Enum(left), Value::Enum(right)) => {
                    left.rank().cmp(&right.rank())
                }
                _ => return Err(unsupported("non-finite ordered comparison")),
            };
            Ok(Value::Boolean(operator.holds(ordering)))
        }
        NodeKind::Equality(operator, _, left, right) => {
            let left = child(*left, active_state, budget)?;
            let right = child(*right, active_state, budget)?;
            if !finite_scalar(&left) || !finite_scalar(&right) {
                return Err(unsupported("equality outside finite scalar values"));
            }
            if std::mem::discriminant(&left) != std::mem::discriminant(&right) {
                return Err(unsupported("equality across incompatible scalar types"));
            }
            let equal = finite_scalar_equal(&left, &right);
            Ok(Value::Boolean(match operator {
                filament_semantic_value::declaration::EqualityOperator::Equal => equal,
                filament_semantic_value::declaration::EqualityOperator::NotEqual => !equal,
            }))
        }
        NodeKind::Connective(Connective::And, _, left, right) => {
            let left = child(*left, active_state, budget)?;
            let Value::Boolean(left) = left else {
                return Err(unsupported("non-Boolean conjunction operand"));
            };
            let right = child(*right, active_state, budget)?;
            let Value::Boolean(right) = right else {
                return Err(unsupported("non-Boolean conjunction operand"));
            };
            Ok(Value::Boolean(left && right))
        }
        NodeKind::Connective(Connective::Or, ..) => Err(unsupported("disjunction")),
        NodeKind::Connective(Connective::Implies, ..) => Err(unsupported("implication")),
        NodeKind::Not(operand) => {
            let value = child(*operand, active_state, budget)?;
            let Value::Boolean(value) = value else {
                return Err(unsupported("non-Boolean negation operand"));
            };
            Ok(Value::Boolean(!value))
        }
        NodeKind::Arithmetic(Arithmetic::Add, left, right)
        | NodeKind::Arithmetic(Arithmetic::Subtract, left, right) => {
            let is_subtract = matches!(
                expression.kind(),
                NodeKind::Arithmetic(Arithmetic::Subtract, ..)
            );
            let left_is_literal = matches!(
                expression.at(*left).kind(),
                NodeKind::Literal(literal) if matches!(literal.0, Value::Integer(_))
            );
            let right_is_literal = matches!(
                expression.at(*right).kind(),
                NodeKind::Literal(literal) if matches!(literal.0, Value::Integer(_))
            );
            if left_is_literal == right_is_literal {
                return Err(unsupported("non-linear integer term"));
            }
            if is_subtract && left_is_literal {
                return Err(unsupported("literal-minus-state subtraction"));
            }
            let left_value = child(*left, active_state, budget)?;
            let right_value = child(*right, active_state, budget)?;
            match (left_value, right_value) {
                (Value::Integer(left), Value::Integer(right)) => {
                    Ok(Value::Integer(if is_subtract {
                        left.sub(&right)
                    } else {
                        left.add(&right)
                    }))
                }
                _ => Err(unsupported("integer arithmetic on non-integer values")),
            }
        }
        NodeKind::Pre(operand) => child(*operand, pre_state, budget),
        kind => Err(unsupported(node_name(kind))),
    }
}

fn finite_scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Boolean(_) | Value::Integer(_) | Value::Rational(_) | Value::Enum(_)
    )
}

fn finite_state_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Boolean(_) | Value::Integer(_) | Value::Enum(_)
    )
}

fn node_name<Child, Literal>(kind: &NodeKind<Child, Literal>) -> &'static str {
    match kind {
        NodeKind::Literal(_) => "literal",
        NodeKind::ReferenceLiteral { .. } => "reference literal",
        NodeKind::Coerce(..) => "integer coercion",
        NodeKind::Local(_) => "local read",
        NodeKind::Let { .. } => "let binding",
        NodeKind::If { .. } => "conditional expression",
        NodeKind::Arithmetic(Arithmetic::Multiply, ..) => "multiplication",
        NodeKind::Arithmetic(..) => "integer arithmetic",
        NodeKind::IntegerDivision { .. } => "integer division",
        NodeKind::Negate(_) => "integer negation",
        NodeKind::Divide { .. } => "rational division",
        NodeKind::Rational { .. } | NodeKind::RationalNegate(..) => "rational arithmetic",
        NodeKind::Decimal { .. } | NodeKind::DecimalNegate(..) => "decimal arithmetic",
        NodeKind::Ieee(..)
        | NodeKind::IeeeSquareRoot(..)
        | NodeKind::IeeeFloat32Fma(..)
        | NodeKind::IeeeFloat64Fma(..) => "floating-point operation",
        NodeKind::Quantity(..)
        | NodeKind::QuantityScale { .. }
        | NodeKind::QuantityScaleDiv { .. }
        | NodeKind::QuantityNegate(..)
        | NodeKind::QuantityPower(..) => "quantity operation",
        NodeKind::ConvertDecimal(..)
        | NodeKind::ConvertScalar(..)
        | NodeKind::ConvertText(..)
        | NodeKind::IeeeToRational(..) => "numeric conversion",
        NodeKind::Order(..) => "ordered comparison",
        NodeKind::Equality(..) | NodeKind::IeeeNumericEquality(..) => "equality comparison",
        NodeKind::IeeeTotalOrder(..) => "floating-point ordering",
        NodeKind::IeeeBitIdentical(..) => "floating-point identity comparison",
        NodeKind::Connective(Connective::And, ..) => "conjunction",
        NodeKind::Connective(Connective::Or, ..) => "disjunction",
        NodeKind::Connective(Connective::Implies, ..) => "implication",
        NodeKind::Not(_) => "negation",
        NodeKind::Field { .. } => "record field read",
        NodeKind::Attribute { .. } => "model field read",
        NodeKind::TupleProject { .. } => "tuple projection",
        NodeKind::Index { .. } => "collection indexing",
        NodeKind::Navigate { .. } => "relationship navigation",
        NodeKind::Present(_) | NodeKind::Value(_) | NodeKind::OptionSome(_) => "optional value",
        NodeKind::PlaceReference { .. }
        | NodeKind::PlacePresence { .. }
        | NodeKind::SlotPresence { .. } => "clock place access",
        NodeKind::Call { .. } => "called function",
        NodeKind::ImportedCall { .. } => "imported function call",
        NodeKind::Tuple { .. } | NodeKind::Record { .. } | NodeKind::UnionValue { .. } => {
            "composite construction"
        }
        NodeKind::Collection { .. } | NodeKind::ConvertCollection { .. } => "collection operation",
        NodeKind::Query { .. }
        | NodeKind::Fold { .. }
        | NodeKind::Flatten(_)
        | NodeKind::Size(_) => "collection query",
        NodeKind::Contains(..) => "collection membership",
        NodeKind::BytesLength(_) | NodeKind::BytesConcat(..) => "byte operation",
        NodeKind::DropFirst(_) | NodeKind::RemoveOne(..) => "collection mutation",
        NodeKind::AllInstances { .. }
        | NodeKind::NamedPopulation { .. }
        | NodeKind::PopulationObjects { .. }
        | NodeKind::Lookup { .. } => "population operation",
        NodeKind::Pre(_) => "pre-state expression",
        NodeKind::Reaches { .. } | NodeKind::ReachesEnd { .. } => "reachability predicate",
        NodeKind::ProtocolReference { .. } => "protocol reference",
        NodeKind::Dispatch { .. } => "dispatched function call",
        NodeKind::Case { .. } => "union case selection",
        NodeKind::History { .. } => "history read",
        NodeKind::Seq { .. } => "sequence construction",
    }
}

fn finite_scalar_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Boolean(left), Value::Boolean(right)) => left == right,
        (Value::Integer(left), Value::Integer(right)) => left == right,
        (Value::Rational(left), Value::Rational(right)) => left == right,
        (Value::Enum(left), Value::Enum(right)) => left == right,
        _ => false,
    }
}

fn unsupported(construct: impl Into<String>) -> ReferenceRefusal {
    ReferenceRefusal {
        construct: construct.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::ReferenceFiniteState;
    use filament_exact::{Integer, Rational, Value};
    use ix_trace_rs::trace;

    #[trace("FR-1051-AC-17")]
    #[test]
    fn finite_boolean_fields_enumerate_the_complete_product() {
        let states = ReferenceFiniteState::enumerate(vec![
            (
                "a".to_owned(),
                "left".to_owned(),
                vec![Value::Boolean(false), Value::Boolean(true)],
            ),
            (
                "a".to_owned(),
                "right".to_owned(),
                vec![
                    Value::Integer(Integer::from(0_i64)),
                    Value::Integer(Integer::from(1_i64)),
                ],
            ),
        ])
        .expect("the four finite valuations fit the checked state cap");
        assert_eq!(states.len(), 4);
        assert!(ReferenceFiniteState::enumerate(vec![(
            "a".to_owned(),
            "large".to_owned(),
            (0..=65_536)
                .map(|value| Value::Integer(Integer::from(value as i64)))
                .collect(),
        )])
        .is_err());
        assert!(ReferenceFiniteState::enumerate(vec![(
            "a".to_owned(),
            "rational".to_owned(),
            vec![Value::Rational(Rational::from_integer(Integer::one()))],
        )])
        .is_err());
    }
}
