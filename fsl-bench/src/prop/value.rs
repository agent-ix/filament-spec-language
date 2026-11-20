// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed source construction: the strategy never asks the checker to filter values.

use proptest::prelude::{BoxedStrategy, Just, Strategy};
use proptest::prop_oneof;

/// Header for closed value-profile inputs.
pub const VALUE_HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";

/// Independently inspectable parameter and result domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type {
    /// Unbounded exact integer.
    Integer,
    /// Inclusive integer interval.
    BoundedInt {
        /// Inclusive lower bound.
        lower: i64,
        /// Inclusive upper bound.
        upper: i64,
    },
    /// Boolean truth values.
    Boolean,
    /// Bounded numerator and denominator.
    Rational,
    /// Bounded coefficient and scale with explicit rounding.
    Decimal {
        /// Explicit coefficient-rounding policy.
        mode: DecimalMode,
    },
    /// Bounded scalar text, unicode-scalars profile.
    Text,
    /// Optional member.
    Option(Box<Type>),
    /// A one-field named record.
    Record(Box<Type>),
    /// A named two-member tuple.
    Tuple(Box<Type>),
    /// Bounded sequence, set or bag.
    Collection {
        /// Collection ordering and multiplicity.
        kind: CollectionKind,
        /// Domain of each member.
        member: Box<Type>,
    },
}

/// Decimal modes admitted by the value grammar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecimalMode {
    /// Exact rounding.
    Exact,
    /// Half-even rounding.
    HalfEven,
}

/// Collection domains covered by these generators.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollectionKind {
    /// Ordered sequence.
    Sequence,
    /// Unique unordered members.
    Set,
    /// Unordered members with multiplicity.
    Bag,
}

impl Type {
    /// Source spelling and any named declarations required by this domain.
    pub fn source(&self) -> (String, String) {
        match self {
            Self::Integer => ("Integer".into(), String::new()),
            Self::BoundedInt { lower, upper } => (format!("Int[{lower}, {upper}]"), String::new()),
            Self::Boolean => ("Boolean".into(), String::new()),
            Self::Rational => ("Rational[-20, 20; 1, 9]".into(), String::new()),
            Self::Decimal { mode } => (
                format!(
                    "Decimal[-20, 20; 0, 3; {}]",
                    match mode {
                        DecimalMode::Exact => "exact",
                        DecimalMode::HalfEven => "nearest-even",
                    }
                ),
                String::new(),
            ),
            Self::Text => ("Text[0, 16; unicode-scalars]".into(), String::new()),
            Self::Option(member) => {
                let (ty, declarations) = member.source();
                (format!("Option<{ty}>"), declarations)
            }
            Self::Record(member) => {
                let (ty, declarations) = member.source();
                (
                    "RecordValue".into(),
                    format!("{declarations}record RecordValue {{ field: {ty}; }}\n"),
                )
            }
            Self::Tuple(member) => {
                let (ty, declarations) = member.source();
                (
                    "TupleValue".into(),
                    format!("{declarations}tuple TupleValue({ty}, Boolean);\n"),
                )
            }
            Self::Collection { kind, member } => {
                let (ty, declarations) = member.source();
                let name = match kind {
                    CollectionKind::Sequence => "Sequence",
                    CollectionKind::Set => "Set",
                    CollectionKind::Bag => "Bag",
                };
                (format!("{name}<{ty}>[0, 4]"), declarations)
            }
        }
    }
}

/// The independently traversable expression tree, before parser or checker involvement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    /// Named parameter.
    Parameter(String),
    /// Exact integer literal.
    Integer(i64),
    /// Boolean literal.
    Boolean(bool),
    /// ASCII text literal.
    Text(String),
    /// Exact rational numerator and positive denominator.
    Rational {
        /// Signed numerator.
        numerator: i64,
        /// Positive denominator.
        denominator: u8,
    },
    /// Exact decimal coefficient and scale.
    Decimal {
        /// Base-ten coefficient.
        coefficient: i64,
        /// Nonnegative decimal scale.
        scale: u8,
    },
    /// A present optional value.
    OptionSome(Box<Expr>),
    /// A typed local binding, preserving a refined member domain in constructors.
    Let {
        /// Local binding name.
        name: String,
        /// Declared binding domain.
        ty: Type,
        /// Initializer evaluated outside the new binding's scope.
        value: Box<Expr>,
        /// Expression evaluated with the new binding in scope.
        body: Box<Expr>,
    },
    /// A one-field record constructor.
    Record(Box<Expr>),
    /// A two-member tuple constructor.
    Tuple(Box<Expr>, bool),
    /// A finite collection constructor.
    Collection {
        /// Source constructor to use.
        kind: CollectionKind,
        /// Members in source order.
        members: Vec<Expr>,
    },
    /// Arithmetic addition with integer operands.
    Add(Box<Expr>, Box<Expr>),
    /// Arithmetic subtraction with integer operands.
    Subtract(Box<Expr>, Box<Expr>),
    /// Arithmetic multiplication with integer operands.
    Multiply(Box<Expr>, Box<Expr>),
    /// Integer ordering comparison.
    Less(Box<Expr>, Box<Expr>),
    /// Boolean conjunction.
    And(Box<Expr>, Box<Expr>),
    /// Boolean disjunction.
    Or(Box<Expr>, Box<Expr>),
    /// Boolean negation.
    Not(Box<Expr>),
    /// Equality of two values in the same domain.
    Equal(Box<Expr>, Box<Expr>),
    /// Universal aggregate over a finite collection.
    ForAll {
        /// Lexical member binding.
        binder: String,
        /// Collection to quantify over.
        collection: Box<Expr>,
        /// Boolean expression in the binder's scope.
        predicate: Box<Expr>,
    },
    /// Branch on a Boolean with same-domain arms.
    If {
        /// Boolean condition.
        condition: Box<Expr>,
        /// Arm evaluated when the condition is true.
        yes: Box<Expr>,
        /// Arm evaluated when the condition is false.
        no: Box<Expr>,
    },
}

impl Expr {
    /// Render the expression without consulting the compiler.
    pub fn source(&self) -> String {
        match self {
            Self::Parameter(name) => name.clone(),
            Self::Integer(value) => value.to_string(),
            Self::Boolean(value) => value.to_string(),
            Self::Text(value) => format!("Text({value:?})"),
            Self::Rational {
                numerator,
                denominator,
            } => format!("rational({numerator}, {denominator})"),
            Self::Decimal { coefficient, scale } => format!("decimal({coefficient}, {scale})"),
            Self::OptionSome(member) => format!("Some({})", member.source()),
            Self::Let {
                name,
                ty,
                value,
                body,
            } => format!(
                "let {name}: {} = ({}) in ({})",
                ty.source().0,
                value.source(),
                body.source()
            ),
            Self::Record(member) => format!("RecordValue {{ field: {} }}", member.source()),
            Self::Tuple(member, flag) => format!("TupleValue({}, {flag})", member.source()),
            Self::Collection { kind, members } => {
                let constructor = match kind {
                    CollectionKind::Sequence => "sequence",
                    CollectionKind::Set => "set",
                    CollectionKind::Bag => "bag",
                };
                format!(
                    "{constructor}[{}]",
                    members
                        .iter()
                        .map(Self::source)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Self::Add(left, right) => format!("({} + {})", left.source(), right.source()),
            Self::Subtract(left, right) => format!("({} - {})", left.source(), right.source()),
            Self::Multiply(left, right) => format!("({} * {})", left.source(), right.source()),
            Self::Less(left, right) => format!("({} < {})", left.source(), right.source()),
            Self::And(left, right) => format!("({} and {})", left.source(), right.source()),
            Self::Or(left, right) => format!("({} or {})", left.source(), right.source()),
            Self::Not(value) => format!("not ({})", value.source()),
            Self::Equal(left, right) => format!("({} = {})", left.source(), right.source()),
            Self::ForAll {
                binder,
                collection,
                predicate,
            } => format!(
                "forall({binder} in {}: {})",
                collection.source(),
                predicate.source()
            ),
            Self::If { condition, yes, no } => format!(
                "if {} then ({}) else ({})",
                condition.source(),
                yes.source(),
                no.source()
            ),
        }
    }
}

fn inhabit(ty: &Type, choice: u8) -> Expr {
    match ty {
        Type::Integer => Expr::Integer(i64::from(choice)),
        Type::BoundedInt { lower, upper } => {
            Expr::Integer(lower + i64::from(choice) % (upper - lower + 1))
        }
        Type::Boolean => Expr::Boolean(choice % 2 == 0),
        Type::Rational => Expr::Rational {
            numerator: i64::from(choice % 10),
            denominator: choice % 3 + 1,
        },
        Type::Decimal { .. } => Expr::Decimal {
            coefficient: i64::from(choice % 20),
            scale: choice % 3,
        },
        Type::Text => Expr::Text("a".repeat(usize::from(choice % 10))),
        Type::Option(member) => Expr::Let {
            name: "some_member".into(),
            ty: (**member).clone(),
            value: Box::new(inhabit(member, choice)),
            body: Box::new(Expr::OptionSome(Box::new(Expr::Parameter(
                "some_member".into(),
            )))),
        },
        Type::Record(member) => Expr::Record(Box::new(inhabit(member, choice))),
        Type::Tuple(member) => Expr::Tuple(Box::new(inhabit(member, choice)), choice % 2 == 0),
        Type::Collection { kind, member } => Expr::Collection {
            kind: *kind,
            members: vec![inhabit(member, choice)],
        },
    }
}

/// A valid-by-construction function and its independent description.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedExpression {
    /// Named parameters in declaration order.
    pub parameters: Vec<(String, Type)>,
    /// Declared result domain.
    pub result: Type,
    /// Expression before source rendering.
    pub expression: Expr,
}

impl TypedExpression {
    /// Complete source suitable for parsing and Linked checking.
    pub fn source(&self) -> String {
        let (result, mut declarations) = self.result.source();
        let parameters = self
            .parameters
            .iter()
            .map(|(name, ty)| {
                let (ty, needed) = ty.source();
                if !declarations.contains(&needed) {
                    declarations.push_str(&needed);
                }
                format!("{name}: {ty}")
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("{VALUE_HEADER}{declarations}function f using v({parameters}): {result} pure {{ {} }}\n", self.expression.source())
    }
}

/// Scalar types with small, explicit domains.
pub fn scalar_types() -> BoxedStrategy<Type> {
    prop_oneof![
        Just(Type::Integer),
        (-8i64..=0, 0i64..=8).prop_map(|(lower, upper)| Type::BoundedInt { lower, upper }),
        Just(Type::Boolean),
        Just(Type::Rational),
        Just(Type::Decimal {
            mode: DecimalMode::Exact
        }),
        Just(Type::Decimal {
            mode: DecimalMode::HalfEven
        }),
        Just(Type::Text)
    ]
    .boxed()
}

/// Bounded compound domains; named compounds contain scalar leaves so names stay unique.
pub fn types() -> BoxedStrategy<Type> {
    prop_oneof![
        scalar_types(),
        scalar_types().prop_map(|ty| Type::Option(Box::new(ty))),
        scalar_types().prop_map(|ty| Type::Record(Box::new(ty))),
        scalar_types().prop_map(|ty| Type::Tuple(Box::new(ty))),
        (
            proptest::sample::select(vec![
                CollectionKind::Sequence,
                CollectionKind::Set,
                CollectionKind::Bag
            ]),
            scalar_types()
        )
            .prop_map(|(kind, member)| Type::Collection {
                kind,
                member: Box::new(member)
            })
    ]
    .boxed()
}

#[derive(Clone, Copy, Debug)]
enum IntegerOperator {
    Add,
    Subtract,
    Multiply,
    Less,
}

#[derive(Clone, Copy, Debug)]
enum BooleanOperator {
    And,
    Or,
    Not,
}

/// Typed identity/conditional expressions over every family, plus integer and Boolean operators.
pub fn expressions() -> BoxedStrategy<TypedExpression> {
    let constructors = (types(), 0u8..40).prop_map(|(ty, choice)| TypedExpression {
        expression: inhabit(&ty, choice),
        parameters: Vec::new(),
        result: ty,
    });
    let identity = (types(), 0u8..4).prop_map(|(ty, depth)| {
        let mut expression = Expr::Parameter("x".into());
        for index in 0..depth {
            expression = Expr::If {
                condition: Box::new(Expr::Boolean(index % 2 == 0)),
                yes: Box::new(expression),
                no: Box::new(Expr::Parameter("x".into())),
            };
        }
        TypedExpression {
            parameters: vec![("x".into(), ty.clone())],
            result: ty,
            expression,
        }
    });
    let integers = (
        -20i64..=20,
        proptest::sample::select(vec![
            IntegerOperator::Add,
            IntegerOperator::Subtract,
            IntegerOperator::Multiply,
            IntegerOperator::Less,
        ]),
    )
        .prop_map(|(literal, operation)| {
            let left = Box::new(Expr::Parameter("x".into()));
            let right = Box::new(Expr::Integer(literal));
            let (result, expression) = match operation {
                IntegerOperator::Add => (Type::Integer, Expr::Add(left, right)),
                IntegerOperator::Subtract => (Type::Integer, Expr::Subtract(left, right)),
                IntegerOperator::Multiply => (Type::Integer, Expr::Multiply(left, right)),
                IntegerOperator::Less => (Type::Boolean, Expr::Less(left, right)),
            };
            TypedExpression {
                parameters: vec![("x".into(), Type::Integer)],
                result,
                expression,
            }
        });
    let booleans = (
        proptest::bool::ANY,
        proptest::sample::select(vec![
            BooleanOperator::And,
            BooleanOperator::Or,
            BooleanOperator::Not,
        ]),
    )
        .prop_map(|(literal, operation)| {
            let left = Box::new(Expr::Parameter("x".into()));
            let right = Box::new(Expr::Boolean(literal));
            let expression = match operation {
                BooleanOperator::And => Expr::And(left, right),
                BooleanOperator::Or => Expr::Or(left, right),
                BooleanOperator::Not => Expr::Not(left),
            };
            TypedExpression {
                parameters: vec![("x".into(), Type::Boolean)],
                result: Type::Boolean,
                expression,
            }
        });
    let aggregates = (
        proptest::sample::select(vec![
            CollectionKind::Sequence,
            CollectionKind::Set,
            CollectionKind::Bag,
        ]),
        scalar_types(),
    )
        .prop_map(|(kind, member)| TypedExpression {
            parameters: vec![(
                "xs".into(),
                Type::Collection {
                    kind,
                    member: Box::new(member),
                },
            )],
            result: Type::Boolean,
            expression: Expr::ForAll {
                binder: "item".into(),
                collection: Box::new(Expr::Parameter("xs".into())),
                predicate: Box::new(Expr::Equal(
                    Box::new(Expr::Parameter("item".into())),
                    Box::new(Expr::Parameter("item".into())),
                )),
            },
        });
    prop_oneof![3 => identity, 3 => constructors, 1 => integers, 1 => booleans, 1 => aggregates]
        .boxed()
}
