// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531: immutable runtime enums retained from the package's bounded joins.

use std::sync::Arc;

use filament_contract_model::{
    CheckedNodeId, NativeJoinBudget, NativeJoinChargeKind, NativeJoinLocus, NativeJoinLocusSource,
    SelectedExactIdentities, SelectedModelEnumShapeRow, SelectedSourceEnumRow,
};
use filament_exact::{EnumShape, NodeKey, NodeKeyView, NodePathIter, PathSegmentView, VariantId};
use filament_semantic_value::enumeration::{EnumDeclaration, EnumMemberIndex};

use crate::{native_row, NativeRowError};

/// The authenticated row family at a catalog construction refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceEnumRowKind {
    /// An enum declared by an admitted source unit.
    Source,
    /// An enum declared by an admitted model document.
    Model,
}

/// A catalog failure, retaining the original native denial without allocation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SourceEnumCatalogError {
    /// The caller's original work or storage allowance refused a charge.
    #[error(transparent)]
    Native(#[from] NativeRowError),
    /// A paid physical reservation failed.
    #[error("enum catalog allocation failed")]
    Allocation,
    /// Joined rows did not satisfy a runtime enum invariant.
    #[error("invalid joined enum at {row_kind:?} row {index}")]
    InvalidJoinedInvariant {
        /// The row family.
        row_kind: SourceEnumRowKind,
        /// Position in that family's joined rows.
        index: usize,
    },
    /// Two retained members claimed the same variant identity.
    #[error("conflicting enum member at {row_kind:?} row {index}, case {case}")]
    ConflictingMember {
        /// The row family.
        row_kind: SourceEnumRowKind,
        /// Position in that family's joined rows.
        index: usize,
        /// Position in the row's canonical case list.
        case: usize,
    },
    /// Forms cannot name this generated or qualified source enum yet.
    #[error("source enum Forms scope is unsupported")]
    UnsupportedSourceScope(SourceEnumUnsupportedScope),
    /// An admitted lookup was attempted without its original permission.
    #[error("source enum lookup requires the original native work permission")]
    MissingNativePermission,
    /// No authenticated source or model enum supplies this identity.
    #[error("source enum identity witness is unavailable")]
    MissingIdentityWitness,
}

/// Original unsupported source metadata retained without copying its payload.
#[derive(Clone, Debug)]
pub struct SourceEnumUnsupportedScope {
    catalog: SourceEnumCatalog,
    index: usize,
}

impl PartialEq for SourceEnumUnsupportedScope {
    fn eq(&self, other: &Self) -> bool {
        self.declaration() == other.declaration()
            && self.qualified_declaration() == other.qualified_declaration()
    }
}

impl Eq for SourceEnumUnsupportedScope {}

impl SourceEnumUnsupportedScope {
    /// The actual admitted declaration, never a reconstructed identity.
    pub fn declaration(&self) -> &CheckedNodeId {
        &self.row().declaration.structural_node
    }

    /// The actual qualified declaration retained by the native join.
    pub fn qualified_declaration(&self) -> &[Box<str>] {
        &self.row().declaration.qualified_declaration
    }

    fn row(&self) -> &SelectedSourceEnumRow {
        // Only unsupported_scope_refusal constructs this immutable handle.
        &self.catalog.0.sources[self.index]
    }
}

/// A declaration classification whose metadata borrows the paid catalog.
#[derive(Clone, Copy, Debug)]
pub enum EnumScope<'a> {
    /// An ordinary authored enum's exact singleton Forms name.
    NamedSource {
        /// The name supplied by its authenticated declaration row.
        name: &'a str,
    },
    /// A source enum whose qualified/generated Forms name is unsupported.
    UnsupportedSourceScope {
        /// Its actual admitted declaration.
        declaration: &'a CheckedNodeId,
        /// Its actual qualified declaration.
        qualified_declaration: &'a [Box<str>],
        /// Catalog position for a paid, lifetime-independent refusal handle.
        index: usize,
    },
    /// An exact shape or variant authenticated by an actual model row.
    Model {
        /// The original declaration identity authenticated by its model row.
        name: &'a str,
    },
    /// Neither joined source nor model rows supplied this identity.
    Missing,
}

/// Source and model enum rows and one shared runtime member index.
///
/// Construction is private to the successful package-local native join. A
/// clone retains the same immutable rows and member index through an Arc.
#[derive(Clone, Debug)]
pub struct SourceEnumCatalog(Arc<CatalogData>);

#[derive(Debug)]
struct CatalogData {
    sources: Vec<SelectedSourceEnumRow>,
    models: Vec<SelectedModelEnumShapeRow>,
    members: EnumMemberIndex,
}

fn locus(index: usize, form: &'static str) -> NativeJoinLocus {
    NativeJoinLocus {
        source: NativeJoinLocusSource::AdapterRow(index),
        form,
    }
}

fn charge(
    budget: &mut NativeJoinBudget,
    at: NativeJoinLocus,
    kind: NativeJoinChargeKind,
    amount: u128,
) -> Result<(), SourceEnumCatalogError> {
    budget
        .accounted_charge(kind, amount, at)
        .map_err(|denied| NativeRowError::exhausted(denied, budget).into())
}

fn work(budget: &mut NativeJoinBudget, at: NativeJoinLocus) -> Result<(), SourceEnumCatalogError> {
    charge(budget, at, NativeJoinChargeKind::Work, 1)
}

fn push<T>(
    items: &mut Vec<T>,
    item: T,
    budget: &mut NativeJoinBudget,
    at: NativeJoinLocus,
) -> Result<(), SourceEnumCatalogError> {
    if items.len() == items.capacity() {
        charge(budget, at, NativeJoinChargeKind::Storage, 2)?;
        items
            .try_reserve_exact(1)
            .map_err(|_| SourceEnumCatalogError::Allocation)?;
    }
    items.push(item);
    Ok(())
}

impl SourceEnumCatalog {
    pub(crate) fn from_joined(
        joined: &SelectedExactIdentities,
        model_rows: &[SelectedModelEnumShapeRow],
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, SourceEnumCatalogError> {
        let at = locus(0, "source_enum_catalog");
        let has_enum_rows = joined.source_enum_rows().next().is_some() || !model_rows.is_empty();
        if has_enum_rows {
            work(budget, at)?;
            charge(budget, at, NativeJoinChargeKind::Storage, 1)?;
        }
        let mut sources = Vec::new();
        for row in joined.source_enum_rows() {
            let at = locus(sources.len(), "source_enum_catalog_source");
            work(budget, at)?;
            native_row::enum_declaration(budget, at, &row.declaration)?;
            native_row::items(budget, at, row.members.len())?;
            for member in &row.members {
                work(budget, at)?;
                if !node_equal(
                    member.variant.owner(),
                    &row.declaration.node_key,
                    budget,
                    at,
                )? || !text_equal(member.variant.case(), &member.case, budget, at)?
                {
                    return Err(SourceEnumCatalogError::InvalidJoinedInvariant {
                        row_kind: SourceEnumRowKind::Source,
                        index: sources.len(),
                    });
                }
                native_row::enum_member(budget, at, member)?;
            }
            // Reserve before cloning the row, including its owned member vector.
            if sources.len() == sources.capacity() {
                charge(budget, at, NativeJoinChargeKind::Storage, 2)?;
                sources
                    .try_reserve_exact(1)
                    .map_err(|_| SourceEnumCatalogError::Allocation)?;
            }
            sources.push(row.clone());
        }
        native_row::items(budget, at, model_rows.len())?;
        let mut models = Vec::new();
        models
            .try_reserve_exact(model_rows.len())
            .map_err(|_| SourceEnumCatalogError::Allocation)?;
        for (index, row) in model_rows.iter().enumerate() {
            let at = locus(index, "source_enum_catalog_model");
            work(budget, at)?;
            native_row::node(budget, at, &row.structural_node)?;
            native_row::text(budget, at, &row.selected_document)?;
            native_row::text(budget, at, &row.original_identity)?;
            native_row::enum_shape(budget, at, &row.shape)?;
            native_row::items(budget, at, row.cases.len())?;
            for case in &row.cases {
                native_row::node(budget, at, &case.structural_node)?;
                native_row::text(budget, at, &case.case)?;
                native_row::text(budget, at, &case.member_original_identity)?;
                native_row::text(budget, at, case.variant.case())?;
            }
            models.push(row.clone());
        }
        // This builder returns one fresh, uniquely owned index. It is never
        // extended after a shallow clone, which would copy the index's map.
        if has_enum_rows {
            let index_at = locus(0, "source_enum_catalog_index");
            work(budget, index_at)?;
            charge(budget, index_at, NativeJoinChargeKind::Storage, 1)?;
        }
        let mut members = Self::model_runtime_members(&models, budget)?;
        for (index, row) in sources.iter().enumerate() {
            let at = locus(index, "source_enum_catalog_members");
            let mut cases = Vec::new();
            native_row::items(budget, at, row.members.len())?;
            cases
                .try_reserve_exact(row.members.len())
                .map_err(|_| SourceEnumCatalogError::Allocation)?;
            for member in &row.members {
                native_row::text(budget, at, &member.case)?;
                cases.push(member.case.to_string());
            }
            native_row::items(budget, at, cases.len())?;
            let declaration = EnumDeclaration::new(
                row.declaration.node_key.clone(),
                row.declaration.shape.is_ordered(),
                cases,
            )
            .map_err(|_| SourceEnumCatalogError::InvalidJoinedInvariant {
                row_kind: SourceEnumRowKind::Source,
                index,
            })?;
            for (case, member) in row.members.iter().enumerate() {
                work(budget, at)?;
                // Pay the member's case and the index's variant case before
                // their two owned strings are constructed by SV.
                native_row::text(budget, at, &member.case)?;
                native_row::text(budget, at, &member.case)?;
                native_row::items(budget, at, 1)?;
                let previous_variants = models
                    .iter()
                    .flat_map(|model| model.cases.iter().map(|case| &case.variant))
                    .chain(
                        sources[..index]
                            .iter()
                            .flat_map(|source| source.members.iter().map(|member| &member.variant)),
                    )
                    .chain(row.members[..case].iter().map(|member| &member.variant));
                for previous in previous_variants {
                    if variant_equal(previous, &member.variant, budget, at)? {
                        return Err(SourceEnumCatalogError::ConflictingMember {
                            row_kind: SourceEnumRowKind::Source,
                            index,
                            case,
                        });
                    }
                }
                let value = declaration
                    .member(&member.case, member.member_key.clone())
                    .map_err(|_| SourceEnumCatalogError::InvalidJoinedInvariant {
                        row_kind: SourceEnumRowKind::Source,
                        index,
                    })?;
                members.record(value);
            }
        }
        Ok(Self(Arc::new(CatalogData {
            sources,
            models,
            members,
        })))
    }

    /// The single immutable index containing actual source and model members.
    pub fn enum_members(&self) -> &EnumMemberIndex {
        &self.0.members
    }

    /// Match a complete ordered shape under the caller's original permission.
    pub fn classify_shape<'a>(
        &'a self,
        shape: &EnumShape,
        budget: &mut NativeJoinBudget,
    ) -> Result<EnumScope<'a>, SourceEnumCatalogError> {
        let at = locus(0, "source_enum_catalog_shape");
        work(budget, at)?;
        for (index, row) in self.0.sources.iter().enumerate() {
            let at = locus(index, "source_enum_catalog_source_shape");
            if shape_equal(&row.declaration.shape, shape, budget, at)? {
                return Ok(self.source_scope(index));
            }
        }
        for (index, row) in self.0.models.iter().enumerate() {
            let at = locus(index, "source_enum_catalog_model_shape");
            if shape_equal(&row.shape, shape, budget, at)? {
                return Ok(EnumScope::Model {
                    name: &row.original_identity,
                });
            }
        }
        Ok(EnumScope::Missing)
    }

    /// Match an exact variant identity, without guessing a name or model owner.
    pub fn classify_variant<'a>(
        &'a self,
        variant: &VariantId,
        budget: &mut NativeJoinBudget,
    ) -> Result<EnumScope<'a>, SourceEnumCatalogError> {
        let at = locus(0, "source_enum_catalog_variant");
        work(budget, at)?;
        for (index, row) in self.0.sources.iter().enumerate() {
            let at = locus(index, "source_enum_catalog_source_variant");
            for member in &row.members {
                if variant_equal(&member.variant, variant, budget, at)? {
                    return Ok(self.source_scope(index));
                }
            }
        }
        for (index, row) in self.0.models.iter().enumerate() {
            let at = locus(index, "source_enum_catalog_model_variant");
            for case in &row.cases {
                if variant_equal(&case.variant, variant, budget, at)? {
                    return Ok(EnumScope::Model {
                        name: &row.original_identity,
                    });
                }
            }
        }
        Ok(EnumScope::Missing)
    }

    fn source_scope(&self, index: usize) -> EnumScope<'_> {
        let row = &self.0.sources[index];
        match row.declaration.qualified_declaration.as_slice() {
            [name] => EnumScope::NamedSource { name },
            qualified_declaration => EnumScope::UnsupportedSourceScope {
                declaration: &row.declaration.structural_node,
                qualified_declaration,
                index,
            },
        }
    }

    /// Retain an unsupported declaration's actual context after paying for its
    /// fixed shared handle. A native denial replaces no original metadata.
    pub fn unsupported_scope_refusal(
        &self,
        index: usize,
        budget: &mut NativeJoinBudget,
    ) -> SourceEnumCatalogError {
        let at = locus(index, "source_enum_catalog_unsupported");
        if let Err(error) = work(budget, at) {
            return error;
        }
        let Some(row) = self.0.sources.get(index) else {
            return SourceEnumCatalogError::InvalidJoinedInvariant {
                row_kind: SourceEnumRowKind::Source,
                index,
            };
        };
        if row.declaration.qualified_declaration.len() == 1 {
            return SourceEnumCatalogError::InvalidJoinedInvariant {
                row_kind: SourceEnumRowKind::Source,
                index,
            };
        }
        if let Err(error) = charge(budget, at, NativeJoinChargeKind::Storage, 1) {
            return error;
        }
        SourceEnumCatalogError::UnsupportedSourceScope(SourceEnumUnsupportedScope {
            catalog: self.clone(),
            index,
        })
    }
}

fn text_equal(
    left: &str,
    right: &str,
    budget: &mut NativeJoinBudget,
    at: NativeJoinLocus,
) -> Result<bool, SourceEnumCatalogError> {
    work(budget, at)?;
    if left.len() != right.len() {
        return Ok(false);
    }
    let (mut left, mut right) = (left.bytes(), right.bytes());
    loop {
        work(budget, at)?;
        match (left.next(), right.next()) {
            (Some(left), Some(right)) if left == right => {}
            (None, None) => return Ok(true),
            _ => return Ok(false),
        }
    }
}

fn variant_equal(
    left: &VariantId,
    right: &VariantId,
    budget: &mut NativeJoinBudget,
    at: NativeJoinLocus,
) -> Result<bool, SourceEnumCatalogError> {
    work(budget, at)?;
    Ok(node_equal(left.owner(), right.owner(), budget, at)?
        && text_equal(left.case(), right.case(), budget, at)?)
}

fn shape_equal(
    left: &EnumShape,
    right: &EnumShape,
    budget: &mut NativeJoinBudget,
    at: NativeJoinLocus,
) -> Result<bool, SourceEnumCatalogError> {
    work(budget, at)?;
    if left.is_ordered() != right.is_ordered() {
        return Ok(false);
    }
    let (mut left, mut right) = (left.variants(), right.variants());
    loop {
        work(budget, at)?;
        match (left.next(), right.next()) {
            (Some(left), Some(right)) => {
                if !variant_equal(left, right, budget, at)? {
                    return Ok(false);
                }
            }
            (None, None) => return Ok(true),
            _ => return Ok(false),
        }
    }
}

fn node_header(
    left: NodeKeyView<'_>,
    right: NodeKeyView<'_>,
    budget: &mut NativeJoinBudget,
    at: NativeJoinLocus,
) -> Result<bool, SourceEnumCatalogError> {
    work(budget, at)?;
    Ok(left.ordinal() == right.ordinal()
        && text_equal(left.package(), right.package(), budget, at)?)
}

fn node_equal(
    left: &NodeKey,
    right: &NodeKey,
    budget: &mut NativeJoinBudget,
    at: NativeJoinLocus,
) -> Result<bool, SourceEnumCatalogError> {
    if !node_header(left.view(), right.view(), budget, at)? {
        return Ok(false);
    }
    let (mut left, mut right) = (left.view().path(), right.view().path());
    let mut parents: Vec<(NodePathIter<'_>, NodePathIter<'_>)> = Vec::new();
    loop {
        work(budget, at)?;
        match (left.next(), right.next()) {
            (Some(PathSegmentView::Text(left)), Some(PathSegmentView::Text(right))) => {
                if !text_equal(left, right, budget, at)? {
                    return Ok(false);
                }
            }
            (Some(PathSegmentView::Node(child_left)), Some(PathSegmentView::Node(child_right))) => {
                if !node_header(child_left, child_right, budget, at)? {
                    return Ok(false);
                }
                push(&mut parents, (left, right), budget, at)?;
                left = child_left.path();
                right = child_right.path();
            }
            (None, None) => match parents.pop() {
                Some((parent_left, parent_right)) => {
                    left = parent_left;
                    right = parent_right;
                }
                None => return Ok(true),
            },
            _ => return Ok(false),
        }
    }
}

#[cfg(test)]
#[path = "../../fsl-semantics/tests/support/variant_fields.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "package tests share the genuine model fixture"
)]
mod model_fixture;

#[cfg(test)]
mod tests {
    use super::*;
    use filament_contract_model::{
        CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2,
        CheckedPackageV2ReadResult, ExactTypeProjectionError,
    };
    use filament_semantic_value::checking::{CheckMode, CheckingLimits};
    use ix_trace_rs::trace;

    const ENUMS: &str = "enum Status { READY, DONE }\nordered enum Level { LOW, MID, HIGH }\nfunction ready using v(s: Status): Boolean pure { s = Status::READY }\n";

    struct Fixture {
        emission: crate::Emission,
        admitted: CheckedPackageV2,
        checked: crate::CheckedPackage,
    }

    fn fixture(body: &str, model: bool) -> Fixture {
        let mut evidence = CheckedPackageEvidence::new();
        let checked = if model {
            let document = serde_json::to_vec(&model_fixture::document()).unwrap();
            let packages =
                fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
            let text = format!(
                "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nmodel M = {:?};\n{body}",
                model_fixture::PACKAGE
            );
            let parsed = fsl_cst::parse(
                fsl_foundation::SourceIdentity::new("test", "enum-catalog"),
                "catalog.native",
                text.as_bytes(),
                fsl_cst::Limits::default(),
            )
            .unwrap();
            assert_eq!(parsed.diagnostics(), []);
            let unit = fsl_forms::build_unit(&parsed).unwrap();
            let models = fsl_semantics::model::intake::admit_unit(
                &unit.selections().models,
                &packages,
                fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
            )
            .unwrap();
            let declarations = fsl_semantics::check::PackageDeclarations::assemble(
                parsed.source().reference().clone(),
                "pkg".into(),
                unit,
                models,
                Vec::new(),
            )
            .unwrap();
            evidence
                .insert_domain_package_document(model_fixture::PACKAGE, document.as_slice())
                .unwrap();
            crate::CheckedPackage::link(
                declarations
                    .check(
                        CheckMode::Linked,
                        CheckingLimits::default(),
                        &fsl_semantics::check::test_support::ZeroRewardEvaluator,
                    )
                    .unwrap(),
            )
        } else {
            crate::emit::tests::package_from_text(body).1
        };
        let emission = crate::emit_checked(&checked, crate::EmitLimits::fixture()).unwrap();
        let wire: serde_json::Value = serde_json::from_slice(emission.package.bytes()).unwrap();
        for feature in wire["lock"]["required_features"].as_array().unwrap() {
            evidence.support_feature(feature.as_str().unwrap());
        }
        let CheckedPackageV2ReadResult::Admitted(admitted) = CheckedPackageV2::read(
            emission.package.bytes(),
            CheckedPackageReadLimits::bounded(),
            &evidence,
        ) else {
            panic!("the actual source emission must admit")
        };
        Fixture {
            emission,
            admitted: *admitted,
            checked,
        }
    }

    #[derive(Debug)]
    enum BuildStop {
        Adapter(NativeRowError),
        Join(ExactTypeProjectionError),
        Catalog(SourceEnumCatalogError),
    }

    fn build(
        fixture: &Fixture,
        budget: &mut NativeJoinBudget,
    ) -> Result<SourceEnumCatalog, BuildStop> {
        let sources = fixture
            .emission
            .selected_source_enum_rows(budget)
            .map_err(BuildStop::Adapter)?;
        let joined = SelectedExactIdentities::empty()
            .with_source_enum_rows(&fixture.admitted, sources, budget)
            .map_err(BuildStop::Join)?;
        SourceEnumCatalog::from_joined(&joined, &[], budget).map_err(BuildStop::Catalog)
    }

    #[trace("TC-1331", "FR-531-AC-50", "FR-531-AC-44")]
    #[test]
    fn source_enum_catalog_retains_real_unused_cases_order_keys_and_shared_members() {
        let fixture = fixture(ENUMS, false);
        let mut budget = NativeJoinBudget::new(u64::MAX, u64::MAX);
        let original = fixture
            .emission
            .selected_source_enum_rows(&mut budget)
            .unwrap();
        let catalog = build(&fixture, &mut budget).unwrap();
        let cloned = catalog.clone();
        assert!(Arc::ptr_eq(&catalog.0, &cloned.0));
        for row in &original {
            let EnumScope::NamedSource { name } = catalog
                .classify_shape(&row.declaration.shape, &mut budget)
                .unwrap()
            else {
                panic!("ordinary source enum keeps its actual name")
            };
            assert_eq!(name, row.declaration.qualified_declaration[0].as_ref());
            let cases: Vec<_> = row
                .members
                .iter()
                .map(|member| member.case.as_ref())
                .collect();
            assert_eq!(
                cases,
                if name == "Status" {
                    vec!["DONE", "READY"]
                } else {
                    vec!["LOW", "MID", "HIGH"]
                }
            );
            for (rank, member) in row.members.iter().enumerate() {
                let actual = catalog
                    .enum_members()
                    .resolve(member.variant.clone())
                    .unwrap();
                assert_eq!(actual.member(), member.member_key);
                assert_eq!(actual.variant(), member.variant);
                assert_eq!(actual.position(), rank);
                assert_eq!(actual.is_ordered(), row.declaration.shape.is_ordered());
                assert!(std::ptr::eq(
                    actual,
                    cloned
                        .enum_members()
                        .resolve(member.variant.clone())
                        .unwrap()
                ));
                assert!(matches!(
                    catalog
                        .classify_variant(&member.variant, &mut budget)
                        .unwrap(),
                    EnumScope::NamedSource { .. }
                ));
            }
            let wrong = EnumShape::new(
                !row.declaration.shape.is_ordered(),
                row.declaration.shape.variants().cloned(),
            );
            assert!(matches!(
                catalog.classify_shape(&wrong, &mut budget).unwrap(),
                EnumScope::Missing
            ));
        }
    }

    #[trace("TC-1331", "FR-531-AC-50", "FR-531-AC-43")]
    #[test]
    fn source_enum_catalog_refuses_paired_variant_identity_changes_accepted_by_native_join() {
        let fixture = fixture(ENUMS, false);
        let mut budget = NativeJoinBudget::new(u64::MAX, u64::MAX);
        let original = fixture
            .emission
            .identity_rows(&fixture.admitted, &mut budget)
            .unwrap();
        let bytes = original.to_bytes(&fixture.admitted).unwrap();
        let file: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(file["format"], "filament.checked-handoff.identities/7");
        let rows = file["source_enums"]
            .as_array()
            .expect("real source enum carrier");
        assert_eq!(rows.len(), 2);
        let target = rows
            .iter()
            .position(|row| {
                row["declaration"]["qualified_declaration"] == serde_json::json!(["Status"])
            })
            .expect("real Status declaration");
        let other = rows
            .iter()
            .position(|row| {
                row["declaration"]["qualified_declaration"] == serde_json::json!(["Level"])
            })
            .expect("another actual admitted enum key");
        let target_node = rows[target]["declaration"]["structural_node"].clone();
        for (field, replacement) in [
            ("owner", rows[other]["declaration"]["node_key"].clone()),
            ("case", serde_json::json!("FRESH_CASE_2765")),
        ] {
            let mut changed = file.clone();
            let row = &mut changed["source_enums"][target];
            row["declaration"]["shape"]["variants"][0][field] = replacement.clone();
            row["members"][0]["variant"][field] = replacement;
            let bytes = serde_json::to_vec(&changed).unwrap();
            let mut ledger = NativeJoinBudget::new(u64::MAX, u64::MAX);
            let reread = crate::ReadIdentityRows::read(&bytes, &fixture.admitted, &mut ledger)
                .expect("paired claims still satisfy the pinned IR structural join");
            let joined = reread
                .selected(&fixture.admitted, &mut ledger)
                .expect("the real native join accepts the paired shape/member mutation");
            let (index, row) = joined
                .source_enum_rows()
                .enumerate()
                .find(|(_, row)| {
                    serde_json::to_value(&row.declaration.structural_node).unwrap() == target_node
                })
                .expect("mutated original declaration remains joined");
            let variant = row.declaration.shape.variants().next().unwrap();
            assert_eq!(variant, &row.members[0].variant);
            match field {
                "owner" => assert_ne!(variant.owner(), &row.declaration.node_key),
                "case" => assert_ne!(variant.case(), row.members[0].case.as_ref()),
                _ => unreachable!(),
            }
            assert!(
                matches!(
                    SourceEnumCatalog::from_joined(&joined, &[], &mut ledger),
                    Err(SourceEnumCatalogError::InvalidJoinedInvariant {
                        row_kind: SourceEnumRowKind::Source, index: refused,
                    }) if refused == index
                ),
                "the catalog must refuse instead of substituting a different VariantId for {field}"
            );
            assert_eq!(
                ledger.first_unpaid(),
                None,
                "semantic inconsistency is not exhaustion"
            );
        }
    }

    #[trace("TC-1331", "FR-531-AC-42", "FR-531-AC-50")]
    #[test]
    fn source_enum_catalog_combines_actual_model_and_source_members_without_overwrite() {
        let fixture = fixture(
            &format!("{ENUMS}\nfunction tier using v(t: M::Tier): M::Tier pure {{ t }}"),
            true,
        );
        let mut budget = NativeJoinBudget::new(u64::MAX, u64::MAX);
        let sources = fixture
            .emission
            .selected_source_enum_rows(&mut budget)
            .unwrap();
        let models = fixture
            .emission
            .selected_ir_model_enum_shape_rows(&mut budget)
            .unwrap();
        let joined = SelectedExactIdentities::empty()
            .with_source_enum_rows(&fixture.admitted, sources.clone(), &mut budget)
            .unwrap()
            .with_admitted_model_enum_rows(
                &fixture.admitted,
                filament_contract_model::SelectedModelEnumRows {
                    declarations: fixture.emission.selected_model_enum_declaration_rows(),
                    members: fixture.emission.selected_model_enum_member_rows(),
                    shapes: &models,
                },
                &mut budget,
            )
            .unwrap();
        let catalog = SourceEnumCatalog::from_joined(&joined, &models, &mut budget).unwrap();
        assert!(!models.is_empty());
        for row in &models {
            let EnumScope::Model { name } =
                catalog.classify_shape(&row.shape, &mut budget).unwrap()
            else {
                panic!("actual model shape")
            };
            assert_eq!(name, row.original_identity.as_ref());
            for case in &row.cases {
                let member = catalog
                    .enum_members()
                    .resolve(case.variant.clone())
                    .unwrap();
                assert_eq!(member.member(), case.member_key);
                assert!(matches!(
                    catalog
                        .classify_variant(&case.variant, &mut budget)
                        .unwrap(),
                    EnumScope::Model { .. }
                ));
            }
        }
        for row in &sources {
            for member in &row.members {
                assert!(catalog
                    .enum_members()
                    .resolve(member.variant.clone())
                    .is_some());
                assert!(matches!(
                    catalog
                        .classify_variant(&member.variant, &mut budget)
                        .unwrap(),
                    EnumScope::NamedSource { .. }
                ));
            }
        }
    }

    #[trace("TC-1331", "FR-531-AC-44")]
    #[test]
    fn source_enum_catalog_native_allowances_keep_one_cumulative_prefix_and_first_denial() {
        let fixture = fixture(ENUMS, false);
        let mut measured = NativeJoinBudget::new(u64::MAX, u64::MAX);
        let catalog = build(&fixture, &mut measured).unwrap();
        assert!(
            !catalog.0.sources.is_empty(),
            "the measured catalog contains actual source enum rows"
        );
        let spent = measured.consumed();
        assert!(spent.work > 0, "source enum rows consume work");
        assert!(spent.storage > 0, "source enum rows consume storage");
        let mut exact = NativeJoinBudget::new(spent.work, spent.storage);
        let admitted = build(&fixture, &mut exact).unwrap();
        assert!(!admitted.0.sources.is_empty());
        assert_eq!(exact.consumed(), spent);
        for (work_limit, storage_limit, kind) in [
            (0, spent.storage, NativeJoinChargeKind::Work),
            (spent.work, 0, NativeJoinChargeKind::Storage),
            (spent.work - 1, spent.storage, NativeJoinChargeKind::Work),
            (spent.work, spent.storage - 1, NativeJoinChargeKind::Storage),
        ] {
            let mut budget = NativeJoinBudget::new(work_limit, storage_limit);
            let error = build(&fixture, &mut budget).unwrap_err();
            let denied = budget.first_unpaid().expect("actual owner denial");
            assert_eq!(denied.kind, kind);
            assert!(denied.amount > 0);
            assert!(budget.first_unpaid_locus().is_some());
            match error {
                BuildStop::Adapter(error)
                | BuildStop::Catalog(SourceEnumCatalogError::Native(error)) => {
                    assert_eq!(error.denied(), Some(denied));
                    assert_eq!(error.locus(), budget.first_unpaid_locus());
                    assert_eq!(
                        error.configured_bound(),
                        Some(if kind == NativeJoinChargeKind::Work {
                            work_limit
                        } else {
                            storage_limit
                        })
                    );
                }
                BuildStop::Join(error) => assert!(
                    fsl_semantics::model::intake::is_native_budget_denial(&error)
                ),
                other => panic!("must retain native owner denial: {other:?}"),
            }
            if work_limit != 0 && storage_limit != 0 {
                assert!(budget.consumed().work > 0);
            }
        }
    }

    #[trace("TC-1331", "FR-531-AC-30", "FR-531-AC-44")]
    #[test]
    fn source_enum_catalog_charges_already_joined_source_rows() {
        let fixture = fixture(ENUMS, false);
        let mut setup = NativeJoinBudget::new(u64::MAX, u64::MAX);
        let source_rows = fixture
            .emission
            .selected_source_enum_rows(&mut setup)
            .expect("source rows materialize");
        let joined = SelectedExactIdentities::empty()
            .with_source_enum_rows(&fixture.admitted, source_rows, &mut setup)
            .expect("actual source enum rows join");

        let mut measured = NativeJoinBudget::new(u64::MAX, u64::MAX);
        let catalog = SourceEnumCatalog::from_joined(&joined, &[], &mut measured)
            .expect("catalog materializes the already-joined rows");
        assert!(!catalog.0.sources.is_empty());
        let last_source_row = catalog.0.sources.len() - 1;
        let need = measured.consumed();
        assert!(need.work > 0, "catalog materialization consumes work");
        assert!(need.storage > 0, "catalog materialization owns storage");

        let mut exact = NativeJoinBudget::new(need.work, need.storage);
        SourceEnumCatalog::from_joined(&joined, &[], &mut exact)
            .expect("exact catalog-only budget admits");
        assert_eq!(exact.consumed(), need);

        for (work_limit, storage_limit, kind) in [
            (need.work - 1, need.storage, NativeJoinChargeKind::Work),
            (need.work, need.storage - 1, NativeJoinChargeKind::Storage),
        ] {
            let mut short = NativeJoinBudget::new(work_limit, storage_limit);
            let error = SourceEnumCatalog::from_joined(&joined, &[], &mut short)
                .expect_err("one-short catalog-only budget refuses");
            let SourceEnumCatalogError::Native(error) = error else {
                panic!("budget denial retains its native owner error")
            };
            let denied = short
                .first_unpaid()
                .expect("first unpaid source-row charge");
            let at = short
                .first_unpaid_locus()
                .expect("first unpaid source-row locus");
            assert_eq!(denied.kind, kind);
            assert_eq!(error.denied(), Some(denied));
            assert_eq!(error.locus(), Some(at));
            let NativeJoinLocusSource::AdapterRow(index) = at.source else {
                panic!("catalog denial retains its source-row index: {at:?}")
            };
            assert_eq!(
                index, last_source_row,
                "the last source row owns the denial"
            );
            assert_eq!(at.form, "source_enum_catalog_members");
        }
    }

    #[trace("TC-1331", "FR-531-AC-44")]
    #[test]
    fn source_enum_catalog_lookup_zero_denial_precedes_inspection_and_stays_original() {
        let fixture = fixture(ENUMS, false);
        let mut preparation = NativeJoinBudget::new(u64::MAX, u64::MAX);
        let catalog = build(&fixture, &mut preparation).unwrap();
        let row = &catalog.0.sources[0];
        let mut budget = NativeJoinBudget::new(0, u64::MAX);
        let error = catalog
            .classify_shape(&row.declaration.shape, &mut budget)
            .unwrap_err();
        let SourceEnumCatalogError::Native(original) = error else {
            panic!("native lookup denial")
        };
        let denied = original.denied().unwrap();
        assert_eq!(denied.kind, NativeJoinChargeKind::Work);
        assert_eq!(denied.amount, 1);
        assert_eq!(original.configured_bound(), Some(0));
        let SourceEnumCatalogError::Native(again) = catalog
            .classify_variant(&row.members[0].variant, &mut budget)
            .unwrap_err()
        else {
            panic!("prior denial stays first")
        };
        assert_eq!(original, again);
        assert_eq!(budget.consumed().work, 0);
        assert_eq!(budget.consumed().storage, 0);
    }

    #[trace("TC-1331", "FR-531-AC-44")]
    #[test]
    fn source_enum_catalog_construction_denial_precedes_any_catalog_allocation() {
        let fixture = fixture(ENUMS, false);
        let mut preparation = NativeJoinBudget::new(u64::MAX, u64::MAX);
        let rows = fixture
            .emission
            .selected_source_enum_rows(&mut preparation)
            .unwrap();
        let joined = SelectedExactIdentities::empty()
            .with_source_enum_rows(&fixture.admitted, rows, &mut preparation)
            .unwrap();
        for (work_limit, storage_limit, kind) in [
            (0, u64::MAX, NativeJoinChargeKind::Work),
            (u64::MAX, 0, NativeJoinChargeKind::Storage),
        ] {
            let mut budget = NativeJoinBudget::new(work_limit, storage_limit);
            let mut refusal = None;
            let allocation = allocation_counter::measure(|| {
                refusal =
                    Some(SourceEnumCatalog::from_joined(&joined, &[], &mut budget).unwrap_err());
            });
            assert_eq!(
                allocation.count_total, 0,
                "unpaid catalog construction must allocate nothing"
            );
            let Some(SourceEnumCatalogError::Native(error)) = refusal else {
                panic!("original native denial")
            };
            assert_eq!(error.denied().unwrap().kind, kind);
            assert_eq!(error.denied().unwrap().amount, 1);
            assert_eq!(error.configured_bound(), Some(0));
            assert_eq!(error.locus(), Some(locus(0, "source_enum_catalog")));
        }
    }

    #[trace("TC-1331", "FR-531-AC-50", "FR-531-AC-44")]
    #[test]
    fn source_enum_catalog_generated_scope_keeps_genuine_context_after_catalog_drop() {
        let fixture = fixture(
            "type Lengths = Int[0,1];\nsynthesis Reach using v grammar M::Account domain Lengths minimize Reach.length satisfies { true };\n",
            true,
        );
        let mut budget = NativeJoinBudget::new(u64::MAX, u64::MAX);
        let rows = fixture
            .emission
            .selected_source_enum_rows(&mut budget)
            .unwrap();
        let generated = rows
            .iter()
            .find(|row| row.declaration.qualified_declaration.len() > 1)
            .expect("the real plan synthesis emits its qualified operation enum");
        let operation_enum = &fixture
            .checked
            .graph()
            .syntheses()
            .iter()
            .find_map(|synthesis| synthesis.plan())
            .expect("the checked source has its real plan synthesis")
            .shape()
            .operation_enum;
        assert_eq!(operation_enum.name, "M::Op");
        assert_eq!(
            operation_enum.declaration.key(),
            generated.declaration.node_key
        );
        assert_eq!(
            generated
                .declaration
                .qualified_declaration
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>(),
            ["Reach", "M", "Op"]
        );
        let joined = SelectedExactIdentities::empty()
            .with_source_enum_rows(&fixture.admitted, rows.clone(), &mut budget)
            .unwrap();
        let catalog = SourceEnumCatalog::from_joined(&joined, &[], &mut budget).unwrap();
        let EnumScope::UnsupportedSourceScope {
            declaration,
            qualified_declaration,
            index,
        } = catalog
            .classify_shape(&generated.declaration.shape, &mut budget)
            .unwrap()
        else {
            panic!("a generated qualified enum must not acquire a fabricated Forms name")
        };
        assert_eq!(declaration, &generated.declaration.structural_node);
        assert_eq!(
            qualified_declaration,
            generated.declaration.qualified_declaration.as_slice()
        );
        let strong_count = Arc::strong_count(&catalog.0);
        let mut no_work = NativeJoinBudget::new(0, u64::MAX);
        assert!(matches!(
            catalog.unsupported_scope_refusal(index, &mut no_work),
            SourceEnumCatalogError::Native(_)
        ));
        assert_eq!(Arc::strong_count(&catalog.0), strong_count);
        let mut no_storage = NativeJoinBudget::new(u64::MAX, 0);
        let SourceEnumCatalogError::Native(error) =
            catalog.unsupported_scope_refusal(index, &mut no_storage)
        else {
            panic!("retaining an unsupported scope handle requires original storage permission")
        };
        assert_eq!(error.denied().unwrap().kind, NativeJoinChargeKind::Storage);
        assert_eq!(error.denied().unwrap().amount, 1);
        assert_eq!(Arc::strong_count(&catalog.0), strong_count);
        let refusal = catalog.unsupported_scope_refusal(index, &mut budget);
        assert_eq!(Arc::strong_count(&catalog.0), strong_count + 1);
        let second = catalog.unsupported_scope_refusal(index, &mut budget);
        assert_eq!(
            refusal, second,
            "diagnostic equality compares actual retained context"
        );
        drop(second);
        drop(catalog);
        let SourceEnumCatalogError::UnsupportedSourceScope(context) = refusal else {
            panic!("typed unsupported source scope")
        };
        assert_eq!(
            context.declaration(),
            &generated.declaration.structural_node
        );
        assert_eq!(
            context.qualified_declaration(),
            generated.declaration.qualified_declaration.as_slice()
        );
    }

    #[trace("TC-1331", "FR-531-AC-44")]
    #[test]
    fn source_enum_catalog_nested_identity_scratch_denies_before_allocating() {
        let child = NodeKey::new(
            "pkg",
            vec![filament_exact::PathSegment::Text("decl".into())],
            0,
        )
        .unwrap();
        let key = NodeKey::new("pkg", vec![filament_exact::PathSegment::Node(child)], 0).unwrap();
        let mut budget = NativeJoinBudget::new(u64::MAX, 0);
        let error = node_equal(
            &key,
            &key,
            &mut budget,
            locus(7, "source_enum_nested_identity"),
        )
        .unwrap_err();
        let SourceEnumCatalogError::Native(error) = error else {
            panic!("scratch storage must be paid")
        };
        assert_eq!(error.denied().unwrap().kind, NativeJoinChargeKind::Storage);
        assert_eq!(error.denied().unwrap().amount, 2);
        assert_eq!(error.locus(), Some(locus(7, "source_enum_nested_identity")));
        assert_eq!(budget.consumed().storage, 0);
        let mut exact = NativeJoinBudget::new(u64::MAX, 2);
        assert!(node_equal(
            &key,
            &key,
            &mut exact,
            locus(7, "source_enum_nested_identity")
        )
        .unwrap());
    }
}
