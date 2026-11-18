// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-495-AC-13 (Rational half): a checked Rational operation whose operand
//! is not a Rational is a structural fault no typed body can hold, so the
//! definedness walk refuses it `unproved-range` under both check modes and
//! `Kernel` never leaves it to the evaluator. The bodies are built node by
//! node because the typer admits none of them.

use filament_checked::node::{CheckedLiteral, NodeKind};
use filament_exact::{
    ArithmeticOperator, Integer, IntegerInterval, RationalDomain, Value, ValueType,
};
use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::location::{
    ExpressionLocation as Location, ExpressionOwner as Origin,
};
use ix_trace_rs::trace;

use super::super::ir::{BodyBuilder, CheckedBody, Node, NodeId};
use super::super::{CheckCause, Obligation};
use super::Definedness;

fn node(kind: NodeKind<NodeId>, value_type: ValueType) -> Node {
    Node {
        kind,
        value_type,
        location: Location::root(Origin::Expression),
        equality_producer: None,
        ordering_part: false,
        field_population: None,
        equality_range_context: None,
    }
}

fn domain() -> RationalDomain {
    let interval = |low: i64, high: i64| {
        IntegerInterval::new(Integer::from(low), Integer::from(high)).expect("a non-empty interval")
    };
    RationalDomain::new(interval(0, 1), interval(1, 2)).expect("the denominators are positive")
}

/// `1 + 1` and `-1` over `Integer` operands, each typed as a Rational.
fn malformed_bodies() -> [CheckedBody; 2] {
    let one = || {
        node(
            NodeKind::Literal(CheckedLiteral(Value::Integer(Integer::from(1_i64)))),
            ValueType::Integer,
        )
    };
    let rational = ValueType::Rational(domain());
    let mut sum = BodyBuilder::default();
    let left = sum.push(one());
    let right = sum.push(one());
    let sum = sum.finish(node(
        NodeKind::Rational {
            operator: ArithmeticOperator::Add,
            left,
            right,
            domain: domain(),
        },
        rational.clone(),
    ));
    let mut negate = BodyBuilder::default();
    let operand = negate.push(one());
    let negate = negate.finish(node(NodeKind::RationalNegate(operand, domain()), rational));
    [sum, negate]
}

#[trace("FR-495-AC-13", "TC-1420")]
#[test]
fn a_rational_operation_over_an_integer_operand_is_refused_under_both_check_modes() {
    for body in malformed_bodies() {
        for mode in [CheckMode::Linked, CheckMode::Kernel] {
            let refusal = Definedness::new(0, &[], &[])
                .with_check_mode(mode)
                .check(body.root())
                .expect_err("the malformed operation is refused");
            assert_eq!(
                refusal.cause,
                CheckCause::Unproved(Obligation::RationalRange),
                "{mode:?}"
            );
            assert_eq!(&refusal.location, body.root().location(), "{mode:?}");
        }
    }
}
