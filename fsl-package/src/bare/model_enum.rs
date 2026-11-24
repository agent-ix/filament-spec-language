// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531: original selected-model enum witnesses in the identities carrier.

use filament_contract_model::{
    CheckedNodeId, SelectedModelEnumDeclarationRow, SelectedModelEnumMemberRow,
    SelectedModelEnumShapeCaseRow, SelectedModelEnumShapeRow,
};
use filament_exact::{ValueType, VariantId};
use serde::{Deserialize, Serialize};

use super::{IdentityFileRefusal, IdentityMember};
use crate::equality_file::{OriginalNodeKey, OriginalValueType};

impl crate::SourceEnumCatalog {
    pub(crate) fn model_runtime_members(
        rows: &[SelectedModelEnumShapeRow],
        budget: &mut filament_contract_model::NativeJoinBudget,
    ) -> Result<filament_semantic_value::enumeration::EnumMemberIndex, crate::NativeRowError> {
        build_runtime_members(
            rows,
            budget,
            filament_contract_model::NativeJoinLocus {
                source: filament_contract_model::NativeJoinLocusSource::Package,
                form: "bare_model_enum_members",
            },
        )
    }
}

fn build_runtime_members(
    rows: &[SelectedModelEnumShapeRow],
    budget: &mut filament_contract_model::NativeJoinBudget,
    locus: filament_contract_model::NativeJoinLocus,
) -> Result<filament_semantic_value::enumeration::EnumMemberIndex, crate::NativeRowError> {
    use crate::native_row as charge;
    use filament_semantic_value::enumeration::{EnumDeclaration, EnumMemberIndex};
    charge::items(budget, locus, rows.len())?;
    for row in rows {
        charge::enum_shape(budget, locus, &row.shape)?;
        charge::items(budget, locus, row.cases.len())?;
        for case in &row.cases {
            charge::work(budget, locus, 1)?;
            charge::text(budget, locus, &case.case)?;
            charge::text(budget, locus, case.variant.case())?;
        }
    }
    let mut members = EnumMemberIndex::default();
    for row in rows {
        let declaration = EnumDeclaration::model_owned(
            row.node_key.clone(),
            row.cases.iter().map(|case| case.case.to_string()).collect(),
        )
        .expect("native model enum join authenticated canonical cases");
        for case in &row.cases {
            members.record(
                declaration
                    .member(&case.case, case.member_key.clone())
                    .expect("native model enum join authenticated each member"),
            );
        }
    }
    Ok(members)
}

fn malformed() -> IdentityFileRefusal {
    IdentityFileRefusal::Malformed {
        member: IdentityMember::ModelEnums,
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeclarationJson {
    structural_node: CheckedNodeId,
    selected_document: String,
    original_identity: String,
    node_key: OriginalNodeKey,
}

impl DeclarationJson {
    pub(super) fn of(row: &SelectedModelEnumDeclarationRow) -> Result<Self, IdentityFileRefusal> {
        Ok(Self {
            structural_node: row.structural_node.clone(),
            selected_document: row.selected_document.to_string(),
            original_identity: row.original_identity.to_string(),
            node_key: OriginalNodeKey::of(&row.node_key).map_err(|_| malformed())?,
        })
    }

    pub(super) fn row(self) -> Result<SelectedModelEnumDeclarationRow, IdentityFileRefusal> {
        Ok(SelectedModelEnumDeclarationRow::new(
            self.structural_node,
            self.selected_document,
            self.original_identity,
            self.node_key.key().map_err(|_| malformed())?,
        ))
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MemberJson {
    structural_node: CheckedNodeId,
    declaration_node: CheckedNodeId,
    selected_document: String,
    enum_original_identity: String,
    member_original_identity: String,
    case: String,
    member_key: OriginalNodeKey,
}

impl MemberJson {
    pub(super) fn of(row: &SelectedModelEnumMemberRow) -> Result<Self, IdentityFileRefusal> {
        Ok(Self {
            structural_node: row.structural_node.clone(),
            declaration_node: row.declaration_node.clone(),
            selected_document: row.selected_document.to_string(),
            enum_original_identity: row.enum_original_identity.to_string(),
            member_original_identity: row.member_original_identity.to_string(),
            case: row.case.to_string(),
            member_key: OriginalNodeKey::of(&row.member_key).map_err(|_| malformed())?,
        })
    }

    pub(super) fn row(self) -> Result<SelectedModelEnumMemberRow, IdentityFileRefusal> {
        Ok(SelectedModelEnumMemberRow::new(
            self.structural_node,
            self.declaration_node,
            self.selected_document,
            self.enum_original_identity,
            self.member_original_identity,
            self.case,
            self.member_key.key().map_err(|_| malformed())?,
        ))
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ShapeJson {
    structural_node: CheckedNodeId,
    node_key: OriginalNodeKey,
    selected_document: String,
    original_identity: String,
    shape: OriginalValueType,
    cases: Vec<CaseJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CaseJson {
    case: String,
    structural_node: CheckedNodeId,
    member_key: OriginalNodeKey,
    member_original_identity: String,
    variant: VariantJson,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VariantJson {
    owner: OriginalNodeKey,
    case: String,
}

impl ShapeJson {
    pub(super) fn of(row: &SelectedModelEnumShapeRow) -> Result<Self, IdentityFileRefusal> {
        Ok(Self {
            structural_node: row.structural_node.clone(),
            node_key: OriginalNodeKey::of(&row.node_key).map_err(|_| malformed())?,
            selected_document: row.selected_document.to_string(),
            original_identity: row.original_identity.to_string(),
            shape: OriginalValueType::of_enum_shape(&row.shape).map_err(|_| malformed())?,
            cases: row
                .cases
                .iter()
                .map(|case| {
                    Ok(CaseJson {
                        case: case.case.to_string(),
                        structural_node: case.structural_node.clone(),
                        member_key: OriginalNodeKey::of(&case.member_key)
                            .map_err(|_| malformed())?,
                        member_original_identity: case.member_original_identity.to_string(),
                        variant: VariantJson {
                            owner: OriginalNodeKey::of(case.variant.owner())
                                .map_err(|_| malformed())?,
                            case: case.variant.case().to_owned(),
                        },
                    })
                })
                .collect::<Result<_, IdentityFileRefusal>>()?,
        })
    }

    pub(super) fn row(self) -> Result<SelectedModelEnumShapeRow, IdentityFileRefusal> {
        let mut value_type = self.shape.value().map_err(|_| malformed())?;
        let ValueType::Enum(shape) = &mut value_type else {
            return Err(malformed());
        };
        let shape = std::mem::replace(shape, filament_exact::EnumShape::new(false, []));
        Ok(SelectedModelEnumShapeRow {
            structural_node: self.structural_node,
            node_key: self.node_key.key().map_err(|_| malformed())?,
            selected_document: self.selected_document.into(),
            original_identity: self.original_identity.into(),
            shape,
            cases: self
                .cases
                .into_iter()
                .map(|case| {
                    Ok(SelectedModelEnumShapeCaseRow {
                        case: case.case.into(),
                        structural_node: case.structural_node,
                        member_key: case.member_key.key().map_err(|_| malformed())?,
                        member_original_identity: case.member_original_identity.into(),
                        variant: VariantId::new(
                            case.variant.owner.key().map_err(|_| malformed())?,
                            case.variant.case,
                        )
                        .map_err(|_| malformed())?,
                    })
                })
                .collect::<Result<_, IdentityFileRefusal>>()?,
        })
    }
}
