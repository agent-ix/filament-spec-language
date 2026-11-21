// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230/FR-995: original declaration provenance from the admitted bare
//! graph. No source-side package or new witness participates in this view.

use filament_contract_model::CheckedNodeOwner;
use filament_exact::{Identifier, NodeKey};
use filament_semantic_value::catalog::InternalFault;
use fsl_semantics::check::SourceOwner;
use fsl_semantics::model::clock_reference::identity::{DeclarationScope, PlaceTemplate};
use fsl_semantics::model::clock_reference::normalization::{
    ClockDeclarationCollision, ClockDeclarationOrigin, ClockDeclarationResolutionError,
    ClockDeclarationTable, SourceDeclarationFact, SourceDeclarations,
};

use crate::{BarePackage, BareSelectionFailure, CheckedClockPlace, ClockPathStep};

/// Original source and model declaration scopes retained by admitted nodes.
#[derive(Clone, Debug)]
pub struct ClockDeclarations {
    source: SourceDeclarations,
}

impl ClockDeclarations {
    /// The same declaration normalizer the source route and emitter use.
    pub fn source(&self) -> &SourceDeclarations {
        &self.source
    }

    /// Read the actual admitted node's original scope. An absent declaration
    /// is not replaced by a short-name or effective-receiver lookup.
    pub fn scope(&self, node: &NodeKey) -> Option<&DeclarationScope> {
        self.source.declaration_scope_for_node(node)
    }

    /// Normalize an admitted clock place through the same typed path
    /// normalizer as the checked-source route.
    pub fn clock_template(
        &self,
        clock: &CheckedClockPlace,
    ) -> Result<PlaceTemplate, ClockDeclarationResolutionError> {
        let (steps, rows) = clock_rows(clock)?;
        self.source.clock_template_rows(&steps, &rows)
    }

    /// Recover the original inline member chain from the admitted typed rows.
    pub fn clock_origin(
        &self,
        clock: &CheckedClockPlace,
    ) -> Result<ClockDeclarationOrigin, ClockDeclarationResolutionError> {
        let (steps, rows) = clock_rows(clock)?;
        self.source.clock_origin_rows(&steps, &rows)
    }

    /// Build atomically from actual admitted timed-model rows. Map places
    /// retain their existing unsupported boundary and acquire no encoding.
    pub fn table(
        &self,
        models: &[crate::CheckedTimedModel],
    ) -> Result<ClockDeclarationTable, ClockDeclarationTableError> {
        let mut entries = Vec::new();
        for clock in models.iter().flat_map(|model| &model.clock_places) {
            if clock.path.iter().any(
                |step| matches!(step, ClockPathStep::Slot {collection, ..} if collection == "map"),
            ) {
                continue;
            }
            entries.push((self.clock_template(clock)?, self.clock_origin(clock)?));
        }
        Ok(ClockDeclarationTable::from_entries(entries)?
            .with_source_declarations(self.source.clone()))
    }
}

/// Existing declaration-resolution and collision outcomes, retained before a
/// bare route publishes any normalized place table.
#[derive(Debug, thiserror::Error)]
pub enum ClockDeclarationTableError {
    /// Original scope resolution refused or found a broken checked invariant.
    #[error(transparent)]
    Resolution(#[from] ClockDeclarationResolutionError),
    /// Distinct original declarations claimed one canonical place template.
    #[error(transparent)]
    Collision(#[from] ClockDeclarationCollision),
}

fn clock_rows(
    clock: &CheckedClockPlace,
) -> Result<
    (
        Vec<fsl_semantics::check::PlaceStep>,
        Vec<fsl_semantics::check::PlaceRow>,
    ),
    InternalFault,
> {
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
                        _ => return Err(projection_fault("admitted-clock-slot-has-checked-kind")),
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
        .collect::<Result<Vec<_>, InternalFault>>()?;
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
        .collect();
    Ok((steps, rows))
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
        let mut models = Vec::new();
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
                    models.push((
                        key,
                        filament_checked::key::DeclarationKey {
                            package: identity.to_string(),
                            node: node.to_string(),
                        },
                    ));
                }
                CheckedNodeOwner::ModelRoot { .. } => {}
            }
        }
        let mut source = SourceDeclarations::from_facts(source)
            .map_err(|_| fault("admitted-source-declaration-provenance-is-consistent"))?
            .with_model_declarations(models);
        let aliases = match self.witness() {
            Ok((_, identities)) => identities
                .source_record_rows()
                .map(|row| (&row.structural_node, row.record_key.clone()))
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
        Ok(ClockDeclarations { source })
    }
}

fn fault(invariant: &'static str) -> BareSelectionFailure {
    BareSelectionFailure::from(crate::protocol_selection::SlotSelectionFailure::Fault(
        InternalFault::new("clock-declaration-projection", invariant),
    ))
}

fn projection_fault(invariant: &'static str) -> InternalFault {
    InternalFault::new("clock-declaration-projection", invariant)
}
