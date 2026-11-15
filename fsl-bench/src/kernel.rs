// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked evaluator kernels and their plain-Rust references.

use filament_eval::Evaluation;
use filament_exact::{
    from_admitted, CardinalityBound, CollectionKind, CollectionType, Integer, Meter, Outcome,
    Value, ValueType,
};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_forms::{
    Accumulation, BinaryOperator, BuiltinType, DeclaredName, Expression, FunctionDeclaration,
    TypeForm,
};
use fsl_foundation::Span;
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::object_environment::ObjectEnvironment;

use crate::check::{check, source, SCALAR_UNLIMITED};

/// The two fixed sizes included in the evaluator-kernel benchmark.
pub const SIZES: [usize; 2] = [5, 10];

/// A kernel measured through `CheckedPackage::call` and in plain Rust.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelKind {
    /// `sum(n) = if n > 0 then n + sum(n - 1) else 0`.
    ArithmeticRecursion,
    /// A recursive step guarded by three comparisons.
    ComparisonHeavy,
    /// A fold that sums the sequence `1..=size`.
    CollectionFold,
}

impl KernelKind {
    /// The stable benchmark label.
    pub const fn name(self) -> &'static str {
        match self {
            Self::ArithmeticRecursion => "sum",
            Self::ComparisonHeavy => "compare",
            Self::CollectionFold => "fold",
        }
    }

    /// The logical operation count for this kernel size.
    pub const fn operation_count(self, size: usize) -> usize {
        let _ = self;
        size
    }

    /// The evaluator call-frame count for this kernel size.
    pub const fn frame_count(self, size: usize) -> usize {
        match self {
            Self::ArithmeticRecursion | Self::ComparisonHeavy => size.saturating_add(1),
            Self::CollectionFold => 1,
        }
    }

    /// The plain-Rust equivalent of this kernel at `size`.
    pub fn rust_reference(self, size: usize) -> i64 {
        let size = i64::try_from(size).expect("a benchmark size fits in i64");
        match self {
            Self::ArithmeticRecursion => sum_recursive(size),
            Self::ComparisonHeavy => comparison_recursive(size),
            Self::CollectionFold => (1..=size).sum(),
        }
    }
}

/// The checked package, call inputs and operation counts for one kernel size.
///
/// The same checked package and arguments can be passed to another evaluator
/// backend, including the later JIT benchmark.
pub struct KernelCase {
    /// The kernel being run.
    pub kind: KernelKind,
    /// The size used to generate the kernel input.
    pub size: usize,
    /// The linked checked package containing the kernel entry.
    pub package: CheckedPackage,
    /// Arguments supplied to the named kernel entry.
    pub arguments: Vec<Value>,
    /// The number of logical kernel operations at this size.
    pub operations: u64,
    /// The number of evaluator call frames at this size.
    pub frames: u64,
    /// The expected result from the plain-Rust reference.
    pub rust_result: i64,
}

impl KernelCase {
    /// Build and link one kernel package for reuse by evaluator backends.
    ///
    /// # Panics
    ///
    /// Panics when a generated benchmark input does not check or link.
    pub fn new(kind: KernelKind, size: usize) -> Self {
        assert!(size > 0, "kernel size must be positive");
        let integer = || TypeForm::builtin(BuiltinType::Integer, Span { start: 0, end: 0 });
        let bounded_integer = || {
            TypeForm::builtin(BuiltinType::Int, Span { start: 0, end: 0 })
                .with_bounds(vec!["0".to_owned(), size.to_string()])
        };
        let x = || Expression::name("n".to_owned());
        let subtract_one =
            || Expression::binary(BinaryOperator::Subtract, x(), Expression::integer(1_i64));
        let recursive_call = || Expression::call("kernel", vec![subtract_one()]);
        let (parameters, measure, body, arguments, frames) = match kind {
            KernelKind::ArithmeticRecursion => (
                vec![("n".to_owned(), bounded_integer())],
                Some(x()),
                Expression::if_then_else(
                    Expression::binary(BinaryOperator::Greater, x(), Expression::integer(0_i64)),
                    Expression::binary(BinaryOperator::Add, x(), recursive_call()),
                    Expression::integer(0_i64),
                ),
                vec![Value::Integer(Integer::from(
                    i64::try_from(size).expect("a benchmark size fits in i64"),
                ))],
                u64::try_from(kind.frame_count(size)).expect("benchmark frames fit in u64"),
            ),
            KernelKind::ComparisonHeavy => {
                let positive =
                    || Expression::binary(BinaryOperator::Greater, x(), Expression::integer(0_i64));
                let at_least_one = || {
                    Expression::binary(
                        BinaryOperator::GreaterOrEqual,
                        x(),
                        Expression::integer(1_i64),
                    )
                };
                let nonzero = || {
                    Expression::binary(BinaryOperator::NotEqual, x(), Expression::integer(0_i64))
                };
                (
                    vec![("n".to_owned(), bounded_integer())],
                    Some(x()),
                    Expression::if_then_else(
                        positive(),
                        Expression::if_then_else(
                            at_least_one(),
                            Expression::if_then_else(
                                nonzero(),
                                Expression::binary(
                                    BinaryOperator::Add,
                                    Expression::integer(1_i64),
                                    recursive_call(),
                                ),
                                Expression::integer(0_i64),
                            ),
                            Expression::integer(0_i64),
                        ),
                        Expression::integer(0_i64),
                    ),
                    vec![Value::Integer(Integer::from(
                        i64::try_from(size).expect("a benchmark size fits in i64"),
                    ))],
                    u64::try_from(kind.frame_count(size)).expect("benchmark frames fit in u64"),
                )
            }
            KernelKind::CollectionFold => {
                let count = u64::try_from(size).expect("benchmark collection size fits in u64");
                let collection_type = CollectionType::new(
                    CollectionKind::Sequence,
                    ValueType::Integer,
                    Some(
                        CardinalityBound::new(count, count)
                            .expect("a fixed collection bound is nonempty"),
                    ),
                );
                let source_type =
                    TypeForm::collection(CollectionKind::Sequence, Span { start: 0, end: 0 });
                let source_type = source_type
                    .with_arguments(vec![integer()])
                    .with_bounds(vec![size.to_string(), size.to_string()]);
                let source = Expression::name("items".to_owned());
                let body = Expression::accumulate(
                    Accumulation::Fold,
                    DeclaredName {
                        name: "Total".to_owned(),
                        span: Span { start: 0, end: 0 },
                    },
                    "total",
                    "item",
                    source,
                    Expression::binary(
                        BinaryOperator::Add,
                        Expression::name("total".to_owned()),
                        Expression::name("item".to_owned()),
                    ),
                    Some(Expression::integer(0_i64)),
                );
                let values = (1..=size)
                    .map(|value| {
                        Value::Integer(Integer::from(
                            i64::try_from(value).expect("a benchmark element fits in i64"),
                        ))
                    })
                    .collect();
                let collection = from_admitted(collection_type, values);
                (
                    vec![("items".to_owned(), source_type)],
                    None,
                    body,
                    vec![collection],
                    1,
                )
            }
        };
        let result = if kind == KernelKind::CollectionFold {
            TypeForm::name("Total", Span { start: 0, end: 0 })
        } else {
            integer()
        };
        let declaration = FunctionDeclaration::new("kernel", parameters, result, measure, body);
        let mut declarations = PackageDeclarations::new(
            source(),
            "fsl-bench",
            fsl_foundation::IdentityLimits::default(),
        );
        declarations.functions = vec![declaration];
        if kind == KernelKind::CollectionFold {
            declarations.aliases = vec![("Total".to_owned(), ValueType::Integer)];
        }
        let package = CheckedPackage::link(check(declarations).expect("kernel checks"));
        Self {
            kind,
            size,
            package,
            arguments,
            operations: u64::try_from(kind.operation_count(size))
                .expect("benchmark operations fit in u64"),
            frames,
            rust_result: kind.rust_reference(size),
        }
    }

    /// Call the kernel through the interpreter's checked-package entry.
    pub fn call(&self, objects: &ObjectEnvironment) -> Evaluation {
        let mut meter = Meter::new(SCALAR_UNLIMITED);
        self.package
            .call(
                &QualifiedName::unqualified("kernel").expect("kernel is identifier-shaped"),
                self.arguments.clone(),
                objects,
                &mut meter,
            )
            .expect("kernel resolves and its arguments validate")
    }

    /// Whether `evaluation` completed with the plain-Rust reference value.
    pub fn matches_rust_reference(&self, evaluation: &Evaluation) -> bool {
        matches!(
            &evaluation.outcome,
            filament_eval::FamilyOutcome::Evaluated(
                Outcome::Completed(Value::Integer(value))
            ) if *value == Integer::from(self.rust_result)
        )
    }
}

fn sum_recursive(n: i64) -> i64 {
    if n > 0 {
        n + sum_recursive(n - 1)
    } else {
        0
    }
}

fn comparison_recursive(n: i64) -> i64 {
    if n > 0 {
        if n >= 1 {
            if n != 0 {
                1 + comparison_recursive(n - 1)
            } else {
                0
            }
        } else {
            0
        }
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn smallest_kernels_match_their_plain_rust_references() {
        for kind in [
            KernelKind::ArithmeticRecursion,
            KernelKind::ComparisonHeavy,
            KernelKind::CollectionFold,
        ] {
            let case = KernelCase::new(kind, SIZES[0]);
            assert_eq!(case.operations, 5);
            assert_eq!(
                case.frames,
                match kind {
                    KernelKind::ArithmeticRecursion | KernelKind::ComparisonHeavy => 6,
                    KernelKind::CollectionFold => 1,
                }
            );
            let evaluation = case.call(&ObjectEnvironment::default());
            assert!(
                case.matches_rust_reference(&evaluation),
                "{} at size {} equals its Rust reference",
                kind.name(),
                case.size
            );
        }
    }
}
