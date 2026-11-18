// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "Directly solved assignments" and FR-851 "Eligible conjuncts": what
//! the top-level conjuncts of a postcondition say about the receiver's
//! fields, with each conjunct's pre-state side evaluated once.
//!
//! A conjunct is an *equation* `self.f = e` (either side), or the Boolean
//! field literal `self.b` or `not self.b`; or a *bound* `self.f op e` (either
//! side) with `op` one of `<`, `<=`, `>=`, `>`. In both, `e` reads only
//! `pre(...)` terms, the operation's arguments and literals. Every other
//! conjunct says nothing here and is left to the clause's own evaluation on
//! each candidate. Only the clause's defined prefix is read (FR-851
//! "Defined prefix"): the targets stop at the first conjunct that may be
//! undefined on some candidate, so narrowing never skips a candidate whose
//! clause enumeration would leave undefined. Both routes read their clause through
//! [`NodeSource`], so they find the same conjuncts.

use filament_checked::node::{Connective, NodeKind};
use filament_eval::FamilyOutcome;
use filament_exact::{OrderingOperator, Outcome, Value, ValueType};
use filament_semantic_value::declaration::EqualityOperator;

use super::evaluate::node_source::NodeSource;
use super::CallFailure;

/// What one conjunct of a postcondition says about a field of `self`.
#[derive(Clone, Debug)]
pub enum PostTarget {
    /// `self.field = e`: `value` is `e`'s value, `None` when its evaluation
    /// was not `Completed`.
    Equation {
        /// The field's declared name.
        field: String,
        /// The value of `e`.
        value: Option<Value>,
    },
    /// `self.field operator e`, the field on the left: `value` is `e`'s
    /// value, `None` when its evaluation was not `Completed`.
    Bound {
        /// The field's declared name.
        field: String,
        /// The comparison, read with the field as its left operand.
        operator: OrderingOperator,
        /// The value of `e`.
        value: Option<Value>,
    },
}

/// What a postcondition says about the receiver's fields.
#[derive(Clone, Debug, Default)]
pub struct PostTargets {
    /// Whether the clause names `result`: an application with such a clause
    /// is enumerated (FR-120 "Eligible").
    pub names_result: bool,
    /// The clause's equation and bound conjuncts, in conjunct order.
    pub targets: Vec<PostTarget>,
}

/// The targets of the postcondition `body`, whose slot 0 is `self` and whose
/// slot 1 is `result` when `binds_result`. `evaluate` computes one `e` with
/// a fresh meter; it is called only for an `e` that reads no post-state.
pub(super) fn targets<'a, N: NodeSource<'a>>(
    body: N,
    binds_result: bool,
    mut evaluate: impl FnMut(N) -> Result<FamilyOutcome<Value>, CallFailure>,
) -> Result<PostTargets, CallFailure> {
    if binds_result && reads_slot(body, 1) {
        return Ok(PostTargets {
            names_result: true,
            targets: Vec::new(),
        });
    }
    let mut completed = |node: N| -> Result<Option<Value>, CallFailure> {
        Ok(match evaluate(node)? {
            FamilyOutcome::Evaluated(Outcome::Completed(value)) => Some(value),
            _ => None,
        })
    };
    let mut targets = Vec::new();
    // FR-851 "Defined prefix": a conjunct narrows only while every conjunct
    // before it is defined on every candidate, so each candidate narrowing
    // skips has its clause `Completed(false)` under enumeration.
    for conjunct in conjuncts(body) {
        let target = match conjunct.kind() {
            NodeKind::Attribute { .. } if is_boolean(conjunct) => {
                self_field(conjunct).map(|field| PostTarget::Equation {
                    field,
                    value: Some(Value::Boolean(true)),
                })
            }
            NodeKind::Not(operand) => {
                let operand = conjunct.at(*operand);
                self_field(operand)
                    .filter(|_| is_boolean(operand))
                    .map(|field| PostTarget::Equation {
                        field,
                        value: Some(Value::Boolean(false)),
                    })
            }
            NodeKind::Equality(EqualityOperator::Equal, _, left, right) => {
                match oriented(conjunct.at(*left), conjunct.at(*right)) {
                    Some((field, e)) => Some(PostTarget::Equation {
                        field,
                        value: completed(e)?,
                    }),
                    None => None,
                }
            }
            NodeKind::Order(operator, _, left, right) => {
                let (left, right) = (conjunct.at(*left), conjunct.at(*right));
                if let (Some(field), true) = (self_field(left), pre_only(right)) {
                    Some(PostTarget::Bound {
                        field,
                        operator: *operator,
                        value: completed(right)?,
                    })
                } else if let (Some(field), true) = (self_field(right), pre_only(left)) {
                    Some(PostTarget::Bound {
                        field,
                        operator: flipped(*operator),
                        value: completed(left)?,
                    })
                } else {
                    None
                }
            }
            _ => None,
        };
        match target {
            Some(target) => {
                // A target whose `e` has no value is not defined on every
                // candidate: it ends the prefix, and the application is
                // enumerated (FR-120 "Value").
                let undecided = matches!(
                    &target,
                    PostTarget::Equation { value: None, .. }
                        | PostTarget::Bound { value: None, .. }
                );
                targets.push(target);
                if undecided {
                    break;
                }
            }
            None if always_defined(conjunct) => {}
            None => break,
        }
    }
    Ok(PostTargets {
        names_result: false,
        targets,
    })
}

/// FR-851 "Defined prefix": whether `conjunct`, no equation or bound, is
/// defined on every candidate: a comparison whose operands are each a
/// literal, an operation argument, or `self.g` or `pre(self.g)` of a
/// required Boolean, enumeration or integer field, or `not` of one.
fn always_defined<'a, N: NodeSource<'a>>(conjunct: N) -> bool {
    match conjunct.kind() {
        NodeKind::Not(operand) => always_defined(conjunct.at(*operand)),
        NodeKind::Equality(_, _, left, right) | NodeKind::Order(_, _, left, right) => {
            plain_operand(conjunct.at(*left)) && plain_operand(conjunct.at(*right))
        }
        _ => false,
    }
}

/// A literal, an operation argument, or a read `self.g` or `pre(self.g)`
/// of a required Boolean, enumeration or integer field.
fn plain_operand<'a, N: NodeSource<'a>>(node: N) -> bool {
    let field = |node: N| {
        self_field(node).is_some()
            && matches!(
                node.value_type(),
                ValueType::Boolean | ValueType::Enum(_) | ValueType::Int(_) | ValueType::Integer
            )
    };
    match node.kind() {
        NodeKind::Literal(_) => true,
        NodeKind::Local(slot) => *slot != 0,
        NodeKind::Pre(operand) => field(node.at(*operand)),
        _ => field(node),
    }
}

/// The top-level `and` operands of `body`, left to right.
fn conjuncts<'a, N: NodeSource<'a>>(body: N) -> Vec<N> {
    let mut found = Vec::new();
    let mut pending = vec![body];
    while let Some(node) = pending.pop() {
        if let NodeKind::Connective(Connective::And, _, left, right) = node.kind() {
            pending.push(node.at(*right));
            pending.push(node.at(*left));
        } else {
            found.push(node);
        }
    }
    found
}

/// The field and `e` of `left = right` or `right = left`, when one side is
/// a field of `self` and the other reads no post-state.
fn oriented<'a, N: NodeSource<'a>>(left: N, right: N) -> Option<(String, N)> {
    match (self_field(left), self_field(right)) {
        (Some(field), _) if pre_only(right) => Some((field, right)),
        (_, Some(field)) if pre_only(left) => Some((field, left)),
        _ => None,
    }
}

/// `op` read with its operands swapped: `e < self.f` is `self.f > e`.
const fn flipped(operator: OrderingOperator) -> OrderingOperator {
    match operator {
        OrderingOperator::Less => OrderingOperator::Greater,
        OrderingOperator::LessOrEqual => OrderingOperator::GreaterOrEqual,
        OrderingOperator::Greater => OrderingOperator::Less,
        OrderingOperator::GreaterOrEqual => OrderingOperator::LessOrEqual,
    }
}

fn is_boolean<'a, N: NodeSource<'a>>(node: N) -> bool {
    matches!(node.value_type(), ValueType::Boolean)
}

/// The name of the required field `self.f` that `node` reads in the
/// post-state.
fn self_field<'a, N: NodeSource<'a>>(node: N) -> Option<String> {
    match node.kind() {
        NodeKind::Attribute {
            reference,
            field,
            optional: false,
            ..
        } if matches!(node.at(*reference).kind(), NodeKind::Local(0)) => Some(field.name.clone()),
        _ => None,
    }
}

/// Whether `node` reads only `pre(...)` terms, the operation's arguments
/// and literals, combined by pure operators: never `self` outside
/// `pre(...)`, a post-state read or a call.
///
/// A `pre(...)` term that calls a function or a dispatched query is not
/// read here: the clause evaluator does not give such a call the value an
/// evaluation over the pre-state alone gives it, so solving it could skip a
/// candidate the clause leaves undefined. The equation is left to the clause.
fn pre_only<'a, N: NodeSource<'a>>(node: N) -> bool {
    match node.kind() {
        NodeKind::Pre(_) => !calls(node),
        NodeKind::Literal(_) => true,
        NodeKind::Local(slot) => *slot != 0,
        NodeKind::Coerce(..)
        | NodeKind::Negate(_)
        | NodeKind::Not(_)
        | NodeKind::Arithmetic(..)
        | NodeKind::IntegerDivision { .. }
        | NodeKind::Order(..)
        | NodeKind::Equality(..)
        | NodeKind::Connective(..)
        | NodeKind::If { .. }
        | NodeKind::Let { .. } => node.children().into_iter().all(pre_only),
        _ => false,
    }
}

/// Whether `node` calls a function or a dispatched query anywhere.
fn calls<'a, N: NodeSource<'a>>(node: N) -> bool {
    let mut pending = vec![node];
    while let Some(next) = pending.pop() {
        if matches!(
            next.kind(),
            NodeKind::Call { .. } | NodeKind::ImportedCall { .. } | NodeKind::Dispatch { .. }
        ) {
            return true;
        }
        pending.extend(next.children());
    }
    false
}

/// Whether `node` reads the local `slot` anywhere.
fn reads_slot<'a, N: NodeSource<'a>>(node: N, slot: usize) -> bool {
    let mut pending = vec![node];
    while let Some(next) = pending.pop() {
        if matches!(next.kind(), NodeKind::Local(read) if *read == slot) {
            return true;
        }
        pending.extend(next.children());
    }
    false
}
