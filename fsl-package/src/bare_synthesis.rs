// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1039: value synthesis terms read from an admitted package and its
//! `claim_names` rows, without source recompilation.

use filament_contract_model::{CheckedNodeId, CheckedPackageV2, CheckedTerm};
use filament_exact::{Integer, IntegerInterval, NodeKey};
use filament_semantic_value::location::ExpressionOwner;

use crate::bare::BarePackage;
use crate::bare_slot::BareSelectionFailure;
use crate::bare_term::{fault, walk_failure};
use crate::checked_v2::wire_graph;
use crate::protocol_selection::SlotSelectionFailure;
use crate::reader_protocol_projection::{BareWalker, ReaderProtocolProjection};

/// The optional objective of an admitted value synthesis.
pub struct BareSynthesisObjective {
    direction: String,
    text: String,
    is_candidate: bool,
    expression: ReaderProtocolProjection,
}

impl BareSynthesisObjective {
    /// The checked objective direction (`minimize` or `maximize`).
    pub fn direction(&self) -> &str {
        &self.direction
    }

    /// The authored objective expression's source spelling.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the objective is exactly the candidate binder, so the
    /// candidate's own value order is the cost order.
    pub fn is_candidate(&self) -> bool {
        self.is_candidate
    }

    /// The checked objective expression, projected for the shared evaluator.
    pub fn expression(&self) -> &ReaderProtocolProjection {
        &self.expression
    }
}

/// A value synthesis declaration read from checked package terms.
pub struct BareSynthesis {
    candidate: NodeKey,
    candidate_slot: usize,
    domain: IntegerInterval,
    satisfies: ReaderProtocolProjection,
    objective: Option<BareSynthesisObjective>,
}

impl BareSynthesis {
    /// The checked candidate binder's node key.
    pub fn candidate(&self) -> &NodeKey {
        &self.candidate
    }

    /// The candidate's local frame slot in each projected expression.
    pub fn candidate_slot(&self) -> usize {
        self.candidate_slot
    }

    /// The finite integer domain admitted for the candidate.
    pub fn domain(&self) -> &IntegerInterval {
        &self.domain
    }

    /// The checked `satisfies` expression.
    pub fn satisfies(&self) -> &ReaderProtocolProjection {
        &self.satisfies
    }

    /// The checked objective, when present.
    pub fn objective(&self) -> Option<&BareSynthesisObjective> {
        self.objective.as_ref()
    }
}

impl BarePackage<'_> {
    /// Read and project a value `synthesis_request` node from this admitted
    /// package. `declaration` must be selected through this package's
    /// `claim_names` rows by the caller.
    pub fn project_value_synthesis(
        &self,
        declaration: &CheckedNodeId,
    ) -> Result<BareSynthesis, BareSelectionFailure> {
        let package = self.package();
        let graph = wire_graph(package).map_err(|_| fault("bare-package-has-structural-keys"))?;
        let nodes: std::collections::BTreeMap<_, _> = package
            .resolved_graph()
            .nodes
            .iter()
            .map(|node| (&node.node_id, node))
            .collect();
        let node = nodes
            .get(declaration)
            .copied()
            .filter(|node| {
                node.node_tag.as_ref() == "claim"
                    && node.semantic_form.as_ref() == "synthesis_request"
            })
            .ok_or_else(|| malformed(package, declaration))?;
        let domain_id = reference(
            binding(&node.body, "domain").ok_or_else(|| malformed(package, declaration))?,
        )
        .ok_or_else(|| malformed(package, declaration))?;
        let candidate_id = reference(
            binding(&node.body, "candidate").ok_or_else(|| malformed(package, declaration))?,
        )
        .ok_or_else(|| malformed(package, declaration))?;
        let satisfies_id = reference(
            binding(&node.body, "satisfies").ok_or_else(|| malformed(package, declaration))?,
        )
        .ok_or_else(|| malformed(package, declaration))?;
        let candidate = graph
            .key(candidate_id)
            .map_err(|_| fault("bare-package-has-structural-keys"))?;
        let domain = nodes
            .get(domain_id)
            .copied()
            .filter(|node| {
                node.node_tag.as_ref() == "bounded_domain"
                    && node.semantic_form.as_ref() == "integer_range"
            })
            .ok_or_else(|| malformed(package, declaration))?;
        let lower =
            integer_binding(&domain.body, "min").ok_or_else(|| malformed(package, declaration))?;
        let upper =
            integer_binding(&domain.body, "max").ok_or_else(|| malformed(package, declaration))?;
        let domain =
            IntegerInterval::new(lower, upper).map_err(|_| malformed(package, declaration))?;
        let satisfies_key = graph
            .key(satisfies_id)
            .map_err(|_| fault("bare-package-has-structural-keys"))?;
        let candidate_key = candidate.clone();
        let (_supply, identities) = self.witness()?;
        let walker = BareWalker::new(
            package,
            identities,
            self.equality_decisions(),
            self.exact_type_projections(),
            self.dependency_views(),
        )
        .map_err(SlotSelectionFailure::Fault)?;
        let owner = ExpressionOwner::Expression;
        let satisfies = walker
            .project_term(
                &satisfies_key,
                std::slice::from_ref(&candidate),
                owner.clone(),
            )
            .map_err(walk_failure)?;
        let candidate_slot = satisfies
            .local_slot(&candidate)
            .ok_or_else(|| malformed(package, declaration))?;
        let world = self.world()?;
        if satisfies.has_unrepresented_types(world.types()) {
            return Err(BareSelectionFailure::Unsupported(Box::new(
                crate::reader_protocol_projection::UnsupportedView::missing_type(
                    package.name(),
                    satisfies_key,
                ),
            )));
        }
        let objective = parse_objective(&node.body)
            .map(|(direction, text, root)| {
                let is_candidate = root == candidate_id;
                let root_key = graph
                    .key(root)
                    .map_err(|_| fault("bare-package-has-structural-keys"))?;
                let expression = walker
                    .project_term(&root_key, std::slice::from_ref(&candidate), owner)
                    .map_err(walk_failure)?;
                if expression.has_unrepresented_types(world.types()) {
                    return Err(BareSelectionFailure::Unsupported(Box::new(
                        crate::reader_protocol_projection::UnsupportedView::missing_type(
                            package.name(),
                            root_key,
                        ),
                    )));
                }
                Ok(BareSynthesisObjective {
                    direction: direction.to_owned(),
                    text: text.to_owned(),
                    is_candidate,
                    expression,
                })
            })
            .transpose()?;
        Ok(BareSynthesis {
            candidate: candidate_key,
            candidate_slot,
            domain,
            satisfies,
            objective,
        })
    }
}

/// Whether the `synthesis_request` node `declaration` names a model object
/// type as its grammar, which makes it a plan synthesis rather than a value
/// synthesis (FR-1037).
pub fn is_plan_synthesis(package: &CheckedPackageV2, declaration: &CheckedNodeId) -> bool {
    let nodes = &package.resolved_graph().nodes;
    let Some(grammar) = nodes
        .iter()
        .find(|node| &node.node_id == declaration)
        .and_then(|node| binding(&node.body, "grammar"))
        .and_then(reference)
    else {
        return false;
    };
    nodes.iter().any(|node| {
        &node.node_id == grammar
            && node.node_tag.as_ref() == "model"
            && node.semantic_form.as_ref() == "object_type"
    })
}

/// The source-plan binder is a plan record, never its object grammar.
/// Generic graph admission supplies identity and reference validity; this
/// FSL declaration rule checks the parameter's admitted type family.
pub(crate) fn plan_candidate_refusal(
    package: &CheckedPackageV2,
) -> Option<filament_contract_model::CheckedPackageRefusal> {
    use filament_contract_model::{
        CheckedNodeKind as Kind, CheckedPackageRefusalCause as Cause,
        CheckedPackageRefusalCode as Code, CheckedPackageRefusalLocation, ClaimForm,
        CompositeTypeForm, HybridForm, ModelForm, ValueForm,
    };
    let nodes = &package.resolved_graph().nodes;
    // Borrow the retained ids rather than cloning structural-key trees.
    let parameters: std::collections::BTreeMap<_, _> = nodes
        .iter()
        .filter(|node| package.node_kind(&node.node_id) == Some(Kind::Value(ValueForm::Parameter)))
        .map(|node| (&node.node_id, &node.semantic_type))
        .collect();
    for (position, node) in nodes.iter().enumerate() {
        if package.node_kind(&node.node_id) != Some(Kind::Claim(ClaimForm::SynthesisRequest)) {
            continue;
        }
        let Some(grammar) = binding(&node.body, "grammar").and_then(reference) else {
            continue;
        };
        if !matches!(
            package.node_kind(grammar),
            Some(Kind::Model(ModelForm::ObjectType) | Kind::Hybrid(HybridForm::Model))
        ) {
            continue;
        }
        let CheckedTerm::Aggregate { members, .. } = &node.body else {
            continue;
        };
        let Some((at, candidate)) =
            members
                .iter()
                .enumerate()
                .find_map(|(at, member)| match member {
                    CheckedTerm::Binding { name, value, .. } if &**name == "candidate" => {
                        Some((at, &**value))
                    }
                    _ => None,
                })
        else {
            continue;
        };
        let candidate_type = reference(candidate).and_then(|id| parameters.get(id));
        if candidate_type.is_some_and(|ty| {
            package.node_kind(ty) == Some(Kind::CompositeType(CompositeTypeForm::Record))
        }) {
            continue;
        }
        return Some(filament_contract_model::CheckedPackageRefusal {
            code: Code::InvalidPackage,
            cause: Some(Cause::InvalidValue),
            loci: vec![CheckedPackageRefusalLocation {
                path: Some(candidate_binding_path(position, at)),
                source_region: None,
            }],
            locus: Some(node.node_id.clone()),
            contract_version: None,
            document_pointer: None,
            extension_detail: None,
            protocol_detail: None,
            type_argument_detail: None,
        });
    }
    None
}

/// Locate only a strict parameter-type refusal at the uniquely joined plan
/// candidate binding. The original refusal code, cause and node are retained.
pub(crate) fn locate_plan_candidate_refusal(
    bytes: &[u8],
    refusal: &mut filament_contract_model::CheckedPackageRefusal,
) {
    use filament_contract_model::{
        CheckedPackageRefusalCause as Cause, CheckedPackageRefusalCode as Code,
    };
    if !matches!(
        refusal.code,
        Code::InvalidSemanticGraph | Code::InvalidPackage
    ) || !matches!(refusal.cause, None | Some(Cause::InvalidValue))
    {
        return;
    }
    let Some(primary) = refusal.loci.first() else {
        return;
    };
    let Some(path) = primary.path.as_ref() else {
        return;
    };
    let Some(position) = path
        .as_str()
        .strip_prefix("/semantic_graph/nodes/")
        .and_then(|path| path.strip_suffix("/semantic_type"))
        .and_then(|position| position.parse::<usize>().ok())
    else {
        return;
    };
    let Ok(mut wire) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return;
    };
    let Ok(graph) = fsl_semantics::check::wire_id::WireGraph::of_package_value(&wire) else {
        return;
    };
    let Some(locus) = refusal.locus.as_ref().and_then(|id| graph.key(id).ok()) else {
        return;
    };
    if graph.keys().get(position) != Some(&locus) {
        return;
    }
    if fsl_semantics::check::wire_id::resolve_operand_slots(&mut wire).is_err() {
        return;
    }
    let Some(nodes) = wire["semantic_graph"]["nodes"].as_array() else {
        return;
    };
    let Some(parameter) = nodes
        .get(position)
        .filter(|node| node["node_tag"] == "value" && node["semantic_form"] == "parameter")
    else {
        return;
    };
    let mut joined = None;
    for (claim_at, claim) in nodes.iter().enumerate().filter(|(_, node)| {
        node["node_tag"] == "claim" && node["semantic_form"] == "synthesis_request"
    }) {
        let Some(members) = claim["body"]["members"].as_array() else {
            continue;
        };
        let grammar = members
            .iter()
            .find(|member| member["term"] == "binding" && member["name"] == "grammar")
            .map(|member| &member["value"]);
        let Some(grammar) = grammar
            .filter(|value| value["term"] == "reference")
            .map(|value| &value["target"])
        else {
            continue;
        };
        if grammar != &parameter["semantic_type"] {
            continue;
        }
        let mut targets = nodes.iter().filter(|node| &node["node_id"] == grammar);
        let Some(target) = targets.next() else {
            continue;
        };
        if targets.next().is_some()
            || !((target["node_tag"] == "model" && target["semantic_form"] == "object_type")
                || (target["node_tag"] == "hybrid" && target["semantic_form"] == "hybrid_model"))
        {
            continue;
        }
        for (at, member) in members
            .iter()
            .enumerate()
            .filter(|(_, member)| member["term"] == "binding" && member["name"] == "candidate")
        {
            if member["value"]["term"] == "reference"
                && member["value"]["target"] == parameter["node_id"]
            {
                if joined.is_some() {
                    return;
                }
                joined = Some((claim_at, at));
            }
        }
    }
    if let Some((claim_at, at)) = joined {
        refusal.loci[0].path = Some(candidate_binding_path(claim_at, at));
    }
}

fn candidate_binding_path(position: usize, at: usize) -> filament_contract_model::JsonPointer {
    filament_contract_model::JsonPointer::root()
        .key("semantic_graph")
        .key("nodes")
        .index(position)
        .key("body")
        .key("members")
        .index(at)
        .key("value")
}

fn binding<'a>(term: &'a CheckedTerm, name: &str) -> Option<&'a CheckedTerm> {
    let CheckedTerm::Aggregate { members, .. } = term else {
        return None;
    };
    members.iter().find_map(|member| match member {
        CheckedTerm::Binding {
            name: bound, value, ..
        } if &**bound == name => Some(&**value),
        _ => None,
    })
}

fn reference(term: &CheckedTerm) -> Option<&CheckedNodeId> {
    match term {
        CheckedTerm::Reference { target } => Some(target),
        _ => None,
    }
}

fn integer_binding(term: &CheckedTerm, name: &str) -> Option<Integer> {
    binding(term, name)?
        .literal_read()
        .and_then(|literal| match literal.value {
            filament_contract_model::CheckedLiteralValue::Integer(value) => Some(value),
            _ => None,
        })
}

fn parse_objective(term: &CheckedTerm) -> Option<(&str, &str, &CheckedNodeId)> {
    let objective = binding(term, "objective")?;
    let direction = binding(objective, "direction")?
        .literal_read()
        .and_then(|literal| match literal.value {
            filament_contract_model::CheckedLiteralValue::Text(value) => Some(value),
            _ => None,
        })?;
    let text = binding(objective, "text")?
        .literal_read()
        .and_then(|literal| match literal.value {
            filament_contract_model::CheckedLiteralValue::Text(value) => Some(value),
            _ => None,
        })?;
    let root = binding(objective, "expression").and_then(reference)?;
    Some((direction, text, root))
}

fn malformed(package: &CheckedPackageV2, declaration: &CheckedNodeId) -> BareSelectionFailure {
    let key = wire_graph(package)
        .ok()
        .and_then(|graph| graph.key(declaration).ok());
    match key {
        Some(key) => SlotSelectionFailure::MalformedNode(key).into(),
        None => fault("bare-package-has-structural-keys"),
    }
}
