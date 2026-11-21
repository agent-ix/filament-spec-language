// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230/FR-995: original declaration provenance from the admitted bare
//! graph. No source-side package or new witness participates in this view.

use std::collections::BTreeMap;

use filament_contract_model::CheckedNodeOwner;
use filament_exact::{Identifier, NodeKey};
use filament_semantic_value::catalog::InternalFault;
use fsl_semantics::check::SourceOwner;
use fsl_semantics::model::clock_reference::identity::{DeclarationScope, PlaceTemplate};
use fsl_semantics::model::clock_reference::normalization::{
    SourceDeclarationFact, SourceDeclarations,
};

use crate::{BarePackage, BareSelectionFailure, CheckedClockPlace, ClockPathStep};

/// Original source and model declaration scopes retained by admitted nodes.
#[derive(Clone, Debug)]
pub struct ClockDeclarations {
    source: SourceDeclarations,
    models: BTreeMap<NodeKey, DeclarationScope>,
}

impl ClockDeclarations {
    /// The same declaration normalizer the source route and emitter use.
    pub fn source(&self) -> &SourceDeclarations {
        &self.source
    }

    /// Read the actual admitted node's original scope. An absent declaration
    /// is not replaced by a short-name or effective-receiver lookup.
    pub fn scope(&self, node: &NodeKey) -> Option<&DeclarationScope> {
        self.source
            .node_scope(node)
            .or_else(|| self.models.get(node))
    }

    /// Normalize an admitted clock place through the same typed path
    /// normalizer as the checked-source route.
    pub fn clock_template(
        &self,
        clock: &CheckedClockPlace,
    ) -> Result<PlaceTemplate, BareSelectionFailure> {
        use fsl_semantics::check::{CollectionShape, PlaceRow, PlaceStep};

        let steps = clock
            .path
            .iter()
            .map(|step| {
                Ok(match step {
                    ClockPathStep::Field(name) => PlaceStep::Field(name.clone()),
                    ClockPathStep::Tuple(position) => PlaceStep::Field(position.to_string()),
                    ClockPathStep::Union(case) => PlaceStep::Union(case.clone()),
                    ClockPathStep::Slot {
                        collection,
                        minimum,
                        maximum,
                        ..
                    } => {
                        let (ordered, unique, map) = match collection.as_str() {
                            "sequence" => (true, false, false),
                            "set" => (false, true, false),
                            "bag" => (false, false, false),
                            "ordered_set" => (true, true, false),
                            "map" => (false, true, true),
                            _ => return Err(fault("admitted-clock-slot-has-checked-kind")),
                        };
                        PlaceStep::Slot(CollectionShape {
                            ordered,
                            unique,
                            min: *minimum,
                            max: *maximum,
                            map,
                        })
                    }
                })
            })
            .collect::<Result<Vec<_>, BareSelectionFailure>>()?;
        let rows = clock
            .resolved_path
            .iter()
            .map(|row| PlaceRow {
                declaring_type: row.declaring_type.clone(),
                member_identity: row.member_identity.clone(),
                member_type: row.member_type.clone(),
                collection_kind: row.collection_kind.clone(),
                minimum: row.minimum.clone(),
                maximum: row.maximum.clone(),
            })
            .collect::<Vec<_>>();
        let first = rows
            .first()
            .ok_or_else(|| fault("admitted-clock-has-resolution-rows"))?;
        let scope = self
            .row_scope(&first.declaring_type)
            .map_err(|_| fault("admitted-clock-scope-is-unambiguous"))?
            .ok_or_else(|| fault("admitted-clock-has-original-declaring-scope"))?;
        fsl_semantics::model::clock_reference::path::normalize(
            scope,
            &steps,
            &rows,
            |text| self.source.text(text),
            |declaring, member| self.source.member(declaring, member),
            |text| self.row_scope(text),
        )
        .map_err(|_| fault("admitted-clock-path-normalizes"))
    }

    fn row_scope(&self, text: &str) -> Result<Option<DeclarationScope>, InternalFault> {
        let mut found = self.source.row_scope(text)?;
        for scope in self.models.values() {
            let DeclarationScope::Model { declaration, .. } = scope else {
                continue;
            };
            if declaration == text {
                if found.as_ref().is_some_and(|previous| previous != scope) {
                    return Err(InternalFault::new(
                        "clock-declaration-projection",
                        "clock-type-rendering-has-one-original-scope",
                    ));
                }
                found = Some(scope.clone());
            }
        }
        Ok(found)
    }
}

impl BarePackage<'_> {
    /// Read original declaration facts from this package's already admitted
    /// node metadata. A retained SourceOwner wins on the bare route too.
    ///
    /// # Errors
    /// An internal fault if admitted owner or declaration metadata no longer
    /// agrees with the package's checked structural node correspondence.
    pub fn clock_declarations(&self) -> Result<ClockDeclarations, BareSelectionFailure> {
        let mut source = Vec::new();
        let mut models = BTreeMap::new();
        for node in &self.package().resolved_graph().nodes {
            let Some(owner) = &node.owner else {
                continue;
            };
            let key = self
                .node_key(&node.node_id)
                .ok_or_else(|| fault("clock-declaration-has-admitted-node-key"))?;
            match owner {
                CheckedNodeOwner::Source {
                    authority,
                    identity,
                } => {
                    let Some(declaration) = &node.declaration else {
                        continue;
                    };
                    let name = declaration
                        .qualified_name
                        .iter()
                        .map(|part| Identifier::new(part.to_string()))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| fault("admitted-source-name-is-qualified-identifiers"))?;
                    let owner = SourceOwner::new(authority.as_ref(), identity.as_ref())
                        .map_err(|_| fault("admitted-source-owner-is-valid"))?;
                    source.push(SourceDeclarationFact {
                        owner,
                        node: key,
                        name,
                        original: None,
                    });
                }
                CheckedNodeOwner::Model { identity, node } => {
                    models.insert(
                        key,
                        DeclarationScope::Model {
                            identity: identity.to_string(),
                            declaration: node.to_string(),
                        },
                    );
                }
                CheckedNodeOwner::ModelRoot { .. } => {}
            }
        }
        let mut source = SourceDeclarations::from_facts(source)
            .map_err(|_| fault("admitted-source-declaration-provenance-is-consistent"))?;
        let aliases = match self.witness() {
            Ok((_, identities)) => identities
                .source_record_rows()
                .map(|row| (&row.structural_node, row.type_handle.clone()))
                .chain(
                    identities
                        .source_union_rows()
                        .map(|row| (&row.structural_node, row.type_handle.clone())),
                )
                .chain(
                    identities
                        .source_tuple_rows()
                        .map(|row| (&row.structural_node, row.type_handle.clone())),
                )
                .collect::<Vec<_>>(),
            Err(_) => Vec::new(),
        };
        for (node, handle) in aliases {
            let key = self
                .node_key(node)
                .ok_or_else(|| fault("source-type-row-has-admitted-node"))?;
            source
                .retain_node_alias(&key, handle)
                .map_err(|_| fault("source-type-handle-joins-its-original-declaration"))?;
        }
        Ok(ClockDeclarations { source, models })
    }
}

fn fault(invariant: &'static str) -> BareSelectionFailure {
    BareSelectionFailure::from(crate::protocol_selection::SlotSelectionFailure::Fault(
        InternalFault::new("clock-declaration-projection", invariant),
    ))
}
