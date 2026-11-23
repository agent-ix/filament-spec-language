// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: source-independent facts of a universal hyper clause.

mod normal;
mod projection;

pub use projection::BareHyperTerm;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use filament_contract_model::{
    CheckedLiteralValue, CheckedNodeId, CheckedOccurrenceRole, CheckedSemanticNodeV2, CheckedTerm,
};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_semantics::check::{
    AtomSlot, BinaryTemporal, PackageFormulaNode, PackageHyperForm, PackageHyperParameter,
    PackageHyperQuantifier, TemporalProfile, UnaryTemporal,
};

use super::{FormulaOperation, FORMULA_VISITS};
use crate::bare_slot::BareSelectionFailure;
use crate::protocol_selection::SlotSelectionFailure;
use crate::protocol_view::{group_member, reference, text};
use crate::{BareClauseRefusal, BarePackage};

type Nodes<'p> = BTreeMap<&'p CheckedNodeId, &'p CheckedSemanticNodeV2>;

fn malformed(claim: &OccurrenceKey) -> BareClauseRefusal {
    BareSelectionFailure::from(SlotSelectionFailure::MalformedNode(claim.node().clone())).into()
}

fn pending(node: &CheckedNodeId, fact: &'static str) -> BareClauseRefusal {
    BareClauseRefusal::Pending {
        fact,
        node: node.clone(),
    }
}

fn members(term: &CheckedTerm) -> Option<&[CheckedTerm]> {
    match term {
        CheckedTerm::Aggregate { members, .. } => Some(members),
        _ => None,
    }
}

fn binding(term: &CheckedTerm) -> Option<(&str, &CheckedTerm)> {
    match term {
        CheckedTerm::Binding {
            name,
            value,
            binder: None,
        } => Some((name.as_ref(), value)),
        _ => None,
    }
}

fn integer(term: &CheckedTerm) -> Option<usize> {
    match term.literal_read()?.value {
        CheckedLiteralValue::Integer(value) => usize::try_from(value.to_u64()?).ok(),
        _ => None,
    }
}

/// Decode FR-488's escaped identifier field, preserving UTF-8 bytes.
fn field(bytes: &[u8], at: &mut usize) -> Option<String> {
    let mut decoded = Vec::new();
    while let Some(&byte) = bytes.get(*at) {
        if byte != b'_' {
            if !byte.is_ascii_alphanumeric() {
                return None;
            }
            decoded.push(byte);
            *at += 1;
        } else {
            match bytes.get(*at + 1)? {
                b'_' => {
                    decoded.push(b'_');
                    *at += 2;
                }
                b's' => break,
                b'x' => {
                    let digit = |byte: u8| match byte {
                        b'0'..=b'9' => Some(byte - b'0'),
                        b'A'..=b'F' => Some(byte - b'A' + 10),
                        _ => None,
                    };
                    decoded.push(digit(*bytes.get(*at + 2)?)? * 16 + digit(*bytes.get(*at + 3)?)?);
                    *at += 4;
                }
                _ => return None,
            }
        }
    }
    String::from_utf8(decoded)
        .ok()
        .filter(|value| !value.is_empty())
}

fn slot(name: &str) -> Option<(String, String)> {
    let bytes = name.strip_prefix("slot_s")?.as_bytes();
    let mut at = 0;
    let parameter = field(bytes, &mut at)?;
    if bytes.get(at..at.checked_add(2)?)? != b"_s" {
        return None;
    }
    at += 2;
    let trace = field(bytes, &mut at)?;
    (at == bytes.len()).then_some((parameter, trace))
}

impl BarePackage<'_> {
    /// Read the admitted universal hyper form at its exact claim occurrence.
    /// No source body is retained or reconstructed. Other hyper forms remain
    /// explicit pending facts until their package evaluators are available.
    /// Every `holds` span is the selected claim row's declaration region;
    /// expression source-map regions never contribute to the form.
    ///
    /// # Errors
    /// A missing occurrence or inconsistent slot is `MalformedNode`; a
    /// source-checked feature this form does not read is `Pending`.
    #[ix_trace_rs::string_edge]
    pub fn hyper_clause_form(
        &self,
        claim: &OccurrenceKey,
    ) -> Result<PackageHyperForm, BareClauseRefusal> {
        let package = self.package();
        let root = crate::resolve_hyper_root(package, claim).map_err(|_| malformed(claim))?;
        let node = self
            .node_by_key(claim.node())?
            .ok_or_else(|| malformed(claim))?;
        let row = package
            .claim_names()
            .unwrap_or_default()
            .iter()
            .find(|row| {
                row.declaration == node.node_id
                    && row.occurrence.role == CheckedOccurrenceRole::Claim
                    && row.occurrence.ordinal == claim.origin().ordinal()
            })
            .ok_or_else(|| malformed(claim))?;
        let nodes: Nodes<'_> = package
            .resolved_graph()
            .nodes
            .iter()
            .map(|node| (&node.node_id, node))
            .collect();
        let wrapper_key = root.body().ok_or_else(|| malformed(claim))?;
        let wrapper = self
            .node_by_key(wrapper_key)?
            .ok_or_else(|| malformed(claim))?;
        let CheckedTerm::Application {
            operation,
            arguments,
            ..
        } = &wrapper.body
        else {
            return Err(malformed(claim));
        };
        let profile = operation
            .read()
            .ok()
            .and_then(|read| {
                read.laws
                    .iter()
                    .find(|law| law.role == filament_contract_model::LawRole::TemporalProfile)
            })
            .and_then(|law| TemporalProfile::from_identity(&law.definition.identity))
            .ok_or_else(|| malformed(claim))?;
        if profile != TemporalProfile::InfiniteTrace {
            return Err(pending(&node.node_id, "hyper-profile"));
        }
        let [over, _, _, _, _, formula] = arguments.as_slice() else {
            return Err(malformed(claim));
        };
        let formula_root = reference(formula).ok_or_else(|| malformed(claim))?;
        let mut aliases = Vec::new();
        let mut subject_roots = BTreeMap::new();
        let mut subject_models = BTreeMap::new();
        let selected_models = self.model_selections();
        let selected_identities: BTreeSet<&str> = selected_models
            .iter()
            .map(|model| model.identity.as_str())
            .collect();
        // An alias may repeat a large model identity already present once in
        // the admitted graph. Keep one allocation per root, not one per alias.
        let mut model_identities: BTreeMap<&CheckedNodeId, Arc<str>> = BTreeMap::new();
        for entry in group_member(&node.body, "subjects")
            .and_then(members)
            .ok_or_else(|| malformed(claim))?
        {
            let (alias, root) = binding(entry).ok_or_else(|| malformed(claim))?;
            let root = reference(root).ok_or_else(|| malformed(claim))?;
            let identity = match model_identities.get(root) {
                Some(identity) => Arc::clone(identity),
                None => {
                    let selected = nodes.get(root).copied().ok_or_else(|| malformed(claim))?;
                    let Some(filament_contract_model::CheckedNodeOwner::ModelRoot { identity }) =
                        &selected.owner
                    else {
                        return Err(malformed(claim));
                    };
                    if !selected_identities.contains(identity.as_ref()) {
                        return Err(malformed(claim));
                    }
                    let identity: Arc<str> = Arc::from(identity.as_ref());
                    model_identities.insert(root, Arc::clone(&identity));
                    identity
                }
            };
            if subject_roots.insert(alias.to_owned(), root).is_some()
                || subject_models.insert(alias.to_owned(), identity).is_some()
            {
                return Err(malformed(claim));
            }
            aliases.push(alias.to_owned());
        }
        let quantifiers_term =
            group_member(&node.body, "quantifiers").ok_or_else(|| malformed(claim))?;
        let elements = members(quantifiers_term).ok_or_else(|| malformed(claim))?;
        let mut quantifiers = Vec::new();
        for entry in elements {
            let variable = group_member(entry, "variable")
                .and_then(text)
                .ok_or_else(|| malformed(claim))?;
            let alias = group_member(entry, "alias")
                .and_then(text)
                .ok_or_else(|| malformed(claim))?;
            if !subject_roots.contains_key(alias) {
                return Err(malformed(claim));
            }
            if group_member(entry, "quantifier").and_then(text) != Some("forall") {
                return Err(pending(&node.node_id, "hyper-quantifier-form"));
            }
            if members(entry).ok_or_else(|| malformed(claim))?.len() != 3 {
                return Err(pending(&node.node_id, "hyper-fairness"));
            }
            quantifiers.push(PackageHyperQuantifier {
                variable: variable.to_owned(),
                kind: fsl_semantics::check::QuantifierKind::Forall,
                alias: alias.to_owned(),
                fairness: Vec::new(),
            });
        }
        for (member, fact) in [("match", "hyper-match"), ("align_skip", "hyper-alignment")] {
            if !group_member(&node.body, member)
                .and_then(members)
                .ok_or_else(|| malformed(claim))?
                .is_empty()
            {
                return Err(pending(&node.node_id, fact));
            }
        }
        let parameter_terms = group_member(&node.body, "parameters")
            .and_then(members)
            .ok_or_else(|| malformed(claim))?;
        let mut declared_parameters = Vec::new();
        for entry in parameter_terms {
            let (name, model) = binding(entry).ok_or_else(|| malformed(claim))?;
            let model = reference(model).ok_or_else(|| malformed(claim))?;
            declared_parameters.push((name.to_owned(), self.protocol_role_model(model)?));
        }
        let uses = super::subject::formula_graph_of(package, formula_root)
            .map_err(|_| malformed(claim))?;
        let mut formula = Vec::with_capacity(uses.len());
        let mut atoms = Vec::new();
        let graph = crate::checked_v2::wire_graph(package).map_err(|_| malformed(claim))?;
        let mut reachable = BTreeSet::new();
        let mut work = vec![reference(over).ok_or_else(|| malformed(claim))?];
        for (index, entry) in uses.iter().enumerate() {
            let kind = entry.kind().ok_or_else(|| malformed(claim))?;
            if entry.interval.is_some() {
                return Err(pending(entry.node, "hyper-interval"));
            }
            let operand = |at| {
                entry
                    .operands
                    .get(at)
                    .copied()
                    .ok_or_else(|| malformed(claim))
            };
            formula.push(match kind {
                FormulaOperation::True => PackageFormulaNode::Constant(true),
                FormulaOperation::False => PackageFormulaNode::Constant(false),
                FormulaOperation::Holds => {
                    let expression = nodes
                        .get(entry.node)
                        .and_then(|node| match &node.body {
                            CheckedTerm::Application { arguments, .. } => {
                                arguments.first().and_then(reference)
                            }
                            _ => None,
                        })
                        .ok_or_else(|| malformed(claim))?;
                    work.push(expression);
                    atoms.push((
                        index,
                        graph.key(expression).map_err(|_| malformed(claim))?,
                        0,
                    ));
                    let region = &row.declaration_region;
                    PackageFormulaNode::Holds {
                        span: fsl_foundation::Span {
                            start: usize::try_from(region.start).map_err(|_| malformed(claim))?,
                            end: usize::try_from(region.end).map_err(|_| malformed(claim))?,
                        },
                    }
                }
                FormulaOperation::Not => PackageFormulaNode::Not(operand(0)?),
                FormulaOperation::And => PackageFormulaNode::And(operand(0)?, operand(1)?),
                FormulaOperation::Or => PackageFormulaNode::Or(operand(0)?, operand(1)?),
                FormulaOperation::Implies => PackageFormulaNode::Implies(operand(0)?, operand(1)?),
                FormulaOperation::Always
                | FormulaOperation::Eventually
                | FormulaOperation::Once
                | FormulaOperation::Historically => PackageFormulaNode::Unary {
                    operator: match kind {
                        FormulaOperation::Always => UnaryTemporal::Always,
                        FormulaOperation::Eventually => UnaryTemporal::Eventually,
                        FormulaOperation::Once => UnaryTemporal::Once,
                        FormulaOperation::Historically => UnaryTemporal::Historically,
                        _ => return Err(malformed(claim)),
                    },
                    interval: None,
                    operand: operand(0)?,
                },
                FormulaOperation::Until
                | FormulaOperation::Release
                | FormulaOperation::Since
                | FormulaOperation::Triggered => PackageFormulaNode::Binary {
                    operator: match kind {
                        FormulaOperation::Until => BinaryTemporal::Until,
                        FormulaOperation::Release => BinaryTemporal::Release,
                        FormulaOperation::Since => BinaryTemporal::Since,
                        FormulaOperation::Triggered => BinaryTemporal::Triggered,
                        _ => return Err(malformed(claim)),
                    },
                    interval: None,
                    left: operand(0)?,
                    right: operand(1)?,
                },
                FormulaOperation::Aggregate => return Err(pending(entry.node, "hyper-aggregate")),
            });
        }
        let mut slots = BTreeMap::new();
        let mut slots_by_node = BTreeMap::new();
        while let Some(id) = work.pop() {
            if !reachable.insert(id) {
                continue;
            }
            if u64::try_from(reachable.len()).map_or(true, |visits| visits > FORMULA_VISITS) {
                return Err(pending(&node.node_id, "hyper-formula-work"));
            }
            let found = nodes.get(id).ok_or_else(|| malformed(claim))?;
            if found.semantic_form.as_ref() == "parameter" {
                let name = group_member(&found.body, "name")
                    .and_then(text)
                    .ok_or_else(|| malformed(claim))?;
                if name.starts_with("slot_") {
                    let (parameter, trace) = slot(name).ok_or_else(|| malformed(claim))?;
                    let parameter = declared_parameters
                        .iter()
                        .position(|(name, _)| *name == parameter)
                        .ok_or_else(|| malformed(claim))?;
                    let trace = quantifiers
                        .iter()
                        .position(|q| q.variable == trace)
                        .ok_or_else(|| malformed(claim))?;
                    let level = group_member(&found.body, "level")
                        .and_then(integer)
                        .ok_or_else(|| malformed(claim))?;
                    if slots.insert(level, AtomSlot { parameter, trace }).is_some() {
                        return Err(malformed(claim));
                    }
                    slots_by_node.insert((*id).clone(), (parameter, trace));
                    continue;
                }
            }
            work.extend(found.body.reference_targets());
        }
        let atom_slots: Vec<_> = slots
            .into_iter()
            .enumerate()
            .map(|(expected, (level, slot))| {
                if expected == level {
                    Ok(slot)
                } else {
                    Err(malformed(claim))
                }
            })
            .collect::<Result<_, _>>()?;
        for (_, _, count) in &mut atoms {
            *count = atom_slots.len();
        }
        let catalog = package
            .selected_model_population_catalog(
                filament_contract_model::CheckedModelPopulationCatalogLimits {
                    work: FORMULA_VISITS,
                    entries: FORMULA_VISITS,
                },
            )
            .map_err(|_| malformed(claim))?;
        let mut parameters = Vec::new();
        for (index, (name, object)) in declared_parameters.into_iter().enumerate() {
            let used_aliases: BTreeSet<_> = atom_slots
                .iter()
                .filter(|slot| slot.parameter == index)
                .map(|slot| quantifiers[slot.trace].alias.as_str())
                .collect();
            if used_aliases.len() > 1 {
                return Err(malformed(claim));
            }
            let mut candidates = Vec::new();
            for alias in &aliases {
                if used_aliases
                    .first()
                    .is_some_and(|used| *used != alias.as_str())
                {
                    continue;
                }
                for root in catalog.roots() {
                    if subject_roots.get(alias).copied() != Some(root.root()) {
                        continue;
                    }
                    for (ordinal, population) in root.populations().iter().enumerate() {
                        if population.members().len() == 1
                            && population.members()[0].normalized_type_identity() == &object
                        {
                            candidates.push((alias.clone(), ordinal));
                        }
                    }
                }
            }
            let Some((alias, ordinal)) = candidates.first() else {
                return Err(pending(&node.node_id, "hyper-population-cover"));
            };
            if candidates.len() != 1 {
                return Err(malformed(claim));
            }
            parameters.push(PackageHyperParameter {
                name,
                alias: alias.clone(),
                object: object.clone(),
                population_object: object,
                population_ordinal: *ordinal,
                maximum: None,
            });
        }
        let (normal_body, copy_group) = normal::copy_group(
            normal::Inputs {
                nodes: &nodes,
                graph: &graph,
                root: &node.node_id,
            },
            &formula,
            &atoms,
            &slots_by_node,
            &quantifiers,
            claim,
        )?;
        Ok(PackageHyperForm {
            name: row.name.as_ref().to_owned(),
            span: fsl_foundation::Span {
                start: usize::try_from(row.declaration_region.start)
                    .map_err(|_| malformed(claim))?,
                end: usize::try_from(row.declaration_region.end).map_err(|_| malformed(claim))?,
            },
            occurrence: claim.clone(),
            profile,
            aliases,
            subject_models,
            parameters,
            quantifiers,
            atom_slots,
            formula,
            atoms,
            copy_group,
            normal_body,
        })
    }
}
