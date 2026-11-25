// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-400-AC-14: carry the admitted FR-081 effective declaration preimages
//! through the checked-package/v2 correspondence node.

use filament_contract_model::{
    CheckedNodeId, CheckedOccurrence, CheckedOccurrenceRole, CheckedSemanticNodeV2,
    CheckedSourceMapEntry, CheckedSourceRegion, CheckedTerm, LiteralKind,
};
use filament_exact::{EffectiveId, PathSegment};
use fsl_foundation::source::provenance::SourceRegion;
use fsl_semantics::check::wire_id::StructuralIds;
use fsl_semantics::check::CheckedGraph;
use fsl_semantics::model::key::EffectiveDeclarationPreimage;

use super::{claim_literal, node_id, push_scalar_type, source_artifact, EmitRefusal, GRAPH_V2};

/// Append one model-correspondence node per `model_root` that has an admitted
/// effective view, in `model_roots` order. Aliases that select one package
/// share its root, so they share its node. The view of the unit's own `object`
/// declarations (FR-569 "The source model") is no model selection: it has no
/// root and no node.
/// The body is a flat, ordered sequence of typed scalar terms. Its closed
/// grammar is documented by `append_preimage`; the strict reader rebuilds
/// the original model preimage types without encoding identities as strings.
pub(super) fn append(
    ids: &StructuralIds,
    graph: &CheckedGraph,
    regions: &impl Fn(&filament_semantic_value::location::ExpressionLocation) -> Option<SourceRegion>,
    nodes: &mut Vec<CheckedSemanticNodeV2>,
    source_map: &mut Vec<CheckedSourceMapEntry>,
) -> Result<(), EmitRefusal> {
    let mut selected = Vec::new();
    for ((selection, _), location) in graph.model_roots().zip(graph.model_root_locations().iter()) {
        let Some(view) = graph
            .selected_model_views()
            .values()
            .find(|view| view.model_selection() == selection)
        else {
            continue;
        };
        let region = regions(location).ok_or(EmitRefusal::SourceMapInvariant)?;
        selected.push((selection.identity.as_str(), view, source_region(&region)));
    }
    let Some((_, _, first_region)) = selected.first() else {
        return Ok(());
    };
    let text_type = node_id(
        ids,
        fsl_semantics::check::builtin_text_type_key(graph.package_name()).map_err(encoding)?,
    );
    let integer_type = node_id(
        ids,
        fsl_semantics::check::builtin_integer_type_key(graph.package_name()).map_err(encoding)?,
    );
    push_scalar_type(nodes, source_map, &text_type, "text", first_region);
    push_scalar_type(nodes, source_map, &integer_type, "integer", first_region);

    for (identity, view, region) in selected {
        let mut members = Vec::new();
        push_integer(
            &mut members,
            &integer_type,
            view.declarations().len().to_string(),
        )?;
        for entry in view.declarations() {
            append_preimage(&mut members, &entry.preimage, &text_type, &integer_type)?;
        }
        let key = fsl_semantics::check::structural_node_key(
            graph.package_name(),
            vec![
                PathSegment::Text("correspondence".to_owned()),
                PathSegment::Text("model_correspondence".to_owned()),
                PathSegment::Text(identity.to_owned()),
            ],
            0,
        )
        .map_err(encoding)?;
        let id = node_id(ids, key);
        let occurrence = CheckedOccurrence {
            role: CheckedOccurrenceRole::Generated,
            ordinal: 0,
        };
        source_map.push(CheckedSourceMapEntry {
            node_id: id.clone(),
            role: occurrence.role.clone(),
            ordinal: occurrence.ordinal,
            regions: vec![region],
        });
        nodes.push(CheckedSemanticNodeV2 {
            owner_model: None,
            check_mode: None,
            node_id: id.clone(),
            schema_version: GRAPH_V2.into(),
            node_tag: filament_contract_model::CheckedNodeTag::Correspondence
                .as_wire()
                .into(),
            semantic_form: "model_correspondence".into(),
            semantic_type: id,
            dependencies: Vec::new(),
            occurrences: vec![occurrence],
            recursion_group: None,
            declaration: None,
            owner: None,
            body: aggregate(members),
        });
    }
    Ok(())
}

fn source_region(region: &SourceRegion) -> CheckedSourceRegion {
    CheckedSourceRegion {
        source: source_artifact(region.source()),
        start: region.start(),
        end: region.end(),
    }
}

/// `preimage := optional_effective_id, declaration_key, fact_count, facts`;
/// each key is two text literals (package then node), each fact is ordinal,
/// rule, input_count, input keys, and optional parent effective ID. An
/// effective ID is owner-presence (`none`/`some`), recursively encoded owner,
/// then its original declaration key. Counts delimit every variable list.
fn append_preimage(
    members: &mut Vec<CheckedTerm>,
    preimage: &EffectiveDeclarationPreimage,
    text_type: &CheckedNodeId,
    integer_type: &CheckedNodeId,
) -> Result<(), EmitRefusal> {
    append_optional_effective_id(members, preimage.owner_effective_type.as_ref(), text_type);
    append_declaration(members, &preimage.original, text_type);
    push_integer(members, integer_type, preimage.derivation.len().to_string())?;
    for fact in &preimage.derivation {
        push_integer(members, integer_type, fact.ordinal.to_string())?;
        push_text(members, text_type, fact.rule.identity);
        push_integer(members, integer_type, fact.inputs.len().to_string())?;
        for input in &fact.inputs {
            append_declaration(members, input, text_type);
        }
        append_optional_effective_id(members, fact.parent.as_ref(), text_type);
    }
    Ok(())
}

fn append_optional_effective_id(
    members: &mut Vec<CheckedTerm>,
    identity: Option<&EffectiveId>,
    text_type: &CheckedNodeId,
) {
    let mut chain = Vec::new();
    let mut current = identity.cloned();
    while let Some(value) = current {
        current = value.owner_effective_type().clone();
        chain.push(value);
    }
    if chain.is_empty() {
        push_text(members, text_type, "none");
    } else {
        for _ in &chain {
            push_text(members, text_type, "some");
        }
        push_text(members, text_type, "none");
        for value in chain.iter().rev() {
            append_original_pair(
                members,
                value.original().package(),
                value.original().node(),
                text_type,
            );
        }
    }
}

fn append_declaration(
    members: &mut Vec<CheckedTerm>,
    declaration: &filament_checked::key::DeclarationKey,
    text_type: &CheckedNodeId,
) {
    append_original_pair(
        members,
        declaration.package.as_str(),
        declaration.node.as_str(),
        text_type,
    );
}

fn append_original_pair(
    members: &mut Vec<CheckedTerm>,
    package: &str,
    node: &str,
    text_type: &CheckedNodeId,
) {
    push_text(members, text_type, package);
    push_text(members, text_type, node);
}

fn push_text(members: &mut Vec<CheckedTerm>, type_id: &CheckedNodeId, value: &str) {
    members.push(text(type_id, value));
}

fn push_integer(
    members: &mut Vec<CheckedTerm>,
    type_id: &CheckedNodeId,
    value: String,
) -> Result<(), EmitRefusal> {
    members.push(claim_literal(type_id, LiteralKind::Integer, value)?);
    Ok(())
}

fn text(type_id: &CheckedNodeId, value: &str) -> CheckedTerm {
    // All strings in the model's structural keys are valid checked Text
    // literals; encoding failures are impossible for JSON strings.
    claim_literal(type_id, LiteralKind::Text, value.to_owned())
        .expect("structural identity text serializes as checked JSON")
}

fn aggregate(members: Vec<CheckedTerm>) -> CheckedTerm {
    CheckedTerm::Aggregate {
        members,
        temporal_profile: None,
    }
}

fn encoding(error: impl std::fmt::Display) -> EmitRefusal {
    EmitRefusal::Encoding {
        reason: error.to_string(),
    }
}
