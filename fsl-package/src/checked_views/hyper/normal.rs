// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940's package normal form, without source expressions or recursive walks.

use super::{FORMULA_VISITS, Nodes, malformed, pending};
use crate::BareClauseRefusal;
use filament_contract_model::{CheckedNodeId, CheckedTerm};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_semantics::check::{CopyGenerator, CopyGroup, PackageFormulaNode, PackageHyperQuantifier};
use ix_trace_rs::implements;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) struct Inputs<'a, 'p> {
    pub(super) nodes: &'a Nodes<'p>,
    pub(super) graph: &'a fsl_semantics::check::wire_id::WireGraph,
    pub(super) root: &'a CheckedNodeId,
}

/// Cumulative admission: bytes are never refunded, so repeated encodings and
/// growing unary suffixes consume the same finite allowance as expression work.
struct Budget<'a> {
    node: &'a CheckedNodeId,
    work: u64,
    storage: u64,
}

impl Budget<'_> {
    fn json(&mut self, value: &impl serde::Serialize) -> Result<Vec<u8>, BareClauseRefusal> {
        struct Count {
            bytes: usize,
            limit: u64,
        }
        impl std::io::Write for Count {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                let total = self
                    .bytes
                    .checked_add(bytes.len())
                    .ok_or_else(|| std::io::Error::other("normal form bound"))?;
                if u64::try_from(total).map_or(true, |total| total > self.limit) {
                    return Err(std::io::Error::other("normal form bound"));
                }
                self.bytes = total;
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut count = Count {
            bytes: 0,
            limit: self.work.min(self.storage),
        };
        serde_json::to_writer(&mut count, value)
            .map_err(|_| pending(self.node, "hyper-normal-form-work"))?;
        self.charge(count.bytes.saturating_mul(2), count.bytes)?;
        serde_json::to_vec(value).map_err(|_| pending(self.node, "hyper-normal-form-work"))
    }
    fn copy(&mut self, bytes: &[u8]) -> Result<Vec<u8>, BareClauseRefusal> {
        self.charge(bytes.len(), bytes.len())?;
        Ok(bytes.to_vec())
    }
    fn operands(&mut self, operands: &[u64]) -> Result<Vec<u8>, BareClauseRefusal> {
        let size = operands
            .len()
            .checked_mul(8)
            .ok_or_else(|| pending(self.node, "hyper-normal-form-work"))?;
        self.charge(size, size)?;
        Ok(operands
            .iter()
            .flat_map(|operand| operand.to_be_bytes())
            .collect())
    }
    fn charge(&mut self, work: usize, storage: usize) -> Result<(), BareClauseRefusal> {
        let work = u64::try_from(work).ok();
        let storage = u64::try_from(storage).ok();
        let admitted = work
            .and_then(|work| self.work.checked_sub(work))
            .zip(storage.and_then(|storage| self.storage.checked_sub(storage)));
        let Some((work, storage)) = admitted else {
            return Err(pending(self.node, "hyper-normal-form-work"));
        };
        self.work = work;
        self.storage = storage;
        Ok(())
    }
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
        budget: &mut Budget<'_>,
    ) -> Result<Vec<u8>, BareClauseRefusal> {
        let key = self.graph.key(node).map_err(|_| malformed(claim))?;
        struct Discard;
        impl quire_canonical::Sink for Discard {
            fn write_bytes(&mut self, _: &[u8]) -> Result<(), quire_canonical::Error> {
                Ok(())
            }
        }
        let limits = quire_canonical::Limits::new(budget.work.min(budget.storage));
        let size = quire_canonical::encode(&mut Discard, &NodeBytes(&key), limits)
            .map_err(|_| pending(budget.node, "hyper-normal-form-work"))?;
        let size =
            usize::try_from(size).map_err(|_| pending(budget.node, "hyper-normal-form-work"))?;
        budget.charge(size.saturating_mul(2), size)?;
        quire_canonical::to_vec(&NodeBytes(&key), limits).map_err(|_| malformed(claim))
    }
}

#[derive(Clone)]
struct Encoded {
    bytes: Vec<u8>,
    associative: Option<(Vec<u8>, Vec<Vec<u8>>)>,
    traces: BTreeSet<usize>,
    opaque: bool,
}

fn encode(
    tag: &[u8],
    parts: &[Vec<u8>],
    budget: &mut Budget<'_>,
) -> Result<Vec<u8>, BareClauseRefusal> {
    let size = std::iter::once(tag)
        .chain(parts.iter().map(Vec::as_slice))
        .try_fold(0usize, |total, part| {
            total.checked_add(8)?.checked_add(part.len())
        })
        .ok_or_else(|| pending(budget.node, "hyper-normal-form-work"))?;
    budget.charge(size, size)?;
    let mut bytes = Vec::with_capacity(size);
    for part in std::iter::once(tag).chain(parts.iter().map(Vec::as_slice)) {
        bytes.extend_from_slice(&u64::try_from(part.len()).unwrap_or(u64::MAX).to_be_bytes());
        bytes.extend_from_slice(part);
    }
    Ok(bytes)
}

fn combine(
    tag: Vec<u8>,
    children: Vec<Encoded>,
    associative: bool,
    symmetric: bool,
    budget: &mut Budget<'_>,
) -> Result<Encoded, BareClauseRefusal> {
    let count = children
        .iter()
        .try_fold(0usize, |count, child| {
            count.checked_add(child.traces.len())?.checked_add(
                child
                    .associative
                    .as_ref()
                    .map_or(1, |(_, parts)| parts.len()),
            )
        })
        .ok_or_else(|| pending(budget.node, "hyper-normal-form-work"))?;
    budget.charge(count, count.saturating_mul(std::mem::size_of::<Encoded>()))?;
    let opaque = children.iter().any(|child| child.opaque);
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
        let bytes = parts
            .iter()
            .try_fold(0usize, |sum, part| sum.checked_add(part.len()))
            .ok_or_else(|| pending(budget.node, "hyper-normal-form-work"))?;
        // Every pair can compare the entire shorter value. Reserve that
        // conservative bound before the infallible library sort starts.
        budget.charge(bytes.saturating_mul(parts.len()), 0)?;
        parts.sort();
    }
    Ok(Encoded {
        bytes: encode(&tag, &parts, budget)?,
        associative: associative.then_some((tag, parts)),
        traces,
        opaque,
    })
}

enum Work<'a> {
    Node(&'a CheckedNodeId),
    Term(&'a CheckedTerm),
    BindLet {
        binder: &'a CheckedNodeId,
        body: &'a CheckedTerm,
    },
    RestoreLocal {
        binder: &'a CheckedNodeId,
        previous: Option<BTreeSet<usize>>,
    },
    Finish {
        tag: Vec<u8>,
        count: usize,
        associative: bool,
        symmetric: bool,
        model_read: bool,
        opaque: bool,
    },
}

#[implements("FR-176-AC-7")]
fn expression(
    inputs: &Inputs<'_, '_>,
    root: &CheckedNodeId,
    slots: &BTreeMap<CheckedNodeId, (usize, usize)>,
    swap: Option<(usize, usize)>,
    budget: &mut Budget<'_>,
    claim: &OccurrenceKey,
) -> Result<Encoded, BareClauseRefusal> {
    budget.charge(1, std::mem::size_of::<Work<'_>>())?;
    let mut pending_work = vec![Work::Node(root)];
    let mut values: Vec<Encoded> = Vec::new();
    let mut locals: BTreeMap<&CheckedNodeId, BTreeSet<usize>> = BTreeMap::new();
    while let Some(work) = pending_work.pop() {
        budget.charge(
            1,
            std::mem::size_of::<Work<'_>>().saturating_add(std::mem::size_of::<Encoded>()),
        )?;
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
                            budget,
                        )?,
                        associative: None,
                        traces: BTreeSet::from([trace]),
                        opaque: false,
                    });
                    continue;
                }
                let node = inputs.nodes.get(id).ok_or_else(|| malformed(claim))?;
                if node.node_tag.as_ref() == "expression"
                    || matches!(node.body, CheckedTerm::Literal { .. })
                {
                    pending_work.push(Work::Term(&node.body));
                } else if node.semantic_form.as_ref() == "parameter" {
                    let level = super::group_member(&node.body, "level")
                        .and_then(super::integer)
                        .ok_or_else(|| malformed(claim))?;
                    let traces = locals
                        .get(id)
                        .ok_or_else(|| pending(id, "hyper-local-binder"))?;
                    budget.charge(
                        traces.len(),
                        traces.len().saturating_mul(std::mem::size_of::<Encoded>()),
                    )?;
                    let level = u64::try_from(level)
                        .map_err(|_| malformed(claim))?
                        .to_be_bytes();
                    values.push(Encoded {
                        bytes: encode(b"local", &[budget.copy(&level)?], budget)?,
                        associative: None,
                        traces: traces.clone(),
                        opaque: false,
                    });
                } else {
                    values.push(Encoded {
                        bytes: encode(b"declaration", &[inputs.key(id, claim, budget)?], budget)?,
                        associative: None,
                        traces: BTreeSet::new(),
                        opaque: false,
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
                        &[inputs.key(type_id, claim, budget)?, budget.json(value)?],
                        budget,
                    )?;
                    values.push(Encoded {
                        bytes,
                        associative: None,
                        traces: BTreeSet::new(),
                        opaque: false,
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
                    if (identity.starts_with("filament.op.control.")
                        && identity != "filament.op.control.let")
                        || identity.starts_with("filament.op.collection.")
                    {
                        return Err(pending(root, "hyper-normal-form-operation"));
                    }
                    // FR-940's opaque table: `lookup` is opaque, and its mode (the
                    // FR-084 absence result) and type argument are part of its tag.
                    let lookup = identity == "filament.op.model.lookup";
                    if (read.mode.is_some() && !lookup)
                        || !read.laws.is_empty()
                        || !read.leaves.is_empty()
                    {
                        return Err(pending(root, "hyper-normal-form-metadata"));
                    }
                    let mode = match &read.mode {
                        Some(mode) => budget.copy(format!("{mode:?}").as_bytes())?,
                        None => Vec::new(),
                    };
                    let member = match read.member {
                        None => Vec::new(),
                        Some(filament_contract_model::CheckedOperationMember::Field {
                            declaration,
                            name,
                            read_form,
                        }) => encode(
                            b"field",
                            &[
                                inputs.key(declaration, claim, budget)?,
                                budget.copy(name.as_bytes())?,
                                format!("{read_form:?}").into_bytes(),
                            ],
                            budget,
                        )?,
                        Some(filament_contract_model::CheckedOperationMember::TypeArgument {
                            declaration,
                            alias,
                        }) if lookup
                            || matches!(
                                identity,
                                "filament.op.numeric.convert" | "filament.op.numeric.narrow"
                            ) =>
                        {
                            let alias = match alias {
                                Some(alias) => encode(
                                    b"alias",
                                    &[
                                        budget.copy(alias.package.as_bytes())?,
                                        budget.copy(alias.node.as_bytes())?,
                                    ],
                                    budget,
                                )?,
                                None => Vec::new(),
                            };
                            encode(
                                b"type-argument",
                                &[inputs.key(declaration, claim, budget)?, alias],
                                budget,
                            )?
                        }
                        _ => return Err(pending(root, "hyper-normal-form-member")),
                    };
                    budget.charge(
                        read.constraints.len(),
                        read.constraints
                            .len()
                            .saturating_mul(std::mem::size_of::<Vec<u8>>()),
                    )?;
                    let tag = encode(
                        b"application",
                        &[
                            budget.copy(identity.as_bytes())?,
                            member,
                            mode,
                            inputs.key(result_type, claim, budget)?,
                            encode(
                                b"constraints",
                                &read
                                    .constraints
                                    .iter()
                                    .map(|constraint| {
                                        encode(
                                            format!("{:?}", constraint.kind).as_bytes(),
                                            &[
                                                budget.operands(&constraint.operands)?,
                                                budget.copy(
                                                    constraint
                                                        .family
                                                        .as_deref()
                                                        .unwrap_or_default()
                                                        .as_bytes(),
                                                )?,
                                            ],
                                            budget,
                                        )
                                    })
                                    .collect::<Result<Vec<_>, _>>()?,
                                budget,
                            )?,
                        ],
                        budget,
                    )?;
                    budget.charge(
                        arguments.len(),
                        arguments
                            .len()
                            .saturating_mul(std::mem::size_of::<Work<'_>>()),
                    )?;
                    if identity == "filament.op.control.let" {
                        let [
                            CheckedTerm::Binding {
                                binder: Some(binder),
                                value,
                                ..
                            },
                            body,
                        ] = arguments.as_slice()
                        else {
                            return Err(malformed(claim));
                        };
                        let declared = inputs.nodes.get(binder).ok_or_else(|| malformed(claim))?;
                        if declared.semantic_form.as_ref() != "parameter" {
                            return Err(malformed(claim));
                        }
                        let level = super::group_member(&declared.body, "level")
                            .and_then(super::integer)
                            .ok_or_else(|| malformed(claim))?;
                        let level = u64::try_from(level)
                            .map_err(|_| malformed(claim))?
                            .to_be_bytes();
                        let tag = encode(b"let", &[tag, budget.copy(&level)?], budget)?;
                        budget
                            .charge(3, 3_usize.saturating_mul(std::mem::size_of::<Work<'_>>()))?;
                        pending_work.push(Work::Finish {
                            tag,
                            count: 2,
                            associative: false,
                            symmetric: false,
                            model_read: false,
                            opaque: true,
                        });
                        pending_work.push(Work::BindLet { binder, body });
                        pending_work.push(Work::Term(value));
                        continue;
                    }
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
                        opaque: lookup,
                    });
                    pending_work.extend(arguments.iter().rev().map(Work::Term));
                }
                _ => return Err(pending(root, "hyper-normal-form-term")),
            },
            Work::BindLet { binder, body } => {
                let traces = &values.last().ok_or_else(|| malformed(claim))?.traces;
                budget.charge(
                    2 + traces.len(),
                    2_usize
                        .saturating_mul(std::mem::size_of::<Work<'_>>())
                        .saturating_add(
                            traces.len().saturating_mul(std::mem::size_of::<Encoded>()),
                        ),
                )?;
                let previous = locals.insert(binder, traces.clone());
                pending_work.push(Work::RestoreLocal { binder, previous });
                pending_work.push(Work::Term(body));
            }
            Work::RestoreLocal { binder, previous } => match previous {
                Some(previous) => {
                    locals.insert(binder, previous);
                }
                None => {
                    locals.remove(binder);
                }
            },
            Work::Finish {
                tag,
                count,
                associative,
                symmetric,
                model_read,
                opaque,
            } => {
                let from = values
                    .len()
                    .checked_sub(count)
                    .ok_or_else(|| malformed(claim))?;
                let children = values.split_off(from);
                let mut value = combine(tag, children, associative, symmetric, budget)?;
                value.opaque |= opaque && !value.traces.is_empty();
                if model_read && value.traces.len() != 1 {
                    return Err(malformed(claim));
                }
                values.push(value);
            }
        }
    }
    let [_] = values.as_slice() else {
        return Err(malformed(claim));
    };
    values.pop().ok_or_else(|| malformed(claim))
}

struct EncodedBody {
    bytes: Vec<u8>,
    atoms: Vec<Vec<u8>>,
    opaque: bool,
}

fn body(
    inputs: &Inputs<'_, '_>,
    formula: &[PackageFormulaNode],
    atoms: &[(usize, filament_exact::NodeKey, usize)],
    slots: &BTreeMap<CheckedNodeId, (usize, usize)>,
    swap: Option<(usize, usize)>,
    budget: &mut Budget<'_>,
    claim: &OccurrenceKey,
) -> Result<EncodedBody, BareClauseRefusal> {
    budget.charge(
        formula.len(),
        formula
            .len()
            .saturating_mul(std::mem::size_of::<Option<Encoded>>()),
    )?;
    budget.charge(
        atoms.len(),
        atoms.len().saturating_mul(std::mem::size_of::<Encoded>()),
    )?;
    let mut atom_bytes = Vec::new();
    let mut by_index = BTreeMap::new();
    for (at, key, _) in atoms {
        let mut found = None;
        for id in inputs.nodes.keys() {
            budget.charge(1, 0)?;
            if inputs.graph.key(id).as_ref().ok() == Some(key) {
                found = Some(*id);
                break;
            }
        }
        let id = found.ok_or_else(|| malformed(claim))?;
        let value = expression(inputs, id, slots, swap, budget, claim)?;
        atom_bytes.push(budget.copy(&value.bytes)?);
        by_index.insert(*at, value);
    }
    let mut encoded: Vec<Option<Encoded>> = vec![None; formula.len()];
    for (at, node) in formula.iter().enumerate().rev() {
        budget.charge(1, 0)?;
        let (tag, children, associative, symmetric) = match node {
            PackageFormulaNode::Constant(value) => (
                format!("constant:{value}").into_bytes(),
                Vec::new(),
                false,
                false,
            ),
            PackageFormulaNode::Holds { .. } => {
                let value = by_index.remove(&at).ok_or_else(|| malformed(claim))?;
                encoded[at] = Some(combine(
                    b"holds".to_vec(),
                    vec![value],
                    false,
                    false,
                    budget,
                )?);
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
        encoded[at] = Some(combine(tag, children, associative, symmetric, budget)?);
    }
    let body = encoded
        .first_mut()
        .and_then(Option::take)
        .ok_or_else(|| malformed(claim))?;
    Ok(EncodedBody {
        bytes: body.bytes,
        atoms: atom_bytes,
        opaque: body.opaque,
    })
}

fn exchange(
    plain: &[Vec<u8>],
    swapped: &[Vec<u8>],
    budget: &mut Budget<'_>,
) -> Result<Option<Vec<usize>>, BareClauseRefusal> {
    let bytes = plain
        .iter()
        .chain(swapped)
        .try_fold(0usize, |sum, bytes| sum.checked_add(bytes.len()))
        .ok_or_else(|| pending(budget.node, "hyper-normal-form-work"))?;
    budget.charge(
        bytes.saturating_mul(
            usize::try_from(plain.len().max(1).ilog2())
                .unwrap_or(usize::MAX)
                .saturating_add(1),
        ),
        plain.len().saturating_mul(
            std::mem::size_of::<VecDeque<usize>>() + 2 * std::mem::size_of::<usize>(),
        ),
    )?;
    let mut positions: BTreeMap<&[u8], VecDeque<usize>> = BTreeMap::new();
    for (index, bytes) in plain.iter().enumerate() {
        positions.entry(bytes).or_default().push_back(index);
    }
    let exchange = swapped
        .iter()
        .map(|bytes| positions.get_mut(bytes.as_slice())?.pop_front())
        .collect::<Option<Vec<_>>>();
    let Some(exchange) = exchange else {
        return Ok(None);
    };
    Ok(exchange
        .iter()
        .enumerate()
        .all(|(index, target)| exchange.get(*target) == Some(&index))
        .then_some(exchange))
}

pub(super) fn copy_group(
    inputs: Inputs<'_, '_>,
    formula: &[PackageFormulaNode],
    atoms: &[(usize, filament_exact::NodeKey, usize)],
    slots: &BTreeMap<CheckedNodeId, (usize, usize)>,
    quantifiers: &[PackageHyperQuantifier],
    claim: &OccurrenceKey,
) -> Result<(Vec<u8>, CopyGroup), BareClauseRefusal> {
    let mut budget = Budget {
        node: inputs.root,
        work: FORMULA_VISITS,
        storage: fsl_foundation::IdentityLimits::default().input_bytes,
    };
    copy_group_with_budget(
        inputs,
        formula,
        atoms,
        slots,
        quantifiers,
        claim,
        &mut budget,
    )
}

fn copy_group_with_budget(
    inputs: Inputs<'_, '_>,
    formula: &[PackageFormulaNode],
    atoms: &[(usize, filament_exact::NodeKey, usize)],
    slots: &BTreeMap<CheckedNodeId, (usize, usize)>,
    quantifiers: &[PackageHyperQuantifier],
    claim: &OccurrenceKey,
    budget: &mut Budget<'_>,
) -> Result<(Vec<u8>, CopyGroup), BareClauseRefusal> {
    budget.charge(
        quantifiers.len(),
        quantifiers.len().saturating_mul(
            std::mem::size_of::<CopyGenerator>() + 4 * std::mem::size_of::<usize>(),
        ),
    )?;
    let EncodedBody {
        bytes: plain,
        atoms: atom_bytes,
        opaque,
    } = body(&inputs, formula, atoms, slots, None, budget, claim)?;
    if opaque {
        return Ok((
            plain,
            CopyGroup {
                classes: Vec::new(),
                generators: Vec::new(),
            },
        ));
    }
    let mut classes: Vec<Vec<usize>> = Vec::new();
    let mut generators = Vec::new();
    for later in 0..quantifiers.len() {
        budget.charge(
            1 + quantifiers[later].variable.len(),
            quantifiers[later].variable.len(),
        )?;
        let mut joined = false;
        for class in &mut classes {
            budget.charge(1, 0)?;
            let Some(&earlier) = class.last() else {
                return Err(malformed(claim));
            };
            if quantifiers[earlier].alias != quantifiers[later].alias {
                continue;
            }
            let EncodedBody {
                bytes: swapped,
                atoms: swapped_atoms,
                ..
            } = body(
                &inputs,
                formula,
                atoms,
                slots,
                Some((earlier, later)),
                budget,
                claim,
            )?;
            budget.charge(plain.len().min(swapped.len()), 0)?;
            if swapped != plain {
                continue;
            }
            let Some(atoms) = exchange(&atom_bytes, &swapped_atoms, budget)? else {
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
#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// Atom-free formulas still pay for every repeated encoding and swap.
    #[trace("FR-940-AC-16")]
    #[test]
    fn atom_free_formulas_and_swap_trials_share_finite_work_and_storage() {
        let root: CheckedNodeId = serde_json::from_value(serde_json::json!({
            "package": "test", "path": ["claim"], "ordinal": 0
        }))
        .expect("node id");
        let graph = fsl_semantics::check::wire_id::WireGraph::new([&root]).expect("graph");
        let claim = OccurrenceKey::new(
            graph.key(&root).expect("key"),
            filament_exact::Origin::new(filament_exact::Role::new("claim"), 0),
        );
        let nodes = BTreeMap::new();
        let slots = BTreeMap::new();
        let run = |depth: usize, copies: usize, work: u64, storage: u64| {
            let mut formula: Vec<_> = (0..depth)
                .map(|at| PackageFormulaNode::Not(at + 1))
                .collect();
            formula.push(PackageFormulaNode::Constant(true));
            let quantifiers: Vec<_> = (0..copies)
                .map(|at| PackageHyperQuantifier {
                    variable: format!("t{at}"),
                    kind: fsl_forms::QuantifierKind::Forall,
                    alias: "M".to_owned(),
                    fairness: Vec::new(),
                })
                .collect();
            let mut budget = Budget {
                node: &root,
                work,
                storage,
            };
            copy_group_with_budget(
                Inputs {
                    nodes: &nodes,
                    graph: &graph,
                    root: &root,
                },
                &formula,
                &[],
                &slots,
                &quantifiers,
                &claim,
                &mut budget,
            )
        };
        let (_, group) = run(0, 2, 2_000, 1_000_000).expect("a small atom-free swap admits");
        assert_eq!(group.classes, [vec!["t0".to_owned(), "t1".to_owned()]]);
        assert_eq!(group.generators.len(), 1);
        assert!(
            run(64, 1, 1_000_000, 1_000_000).is_ok(),
            "the formula itself is valid"
        );
        for (depth, copies, work, storage) in [
            (64, 1, 2_000, 1_000_000),
            (0, 64, 2_000, 1_000_000),
            (64, 1, 1_000_000, 10_000),
        ] {
            let error = run(depth, copies, work, storage).expect_err("shared normal-form bound");
            assert!(
                matches!(error, BareClauseRefusal::Pending { fact: "hyper-normal-form-work", node } if node == root)
            );
        }
    }

    /// Encoding admission precedes allocation and leaves both counters intact
    /// if either limit cannot admit the complete output.
    #[trace("FR-940-AC-16")]
    #[test]
    fn encoding_checks_the_complete_size_before_spending_or_allocating() {
        let root: CheckedNodeId = serde_json::from_value(serde_json::json!({
            "package": "test", "path": ["claim"], "ordinal": 0
        }))
        .expect("node id");
        for (work, storage) in [(8, 100), (100, 8)] {
            let mut budget = Budget {
                node: &root,
                work,
                storage,
            };
            assert!(matches!(
                encode(b"x", &[], &mut budget),
                Err(BareClauseRefusal::Pending {
                    fact: "hyper-normal-form-work",
                    ..
                })
            ));
            assert_eq!((budget.work, budget.storage), (work, storage));
        }
        let mut budget = Budget {
            node: &root,
            work: 9,
            storage: 9,
        };
        assert_eq!(
            encode(b"x", &[], &mut budget).expect("exact bound").len(),
            9
        );
        assert_eq!((budget.work, budget.storage), (0, 0));
    }
}
