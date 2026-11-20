// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-931-AC-11/12: canonical atom labels from checked expressions. Source
//! check nodes are normalized to this one arena before printing, so aliases
//! and source trivia cannot affect labels.

use std::collections::{BTreeMap, HashMap, HashSet};

use filament_checked::node::{
    Arithmetic, CheckedLiteral, Connective, IntegerDivisionOperator, NodeKind,
};
use filament_exact::{CollectionKind, NodeKey, Value, ValueType};
use filament_semantic_value::declaration::EqualityOperator;
use fsl_semantics::check::{CheckedNode, NodeId};

struct ExpressionNode {
    kind: NodeKind<usize, CheckedLiteral>,
    value_type: ValueType,
}

#[derive(Default)]
pub(super) struct CanonicalRows {
    declarations: BTreeMap<NodeKey, String>,
    functions: BTreeMap<usize, String>,
    record_fields: BTreeMap<NodeKey, Vec<String>>,
}

/// A checked expression arena with children remapped to its own indices.
pub(super) struct CheckedExpression {
    nodes: Vec<ExpressionNode>,
    root: usize,
}

/// A canonical printer refusal differs from an internal rendering fault:
/// checked case arms do not retain their authored binder names.
pub(super) enum CanonicalError {
    UnsupportedCase,
    Fault(String),
}

impl From<String> for CanonicalError {
    fn from(message: String) -> Self {
        Self::Fault(message)
    }
}

impl CheckedExpression {
    /// Normalize a source condition from the checked tree, without reading
    /// the expression's source span.
    pub(super) fn from_source(root: CheckedNode<'_>) -> Result<Self, String> {
        let mut pending = vec![(root, false)];
        let mut visited = HashSet::new();
        let mut postorder = Vec::new();
        while let Some((node, exiting)) = pending.pop() {
            let id = node.id();
            if exiting {
                postorder.push(node);
                continue;
            }
            if !visited.insert(id) {
                continue;
            }
            pending.push((node, true));
            let children = checked_children(node);
            pending.extend(children.into_iter().rev().map(|child| (child, false)));
        }
        let indices = postorder
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id(), index))
            .collect::<HashMap<_, _>>();
        let nodes = postorder
            .iter()
            .map(|node| {
                let kind = node.kind().map_ref(
                    |child| indices.get(child).copied().unwrap_or(usize::MAX),
                    Clone::clone,
                );
                ExpressionNode {
                    kind,
                    value_type: node.value_type().clone(),
                }
            })
            .collect();
        let root = *indices
            .get(&root.id())
            .ok_or_else(|| "checked expression root is retained".to_owned())?;
        Ok(Self { nodes, root })
    }

    /// Render one checked condition using the FR-003 expression precedence
    /// rules and declaration keys held by the checked tree.
    pub(super) fn render(
        &self,
        binders: &[String],
        rows: &CanonicalRows,
    ) -> Result<String, CanonicalError> {
        let mut rendered: Vec<Option<(String, u8)>> = vec![None; self.nodes.len()];
        for (index, node) in self.nodes.iter().enumerate() {
            let child = |child: usize| {
                rendered
                    .get(child)
                    .and_then(Option::as_ref)
                    .map(|(text, _)| text.clone())
                    .ok_or_else(|| "checked expression operands precede their node".to_owned())
            };
            let child_prec = |child: usize| {
                rendered
                    .get(child)
                    .and_then(Option::as_ref)
                    .map(|(_, precedence)| *precedence)
                    .ok_or_else(|| "checked expression operands precede their node".to_owned())
            };
            let (text, precedence) = match &node.kind {
                NodeKind::Literal(literal) => (literal_text(&literal.0, &rows.declarations)?, 10),
                NodeKind::Coerce(operand, _) => (child(*operand)?, child_prec(*operand)?),
                NodeKind::Local(slot) => (
                    binders
                        .get(*slot)
                        .cloned()
                        .ok_or_else(|| "checked local has a clause binder".to_owned())?,
                    10,
                ),
                // Checked field and attribute names are already alias-free; no selected
                // row is needed to spell them (source records and object attributes have none).
                NodeKind::Attribute {
                    reference, field, ..
                } => (format!("{}.{}", child(*reference)?, field.name), 9),
                NodeKind::Field { operand, field, .. } => {
                    (format!("{}.{}", child(*operand)?, field.name()), 9)
                }
                NodeKind::If {
                    condition,
                    then,
                    otherwise,
                } => (
                    format!(
                        "if {} then {} else {}",
                        child(*condition)?,
                        child(*then)?,
                        child(*otherwise)?
                    ),
                    1,
                ),
                NodeKind::Contains(collection, value) => (
                    format!("contains({}, {})", child(*collection)?, child(*value)?),
                    10,
                ),
                NodeKind::Collection {
                    collection_type,
                    elements,
                } => {
                    let constructor = match collection_type.kind() {
                        CollectionKind::Sequence => "sequence",
                        CollectionKind::Set => "set",
                        CollectionKind::Bag => "bag",
                        CollectionKind::OrderedSet => "ordered_set",
                    };
                    let elements = elements
                        .iter()
                        .map(|element| child(*element))
                        .collect::<Result<Vec<_>, _>>()?
                        .join(", ");
                    (format!("{constructor}[{elements}]"), 10)
                }
                NodeKind::Call {
                    function,
                    arguments,
                } => {
                    let function = rows.functions.get(function).ok_or_else(|| {
                        "checked call has a canonical declaration identity".to_owned()
                    })?;
                    let arguments = arguments
                        .iter()
                        .map(|argument| child(*argument))
                        .collect::<Result<Vec<_>, _>>()?
                        .join(", ");
                    (format!("{function}({arguments})"), 10)
                }
                NodeKind::ImportedCall {
                    callee, arguments, ..
                } => {
                    let function = rows.declarations.get(callee).ok_or_else(|| {
                        "imported call has a canonical declaration identity".to_owned()
                    })?;
                    let arguments = arguments
                        .iter()
                        .map(|argument| child(*argument))
                        .collect::<Result<Vec<_>, _>>()?
                        .join(", ");
                    (format!("{function}({arguments})"), 10)
                }
                NodeKind::Record { declaration, slots } => {
                    let identity = rows.declarations.get(declaration).ok_or_else(|| {
                        "record constructor has a canonical declaration identity".to_owned()
                    })?;
                    let fields = rows.record_fields.get(declaration).ok_or_else(|| {
                        "record constructor has checked declaration field order".to_owned()
                    })?;
                    if fields.len() != slots.len() {
                        return Err("record constructor slots match its declaration"
                            .to_owned()
                            .into());
                    }
                    let members = fields
                        .iter()
                        .zip(slots)
                        .filter_map(|(field, slot)| match slot {
                            filament_checked::node::RecordSlot::Absent => None,
                            filament_checked::node::RecordSlot::Null => {
                                Some(Ok(format!("{field}: none")))
                            }
                            filament_checked::node::RecordSlot::Present(value) => {
                                Some(child(*value).map(|value| format!("{field}: {value}")))
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()?
                        .join(", ");
                    (format!("{identity} {{ {members} }}"), 10)
                }
                NodeKind::Case { .. } => {
                    return Err(CanonicalError::UnsupportedCase);
                }
                NodeKind::Order(operator, _, left, right) => binary(
                    child(*left)?,
                    child_prec(*left)?,
                    ordering_text(*operator),
                    child(*right)?,
                    child_prec(*right)?,
                    4,
                    false,
                ),
                NodeKind::Equality(operator, _, left, right) => binary(
                    child(*left)?,
                    child_prec(*left)?,
                    equality_text(*operator),
                    child(*right)?,
                    child_prec(*right)?,
                    4,
                    false,
                ),
                NodeKind::Arithmetic(operator, left, right) => {
                    let (operator, precedence, right_associative) = match operator {
                        Arithmetic::Add => ("+", 6, false),
                        Arithmetic::Subtract => ("-", 6, false),
                        Arithmetic::Multiply => ("*", 7, false),
                    };
                    binary(
                        child(*left)?,
                        child_prec(*left)?,
                        operator,
                        child(*right)?,
                        child_prec(*right)?,
                        precedence,
                        right_associative,
                    )
                }
                NodeKind::IntegerDivision {
                    operator,
                    left,
                    right,
                    ..
                } => binary(
                    child(*left)?,
                    child_prec(*left)?,
                    match operator {
                        IntegerDivisionOperator::Div => "div",
                        IntegerDivisionOperator::Rem => "rem",
                        IntegerDivisionOperator::Mod => "mod",
                    },
                    child(*right)?,
                    child_prec(*right)?,
                    7,
                    false,
                ),
                NodeKind::Connective(operator, _, left, right) => {
                    let (operator, precedence, right_associative) = match operator {
                        Connective::And => ("and", 3, false),
                        Connective::Or => ("or", 2, false),
                        Connective::Implies => ("implies", 1, true),
                    };
                    binary(
                        child(*left)?,
                        child_prec(*left)?,
                        operator,
                        child(*right)?,
                        child_prec(*right)?,
                        precedence,
                        right_associative,
                    )
                }
                NodeKind::Not(operand) => unary("not", child(*operand)?, child_prec(*operand)?, 8),
                NodeKind::Negate(operand) => {
                    unary("- ", child(*operand)?, child_prec(*operand)?, 8)
                }
                NodeKind::Size(operand) => (format!("size({})", child(*operand)?), 10),
                NodeKind::PopulationObjects { population } => (declaration_text(population), 10),
                NodeKind::NamedPopulation { population, .. } => (declaration_text(population), 10),
                NodeKind::AllInstances { population } => {
                    let population = child(*population)?;
                    let type_argument = match &node.value_type {
                        ValueType::Collection(collection) => match collection.element() {
                            ValueType::Reference(identity) => identity
                                .owner_chain()
                                .last()
                                .map(|key| declaration_uri(key.package(), key.node())),
                            _ => None,
                        },
                        _ => None,
                    };
                    let prefix = type_argument
                        .map(|argument| format!("allInstances<{argument}>"))
                        .unwrap_or_else(|| "allInstances".to_owned());
                    (format!("{prefix}({population})"), 10)
                }
                _ => {
                    return Err("checked expression has no canonical printer for this form"
                        .to_owned()
                        .into());
                }
            };
            rendered[index] = Some((text, precedence));
        }
        rendered
            .get(self.root)
            .and_then(Option::as_ref)
            .map(|(text, _)| text.clone())
            .ok_or_else(|| "checked expression root is rendered".to_owned().into())
    }
}

/// Build labels solely from the retained checked source conditions and their
/// positional clause binders. Source slices are deliberately not involved.
pub(super) fn source_labels(
    valuation: &fsl_replay::spine::TemporalValuation,
) -> Result<BTreeMap<fsl_semantics::check::FormulaNodeId, String>, CanonicalError> {
    let clause = &valuation.clause;
    let mut binders = vec![clause.over().0.clone()];
    if let Some((trigger, _)) = clause.trigger() {
        binders.push(trigger.to_owned());
    }
    binders.extend(clause.captures().map(|(name, _)| name.to_owned()));
    let rows = CanonicalRows {
        declarations: valuation.canonical_declarations.clone(),
        functions: valuation.canonical_functions.clone(),
        record_fields: valuation.canonical_record_fields.clone(),
    };
    valuation
        .checked_conditions
        .iter()
        .map(|(atom, body)| {
            CheckedExpression::from_source(body.root())
                .map_err(CanonicalError::Fault)
                .and_then(|expression| expression.render(&binders, &rows))
                .map(|label| (*atom, format!("holds({label})")))
        })
        .collect()
}

fn checked_children(node: CheckedNode<'_>) -> Vec<CheckedNode<'_>> {
    let mut ids = Vec::<NodeId>::new();
    let _ = node.kind().map_ref(
        |child| {
            let index = ids.len();
            ids.push(*child);
            index
        },
        Clone::clone,
    );
    ids.into_iter().map(|id| node.at(id)).collect()
}

fn binary(
    left: String,
    left_precedence: u8,
    operator: &str,
    right: String,
    right_precedence: u8,
    precedence: u8,
    right_associative: bool,
) -> (String, u8) {
    let left =
        if left_precedence < precedence || (right_associative && left_precedence == precedence) {
            format!("({left})")
        } else {
            left
        };
    let right = if right_precedence < precedence
        || (!right_associative && right_precedence == precedence)
    {
        format!("({right})")
    } else {
        right
    };
    (format!("{left} {operator} {right}"), precedence)
}

fn unary(operator: &str, operand: String, precedence: u8, own_precedence: u8) -> (String, u8) {
    if precedence < own_precedence {
        (format!("{operator} ({operand})"), own_precedence)
    } else {
        (format!("{operator}{operand}"), own_precedence)
    }
}

fn ordering_text(operator: filament_exact::OrderingOperator) -> &'static str {
    use filament_exact::OrderingOperator as Operator;
    match operator {
        Operator::Less => "<",
        Operator::LessOrEqual => "<=",
        Operator::Greater => ">",
        Operator::GreaterOrEqual => ">=",
    }
}

fn equality_text(operator: EqualityOperator) -> &'static str {
    match operator {
        EqualityOperator::Equal => "=",
        EqualityOperator::NotEqual => "!=",
    }
}

fn declaration_text(declaration: &filament_checked::key::DeclarationKey) -> String {
    declaration_uri(&declaration.package, &declaration.node)
}

fn declaration_uri(package: &str, node: &str) -> String {
    format!("ix://{package}/{node}")
}

fn literal_text(value: &Value, declarations: &BTreeMap<NodeKey, String>) -> Result<String, String> {
    match value {
        Value::Boolean(value) => Ok(value.to_string()),
        Value::Integer(value) => Ok(value.to_string()),
        Value::Rational(value) => Ok(format!(
            "rational({}, {})",
            value.numerator(),
            value.denominator()
        )),
        Value::Decimal(value) => Ok(format!(
            "decimal({}, {})",
            value.representation().coefficient(),
            value.representation().scale()
        )),
        Value::Text(value) => serde_json::to_string(value.retained()).map_err(|e| e.to_string()),
        Value::String(value) => serde_json::to_string(value).map_err(|e| e.to_string()),
        Value::Enum(member) => {
            let owner = member.variant().owner();
            let identity = declarations
                .get(owner)
                .cloned()
                .ok_or_else(|| "enum atom declaration key is available".to_owned())?;
            Ok(format!("{identity}::{}", member.variant().case()))
        }
        _ => Err("checked literal has no canonical expression spelling".to_owned()),
    }
}
