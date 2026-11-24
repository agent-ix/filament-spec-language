// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120: the layer-3 state-model seam a simulator of a checked package's
//! state family calls.
//!
//! [`StateModel`] re-derives or retains the domain package views FR-106
//! admission reads and keeps the checked [`TypeEnvironment`]. It admits an initial snapshot
//! into a [`ModelState`] ([`StateModel::admit_initial`]) and builds the
//! [`Observation`] FR-107 evaluates a clause over for any state
//! ([`StateModel::observation`]). The frame decision a simulator makes
//! between two states is [`crate::model::population::decide_frame`], called
//! directly over the model's [`StateModel::population_index`].

use std::collections::BTreeMap;
use std::sync::Arc;

use filament_contract_model::{
    NativeJoinBudget, NativeJoinChargeKind, NativeJoinDeniedCharge, NativeJoinLocus,
    NativeJoinLocusSource,
};

use filament_exact::{EffectiveId, ObjectReference, Value, ValueType};
use filament_semantic_value::declaration::TypeEnvironment;
use filament_semantic_value::relationship_extent::RelationshipExtent;

use crate::model::accounting::ModelNormalizationLimits;
use crate::model::domain_package::DomainPackageRecord;
use crate::model::index::ModelIndex;
use crate::model::intake::{NormalizedModel, PackageInput};
use crate::model::normalize::EffectiveView;
use crate::model::observation::{
    admit_initial_state, model_views, observe_state, state_fields, state_reference,
    AdmissionFailure, DocumentRef, ModelSelections, ModelView, Observation, ObservationLimits,
    PathStep, SelectedAnchor,
};
use filament_checked::key::DeclarationKey;

/// One complete, closed population state: for each population, its objects
/// by declared key.
#[derive(Clone, Debug)]
pub struct ModelState {
    /// Each listed population's objects, by the object's declared key.
    pub populations: BTreeMap<DeclarationKey, BTreeMap<String, StateObject>>,
    /// The relationship extents the state lists, one per listed
    /// relationship.
    pub extents: Vec<RelationshipExtent<DeclarationKey>>,
}

/// One object of a [`ModelState`].
#[derive(Clone, Debug)]
pub struct StateObject {
    /// The object's most-specific type.
    pub type_identity: DeclarationKey,
    /// The object's present declared field values, by the field's own
    /// declaration key. A field declared `?` that the object leaves absent
    /// has no entry.
    pub fields: BTreeMap<DeclarationKey, Value>,
}

/// One declared field of an object type, in the type's effective attribute
/// order.
#[derive(Clone, Debug)]
pub struct StateField {
    /// The field's own declaration key, the key a [`StateObject`] holds its
    /// value under.
    pub key: DeclarationKey,
    /// The field's declared name.
    pub name: String,
    /// The field's declared value type.
    pub value_type: ValueType,
    /// Whether the field is declared `?` and so may be absent.
    pub optional: bool,
}

/// A `min` or `max` bound on the quantities a member holds (FSpec FR-154
/// "Unit bounds"): read from a constraint on the unit the quantity is typed
/// by, or on an alias on the way to it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberBound {
    /// The member whose value holds the quantities: an object type's field
    /// (the key a [`StateObject`] holds its value under), an operation
    /// parameter, or an operation for its result.
    pub member: DeclarationKey,
    /// The record fields from the member's value down to the quantity,
    /// outermost first: empty when the member is typed by the unit itself.
    pub path: Vec<String>,
    /// The constraint's declaration key.
    pub constraint: DeclarationKey,
    /// `max` or `min`.
    pub upper: bool,
    /// The bound, a magnitude in the unit.
    pub bound: filament_exact::Rational,
}

impl MemberBound {
    /// The constraint's keyword, `max` or `min`.
    pub fn keyword(&self) -> &'static str {
        if self.upper {
            "max"
        } else {
            "min"
        }
    }

    /// The bound spelled canonically, as the constraint's `operands.value`
    /// is: `n`, or `n/d` when the denominator is not 1.
    pub fn canonical_bound(&self) -> String {
        if self.bound.denominator() == &filament_exact::Integer::from(1_i64) {
            self.bound.numerator().to_string()
        } else {
            self.bound.to_string()
        }
    }

    /// Whether the quantity `value` satisfies the bound, equality
    /// satisfying; `true` for any value that is no quantity.
    pub fn admits(&self, value: &Value) -> bool {
        let Value::Quantity(quantity) = value else {
            return true;
        };
        if self.upper {
            quantity.magnitude() <= &self.bound
        } else {
            quantity.magnitude() >= &self.bound
        }
    }

    /// The first quantity the member's value `value` holds at [`Self::path`]
    /// that violates the bound, with its path below the member: through
    /// each element of a collection, the payload of a present optional
    /// value and the record field the path names, read by `types`.
    pub fn violation(&self, value: &Value, types: &TypeEnvironment) -> Option<Vec<PathStep>> {
        violation_at(self, value, &self.path, types)
    }
}

/// [`MemberBound::violation`] with `rest` of the path still to walk.
fn violation_at(
    bound: &MemberBound,
    value: &Value,
    rest: &[String],
    types: &TypeEnvironment,
) -> Option<Vec<PathStep>> {
    let below = |step: PathStep, path: Option<Vec<PathStep>>| {
        path.map(|mut path| {
            path.insert(0, step);
            path
        })
    };
    match value {
        Value::Collection(collection) => {
            collection
                .elements()
                .iter()
                .enumerate()
                .find_map(|(index, element)| {
                    below(
                        PathStep::Index(index),
                        violation_at(bound, element, rest, types),
                    )
                })
        }
        Value::Option(option) => below(
            PathStep::OptionValue,
            violation_at(bound, option.payload()?, rest, types),
        ),
        Value::Composite(composite) => {
            let (name, rest) = rest.split_first()?;
            let filament_semantic_value::declaration::CompositeShape::Record(fields) =
                types.composite(composite.declaration().clone())?.shape()
            else {
                return None;
            };
            let slot = fields.iter().position(|field| field.name() == name)?;
            let filament_exact::FieldValue::Present(inner) = composite.slots().get(slot)? else {
                return None;
            };
            below(
                PathStep::Field(name.clone()),
                violation_at(bound, inner, rest, types),
            )
        }
        value => (rest.is_empty() && !bound.admits(value)).then(Vec::new),
    }
}

/// The bounds of every member of `views`' packages (FSpec FR-154 "Unit
/// bounds"): for a quantity of the unit `u` an object field, an operation
/// parameter or result holds, directly, through aliases, as a sequence's
/// items or as a field of a record it holds, every bound on `u` itself and
/// every bound on an alias the quantity's type passes. A bound on an alias
/// bounds no other member of `u`.
pub(crate) fn member_bounds(views: &[ModelView]) -> Vec<MemberBound> {
    let mut bounds = Vec::new();
    for view in views {
        let records = &view.view.domain_package().records;
        let walk = Walk { records };
        for record in records {
            match record {
                DomainPackageRecord::FieldMember(field)
                    if matches!(
                        walk.record(&field.owner),
                        Some(DomainPackageRecord::ObjectType(_))
                    ) =>
                {
                    walk.member(&field.key, &field.value_type, &mut Vec::new(), &mut bounds);
                }
                DomainPackageRecord::OperationMember(operation) => {
                    for parameter in &operation.parameters {
                        walk.member(
                            &parameter.key,
                            &parameter.value_type,
                            &mut Vec::new(),
                            &mut bounds,
                        );
                    }
                    if let Some(result) = &operation.result {
                        walk.member(
                            &operation.key,
                            &result.value_type,
                            &mut Vec::new(),
                            &mut bounds,
                        );
                    }
                }
                _ => {}
            }
        }
    }
    bounds
}

/// The walk of one package's records for [`member_bounds`].
struct Walk<'r> {
    records: &'r [DomainPackageRecord],
}

impl Walk<'_> {
    fn record(&self, key: &DeclarationKey) -> Option<&DomainPackageRecord> {
        self.records.iter().find(|record| record.key() == key)
    }

    /// The bounds on the quantities a value of `value_type` holds at
    /// `path` below `member`.
    fn member(
        &self,
        member: &DeclarationKey,
        value_type: &crate::model::domain_package::ValueTypeRef,
        path: &mut Vec<String>,
        bounds: &mut Vec<MemberBound>,
    ) {
        use crate::model::domain_package::ValueTypeRef;
        let mut aliases = Vec::new();
        let mut value_type = value_type;
        let mut steps = 0_usize;
        while let ValueTypeRef::Package(key) = value_type {
            steps += 1;
            if steps > self.records.len() {
                return;
            }
            value_type = match self.record(key) {
                Some(DomainPackageRecord::AliasType(alias)) => {
                    aliases.push(key);
                    &alias.target
                }
                Some(DomainPackageRecord::SequenceType(sequence)) => &sequence.items,
                Some(DomainPackageRecord::OptionType(option)) => &option.inner,
                Some(DomainPackageRecord::MapType(map)) => &map.values,
                Some(DomainPackageRecord::RecordValueType(_))
                    if path.len() < self.records.len() =>
                {
                    for field in self.records.iter().filter_map(|record| match record {
                        DomainPackageRecord::FieldMember(field) if &field.owner == key => {
                            Some(field)
                        }
                        _ => None,
                    }) {
                        let Some(name) = crate::model::intake::member_identity_name(
                            &field.owner.node,
                            &field.key.node,
                        ) else {
                            continue;
                        };
                        path.push(name.to_owned());
                        self.member(member, &field.value_type, path, bounds);
                        path.pop();
                    }
                    return;
                }
                _ => return,
            };
        }
        let ValueTypeRef::Unit(unit) = value_type else {
            return;
        };
        for record in self.records {
            let DomainPackageRecord::ValueBound(bound) = record else {
                continue;
            };
            let applies = match &bound.alias {
                Some(alias) => aliases.contains(&alias),
                None => bound.target == ValueTypeRef::Unit(unit.clone()),
            };
            if applies {
                bounds.push(MemberBound {
                    member: member.clone(),
                    path: path.clone(),
                    constraint: bound.key.clone(),
                    upper: bound.upper,
                    bound: bound.value.clone(),
                });
            }
        }
    }
}

/// One population of the selected models and its declared member types.
#[derive(Clone, Debug)]
pub struct StatePopulation {
    /// The population's declaration key.
    pub key: DeclarationKey,
    /// The population's declared member types, in declaration order.
    pub member_types: Vec<DeclarationKey>,
    /// Whether the population is declared `symmetric` (FR-150).
    pub symmetric: bool,
}

/// FR-106's re-derived model views and the checked type environment.
#[derive(Debug)]
pub struct StateModel {
    pub(crate) views: Vec<ModelView>,
    pub(crate) types: StateTypes,
    pub(crate) limits: ModelNormalizationLimits,
}

#[derive(Debug)]
pub(crate) enum StateTypes {
    Owned(TypeEnvironment),
    Shared(Arc<TypeEnvironment>),
}

impl std::ops::Deref for StateTypes {
    type Target = TypeEnvironment;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(types) => types,
            Self::Shared(types) => types,
        }
    }
}

/// A failure to retain or select genuine normalized model owners.
#[derive(Debug, thiserror::Error)]
pub enum RetainedModelFailure {
    /// Selected owners came from incompatible structural admission policies.
    #[error(
        "normalized model structural policies differ between owners {first_owner} and {owner}"
    )]
    IncompatiblePolicy {
        /// Index of the first selected owner in the shared owner vector.
        first_owner: usize,
        /// That owner's original structural ancestor bound.
        first_ancestor_steps: u64,
        /// Index of the incompatible owner in the shared owner vector.
        owner: usize,
        /// The incompatible owner's original structural ancestor bound.
        ancestor_steps: u64,
    },
    /// A requested model is absent, duplicated, or supplied more than once.
    #[error("normalized model selection is missing or duplicated at {selection}")]
    Selection {
        /// Position in the caller's requested selection list.
        selection: usize,
    },
    /// The original model's normalization allowance refused inspection.
    #[error("normalized model reference inspection was denied")]
    Normalization {
        /// Index of the original owner whose meter denied the inspection.
        owner: usize,
        /// The original normalization refusal, unchanged.
        incomplete: crate::model::accounting::NormalizationIncomplete,
    },
    /// The invocation's native allowance refused wrapper work or storage.
    #[error("retained model wrapper charge was denied")]
    Native {
        /// The original first unpaid charge.
        denied: NativeJoinDeniedCharge,
        /// The original first unpaid location.
        locus: Option<NativeJoinLocus>,
        /// The original configured bound for the denied resource.
        configured_bound: u64,
    },
    /// Wrapper allocation failed after its reservation was admitted.
    #[error("retained model wrapper allocation failed")]
    Allocation,
}

fn retain_charge(
    budget: &mut NativeJoinBudget,
    kind: NativeJoinChargeKind,
    amount: u128,
    source: NativeJoinLocusSource,
) -> Result<(), RetainedModelFailure> {
    let locus = NativeJoinLocus {
        source,
        form: "retained-normalized-model",
    };
    budget
        .accounted_charge(kind, amount, locus)
        .map_err(|denied| RetainedModelFailure::Native {
            denied,
            locus: budget.first_unpaid_locus(),
            configured_bound: match denied.kind {
                NativeJoinChargeKind::Work => budget.work_limit(),
                NativeJoinChargeKind::Storage => budget.storage_limit(),
            },
        })
}

impl StateModel {
    /// Restore the complete data declarations of dense models from this
    /// model's existing admitted views. Each source has already been joined
    /// to its actual selected object node and source declaration region.
    /// Clock places remain outside the data environment. No model is
    /// re-normalized and the admitted views and their allowances are retained.
    pub fn read_dense_data_types(
        &mut self,
        sources: &[crate::check::ModelTimeSource],
        units: &BTreeMap<String, filament_exact::UnitId>,
    ) -> Result<&TypeEnvironment, crate::check::AssemblyRefusal> {
        let mut types = (*self.types).clone();
        for source in sources {
            let view = self.views.iter().find(|view| {
                !view.is_source()
                    && view.view.domain_package().model_selection.identity == source.model
                    && crate::check::lowering::AdmittedModel::from_view(&view.view)
                        .object_type_model_node_keys()
                        .as_slice()
                        == [source.model_node.clone()]
            });
            let Some(view) = view else {
                return Err(crate::check::AssemblyRefusal {
                    errors: vec![crate::check::AssemblyError {
                        span: source.span,
                        cause: crate::check::AssemblyCause::ModelType {
                            alias: source.model.clone(),
                            node: source.model_node.to_string(),
                        },
                    }],
                });
            };
            types =
                crate::check::dense_model_data_environment(&view.view, source.span, &types, units)?;
        }
        self.types = StateTypes::Owned(types);
        Ok(&self.types)
    }

    /// Reconstruct a clause-free dense timed model from the selected domain
    /// package whose actual object node is `node`. Returns no model for an
    /// ambiguous selection or a clock place outside required direct fields.
    pub fn direct_dense_model(
        &self,
        node: &filament_exact::NodeKey,
    ) -> Option<crate::check::TimedModel> {
        let mut matches = self
            .views
            .iter()
            .filter(|view| !view.is_source())
            .filter_map(|view| {
                let model = crate::check::lowering::AdmittedModel::from_view(&view.view);
                (model.object_type_model_node_keys().as_slice() == [node.clone()])
                    .then_some((model, view))
            });
        let (model, view) = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        let package = view.view.domain_package();
        let lookup =
            |key: &DeclarationKey| package.records.iter().find(|record| record.key() == key);
        if package.records.iter().any(|record| match record {
            DomainPackageRecord::OperationMember(operation) => {
                operation.parameters.iter().any(|parameter| {
                    matches!(
                        crate::model::domain_package::unaliased(&parameter.value_type, lookup),
                        Some((crate::model::domain_package::ValueTypeRef::Clock, _))
                    )
                })
            }
            _ => false,
        }) {
            return None;
        }
        crate::check::TimedModel::direct_dense(
            &model,
            view.view.domain_package().model_selection.identity.clone(),
            node.clone(),
        )
    }

    /// Move the genuine owners into one shared invocation container, reserving
    /// its fixed control block before allocation on the caller's native ledger.
    /// The existing vector, payloads, maps and original meters are moved intact.
    ///
    /// # Errors
    /// Returns the original native denial before allocating the shared owner.
    pub fn retain_normalized_models(
        models: Vec<NormalizedModel>,
        budget: &mut NativeJoinBudget,
    ) -> Result<Arc<Vec<NormalizedModel>>, RetainedModelFailure> {
        retain_charge(
            budget,
            NativeJoinChargeKind::Work,
            1,
            NativeJoinLocusSource::Package,
        )?;
        retain_charge(
            budget,
            NativeJoinChargeKind::Storage,
            (std::mem::size_of::<Vec<NormalizedModel>>() + 2 * std::mem::size_of::<usize>())
                as u128,
            NativeJoinLocusSource::Package,
        )?;
        Ok(Arc::new(models))
    }

    /// Construct a state model by borrowing the actual selected owners. The
    /// supplied type environment is moved; no model or identity map is copied
    /// and every inspection consumes that owner's original normalization meter.
    /// Selected owners must have the same original structural ancestor bound;
    /// that exact bound is retained.
    ///
    /// # Errors
    /// Refuses absent or repeated selections, incompatible original structural
    /// policies, original normalization denials, native wrapper denials, and
    /// failed vector reservation distinctly.
    pub fn from_normalized_models(
        selections: &[crate::model::domain_package::DomainPackageRef],
        types: TypeEnvironment,
        owners: &Arc<Vec<NormalizedModel>>,
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, RetainedModelFailure> {
        Self::reserve_retained_wrapper(budget)?;
        Self::from_retained_parts(selections, StateTypes::Owned(types), owners, budget)
    }

    /// Construct a retained model sharing the immutable genuine type environment
    /// of its prepared package world. Only Arc handles are copied, after native
    /// reservation; original model policy and work counters remain unchanged.
    ///
    /// # Errors
    /// The same typed refusals as [`Self::from_normalized_models`].
    pub fn from_normalized_models_shared_types(
        selections: &[crate::model::domain_package::DomainPackageRef],
        types: &Arc<TypeEnvironment>,
        owners: &Arc<Vec<NormalizedModel>>,
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, RetainedModelFailure> {
        Self::reserve_retained_wrapper(budget)?;
        retain_charge(
            budget,
            NativeJoinChargeKind::Work,
            1,
            NativeJoinLocusSource::Package,
        )?;
        Self::from_retained_parts(
            selections,
            StateTypes::Shared(types.clone()),
            owners,
            budget,
        )
    }

    fn reserve_retained_wrapper(budget: &mut NativeJoinBudget) -> Result<(), RetainedModelFailure> {
        retain_charge(
            budget,
            NativeJoinChargeKind::Work,
            1,
            NativeJoinLocusSource::Package,
        )?;
        retain_charge(
            budget,
            NativeJoinChargeKind::Storage,
            std::mem::size_of::<Self>() as u128,
            NativeJoinLocusSource::Package,
        )
    }

    fn from_retained_parts(
        selections: &[crate::model::domain_package::DomainPackageRef],
        types: StateTypes,
        owners: &Arc<Vec<NormalizedModel>>,
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, RetainedModelFailure> {
        retain_charge(
            budget,
            NativeJoinChargeKind::Storage,
            (selections.len() as u128) * (std::mem::size_of::<ModelView>() as u128),
            NativeJoinLocusSource::Package,
        )?;
        let mut views: Vec<ModelView> = Vec::new();
        views
            .try_reserve_exact(selections.len())
            .map_err(|_| RetainedModelFailure::Allocation)?;
        let mut limits = None::<(usize, ModelNormalizationLimits)>;
        for (selection, requested) in selections.iter().enumerate() {
            retain_charge(
                budget,
                NativeJoinChargeKind::Work,
                1,
                NativeJoinLocusSource::PreparationSelection(selection),
            )?;
            let mut selected = None;
            for (index, owner) in owners.iter().enumerate() {
                let actual = owner.view().model_selection();
                let mut work = |units| {
                    owner.charge_reference_work(
                        crate::model::accounting::NormalizationChargePoint::ReferenceCatalog,
                        units,
                    )
                };
                owner
                    .charge_reference_work(
                        crate::model::accounting::NormalizationChargePoint::ReferenceCatalog,
                        1,
                    )
                    .map_err(|incomplete| RetainedModelFailure::Normalization {
                        owner: index,
                        incomplete,
                    })?;
                if crate::model::key::text_equal_with_work(
                    &actual.identity,
                    &requested.identity,
                    &mut work,
                )
                .map_err(|incomplete| RetainedModelFailure::Normalization {
                    owner: index,
                    incomplete,
                })? {
                    if selected.is_some() {
                        return Err(RetainedModelFailure::Selection { selection });
                    }
                    selected = Some(index);
                }
            }
            let index = selected.ok_or(RetainedModelFailure::Selection { selection })?;
            for previous in &views {
                retain_charge(
                    budget,
                    NativeJoinChargeKind::Work,
                    1,
                    NativeJoinLocusSource::PreparationSelection(selection),
                )?;
                if std::ptr::eq::<EffectiveView>(&previous.view, owners[index].view()) {
                    return Err(RetainedModelFailure::Selection { selection });
                }
            }
            let original = owners[index].normalization_limits();
            match limits {
                Some((first_owner, first)) if first.ancestor_steps != original.ancestor_steps => {
                    return Err(RetainedModelFailure::IncompatiblePolicy {
                        first_owner,
                        first_ancestor_steps: first.ancestor_steps,
                        owner: index,
                        ancestor_steps: original.ancestor_steps,
                    });
                }
                Some(_) => {}
                None => limits = Some((index, original)),
            }
            let view = ModelView::retained(owners.clone(), index)
                .ok_or(RetainedModelFailure::Selection { selection })?;
            views.push(view);
        }
        Ok(Self {
            views,
            types,
            limits: limits.map_or_else(ModelNormalizationLimits::default, |(_, limits)| limits),
        })
    }

    /// Compare complete native type identities under the caller's work budget.
    ///
    /// # Errors
    /// Returns the first work denial unchanged, before the denied comparison.
    pub fn reference_identity_equal_with_work<E>(
        left: &EffectiveId,
        right: &EffectiveId,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<bool, E> {
        crate::model::key::effective_equal_with_work(left, right, work)
    }

    /// Whether a selected domain package declares an operation member.
    pub fn declares_operations(&self) -> bool {
        self.views.iter().any(|view| {
            view.view
                .domain_package()
                .records
                .iter()
                .any(|record| matches!(record, DomainPackageRecord::OperationMember(_)))
        })
    }

    /// FR-940: the operations the selected domain packages declare, typed as
    /// S3's assembly types them ([`crate::check::model_operation_table`]),
    /// each type naming a unit of `units`. The unit's own source package
    /// (FR-569) is not a domain package and contributes none.
    ///
    /// # Errors
    ///
    /// As [`crate::check::model_operation_table`].
    pub fn operation_table(
        &self,
        units: &BTreeMap<String, filament_exact::UnitId>,
    ) -> Result<crate::model::operation::OperationTable, crate::check::AssemblyRefusal> {
        crate::check::model_operation_table(self.views.iter().map(|view| &view.view), units)
    }

    /// FR-940: the one population domain of the selected domain packages
    /// that covers `object` by conformance, as S3 reads a state clause's
    /// context population (FR-104 "Resolution"), before the unit's authored
    /// bounds (FR-380) are applied: `Ok(None)` when none does, and the
    /// populations' keys when several do.
    ///
    /// # Errors
    ///
    /// The covering populations' declaration keys, ascending, when more
    /// than one covers `object`.
    pub fn covering_population(
        &self,
        object: filament_exact::EffectiveId,
    ) -> Result<Option<crate::check::PopulationDomain>, Vec<filament_checked::key::DeclarationKey>>
    {
        let models: Vec<crate::check::lowering::AdmittedModel> = self
            .views
            .iter()
            .filter(|view| !view.is_source())
            .map(|view| crate::check::lowering::AdmittedModel::from_view(&view.view))
            .collect();
        crate::check::state_clause::population_of(&models, object, &self.types)
    }

    /// FR-940: the population domains of the selected domain packages whose
    /// member types overlap `object`, as S3 reads them for a temporal
    /// clause's claim over `object` (FR-336), before the unit's authored
    /// bounds (FR-380) are applied.
    pub fn overlapping_populations(
        &self,
        object: filament_exact::EffectiveId,
    ) -> Vec<crate::check::PopulationDomain> {
        let models: Vec<crate::check::lowering::AdmittedModel> = self
            .views
            .iter()
            .filter(|view| !view.is_source())
            .map(|view| crate::check::lowering::AdmittedModel::from_view(&view.view))
            .collect();
        crate::check::state_clause::populations_overlapping(&models, object, &self.types)
    }

    /// Re-admit and re-normalize the domain packages `model_selections` name
    /// against `packages`, as FR-106 admission does.
    ///
    /// # Errors
    ///
    /// An [`AdmissionFailure`] when the packages do not re-normalize within
    /// `limits`.
    pub fn new<'a>(
        model_selections: impl Into<ModelSelections<'a>>,
        types: &TypeEnvironment,
        packages: &PackageInput,
        limits: ModelNormalizationLimits,
    ) -> Result<Self, AdmissionFailure> {
        Ok(Self {
            views: model_views(model_selections, packages, limits)?,
            types: StateTypes::Owned(types.clone()),
            limits,
        })
    }

    /// The model of a package still being checked, over the effective views
    /// its `model` declarations admitted. The views stand for the same
    /// packages [`Self::new`] re-admits from their bytes.
    pub(crate) fn from_views(
        views: impl IntoIterator<Item = EffectiveView>,
        types: &TypeEnvironment,
        limits: ModelNormalizationLimits,
    ) -> Self {
        Self {
            views: views.into_iter().map(ModelView::new).collect(),
            types: StateTypes::Owned(types.clone()),
            limits,
        }
    }

    /// Admit the `filament.state.snapshot/v1` document `selected` names in
    /// `provision` as an initial state: FR-106 check 1 (read), check 3 (an
    /// `observation` of `current` anchored by an `initialization`), check 4
    /// (model), check 6 (populations and values), check 7 (every population
    /// the document lists is required complete) and check 8 (closure).
    ///
    /// # Errors
    ///
    /// The first [`AdmissionFailure`] in check order.
    pub fn admit_initial(
        &self,
        provision: &BTreeMap<DocumentRef, Vec<u8>>,
        selected: &DocumentRef,
        limits: ObservationLimits,
    ) -> Result<(ModelState, SelectedAnchor), AdmissionFailure> {
        admit_initial_state(
            &self.views,
            &self.types,
            provision,
            selected,
            limits,
            self.limits.ancestor_steps,
        )
    }

    /// Admit a multi-model initial snapshot for this model's subject.
    /// Original bytes pass native read, duplicate-member and resource checks
    /// before another selected model's declared populations are excluded.
    /// Unknown populations remain subject to ordinary admission.
    ///
    /// # Errors
    /// The same ordered admission failures as [`Self::admit_initial`].
    pub fn admit_initial_scoped(
        &self,
        all_models: &Self,
        provision: &BTreeMap<DocumentRef, Vec<u8>>,
        selected: &DocumentRef,
        limits: ObservationLimits,
    ) -> Result<(ModelState, SelectedAnchor), AdmissionFailure> {
        crate::model::observation::admit_initial_state_scoped(
            &self.views,
            Some(&all_models.views),
            &self.types,
            provision,
            selected,
            limits,
            self.limits.ancestor_steps,
        )
    }

    /// The `min`/`max` bounds on the quantities each member of the
    /// selected models holds (the `member_bounds` of its views).
    pub fn member_bounds(&self) -> Vec<MemberBound> {
        member_bounds(&self.views)
    }

    /// Each selected record value type's field names in declaration order
    /// (each field's `position`), by the record's node key: the order of its
    /// components in an enumerated value domain (FR-120, FR-215). The
    /// record's slots stay name-sorted.
    pub fn record_orders(&self) -> BTreeMap<filament_exact::NodeKey, Vec<String>> {
        let mut orders = BTreeMap::new();
        for view in &self.views {
            let records = &view.view.domain_package().records;
            for record in records {
                let DomainPackageRecord::RecordValueType(declared) = record else {
                    continue;
                };
                let Ok(Some(node)) = crate::check::record_node_key(record) else {
                    continue;
                };
                let mut fields: Vec<(usize, &str)> = records
                    .iter()
                    .filter_map(|record| match record {
                        DomainPackageRecord::FieldMember(field) if field.owner == declared.key => {
                            crate::model::intake::member_identity_name(
                                &declared.key.node,
                                &field.key.node,
                            )
                            .map(|name| (field.position, name))
                        }
                        _ => None,
                    })
                    .collect();
                fields.sort_unstable();
                orders.entry(node).or_insert_with(|| {
                    fields
                        .into_iter()
                        .map(|(_, name)| name.to_owned())
                        .collect()
                });
            }
        }
        orders
    }

    /// Each model tuple's authored positions mapped to its data slots.
    /// Clock positions have no data slot, including aliases of Clock.
    pub fn tuple_data_positions(&self) -> BTreeMap<filament_exact::NodeKey, Vec<Option<usize>>> {
        let mut positions = BTreeMap::new();
        for view in &self.views {
            let records = &view.view.domain_package().records;
            for record in records {
                let DomainPackageRecord::TupleValueType(tuple) = record else {
                    continue;
                };
                let Ok(Some(node)) = crate::check::record_node_key(record) else {
                    continue;
                };
                positions.insert(
                    node,
                    crate::model::domain_package::tuple_data_projection(records, &tuple.key),
                );
            }
        }
        positions
    }

    /// Declaration row count and identity-byte size for pre-admitting atomic observation validation.
    /// The callback admits each traversal step and may stop or cancel it. Saturation
    /// reports the largest size, so the caller must refuse an unrepresentable cost.
    ///
    /// # Errors
    /// Returns the callback's actual admission failure without validation or allocation.
    pub fn observation_work_size<E>(
        &self,
        mut admit: impl FnMut(usize) -> Result<(), E>,
    ) -> Result<(usize, usize), E> {
        let size = |key: &DeclarationKey| {
            key.package
                .len()
                .saturating_add(key.node.len())
                .saturating_add(1)
        };
        let mut rows = self.views.len();
        let mut units = 0usize;
        for view in &self.views {
            admit(1)?;
            let records = &view.view.domain_package().records;
            admit(records.len())?;
            rows = rows.saturating_add(records.len());
            for record in records {
                match record {
                    DomainPackageRecord::Relationship(row) => {
                        units = units
                            .saturating_add(size(&row.key))
                            .saturating_add(size(&row.source.type_identity))
                            .saturating_add(size(&row.target.type_identity));
                    }
                    DomainPackageRecord::Population(row) => {
                        units = units.saturating_add(size(&row.key));
                        for member in &row.member_types {
                            admit(1)?;
                            rows = rows.saturating_add(1);
                            units = units.saturating_add(size(member));
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok((rows, units))
    }

    /// The [`Observation`] of `state`, named `identity`: the object
    /// environment of every object with every field value, and every listed
    /// population complete.
    ///
    /// # Preconditions
    ///
    /// `state` must have been produced during this model's current run or
    /// already admitted for that run. This trusted Rust interface does not
    /// authenticate caller-constructed Clock references from another model
    /// or run; handcrafted Clock references are outside its contract. Raw
    /// transport inputs must use document admission.
    ///
    /// # Errors
    ///
    /// A `Fault` when `state` is not a state of this model.
    pub fn observation(
        &self,
        state: &ModelState,
        identity: DocumentRef,
    ) -> Result<Observation, AdmissionFailure> {
        observe_state(&self.views, &self.types, state, identity)
    }

    /// Observe the complete simulated state with the population authority
    /// required to admit protocol reference arguments. All listed populations
    /// are complete and their whole object closure is validated together.
    ///
    /// # Preconditions
    ///
    /// `state` must have been produced during this model's current run or
    /// already admitted for that run. This trusted Rust interface does not
    /// authenticate caller-constructed Clock references from another model
    /// or run; handcrafted Clock references are outside its contract. Raw
    /// transport inputs must use document admission.
    ///
    /// # Errors
    /// The same state/model invariant failures as [`Self::observation`].
    pub fn protocol_observation(
        &self,
        state: &ModelState,
        identity: DocumentRef,
    ) -> Result<Observation, AdmissionFailure> {
        crate::model::observation::observe_protocol_state(&self.views, &self.types, state, identity)
    }

    /// The model index `population`'s declarations resolve in, for the
    /// [`FrameDecision`](crate::model::population::FrameDecision) over that
    /// population; `None` when no selected model declares it.
    pub fn population_index(&self, population: &DeclarationKey) -> Option<&ModelIndex> {
        self.views
            .iter()
            .find(|view| view.view.population(population).is_some())
            .map(|view| view.view.model_index())
    }

    /// The conformance walk's `ancestor_steps` ceiling, for
    /// [`FrameDecision::ancestor_steps`](crate::model::population::FrameDecision).
    pub fn ancestor_steps(&self) -> u64 {
        self.limits.ancestor_steps
    }

    /// Every population of the selected models, in ascending declaration
    /// key order.
    pub fn populations(&self) -> Vec<StatePopulation> {
        let mut populations: Vec<StatePopulation> = self
            .views
            .iter()
            .flat_map(|view| view.view.domain_package().records.iter())
            .filter_map(|record| match record {
                DomainPackageRecord::Population(population) => Some(StatePopulation {
                    key: population.key.clone(),
                    member_types: population.member_types.clone(),
                    symmetric: population.symmetric,
                }),
                _ => None,
            })
            .collect();
        populations.sort_by(|left, right| left.key.cmp(&right.key));
        populations
    }

    /// The effective identity of the declared type `key`.
    pub fn type_identity(&self, key: &DeclarationKey) -> Option<EffectiveId> {
        self.views
            .iter()
            .find_map(|view| view.view.type_identities().get(key).cloned())
    }

    /// Borrow an admitted type identity, charging each model-index visit and
    /// declaration text comparison before it occurs.
    ///
    /// # Errors
    /// The caller's original work or cancellation refusal.
    pub fn type_identity_with_work<E>(
        &self,
        key: &DeclarationKey,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Option<&EffectiveId>, E> {
        for view in &self.views {
            work(1)?;
            for (candidate, effective) in view.view.type_identities() {
                work(1)?;
                if crate::model::key::declaration_equal_with_work(candidate, key, work)? {
                    return Ok(Some(effective));
                }
            }
        }
        Ok(None)
    }

    /// The declaration key of the type whose effective identity is
    /// `identity`.
    pub fn type_key(&self, identity: EffectiveId) -> Option<&DeclarationKey> {
        self.views
            .iter()
            .find_map(|view| view.declaration_key(identity.clone()))
    }

    /// The original operation member declared by `owner` under `name`.
    /// This borrows the retained normalized domain records; it creates no
    /// declaration, type identity or operation binding.
    pub fn operation_declaration(
        &self,
        owner: &DeclarationKey,
        name: &str,
    ) -> Option<&crate::model::domain_package::OperationMemberRecord> {
        let mut found = self
            .views
            .iter()
            .flat_map(|view| &view.view.domain_package().records)
            .filter_map(|record| match record {
                DomainPackageRecord::OperationMember(operation)
                    if &operation.owner == owner
                        && crate::model::intake::member_identity_name(
                            &owner.node,
                            &operation.key.node,
                        ) == Some(name) =>
                {
                    Some(operation)
                }
                _ => None,
            });
        let first = found.next()?;
        found.next().is_none().then_some(first)
    }

    /// Whether the object type `key` is declared abstract.
    pub fn is_abstract(&self, key: &DeclarationKey) -> bool {
        self.views
            .iter()
            .any(|view| view.view.model_index().is_abstract(key))
    }

    /// The fields of the object type `type_key`, in its effective attribute
    /// order; `None` when no selected model declares it.
    ///
    /// # Errors
    ///
    /// A `Fault` when a field's owner has no declaration key.
    pub fn fields(
        &self,
        type_key: &DeclarationKey,
    ) -> Result<Option<Vec<StateField>>, AdmissionFailure> {
        state_fields(&self.views, &self.types, type_key)
    }

    /// The reference to the object `object` of the type `type_key`, in the
    /// universe admission assigns it.
    ///
    /// # Errors
    ///
    /// A `Refused` record when `object` is empty, and a `Fault` when no
    /// selected model declares the type.
    pub fn object_reference(
        &self,
        type_key: &DeclarationKey,
        object: &str,
    ) -> Result<ObjectReference, AdmissionFailure> {
        state_reference(&self.views, type_key, object)
    }

    /// Borrow the effective object type and its normalized object universe
    /// that [`Self::object_reference`] uses, without constructing a reference.
    /// The first matching model view and its validated reverse identity index
    /// are selected in the same order as reference admission.
    ///
    /// `None` means the type or its universe cannot be resolved. The caller
    /// remains responsible for validating the object's nonempty identity.
    pub fn reference_components(
        &self,
        type_key: &DeclarationKey,
    ) -> Option<(&EffectiveId, &crate::model::normalize::ObjectUniverse)> {
        match self.reference_components_with_work(type_key, &mut |_| {
            Ok::<(), std::convert::Infallible>(())
        }) {
            Ok(value) => value,
            Err(never) => match never {},
        }
    }

    /// The same validated reference components with admission before each
    /// index visit and compared text byte. No identity is copied. The lookup
    /// retains the first-view and validated reverse-index choices.
    ///
    /// # Errors
    /// The callback's work or cancellation denial.
    pub fn reference_components_with_work<E>(
        &self,
        type_key: &DeclarationKey,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Option<(&EffectiveId, &crate::model::normalize::ObjectUniverse)>, E> {
        let mut effective = None;
        'views: for view in &self.views {
            work(1)?;
            for (candidate, value) in view.view.type_identities() {
                work(1)?;
                if crate::model::key::declaration_equal_with_work(candidate, type_key, work)? {
                    effective = Some(value);
                    break 'views;
                }
            }
        }
        let Some(effective) = effective else {
            return Ok(None);
        };
        for view in &self.views {
            work(1)?;
            if let Some(declaration) = view.declaration_key_with_work(effective, work)? {
                return Ok(view
                    .view
                    .object_universe_with_work(declaration, work)?
                    .map(|universe| (effective, universe)));
            }
        }
        Ok(None)
    }

    /// Resolve an admitted effective type to its actual declaring model node.
    /// Every owner-chain comparison is charged through the caller's work hook.
    ///
    /// # Errors
    /// Returns the work hook's first refusal unchanged.
    pub fn declaration_key_with_work<E>(
        &self,
        effective: &EffectiveId,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Option<&DeclarationKey>, E> {
        for view in &self.views {
            work(1)?;
            if let Some(key) = view.declaration_key_with_work(effective, work)? {
                return Ok(Some(key));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod retained_owner_tests {
    use super::*;
    use crate::model::accounting::{NormalizationChargePoint, NormalizationLimitKind};
    use crate::model::domain_package::DomainPackageRef;
    use crate::model::intake::package_input;
    use crate::model::observation::admit_current_snapshot_with_provenance;
    use ix_trace_rs::trace;
    use serde_json::json;

    fn selection(identity: &str) -> DomainPackageRef {
        DomainPackageRef {
            identity: identity.to_owned(),
        }
    }

    fn owner(identity: &str, limits: ModelNormalizationLimits) -> NormalizedModel {
        let document = include_str!("../../../examples/config-version/model.semantic-ir.json")
            .replace("example/config-version", identity);
        let packages = package_input([document.as_bytes()]).expect("real model document admits");
        let (model_selection, document) = crate::model::intake::admit(
            identity,
            &packages,
            fsl_foundation::IntakeLimits::default(),
        )
        .unwrap();
        let package =
            crate::model::intake::read_domain_package(model_selection, &document).unwrap();
        let crate::model::normalize::NormalizeOutcome::Completed(view) =
            crate::model::normalize::normalize(&package, ModelNormalizationLimits::default())
        else {
            panic!("fixture view normalizes for its checked type environment")
        };
        let types =
            crate::check::model_type_environment("M", &view).expect("fixture types assemble");
        let selected = DocumentRef {
            authority: "test".into(),
            identity: "current".into(),
        };
        let snapshot = json!({
            "format": "filament.state.snapshot/v1",
            "identity": {"authority": "test", "identity": "current"},
            "observation": "current", "model": {"identity": identity},
            "populations": [{"population": format!("ix://{identity}/config_history"),
                "complete": true, "objects": []}]
        });
        let (_, _, mut owners) = admit_current_snapshot_with_provenance(
            &[selection(identity)],
            &types,
            &packages,
            limits,
            &BTreeMap::from([(selected.clone(), snapshot.to_string().into_bytes())]),
            &selected,
            ObservationLimits::default(),
        )
        .expect("real snapshot producer retains its model");
        assert_eq!(owners.len(), 1);
        owners.remove(0)
    }

    #[trace("FR-940-AC-10")]
    #[test]
    fn retained_owner_contexts_borrow_the_same_payload_reverse_index_and_counter() {
        let models = vec![
            owner(
                "test/first",
                ModelNormalizationLimits::default().with_ancestor_steps(4),
            ),
            owner(
                "test/second",
                ModelNormalizationLimits::default().with_ancestor_steps(4),
            ),
        ];
        let records = models[1].view().domain_package().records.as_ptr();
        let exact_rows = models[1].exact_declaration_rows().as_ptr();
        let before = models[1].consumed(NormalizationLimitKind::WorkUnits);
        let mut budget = NativeJoinBudget::new(u64::MAX, u64::MAX);
        let owners = StateModel::retain_normalized_models(models, &mut budget).unwrap();
        let types = Arc::new(crate::check::model_type_environment("M", owners[0].view()).unwrap());
        let selected = [selection("test/second"), selection("test/first")];
        let first = StateModel::from_normalized_models_shared_types(
            &selected,
            &types,
            &owners,
            &mut budget,
        )
        .unwrap();
        let second = StateModel::from_normalized_models_shared_types(
            &selected,
            &types,
            &owners,
            &mut budget,
        )
        .unwrap();
        assert_eq!(first.views.len(), 2);
        assert_eq!(second.views.len(), 2);
        assert!(std::ptr::eq(&*first.views[0], &owners[1].data));
        assert!(std::ptr::eq(&*first.views[1], &owners[0].data));
        assert!(std::ptr::eq(&*second.views[0], &owners[1].data));
        assert!(std::ptr::eq(&*second.views[1], &owners[0].data));
        assert_eq!(first.ancestor_steps(), 4);
        assert_eq!(
            first.ancestor_steps(),
            owners[0].view().effective_limits().ancestor_steps
        );
        assert!(std::ptr::eq::<TypeEnvironment>(&*first.types, &*types));
        assert!(std::ptr::eq::<TypeEnvironment>(&*second.types, &*types));
        let legacy = StateModel::from_views(
            [owners[0].view().clone()],
            &types,
            *owners[0].view().effective_limits(),
        );
        assert!(!std::ptr::eq::<TypeEnvironment>(&*legacy.types, &*types));
        assert_eq!(first.views[0].view.model_selection(), &selected[0]);
        assert_eq!(
            first.views[0].view.domain_package().records.as_ptr(),
            records
        );
        assert_eq!(owners[1].exact_declaration_rows().as_ptr(), exact_rows);
        let (key, effective) = first.views[0].view.type_identities().iter().next().unwrap();
        let first_key = first.views[0].declaration_key_borrowed(effective).unwrap();
        let second_key = second.views[0].declaration_key_borrowed(effective).unwrap();
        assert_eq!(first_key, key);
        assert!(std::ptr::eq(first_key, second_key));
        assert!(owners[1].consumed(NormalizationLimitKind::WorkUnits) > before);
        let consumed = owners[1].consumed(NormalizationLimitKind::WorkUnits);
        let other_consumed = owners[0].consumed(NormalizationLimitKind::WorkUnits);
        owners[1]
            .charge_reference_work(NormalizationChargePoint::ReferenceResult, 7)
            .unwrap();
        assert_eq!(
            owners[1].consumed(NormalizationLimitKind::WorkUnits),
            consumed + 7
        );
        assert_eq!(
            owners[0].consumed(NormalizationLimitKind::WorkUnits),
            other_consumed
        );
        assert!(std::ptr::eq(&*first.views[0], &*second.views[0]));
    }

    #[trace("FR-940-AC-10")]
    #[test]
    fn retained_owner_structural_policy_refuses_mixed_original_bounds_exactly() {
        let mut budget = NativeJoinBudget::new(u64::MAX, u64::MAX);
        let owners = StateModel::retain_normalized_models(
            vec![
                owner(
                    "test/first",
                    ModelNormalizationLimits::default().with_ancestor_steps(4),
                ),
                owner(
                    "test/second",
                    ModelNormalizationLimits::default().with_ancestor_steps(2),
                ),
            ],
            &mut budget,
        )
        .unwrap();
        let result = StateModel::from_normalized_models(
            &[selection("test/second"), selection("test/first")],
            TypeEnvironment::default(),
            &owners,
            &mut budget,
        );
        let Err(RetainedModelFailure::IncompatiblePolicy {
            first_owner,
            first_ancestor_steps,
            owner,
            ancestor_steps,
        }) = result
        else {
            panic!("mixed original structural policies must refuse explicitly")
        };
        assert_eq!(
            (first_owner, first_ancestor_steps, owner, ancestor_steps),
            (1, 2, 0, 4)
        );
        assert_eq!(budget.first_unpaid(), None);
        assert_eq!(owners[0].view().effective_limits().ancestor_steps, 4);
        assert_eq!(owners[1].view().effective_limits().ancestor_steps, 2);
    }

    #[trace("FR-940-AC-10")]
    #[test]
    fn retained_owner_selection_refuses_missing_repeated_and_ambiguous_owners() {
        let mut budget = NativeJoinBudget::new(u64::MAX, u64::MAX);
        let owners = StateModel::retain_normalized_models(
            vec![owner("test/model", ModelNormalizationLimits::default())],
            &mut budget,
        )
        .unwrap();
        for selected in [
            vec![selection("test/missing")],
            vec![selection("test/model"), selection("test/model")],
        ] {
            assert!(matches!(
                StateModel::from_normalized_models(
                    &selected,
                    TypeEnvironment::default(),
                    &owners,
                    &mut budget
                ),
                Err(RetainedModelFailure::Selection { .. })
            ));
        }
        let ambiguous = StateModel::retain_normalized_models(
            vec![
                owner("test/model", ModelNormalizationLimits::default()),
                owner("test/model", ModelNormalizationLimits::default()),
            ],
            &mut budget,
        )
        .unwrap();
        assert!(matches!(
            StateModel::from_normalized_models(
                &[selection("test/model")],
                TypeEnvironment::default(),
                &ambiguous,
                &mut budget
            ),
            Err(RetainedModelFailure::Selection { selection: 0 })
        ));
    }

    #[trace("FR-940-AC-10")]
    #[test]
    fn retained_owner_native_reservation_denial_precedes_model_inspection() {
        let mut budget = NativeJoinBudget::new(100, 0);
        let error = StateModel::retain_normalized_models(Vec::new(), &mut budget).unwrap_err();
        let RetainedModelFailure::Native {
            denied,
            locus,
            configured_bound,
        } = error
        else {
            panic!("native storage denial expected")
        };
        assert_eq!(denied.kind, NativeJoinChargeKind::Storage);
        assert_eq!(
            denied.amount,
            (std::mem::size_of::<Vec<NormalizedModel>>() + 2 * std::mem::size_of::<usize>())
                as u128
        );
        assert_eq!(configured_bound, 0);
        assert_eq!(Some(denied), budget.first_unpaid());
        assert_eq!(locus, budget.first_unpaid_locus());
        let owners = Arc::new(vec![owner(
            "test/model",
            ModelNormalizationLimits::default(),
        )]);
        let consumed = owners[0].consumed(NormalizationLimitKind::WorkUnits);
        let result = StateModel::from_normalized_models(
            &[selection("test/model")],
            TypeEnvironment::default(),
            &owners,
            &mut budget,
        );
        let Err(RetainedModelFailure::Native {
            denied: retained_denied,
            locus: retained_locus,
            configured_bound: retained_bound,
        }) = result
        else {
            panic!("original native denial must remain first")
        };
        assert_eq!(retained_denied, denied);
        assert_eq!(retained_locus, locus);
        assert_eq!(retained_bound, configured_bound);
        assert_eq!(
            owners[0].consumed(NormalizationLimitKind::WorkUnits),
            consumed
        );
        let mut work_short = NativeJoinBudget::new(0, u64::MAX);
        let result = StateModel::from_normalized_models(
            &[selection("test/model")],
            TypeEnvironment::default(),
            &owners,
            &mut work_short,
        );
        let Err(RetainedModelFailure::Native {
            denied,
            configured_bound,
            ..
        }) = result
        else {
            panic!("wrapper work must be admitted before allocating its vector")
        };
        assert_eq!(denied.kind, NativeJoinChargeKind::Work);
        assert_eq!(denied.amount, 1);
        assert_eq!(configured_bound, 0);
        assert_eq!(work_short.consumed().storage, 0);
        assert_eq!(
            owners[0].consumed(NormalizationLimitKind::WorkUnits),
            consumed
        );
    }

    #[trace("FR-940-AC-10")]
    #[test]
    fn retained_owner_reference_denial_keeps_the_original_limit_and_consumption() {
        let baseline = owner("test/model", ModelNormalizationLimits::default())
            .consumed(NormalizationLimitKind::WorkUnits);
        let original = ModelNormalizationLimits::default().with_work_units(baseline + 26);
        let mut budget = NativeJoinBudget::new(u64::MAX, u64::MAX);
        let owners =
            StateModel::retain_normalized_models(vec![owner("test/model", original)], &mut budget)
                .unwrap();
        for _ in 0..2 {
            StateModel::from_normalized_models(
                &[selection("test/model")],
                TypeEnvironment::default(),
                &owners,
                &mut budget,
            )
            .expect("the original allowance admits two retained contexts");
        }
        let result = StateModel::from_normalized_models(
            &[selection("test/model")],
            TypeEnvironment::default(),
            &owners,
            &mut budget,
        );
        let Err(RetainedModelFailure::Normalization {
            owner: index,
            incomplete,
        }) = result
        else {
            panic!("original normalization owner must deny reference inspection")
        };
        assert_eq!(index, 0);
        assert_eq!(incomplete.limit, original.work_units);
        assert_eq!(incomplete.consumed, original.work_units);
        assert_eq!(incomplete.next_charge, 1);
        assert_eq!(
            incomplete.charge_point,
            NormalizationChargePoint::ReferenceCatalog
        );
        assert_eq!(
            owners[0].consumed(NormalizationLimitKind::WorkUnits),
            original.work_units
        );
    }
}
