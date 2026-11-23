// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deliberate semantic refusals with independently authored expected loci.
use crate::value::VALUE_HEADER;
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
    pub code: &'static str,
    /// Expected catalog cause.
    pub cause: &'static str,
    /// Whole expression containing the invalid construct.
    pub expression_span: (usize, usize),
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
                    "ill_typed",
                    "type-mismatch",
                ),
                NearMissKind::OutOfRange => (
                    "Int[0, 3]",
                    value.to_string(),
                    "undefined_expression",
                    "unproved-range",
                ),
                NearMissKind::DivisionByZero => (
                    "Integer",
                    format!("{value} div 0"),
                    "undefined_expression",
                    "unproved-nonzero",
                ),
            };
            let division = if kind == NearMissKind::DivisionByZero {
                "profile d = \"filament.value.integer-division.euclidean/v1\";\n"
            } else {
                ""
            };
            let prefix = format!("{VALUE_HEADER}{division}function f using v(): {result} pure {{ ");
            let start = prefix.len();
            let end = start + expression.len();
            NearMiss {
                kind,
                source: format!("{prefix}{expression} }}\n"),
                code,
                cause,
                expression_span: (start, end),
            }
        })
        .boxed()
}
