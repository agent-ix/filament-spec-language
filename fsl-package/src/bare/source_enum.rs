// SPDX-License-Identifier: AGPL-3.0-or-later
//! Closed identities/6 encoding of the original source-enum producer rows.

use filament_contract_model::{
    CheckedNodeId, SelectedEnumDeclarationRow, SelectedEnumMemberRow, SelectedSourceEnumRow,
};
use filament_exact::{ValueType, VariantId};
use serde::{Deserialize, Serialize};

use super::{IdentityFileRefusal, IdentityMember};
use crate::equality_file::{OriginalNodeKey, OriginalValueType};

fn malformed() -> IdentityFileRefusal {
    IdentityFileRefusal::Malformed {
        member: IdentityMember::SourceEnums,
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceEnumJson {
    declaration: DeclarationJson,
    members: Vec<MemberJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DeclarationJson {
    structural_node: CheckedNodeId,
    source_authority: String,
    source_identity: String,
    qualified_declaration: Vec<String>,
    node_key: OriginalNodeKey,
    shape: OriginalValueType,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MemberJson {
    structural_node: CheckedNodeId,
    source_authority: String,
    source_identity: String,
    declaration_node: CheckedNodeId,
    declaration_qualified_name: Vec<String>,
    case: String,
    member_key: OriginalNodeKey,
    variant: VariantJson,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VariantJson {
    owner: OriginalNodeKey,
    case: String,
}

impl SourceEnumJson {
    pub(super) fn of(row: &SelectedSourceEnumRow) -> Result<Self, IdentityFileRefusal> {
        let declaration = &row.declaration;
        Ok(Self {
            declaration: DeclarationJson {
                structural_node: declaration.structural_node.clone(),
                source_authority: declaration.source_authority.to_string(),
                source_identity: declaration.source_identity.to_string(),
                qualified_declaration: declaration
                    .qualified_declaration
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                node_key: OriginalNodeKey::of(&declaration.node_key).map_err(|_| malformed())?,
                shape: OriginalValueType::of_enum_shape(&declaration.shape)
                    .map_err(|_| malformed())?,
            },
            members: row
                .members
                .iter()
                .map(|member| {
                    Ok(MemberJson {
                        structural_node: member.structural_node.clone(),
                        source_authority: member.source_authority.to_string(),
                        source_identity: member.source_identity.to_string(),
                        declaration_node: member.declaration_node.clone(),
                        declaration_qualified_name: member
                            .declaration_qualified_name
                            .iter()
                            .map(ToString::to_string)
                            .collect(),
                        case: member.case.to_string(),
                        member_key: OriginalNodeKey::of(&member.member_key)
                            .map_err(|_| malformed())?,
                        variant: VariantJson {
                            owner: OriginalNodeKey::of(member.variant.owner())
                                .map_err(|_| malformed())?,
                            case: member.variant.case().to_owned(),
                        },
                    })
                })
                .collect::<Result<_, IdentityFileRefusal>>()?,
        })
    }

    pub(super) fn row(self) -> Result<SelectedSourceEnumRow, IdentityFileRefusal> {
        let ValueType::Enum(shape) = self.declaration.shape.value().map_err(|_| malformed())?
        else {
            return Err(malformed());
        };
        Ok(SelectedSourceEnumRow {
            declaration: SelectedEnumDeclarationRow::new(
                self.declaration.structural_node,
                self.declaration.source_authority,
                self.declaration.source_identity,
                self.declaration
                    .qualified_declaration
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                self.declaration.node_key.key().map_err(|_| malformed())?,
                shape,
            ),
            members: self
                .members
                .into_iter()
                .map(|member| {
                    Ok(SelectedEnumMemberRow::new(
                        member.structural_node,
                        member.source_authority,
                        member.source_identity,
                        member.declaration_node,
                        member
                            .declaration_qualified_name
                            .into_iter()
                            .map(Into::into)
                            .collect(),
                        member.case,
                        member.member_key.key().map_err(|_| malformed())?,
                        VariantId::new(
                            member.variant.owner.key().map_err(|_| malformed())?,
                            member.variant.case,
                        )
                        .map_err(|_| malformed())?,
                    ))
                })
                .collect::<Result<_, IdentityFileRefusal>>()?,
        })
    }
}

/// Pay the owned iterator consumed by IR before cloning its producer rows.
pub(super) fn copy_rows(
    rows: &[SelectedSourceEnumRow],
    budget: &mut filament_contract_model::NativeJoinBudget,
) -> Result<Vec<SelectedSourceEnumRow>, crate::NativeRowError> {
    use crate::native_row as charge;
    let locus = charge::start(budget, "identity_source_enum_input")?;
    charge::items(budget, locus, rows.len())?;
    for (index, row) in rows.iter().enumerate() {
        let locus = filament_contract_model::NativeJoinLocus {
            source: filament_contract_model::NativeJoinLocusSource::AdapterRow(index),
            form: "identity_source_enum_input",
        };
        charge::work(budget, locus, 1)?;
        charge::enum_declaration(budget, locus, &row.declaration)?;
        charge::items(budget, locus, row.members.len())?;
        for member in &row.members {
            charge::enum_member(budget, locus, member)?;
        }
    }
    Ok(rows.to_vec())
}
