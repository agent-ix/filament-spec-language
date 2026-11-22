// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531: original selected-model union witnesses in the identities carrier.

use filament_contract_model::{
    CheckedNodeId, SelectedModelUnionCaseRow, SelectedModelUnionPayload, SelectedModelUnionRow,
};
use filament_exact::{OriginalDeclarationKey, UnionMember};
use serde::{Deserialize, Serialize};

use super::{IdentityFileRefusal, IdentityMember};
use crate::equality_file::{OriginalNodeKey, OriginalValueType};

fn malformed() -> IdentityFileRefusal {
    IdentityFileRefusal::Malformed {
        member: IdentityMember::ModelUnions,
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ModelUnionJson {
    structural_node: CheckedNodeId,
    selected_document: String,
    type_handle: OriginalNodeKey,
    settled_key: OriginalNodeKey,
    cases: Vec<CaseJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CaseJson {
    rank: u64,
    position: u64,
    name: String,
    original_case: OriginalCaseJson,
    member: OriginalNodeKey,
    payload: PayloadJson,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OriginalCaseJson {
    package: String,
    node: String,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum PayloadJson {
    None,
    Clock {
        payload_type: String,
    },
    Data {
        node: CheckedNodeId,
        original_type: OriginalValueType,
    },
}

impl ModelUnionJson {
    pub(super) fn of(row: &SelectedModelUnionRow) -> Result<Self, IdentityFileRefusal> {
        Ok(Self {
            structural_node: row.structural_node.clone(),
            selected_document: row.selected_document.to_string(),
            type_handle: OriginalNodeKey::of(&row.type_handle).map_err(|_| malformed())?,
            settled_key: OriginalNodeKey::of(&row.settled_key).map_err(|_| malformed())?,
            cases: row
                .cases
                .iter()
                .map(|case| {
                    Ok(CaseJson {
                        rank: case.rank,
                        position: case.position,
                        name: case.name.to_string(),
                        original_case: OriginalCaseJson {
                            package: case.original_case.package().to_owned(),
                            node: case.original_case.node().to_owned(),
                        },
                        member: OriginalNodeKey::of(case.member.declaration())
                            .map_err(|_| malformed())?,
                        payload: match &case.payload {
                            SelectedModelUnionPayload::None => PayloadJson::None,
                            SelectedModelUnionPayload::Clock { payload_type } => {
                                PayloadJson::Clock {
                                    payload_type: payload_type.to_string(),
                                }
                            }
                            SelectedModelUnionPayload::Data {
                                node,
                                original_type,
                            } => PayloadJson::Data {
                                node: node.clone(),
                                original_type: OriginalValueType::of(original_type)
                                    .map_err(|_| malformed())?,
                            },
                        },
                    })
                })
                .collect::<Result<_, IdentityFileRefusal>>()?,
        })
    }

    pub(super) fn row(self) -> Result<SelectedModelUnionRow, IdentityFileRefusal> {
        Ok(SelectedModelUnionRow {
            structural_node: self.structural_node,
            selected_document: self.selected_document.into(),
            type_handle: self.type_handle.key().map_err(|_| malformed())?,
            settled_key: self.settled_key.key().map_err(|_| malformed())?,
            cases: self
                .cases
                .into_iter()
                .map(|case| {
                    Ok(SelectedModelUnionCaseRow {
                        rank: case.rank,
                        position: case.position,
                        member: UnionMember::from_admitted(
                            case.member.key().map_err(|_| malformed())?,
                            case.name.clone(),
                        )
                        .map_err(|_| malformed())?,
                        name: case.name.into(),
                        original_case: OriginalDeclarationKey::new(
                            case.original_case.package,
                            case.original_case.node,
                        ),
                        payload: match case.payload {
                            PayloadJson::None => SelectedModelUnionPayload::None,
                            PayloadJson::Clock { payload_type } => {
                                SelectedModelUnionPayload::Clock {
                                    payload_type: payload_type.into(),
                                }
                            }
                            PayloadJson::Data {
                                node,
                                original_type,
                            } => SelectedModelUnionPayload::Data {
                                node,
                                original_type: original_type.value().map_err(|_| malformed())?,
                            },
                        },
                    })
                })
                .collect::<Result<_, IdentityFileRefusal>>()?,
        })
    }
}
