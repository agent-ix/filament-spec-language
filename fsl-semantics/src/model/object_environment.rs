// SPDX-License-Identifier: AGPL-3.0-or-later
//! The object environment S6a evaluates over: SV's core object closure plus
//! FR-089's populations map (ADR-011 §6.2).
//!
//! The closure -- the admitted objects and the reference closure between
//! them, keyed by kernel ids -- is SV's
//! [`ObjectClosure`].
//! This module holds only what SV cannot: FR-089's recorded `PopulationId`
//! -> [`PopulationBinding`] correspondence. `PopulationBinding` is `model`'s
//! (ADR-011 §6.1, "K is a leaf"), and §6.1's order `semantic_value < model`
//! forbids the upward import, so the populations map stays in `model`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, PoisonError};

use crate::model::clause_observation::Observation;
use crate::model::observation::{OutOfRange, RelationshipExtents};
use crate::model::population::PopulationBinding;
use filament_checked::key::DeclarationKey;
use filament_exact::{EffectiveId, NodeKey, ObjectReference, PopulationId, UniverseId, Value};
use filament_semantic_value::admission::PathStep;
use filament_semantic_value::declaration::{FieldRef, TypeEnvironment};
use filament_semantic_value::object_closure::ObjectClosure;

/// The explicitly selected population of one reference occurrence. These
/// facts accompany values; constructing this record does not authenticate it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceOccurrence {
    /// The typed path from the value's root to this reference.
    pub path: Vec<PathStep>,
    /// The actual checked population declaration selected for the reference.
    pub population: NodeKey,
}

/// A typed subtree whose raw element positions cannot authenticate the
/// positions of the resulting canonical collection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnavailableReferenceOrigin {
    /// The affected subtree's original value-relative path.
    pub path: Vec<PathStep>,
    /// The existing collection construct that prevents positional attribution.
    pub collection: filament_exact::CollectionKind,
}

/// The original admitted value root, retained when helpers forward a value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReferenceOriginLocus {
    /// The explicitly selected receiver.
    Receiver,
    /// An operation parameter, by its admitted declaration name.
    Parameter(String),
    /// The operation result.
    Result,
    /// A field of an explicitly attributed population occurrence.
    ObjectField {
        /// Actual declared holder population.
        population: NodeKey,
        /// Holder's local key.
        owner: String,
        /// Actual declared field.
        field: FieldRef,
    },
    /// A field of the observation's explicitly attached state-machine context.
    ContextField {
        /// The context reference created by that observation's admission.
        owner: ObjectReference,
        /// Actual declared field.
        field: FieldRef,
    },
}

/// One position in the admission walk's flat typed-prefix arena.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ReferenceOriginPrefix(usize);

#[derive(Clone, Debug)]
struct OriginPrefix {
    parent: ReferenceOriginPrefix,
    step: PathStep,
}

/// The flat metadata built by the existing typed admission walk. Each
/// visited path step is retained once; occurrences share their ancestors.
#[derive(Debug, Default)]
pub struct ReferenceOriginBuilder {
    prefixes: Vec<OriginPrefix>,
    fields: BTreeMap<ReferenceOriginPrefix, BTreeMap<String, ReferenceOriginPrefix>>,
    options: BTreeMap<ReferenceOriginPrefix, ReferenceOriginPrefix>,
    indices: BTreeMap<ReferenceOriginPrefix, BTreeMap<usize, ReferenceOriginPrefix>>,
    references: BTreeMap<ReferenceOriginPrefix, NodeKey>,
    unavailable: BTreeMap<ReferenceOriginPrefix, filament_exact::CollectionKind>,
}

impl ReferenceOriginBuilder {
    /// The admitted value's root, before any typed path step.
    pub fn root(&self) -> ReferenceOriginPrefix {
        ReferenceOriginPrefix(0)
    }

    /// Enter the actual typed walk's next step. Repeated supplied paths
    /// reuse the same prefix; raw admission calls this once per entered step.
    pub fn enter(
        &mut self,
        parent: ReferenceOriginPrefix,
        step: PathStep,
    ) -> ReferenceOriginPrefix {
        if let Some(prefix) = self.child(parent, &step) {
            return prefix;
        }
        let prefix = ReferenceOriginPrefix(self.prefixes.len() + 1);
        match &step {
            PathStep::Field(name) => {
                self.fields
                    .entry(parent)
                    .or_default()
                    .insert(name.clone(), prefix);
            }
            PathStep::OptionValue => {
                self.options.insert(parent, prefix);
            }
            PathStep::Index(index) => {
                self.indices
                    .entry(parent)
                    .or_default()
                    .insert(*index, prefix);
            }
        }
        self.prefixes.push(OriginPrefix { parent, step });
        prefix
    }

    /// Retain the explicitly validated population of this occurrence.
    pub fn reference(&mut self, prefix: ReferenceOriginPrefix, population: NodeKey) {
        self.references.insert(prefix, population);
    }

    /// Mark only the actual canonicalized Set or Bag subtree unavailable.
    pub fn unavailable(
        &mut self,
        prefix: ReferenceOriginPrefix,
        kind: filament_exact::CollectionKind,
    ) {
        self.unavailable.insert(prefix, kind);
    }

    fn child(
        &self,
        parent: ReferenceOriginPrefix,
        step: &PathStep,
    ) -> Option<ReferenceOriginPrefix> {
        match step {
            PathStep::Field(name) => self.fields.get(&parent)?.get(name).copied(),
            PathStep::OptionValue => self.options.get(&parent).copied(),
            PathStep::Index(index) => self.indices.get(&parent)?.get(index).copied(),
        }
    }

    /// Publish one immutable backing without retaining empty ordinary values.
    pub fn finish(self) -> ReferenceOrigins {
        if self.references.is_empty() && self.unavailable.is_empty() {
            return ReferenceOrigins::default();
        }
        let root = self.root();
        let blocked = self.unavailable.get(&root).copied();
        ReferenceOrigins {
            backing: Some(Arc::new(self)),
            prefix: root,
            blocked,
            locus: None,
        }
    }
}

/// Immutable flat occurrence metadata shared by admission and call frames.
/// A selection holds a prefix index; Local reads and calls clone only Arc
/// handles. No value or wire identity is changed by this metadata.
#[derive(Clone, Debug)]
pub struct ReferenceOrigins {
    backing: Option<Arc<ReferenceOriginBuilder>>,
    prefix: ReferenceOriginPrefix,
    blocked: Option<filament_exact::CollectionKind>,
    locus: Option<Arc<ReferenceOriginLocus>>,
}

impl Default for ReferenceOrigins {
    fn default() -> Self {
        Self {
            backing: None,
            prefix: ReferenceOriginPrefix(0),
            blocked: None,
            locus: None,
        }
    }
}

type FieldOriginIndex = BTreeMap<NodeKey, BTreeMap<String, BTreeMap<FieldRef, ReferenceOrigins>>>;
type ContextOriginIndex = BTreeMap<ObjectReference, BTreeMap<FieldRef, ReferenceOrigins>>;

impl ReferenceOrigins {
    /// Move existing validated supplied paths into the shared prefix arena.
    pub fn from_parts(
        references: Vec<ReferenceOccurrence>,
        unavailable: Vec<UnavailableReferenceOrigin>,
    ) -> Self {
        let mut builder = ReferenceOriginBuilder::default();
        for occurrence in references {
            let mut prefix = builder.root();
            for step in occurrence.path {
                prefix = builder.enter(prefix, step);
            }
            builder.reference(prefix, occurrence.population);
        }
        for marker in unavailable {
            let mut prefix = builder.root();
            for step in marker.path {
                prefix = builder.enter(prefix, step);
            }
            builder.unavailable(prefix, marker.collection);
        }
        builder.finish()
    }

    /// Move an exact validated occurrence list into shared storage once.
    pub fn from_occurrences(references: Vec<ReferenceOccurrence>) -> Self {
        Self::from_parts(references, Vec::new())
    }

    /// Retain the admitted value root once, without copying occurrence paths.
    pub fn with_locus(mut self, locus: ReferenceOriginLocus) -> Self {
        if !self.is_empty() {
            self.locus = Some(Arc::new(locus));
        }
        self
    }

    /// Select one stable typed child through the flat edge index. Lookup is
    /// logarithmic in the recorded edges and does not copy ancestor paths.
    pub fn select(&self, step: &PathStep) -> Self {
        if self.blocked.is_some() {
            return self.clone();
        }
        let Some(backing) = &self.backing else {
            return self.clone();
        };
        let Some(prefix) = backing.child(self.prefix, step) else {
            return Self {
                locus: self.locus.clone(),
                ..Self::default()
            };
        };
        Self {
            backing: self.backing.clone(),
            prefix,
            blocked: backing.unavailable.get(&prefix).copied(),
            locus: self.locus.clone(),
        }
    }

    /// The exact population at the selected scalar reference, never one
    /// inferred from a reference's membership in an environment.
    pub fn population(&self) -> Option<&NodeKey> {
        if self.blocked.is_some() {
            return None;
        }
        self.backing.as_ref()?.references.get(&self.prefix)
    }

    /// The population at a known stable path below this selected value.
    pub fn population_at_path(&self, path: &[PathStep]) -> Option<&NodeKey> {
        if self.blocked.is_some() {
            return None;
        }
        let backing = self.backing.as_ref()?;
        let mut prefix = self.prefix;
        for step in path {
            prefix = backing.child(prefix, step)?;
            if backing.unavailable.contains_key(&prefix) {
                return None;
            }
        }
        backing.references.get(&prefix)
    }

    /// A Set or Bag ancestor preventing attribution of this selected owner.
    pub fn unavailable_selection(&self) -> Option<filament_exact::CollectionKind> {
        self.blocked
    }

    /// Number of exact facts in this value root's immutable backing.
    pub fn occurrence_count(&self) -> usize {
        self.backing
            .as_ref()
            .map_or(0, |backing| backing.references.len())
    }

    /// Number of shared typed prefixes retained for this admitted value root.
    pub fn prefix_count(&self) -> usize {
        self.backing
            .as_ref()
            .map_or(0, |backing| backing.prefixes.len())
    }

    /// Number of affected subtrees in this value root's immutable backing.
    pub fn unavailable_count(&self) -> usize {
        self.backing
            .as_ref()
            .map_or(0, |backing| backing.unavailable.len())
    }

    /// Reconstruct only a diagnostic path, by walking the selected prefix's
    /// actual depth. Normal selection and forwarding do not reconstruct it.
    pub fn selected_path(&self) -> Vec<PathStep> {
        let Some(backing) = &self.backing else {
            return Vec::new();
        };
        let mut prefix = self.prefix;
        let mut path = Vec::new();
        while let Some(index) = prefix.0.checked_sub(1) {
            let Some(node) = backing.prefixes.get(index) else {
                break;
            };
            path.push(node.step.clone());
            prefix = node.parent;
        }
        path.reverse();
        path
    }

    /// Original binding or field, independent of a helper's frame names.
    pub fn locus(&self) -> Option<&ReferenceOriginLocus> {
        self.locus.as_deref()
    }

    /// Whether the value carries no exact or unavailable origin metadata.
    pub fn is_empty(&self) -> bool {
        self.backing.is_none()
    }
}

/// [`ObjectEnvironment::with_population`]'s refusal: `population_id` is
/// already recorded under a binding that does not equal the one this call
/// tried to record.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("population {population_id:?} is already recorded under a different binding")]
pub struct PopulationConflict {
    /// The colliding [`PopulationId`].
    pub population_id: PopulationId,
}

/// The selected-model declaration facts needed to validate one explicitly
/// attributed protocol reference without inferring its population.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationProvenance {
    declaration: DeclarationKey,
    node: NodeKey,
    member_types: Vec<(EffectiveId, UniverseId)>,
}

impl PopulationProvenance {
    pub(crate) fn new(
        declaration: DeclarationKey,
        node: NodeKey,
        member_types: Vec<(EffectiveId, UniverseId)>,
    ) -> Self {
        Self {
            declaration,
            node,
            member_types,
        }
    }

    /// The exact population declaration key admitted by model normalization.
    pub fn declaration(&self) -> &DeclarationKey {
        &self.declaration
    }

    /// The canonical checked node for the population declaration.
    pub fn node(&self) -> &NodeKey {
        &self.node
    }

    /// Effective member types and their normalized universes.
    pub fn member_types(&self) -> &[(EffectiveId, UniverseId)] {
        &self.member_types
    }
}

/// An object environment: SV's closed object closure, plus FR-089's
/// recorded `PopulationId` -> `PopulationBinding` correspondence. `model`
/// mints a `PopulationId` at binding-admission time
/// (`admit_binding`/`admit_invocation`), and the caller records the pair
/// here, in the same environment already threaded through
/// `CheckedPackage::call`/`evaluate` and `Machine`, before constructing the
/// `Value::Population(population_id)` argument that names it.
#[derive(Clone, Debug, Default)]
pub struct ObjectEnvironment {
    /// Immutable admitted object data, shared by cloned observations.
    objects: Arc<ObjectClosure>,
    populations: BTreeMap<PopulationId, Arc<PopulationBinding>>,
    /// The post-state integers FR-106 check 6.5's witnessing admitted
    /// outside their declared range; empty for every other environment.
    out_of_range: Vec<OutOfRange>,
    unresolved_references: bool,
    /// The relationship instances this observation lists (FR-364); `None`
    /// for an environment no snapshot admitted.
    extents: Option<RelationshipExtents>,
    /// How many objects each population of the snapshot lists, by the
    /// population's declaration identity (FR-380); empty for an environment
    /// no snapshot admitted.
    population_sizes: BTreeMap<String, u64>,
    /// Every object the snapshot admitted, ascending; empty for an
    /// environment no snapshot admitted.
    references: Vec<ObjectReference>,
    /// The objects each population of the snapshot lists, by the
    /// population's declaration identity, in ascending key order (FR-139);
    /// empty for an environment no snapshot admitted.
    population_members: BTreeMap<String, Vec<ObjectReference>>,
    /// Whether the snapshot marks each population it lists `complete`, by
    /// the population's declaration identity (FR-106); empty for an
    /// environment no snapshot document admitted.
    population_completeness: BTreeMap<String, bool>,
    /// Exact declared population nodes and their normalized member types.
    population_provenance: BTreeMap<NodeKey, PopulationProvenance>,
    field_origins: Option<Arc<FieldOriginIndex>>,
    context_origins: Option<Arc<ContextOriginIndex>>,
    /// The FR-106 check-7 population closure rooted at each declared
    /// population, retaining the snapshot's exact reference provenance.
    population_reference_closures: BTreeMap<String, Vec<String>>,
    /// The history fields a refinement keeps for these objects, by object
    /// and field name (FR-138); empty for every other environment.
    history: BTreeMap<(ObjectReference, String), Value>,
    /// Where an evaluation over this environment records what it reads.
    read_log: Option<ReadLog>,
    /// The presence of each guarded clock place of a timed model's
    /// objects: by object, field and union case (FR-230 "Presence is
    /// derived"); empty outside a timed predicate's evaluation.
    place_presence: PlacePresences,
}

/// The presence of guarded clock places a timed predicate reads (FR-230
/// "Presence is derived").
#[derive(Clone, Debug, Default)]
pub struct PlacePresences {
    /// Each place outside collections, by the owning object, the field
    /// holding the place and the union case whose payload it is (`None` for
    /// an `Option<Clock>` field).
    pub places: BTreeMap<(ObjectReference, String, Option<String>), bool>,
    /// Each `Sequence<Option<Clock>>`'s element bits, by the owning object
    /// and the field, one for each element the collection holds.
    pub slots: BTreeMap<(ObjectReference, String), Vec<bool>>,
}

impl ObjectEnvironment {
    /// The environment over `objects`, with no population recorded yet.
    pub fn new(objects: ObjectClosure) -> Self {
        Self {
            objects: Arc::new(objects),
            populations: BTreeMap::new(),
            out_of_range: Vec::new(),
            unresolved_references: false,
            extents: None,
            population_sizes: BTreeMap::new(),
            references: Vec::new(),
            population_members: BTreeMap::new(),
            population_completeness: BTreeMap::new(),
            population_provenance: BTreeMap::new(),
            field_origins: None,
            context_origins: None,
            population_reference_closures: BTreeMap::new(),
            history: BTreeMap::new(),
            read_log: None,
            place_presence: PlacePresences::default(),
        }
    }

    /// Records the presence of each guarded clock place a timed predicate
    /// may read (FR-230 "Presence is derived").
    pub fn set_place_presence(&mut self, presence: PlacePresences) {
        self.place_presence = presence;
    }

    /// Whether the guarded clock place `field` (its union case `case`) of
    /// `object` is present, when recorded.
    pub fn place_presence(
        &self,
        object: &ObjectReference,
        field: &str,
        case: Option<&str>,
    ) -> Option<bool> {
        self.place_presence
            .places
            .get(&(object.clone(), field.to_owned(), case.map(str::to_owned)))
            .copied()
    }

    /// The presence bit of each element of the `Sequence<Option<Clock>>`
    /// `field` of `object`, when recorded.
    pub fn slot_presences(&self, object: &ObjectReference, field: &str) -> Option<&[bool]> {
        self.place_presence
            .slots
            .get(&(object.clone(), field.to_owned()))
            .map(Vec::as_slice)
    }

    /// Records the history fields a refinement keeps for these objects, so
    /// a refinement row that reads one finds its value (FR-138).
    #[must_use]
    pub fn with_history(mut self, history: BTreeMap<(ObjectReference, String), Value>) -> Self {
        self.history = history;
        self
    }

    /// The value of the history field `field` of `object`, when the
    /// refinement keeps one.
    pub fn history(&self, object: &ObjectReference, field: &str) -> Option<&Value> {
        self.history.get(&(object.clone(), field.to_owned()))
    }

    /// Records how many objects each population of the admitted snapshot
    /// lists, by the population's declaration identity (FR-380).
    #[must_use]
    pub fn with_population_sizes(mut self, sizes: BTreeMap<String, u64>) -> Self {
        self.population_sizes = sizes;
        self
    }

    /// How many objects the snapshot lists for the population whose
    /// declaration identity is `population`, when it lists that population.
    pub fn population_size(&self, population: &str) -> Option<u64> {
        self.population_sizes.get(population).copied()
    }

    /// Records the objects each population of the admitted snapshot lists,
    /// by the population's declaration identity and in ascending key order,
    /// so a refinement row that reads a whole population can enumerate it
    /// (FR-139).
    #[must_use]
    pub fn with_population_members(
        mut self,
        members: BTreeMap<String, Vec<ObjectReference>>,
    ) -> Self {
        self.population_members = members;
        self
    }

    /// The objects the snapshot lists for the population whose declaration
    /// identity is `population`, in ascending key order, when it lists that
    /// population.
    pub fn population_members(&self, population: &str) -> Option<&[ObjectReference]> {
        self.population_members.get(population).map(Vec::as_slice)
    }

    /// Records whether the admitted snapshot marks each population it lists
    /// `complete`, by the population's declaration identity (FR-106).
    #[must_use]
    pub fn with_population_completeness(mut self, completeness: BTreeMap<String, bool>) -> Self {
        self.population_completeness = completeness;
        self
    }

    /// Whether the snapshot lists the population whose declaration identity
    /// is `population` and marks it `complete`: what a clause naming it
    /// requires before it is read (FR-391 "Evaluation").
    pub fn population_complete(&self, population: &str) -> bool {
        self.population_completeness
            .get(population)
            .copied()
            .unwrap_or(false)
    }

    /// Every population's admitted completeness flag, keyed by its exact
    /// declaration identity (`package/node` form).
    pub fn population_completeness(&self) -> &BTreeMap<String, bool> {
        &self.population_completeness
    }

    /// Records the exact model declaration catalog for admitted populations.
    #[must_use]
    pub fn with_population_provenance(
        mut self,
        provenance: impl IntoIterator<Item = PopulationProvenance>,
    ) -> Self {
        self.population_provenance = provenance
            .into_iter()
            .map(|entry| (entry.node.clone(), entry))
            .collect();
        self
    }

    /// The admitted declaration facts for `node`, when it names a selected
    /// model population.
    pub fn population_provenance(&self, node: &NodeKey) -> Option<&PopulationProvenance> {
        self.population_provenance.get(node)
    }

    /// The admitted field's exact targets and unavailable subtrees, addressed
    /// by its explicitly declared holder population, local key and field.
    pub fn field_reference_origins(
        &self,
        population: &NodeKey,
        owner: &str,
        field: &FieldRef,
    ) -> Option<&ReferenceOrigins> {
        self.field_origins
            .as_ref()?
            .get(population)?
            .get(owner)?
            .get(field)
    }

    /// The field origins of the context explicitly attached by this snapshot.
    pub fn context_field_reference_origins(
        &self,
        owner: &ObjectReference,
        field: &FieldRef,
    ) -> Option<&ReferenceOrigins> {
        self.context_origins.as_ref()?.get(owner)?.get(field)
    }

    pub(crate) fn with_field_reference_origins(
        mut self,
        rows: impl IntoIterator<Item = (NodeKey, String, FieldRef, ReferenceOrigins)>,
    ) -> Self {
        let mut index: FieldOriginIndex = BTreeMap::new();
        for (population, owner, field, origins) in rows {
            if !origins.is_empty() {
                let origins = origins.with_locus(ReferenceOriginLocus::ObjectField {
                    population: population.clone(),
                    owner: owner.clone(),
                    field: field.clone(),
                });
                index
                    .entry(population)
                    .or_default()
                    .entry(owner)
                    .or_default()
                    .insert(field, origins);
            }
        }
        self.field_origins = (!index.is_empty()).then(|| Arc::new(index));
        self
    }

    pub(crate) fn with_context_reference_origins(
        mut self,
        rows: impl IntoIterator<Item = (ObjectReference, FieldRef, ReferenceOrigins)>,
    ) -> Self {
        let mut index: ContextOriginIndex = BTreeMap::new();
        for (owner, field, origins) in rows {
            if !origins.is_empty() {
                let origins = origins.with_locus(ReferenceOriginLocus::ContextField {
                    owner: owner.clone(),
                    field: field.clone(),
                });
                index.entry(owner).or_default().insert(field, origins);
            }
        }
        self.context_origins = (!index.is_empty()).then(|| Arc::new(index));
        self
    }

    /// The node key of a population by its exact declaration identity.
    pub fn population_node(&self, identity: &str) -> Option<&NodeKey> {
        self.population_provenance
            .iter()
            .find_map(|(node, provenance)| {
                (provenance.declaration.node == identity).then_some(node)
            })
    }

    /// Whether `universe` belongs to any population in the admitted model.
    pub fn has_population_universe(&self, universe: &UniverseId) -> bool {
        self.population_provenance.values().any(|provenance| {
            provenance
                .member_types
                .iter()
                .any(|(_, member_universe)| member_universe == universe)
        })
    }

    /// The universe selected for a checked member type in this observation.
    pub fn population_universe_for_type(&self, member_type: &EffectiveId) -> Option<&UniverseId> {
        self.population_provenance
            .values()
            .flat_map(|provenance| provenance.member_types.iter())
            .find_map(|(candidate, universe)| (candidate == member_type).then_some(universe))
    }

    /// Records the parsed check-7 closure rooted at each model population.
    #[must_use]
    pub fn with_population_reference_closures(
        mut self,
        closures: BTreeMap<String, Vec<String>>,
    ) -> Self {
        self.population_reference_closures = closures;
        self
    }

    /// The exact FR-106 check-7 closure rooted at `population`.
    pub fn population_reference_closure(&self, population: &str) -> Option<&[String]> {
        self.population_reference_closures
            .get(population)
            .map(Vec::as_slice)
    }

    /// Records every object the snapshot admitted, so an evaluation that
    /// ranges over the observation's objects (FR-366) can enumerate them.
    #[must_use]
    pub fn with_references(mut self, mut references: Vec<ObjectReference>) -> Self {
        references.sort();
        self.references = references;
        self
    }

    /// Every object the snapshot admitted, ascending by reference; empty
    /// for an environment no snapshot admitted.
    pub fn references(&self) -> &[ObjectReference] {
        &self.references
    }

    /// Records the relationship extents and the population traversal this
    /// environment's snapshot admitted (FR-364).
    #[must_use]
    pub fn with_extents(mut self, extents: RelationshipExtents) -> Self {
        self.extents = Some(extents);
        self
    }

    /// The relationship extents this environment holds, when a snapshot
    /// admitted it.
    pub fn extents(&self) -> Option<&RelationshipExtents> {
        self.extents.as_ref()
    }

    /// This environment with its references left unresolved: it holds no
    /// object, and a reference argument is admitted by its identity alone
    /// instead of being refused as dangling. A read through such a
    /// reference has no object to read, so it completes no value. A replay,
    /// which has no object environment of its own, runs under this
    /// (FR-098).
    #[must_use]
    pub fn with_unresolved_references(mut self) -> Self {
        self.unresolved_references = true;
        self
    }

    /// Whether references are left unresolved
    /// ([`Self::with_unresolved_references`]).
    pub fn references_are_unresolved(&self) -> bool {
        self.unresolved_references
    }

    /// Records the integers this environment's snapshot holds outside
    /// their declared range (FR-106 check 6.5), so evaluation can tell a
    /// witnessed value from a value that breaks the checked types.
    #[must_use]
    pub fn with_out_of_range(mut self, out_of_range: Vec<OutOfRange>) -> Self {
        self.out_of_range = out_of_range;
        self
    }

    /// Whether `reference`'s slot for `field` holds a witnessed
    /// out-of-range value (a scalar, or a sequence with a witnessed
    /// element).
    pub fn is_witnessed(
        &self,
        types: &TypeEnvironment,
        reference: &ObjectReference,
        field: &FieldRef,
    ) -> bool {
        self.witnesses(types, reference, field).next().is_some()
    }

    /// Whether this environment holds any FR-106 check 6.5 out-of-range
    /// witness, so its options and collections carry range-widened types.
    pub fn holds_out_of_range(&self) -> bool {
        !self.out_of_range.is_empty()
    }

    /// The out-of-range witnesses recorded for `reference`'s slot for
    /// `field`.
    pub fn witnesses<'a>(
        &'a self,
        types: &'a TypeEnvironment,
        reference: &'a ObjectReference,
        field: &'a FieldRef,
    ) -> impl Iterator<Item = &'a OutOfRange> + 'a {
        let attributes = types.attributes(reference.object_type().clone());
        self.out_of_range.iter().filter(move |witnessed| {
            witnessed.object == *reference
                && attributes.is_some_and(|attributes| {
                    attributes.iter().any(|attribute| {
                        attribute.identity() == witnessed.field && attribute.stands_for(field)
                    })
                })
        })
    }

    /// Records into `log` every field and population this environment
    /// serves to an evaluation.
    #[must_use]
    pub fn with_read_log(mut self, log: ReadLog) -> Self {
        self.read_log = Some(log);
        self
    }

    /// The log an evaluation over this environment records its reads in.
    pub fn read_log(&self) -> Option<&ReadLog> {
        self.read_log.as_ref()
    }

    /// The environment's core object closure.
    pub fn objects(&self) -> &ObjectClosure {
        &self.objects
    }

    /// Records `binding`'s admission under its own
    /// [`PopulationBinding::population_id`] -- FR-089's `PopulationId` ->
    /// `PopulationBinding` correspondence `model` mints at binding-admission
    /// time. Consuming builder: call once per admitted binding, before
    /// constructing the `Value::Population(population_id)` argument that
    /// names it, so [`Self::resolve_population`] can resolve it. Keyed by
    /// the binding's own id (never a separately supplied one), so a caller
    /// cannot record a binding under an id it does not carry.
    ///
    /// FR-089's own admission preimage (package, `population_key`, role --
    /// see `model::population::population_id_preimage`'s own doc) does not
    /// yet include the admitted document's content or its declared maximum,
    /// an open spec question. Two distinct
    /// bindings can therefore collide on one id within a single evaluation
    /// (for example, two invocations of the same population role with
    /// different declared maxima). This is the interim guard: recording an
    /// id already bound to an *equal* binding is `Ok` (idempotent
    /// re-admission, TC-291's own case), but recording a *different*
    /// binding under an id already bound refuses loudly, with
    /// [`PopulationConflict`], rather than silently letting the later
    /// admission overwrite the earlier one.
    pub fn with_population(
        mut self,
        binding: PopulationBinding,
    ) -> Result<Self, PopulationConflict> {
        let population_id = binding.population_id();
        if let Some(existing) = self.populations.get(&population_id) {
            if **existing != binding {
                return Err(PopulationConflict { population_id });
            }
            return Ok(self);
        }
        self.populations.insert(population_id, Arc::new(binding));
        Ok(self)
    }

    /// The `PopulationBinding` FR-089's recorded correspondence resolves
    /// `population_id` to, or `None` when `population_id` names no binding
    /// [`Self::with_population`] recorded in this environment
    /// (FR-089-AC-4: the evaluator refuses this case, never panicking or
    /// substituting a default binding).
    pub fn resolve_population(&self, population_id: PopulationId) -> Option<&PopulationBinding> {
        self.populations.get(&population_id).map(Arc::as_ref)
    }

    /// Borrow the actual admitted bindings retained by this environment.
    /// Consumers that account traversal charge before advancing the iterator.
    /// This does not select a population for a reference or reconstruct an id.
    pub fn population_bindings(&self) -> impl Iterator<Item = &PopulationBinding> {
        self.populations.values().map(Arc::as_ref)
    }

    /// The binding already admitted for an exact population declaration
    /// identity. This reads the recorded correspondence; it creates no id.
    pub fn population_binding(&self, identity: &str) -> Option<&PopulationBinding> {
        if let Some(provenance) = self
            .population_node(identity)
            .and_then(|node| self.population_provenance(node))
        {
            return self
                .populations
                .values()
                .find(|binding| binding.population_key() == provenance.declaration())
                .map(Arc::as_ref);
        }
        // A snapshot-admitted environment records no provenance: name the
        // binding by its declaration node.
        self.populations
            .values()
            .find(|binding| binding.population_key().node == identity)
            .map(Arc::as_ref)
    }
}

/// One read an evaluation made of an observation: a field of an object, or
/// the membership of objects in a population.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ObservedRead {
    /// A field of `object`, named by the attribute that stands for it in the
    /// object's effective attribute set: the type that owns it and its name.
    Field {
        /// The instant the read observed.
        observation: Observation,
        /// The object read.
        object: ObjectReference,
        /// The type owning the attribute read.
        owner: EffectiveId,
        /// The attribute's declared name.
        name: String,
    },
    /// The membership of a population: of `object` alone when the read
    /// resolves one reference, otherwise of every object.
    Membership {
        /// The instant the read observed.
        observation: Observation,
        /// The population read.
        population: DeclarationKey,
        /// The one object looked up, or `None` for a read of every member.
        object: Option<ObjectReference>,
    },
}

/// The reads an evaluation made, shared by every observation environment it
/// ran over.
#[derive(Clone, Debug, Default)]
pub struct ReadLog(Arc<Mutex<BTreeSet<ObservedRead>>>);

impl ReadLog {
    /// A log with no read in it.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `read`.
    pub fn record(&self, read: ObservedRead) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(read);
    }

    /// Every read recorded so far, in order.
    pub fn reads(&self) -> Vec<ObservedRead> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .cloned()
            .collect()
    }
}
