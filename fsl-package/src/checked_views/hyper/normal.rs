// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940's package normal form, without source expressions or recursive walks.

use super::{malformed, pending, Nodes, FORMULA_VISITS};
use crate::BareClauseRefusal;
use filament_contract_model::{CheckedNodeId, CheckedTerm};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_semantics::check::{CopyGenerator, CopyGroup, PackageFormulaNode, PackageHyperQuantifier};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) struct Inputs<'a, 'p> {
    pub(super) nodes: &'a Nodes<'p>,
    pub(super) graph: &'a fsl_semantics::check::wire_id::WireGraph<'p>,
}

struct NodeBytes<'a>(&'a filament_exact::NodeKey);
impl quire_canonical::Encode for NodeBytes<'_> {
    fn encode_into<S: quire_canonical::Sink + ?Sized>(
        &self,
        writer: &mut quire_canonical::Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        fsl_foundation::node_wire::encode_node_key(self.0, writer)
    }
}

impl Inputs<'_, '_> {
    fn key(
        &self,
        node: &CheckedNodeId,
        claim: &OccurrenceKey,
    ) -> Result<Vec<u8>, BareClauseRefusal> {
        let key = self.graph.key(node).map_err(|_| malformed(claim))?;
        quire_canonical::to_vec(
            &NodeBytes(&key),
            quire_canonical::Limits::new(fsl_foundation::IdentityLimits::default().input_bytes),
        )
        .map_err(|_| malformed(claim))
    }
}

#[derive(Clone)]
struct Encoded {
    bytes: Vec<u8>,
    associative: Option<(Vec<u8>, Vec<Vec<u8>>)>,
    traces: BTreeSet<usize>,
}

fn encode(tag: &[u8], parts: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for part in std::iter::once(tag).chain(parts.iter().map(Vec::as_slice)) {
        bytes.extend_from_slice(&u64::try_from(part.len()).unwrap_or(u64::MAX).to_be_bytes());
        bytes.extend_from_slice(part);
    }
    bytes
}

fn combine(tag: Vec<u8>, children: Vec<Encoded>, associative: bool, symmetric: bool) -> Encoded {
    let traces = children
        .iter()
        .flat_map(|child| child.traces.iter().copied())
        .collect();
    let mut parts = Vec::new();
    for child in children {
        match child.associative {
            Some((own_tag, flattened)) if associative && own_tag == tag => parts.extend(flattened),
            _ => parts.push(child.bytes),
        }
    }
    if associative || symmetric {
        parts.sort();
    }
    Encoded {
        bytes: encode(&tag, &parts),
        associative: associative.then_some((tag, parts)),
        traces,
    }
}

enum Work<'a> {
    Node(&'a CheckedNodeId),
    Term(&'a CheckedTerm),
    Finish {
        tag: Vec<u8>,
        count: usize,
        associative: bool,
        symmetric: bool,
        model_read: bool,
    },
}

fn expression(
    inputs: &Inputs<'_, '_>,
    root: &CheckedNodeId,
    slots: &BTreeMap<CheckedNodeId, (usize, usize)>,
    swap: Option<(usize, usize)>,
    work_left: &mut u64,
    claim: &OccurrenceKey,
) -> Result<Encoded, BareClauseRefusal> {
    let mut pending_work = vec![Work::Node(root)];
    let mut values: Vec<Encoded> = Vec::new();
    while let Some(work) = pending_work.pop() {
        *work_left = work_left
            .checked_sub(1)
            .ok_or_else(|| pending(root, "hyper-normal-form-work"))?;
        match work {
            Work::Node(id) => {
                if let Some(&(parameter, trace)) = slots.get(id) {
                    let trace = match swap {
                        Some((first, second)) if trace == first => second,
                        Some((first, second)) if trace == second => first,
                        _ => trace,
                    };
                    values.push(Encoded {
                        bytes: encode(
                            b"slot",
                            &[
                                u64::try_from(parameter)
                                    .map_err(|_| malformed(claim))?
                                    .to_be_bytes()
                                    .to_vec(),
                                u64::try_from(trace)
                                    .map_err(|_| malformed(claim))?
                                    .to_be_bytes()
                                    .to_vec(),
                            ],
                        ),
                        associative: None,
                        traces: BTreeSet::from([trace]),
                    });
                    continue;
                }
                let node = inputs.nodes.get(id).ok_or_else(|| malformed(claim))?;
                if node.node_tag.as_ref() == "expression"
                    || matches!(node.body, CheckedTerm::Literal { .. })
                {
                    pending_work.push(Work::Term(&node.body));
                } else if node.semantic_form.as_ref() == "parameter" {
                    return Err(pending(id, "hyper-local-binder"));
                } else {
                    values.push(Encoded {
                        bytes: encode(b"declaration", &[inputs.key(id, claim)?]),
                        associative: None,
                        traces: BTreeSet::new(),
                    });
                }
            }
            Work::Term(term) => match term {
                CheckedTerm::Reference { target } => pending_work.push(Work::Node(target)),
                CheckedTerm::Literal {
                    type_id,
                    value_kind,
                    value,
                } => {
                    let bytes = encode(
                        format!("literal:{value_kind:?}").as_bytes(),
                        &[
                            inputs.key(type_id, claim)?,
                            serde_json::to_vec(value).map_err(|_| malformed(claim))?,
                        ],
                    );
                    values.push(Encoded {
                        bytes,
                        associative: None,
                        traces: BTreeSet::new(),
                    });
                }
                CheckedTerm::Application {
                    operation,
                    result_type,
                    arguments,
                    ..
                } => {
                    let read = operation.read().map_err(|_| malformed(claim))?;
                    let identity = read.identity;
                    if identity.starts_with("filament.op.control.")
                        || identity.starts_with("filament.op.collection.")
                    {
                        return Err(pending(root, "hyper-normal-form-operation"));
                    }
                    if read.mode.is_some()
                        || !read.laws.is_empty()
                        || !read.constraints.is_empty()
                        || !read.leaves.is_empty()
                    {
                        return Err(pending(root, "hyper-normal-form-metadata"));
                    }
                    let member = match read.member {
                        None => Vec::new(),
                        Some(filament_contract_model::CheckedOperationMember::Field {
                            declaration,
                            name,
                            read_form,
                        }) => encode(
                            b"field",
                            &[
                                inputs.key(declaration, claim)?,
                                name.as_bytes().to_vec(),
                                format!("{read_form:?}").into_bytes(),
                            ],
                        ),
                        _ => return Err(pending(root, "hyper-normal-form-member")),
                    };
                    let tag = encode(
                        b"application",
                        &[
                            identity.as_bytes().to_vec(),
                            member,
                            inputs.key(result_type, claim)?,
                        ],
                    );
                    pending_work.push(Work::Finish {
                        tag,
                        count: arguments.len(),
                        associative: matches!(
                            identity,
                            "filament.op.boolean.and" | "filament.op.boolean.or"
                        ),
                        symmetric: identity.ends_with(".eq")
                            || identity.ends_with(".ne")
                            || identity == "filament.op.ieee.numeric_equal",
                        model_read: identity == "filament.op.model.deref",
                    });
                    pending_work.extend(arguments.iter().rev().map(Work::Term));
                }
                _ => return Err(pending(root, "hyper-normal-form-term")),
            },
            Work::Finish {
                tag,
                count,
                associative,
                symmetric,
                model_read,
            } => {
                let from = values
                    .len()
                    .checked_sub(count)
                    .ok_or_else(|| malformed(claim))?;
                let children = values.split_off(from);
                let value = combine(tag, children, associative, symmetric);
                if model_read && value.traces.len() != 1 {
                    return Err(malformed(claim));
                }
                values.push(value);
            }
        }
    }
    let [value] = values.as_slice() else {
        return Err(malformed(claim));
    };
    Ok(value.clone())
}

fn body(
    inputs: &Inputs<'_, '_>,
    formula: &[PackageFormulaNode],
    atoms: &[(usize, filament_exact::NodeKey, usize)],
    slots: &BTreeMap<CheckedNodeId, (usize, usize)>,
    swap: Option<(usize, usize)>,
    work_left: &mut u64,
    claim: &OccurrenceKey,
) -> Result<(Vec<u8>, Vec<Vec<u8>>), BareClauseRefusal> {
    let mut atom_bytes = Vec::new();
    let mut by_index = BTreeMap::new();
    for (at, key, _) in atoms {
        let id = inputs
            .nodes
            .keys()
            .find(|id| inputs.graph.key(id).as_ref().ok() == Some(key))
            .ok_or_else(|| malformed(claim))?;
        let value = expression(inputs, id, slots, swap, work_left, claim)?;
        atom_bytes.push(value.bytes.clone());
        by_index.insert(*at, value);
    }
    let mut encoded: Vec<Option<Encoded>> = vec![None; formula.len()];
    for (at, node) in formula.iter().enumerate().rev() {
        let (tag, children, associative, symmetric) = match node {
            PackageFormulaNode::Constant(value) => (
                format!("constant:{value}").into_bytes(),
                Vec::new(),
                false,
                false,
            ),
            PackageFormulaNode::Holds { .. } => {
                let value = by_index.remove(&at).ok_or_else(|| malformed(claim))?;
                encoded[at] = Some(combine(b"holds".to_vec(), vec![value], false, false));
                continue;
            }
            PackageFormulaNode::Not(only) => (b"not".to_vec(), vec![*only], false, false),
            PackageFormulaNode::And(left, right) => {
                (b"and".to_vec(), vec![*left, *right], true, false)
            }
            PackageFormulaNode::Or(left, right) => {
                (b"or".to_vec(), vec![*left, *right], true, false)
            }
            PackageFormulaNode::Implies(left, right) => {
                (b"implies".to_vec(), vec![*left, *right], false, false)
            }
            PackageFormulaNode::Unary {
                operator,
                interval,
                operand,
            } => (
                format!("{operator:?}:{interval:?}").into_bytes(),
                vec![*operand],
                false,
                false,
            ),
            PackageFormulaNode::Binary {
                operator,
                interval,
                left,
                right,
            } => (
                format!("{operator:?}:{interval:?}").into_bytes(),
                vec![*left, *right],
                false,
                false,
            ),
        };
        let children = children
            .into_iter()
            .map(|child| {
                encoded
                    .get_mut(child)
                    .and_then(Option::take)
                    .ok_or_else(|| malformed(claim))
            })
            .collect::<Result<Vec<_>, _>>()?;
        encoded[at] = Some(combine(tag, children, associative, symmetric));
    }
    Ok((
        encoded
            .first_mut()
            .and_then(Option::take)
            .ok_or_else(|| malformed(claim))?
            .bytes,
        atom_bytes,
    ))
}

fn exchange(plain: &[Vec<u8>], swapped: &[Vec<u8>]) -> Option<Vec<usize>> {
    let mut positions: BTreeMap<&[u8], VecDeque<usize>> = BTreeMap::new();
    for (index, bytes) in plain.iter().enumerate() {
        positions.entry(bytes).or_default().push_back(index);
    }
    let exchange = swapped
        .iter()
        .map(|bytes| positions.get_mut(bytes.as_slice())?.pop_front())
        .collect::<Option<Vec<_>>>()?;
    exchange
        .iter()
        .enumerate()
        .all(|(index, target)| exchange.get(*target) == Some(&index))
        .then_some(exchange)
}

pub(super) fn copy_group(
    inputs: Inputs<'_, '_>,
    formula: &[PackageFormulaNode],
    atoms: &[(usize, filament_exact::NodeKey, usize)],
    slots: &BTreeMap<CheckedNodeId, (usize, usize)>,
    quantifiers: &[PackageHyperQuantifier],
    claim: &OccurrenceKey,
) -> Result<(Vec<u8>, CopyGroup), BareClauseRefusal> {
    let mut work_left = FORMULA_VISITS;
    let (plain, atom_bytes) = body(&inputs, formula, atoms, slots, None, &mut work_left, claim)?;
    let mut classes: Vec<Vec<usize>> = Vec::new();
    let mut generators = Vec::new();
    for later in 0..quantifiers.len() {
        let mut joined = false;
        for class in &mut classes {
            let Some(&earlier) = class.last() else {
                return Err(malformed(claim));
            };
            if quantifiers[earlier].alias != quantifiers[later].alias {
                continue;
            }
            let (swapped, swapped_atoms) = body(
                &inputs,
                formula,
                atoms,
                slots,
                Some((earlier, later)),
                &mut work_left,
                claim,
            )?;
            if swapped != plain {
                continue;
            }
            let Some(atoms) = exchange(&atom_bytes, &swapped_atoms) else {
                continue;
            };
            generators.push(CopyGenerator {
                first: earlier,
                second: later,
                atoms,
            });
            class.push(later);
            joined = true;
            break;
        }
        if !joined {
            classes.push(vec![later]);
        }
    }
    Ok((
        plain,
        CopyGroup {
            classes: classes
                .into_iter()
                .filter(|class| class.len() >= 2)
                .map(|class| {
                    class
                        .into_iter()
                        .map(|index| quantifiers[index].variable.clone())
                        .collect()
                })
                .collect(),
            generators,
        },
    ))
}
