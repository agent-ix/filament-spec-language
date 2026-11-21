// SPDX-License-Identifier: AGPL-3.0-or-later
//! Package-admitted declaration identities joined to genuine native references.

use filament_checked::key::DeclarationKey;
use filament_contract_model::{
    CheckedNodeId, CheckedNodePathSegment, CheckedPackageV2, ExactTypeProjectionError,
    NativeJoinBudget, NativeJoinChargeKind, NativeJoinDeniedCharge, NativeJoinLocus,
    NativeJoinLocusSource, SelectedExactDeclarationForm, SelectedExactDeclarationRow,
    SelectedExactIdentity,
};
use filament_exact::EffectiveId;
use fsl_semantics::model::intake::SelectedModels;
use fsl_semantics::model::normalize::ObjectUniverse;
use fsl_semantics::model::state::StateModel;

use crate::{AdmittedPackageRead, CheckedPackage};

/// A catalog could not establish a complete authoritative reference mapping.
#[derive(Debug, thiserror::Error)]
pub enum ReferenceCatalogError {
    /// Structural package admission rejected the genuine producer rows.
    #[error(transparent)]
    Projection(#[from] ExactTypeProjectionError),
    /// Caller work/storage admission stopped before the requested operation.
    #[error("reference catalog admission denied: {0:?}")]
    Denied(NativeJoinDeniedCharge),
    /// A producer identity has no object declaration in the actual native model.
    #[error("reference declaration is absent from the native model")]
    UnknownDeclaration,
    /// Authoritative rows disagree about a complete identity or declaration.
    #[error("conflicting reference declaration mapping")]
    ConflictingMapping,
    /// Structural and native declaring identities disagree.
    #[error("reference declaration belongs to another model selection")]
    ForeignModel,
    /// A platform size or fallible allocation cannot be satisfied.
    #[error("reference catalog storage cannot be represented or allocated")]
    Allocation,
}

/// One admitted object-type mapping. Fields cannot be supplied by consumers.
#[derive(Debug)]
pub struct ReferenceDeclaration<'a> {
    row: &'a SelectedExactDeclarationRow,
    effective: &'a EffectiveId,
    key: &'a DeclarationKey,
    universe: &'a ObjectUniverse,
}

impl ReferenceDeclaration<'_> {
    /// The actual structural declaration node admitted by the package.
    pub fn node(&self) -> &CheckedNodeId {
        &self.row.structural_node
    }

    /// The complete native object-type identity.
    pub fn effective(&self) -> &EffectiveId {
        self.effective
    }

    /// The native model owner's declaring key.
    pub fn key(&self) -> &DeclarationKey {
        self.key
    }

    /// The model owner's normalized object universe.
    pub fn universe(&self) -> &ObjectUniverse {
        self.universe
    }
}

/// Invocation-local reference authority, borrowing its real producer owners.
/// There is no constructor from bare rows, maps or Exact projection output.
#[derive(Debug)]
pub struct ReferenceCatalog<'a> {
    package: &'a CheckedPackageV2,
    source: Option<&'a CheckedPackage>,
    model: &'a StateModel,
    declarations: Vec<ReferenceDeclaration<'a>>,
}

fn charge(
    budget: &mut NativeJoinBudget,
    kind: NativeJoinChargeKind,
    amount: usize,
) -> Result<(), ReferenceCatalogError> {
    budget
        .accounted_charge(
            kind,
            amount as u128,
            NativeJoinLocus {
                source: NativeJoinLocusSource::AuthorityRow,
                form: "reference-catalog",
            },
        )
        .map_err(ReferenceCatalogError::Denied)
}

fn work(budget: &mut NativeJoinBudget, amount: u64) -> Result<(), ReferenceCatalogError> {
    budget
        .accounted_charge(
            NativeJoinChargeKind::Work,
            u128::from(amount),
            NativeJoinLocus {
                source: NativeJoinLocusSource::AuthorityRow,
                form: "reference-catalog",
            },
        )
        .map_err(ReferenceCatalogError::Denied)
}

fn text_equal(
    left: &str,
    right: &str,
    budget: &mut NativeJoinBudget,
) -> Result<bool, ReferenceCatalogError> {
    charge(
        budget,
        NativeJoinChargeKind::Work,
        left.len().max(right.len()),
    )?;
    Ok(left == right)
}

fn object_node_equal(
    left: &CheckedNodeId,
    right: &CheckedNodeId,
    budget: &mut NativeJoinBudget,
) -> Result<bool, ReferenceCatalogError> {
    work(budget, 1)?;
    // Genuine selected object declarations have three named path segments.
    // Refuse a different form without walking arbitrary operand subgraphs.
    if left.ordinal != right.ordinal || left.path.len() != 3 || right.path.len() != 3 {
        return Ok(false);
    }
    if !text_equal(&left.package, &right.package, budget)? {
        return Ok(false);
    }
    for (left, right) in left.path.iter().zip(&right.path) {
        work(budget, 1)?;
        let (CheckedNodePathSegment::Name(left), CheckedNodePathSegment::Name(right)) =
            (left, right)
        else {
            return Ok(false);
        };
        if !text_equal(left, right, budget)? {
            return Ok(false);
        }
    }
    Ok(true)
}

impl<'a> ReferenceCatalog<'a> {
    /// Join rows retained by the source checker to its own admitted emission.
    ///
    /// # Errors
    /// Preserves structural admission, work/storage and mapping refusals.
    pub fn from_source(
        read: &'a AdmittedPackageRead<'_>,
        model: &'a StateModel,
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, ReferenceCatalogError> {
        let graph = read.source().graph();
        graph.selected_exact_identities(read.admitted(), budget)?;
        Self::join(
            read.admitted(),
            Some(read.source()),
            model,
            graph.selected_exact_declaration_rows(),
            budget,
        )
    }

    /// Join the actual model-admission producer to a strictly admitted package.
    ///
    /// # Errors
    /// Preserves structural admission, work/storage and mapping refusals.
    pub fn from_admitted(
        package: &'a CheckedPackageV2,
        models: &'a SelectedModels,
        model: &'a StateModel,
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, ReferenceCatalogError> {
        models.exact_identities(package, budget)?;
        Self::join(
            package,
            None,
            model,
            models.exact_declaration_rows(),
            budget,
        )
    }

    fn join(
        package: &'a CheckedPackageV2,
        source: Option<&'a CheckedPackage>,
        model: &'a StateModel,
        rows: &'a [SelectedExactDeclarationRow],
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, ReferenceCatalogError> {
        let mut declarations: Vec<ReferenceDeclaration<'a>> = Vec::new();
        for row in rows {
            work(budget, 1)?;
            if row.form != SelectedExactDeclarationForm::ObjectType {
                continue;
            }
            // Only rows retained by this package's genuine structural join
            // participate. Do not turn every model-stage row into package authority.
            let mut admitted = false;
            for node in &package.resolved_graph().nodes {
                work(budget, 1)?;
                if text_equal(&node.node_tag, "model", budget)?
                    && text_equal(&node.semantic_form, "object_type", budget)?
                    && object_node_equal(&row.structural_node, &node.node_id, budget)?
                {
                    admitted = true;
                    break;
                }
            }
            if !admitted {
                continue;
            }
            let SelectedExactIdentity::Object(effective) = &row.exact_identity else {
                return Err(ReferenceCatalogError::ForeignModel);
            };
            let key = model
                .declaration_key_with_work(effective, &mut |units| work(budget, units))?
                .ok_or(ReferenceCatalogError::UnknownDeclaration)?;
            let Some((actual, universe)) =
                model.reference_components_with_work(key, &mut |units| work(budget, units))?
            else {
                return Err(ReferenceCatalogError::UnknownDeclaration);
            };
            if !StateModel::reference_identity_equal_with_work(effective, actual, &mut |units| {
                work(budget, units)
            })? {
                return Err(ReferenceCatalogError::ConflictingMapping);
            }
            // This comparison checks an already admitted row. The declaration
            // recipe alone is never an admission path.
            let node = &row.structural_node;
            let [CheckedNodePathSegment::Name(tag), CheckedNodePathSegment::Name(form), CheckedNodePathSegment::Name(original)] =
                node.path.as_slice()
            else {
                return Err(ReferenceCatalogError::ForeignModel);
            };
            if node.ordinal != 0
                || tag.as_ref() != "model"
                || form.as_ref() != "object_type"
                || !text_equal(&node.package, &key.package, budget)?
                || !text_equal(original, &key.node, budget)?
                || !text_equal(&row.selected_document, &key.package, budget)?
                || !text_equal(&row.original_identity, &key.node, budget)?
            {
                return Err(ReferenceCatalogError::ForeignModel);
            }
            let mut duplicate = false;
            for previous in &declarations {
                let same_type = StateModel::reference_identity_equal_with_work(
                    previous.effective,
                    effective,
                    &mut |units| work(budget, units),
                )?;
                let same_node = text_equal(&previous.key.package, &key.package, budget)?
                    && text_equal(&previous.key.node, &key.node, budget)?;
                if same_type != same_node {
                    return Err(ReferenceCatalogError::ConflictingMapping);
                }
                duplicate |= same_type;
            }
            if !duplicate {
                charge(
                    budget,
                    NativeJoinChargeKind::Storage,
                    size_of::<ReferenceDeclaration<'a>>(),
                )?;
                declarations
                    .try_reserve_exact(1)
                    .map_err(|_| ReferenceCatalogError::Allocation)?;
                declarations.push(ReferenceDeclaration {
                    row,
                    effective,
                    key,
                    universe,
                });
            }
        }
        Ok(Self {
            package,
            source,
            model,
            declarations,
        })
    }

    /// The exact admitted package retained by this catalog.
    pub fn package(&self) -> &CheckedPackageV2 {
        self.package
    }

    /// The genuine source checker owner, when built from its own emission.
    pub fn source(&self) -> Option<&CheckedPackage> {
        self.source
    }

    /// The actual native model against which every mapping was checked.
    pub fn model(&self) -> &StateModel {
        self.model
    }

    /// Borrow the admitted declarations; no unchecked insertion is exposed.
    pub fn declarations(&self) -> &[ReferenceDeclaration<'a>] {
        &self.declarations
    }
}
