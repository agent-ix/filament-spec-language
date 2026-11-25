// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531: the evaluation entry over a checked package read from its bytes
//! alone.
//!
//! A [`BarePackage`] borrows the [`CheckedPackageV2`] that Contract IR's
//! `read_checked_package` returned. It has no other constructor, so no value
//! of the source side (`CheckedPackage`, `Emission`, `AdmittedPackageRead`)
//! can reach the evaluation code that takes it. The identity rows a protocol
//! slot call needs come from the handoff directory's `identities.json`
//! through [`ReadIdentityRows`], and the dependency closure from the direct
//! dependencies the reader retained in the package
//! ([`BarePackage::dependencies`]).

use std::collections::{BTreeMap, BTreeSet};

use filament_contract_model::{
    CheckedNodeId, CheckedPackageV2, ExactTypeProjection, ExactTypeProjectionError,
    NativeJoinBudget, RequirementItemRow, SelectedExactDeclarationForm,
    SelectedExactDeclarationRow, SelectedExactFieldRow, SelectedExactIdentities,
    SelectedExactIdentity, SelectedSourceObjectRow, SelectedSourceObjects,
    SelectedSourceOperationRow, SelectedSourceOperations, SelectedSourceRecordRow,
    SelectedSourceUnionRow,
};
use filament_exact::{
    EffectiveId, MemberId, MemberOwner, NodeKey, Origin, OriginalDeclarationKey, Role, UnitId,
};
use fsl_foundation::{source::provenance::OccurrenceKey, RequestIndex};
use serde::{Deserialize, Serialize};

use crate::checked_v2::wire_graph;

mod carried;
mod model_enum;
mod model_tuple;
mod model_union;
mod observation;
mod source_enum;
#[cfg(test)]
mod world_agreement_tests;
use crate::equality_file::OriginalValueType;
use carried::{SourceRecordJson, SourceTupleJson, SourceUnionJson};
use filament_exact::Presence;
use source_enum::SourceEnumJson;

/// The format of `identities.json` (FR-531 "The identity file").
pub const IDENTITIES_FORMAT: &str = "filament.checked-handoff.identities/7";

/// A checked package as a bytes-only consumer holds it (FR-531).
///
/// `BarePackage::new` is the one constructor (`with_canonical_bytes` attaches
/// the bytes the package was already read from); nothing builds one from bytes,
/// from a decoded graph or from a term (FR-531-AC-2):
///
/// ```compile_fail,E0599
/// let _ = fsl_package::BarePackage::from_bytes(b"{}");
/// ```
///
/// ```compile_fail,E0599
/// fn forge(graph: filament_contract_model::CheckedPackageV2) {
///     let _ = fsl_package::BarePackage::from_graph(&graph);
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn forge(term: &filament_contract_model::CheckedTerm) {
///     let _ = fsl_package::BarePackage::from_term(term);
/// }
/// ```
///
/// The package joins its supply's identity rows once, at
/// [`Self::with_supply`], under the caller's one native budget (FR-531,
/// FR-358). It owns the joined identities and every later selection,
/// world and projection call borrows them, so it is not `Copy` and not
/// `Clone`: an unbudgeted copy of the joined identities cannot be made.
#[derive(Debug)]
pub struct BarePackage<'p> {
    dependency_views: Vec<BarePackage<'p>>,
    package: &'p CheckedPackageV2,
    /// The exact bytes that admitted `package`, when this value was built
    /// from a reader-backed handoff. A bare package made directly from a
    /// checked object has no byte witness.
    canonical_bytes: Option<&'p [u8]>,
    /// The package's identity, its admitted `name()` (FR-940 `identity`).
    identity: fsl_semantics::library::LibraryName,
    supply: Option<BareSupply<'p>>,
    /// The supply's join, held with the witness defect it found: a missing,
    /// foreign or unknown witness is refused by the stage that reads the
    /// identities, not when the supply is attached (FR-390).
    joined: Option<Result<SelectedExactIdentities, IdentityWitnessRefusal>>,
    projections: BTreeMap<CheckedNodeId, Result<ExactTypeProjection, ExactTypeProjectionError>>,
    equality: crate::reader_protocol_projection::EqualityDecisions,
    /// The source-owned units' definitions the supply's equality rows select
    /// (FR-533), empty when the rows select none.
    pub(crate) units: crate::bare_units::UnitDefinitions,
    /// The identity an observation this package admits carries (FR-533
    /// "Observation admission"); one per value, never copied.
    owner: std::sync::Arc<()>,
    /// The selected object declarations the bare environment declares, or
    /// the refusal of their projection, retained for admission (FR-533).
    objects: Result<Vec<filament_semantic_value::declaration::ObjectTypeDeclaration>, ()>,
    pub(crate) source_enums: Option<crate::SourceEnumCatalog>,
    /// The population catalog: the declaration and member types of each
    /// selected population node, or the refusal of the read.
    populations: Result<BTreeMap<NodeKey, PopulationTypes>, ()>,
}

/// The declaration and member types of one selected population.
pub(crate) type PopulationTypes = (filament_checked::key::DeclarationKey, Vec<EffectiveId>);

/// What a protocol-slot call reads beside the package (FR-531).
#[derive(Clone, Copy, Debug)]
pub struct BareSupply<'p> {
    /// The selected model identity rows of the package.
    pub identities: &'p ReadIdentityRows,
    /// The producer's equality rows, read from `equality.json`; `None` for
    /// a package whose slots hold no equality application (FR-531).
    pub equality: Option<&'p crate::equality_file::ReadEqualityRows>,
}

impl<'p> BarePackage<'p> {
    /// The bare package over `package`, with no supply.
    pub fn new(package: &'p CheckedPackageV2) -> Self {
        Self {
            dependency_views: Vec::new(),
            identity: fsl_semantics::library::LibraryName::of_package(package),
            package,
            canonical_bytes: None,
            supply: None,
            joined: None,
            projections: BTreeMap::new(),
            equality: BTreeMap::new(),
            units: crate::bare_units::UnitDefinitions::default(),
            owner: std::sync::Arc::new(()),
            objects: Ok(Vec::new()),
            source_enums: None,
            populations: Ok(BTreeMap::new()),
        }
    }

    /// Attach the retained closure, joining each library's own witnesses once
    /// under the invocation's shared budget. The views are flat: a diamond
    /// shares its one admitted artifact rather than copying subtrees.
    ///
    /// # Errors
    /// Returns the original native join denial without a partially joined view.
    pub fn with_dependency_artifacts(
        mut self,
        artifacts: &'p [crate::DependencyArtifact],
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, crate::NativeJoinFailure> {
        for artifact in artifacts {
            self.dependency_views
                .push(Self::new(&artifact.package).with_supply(
                    BareSupply {
                        identities: &artifact.identities,
                        equality: Some(&artifact.equality),
                    },
                    budget,
                )?);
        }
        Ok(self)
    }

    /// Authenticated package views of the unique library closure.
    pub fn dependency_views(&self) -> &[BarePackage<'p>] {
        &self.dependency_views
    }

    /// This package with the exact canonical bytes it was read from.
    ///
    /// The caller passes the bytes the strict reader admitted `package` from
    /// (a reader-backed value has them from
    /// `AdmittedPackageRead::canonical_bytes`). FR-531-AC-2 forbids a
    /// `BarePackage` signature that names `AdmittedPackageRead`, so this
    /// takes the slice, not the read.
    ///
    /// The bytes are not trusted blindly: they attach only when their own
    /// `name` and `contract_version` members are this package's. The package
    /// holds no content digest to compare, so that is the whole check; bytes
    /// that name another package stay unattached and the owner call refuses
    /// for want of package bytes.
    #[must_use]
    pub fn with_canonical_bytes(mut self, bytes: &'p [u8]) -> Self {
        #[derive(serde::Deserialize)]
        struct Head<'a> {
            #[serde(borrow, default)]
            name: Option<&'a str>,
            #[serde(borrow, default)]
            contract_version: Option<&'a str>,
        }
        let names_this_package = serde_json::from_slice::<Head<'_>>(bytes).is_ok_and(|head| {
            head.name == Some(self.package.name())
                && head.contract_version == Some(crate::CHECKED_PACKAGE_V2)
        });
        if names_this_package {
            self.canonical_bytes = Some(bytes);
        }
        self
    }

    /// Whether the package holds a unit node declared under the name `unit`.
    /// A unit the package declares but `time_unit_scale` cannot read here has a
    /// real factor this route lacks, which a caller must not replace by a count.
    #[must_use]
    pub fn declares_unit(&self, unit: &str) -> bool {
        self.package.resolved_graph().nodes.iter().any(|node| {
            node.node_tag.as_ref() == "scalar_type"
                && node.semantic_form.as_ref() == "unit"
                && node.declaration.as_ref().is_some_and(|declaration| {
                    declaration
                        .qualified_name
                        .last()
                        .is_some_and(|last| last.as_ref() == unit)
                })
        })
    }

    /// The exact scale of the time unit `unit` to seconds, read from the
    /// package's own source-owned unit definitions, when it holds one: a
    /// declared unit of the `Time` dimension named `unit` with no offset.
    #[must_use]
    pub fn time_unit_scale(&self, unit: &str) -> Option<filament_exact::Rational> {
        self.units.time_unit_scale(unit)
    }

    /// The exact canonical bytes admitted for this package, when available.
    /// A value constructed with [`Self::new`] has no retained input bytes.
    pub fn canonical_bytes(&self) -> Option<&'p [u8]> {
        self.canonical_bytes
    }

    /// The admitted package's identity (FR-940 `identity`).
    pub(crate) fn admitted_identity(&self) -> &fsl_semantics::library::LibraryName {
        &self.identity
    }

    /// The identity of this value as an observation owner.
    pub(crate) fn owner(&self) -> std::sync::Arc<()> {
        std::sync::Arc::clone(&self.owner)
    }

    /// The model identities the package selects, ascending (FR-533).
    pub fn model_selections(&self) -> Vec<fsl_semantics::model::domain_package::DomainPackageRef> {
        self.package
            .selected_model_packages()
            .map(
                |identity| fsl_semantics::model::domain_package::DomainPackageRef {
                    identity: identity.to_owned(),
                },
            )
            .collect()
    }

    /// The declaration and member types of the selected population `node`,
    /// from the package's population catalog; `None` for a node no selected population has.
    pub fn population_types(
        &self,
        node: &NodeKey,
    ) -> Option<(&filament_checked::key::DeclarationKey, &[EffectiveId])> {
        let (declaration, members) = self.populations.as_ref().ok()?.get(node)?;
        Some((declaration, members.as_slice()))
    }

    /// The identity of the model selection whose population has `object` as a
    /// member type: the selection that owns the type, which a declaration of
    /// the model is named by on the package route (FR-940). `Ok(None)` when no
    /// selected population ranges over it.
    ///
    /// # Errors
    ///
    /// An internal fault when populations of two selections range over it: the
    /// package records no alias to say which selection the declaration is on,
    /// and a type is declared by one package, so no admitted package does this.
    pub fn selection_of_object(
        &self,
        object: &EffectiveId,
    ) -> Result<Option<&str>, filament_semantic_value::catalog::InternalFault> {
        let Ok(populations) = self.populations.as_ref() else {
            return Ok(None);
        };
        let mut owners = populations
            .values()
            .filter(|(_, members)| members.contains(object))
            .map(|(declaration, _)| declaration.package.as_str());
        let first = owners.next();
        if owners.any(|owner| Some(owner) != first) {
            return Err(filament_semantic_value::catalog::InternalFault::new(
                "FR-940",
                "terminal-object-in-two-selections",
            ));
        }
        Ok(first)
    }

    /// FR-124: the canonical member type of each population `package`
    /// declares (the least member by declaration key), once each, ascending.
    /// Empty when the population catalog was refused.
    pub(crate) fn population_canonical_members(&self, package: &str) -> Vec<EffectiveId> {
        let Ok(populations) = self.populations.as_ref() else {
            return Vec::new();
        };
        let canonical: std::collections::BTreeSet<&EffectiveId> = populations
            .values()
            .filter(|(declaration, _)| declaration.package == package)
            .filter_map(|(_, members)| {
                members
                    .iter()
                    .min_by_key(|member| (member.original().package(), member.original().node()))
            })
            .collect();
        canonical.into_iter().cloned().collect()
    }

    /// The selected object declarations, or `Err` when their projection or
    /// the catalog was refused (FR-533 "Observation admission", step 3).
    pub(crate) fn object_declarations(
        &self,
    ) -> Result<&[filament_semantic_value::declaration::ObjectTypeDeclaration], ()> {
        self.populations.as_ref().map_err(|_| ())?;
        self.objects.as_deref().map_err(|_| ())
    }

    /// `identities` extended with the source-owned units the equality rows
    /// select (FR-533): IR joins the rows, then the units' definitions are
    /// read from the package's nodes. Rows IR does not join, or definitions
    /// that are no unit graph, are an unknown identity witness; an exhausted
    /// native allowance is the caller's, never a defect of the rows.
    fn join_source_units(
        &self,
        identities: SelectedExactIdentities,
        equality: Option<&crate::equality_file::ReadEqualityRows>,
        budget: &mut NativeJoinBudget,
    ) -> Result<
        (
            Result<SelectedExactIdentities, IdentityWitnessRefusal>,
            crate::bare_units::UnitDefinitions,
        ),
        crate::NativeJoinFailure,
    > {
        let none = crate::bare_units::UnitDefinitions::default();
        // Another package's rows are the equality stage's refusal to report.
        let Some(equality) = equality.filter(|equality| {
            equality.package_name() == self.package.name() && !equality.source_units().is_empty()
        }) else {
            return Ok((Ok(identities), none));
        };
        let extended = match identities.with_admitted_type_rows(
            self.package,
            &[],
            equality.source_units(),
            budget,
        ) {
            Ok(extended) => extended,
            Err(denial) if fsl_semantics::model::intake::is_native_budget_denial(&denial) => {
                return Err(crate::NativeJoinFailure::from(denial).with_budget(budget));
            }
            Err(_) => return Ok((Err(IdentityWitnessRefusal::Unknown), none)),
        };
        match crate::bare_units::read_unit_table(self.package, equality.source_units(), budget) {
            Ok(units) => Ok((Ok(extended), units)),
            Err(crate::bare_units::UnitDefinitionRefusal::Read(
                filament_contract_model::nominal_reads::NominalReadCode::LimitReached,
            )) => {
                // A native read that reports its allowance exhausted records the
                // unpaid charge; without one the denial is no allowance of this
                // pass, and the witness is refused rather than given a setting.
                let setting = match budget.first_unpaid().map(|charge| charge.kind) {
                    Some(filament_contract_model::NativeJoinChargeKind::Work) => {
                        fsl_foundation::Setting::I2WorkUnits
                    }
                    Some(filament_contract_model::NativeJoinChargeKind::Storage) => {
                        fsl_foundation::Setting::I2NativeJoinStorage
                    }
                    None => return Ok((Err(IdentityWitnessRefusal::Unknown), none)),
                };
                Err(crate::NativeJoinFailure::UnitDefinitions { setting }.with_budget(budget))
            }
            Err(_) => Ok((Err(IdentityWitnessRefusal::Unknown), none)),
        }
    }

    /// The bare package with `supply`, its identity rows joined once and
    /// charged to `budget`, the caller's one finite native work and storage
    /// allowance for the pass (FR-531, FR-358). A missing, foreign or
    /// structurally unknown witness is not refused here: it stays deferred to
    /// the stage that reads the identities, in FR-390's order.
    ///
    /// # Errors
    ///
    /// [`crate::NativeJoinFailure`] retains IR's `ExhaustedWork` or
    /// `ExhaustedStorage` for identity/type projection, or its `WorkLimit` or
    /// `StorageLimit` for equality admission. No package is returned when
    /// `budget` runs out; the denial is never an unknown witness, a fault or
    /// an empty identity set. Resource denials carry a
    /// [`crate::NativeJoinFailure::Limit`] diagnostic with the original owner
    /// error, caller-configured ceiling, setting and cumulative attempted charge.
    pub fn with_supply(
        mut self,
        supply: BareSupply<'p>,
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, crate::NativeJoinFailure> {
        self.joined = match supply.identities.selected(self.package, budget) {
            Ok(identities) => {
                let (joined, units) =
                    self.join_source_units(identities, supply.equality, budget)?;
                self.units = units;
                Some(joined)
            }
            Err(IdentityJoinFailure::Witness(defect)) => Some(Err(defect)),
            Err(IdentityJoinFailure::Exhausted(denial)) => {
                return Err(crate::NativeJoinFailure::from(*denial).with_budget(budget));
            }
            Err(IdentityJoinFailure::Rows(denial)) => {
                return Err(crate::NativeJoinFailure::Rows(denial).with_budget(budget));
            }
        };
        self.supply = Some(supply);
        if let Some(Ok(identities)) = &self.joined {
            for node in &self.package.resolved_graph().nodes {
                let projection = self
                    .package
                    .project_exact_type(&node.node_id, identities, budget);
                if let Err(denial) = &projection {
                    if fsl_semantics::model::intake::is_native_budget_denial(denial) {
                        return Err(
                            crate::NativeJoinFailure::from(denial.clone()).with_budget(budget)
                        );
                    }
                }
                self.projections.insert(node.node_id.clone(), projection);
            }
        }
        if self.identities().is_some() {
            self.equality = crate::reader_protocol_projection::admit_equality_decisions(
                self.package,
                crate::reader_protocol_projection::ProjectionSource::Bytes {
                    equality: self.supply.as_ref().and_then(|supply| supply.equality),
                },
                budget,
            )
            .map_err(|failure| failure.with_budget(budget))?;
            self.read_selected_objects(budget)?;
            let identities = self
                .identities()
                .ok_or(crate::SourceEnumCatalogError::MissingIdentityWitness)
                .map_err(crate::NativeJoinFailure::from)?;
            self.source_enums = Some(
                crate::SourceEnumCatalog::from_joined(
                    identities,
                    &supply.identities.model_enum_shapes,
                    budget,
                )
                .map_err(|error| crate::NativeJoinFailure::from(error).with_budget(budget))?,
            );
            self.read_population_catalog(budget)?;
        }
        Ok(self)
    }

    /// The selected objects, projected once under `budget` (FR-533
    /// "Observation admission"). A budget denial is the caller's; any other
    /// refusal is retained for admission.
    fn read_selected_objects(
        &mut self,
        budget: &mut NativeJoinBudget,
    ) -> Result<(), crate::NativeJoinFailure> {
        let Some(Ok(identities)) = &self.joined else {
            return Ok(());
        };
        let source_units = self
            .supply
            .as_ref()
            .and_then(|supply| supply.equality)
            .map(|equality| equality.source_units())
            .unwrap_or(&[]);
        match self
            .package
            .project_selected_type_declarations(identities, &[], source_units, budget)
        {
            Ok(declarations) => {
                self.objects = Ok(declarations
                    .objects()
                    .iter()
                    .map(|object| {
                        let attributes = object
                            .declared_fields()
                            .iter()
                            .map(|field| {
                                filament_semantic_value::declaration::FieldDeclaration::new(
                                    field.name(),
                                    field.value_type().clone(),
                                    field.presence(),
                                )
                            })
                            .collect();
                        filament_semantic_value::declaration::ObjectTypeDeclaration::new(
                            object.effective_id().clone(),
                            object.original_identity(),
                            attributes,
                        )
                        .with_supertypes(object.supertypes().to_vec())
                    })
                    .collect());
                Ok(())
            }
            Err(refusal) => {
                let refusal = filament_contract_model::ExactTypeProjectionError {
                    code: refusal.code,
                    requested: None,
                    at: refusal.at,
                    form: refusal.form,
                    work: refusal.work,
                    denied: refusal.denied,
                    configured_bound: refusal.denied.map(|charge| match charge.kind {
                        filament_contract_model::NativeJoinChargeKind::Work => budget.work_limit(),
                        filament_contract_model::NativeJoinChargeKind::Storage => {
                            budget.storage_limit()
                        }
                    }),
                };
                if fsl_semantics::model::intake::is_native_budget_denial(&refusal) {
                    return Err(crate::NativeJoinFailure::from(refusal).with_budget(budget));
                }
                self.objects = Err(());
                Ok(())
            }
        }
    }

    /// The population catalog, read once with the budget's unconsumed work
    /// as its work allowance and its unconsumed storage as its entries
    /// allowance; its charges are not added to the budget (FR-533).
    fn read_population_catalog(
        &mut self,
        budget: &NativeJoinBudget,
    ) -> Result<(), crate::NativeJoinFailure> {
        use filament_contract_model::{
            CheckedModelPopulationCatalogError as CatalogError,
            CheckedModelPopulationCatalogLimitKind as Kind,
            CheckedModelPopulationCatalogLimits as Limits,
        };
        let limits = Limits {
            work: budget.work_limit().saturating_sub(budget.consumed().work),
            entries: budget
                .storage_limit()
                .saturating_sub(budget.consumed().storage),
        };
        let catalog = match self.package.selected_model_population_catalog(limits) {
            Ok(catalog) => catalog,
            Err(CatalogError::Limit {
                kind,
                limit,
                consumed,
                ..
            }) => {
                let setting = match kind {
                    Kind::Work => fsl_foundation::Setting::I2WorkUnits,
                    Kind::Entries => fsl_foundation::Setting::I2NativeJoinStorage,
                };
                return Err(crate::NativeJoinFailure::PopulationCatalog {
                    setting,
                    limit: fsl_foundation::diagnostic::LimitExceeded::new(
                        setting,
                        limit,
                        u128::from(consumed),
                    ),
                });
            }
            Err(_) => {
                self.populations = Err(());
                return Ok(());
            }
        };
        let mut populations = BTreeMap::new();
        for root in catalog.roots() {
            for population in root.populations() {
                let declaration = filament_checked::key::DeclarationKey {
                    package: root.model_identity().to_owned(),
                    node: population.original_identity().to_owned(),
                };
                let Ok(node) = fsl_semantics::model::key::population_node_key(&declaration) else {
                    self.populations = Err(());
                    return Ok(());
                };
                let members = population
                    .members()
                    .iter()
                    .map(|member| member.normalized_type_identity().clone())
                    .collect();
                populations.insert(node, (declaration, members));
            }
        }
        self.populations = Ok(populations);
        Ok(())
    }

    pub(crate) fn equality_decisions(
        &self,
    ) -> &crate::reader_protocol_projection::EqualityDecisions {
        &self.equality
    }

    /// Owner-projected Exact types retained under the supply entry's native budget.
    /// Unsupported types retain their original owner refusal for the reading stage.
    pub fn exact_type_projections(
        &self,
    ) -> &BTreeMap<CheckedNodeId, Result<ExactTypeProjection, ExactTypeProjectionError>> {
        &self.projections
    }

    /// The supply with the identities it joined, or the witness refusal
    /// FR-531-AC-5 names.
    pub(crate) fn witness(
        &self,
    ) -> Result<(&BareSupply<'p>, &SelectedExactIdentities), crate::BareSelectionFailure> {
        let supply = self
            .supply
            .as_ref()
            .ok_or(crate::BareSelectionFailure::Identity(
                IdentityWitnessRefusal::Missing,
            ))?;
        match &self.joined {
            Some(Ok(identities)) => Ok((supply, identities)),
            Some(Err(defect)) => Err(crate::BareSelectionFailure::Identity(*defect)),
            None => Err(crate::BareSelectionFailure::Identity(
                IdentityWitnessRefusal::Missing,
            )),
        }
    }

    /// The joined identities, when the supply's witness was admitted.
    pub fn identities(&self) -> Option<&SelectedExactIdentities> {
        self.joined.as_ref().and_then(|joined| joined.as_ref().ok())
    }

    /// The package.
    pub fn package(&self) -> &'p CheckedPackageV2 {
        self.package
    }

    /// The structural node key of the package node `node` (FR-322
    /// "Operand index segments"); `None` for a node the package does not
    /// hold or whose key does not rebuild.
    pub fn node_key(&self, node: &CheckedNodeId) -> Option<NodeKey> {
        wire_graph(self.package).ok()?.key(node).ok()
    }

    /// The requirement items owned by the unique function named `function`,
    /// in admitted package order, with each original request index (FR-531,
    /// FR-395). The rows and value-validity facts are projected from the
    /// admitted package; this does not reconstruct them from source.
    #[must_use]
    pub fn call_requirements(&self, function: &str) -> Vec<BareCallRequirement> {
        let mut owners = self
            .package
            .resolved_graph()
            .nodes
            .iter()
            .filter(|node| node.semantic_form.as_ref() == "pure_function")
            .filter(|node| {
                node.declaration.as_ref().is_some_and(|declaration| {
                    declaration
                        .qualified_name
                        .last()
                        .is_some_and(|name| name.as_ref() == function)
                        || declaration
                            .qualified_name
                            .iter()
                            .map(AsRef::as_ref)
                            .collect::<Vec<&str>>()
                            .join("::")
                            == function
                })
            })
            .map(|node| &node.node_id);
        let Some(owner) = owners.next() else {
            return Vec::new();
        };
        if owners.next().is_some() {
            return Vec::new();
        }

        self.package
            .requirement_items()
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                row.record.as_ref().is_some_and(|record| match record {
                    filament_contract_model::RequirementRecord::ValueValidity(record) => {
                        &record.owner == owner
                    }
                    filament_contract_model::RequirementRecord::OperationContract(_) => false,
                })
            })
            .filter_map(|(index, row)| {
                let node = self.node_key(&row.claim)?;
                let role = checked_occurrence_role(&row.occurrence.role)?;
                Some(BareCallRequirement {
                    occurrence: OccurrenceKey::new(
                        node,
                        Origin::new(Role::new(role), row.occurrence.ordinal),
                    ),
                    request_index: RequestIndex::new(index),
                    item: row.clone(),
                })
            })
            .collect()
    }

    /// The supply, when one was given.
    pub fn supply(&self) -> Option<&BareSupply<'p>> {
        self.supply.as_ref()
    }

    /// The package's dependency closure, read from the direct dependencies
    /// each package retains from its read evidence (IR-1325): every
    /// dependency once, nearest first, in lock-selection order. A package
    /// two parents retain is one retained value and is listed once; two
    /// distinct values that share a name are both listed. The root package is
    /// not listed.
    pub fn dependencies(&self) -> Vec<&'p CheckedPackageV2> {
        crate::analysis_view::closure(self.package)
    }
}

/// One admitted package requirement row selected for a bare function call
/// (FR-531, FR-395).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BareCallRequirement {
    occurrence: OccurrenceKey,
    request_index: RequestIndex,
    item: RequirementItemRow,
}

impl BareCallRequirement {
    /// The requirement row's structural occurrence key.
    #[must_use]
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// Its zero-based position in the admitted package requirement rows.
    #[must_use]
    pub fn request_index(&self) -> RequestIndex {
        self.request_index
    }

    /// The exact admitted row, including its owner, bound, conversion, guards,
    /// roots and objects when the row carries a value-validity record.
    #[must_use]
    pub fn item(&self) -> &RequirementItemRow {
        &self.item
    }
}

fn checked_occurrence_role(
    role: &filament_contract_model::CheckedOccurrenceRole,
) -> Option<&'static str> {
    use filament_contract_model::CheckedOccurrenceRole;
    Some(match role {
        CheckedOccurrenceRole::Declaration => "declaration",
        CheckedOccurrenceRole::Type => "type",
        CheckedOccurrenceRole::Expression => "expression",
        CheckedOccurrenceRole::Anchor => "anchor",
        CheckedOccurrenceRole::Claim => "claim",
        CheckedOccurrenceRole::Generated => "generated",
    })
}

/// The selected model identity rows of one package, read from its
/// `identities.json` (FR-390, FR-531).
#[derive(Debug, Eq, PartialEq)]
pub struct ReadIdentityRows {
    package: String,
    declarations: Vec<SelectedExactDeclarationRow>,
    fields: Vec<SelectedExactFieldRow>,
    source_records: Vec<SelectedSourceRecordRow>,
    source_tuples: Vec<filament_contract_model::SelectedSourceTupleRow>,
    source_unions: Vec<SelectedSourceUnionRow>,
    source_enums: Vec<filament_contract_model::SelectedSourceEnumRow>,
    source_objects: Vec<SelectedSourceObjectRow>,
    source_object_member_facts: Vec<Vec<crate::emit::SourceObjectMemberFacts>>,
    source_object_operations: Vec<Vec<crate::emit::SourceObjectOperationFacts>>,
    source_operations: Vec<SelectedSourceOperationRow>,
    model_enum_declarations: Vec<filament_contract_model::SelectedModelEnumDeclarationRow>,
    model_enum_members: Vec<filament_contract_model::SelectedModelEnumMemberRow>,
    model_enum_shapes: Vec<filament_contract_model::SelectedModelEnumShapeRow>,
    model_tuples: Vec<filament_contract_model::SelectedTupleDeclarationRow>,
    model_unions: Vec<filament_contract_model::SelectedModelUnionRow>,
}

/// Why an identity file was not read: a closed set, naming what it found.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdentityFileRefusal {
    /// Not JSON, not this shape, or an unknown or repeated member.
    #[error("the identity file is malformed at {member}")]
    Malformed {
        /// The member the parse stopped at.
        member: IdentityMember,
    },
    /// Another `format` string.
    #[error("the identity file has format `{0}`")]
    Format(String),
    /// A row the package does not join.
    #[error("the identity file row {index} of {member} is not a node of the package")]
    Unjoined {
        /// `declarations` or `fields`.
        member: IdentityMember,
        /// The first row that does not join.
        index: usize,
    },
    /// IR refused the original producer rows at its typed structural locus.
    #[error("the identity file has an invalid native witness: {0:?}")]
    Native(Box<filament_contract_model::ExactTypeProjectionError>),
    /// IR refused the source-object rows against the admitted graph.
    #[error("the identity file has invalid source-object rows: {0}")]
    SourceObject(#[from] filament_contract_model::SelectedSourceObjectError),
    /// IR refused the source-operation rows against the admitted graph.
    #[error("the identity file has invalid source-operation rows: {0}")]
    SourceOperation(#[from] filament_contract_model::SelectedOperationMemberError),
    /// An allocation-free producer adapter denial with the original budget locus.
    #[error(transparent)]
    Rows(#[from] crate::NativeRowError),
}

/// The member of an identity file a refusal names (FR-531 "The identity file").
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdentityMember {
    /// The file as a whole: not JSON, or not an object.
    #[error("the file")]
    File,
    /// The `format` member.
    #[error("`format`")]
    Format,
    /// A well-shaped but unsupported format string.
    #[error("unsupported format `{0}`")]
    Version(String),
    /// The `declarations` member.
    #[error("`declarations`")]
    Declarations,
    /// The `fields` member.
    #[error("`fields`")]
    Fields,
    /// The required final-emission source-record rows.
    #[error("`source_records`")]
    SourceRecords,
    /// The required final-emission source-tuple rows.
    #[error("`source_tuples`")]
    SourceTuples,
    /// The required final-emission source-union rows.
    #[error("`source_unions`")]
    SourceUnions,
    /// The required final-emission source-enum rows.
    #[error("`source_enums`")]
    SourceEnums,
    /// The required final-emission source-object rows.
    #[error("`source_objects`")]
    SourceObjects,
    /// The required final-emission source-operation rows.
    #[error("`source_operations`")]
    SourceOperations,
    /// The selected model enum declaration, member and shape channels.
    #[error("model enum rows")]
    ModelEnums,
    /// The selected model union channel.
    #[error("model union rows")]
    ModelUnions,
    /// The selected model tuple channel.
    #[error("`model_tuples`")]
    ModelTuples,
    /// A declaration row's `form`.
    #[error("`form`")]
    Form,
    /// A declaration row's `exact_identity`.
    #[error("`exact_identity`")]
    ExactIdentity,
    /// A field row's `member_id`.
    #[error("`member_id`")]
    MemberId,
    /// A member the file does not define.
    #[error("unknown member `{0}`")]
    Unknown(String),
    /// A member written twice.
    #[error("repeated member `{0}`")]
    Repeated(&'static str),
}

#[derive(Serialize)]
struct File {
    format: String,
    declarations: Vec<DeclarationJson>,
    fields: Vec<FieldJson>,
    source_records: Vec<SourceRecordJson>,
    source_tuples: Vec<SourceTupleJson>,
    source_unions: Vec<SourceUnionJson>,
    source_enums: Vec<SourceEnumJson>,
    source_objects: Vec<SourceObjectJson>,
    source_operations: Vec<SourceOperationJson>,
    model_enum_declarations: Vec<model_enum::DeclarationJson>,
    model_enum_members: Vec<model_enum::MemberJson>,
    model_enum_shapes: Vec<model_enum::ShapeJson>,
    model_tuples: Vec<model_tuple::ModelTupleJson>,
    model_unions: Vec<model_union::ModelUnionJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceObjectJson {
    structural_node: CheckedNodeId,
    source_authority: String,
    source_identity: String,
    #[serde(rename = "abstract")]
    abstract_type: bool,
    supertypes: Vec<CheckedNodeId>,
    identity_fields: Vec<String>,
    members: Vec<SourceObjectMemberJson>,
    populations: Vec<SourcePopulationJson>,
    operations: Vec<SourceObjectOperationJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceObjectMemberJson {
    original_member_identity: String,
    admitted_name: String,
    presence: SourceObjectPresenceJson,
    type_node: Option<CheckedNodeId>,
    value_type: Option<OriginalValueType>,
    #[serde(default)]
    member_type: Option<String>,
    #[serde(default)]
    collections: Vec<ClockCollectionLayerJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum SourceObjectPresenceJson {
    Required,
    Optional,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SourcePopulationJson {
    name: String,
    symmetric: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceObjectOperationJson {
    identity: String,
    query: bool,
    clock_parameters: Vec<String>,
    parameters: Vec<SourceObjectParameterJson>,
}

/// One member entry of an operation's parameters: a data entry has a
/// `type_node`, a clock entry a `member_type` and its collection layers.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceObjectParameterJson {
    name: String,
    optional: bool,
    type_node: Option<CheckedNodeId>,
    member_type: Option<String>,
    collections: Vec<ClockCollectionLayerJson>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ClockCollectionLayerJson {
    kind: String,
    minimum: String,
    maximum: String,
}

impl ClockCollectionLayerJson {
    pub(super) fn of(layer: &filament_contract_model::SelectedClockCollectionLayer) -> Self {
        Self {
            kind: layer.kind.to_string(),
            minimum: layer.minimum.to_string(),
            maximum: layer.maximum.to_string(),
        }
    }

    pub(super) fn layer(self) -> filament_contract_model::SelectedClockCollectionLayer {
        filament_contract_model::SelectedClockCollectionLayer {
            kind: self.kind.into(),
            minimum: self.minimum.into(),
            maximum: self.maximum.into(),
        }
    }
}

impl SourceObjectParameterJson {
    fn of(member: &filament_contract_model::SelectedSourceObjectMember) -> Self {
        Self {
            name: member.name.to_string(),
            optional: member.optional,
            type_node: member.type_node.clone(),
            member_type: member.member_type.as_deref().map(str::to_owned),
            collections: member
                .collections
                .iter()
                .map(|layer| ClockCollectionLayerJson {
                    kind: layer.kind.to_string(),
                    minimum: layer.minimum.to_string(),
                    maximum: layer.maximum.to_string(),
                })
                .collect(),
        }
    }

    fn member(self) -> filament_contract_model::SelectedSourceObjectMember {
        filament_contract_model::SelectedSourceObjectMember {
            name: self.name.into(),
            optional: self.optional,
            type_node: self.type_node,
            member_type: self.member_type.map(Into::into),
            collections: self
                .collections
                .into_iter()
                .map(
                    |layer| filament_contract_model::SelectedClockCollectionLayer {
                        kind: layer.kind.into(),
                        minimum: layer.minimum.into(),
                        maximum: layer.maximum.into(),
                    },
                )
                .collect(),
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SourceOperationJson {
    source_authority: String,
    source_identity: String,
    anchor_id: CheckedNodeId,
    structural_node: CheckedNodeId,
}

impl SourceObjectJson {
    fn of(
        row: &SelectedSourceObjectRow,
        member_facts: &[crate::emit::SourceObjectMemberFacts],
        operations: &[crate::emit::SourceObjectOperationFacts],
    ) -> Result<Self, IdentityFileRefusal> {
        let malformed = || IdentityFileRefusal::Malformed {
            member: IdentityMember::SourceObjects,
        };
        if row.members.len() != member_facts.len() {
            return Err(malformed());
        }
        let members = row
            .members
            .iter()
            .zip(member_facts)
            .map(|(member, facts)| {
                if member.name.as_ref() != facts.admitted_name.as_ref()
                    || member.optional != (facts.presence == Presence::Optional)
                {
                    return Err(malformed());
                }
                Ok(SourceObjectMemberJson {
                    original_member_identity: facts.original_member_identity.to_string(),
                    admitted_name: facts.admitted_name.to_string(),
                    presence: match facts.presence {
                        Presence::Required => SourceObjectPresenceJson::Required,
                        Presence::Optional => SourceObjectPresenceJson::Optional,
                    },
                    type_node: member.type_node.clone(),
                    value_type: facts
                        .value_type
                        .as_ref()
                        .map(OriginalValueType::of)
                        .transpose()
                        .map_err(|_| malformed())?,
                    member_type: member.member_type.as_deref().map(str::to_owned),
                    collections: member
                        .collections
                        .iter()
                        .map(ClockCollectionLayerJson::of)
                        .collect(),
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            structural_node: row.structural_node.clone(),
            source_authority: row.source_authority.to_string(),
            source_identity: row.source_identity.to_string(),
            abstract_type: row.is_abstract,
            supertypes: row.supertypes.clone(),
            identity_fields: row
                .identity_fields
                .iter()
                .map(ToString::to_string)
                .collect(),
            members,
            populations: row
                .populations
                .iter()
                .map(|population| SourcePopulationJson {
                    name: population.name.to_string(),
                    symmetric: population.symmetric,
                })
                .collect(),
            operations: if row.operations.len() == operations.len() {
                row.operations
                    .iter()
                    .zip(operations)
                    .map(|(held, operation)| SourceObjectOperationJson {
                        identity: operation.identity.to_string(),
                        query: operation.query,
                        clock_parameters: held
                            .clock_parameters
                            .iter()
                            .map(ToString::to_string)
                            .collect(),
                        parameters: held
                            .parameters
                            .iter()
                            .map(SourceObjectParameterJson::of)
                            .collect(),
                    })
                    .collect()
            } else {
                return Err(malformed());
            },
        })
    }

    fn row(
        self,
    ) -> Result<
        (
            SelectedSourceObjectRow,
            Vec<crate::emit::SourceObjectMemberFacts>,
            Vec<crate::emit::SourceObjectOperationFacts>,
        ),
        IdentityFileRefusal,
    > {
        let malformed = || IdentityFileRefusal::Malformed {
            member: IdentityMember::SourceObjects,
        };
        let mut members = Vec::with_capacity(self.members.len());
        let mut facts = Vec::with_capacity(self.members.len());
        for member in self.members {
            let presence = match member.presence {
                SourceObjectPresenceJson::Required => Presence::Required,
                SourceObjectPresenceJson::Optional => Presence::Optional,
            };
            let optional = presence == Presence::Optional;
            // FR-531: exactly a data entry (both type members, no clock
            // members) or exactly a clock entry (neither, a `member_type`).
            let (entry, value_type) =
                match (member.type_node, member.value_type, member.member_type) {
                    (Some(node), Some(value_type), None) if member.collections.is_empty() => (
                        filament_contract_model::SelectedSourceObjectMember::data(
                            member.admitted_name.clone(),
                            node,
                            optional,
                        ),
                        Some(value_type.value().map_err(|_| malformed())?),
                    ),
                    (None, None, Some(rendering)) => {
                        let mut entry = filament_contract_model::SelectedSourceObjectMember::clock(
                            member.admitted_name.clone(),
                            rendering,
                        );
                        entry.optional = optional;
                        entry.collections = member
                            .collections
                            .into_iter()
                            .map(ClockCollectionLayerJson::layer)
                            .collect();
                        (entry, None)
                    }
                    _ => return Err(malformed()),
                };
            members.push(entry);
            facts.push(crate::emit::SourceObjectMemberFacts {
                original_member_identity: member.original_member_identity.into(),
                admitted_name: member.admitted_name.into(),
                presence,
                value_type,
            });
        }
        let row_operations = self
            .operations
            .iter()
            .map(
                |operation| filament_contract_model::SelectedSourceObjectOperation {
                    identity: operation.identity.as_str().into(),
                    query: operation.query,
                    clock_parameters: operation
                        .clock_parameters
                        .iter()
                        .map(|name| name.as_str().into())
                        .collect(),
                    parameters: operation
                        .parameters
                        .iter()
                        .map(|parameter| parameter.clone().member())
                        .collect(),
                },
            )
            .collect();
        Ok((
            SelectedSourceObjectRow {
                structural_node: self.structural_node,
                source_authority: self.source_authority.into(),
                source_identity: self.source_identity.into(),
                is_abstract: self.abstract_type,
                supertypes: self.supertypes,
                identity_fields: self.identity_fields.into_iter().map(Into::into).collect(),
                members,
                populations: self
                    .populations
                    .into_iter()
                    .map(
                        |population| filament_contract_model::SelectedSourcePopulation {
                            name: population.name.into(),
                            symmetric: population.symmetric,
                        },
                    )
                    .collect(),
                operations: row_operations,
            },
            facts,
            self.operations
                .into_iter()
                .map(|operation| crate::emit::SourceObjectOperationFacts {
                    identity: operation.identity.into(),
                    query: operation.query,
                })
                .collect(),
        ))
    }
}

impl SourceOperationJson {
    fn of(row: &SelectedSourceOperationRow) -> Self {
        Self {
            source_authority: row.source_authority.to_string(),
            source_identity: row.source_identity.to_string(),
            anchor_id: row.anchor_id.clone(),
            structural_node: row.structural_node.clone(),
        }
    }

    fn row(self) -> SelectedSourceOperationRow {
        SelectedSourceOperationRow {
            source_authority: self.source_authority.into(),
            source_identity: self.source_identity.into(),
            anchor_id: self.anchor_id,
            structural_node: self.structural_node,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DeclarationJson {
    structural_node: CheckedNodeId,
    selected_document: String,
    form: String,
    original_identity: String,
    exact_identity: IdentityJson,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct IdentityJson {
    kind: String,
    value: serde_json::Value,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FieldJson {
    record_node: CheckedNodeId,
    selected_document: String,
    record_original_identity: String,
    field_original_identity: String,
    member_id: MemberJson,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MemberJson {
    owner: OwnerJson,
    member: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OwnerJson {
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    node: Option<CheckedNodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    effective: Option<EffectiveJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct EffectiveJson {
    chain: Vec<LinkJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LinkJson {
    package: String,
    node: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct UnitJson {
    kind: String,
    node: CheckedNodeId,
}

struct TopVisitor<'a> {
    problem: &'a std::cell::Cell<Option<IdentityMember>>,
}

impl TopVisitor<'_> {
    fn fail<E: serde::de::Error>(&self, member: IdentityMember) -> E {
        self.problem.set(Some(member));
        E::custom("malformed identity file")
    }
}

impl<'de> serde::de::Visitor<'de> for TopVisitor<'_> {
    type Value = File;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("an identity file object")
    }

    #[ix_trace_rs::string_edge]
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<File, A::Error> {
        let mut format: Option<String> = None;
        let mut declarations: Option<Vec<DeclarationJson>> = None;
        let mut fields: Option<Vec<FieldJson>> = None;
        let mut source_records: Option<Vec<SourceRecordJson>> = None;
        let mut source_tuples: Option<Vec<SourceTupleJson>> = None;
        let mut source_unions: Option<Vec<SourceUnionJson>> = None;
        let mut source_enums: Option<Vec<SourceEnumJson>> = None;
        let mut source_objects: Option<Vec<SourceObjectJson>> = None;
        let mut source_operations: Option<Vec<SourceOperationJson>> = None;
        let mut model_enum_declarations: Option<Vec<model_enum::DeclarationJson>> = None;
        let mut model_enum_members: Option<Vec<model_enum::MemberJson>> = None;
        let mut model_enum_shapes: Option<Vec<model_enum::ShapeJson>> = None;
        let mut model_tuples: Option<Vec<model_tuple::ModelTupleJson>> = None;
        let mut model_unions: Option<Vec<model_union::ModelUnionJson>> = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "format" => {
                    if format.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("format")));
                    }
                    let value: String = map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::Format));
                    })?;
                    if !is_identities_format(&value) {
                        return Err(self.fail(IdentityMember::Version(value)));
                    }
                    format = Some(value);
                }
                "declarations" => {
                    if declarations.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("declarations")));
                    }
                    declarations = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::Declarations));
                    })?);
                }
                "fields" => {
                    if fields.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("fields")));
                    }
                    fields = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::Fields));
                    })?);
                }
                "source_records" => {
                    if source_records.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("source_records")));
                    }
                    source_records = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::SourceRecords));
                    })?);
                }
                "source_tuples" => {
                    if source_tuples.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("source_tuples")));
                    }
                    source_tuples = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::SourceTuples));
                    })?);
                }
                "source_unions" => {
                    if source_unions.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("source_unions")));
                    }
                    source_unions = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::SourceUnions));
                    })?);
                }
                "source_enums" => {
                    if source_enums.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("source_enums")));
                    }
                    source_enums = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::SourceEnums));
                    })?);
                }
                "source_objects" => {
                    if source_objects.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("source_objects")));
                    }
                    source_objects = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::SourceObjects));
                    })?);
                }
                "source_operations" => {
                    if source_operations.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("source_operations")));
                    }
                    source_operations = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::SourceOperations));
                    })?);
                }
                "model_enum_declarations" => {
                    if model_enum_declarations.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("model_enum_declarations")));
                    }
                    model_enum_declarations =
                        Some(map.next_value().inspect_err(|_: &A::Error| {
                            self.problem.set(Some(IdentityMember::ModelEnums));
                        })?);
                }
                "model_enum_members" => {
                    if model_enum_members.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("model_enum_members")));
                    }
                    model_enum_members = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::ModelEnums));
                    })?);
                }
                "model_enum_shapes" => {
                    if model_enum_shapes.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("model_enum_shapes")));
                    }
                    model_enum_shapes = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::ModelEnums));
                    })?);
                }
                "model_tuples" => {
                    if model_tuples.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("model_tuples")));
                    }
                    model_tuples = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::ModelTuples));
                    })?);
                }
                "model_unions" => {
                    if model_unions.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("model_unions")));
                    }
                    model_unions = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::ModelUnions));
                    })?);
                }
                other => return Err(self.fail(IdentityMember::Unknown(other.to_owned()))),
            }
        }
        let source_enums = source_enums.ok_or_else(|| self.fail(IdentityMember::SourceEnums))?;
        let source_tuples = source_tuples.ok_or_else(|| self.fail(IdentityMember::SourceTuples))?;
        let model_enum_declarations =
            model_enum_declarations.ok_or_else(|| self.fail(IdentityMember::ModelEnums))?;
        let model_enum_members =
            model_enum_members.ok_or_else(|| self.fail(IdentityMember::ModelEnums))?;
        let model_enum_shapes =
            model_enum_shapes.ok_or_else(|| self.fail(IdentityMember::ModelEnums))?;
        let model_tuples = model_tuples.ok_or_else(|| self.fail(IdentityMember::ModelTuples))?;
        let model_unions = model_unions.ok_or_else(|| self.fail(IdentityMember::ModelUnions))?;
        match (
            format,
            declarations,
            fields,
            source_records,
            source_unions,
            source_objects,
            source_operations,
        ) {
            (
                Some(format),
                Some(declarations),
                Some(fields),
                Some(source_records),
                Some(source_unions),
                Some(source_objects),
                Some(source_operations),
            ) => Ok(File {
                format,
                declarations,
                fields,
                source_records,
                source_tuples,
                source_unions,
                source_enums,
                source_objects,
                source_operations,
                model_enum_declarations,
                model_enum_members,
                model_enum_shapes,
                model_tuples,
                model_unions,
            }),
            (None, _, _, _, _, _, _) => Err(self.fail(IdentityMember::Format)),
            (_, None, _, _, _, _, _) => Err(self.fail(IdentityMember::Declarations)),
            (_, _, None, _, _, _, _) => Err(self.fail(IdentityMember::Fields)),
            (_, _, _, None, _, _, _) => Err(self.fail(IdentityMember::SourceRecords)),
            (_, _, _, _, None, _, _) => Err(self.fail(IdentityMember::SourceUnions)),
            (_, _, _, _, _, None, _) => Err(self.fail(IdentityMember::SourceObjects)),
            (_, _, _, _, _, _, None) => Err(self.fail(IdentityMember::SourceOperations)),
        }
    }
}

fn parse_file(bytes: &[u8]) -> Result<File, IdentityMember> {
    let problem = std::cell::Cell::new(None);
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    serde::Deserializer::deserialize_map(&mut deserializer, TopVisitor { problem: &problem })
        .and_then(|file| deserializer.end().map(|()| file))
        .map_err(|_| problem.take().unwrap_or(IdentityMember::File))
}

fn form_spelling(form: SelectedExactDeclarationForm) -> &'static str {
    match form {
        SelectedExactDeclarationForm::RecordValueType => "record_value_type",
        SelectedExactDeclarationForm::ObjectType => "object_type",
        SelectedExactDeclarationForm::Union => "union",
        SelectedExactDeclarationForm::DeclaredUnit => "declared_unit",
    }
}

/// Whether `format` spells the identities file format this reader admits.
#[ix_trace_rs::string_edge]
fn is_identities_format(format: &str) -> bool {
    format == IDENTITIES_FORMAT
}

/// Whether `kind` spells a declared unit identity.
#[ix_trace_rs::string_edge]
fn is_declared_unit_kind(kind: &str) -> bool {
    kind == "declared"
}

#[ix_trace_rs::string_edge]
fn form_of(spelling: &str) -> Option<SelectedExactDeclarationForm> {
    Some(match spelling {
        "record_value_type" => SelectedExactDeclarationForm::RecordValueType,
        "object_type" => SelectedExactDeclarationForm::ObjectType,
        "union" => SelectedExactDeclarationForm::Union,
        "declared_unit" => SelectedExactDeclarationForm::DeclaredUnit,
        _ => return None,
    })
}

/// The package's nodes by structural key, for writing a key as the node
/// reference the wire spells it with.
fn node_references(package: &CheckedPackageV2) -> Option<BTreeMap<NodeKey, CheckedNodeId>> {
    let graph = wire_graph(package).ok()?;
    package
        .resolved_graph()
        .nodes
        .iter()
        .map(|node| Some((graph.key(&node.node_id).ok()?, node.node_id.clone())))
        .collect()
}

/// Why identity rows are no witness for a slot call (FR-531-AC-5).
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdentityWitnessRefusal {
    /// The bare package has no supply.
    #[error("the bare package holds no identity rows")]
    Missing,
    /// The rows name no identity the package admits.
    #[error("the identity rows name no admitted identity")]
    Unknown,
    /// The rows were produced for another package.
    #[error("the identity rows belong to another package")]
    Foreign,
}

/// Why the join of identity rows gave no identities: a defect of the witness
/// or the caller's native budget running out (FR-358).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdentityJoinFailure {
    /// The rows are no witness for the package.
    #[error(transparent)]
    Witness(#[from] IdentityWitnessRefusal),
    /// The caller's native work or storage allowance was consumed.
    #[error("the native join budget was consumed: {0}")]
    Exhausted(Box<ExactTypeProjectionError>),
    /// A source-enum input copy or coverage inspection was denied.
    #[error(transparent)]
    Rows(#[from] crate::NativeRowError),
}

#[derive(Debug, thiserror::Error)]
enum IdentityRowsJoinError {
    #[error(transparent)]
    Projection(#[from] ExactTypeProjectionError),
    #[error(transparent)]
    Rows(#[from] crate::NativeRowError),
}

impl ReadIdentityRows {
    pub(crate) fn source_operation_validation_work_bound(
        &self,
        package: &CheckedPackageV2,
    ) -> u128 {
        let effects = package.operation_effect_rows().map_or(0, <[_]>::len) as u128;
        let rows = self.source_operations.len() as u128;
        rows.saturating_mul(effects.saturating_add(1))
    }

    /// Whether the selected model enumeration declared at `node` has its
    /// complete shape row: the declaration is a payload-free enumeration
    /// whose declaration and cases the package reads (FR-531).
    pub(crate) fn holds_model_enum(&self, node: &CheckedNodeId) -> bool {
        self.model_enum_shapes
            .iter()
            .any(|row| &row.structural_node == node)
    }

    /// Whether the successful native join can supply this source declaration.
    pub(crate) fn holds_source_enum(&self, node: &CheckedNodeId) -> bool {
        self.source_enums
            .iter()
            .any(|row| &row.declaration.structural_node == node)
    }

    /// The identity witness these rows give `package`, joined by IR's
    /// `SelectedExactIdentities` projection and charged to `budget`.
    ///
    /// # Errors
    ///
    /// [`IdentityWitnessRefusal::Foreign`] for rows of another package,
    /// [`IdentityWitnessRefusal::Unknown`] for rows IR does not join, and
    /// [`IdentityJoinFailure::Exhausted`] with IR's refusal when `budget`
    /// runs out, which says nothing about the rows.
    pub fn selected(
        &self,
        package: &CheckedPackageV2,
        budget: &mut NativeJoinBudget,
    ) -> Result<SelectedExactIdentities, IdentityJoinFailure> {
        if self.package != package.name() {
            return Err(IdentityWitnessRefusal::Foreign.into());
        }
        let selected = self
            .join(package, budget)
            .map_err(|refusal| match refusal {
                IdentityRowsJoinError::Rows(error) => IdentityJoinFailure::Rows(error),
                IdentityRowsJoinError::Projection(refusal) => {
                    if fsl_semantics::model::intake::is_native_budget_denial(&refusal) {
                        IdentityJoinFailure::Exhausted(Box::new(refusal))
                    } else {
                        IdentityWitnessRefusal::Unknown.into()
                    }
                }
            })?;
        self.source_enum_coverage(package, budget)
            .map_err(|error| {
                if error.denied().is_some() {
                    IdentityJoinFailure::Rows(error)
                } else {
                    IdentityWitnessRefusal::Unknown.into()
                }
            })?;
        Ok(selected)
    }

    fn source_enum_coverage(
        &self,
        package: &CheckedPackageV2,
        budget: &mut NativeJoinBudget,
    ) -> Result<(), crate::NativeRowError> {
        let locus = crate::native_row::start(budget, "source_enum_coverage")?;
        let mut required = 0_usize;
        for node in &package.resolved_graph().nodes {
            crate::native_row::work(budget, locus, 1)?;
            if package.node_kind(&node.node_id)
                == Some(filament_contract_model::CheckedNodeKind::ScalarType(
                    filament_contract_model::ScalarTypeForm::Enum,
                ))
                && matches!(
                    &node.owner,
                    Some(filament_contract_model::CheckedNodeOwner::Source { .. })
                )
                && node.declaration.is_some()
            {
                required += 1;
            }
        }
        // The native join already proved each provided declaration is a
        // unique member of this exact set. Equal cardinality proves coverage.
        if required != self.source_enums.len() {
            return Err(crate::NativeRowError::incomplete_enum(locus));
        }
        Ok(())
    }

    fn join(
        &self,
        package: &CheckedPackageV2,
        budget: &mut NativeJoinBudget,
    ) -> Result<SelectedExactIdentities, IdentityRowsJoinError> {
        let source_enums = source_enum::copy_rows(&self.source_enums, budget)?;
        let identities = SelectedExactIdentities::from_stage_rows(
            package,
            self.declarations.iter().cloned(),
            self.fields.iter().cloned(),
            budget,
        )?
        .with_source_record_rows(package, self.source_records.iter().cloned(), budget)?
        .with_source_tuple_rows(package, self.source_tuples.iter().cloned(), budget)?
        .with_source_union_rows(package, self.source_unions.iter().cloned(), budget)?
        .with_source_enum_rows(package, source_enums, budget)?
        .with_admitted_model_enum_rows(
            package,
            filament_contract_model::SelectedModelEnumRows {
                declarations: &self.model_enum_declarations,
                members: &self.model_enum_members,
                shapes: &self.model_enum_shapes,
            },
            budget,
        )?;
        // Tuple and union witnesses share one full join, including packages
        // that select tuples without a union payload.
        if self.model_tuples.is_empty() && self.model_unions.is_empty() {
            return Ok(identities);
        }
        Ok(identities.with_model_union_rows(
            package,
            &self.model_tuples,
            &self.model_unions,
            filament_contract_model::ModelUnionScope::Full,
            budget,
        )?)
    }

    /// The package the rows were produced for.
    pub fn package_name(&self) -> &str {
        &self.package
    }

    /// The selected declaration rows, in file order.
    pub fn declarations(&self) -> &[SelectedExactDeclarationRow] {
        &self.declarations
    }

    /// The selected record-field rows, in file order.
    pub fn fields(&self) -> &[SelectedExactFieldRow] {
        &self.fields
    }

    /// Original source-record producer rows, with final graph references.
    pub fn source_records(&self) -> &[SelectedSourceRecordRow] {
        &self.source_records
    }

    /// Original source-tuple rows, in declaration and positional order.
    pub fn source_tuples(&self) -> &[filament_contract_model::SelectedSourceTupleRow] {
        &self.source_tuples
    }

    /// Selected model union rows, in structural order.
    pub fn model_unions(&self) -> &[filament_contract_model::SelectedModelUnionRow] {
        &self.model_unions
    }

    /// Selected model tuple declaration rows, in structural order.
    pub fn model_tuples(&self) -> &[filament_contract_model::SelectedTupleDeclarationRow] {
        &self.model_tuples
    }

    /// Original source-union producer rows, in declaration and payload order.
    pub fn source_unions(&self) -> &[SelectedSourceUnionRow] {
        &self.source_unions
    }

    /// Original source-enum declarations and canonical member rows.
    pub fn source_enums(&self) -> &[filament_contract_model::SelectedSourceEnumRow] {
        &self.source_enums
    }

    /// Original source-object rows, with final graph references.
    pub fn source_objects(&self) -> &[SelectedSourceObjectRow] {
        &self.source_objects
    }

    /// The operations each source-object row's type declares itself, aligned
    /// with [`Self::source_objects`].
    pub fn source_object_operations(&self) -> &[Vec<crate::SourceObjectOperationFacts>] {
        &self.source_object_operations
    }

    /// Original source-operation rows, with final anchor references.
    pub fn source_operations(&self) -> &[SelectedSourceOperationRow] {
        &self.source_operations
    }

    /// Validate source-object rows against this package's admitted nodes.
    pub fn selected_source_objects<'a>(
        &self,
        package: &'a CheckedPackageV2,
    ) -> Result<SelectedSourceObjects<'a>, filament_contract_model::SelectedSourceObjectError> {
        SelectedSourceObjects::from_stage_rows(
            package,
            self.source_objects.iter().cloned(),
            filament_contract_model::SelectedSourceClockRows {
                records: &self.source_records,
                tuples: &self.source_tuples,
                unions: &self.source_unions,
            },
        )
    }

    /// Validate source-operation rows against this package under `work_limit`.
    pub fn selected_source_operations<'a>(
        &self,
        package: &'a CheckedPackageV2,
        work_limit: u64,
    ) -> Result<SelectedSourceOperations<'a>, filament_contract_model::SelectedOperationMemberError>
    {
        SelectedSourceOperations::from_stage_rows(
            package,
            self.source_operations.iter().cloned(),
            work_limit,
        )
    }

    /// Read `identities_json`, joining its rows to `package`.
    ///
    /// The rows grant no authority: they are trusted as the package file is,
    /// as an artifact of the producer the consumer chose to read. Only what
    /// the package can answer is verified: that each node a row names is a
    /// node of the package and that each declaration's form matches its
    /// identity. A refusal returns no rows. The validating join charges
    /// `budget`, the caller's one native allowance for the pass (FR-358).
    ///
    /// # Errors
    ///
    /// [`IdentityFileRefusal`]; `Native` carries IR's budget denial when
    /// `budget` runs out.
    pub fn read(
        identities_json: &[u8],
        package: &CheckedPackageV2,
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, IdentityFileRefusal> {
        let malformed = |member: IdentityMember| IdentityFileRefusal::Malformed { member };
        let file = parse_file(identities_json).map_err(|member| match member {
            IdentityMember::Version(format) => IdentityFileRefusal::Format(format),
            other => malformed(other),
        })?;
        if !is_identities_format(&file.format) {
            return Err(IdentityFileRefusal::Format(file.format));
        }
        let graph = wire_graph(package).map_err(|_| malformed(IdentityMember::File))?;
        let held: BTreeSet<&CheckedNodeId> = package
            .resolved_graph()
            .nodes
            .iter()
            .map(|node| &node.node_id)
            .collect();
        let key = |id: &CheckedNodeId, member: IdentityMember, index: usize| {
            if held.contains(id) {
                graph
                    .key(id)
                    .map_err(|_| IdentityFileRefusal::Unjoined { member, index })
            } else {
                Err(IdentityFileRefusal::Unjoined { member, index })
            }
        };
        let mut declarations = Vec::new();
        for (index, row) in file.declarations.iter().enumerate() {
            let unjoined = IdentityFileRefusal::Unjoined {
                member: IdentityMember::Declarations,
                index,
            };
            if !held.contains(&row.structural_node) {
                return Err(unjoined);
            }
            let form = form_of(&row.form).ok_or_else(|| malformed(IdentityMember::Form))?;
            let identity = match (row.exact_identity.kind.as_str(), form) {
                ("record", SelectedExactDeclarationForm::RecordValueType) => {
                    SelectedExactIdentity::Record(node_value(
                        &row.exact_identity.value,
                        &key,
                        IdentityMember::Declarations,
                        index,
                    )?)
                }
                ("union", SelectedExactDeclarationForm::Union) => {
                    SelectedExactIdentity::Union(node_value(
                        &row.exact_identity.value,
                        &key,
                        IdentityMember::Declarations,
                        index,
                    )?)
                }
                ("object", SelectedExactDeclarationForm::ObjectType) => {
                    let effective: EffectiveJson =
                        serde_json::from_value(row.exact_identity.value.clone())
                            .map_err(|_| malformed(IdentityMember::ExactIdentity))?;
                    SelectedExactIdentity::Object(effective_id(&effective, &malformed)?)
                }
                ("declared_unit", SelectedExactDeclarationForm::DeclaredUnit) => {
                    let unit: UnitJson = serde_json::from_value(row.exact_identity.value.clone())
                        .map_err(|_| malformed(IdentityMember::ExactIdentity))?;
                    if !is_declared_unit_kind(&unit.kind) {
                        return Err(malformed(IdentityMember::ExactIdentity));
                    }
                    SelectedExactIdentity::DeclaredUnit(UnitId::declared(key(
                        &unit.node,
                        IdentityMember::Declarations,
                        index,
                    )?))
                }
                ("record" | "union" | "object" | "declared_unit", _) => return Err(unjoined),
                _ => return Err(malformed(IdentityMember::ExactIdentity)),
            };
            declarations.push(SelectedExactDeclarationRow::new(
                row.structural_node.clone(),
                row.selected_document.as_str(),
                form,
                row.original_identity.as_str(),
                identity,
            ));
        }
        let mut fields = Vec::new();
        for (index, row) in file.fields.iter().enumerate() {
            if !held.contains(&row.record_node) {
                return Err(IdentityFileRefusal::Unjoined {
                    member: IdentityMember::Fields,
                    index,
                });
            }
            let owner = match (
                row.member_id.owner.kind.as_str(),
                &row.member_id.owner.node,
                &row.member_id.owner.effective,
            ) {
                ("node", Some(node), None) => {
                    MemberOwner::Node(key(node, IdentityMember::Fields, index)?)
                }
                ("effective", None, Some(effective)) => {
                    MemberOwner::Effective(effective_id(effective, &malformed)?)
                }
                _ => return Err(malformed(IdentityMember::MemberId)),
            };
            let member_id = MemberId::new(owner, row.member_id.member.as_str())
                .map_err(|_| malformed(IdentityMember::MemberId))?;
            fields.push(SelectedExactFieldRow::new(
                row.record_node.clone(),
                row.selected_document.as_str(),
                row.record_original_identity.as_str(),
                row.field_original_identity.as_str(),
                member_id,
            ));
        }
        let source_records = file
            .source_records
            .into_iter()
            .map(SourceRecordJson::row)
            .collect::<Result<Vec<_>, _>>()?;
        let source_object_pairs = file
            .source_objects
            .into_iter()
            .map(SourceObjectJson::row)
            .collect::<Result<Vec<_>, _>>()?;
        let mut source_objects = Vec::with_capacity(source_object_pairs.len());
        let mut source_object_member_facts = Vec::with_capacity(source_object_pairs.len());
        let mut source_object_operations = Vec::with_capacity(source_object_pairs.len());
        for (row, facts, operations) in source_object_pairs {
            source_objects.push(row);
            source_object_member_facts.push(facts);
            source_object_operations.push(operations);
        }
        let rows = Self {
            package: package.name().to_owned(),
            declarations,
            fields,
            source_records,
            source_tuples: file
                .source_tuples
                .into_iter()
                .map(SourceTupleJson::row)
                .collect::<Result<_, _>>()?,
            source_unions: file
                .source_unions
                .into_iter()
                .map(SourceUnionJson::row)
                .collect::<Result<_, _>>()?,
            source_enums: file
                .source_enums
                .into_iter()
                .map(SourceEnumJson::row)
                .collect::<Result<_, _>>()?,
            source_objects,
            source_object_member_facts,
            source_object_operations,
            source_operations: file
                .source_operations
                .into_iter()
                .map(SourceOperationJson::row)
                .collect(),
            model_enum_declarations: file
                .model_enum_declarations
                .into_iter()
                .map(model_enum::DeclarationJson::row)
                .collect::<Result<_, _>>()?,
            model_enum_members: file
                .model_enum_members
                .into_iter()
                .map(model_enum::MemberJson::row)
                .collect::<Result<_, _>>()?,
            model_enum_shapes: file
                .model_enum_shapes
                .into_iter()
                .map(model_enum::ShapeJson::row)
                .collect::<Result<_, _>>()?,
            model_tuples: file
                .model_tuples
                .into_iter()
                .map(model_tuple::ModelTupleJson::row)
                .collect::<Result<_, _>>()?,
            model_unions: file
                .model_unions
                .into_iter()
                .map(model_union::ModelUnionJson::row)
                .collect::<Result<_, _>>()?,
        };
        rows.join(package, budget).map_err(|error| match error {
            IdentityRowsJoinError::Projection(error) => {
                IdentityFileRefusal::Native(Box::new(error))
            }
            IdentityRowsJoinError::Rows(error) => IdentityFileRefusal::Rows(error),
        })?;
        rows.selected_source_objects(package)?;
        let work_bound = rows.source_operation_validation_work_bound(package);
        crate::native_row::work(
            budget,
            filament_contract_model::NativeJoinLocus {
                source: filament_contract_model::NativeJoinLocusSource::Package,
                form: "read-source-operation-rows",
            },
            work_bound,
        )?;
        let work_limit = u64::try_from(work_bound).unwrap_or(0);
        rows.selected_source_operations(package, work_limit)?;
        Ok(rows)
    }

    /// The rows of the in-process unit, for a package it was emitted as.
    pub fn of_rows(
        package: &CheckedPackageV2,
        declarations: Vec<SelectedExactDeclarationRow>,
        fields: Vec<SelectedExactFieldRow>,
        source_records: Vec<SelectedSourceRecordRow>,
        source_unions: Vec<SelectedSourceUnionRow>,
    ) -> Self {
        Self::of_rows_with_source_rows(
            package,
            declarations,
            fields,
            source_records,
            source_unions,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }

    /// The rows of the in-process unit, including source-object and
    /// source-operation witnesses. `source_object_member_facts` must align
    /// one-for-one with `source_objects` and with each object's members. These
    /// checked facts supply the FR-531 original member identity and type data
    /// that [`SelectedSourceObjectRow`] does not retain.
    /// `source_object_operations` aligns one-for-one with `source_objects`,
    /// each entry the operations that object's type declares itself.
    #[allow(
        clippy::too_many_arguments,
        reason = "mirrors the existing wide signature; a parameter struct is a separate refactor"
    )]
    pub fn of_rows_with_source_rows(
        package: &CheckedPackageV2,
        declarations: Vec<SelectedExactDeclarationRow>,
        fields: Vec<SelectedExactFieldRow>,
        source_records: Vec<SelectedSourceRecordRow>,
        source_unions: Vec<SelectedSourceUnionRow>,
        source_objects: Vec<SelectedSourceObjectRow>,
        source_operations: Vec<SelectedSourceOperationRow>,
        source_object_member_facts: Vec<Vec<crate::SourceObjectMemberFacts>>,
        source_object_operations: Vec<Vec<crate::SourceObjectOperationFacts>>,
    ) -> Self {
        Self {
            package: package.name().to_owned(),
            declarations,
            fields,
            source_records,
            source_tuples: Vec::new(),
            source_unions,
            source_enums: Vec::new(),
            source_objects,
            source_object_member_facts,
            source_object_operations,
            source_operations,
            model_enum_declarations: Vec::new(),
            model_enum_members: Vec::new(),
            model_enum_shapes: Vec::new(),
            model_tuples: Vec::new(),
            model_unions: Vec::new(),
        }
    }

    pub(crate) fn with_source_enums(
        mut self,
        enums: Vec<filament_contract_model::SelectedSourceEnumRow>,
    ) -> Self {
        self.source_enums = enums;
        self
    }

    pub(crate) fn with_source_tuples(
        mut self,
        tuples: Vec<filament_contract_model::SelectedSourceTupleRow>,
    ) -> Self {
        self.source_tuples = tuples;
        self
    }

    pub(crate) fn with_model_enums(
        mut self,
        declarations: Vec<filament_contract_model::SelectedModelEnumDeclarationRow>,
        members: Vec<filament_contract_model::SelectedModelEnumMemberRow>,
        shapes: Vec<filament_contract_model::SelectedModelEnumShapeRow>,
    ) -> Self {
        self.model_enum_declarations = declarations;
        self.model_enum_members = members;
        self.model_enum_shapes = shapes;
        self
    }

    pub(crate) fn with_model_unions(
        mut self,
        unions: Vec<filament_contract_model::SelectedModelUnionRow>,
    ) -> Self {
        self.model_unions = unions;
        self
    }

    pub(crate) fn with_model_tuples(
        mut self,
        tuples: Vec<filament_contract_model::SelectedTupleDeclarationRow>,
    ) -> Self {
        self.model_tuples = tuples;
        self
    }

    /// The identity file of these rows (FR-531 "The identity file"): one JSON
    /// object, pretty-printed with two-space indent, members in the
    /// documented order, UTF-8, ending in one newline, byte-identical for
    /// equal inputs.
    ///
    /// # Errors
    ///
    /// [`IdentityFileRefusal::Malformed`] for a compound unit, which is no
    /// selected declaration, and for a key the package does not hold.
    pub fn to_bytes(&self, package: &CheckedPackageV2) -> Result<Vec<u8>, IdentityFileRefusal> {
        let malformed = |member: IdentityMember| IdentityFileRefusal::Malformed { member };
        self.selected_source_objects(package)?;
        let work_bound = self.source_operation_validation_work_bound(package);
        let work_limit = u64::try_from(work_bound).unwrap_or(0);
        self.selected_source_operations(package, work_limit)?;
        let references = node_references(package).ok_or_else(|| malformed(IdentityMember::File))?;
        let reference = |key: &NodeKey| {
            references
                .get(key)
                .cloned()
                .ok_or_else(|| malformed(IdentityMember::ExactIdentity))
        };
        let effective = |id: &EffectiveId| EffectiveJson {
            chain: id
                .owner_chain()
                .iter()
                .map(|link| LinkJson {
                    package: link.package().to_owned(),
                    node: link.node().to_owned(),
                })
                .collect(),
        };
        let mut declarations = Vec::new();
        for row in &self.declarations {
            let (kind, value) = match &row.exact_identity {
                SelectedExactIdentity::Record(key) => (
                    "record",
                    serde_json::to_value(reference(key)?)
                        .map_err(|_| malformed(IdentityMember::File))?,
                ),
                SelectedExactIdentity::Union(key) => (
                    "union",
                    serde_json::to_value(reference(key)?)
                        .map_err(|_| malformed(IdentityMember::File))?,
                ),
                SelectedExactIdentity::Object(id) => (
                    "object",
                    serde_json::to_value(effective(id))
                        .map_err(|_| malformed(IdentityMember::File))?,
                ),
                SelectedExactIdentity::DeclaredUnit(unit) => {
                    let key = unit
                        .declared_key()
                        .ok_or_else(|| malformed(IdentityMember::ExactIdentity))?;
                    (
                        "declared_unit",
                        serde_json::to_value(UnitJson {
                            kind: "declared".to_owned(),
                            node: reference(key)?,
                        })
                        .map_err(|_| malformed(IdentityMember::File))?,
                    )
                }
            };
            declarations.push(DeclarationJson {
                structural_node: row.structural_node.clone(),
                selected_document: row.selected_document.to_string(),
                form: form_spelling(row.form).to_owned(),
                original_identity: row.original_identity.to_string(),
                exact_identity: IdentityJson {
                    kind: kind.to_owned(),
                    value,
                },
            });
        }
        let mut fields = Vec::new();
        for row in &self.fields {
            let owner = match row.member_id.owner() {
                MemberOwner::Node(key) => OwnerJson {
                    kind: "node".to_owned(),
                    node: Some(reference(key)?),
                    effective: None,
                },
                MemberOwner::Effective(id) => OwnerJson {
                    kind: "effective".to_owned(),
                    node: None,
                    effective: Some(effective(id)),
                },
            };
            fields.push(FieldJson {
                record_node: row.record_node.clone(),
                selected_document: row.selected_document.to_string(),
                record_original_identity: row.record_original_identity.to_string(),
                field_original_identity: row.field_original_identity.to_string(),
                member_id: MemberJson {
                    owner,
                    member: row.member_id.member().to_owned(),
                },
            });
        }
        if self.source_objects.len() != self.source_object_member_facts.len()
            || self.source_objects.len() != self.source_object_operations.len()
        {
            return Err(malformed(IdentityMember::SourceObjects));
        }
        let source_objects = self
            .source_objects
            .iter()
            .zip(&self.source_object_member_facts)
            .zip(&self.source_object_operations)
            .map(|((row, facts), operations)| SourceObjectJson::of(row, facts, operations))
            .collect::<Result<_, _>>()?;
        let file = File {
            format: IDENTITIES_FORMAT.to_owned(),
            declarations,
            fields,
            source_records: self
                .source_records
                .iter()
                .map(SourceRecordJson::of)
                .collect::<Result<_, _>>()?,
            source_tuples: self
                .source_tuples
                .iter()
                .map(SourceTupleJson::of)
                .collect::<Result<_, _>>()?,
            source_unions: self
                .source_unions
                .iter()
                .map(SourceUnionJson::of)
                .collect::<Result<_, _>>()?,
            source_enums: self
                .source_enums
                .iter()
                .map(SourceEnumJson::of)
                .collect::<Result<_, _>>()?,
            source_objects,
            source_operations: self
                .source_operations
                .iter()
                .map(SourceOperationJson::of)
                .collect(),
            model_enum_declarations: self
                .model_enum_declarations
                .iter()
                .map(model_enum::DeclarationJson::of)
                .collect::<Result<_, _>>()?,
            model_enum_members: self
                .model_enum_members
                .iter()
                .map(model_enum::MemberJson::of)
                .collect::<Result<_, _>>()?,
            model_enum_shapes: self
                .model_enum_shapes
                .iter()
                .map(model_enum::ShapeJson::of)
                .collect::<Result<_, _>>()?,
            model_tuples: self
                .model_tuples
                .iter()
                .map(model_tuple::ModelTupleJson::of)
                .collect::<Result<_, _>>()?,
            model_unions: self
                .model_unions
                .iter()
                .map(model_union::ModelUnionJson::of)
                .collect::<Result<_, _>>()?,
        };
        let mut bytes =
            serde_json::to_vec_pretty(&file).map_err(|_| malformed(IdentityMember::File))?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

fn node_value(
    value: &serde_json::Value,
    key: &impl Fn(&CheckedNodeId, IdentityMember, usize) -> Result<NodeKey, IdentityFileRefusal>,
    member: IdentityMember,
    index: usize,
) -> Result<NodeKey, IdentityFileRefusal> {
    let id: CheckedNodeId =
        serde_json::from_value(value.clone()).map_err(|_| IdentityFileRefusal::Malformed {
            member: IdentityMember::ExactIdentity,
        })?;
    key(&id, member, index)
}

fn effective_id(
    effective: &EffectiveJson,
    malformed: &impl Fn(IdentityMember) -> IdentityFileRefusal,
) -> Result<EffectiveId, IdentityFileRefusal> {
    let mut chain = effective.chain.iter();
    let first = chain
        .next()
        .ok_or_else(|| malformed(IdentityMember::ExactIdentity))?;
    let mut id = EffectiveId::new(
        None,
        OriginalDeclarationKey::new(&first.package, &first.node),
    );
    for link in chain {
        id = EffectiveId::new(
            Some(id),
            OriginalDeclarationKey::new(&link.package, &link.node),
        );
    }
    Ok(id)
}
