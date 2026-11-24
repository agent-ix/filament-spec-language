// SPDX-License-Identifier: AGPL-3.0-or-later
//! The ownerless model declaration denoted by authenticated context candidates.

use filament_contract_model::{
    NativeJoinBudget, NativeJoinLocus, SelectedExactDeclarationRow, SelectedExactIdentity,
};
use filament_exact::{EffectiveId, NodeKey};
use fsl_semantics::check::wire_id::{
    StructuralKeyMaterializationError, StructuralKeys, WireIdError,
};

use crate::native_row::{self, NativeRowError};
use crate::{BarePackage, BareSelectionFailure, OwnedNodeRef};

/// Equal model declaration facts may be retained by several component
/// libraries. This projection assigns no library owner to their common value.
#[derive(Debug, Eq, PartialEq)]
pub struct CompositionModelContext<'a> {
    document: &'a str,
    identity: &'a EffectiveId,
    node: NodeKey,
}
impl CompositionModelContext<'_> {
    /// The actual selected domain document from the authenticated joining row.
    pub fn selected_document(&self) -> &str {
        self.document
    }
    /// The actual selected effective object identity.
    pub fn identity(&self) -> &EffectiveId {
        self.identity
    }
    /// The document-independent structural declaration identity.
    pub fn node(&self) -> &NodeKey {
        &self.node
    }
}

/// A model context is absent or conflicting; no library is chosen as fallback.
#[derive(Debug, thiserror::Error)]
pub enum CompositionModelContextRefusal {
    /// No retained authenticated view matches a candidate's admission owner.
    #[error("the context's admitted owner is not retained")]
    MissingOwner,
    /// Several retained views match the same admission owner.
    #[error("the context's admission owner has duplicate retained views")]
    DuplicateOwner,
    /// The actual owning view's identity witness is unavailable or refused.
    #[error("the context identity witness refused: {0:?}")]
    Selection(BareSelectionFailure),
    /// No authenticated object declaration row matches the target.
    #[error("the context has no joined object declaration")]
    MissingObject,
    /// A supposedly keyed declaration witness holds several matching rows.
    #[error("the context has duplicate joined declaration rows")]
    DuplicateDeclaration,
    /// Candidate domain, effective or structural identities disagree.
    #[error("authenticated context candidates denote different model declarations")]
    ConflictingContexts,
    /// The original native allowance refused a read or copy.
    #[error(transparent)]
    Native(#[from] NativeRowError),
    /// A structural identity violates the existing key policy.
    #[error(transparent)]
    Structural(#[from] WireIdError),
    /// A prepaid container could not reserve its storage.
    #[error(transparent)]
    Allocation(#[from] std::collections::TryReserveError),
}

impl<'p> BarePackage<'p> {
    /// Project authenticated owner-bound model targets to one common semantic
    /// context. Every actual candidate is inspected. Equal facts collapse
    /// regardless of candidate order; missing or differing facts refuse.
    ///
    /// This also binds a component's single subject to its actual selected
    /// document, without deriving a model name from a node's package spelling.
    ///
    /// # Errors
    /// Preserves identity-witness refusal and the caller's original native
    /// first-unpaid kind, attempted amount, configured bound and locus.
    pub fn model_context_with_budget<'a>(
        &'a self,
        candidates: &[OwnedNodeRef<'p>],
        budget: &mut NativeJoinBudget,
    ) -> Result<CompositionModelContext<'a>, CompositionModelContextRefusal> {
        let locus = native_row::start(budget, "composition_model_context")?;
        native_row::work(budget, locus, 1)?;
        native_row::items(budget, locus, candidates.len())?;
        let mut rows = Vec::new();
        rows.try_reserve_exact(candidates.len())?;
        for candidate in candidates {
            let mut owner = None;
            for view in std::iter::once(self).chain(self.dependency_views()) {
                native_row::work(budget, locus, 1)?;
                if view.package().admission_binding() == candidate.owner().admission_binding() {
                    if owner.is_some() {
                        return Err(CompositionModelContextRefusal::DuplicateOwner);
                    }
                    owner = Some(view);
                }
            }
            let owner = owner.ok_or(CompositionModelContextRefusal::MissingOwner)?;
            let (_, identities) = owner
                .witness()
                .map_err(CompositionModelContextRefusal::Selection)?;
            native_row::work(budget, locus, 1)?;
            let structural = owner
                .package()
                .structural_key(candidate.node())
                .ok_or(CompositionModelContextRefusal::MissingObject)?;
            let mut found = None;
            // declaration_rows is keyed uniquely by structural node identity in
            // the admitted witness. Keep the duplicate guard for this boundary.
            for row in identities.declaration_rows() {
                native_row::work(budget, locus, 1)?;
                if row.structural_node == *structural {
                    if found.is_some() {
                        return Err(CompositionModelContextRefusal::DuplicateDeclaration);
                    }
                    found = Some(row);
                }
            }
            rows.push(found.ok_or(CompositionModelContextRefusal::MissingObject)?);
        }
        common_rows(&rows, budget, locus)
    }
}

fn equal_text(
    left: &str,
    right: &str,
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
) -> Result<bool, NativeRowError> {
    let (mut left, mut right) = (left.bytes(), right.bytes());
    loop {
        native_row::work(budget, locus, 1)?;
        match (left.next(), right.next()) {
            (Some(a), Some(b)) if a == b => {}
            (None, None) => return Ok(true),
            _ => return Ok(false),
        }
    }
}
fn equal_effective(
    left: &EffectiveId,
    right: &EffectiveId,
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
) -> Result<bool, NativeRowError> {
    let (mut left, mut right) = (left.owner_chain().iter(), right.owner_chain().iter());
    loop {
        native_row::work(budget, locus, 1)?;
        match (left.next(), right.next()) {
            (Some(a), Some(b)) => {
                if !equal_text(a.package(), b.package(), budget, locus)?
                    || !equal_text(a.node(), b.node(), budget, locus)?
                {
                    return Ok(false);
                }
            }
            (None, None) => return Ok(true),
            _ => return Ok(false),
        }
    }
}
pub(crate) fn common_rows<'a>(
    rows: &[&'a SelectedExactDeclarationRow],
    budget: &mut NativeJoinBudget,
    locus: NativeJoinLocus,
) -> Result<CompositionModelContext<'a>, CompositionModelContextRefusal> {
    let mut keys = StructuralKeys::default();
    let mut common: Option<CompositionModelContext<'a>> = None;
    for row in rows {
        native_row::work(budget, locus, 1)?;
        let SelectedExactIdentity::Object(identity) = &row.exact_identity else {
            return Err(CompositionModelContextRefusal::MissingObject);
        };
        let node = match keys.try_key(&row.structural_node, budget, locus) {
            Ok(node) => node,
            Err(StructuralKeyMaterializationError::Wire(error)) => return Err(error.into()),
            Err(StructuralKeyMaterializationError::Budget { denied, .. }) => {
                return Err(NativeRowError::exhausted(denied, budget).into());
            }
            Err(StructuralKeyMaterializationError::Allocation(error)) => return Err(error.into()),
        };
        if let Some(common) = &common {
            if !equal_text(common.document, &row.selected_document, budget, locus)?
                || !equal_effective(common.identity, identity, budget, locus)?
            {
                return Err(CompositionModelContextRefusal::ConflictingContexts);
            }
            native_row::work(budget, locus, 1)?;
            if common.node != node {
                return Err(CompositionModelContextRefusal::ConflictingContexts);
            }
        } else {
            common = Some(CompositionModelContext {
                document: &row.selected_document,
                identity,
                node,
            });
        }
    }
    common.ok_or(CompositionModelContextRefusal::MissingObject)
}
