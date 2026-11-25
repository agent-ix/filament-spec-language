// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531: selected-model tuple declaration rows in the identities carrier.

use filament_contract_model::{CheckedNodeId, SelectedTupleDeclarationRow};
use filament_exact::NodeKey;
use serde::{Deserialize, Serialize};

use super::{IdentityFileRefusal, IdentityMember};
use crate::equality_file::OriginalNodeKey;

fn malformed() -> IdentityFileRefusal {
    IdentityFileRefusal::Malformed {
        member: IdentityMember::ModelTuples,
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ModelTupleJson {
    structural_node: CheckedNodeId,
    selected_document: String,
    original_identity: String,
    node_key: OriginalNodeKey,
}

impl ModelTupleJson {
    pub(super) fn of(row: &SelectedTupleDeclarationRow) -> Result<Self, IdentityFileRefusal> {
        Ok(Self {
            structural_node: row.structural_node.clone(),
            selected_document: row.selected_document.to_string(),
            original_identity: row.original_identity.to_string(),
            node_key: OriginalNodeKey::of(&row.node_key).map_err(|_| malformed())?,
        })
    }

    pub(super) fn row(self) -> Result<SelectedTupleDeclarationRow, IdentityFileRefusal> {
        let node_key: NodeKey = self.node_key.key().map_err(|_| malformed())?;
        Ok(SelectedTupleDeclarationRow::new(
            self.structural_node,
            self.selected_document,
            self.original_identity,
            node_key,
        ))
    }
}
