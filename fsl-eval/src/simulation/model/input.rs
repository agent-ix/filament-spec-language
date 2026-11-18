// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531, FR-940: [`ModelCheckInput`], what a model check reads of a
//! checked package, and the two routes that build it.
//!
//! A model check reads the package's declarations as tables (types,
//! operations, model selections, timed and probabilistic models, state
//! clauses, terminals) and evaluates its checked forms through one
//! [`ModelEvaluator`]. The source route builds the input from the source
//! package's `CheckedGraph` ([`ModelCheckInput::source`], FR-531). The package
//! route builds it once from a [`BarePackage`]'s admitted nodes
//! ([`PackageModelInput::of`], FR-940); it never builds a `CheckedGraph`, and
//! a node whose package form is not read yet refuses the input with a typed
//! [`PendingPackageForm`] rather than being left out.

use std::collections::BTreeSet;

use filament_eval::{CallFailure, Evaluation};
use filament_exact::{
    EffectiveId, Meter, NodeKey, ObjectReference, ScalarLimits, Value, ValueType,
};
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::declaration::TypeEnvironment;
use fsl_package::{
    BarePackage, CheckedNodeId, CheckedPackage, PackageTerminal, TerminalResolveRefusal,
    UnreadModelForm,
};
use fsl_semantics::check::footprint::{
    derive_atom_reads, derive_attempt_footprint_fallback, derive_constraint_reads,
    derive_footprint, derive_guard_reads, derive_node_reads, derive_selected_footprint,
    derive_slot_reads, FootprintRefusal, SymbolicFootprint, SymbolicLocation,
};
use fsl_semantics::check::{
    CheckedBody, CheckedHyperClause, CheckedNode, CheckedProbabilisticModel, CheckedProtocol,
    CheckedProtocolNodeKeys, CheckedRow, CheckedStateClause, CheckedTerminalPredicate, EnumBinding,
    FormulaNodeId, ModelTimeSource, Reward, StatePredicate, TimedModel, VariantTypeDeclaration,
    Written,
};
use fsl_semantics::model::clause_observation::Observation;
use fsl_semantics::model::domain_package::{DomainPackage, DomainPackageRef};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::observation::{AdmittedObservations, ModelSelections};
use fsl_semantics::model::operation::{OperationDeclaration, OperationTable};
use fsl_semantics::model::state::StateModel;

use crate::value::{
    CheckedPackageEvaluation, ClauseEvaluation, ClauseSetEvaluation, QualifiedName,
    WrittenCollection,
};

/// One state clause as a model check reads it: its name, node, context,
/// operation and observation (FR-107, FR-120 "Contract clauses").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateClauseEntry {
    /// The clause's declared name.
    pub name: String,
    /// The clause's node identity.
    pub identity: NodeKey,
    /// The object type the clause constrains.
    pub context: EffectiveId,
    /// The declaring type and name of the operation the clause belongs to;
    /// `None` for an invariant.
    pub operation: Option<(EffectiveId, String)>,
    /// Which state the clause observes: an invariant the current state, a
    /// precondition the pre-state, a postcondition the post-state.
    pub observation: Observation,
}

/// What a model check reads of a checked package (FR-531, FR-940): the
/// declaration tables and the evaluator of its checked forms.
#[derive(Clone)]
pub struct ModelCheckInput<'a> {
    /// The package's type environment.
    pub types: &'a TypeEnvironment,
    /// The package's variant types, which a value's canonical form names.
    pub variant_types: &'a [VariantTypeDeclaration],
    /// The package's enumerations, which a value's canonical form names.
    pub enums: &'a [EnumBinding],
    /// The operations each object type declares.
    pub operations: &'a OperationTable,
    /// Whether [`Self::operations`] was read: `false` on the package route,
    /// which reads no operation yet, so a model whose selected domain
    /// packages declare an operation is refused when its system is built.
    pub operations_read: bool,
    /// The domain packages the package selects, ascending by identity
    /// (FR-533).
    pub model_selections: &'a [DomainPackageRef],
    /// The source packages the unit's own `object` declarations build
    /// (FR-569), which have no document.
    pub source_packages: &'a [DomainPackage],
    /// The timed model of each time source, in time-source order (FR-230).
    pub timed_models: &'a [TimedModel],
    /// The model time sources (FR-230).
    pub time_sources: &'a [ModelTimeSource],
    /// The `random`, `reward` and `workload` declarations (FR-185).
    pub probabilistic: &'a CheckedProbabilisticModel,
    /// The state clauses, in the package's order.
    pub state_clauses: Vec<StateClauseEntry>,
    /// The `terminal` declarations (FR-411), or the refusal of their
    /// resolution, which the model system reports when it is built.
    pub terminals: Result<Vec<PackageTerminal<'a>>, TerminalResolveRefusal>,
    /// The evaluator of the package's checked forms.
    pub evaluator: &'a dyn ModelEvaluator,
}

impl<'a> ModelCheckInput<'a> {
    /// The source route's input (FR-531): every table read from the source
    /// package's `CheckedGraph`, and the package itself as the evaluator.
    pub fn source(package: &'a CheckedPackage) -> Self {
        let graph = package.graph();
        let scope = graph.scope();
        Self {
            types: scope.types(),
            variant_types: scope.variant_types(),
            enums: scope.enums(),
            operations: scope.operations(),
            operations_read: true,
            model_selections: graph.model_selections(),
            source_packages: graph.source_packages(),
            timed_models: graph.timed_models(),
            time_sources: graph.time_sources(),
            probabilistic: graph.probabilistic_model(),
            state_clauses: graph
                .state_clauses()
                .iter()
                .map(|clause| StateClauseEntry {
                    name: clause.name().to_owned(),
                    identity: clause.identity(),
                    context: clause.context(),
                    operation: clause.operation().map(|operation| {
                        (
                            operation.declaring.clone(),
                            operation.declaration.name().to_owned(),
                        )
                    }),
                    observation: clause.observation(),
                })
                .collect(),
            terminals: fsl_package::resolve_terminals(package),
            evaluator: package,
        }
    }

    /// The packages an admission of this package's observations
    /// re-normalizes: [`Self::model_selections`] and [`Self::source_packages`].
    pub fn model_inputs(&self) -> ModelSelections<'a> {
        ModelSelections::new(self.model_selections, self.source_packages)
    }

    /// The timed model of the model alias `model`.
    pub fn timed_model(&self, model: &str) -> Option<&'a TimedModel> {
        self.timed_models
            .iter()
            .find(|timed| timed.model() == model)
    }

    /// The source package, for an engine that has no package form yet
    /// (the SMT encoding); `None` on the package route.
    pub fn source_package(&self) -> Option<&'a CheckedPackage> {
        self.evaluator.source_package()
    }
}

/// Why the package route built no model-check input (FR-940).
#[derive(Debug, thiserror::Error)]
pub enum PendingPackageForm {
    /// The package holds `node`, a form the package route does not read
    /// yet. No input that leaves the node out is built.
    #[error("the package route does not read the {} form of node {node:?} yet", form.name())]
    Form {
        /// The form.
        form: UnreadModelForm,
        /// The first node of that form, in the package's node order.
        node: CheckedNodeId,
    },
    /// The package's declarations could not be read from its nodes and
    /// identity rows.
    #[error("the package's declarations are not readable: {0:?}")]
    World(fsl_package::BareSelectionFailure),
}

/// The package route's model-check input, built once from a
/// [`BarePackage`]'s admitted nodes (FR-940): it owns the tables it read and
/// lends them as a [`ModelCheckInput`].
pub struct PackageModelInput<'p> {
    package: &'p BarePackage<'p>,
    world: fsl_package::BareWorld,
    model_selections: Vec<DomainPackageRef>,
    operations: OperationTable,
    probabilistic: CheckedProbabilisticModel,
}

impl<'p> PackageModelInput<'p> {
    /// The package route's input over `package`, whose supply was attached
    /// (FR-517 handoff identity rows, FR-531).
    ///
    /// A package never carries a probabilistic model (S4 refuses to emit one),
    /// so its probabilistic tables are empty.
    ///
    /// # Errors
    ///
    /// [`PendingPackageForm::Form`] at the first node, in the package's node
    /// order, of a form the package route does not read yet;
    /// [`PendingPackageForm::World`] when the declarations are not readable
    /// from the nodes and identity rows.
    pub fn of(package: &'p BarePackage<'p>) -> Result<Self, PendingPackageForm> {
        if let Some((form, node)) = package.first_unread_model_form() {
            return Err(PendingPackageForm::Form {
                form,
                node: node.clone(),
            });
        }
        let world = package.world().map_err(PendingPackageForm::World)?;
        Ok(Self {
            package,
            world,
            model_selections: fsl_package::AnalysisView::model_selections(package),
            operations: OperationTable::default(),
            probabilistic: CheckedProbabilisticModel::default(),
        })
    }

    /// The input this package lends.
    pub fn input(&self) -> ModelCheckInput<'_> {
        ModelCheckInput {
            types: self.world.types(),
            variant_types: &[],
            enums: &[],
            operations: &self.operations,
            operations_read: false,
            model_selections: &self.model_selections,
            source_packages: &[],
            timed_models: &[],
            time_sources: &[],
            probabilistic: &self.probabilistic,
            state_clauses: Vec::new(),
            terminals: Ok(Vec::new()),
            evaluator: self.package,
        }
    }
}

mod sealed {
    /// Private to this module: no other type implements the evaluator.
    pub trait Sealed {}
    impl Sealed for fsl_package::CheckedPackage {}
    impl Sealed for fsl_package::BarePackage<'_> {}
}

/// The fault a source checked form handed to the package route's evaluator
/// raises: a package-route input holds no source form, so none reaches it.
const SOURCE_FORM: InternalFault = InternalFault::new("FR-940", "source-form-on-package-route");

/// The evaluator of a package's checked forms a model check reads through
/// its [`ModelCheckInput`] (FR-531, FR-940).
///
/// The trait is sealed: the source package and the bare package are its only
/// implementors. A method that takes a source checked form (a body, a row, a
/// predicate) is reached on the source route only, because the package
/// route's input holds no such form; the package route answers it with an
/// internal fault. A method keyed by a name or node evaluates on either
/// route, the package route through [`CheckedPackageEvaluation`] over the
/// package's forms.
pub trait ModelEvaluator: sealed::Sealed + Sync {
    /// The source package; `None` on the package route.
    fn source_package(&self) -> Option<&CheckedPackage>;

    /// FR-107: one state clause, by name, over its admitted observations.
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_clause`].
    fn evaluate_clause(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        meter: &mut Meter,
    ) -> Result<ClauseEvaluation, CallFailure>;

    /// FR-107 "Clause sets": the state clauses of one invocation, object or
    /// state, each with a fresh meter of `limits`, polling `cancel`.
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_clause_set`].
    fn evaluate_clause_set(
        &self,
        clauses: &[(QualifiedName, &AdmittedObservations)],
        limits: ScalarLimits,
        cancel: Option<&filament_exact::Cancel>,
    ) -> Result<ClauseSetEvaluation, CallFailure>;

    /// FR-230 "Collection-valued writes".
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_collection_write`].
    fn evaluate_collection_write(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        field: &str,
        written: &Written,
        meter: &mut Meter,
    ) -> Result<Option<WrittenCollection>, CallFailure>;

    /// FR-487 "Evaluating an atom".
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_hyper_holds`].
    fn evaluate_hyper_holds(
        &self,
        clause: &CheckedHyperClause,
        atom: FormulaNodeId,
        slots: Vec<crate::hyper::HyperSlot>,
        components: &[&ObjectEnvironment],
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure>;

    /// [`crate::value::evaluate_state_predicate_with`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_state_predicate(
        &self,
        predicate: &dyn StatePredicate,
        current: &ObjectEnvironment,
        over: Option<ObjectReference>,
        further: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_state_predicate_at_absent_key`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_state_predicate_at_absent_key(
        &self,
        predicate: &dyn StatePredicate,
        current: &ObjectEnvironment,
        key: ObjectReference,
        binding: String,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_activation_guard`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_activation_guard(
        &self,
        guard: &dyn StatePredicate,
        current: &ObjectEnvironment,
        bound: ObjectReference,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_activation_guard_at_absent_key`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_activation_guard_at_absent_key(
        &self,
        guard: &dyn StatePredicate,
        current: &ObjectEnvironment,
        key: ObjectReference,
        binding: String,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_capture`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_capture(
        &self,
        capture: &dyn StatePredicate,
        current: &ObjectEnvironment,
        bindings: Vec<Value>,
        absent: Option<(Vec<usize>, String)>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_terminal_predicate`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_terminal_predicate(
        &self,
        predicate: &CheckedTerminalPredicate,
        current: &ObjectEnvironment,
        self_object: ObjectReference,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_refinement_row`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_refinement_row(
        &self,
        row: &CheckedRow,
        current: &ObjectEnvironment,
        bindings: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_history_update`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_history_update(
        &self,
        row: &CheckedRow,
        states: (&ObjectEnvironment, &ObjectEnvironment),
        bindings: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_protocol_slot`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_protocol_slot(
        &self,
        body: &CheckedBody,
        frame: Vec<Option<Value>>,
        current: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_join_predicate`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_join_predicate(
        &self,
        function: &NodeKey,
        arguments: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_reward`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_reward(
        &self,
        reward: &Reward,
        arguments: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// The checked state clause `name`, for a reading of its body.
    fn state_clause(&self, name: &str) -> Option<&CheckedStateClause>;

    /// The checked protocol `name`.
    fn protocol(&self, name: &str) -> Option<&CheckedProtocol>;

    /// The admitted node keys of the protocol `name`.
    fn protocol_node_keys(&self, name: &str) -> Option<&CheckedProtocolNodeKeys>;

    /// FSpec FR-331: the ordinal of the own field `name` of `owner` among
    /// its own fields in name order.
    fn own_field_ordinal(&self, owner: &EffectiveId, name: &str) -> Option<usize>;

    /// FSpec FR-383: the footprint of `operation`, reading only the clauses
    /// `select` admits.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn selected_footprint(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
        select: &dyn Fn(NodeKey) -> bool,
    ) -> Result<SymbolicFootprint, FootprintRefusal>;

    /// ADR-027 FT-1: [`Self::selected_footprint`] for an attempt whose
    /// clauses hold a form with no footprint rule.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn attempt_footprint_fallback(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
        select: &dyn Fn(NodeKey) -> bool,
    ) -> Result<SymbolicFootprint, FootprintRefusal>;

    /// FSpec FR-383: the footprint of `operation` over every clause.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn footprint(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
    ) -> Result<SymbolicFootprint, FootprintRefusal>;

    /// FR-156 "Visibility": the reads of the state constraint `clause`.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn constraint_reads(
        &self,
        model: &StateModel,
        clause: &CheckedStateClause,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal>;

    /// ADR-027 FT-3: the reads of a protocol expression.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn slot_reads(
        &self,
        model: &StateModel,
        name: &str,
        parameters: &[(String, ValueType)],
        body: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal>;

    /// ADR-025 MA-1: the reads of a protocol node's constraint.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn node_reads(
        &self,
        model: &StateModel,
        name: &str,
        parameters: &[(String, ValueType)],
        body: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal>;

    /// FR-156 "Atom footprints": the reads of a `holds` atom's condition.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn atom_reads(
        &self,
        model: &StateModel,
        over: &(String, ValueType),
        condition: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal>;

    /// FR-337: the reads of an activation guard or capture.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn guard_reads(
        &self,
        model: &StateModel,
        binders: &[(String, ValueType)],
        condition: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal>;
}

impl ModelEvaluator for CheckedPackage {
    fn source_package(&self) -> Option<&CheckedPackage> {
        Some(self)
    }

    fn evaluate_clause(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        meter: &mut Meter,
    ) -> Result<ClauseEvaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_clause(self, clause, observations, meter)
    }

    fn evaluate_clause_set(
        &self,
        clauses: &[(QualifiedName, &AdmittedObservations)],
        limits: ScalarLimits,
        cancel: Option<&filament_exact::Cancel>,
    ) -> Result<ClauseSetEvaluation, CallFailure> {
        crate::value::evaluate_clause_set_with_cancel(self, clauses, limits, cancel)
    }

    fn evaluate_collection_write(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        field: &str,
        written: &Written,
        meter: &mut Meter,
    ) -> Result<Option<WrittenCollection>, CallFailure> {
        CheckedPackageEvaluation::evaluate_collection_write(
            self,
            clause,
            observations,
            field,
            written,
            meter,
        )
    }

    fn evaluate_hyper_holds(
        &self,
        clause: &CheckedHyperClause,
        atom: FormulaNodeId,
        slots: Vec<crate::hyper::HyperSlot>,
        components: &[&ObjectEnvironment],
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_hyper_holds(self, clause, atom, slots, components, meter)
    }

    fn evaluate_state_predicate(
        &self,
        predicate: &dyn StatePredicate,
        current: &ObjectEnvironment,
        over: Option<ObjectReference>,
        further: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_state_predicate_with(self, predicate, current, over, further, meter)
    }

    fn evaluate_state_predicate_at_absent_key(
        &self,
        predicate: &dyn StatePredicate,
        current: &ObjectEnvironment,
        key: ObjectReference,
        binding: String,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_state_predicate_at_absent_key(
            self, predicate, current, key, binding, meter,
        )
    }

    fn evaluate_activation_guard(
        &self,
        guard: &dyn StatePredicate,
        current: &ObjectEnvironment,
        bound: ObjectReference,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_activation_guard(self, guard, current, bound, meter)
    }

    fn evaluate_activation_guard_at_absent_key(
        &self,
        guard: &dyn StatePredicate,
        current: &ObjectEnvironment,
        key: ObjectReference,
        binding: String,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_activation_guard_at_absent_key(
            self, guard, current, key, binding, meter,
        )
    }

    fn evaluate_capture(
        &self,
        capture: &dyn StatePredicate,
        current: &ObjectEnvironment,
        bindings: Vec<Value>,
        absent: Option<(Vec<usize>, String)>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_capture(self, capture, current, bindings, absent, meter)
    }

    fn evaluate_terminal_predicate(
        &self,
        predicate: &CheckedTerminalPredicate,
        current: &ObjectEnvironment,
        self_object: ObjectReference,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_terminal_predicate(self, predicate, current, self_object, meter)
    }

    fn evaluate_refinement_row(
        &self,
        row: &CheckedRow,
        current: &ObjectEnvironment,
        bindings: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_refinement_row(self, row, current, bindings, meter)
    }

    fn evaluate_history_update(
        &self,
        row: &CheckedRow,
        states: (&ObjectEnvironment, &ObjectEnvironment),
        bindings: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_history_update(self, row, states, bindings, meter)
    }

    fn evaluate_protocol_slot(
        &self,
        body: &CheckedBody,
        frame: Vec<Option<Value>>,
        current: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_protocol_slot(self, body, frame, current, meter)
    }

    fn evaluate_join_predicate(
        &self,
        function: &NodeKey,
        arguments: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_join_predicate(self, function, arguments, meter)
    }

    fn evaluate_reward(
        &self,
        reward: &Reward,
        arguments: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_reward(self, reward, arguments, meter)
    }

    fn state_clause(&self, name: &str) -> Option<&CheckedStateClause> {
        self.graph().state_clause(name)
    }

    fn protocol(&self, name: &str) -> Option<&CheckedProtocol> {
        self.graph().protocol(name)
    }

    fn protocol_node_keys(&self, name: &str) -> Option<&CheckedProtocolNodeKeys> {
        self.graph().protocol_node_keys(name)
    }

    fn own_field_ordinal(&self, owner: &EffectiveId, name: &str) -> Option<usize> {
        self.graph().own_field_ordinal(owner, name)
    }

    fn selected_footprint(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
        select: &dyn Fn(NodeKey) -> bool,
    ) -> Result<SymbolicFootprint, FootprintRefusal> {
        derive_selected_footprint(
            self.graph(),
            model,
            receiver_type,
            declaring,
            operation,
            arguments,
            select,
        )
    }

    fn attempt_footprint_fallback(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
        select: &dyn Fn(NodeKey) -> bool,
    ) -> Result<SymbolicFootprint, FootprintRefusal> {
        derive_attempt_footprint_fallback(
            self.graph(),
            model,
            receiver_type,
            declaring,
            operation,
            arguments,
            select,
        )
    }

    fn footprint(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
    ) -> Result<SymbolicFootprint, FootprintRefusal> {
        derive_footprint(
            self.graph(),
            model,
            receiver_type,
            declaring,
            operation,
            arguments,
        )
    }

    fn constraint_reads(
        &self,
        model: &StateModel,
        clause: &CheckedStateClause,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        derive_constraint_reads(self.graph(), model, clause)
    }

    fn slot_reads(
        &self,
        model: &StateModel,
        name: &str,
        parameters: &[(String, ValueType)],
        body: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        derive_slot_reads(self.graph(), model, name, parameters, body)
    }

    fn node_reads(
        &self,
        model: &StateModel,
        name: &str,
        parameters: &[(String, ValueType)],
        body: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        derive_node_reads(self.graph(), model, name, parameters, body)
    }

    fn atom_reads(
        &self,
        model: &StateModel,
        over: &(String, ValueType),
        condition: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        derive_atom_reads(self.graph(), model, over, condition)
    }

    fn guard_reads(
        &self,
        model: &StateModel,
        binders: &[(String, ValueType)],
        condition: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        derive_guard_reads(self.graph(), model, binders, condition)
    }
}

/// The refusal a footprint derivation over a source checked form raises on
/// the package route, whose input holds no such form.
const NO_SOURCE_FOOTPRINT: FootprintRefusal = FootprintRefusal::Unresolved {
    what: "source checked form on the package route",
};

impl ModelEvaluator for BarePackage<'_> {
    fn source_package(&self) -> Option<&CheckedPackage> {
        None
    }

    fn evaluate_clause(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        meter: &mut Meter,
    ) -> Result<ClauseEvaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_clause(self, clause, observations, meter)
    }

    fn evaluate_clause_set(
        &self,
        clauses: &[(QualifiedName, &AdmittedObservations)],
        limits: ScalarLimits,
        _cancel: Option<&filament_exact::Cancel>,
    ) -> Result<ClauseSetEvaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_clause_set(self, clauses, limits)
    }

    fn evaluate_collection_write(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        field: &str,
        written: &Written,
        meter: &mut Meter,
    ) -> Result<Option<WrittenCollection>, CallFailure> {
        CheckedPackageEvaluation::evaluate_collection_write(
            self,
            clause,
            observations,
            field,
            written,
            meter,
        )
    }

    fn evaluate_hyper_holds(
        &self,
        clause: &CheckedHyperClause,
        atom: FormulaNodeId,
        slots: Vec<crate::hyper::HyperSlot>,
        components: &[&ObjectEnvironment],
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_hyper_holds(self, clause, atom, slots, components, meter)
    }

    fn evaluate_state_predicate(
        &self,
        _predicate: &dyn StatePredicate,
        _current: &ObjectEnvironment,
        _over: Option<ObjectReference>,
        _further: Vec<Value>,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_state_predicate_at_absent_key(
        &self,
        _predicate: &dyn StatePredicate,
        _current: &ObjectEnvironment,
        _key: ObjectReference,
        _binding: String,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_activation_guard(
        &self,
        _guard: &dyn StatePredicate,
        _current: &ObjectEnvironment,
        _bound: ObjectReference,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_activation_guard_at_absent_key(
        &self,
        _guard: &dyn StatePredicate,
        _current: &ObjectEnvironment,
        _key: ObjectReference,
        _binding: String,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_capture(
        &self,
        _capture: &dyn StatePredicate,
        _current: &ObjectEnvironment,
        _bindings: Vec<Value>,
        _absent: Option<(Vec<usize>, String)>,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_terminal_predicate(
        &self,
        _predicate: &CheckedTerminalPredicate,
        _current: &ObjectEnvironment,
        _self_object: ObjectReference,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_refinement_row(
        &self,
        _row: &CheckedRow,
        _current: &ObjectEnvironment,
        _bindings: Vec<Value>,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_history_update(
        &self,
        _row: &CheckedRow,
        _states: (&ObjectEnvironment, &ObjectEnvironment),
        _bindings: Vec<Value>,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_protocol_slot(
        &self,
        _body: &CheckedBody,
        _frame: Vec<Option<Value>>,
        _current: &ObjectEnvironment,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_join_predicate(
        &self,
        _function: &NodeKey,
        _arguments: Vec<Value>,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_reward(
        &self,
        _reward: &Reward,
        _arguments: Vec<Value>,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn state_clause(&self, _name: &str) -> Option<&CheckedStateClause> {
        None
    }

    fn protocol(&self, _name: &str) -> Option<&CheckedProtocol> {
        None
    }

    fn protocol_node_keys(&self, _name: &str) -> Option<&CheckedProtocolNodeKeys> {
        None
    }

    fn own_field_ordinal(&self, _owner: &EffectiveId, _name: &str) -> Option<usize> {
        None
    }

    fn selected_footprint(
        &self,
        _model: &StateModel,
        _receiver_type: EffectiveId,
        _declaring: EffectiveId,
        _operation: &OperationDeclaration,
        _arguments: &[(String, EffectiveId)],
        _select: &dyn Fn(NodeKey) -> bool,
    ) -> Result<SymbolicFootprint, FootprintRefusal> {
        Err(NO_SOURCE_FOOTPRINT)
    }

    fn attempt_footprint_fallback(
        &self,
        _model: &StateModel,
        _receiver_type: EffectiveId,
        _declaring: EffectiveId,
        _operation: &OperationDeclaration,
        _arguments: &[(String, EffectiveId)],
        _select: &dyn Fn(NodeKey) -> bool,
    ) -> Result<SymbolicFootprint, FootprintRefusal> {
        Err(NO_SOURCE_FOOTPRINT)
    }

    fn footprint(
        &self,
        _model: &StateModel,
        _receiver_type: EffectiveId,
        _declaring: EffectiveId,
        _operation: &OperationDeclaration,
        _arguments: &[(String, EffectiveId)],
    ) -> Result<SymbolicFootprint, FootprintRefusal> {
        Err(NO_SOURCE_FOOTPRINT)
    }

    fn constraint_reads(
        &self,
        _model: &StateModel,
        _clause: &CheckedStateClause,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Err(NO_SOURCE_FOOTPRINT)
    }

    fn slot_reads(
        &self,
        _model: &StateModel,
        _name: &str,
        _parameters: &[(String, ValueType)],
        _body: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Err(NO_SOURCE_FOOTPRINT)
    }

    fn node_reads(
        &self,
        _model: &StateModel,
        _name: &str,
        _parameters: &[(String, ValueType)],
        _body: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Err(NO_SOURCE_FOOTPRINT)
    }

    fn atom_reads(
        &self,
        _model: &StateModel,
        _over: &(String, ValueType),
        _condition: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Err(NO_SOURCE_FOOTPRINT)
    }

    fn guard_reads(
        &self,
        _model: &StateModel,
        _binders: &[(String, ValueType)],
        _condition: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Err(NO_SOURCE_FOOTPRINT)
    }
}
