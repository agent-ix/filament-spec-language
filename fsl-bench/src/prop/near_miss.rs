// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deliberate semantic refusals with independently authored expected loci.
use super::value::VALUE_HEADER;
use fsl_foundation::{Code, Span};
use proptest::prelude::{BoxedStrategy, Strategy};

/// One deliberate failure family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NearMissKind {
    /// Boolean used as an integer operand.
    WrongDomain,
    /// Integer literal outside its result interval.
    OutOfRange,
    /// Literal zero divisor.
    DivisionByZero,
}

/// Source and the expected typed refusal, constructed before compilation.
#[derive(Clone, Debug)]
pub struct NearMiss {
    /// Failure family for independent downstream assertions.
    pub kind: NearMissKind,
    /// Complete source that parses but must not check.
    pub source: String,
    /// Expected catalog code.
    pub code: Code,
    /// Expected catalog cause.
    pub cause: &'static str,
    /// Whole expression containing the invalid construct.
    pub expression_span: Span,
}

/// Vary literals while preserving a deliberate semantic error.
pub fn near_misses() -> BoxedStrategy<NearMiss> {
    (
        proptest::sample::select(vec![
            NearMissKind::WrongDomain,
            NearMissKind::OutOfRange,
            NearMissKind::DivisionByZero,
        ]),
        5i64..=20,
    )
        .prop_map(|(kind, value)| {
            let (result, expression, code, cause) = match kind {
                NearMissKind::WrongDomain => (
                    "Integer",
                    format!("true + {value}"),
                    Code::IllTyped,
                    "type-mismatch",
                ),
                NearMissKind::OutOfRange => (
                    "Int[0, 3]",
                    value.to_string(),
                    Code::IllTyped,
                    "type-mismatch",
                ),
                NearMissKind::DivisionByZero => (
                    "Integer",
                    format!("{value} div 0"),
                    Code::UndefinedExpression,
                    "unproved-nonzero",
                ),
            };
            let prefix = format!("{VALUE_HEADER}function f using v(): {result} pure {{ ");
            let start = prefix.len();
            let end = start + expression.len();
            NearMiss {
                kind,
                source: format!("{prefix}{expression} }}\n"),
                code,
                cause,
                expression_span: Span { start, end },
            }
        })
        .boxed()
}
