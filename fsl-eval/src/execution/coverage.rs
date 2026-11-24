// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560: check a function body's checked constructs against backend coverage.

use filament_checked::node::{
    Arithmetic, Connective, IntegerDivisionOperator, NodeKind, OrderedKind, Visit,
};
use filament_eval::UntranslatableConstruct;
use filament_exact::ArithmeticOperator;
use filament_exact::{OrderingOperator, ValueType};
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::declaration::{CompositeShape, EqualityOperator};
use filament_semantic_value::location::ExpressionLocation;
use fsl_package::CheckedPackage;
use fsl_semantics::check::{CheckedGraph, CheckedNode};

use super::{BackendCoverage, CoverageOperation, CoverageOperator, CoverageValueType};

/// The first checked construct outside a coverage set and its source location.
#[derive(Clone, Debug)]
pub struct OutsideCoverage {
    /// FR-294's typed unsupported cause.
    pub construct: UntranslatableConstruct,
    /// Source location of the first uncovered construct.
    pub location: ExpressionLocation,
}

/// Find the first function parameter, form, type or operator outside coverage.
///
/// Traversal is pre-order with an explicit stack so a deep checked body does
/// not consume native call stack. Parameters precede the body as FR-294
/// requires.
#[must_use]
pub fn first_outside(
    package: &CheckedPackage,
    entry: &str,
    coverage: &BackendCoverage,
) -> Result<Option<OutsideCoverage>, InternalFault> {
    let graph = package.graph();
    let callable = graph.callable(entry).ok_or_else(|| {
        InternalFault::new(
            "execution-coverage",
            "selected-function-exists-in-checked-package",
        )
    })?;
    let function = graph
        .function_by_identity(callable.identity.clone())
        .ok_or_else(|| {
            InternalFault::new("execution-coverage", "callable-function-state-is-retained")
        })?;
    first_outside_body(graph, callable.parameters, function.body, coverage)
}

/// Find the first uncovered construct in a checked body when the backend
/// already holds its checked graph, callable parameter list and body node.
///
/// # Errors
/// Returns an internal fault if a checked operation has lost its semantic key.
pub fn first_outside_body(
    graph: &CheckedGraph,
    parameters: &[(String, ValueType)],
    body: CheckedNode<'_>,
    coverage: &BackendCoverage,
) -> Result<Option<OutsideCoverage>, InternalFault> {
    for (_, value_type) in parameters {
        if !coverage.lists_type(graph, value_type) {
            return Ok(Some(OutsideCoverage {
                construct: UntranslatableConstruct::ValueType(Box::new(value_type.clone())),
                location: body.location().clone(),
            }));
        }
    }
    let mut pending = vec![body];
    while let Some(node) = pending.pop() {
        let children = node.children();
        if let Some(construct) = outside_node(graph, node, &children, coverage)? {
            return Ok(Some(OutsideCoverage {
                construct,
                location: node.location().clone(),
            }));
        }
        pending.extend(children.into_iter().rev());
    }
    Ok(None)
}

/// Whether the complete function entry lies in coverage.
#[must_use]
pub fn entry_within_coverage(
    package: &CheckedPackage,
    entry: &str,
    coverage: &BackendCoverage,
) -> Result<bool, InternalFault> {
    first_outside(package, entry, coverage).map(|outside| outside.is_none())
}

/// Whether the exact untranslatable construct named by a backend is included
/// in its advertised coverage declaration.
///
/// This is intentionally narrower than checking whether the whole entry is
/// covered: another unsupported form elsewhere in the entry must not hide a
/// coverage gap for this construct.
pub fn construct_within_coverage(
    package: &CheckedPackage,
    entry: &str,
    construct: &UntranslatableConstruct,
    coverage: &BackendCoverage,
) -> Result<bool, InternalFault> {
    let graph = package.graph();
    match construct {
        UntranslatableConstruct::ExpressionForm(form) => Ok(coverage.contains_form(form)),
        UntranslatableConstruct::ValueType(value_type) => {
            Ok(coverage.lists_type(graph, value_type))
        }
        UntranslatableConstruct::Operation(key) => {
            let callable = graph.callable(entry).ok_or_else(|| {
                InternalFault::new(
                    "execution-coverage",
                    "selected-function-exists-in-checked-package",
                )
            })?;
            let function = graph
                .function_by_identity(callable.identity.clone())
                .ok_or_else(|| {
                    InternalFault::new("execution-coverage", "callable-function-state-is-retained")
                })?;
            let mut pending = vec![function.body];
            while let Some(node) = pending.pop() {
                let children = node.children();
                if graph.node_key(node) == Some(key) {
                    return Ok(operation_of(graph, node, &children)
                        .is_some_and(|operation| coverage.lists_operation(&operation)));
                }
                pending.extend(children.into_iter().rev());
            }
            Ok(false)
        }
    }
}

impl BackendCoverage {
    /// Whether the checked type is in the declared type set.
    #[must_use]
    pub fn lists_type(&self, graph: &CheckedGraph, value_type: &ValueType) -> bool {
        lists_type_inner(
            graph,
            &self.value_types,
            value_type,
            &mut std::collections::BTreeSet::new(),
        )
    }

    /// Whether this typed operation is listed.
    #[must_use]
    pub fn lists_operation(&self, operation: &CoverageOperation) -> bool {
        self.operations.contains(operation)
    }
}

fn lists_type_inner(
    graph: &CheckedGraph,
    coverage: &std::collections::BTreeSet<CoverageValueType>,
    value_type: &ValueType,
    visited: &mut std::collections::BTreeSet<filament_exact::NodeKey>,
) -> bool {
    if !coverage.contains(&coverage_type(graph, value_type)) {
        return false;
    }
    match value_type {
        ValueType::Collection(collection) => {
            lists_type_inner(graph, coverage, collection.element(), visited)
        }
        ValueType::Composite(key) => {
            if !visited.insert(key.clone()) {
                return true;
            }
            let Some(declaration) = graph.scope().types().composite(key.clone()) else {
                return false;
            };
            let members = match declaration.shape() {
                CompositeShape::Record(fields) => fields
                    .iter()
                    .map(|field| field.value_type())
                    .collect::<Vec<_>>(),
                CompositeShape::Tuple(elements) => elements.iter().collect(),
                CompositeShape::Union(_) => return false,
            };
            members
                .into_iter()
                .all(|member| lists_type_inner(graph, coverage, member, visited))
        }
        _ => true,
    }
}

fn coverage_type(graph: &CheckedGraph, value_type: &ValueType) -> CoverageValueType {
    match value_type {
        ValueType::Boolean => CoverageValueType::Boolean,
        ValueType::Integer | ValueType::Int(_) => CoverageValueType::Integer,
        ValueType::Rational(_) => CoverageValueType::Rational,
        ValueType::Decimal(_) => CoverageValueType::Decimal,
        ValueType::Text(_) => CoverageValueType::Text,
        ValueType::Composite(key) => match graph.scope().types().composite(key.clone()) {
            Some(declaration) => match declaration.shape() {
                CompositeShape::Record(_) => CoverageValueType::Record,
                CompositeShape::Tuple(_) => CoverageValueType::Tuple,
                CompositeShape::Union(_) => CoverageValueType::Union,
            },
            None => CoverageValueType::Composite,
        },
        ValueType::Collection(_) => CoverageValueType::Collection,
        ValueType::Reference(_) => CoverageValueType::Reference,
        ValueType::Timestamp => CoverageValueType::Timestamp,
        ValueType::Float(_) => CoverageValueType::Other("Float".to_owned()),
        ValueType::Quantity(_) => CoverageValueType::Other("Quantity".to_owned()),
        ValueType::CarriedQuantity(_) => CoverageValueType::Other("CarriedQuantity".to_owned()),
        ValueType::String => CoverageValueType::Other("String".to_owned()),
        ValueType::Bytes => CoverageValueType::Other("Bytes".to_owned()),
        ValueType::JsonObject => CoverageValueType::Other("JsonObject".to_owned()),
        ValueType::ClockReference => CoverageValueType::Other("ClockReference".to_owned()),
        ValueType::Uuid => CoverageValueType::Other("Uuid".to_owned()),
        ValueType::Enum(_) => CoverageValueType::Other("Enum".to_owned()),
        ValueType::Option(_) => CoverageValueType::Other("Option".to_owned()),
        ValueType::Map(_) => CoverageValueType::Other("Map".to_owned()),
        ValueType::BoundedQuantity(_) => CoverageValueType::Other("BoundedQuantity".to_owned()),
        ValueType::Population(_) => CoverageValueType::Other("Population".to_owned()),
    }
}

fn outside_node(
    graph: &CheckedGraph,
    node: CheckedNode<'_>,
    children: &[CheckedNode<'_>],
    coverage: &BackendCoverage,
) -> Result<Option<UntranslatableConstruct>, InternalFault> {
    if let Some(form) = form_name(graph, node) {
        if !coverage.contains_form(&form) {
            return Ok(Some(UntranslatableConstruct::ExpressionForm(form)));
        }
    }
    // A `NamedPopulation` is a checked query operand, not a runtime value
    // carried by the backend. Its parent `AllInstances` consumes the name
    // and the declaration's member set directly.
    if !matches!(node.kind(), NodeKind::NamedPopulation { .. })
        && !coverage.lists_type(graph, node.value_type())
    {
        return Ok(Some(UntranslatableConstruct::ValueType(Box::new(
            node.value_type().clone(),
        ))));
    }
    for child in children {
        if !matches!(child.kind(), NodeKind::NamedPopulation { .. })
            && !coverage.lists_type(graph, child.value_type())
        {
            return Ok(Some(UntranslatableConstruct::ValueType(Box::new(
                child.value_type().clone(),
            ))));
        }
    }
    let Some(operation) = operation_of(graph, node, children) else {
        return Ok(None);
    };
    if coverage.lists_operation(&operation) {
        Ok(None)
    } else {
        graph
            .node_key(node)
            .cloned()
            .map(UntranslatableConstruct::Operation)
            .map(Some)
            .ok_or_else(|| {
                InternalFault::new("execution-coverage", "checked-operation-has-node-key")
            })
    }
}

fn form_name(graph: &CheckedGraph, node: CheckedNode<'_>) -> Option<String> {
    let name = match node.kind() {
        NodeKind::Local(_) => "name",
        NodeKind::Literal(_) => "literal",
        NodeKind::Let { .. } => "let",
        NodeKind::If { .. } => "if",
        NodeKind::Coerce(..)
        | NodeKind::Arithmetic(..)
        | NodeKind::IntegerDivision { .. }
        | NodeKind::Negate(_)
        | NodeKind::Divide { .. }
        | NodeKind::Rational { .. }
        | NodeKind::RationalNegate(..)
        | NodeKind::Decimal { .. }
        | NodeKind::DecimalNegate(..)
        | NodeKind::ConvertDecimal(..)
        | NodeKind::ConvertScalar(..)
        | NodeKind::ConvertText(..)
        | NodeKind::Index { .. }
        | NodeKind::DropFirst(_)
        | NodeKind::RemoveOne(..)
        | NodeKind::Flatten(_)
        | NodeKind::ConvertCollection { .. }
        | NodeKind::Size(_)
        | NodeKind::Contains(..)
        | NodeKind::Order(..)
        | NodeKind::Equality(..)
        | NodeKind::Connective(..)
        | NodeKind::Not(_) => "operator",
        NodeKind::Call { .. } => "call",
        NodeKind::Record { .. } | NodeKind::Tuple { .. } => "record",
        NodeKind::Collection { .. } => "collection",
        NodeKind::Fold { .. } => "collection",
        NodeKind::PopulationObjects { .. } => "population",
        NodeKind::NamedPopulation { .. } => "population",
        NodeKind::Query { visit, .. } => match visit {
            Visit::Forall => "forall",
            Visit::Exists => "exists",
            _ => "collection",
        },
        NodeKind::Present(_) | NodeKind::PlacePresence { .. } | NodeKind::SlotPresence { .. } => {
            "present"
        }
        NodeKind::Attribute { .. } | NodeKind::Navigate { .. } => "deref",
        NodeKind::Field { .. } => {
            let record = node.children().first().is_some_and(|operand| {
                matches!(
                    coverage_type(graph, operand.value_type()),
                    CoverageValueType::Record
                )
            });
            if record {
                "record"
            } else {
                "field"
            }
        }
        NodeKind::Reaches { .. } | NodeKind::ReachesEnd { .. } => "reaches",
        NodeKind::Case { .. } => "case",
        other => return Some(variant_name(other)),
    };
    Some(name.to_owned())
}

/// Name a future checked form conservatively so it cannot pass a coverage
/// set that has not listed it.
fn variant_name<C: std::fmt::Debug>(kind: &NodeKind<C>) -> String {
    format!("{kind:?}")
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn operation_of(
    graph: &CheckedGraph,
    node: CheckedNode<'_>,
    children: &[CheckedNode<'_>],
) -> Option<CoverageOperation> {
    if let NodeKind::Query { visit, source, .. } = node.kind() {
        let source = node.at(*source);
        return Some(CoverageOperation {
            operator: CoverageOperator::Other(format!("query-{visit:?}").to_ascii_lowercase()),
            operand_types: vec![if matches!(source.kind(), NodeKind::NamedPopulation { .. }) {
                CoverageValueType::Collection
            } else {
                coverage_type(graph, source.value_type())
            }],
            result_type: coverage_type(graph, node.value_type()),
        });
    }
    if let NodeKind::Fold { source, .. } = node.kind() {
        let source = node.at(*source);
        return Some(CoverageOperation {
            operator: CoverageOperator::Other("fold".to_owned()),
            operand_types: vec![coverage_type(graph, source.value_type())],
            result_type: coverage_type(graph, node.value_type()),
        });
    }
    if matches!(node.kind(), NodeKind::AllInstances { .. }) {
        return Some(CoverageOperation {
            operator: CoverageOperator::Other("all-instances".to_owned()),
            operand_types: Vec::new(),
            result_type: coverage_type(graph, node.value_type()),
        });
    }
    if matches!(node.kind(), NodeKind::NamedPopulation { .. }) {
        return None;
    }
    let operator = match node.kind() {
        NodeKind::Arithmetic(Arithmetic::Add, ..) => CoverageOperator::Add,
        NodeKind::Arithmetic(Arithmetic::Subtract, ..) => CoverageOperator::Subtract,
        NodeKind::Arithmetic(Arithmetic::Multiply, ..) => CoverageOperator::Multiply,
        NodeKind::Negate(_) => CoverageOperator::Negate,
        NodeKind::IntegerDivision { operator, .. } => match operator {
            IntegerDivisionOperator::Div => CoverageOperator::Divide,
            IntegerDivisionOperator::Rem => CoverageOperator::Remainder,
            IntegerDivisionOperator::Mod => CoverageOperator::Modulo,
        },
        NodeKind::Order(operator, OrderedKind::Integers, ..) => order_operator(operator),
        NodeKind::Equality(operator, ..) => match operator {
            EqualityOperator::Equal => CoverageOperator::Equal,
            EqualityOperator::NotEqual => CoverageOperator::NotEqual,
        },
        NodeKind::Connective(connective, ..) => match connective {
            Connective::And => CoverageOperator::And,
            Connective::Or => CoverageOperator::Or,
            Connective::Implies => CoverageOperator::Implies,
        },
        NodeKind::Not(_) => CoverageOperator::Not,
        NodeKind::Divide { .. } => CoverageOperator::ExactDivide,
        NodeKind::Order(operator, ..) => order_operator(operator),
        NodeKind::Coerce(..) => CoverageOperator::Other("coerce".to_owned()),
        NodeKind::Rational { operator, .. } | NodeKind::Decimal { operator, .. } => {
            match operator {
                ArithmeticOperator::Add => CoverageOperator::Add,
                ArithmeticOperator::Subtract => CoverageOperator::Subtract,
                ArithmeticOperator::Multiply => CoverageOperator::Multiply,
                ArithmeticOperator::Divide => CoverageOperator::Divide,
            }
        }
        NodeKind::RationalNegate(..) | NodeKind::DecimalNegate(..) => CoverageOperator::Negate,
        NodeKind::ConvertDecimal(..) => CoverageOperator::Other("convert-decimal".to_owned()),
        NodeKind::Field { .. }
            if children.first().is_some_and(|child| {
                coverage_type(graph, child.value_type()) == CoverageValueType::Record
            }) =>
        {
            CoverageOperator::Other("record-field-read".to_owned())
        }
        NodeKind::Contains(..) => CoverageOperator::Other("contains".to_owned()),
        NodeKind::Index { .. } => CoverageOperator::Other("index".to_owned()),
        NodeKind::TupleProject { .. } => CoverageOperator::Other("tuple-project".to_owned()),
        NodeKind::DropFirst(_) => CoverageOperator::Other("drop-first".to_owned()),
        NodeKind::RemoveOne(..) => CoverageOperator::Other("remove-one".to_owned()),
        NodeKind::Flatten(_) => CoverageOperator::Other("flatten".to_owned()),
        NodeKind::ConvertCollection { .. } => {
            CoverageOperator::Other("convert-collection".to_owned())
        }
        NodeKind::PopulationObjects { .. } => {
            CoverageOperator::Other("population-objects".to_owned())
        }
        NodeKind::ConvertText(..) => CoverageOperator::Other("convert-text".to_owned()),
        // `ConvertScalar` is an internal equality-plan adapter. The parent
        // Equality node publishes and checks the user-visible operation.
        NodeKind::ConvertScalar(..) => return None,
        NodeKind::Local(_)
        | NodeKind::Literal(_)
        | NodeKind::Let { .. }
        | NodeKind::If { .. }
        | NodeKind::Call { .. }
        | NodeKind::ImportedCall { .. }
        | NodeKind::Dispatch { .. }
        | NodeKind::Record { .. }
        | NodeKind::Tuple { .. }
        | NodeKind::Collection { .. }
        | NodeKind::Present(_)
        | NodeKind::PlacePresence { .. }
        | NodeKind::SlotPresence { .. }
        | NodeKind::Attribute { .. }
        | NodeKind::Navigate { .. }
        | NodeKind::Field { .. }
        | NodeKind::Reaches { .. }
        | NodeKind::ReachesEnd { .. }
        | NodeKind::Case { .. } => return None,
        other => CoverageOperator::Other(variant_name(other)),
    };
    let operand_types = children
        .iter()
        .map(|child| coverage_type(graph, child.value_type()))
        .collect();
    Some(CoverageOperation {
        operator,
        operand_types,
        result_type: coverage_type(graph, node.value_type()),
    })
}

fn order_operator(operator: &OrderingOperator) -> CoverageOperator {
    match operator {
        OrderingOperator::Less => CoverageOperator::Less,
        OrderingOperator::LessOrEqual => CoverageOperator::LessOrEqual,
        OrderingOperator::Greater => CoverageOperator::Greater,
        OrderingOperator::GreaterOrEqual => CoverageOperator::GreaterOrEqual,
    }
}
