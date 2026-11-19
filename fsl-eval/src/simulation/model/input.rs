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
    BarePackage, CheckedNodeId, CheckedPackage, PackageTerminalDeclaration, TerminalResolveRefusal,
    UnreadModelForm,
};
use fsl_semantics::check::footprint::{
    derive_atom_reads, derive_attempt_footprint_fallback, derive_constraint_reads,
    derive_footprint, derive_guard_reads, derive_node_reads, derive_selected_footprint,
    derive_slot_reads, FootprintRefusal, SymbolicFootprint, SymbolicLocation,
};
use fsl_semantics::check::{
    CheckedBody, CheckedHyperClause, CheckedNode, CheckedProbabilisticModel, CheckedProtocol,
    CheckedProtocolNodeKeys, CheckedRow, CheckedStateClause, CheckedTemporalClause,
    CheckedTerminalPredicate, EnumBinding, FormulaNodeId, ModelTimeSource, Reward, StatePredicate,
    TimedModel, VariantTypeDeclaration, Written,
};
use fsl_semantics::model::clause_observation::Observation;
use fsl_semantics::model::domain_package::{DomainPackage, DomainPackageRef};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::observation::{AdmittedObservations, ModelSelections};
use fsl_semantics::model::operation::{OperationDeclaration, OperationTable};
use fsl_semantics::model::state::StateModel;

use crate::value::{
    CheckedPackageEvaluation, ClauseEvaluation, ClauseSetEvaluation, PostTargets, QualifiedName,
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
    /// The declaration's `claim` occurrence of the node.
    pub claim: filament_exact::Origin,
    /// The object type the clause constrains.
    pub context: EffectiveId,
    /// The declaring type and name of the operation the clause belongs to;
    /// `None` for an invariant.
    pub operation: Option<(EffectiveId, String)>,
    /// Which state the clause observes: an invariant the current state, a
    /// precondition the pre-state, a postcondition the post-state.
    pub observation: Observation,
    /// Whether the clause is an `invariant` (and no `constraint`).
    pub invariant: bool,
}

/// One `terminal` declaration as a model check reads it (FR-411, FR-940).
/// The model is named by its source alias on the source route and by the
/// package identity of its selection on the package route.
#[derive(Clone, Debug)]
pub struct ModelTerminal<'a> {
    /// The model, by selection identity.
    pub model: super::TerminalModel,
    /// The declaration's `temporal`/`terminal` node.
    pub node: filament_exact::NodeKey,
    /// The declaration.
    pub declaration: ModelTerminalDeclaration<'a>,
}

/// A [`ModelTerminal`]'s declaration.
#[derive(Clone, Copy, Debug)]
pub enum ModelTerminalDeclaration<'a> {
    /// Every terminal state is intended.
    Any,
    /// A terminal state is intended when the predicate holds at it.
    When(TerminalWhen<'a>),
}

/// A `terminal when` predicate in the form its route reads.
#[derive(Clone, Copy)]
pub enum TerminalWhen<'a> {
    /// The source route's checked body.
    Source(&'a CheckedTerminalPredicate),
    /// The package route's projected condition.
    Package(&'a fsl_package::BareTerminalPredicate),
}

impl std::fmt::Debug for TerminalWhen<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(_) => f.write_str("TerminalWhen::Source"),
            Self::Package(_) => f.write_str("TerminalWhen::Package"),
        }
    }
}

impl TerminalWhen<'_> {
    /// The object type `self` ranges over, when the predicate names one.
    pub fn context(&self) -> Option<&EffectiveId> {
        match self {
            Self::Source(predicate) => match predicate.parameters().first() {
                Some((_, ValueType::Reference(context))) => Some(context),
                _ => None,
            },
            Self::Package(predicate) => Some(predicate.context()),
        }
    }
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
    pub terminals: Result<Vec<ModelTerminal<'a>>, TerminalResolveRefusal>,
    /// The evaluator of the package's checked forms.
    pub evaluator: &'a dyn ModelEvaluator,
    /// The source package the tables were read from; `None` on the package
    /// route.
    pub source: Option<&'a CheckedPackage>,
    /// The admitted package the tables were read from on the package route;
    /// `None` on the source route (FR-940).
    pub admitted: Option<&'a fsl_package::CheckedPackageV2>,
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
                    claim: clause.claim().clone(),
                    context: clause.context(),
                    operation: clause.operation().map(|operation| {
                        (
                            operation.declaring.clone(),
                            operation.declaration.name().to_owned(),
                        )
                    }),
                    observation: clause.observation(),
                    invariant: clause.kind() == fsl_forms::StateClauseKind::Invariant,
                })
                .collect(),
            terminals: fsl_package::resolve_terminals(package).map(|terminals| {
                terminals
                    .into_iter()
                    .map(|terminal| ModelTerminal {
                        model: super::TerminalModel::new(terminal.package, terminal.model),
                        node: terminal.node.clone(),
                        declaration: match terminal.declaration {
                            PackageTerminalDeclaration::Any => ModelTerminalDeclaration::Any,
                            PackageTerminalDeclaration::When(predicate) => {
                                ModelTerminalDeclaration::When(TerminalWhen::Source(predicate))
                            }
                        },
                    })
                    .collect()
            }),
            evaluator: package,
            source: Some(package),
            admitted: None,
        }
    }

    /// FR-411 "Deadlock-freedom verdict join": each declared terminal's node
    /// key, by its model's selection identity, on either route.
    pub fn terminal_keys(&self) -> fsl_semantics::check::TerminalKeys {
        fsl_semantics::check::TerminalKeys::of(
            self.terminals
                .as_ref()
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .map(|terminal| {
                    (
                        terminal.model.identity().to_owned(),
                        Some(terminal.node.clone()),
                    )
                }),
        )
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
        self.source
    }

    /// The admitted package of the package route (FR-940); `None` on the
    /// source route.
    pub fn admitted_package(&self) -> Option<&'a fsl_package::CheckedPackageV2> {
        self.admitted
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
    /// The supplied domain packages the package selects do not re-admit
    /// against its types (FR-106).
    #[error("the selected domain packages do not admit: {0:?}")]
    Model(fsl_semantics::model::observation::AdmissionFailure),
    /// The selected domain packages declare an operation assembly does not
    /// type (FR-120 "Operations").
    #[error("the selected domain packages' operations are not typed: {0:?}")]
    Operations(fsl_semantics::check::AssemblyRefusal),
    /// The package's `probabilistic` nodes do not read (FSL FR-740).
    #[error("the package's probabilistic nodes are not readable: {0}")]
    ProbabilisticNodes(fsl_package::ProbabilisticReadFault),
    /// The package's random parameters and workloads do not rebuild against
    /// the selected domain packages (FSL FR-740).
    #[error("the package's probabilistic model does not rebuild: {0}")]
    Probabilistic(fsl_semantics::check::PackageProbabilisticRefusal),
}

/// Why the package route built no package form of a temporal clause (FR-940).
#[derive(Debug, thiserror::Error)]
pub enum PendingClauseForm {
    /// The clause's facts were not read from the package.
    #[error("the package form of the clause was not read: {0:?}")]
    Read(fsl_package::BareClauseRefusal),
    /// The facts read do not rebuild the clause.
    #[error("the package form of the clause was not built: {0}")]
    Build(fsl_semantics::check::PackageClauseRefusal),
}

impl PendingClauseForm {
    /// The fact the clause carries that its package form does not read yet,
    /// with the node that carries it, when that is why no form was built
    /// (FR-940): a fairness set, an interval, an aggregate atom, a shared
    /// formula node, a profile other than the infinite-trace one or a
    /// capture.
    pub fn pending_fact(&self) -> Option<(&'static str, &fsl_package::CheckedNodeId)> {
        match self {
            Self::Read(fsl_package::BareClauseRefusal::Pending { fact, node }) => {
                Some((fact, node))
            }
            Self::Read(fsl_package::BareClauseRefusal::Selection(_)) | Self::Build(_) => None,
        }
    }
}

/// The package route's model-check input, built once from a
/// [`BarePackage`]'s admitted nodes (FR-940): it owns the tables it read and
/// lends them as a [`ModelCheckInput`].
pub struct PackageModelInput<'p> {
    package: &'p BarePackage<'p>,
    world: fsl_package::BareWorld,
    model: StateModel,
    model_selections: Vec<DomainPackageRef>,
    operations: OperationTable,
    probabilistic: CheckedProbabilisticModel,
    terminals: Vec<fsl_package::BareTerminal>,
    state_clauses: Vec<StateClauseEntry>,
}

impl<'p> PackageModelInput<'p> {
    /// The package route's input over `package`, whose supply was attached
    /// (FR-517 handoff identity rows, FR-531), with the domain package
    /// documents `packages` bound to its model selections (FR-850
    /// "Admission"). Its operation table is the one S3's assembly declares for
    /// those packages ([`fsl_semantics::check::model_operation_table`]), so a
    /// step applies the operations and effects the source route applies.
    ///
    /// Its random parameters and workloads are the ones the package's
    /// `probabilistic` nodes carry, rebuilt as S3 records them against the
    /// selected domain packages (FSL FR-740). A package route reads no
    /// reward (a `reward` node refuses as an unread form, below) and carries
    /// no claim of the model, and its delay tables are empty.
    ///
    /// # Errors
    ///
    /// [`PendingPackageForm::Form`] at the first node, in the package's node
    /// order, of a form the package route does not read yet;
    /// [`PendingPackageForm::World`] when the declarations are not readable
    /// from the nodes and identity rows;
    /// [`PendingPackageForm::Model`] when the supplied packages do not re-admit
    /// against the package's types; [`PendingPackageForm::Operations`] when
    /// their operations are not typed. A type that names a unit is typed
    /// against no unit names: the package route reads none yet, so such an
    /// operation refuses rather than being left out.
    pub fn of(
        package: &'p BarePackage<'p>,
        packages: &fsl_semantics::model::intake::PackageInput,
        normalization: fsl_semantics::model::accounting::ModelNormalizationLimits,
    ) -> Result<Self, PendingPackageForm> {
        if let Some((form, node)) = package.first_unread_model_form() {
            return Err(PendingPackageForm::Form {
                form,
                node: node.clone(),
            });
        }
        let world = package.world().map_err(PendingPackageForm::World)?;
        let model_selections = fsl_package::AnalysisView::model_selections(package);
        let model = StateModel::new(
            ModelSelections::new(&model_selections, &[]),
            world.types(),
            packages,
            normalization,
        )
        .map_err(PendingPackageForm::Model)?;
        let operations = model
            .operation_table(&std::collections::BTreeMap::new())
            .map_err(PendingPackageForm::Operations)?;
        let nodes = package
            .probabilistic()
            .map_err(PendingPackageForm::ProbabilisticNodes)?;
        let probabilistic = model
            .probabilistic_model(&operations, &nodes)
            .map_err(PendingPackageForm::Probabilistic)?;
        // FR-940: the state clauses by their `claim_names` rows (FSpec
        // FR-322), in source order, as the source route lists them.
        let state_clauses = package
            .state_clause_declarations()
            .map_err(PendingPackageForm::World)?
            .into_iter()
            .map(|declaration| StateClauseEntry {
                name: declaration.name,
                identity: declaration.identity,
                claim: declaration.claim,
                context: declaration.context,
                operation: declaration
                    .operation
                    .map(|operation| (operation.declaring, operation.operation)),
                observation: declaration.observation,
                invariant: declaration.invariant,
            })
            .collect();
        // FR-411, FR-940: the `terminal` declarations by their admitted rows.
        let terminals = package
            .terminal_declarations()
            .map_err(PendingPackageForm::World)?;
        Ok(Self {
            package,
            world,
            model,
            model_selections,
            operations,
            probabilistic,
            state_clauses,
            terminals,
        })
    }

    /// FR-940: the package form of the temporal clause declared at the
    /// `claim` occurrence `claim`, the model checker's clause on the package route: read from
    /// the package's nodes ([`BarePackage::temporal_clause_form`]) with the
    /// population domains of the supplied domain packages, and rebuilt as S3
    /// builds it ([`CheckedTemporalClause::of_package_form`]) within S3's
    /// domain `position_limit`.
    ///
    /// # Errors
    ///
    /// [`PendingClauseForm`] for a clause the package does not select or
    /// whose package form reads a fact the form does not read yet. A clause
    /// under a profile other than the infinite-trace one, and a clause with
    /// captures, whose package-route letters do not read them yet, are
    /// refused [`fsl_package::BareClauseRefusal::Pending`] at the clause node
    /// (`profile`, `capture`) before any engine runs.
    pub fn temporal_clause(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        position_limit: u64,
    ) -> Result<CheckedTemporalClause, PendingClauseForm> {
        let clause = claim.node();
        let mut form = self
            .package
            .temporal_clause_form(claim)
            .map_err(PendingClauseForm::Read)?;
        let pending = |fact| -> Result<CheckedTemporalClause, PendingClauseForm> {
            let node = self
                .package
                .temporal_clause_node(clause)
                .map_err(PendingClauseForm::Read)?;
            Err(PendingClauseForm::Read(
                fsl_package::BareClauseRefusal::Pending {
                    fact,
                    node: node.clone(),
                },
            ))
        };
        if form.profile != fsl_semantics::check::TemporalProfile::InfiniteTrace {
            return pending("profile");
        }
        if !form.captures.is_empty() {
            return pending("capture");
        }
        if let ValueType::Reference(object) = &form.over.1 {
            form.populations = self.model.overlapping_populations(object.clone());
        }
        CheckedTemporalClause::of_package_form(form, self.world.types(), position_limit)
            .map_err(PendingClauseForm::Build)
    }

    /// FR-940 "Package form of a state-graph claim": the state-graph claim
    /// declared at the `claim` occurrence `claim`, read from the package's
    /// nodes ([`BarePackage::state_graph_claim_form`]) and rebuilt as S3
    /// builds it ([`CheckedStateGraphClaim::of_package_form`]) within S3's
    /// domain `position_limit`. Its predicates are evaluated through the
    /// evaluator port ([`ModelEvaluator::evaluate_state_graph_predicate`]).
    ///
    /// # Errors
    ///
    /// [`PendingClauseForm`] for a claim the package does not select or
    /// whose facts do not rebuild it.
    pub fn state_graph_claim(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        position_limit: u64,
    ) -> Result<fsl_semantics::check::CheckedStateGraphClaim, PendingClauseForm> {
        let form = self
            .package
            .state_graph_claim_form(claim)
            .map_err(PendingClauseForm::Read)?;
        fsl_semantics::check::CheckedStateGraphClaim::of_package_form(
            form,
            self.world.types(),
            position_limit,
        )
        .map_err(PendingClauseForm::Build)
    }

    /// The state clause declared at the `claim` occurrence `claim` of the
    /// node `clause`, as its `claim_names` row names it (FR-940).
    pub fn state_clause(
        &self,
        clause: &NodeKey,
        claim: &filament_exact::Origin,
    ) -> Option<&StateClauseEntry> {
        self.state_clauses
            .iter()
            .find(|entry| entry.identity == *clause && entry.claim == *claim)
    }

    /// Every state clause declaration, in source order (FR-940).
    pub fn state_clauses(&self) -> &[StateClauseEntry] {
        &self.state_clauses
    }

    /// The state clause whose node key is `clause`, declared at its own
    /// `claim_names` row (FR-940).
    pub fn state_clause_at(&self, clause: &NodeKey) -> Option<&StateClauseEntry> {
        self.state_clauses
            .iter()
            .find(|entry| entry.identity == *clause)
    }

    /// FR-104 "Requirements", FR-940: the domain kinds of the
    /// `operation-contract` record of the `invariant` state clause `entry`,
    /// as S3 classifies them: the domains of its `self` parameter's type
    /// within `position_limit`, then its context's population. `None` when
    /// no single population of the selected domain packages covers the
    /// context, so the record's other populations (those its body walks or
    /// names) would decide the extent, and for a type that does not
    /// classify.
    pub fn invariant_domain_kinds(
        &self,
        entry: &StateClauseEntry,
        position_limit: u64,
    ) -> Option<Vec<fsl_foundation::bound::DomainKind>> {
        let receiver = ValueType::Reference(entry.context.clone());
        let mut kinds: Vec<fsl_foundation::bound::DomainKind> =
            match fsl_semantics::family::classify_extent(
                &[(entry.identity.clone(), &receiver)],
                self.world.types(),
                position_limit,
            )
            .ok()?
            {
                fsl_semantics::family::ClaimExtent::Bounded => Vec::new(),
                fsl_semantics::family::ClaimExtent::Unbounded(domains) => {
                    domains.iter().map(|(_, kind)| kind).collect()
                }
            };
        // A domain package population declares no maximum, and the
        // package route reads no authored bound, as for a temporal clause's
        // populations ([`StateModel::overlapping_populations`]).
        self.model
            .covering_population(entry.context.clone())
            .ok()??;
        kinds.push(fsl_foundation::bound::DomainKind::Population);
        Some(kinds)
    }

    /// The package this input was built from.
    pub fn package(&self) -> &'p BarePackage<'p> {
        self.package
    }

    /// The input this package lends.
    pub fn input(&self) -> ModelCheckInput<'_> {
        ModelCheckInput {
            types: self.world.types(),
            variant_types: &[],
            enums: &[],
            operations: &self.operations,
            model_selections: &self.model_selections,
            source_packages: &[],
            timed_models: &[],
            time_sources: &[],
            probabilistic: &self.probabilistic,
            state_clauses: self.state_clauses.clone(),
            terminals: Ok(self
                .terminals
                .iter()
                .map(|terminal| ModelTerminal {
                    model: super::TerminalModel::named(terminal.model.clone()),
                    node: terminal.node.clone(),
                    declaration: match &terminal.declaration {
                        fsl_package::BareTerminalDeclaration::Any => ModelTerminalDeclaration::Any,
                        fsl_package::BareTerminalDeclaration::When(predicate) => {
                            ModelTerminalDeclaration::When(TerminalWhen::Package(predicate))
                        }
                    },
                })
                .collect()),
            evaluator: self.package,
            source: None,
            admitted: Some(self.package.package()),
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

/// The fault a package-route form handed to the source package's evaluator
/// raises: a source-route input holds no package form, so none reaches it.
const PACKAGE_FORM: InternalFault = InternalFault::new("FR-940", "package-form-on-source-route");

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

    /// FR-940: the `holds` atom `atom` of a temporal clause, keyed by the
    /// clause node and the atom's formula position, over `objects`. Both
    /// routes evaluate it: the source route from its checked graph, the
    /// package route through the atom's package form.
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_holds`].
    fn evaluate_holds(
        &self,
        atom: &crate::value::HoldsAtom,
        over: Value,
        captures: &[Value],
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure>;

    /// FR-940: the `when` guard of the temporal clause `clause`, keyed by
    /// its node, over `objects`, on either route.
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_guard`].
    fn evaluate_guard(
        &self,
        clause: &NodeKey,
        over: Value,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure>;

    /// FR-940: `invocation` against the operation frame `frame`, keyed by
    /// its node, on either route. The package route reads the frame's
    /// declared effect from `operations`, the input's operation table.
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_frame`].
    fn evaluate_frame(
        &self,
        frame: &NodeKey,
        invocation: &fsl_semantics::model::observation::AdmittedInvocation<'_>,
        operations: &OperationTable,
        meter: &mut Meter,
    ) -> Result<crate::value::FrameEvaluation, CallFailure>;

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

    /// FR-120 "Directly solved assignments", FR-851 "Eligible conjuncts":
    /// the equation and bound conjuncts of the postcondition `clause` over
    /// `observations`, each conjunct's `e` evaluated once with a fresh meter
    /// from `meter`. Both routes read the clause's checked form.
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_clause`].
    fn evaluate_post_targets(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        meter: &mut dyn FnMut() -> Meter,
    ) -> Result<PostTargets, CallFailure>;

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

    /// FR-940 "Package form of a state-graph claim": the predicate of
    /// `role` of the state-graph claim declared at `claim`, with its `over`
    /// parameter bound to `over` (`None` for a claim with no `over`) and
    /// the slots after it to `further`, at the state whose objects are
    /// `current`. Both routes evaluate it: the source route its checked
    /// body, the package route the predicate node's reader projection, so a
    /// predicate has one value on both.
    ///
    /// # Errors
    ///
    /// An internal fault when the claim has no written predicate of `role`.
    fn evaluate_state_graph_predicate(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
        current: &ObjectEnvironment,
        over: Option<ObjectReference>,
        further: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// FR-156, FR-940 "Package form of a state-graph claim": the symbolic
    /// locations the predicate of `role` of the claim at `claim` reads, as the
    /// partial-order reduction takes them; none for the constant `true`.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation refuses a form with no
    /// footprint rule.
    fn state_graph_reads(
        &self,
        model: &StateModel,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal>;

    /// [`Self::evaluate_state_graph_predicate`] for an instance whose key
    /// holds no object conforming to the `over` parameter's type, as
    /// [`crate::value::evaluate_state_predicate_at_absent_key`] reads it.
    ///
    /// # Errors
    ///
    /// As [`Self::evaluate_state_graph_predicate`].
    fn evaluate_state_graph_predicate_at_absent_key(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
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
        predicate: TerminalWhen<'_>,
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

    fn evaluate_holds(
        &self,
        atom: &crate::value::HoldsAtom,
        over: Value,
        captures: &[Value],
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_holds(self, atom, over, captures, objects, meter)
    }

    fn evaluate_guard(
        &self,
        clause: &NodeKey,
        over: Value,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_guard(self, clause, over, objects, meter)
    }

    fn evaluate_frame(
        &self,
        frame: &NodeKey,
        invocation: &fsl_semantics::model::observation::AdmittedInvocation<'_>,
        _operations: &OperationTable,
        meter: &mut Meter,
    ) -> Result<crate::value::FrameEvaluation, CallFailure> {
        // The source route reads the declared effect from its checked frame.
        CheckedPackageEvaluation::evaluate_frame(self, frame.clone(), invocation, meter)
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

    fn evaluate_post_targets(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        meter: &mut dyn FnMut() -> Meter,
    ) -> Result<PostTargets, CallFailure> {
        crate::value::source_post_targets(self, clause, observations, meter)
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

    fn state_graph_reads(
        &self,
        model: &StateModel,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        let body =
            source_predicate(self, claim, role).map_err(|_| FootprintRefusal::Unresolved {
                what: "state-graph predicate",
            })?;
        derive_atom_reads(
            self.graph(),
            model,
            &(String::new(), ValueType::Boolean),
            body.body(),
        )
    }

    fn evaluate_state_graph_predicate(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
        current: &ObjectEnvironment,
        over: Option<ObjectReference>,
        further: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        let body = source_predicate(self, claim, role)?;
        crate::value::evaluate_state_predicate_with(self, body, current, over, further, meter)
    }

    fn evaluate_state_graph_predicate_at_absent_key(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
        current: &ObjectEnvironment,
        key: ObjectReference,
        binding: String,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        let body = source_predicate(self, claim, role)?;
        crate::value::evaluate_state_predicate_at_absent_key(
            self, body, current, key, binding, meter,
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
        predicate: TerminalWhen<'_>,
        current: &ObjectEnvironment,
        self_object: ObjectReference,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        match predicate {
            TerminalWhen::Source(predicate) => crate::value::evaluate_terminal_predicate(
                self,
                predicate,
                current,
                self_object,
                meter,
            ),
            TerminalWhen::Package(_) => Err(PACKAGE_FORM),
        }
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

    fn evaluate_holds(
        &self,
        atom: &crate::value::HoldsAtom,
        over: Value,
        captures: &[Value],
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_holds(self, atom, over, captures, objects, meter)
    }

    fn evaluate_guard(
        &self,
        clause: &NodeKey,
        over: Value,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_guard(self, clause, over, objects, meter)
    }

    fn evaluate_frame(
        &self,
        frame: &NodeKey,
        invocation: &fsl_semantics::model::observation::AdmittedInvocation<'_>,
        operations: &OperationTable,
        _meter: &mut Meter,
    ) -> Result<crate::value::FrameEvaluation, CallFailure> {
        crate::bare::evaluate_frame(self, operations, frame, invocation)
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

    fn evaluate_post_targets(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        meter: &mut dyn FnMut() -> Meter,
    ) -> Result<PostTargets, CallFailure> {
        crate::value::bare_post_targets(self, clause, observations, meter)
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

    fn state_graph_reads(
        &self,
        model: &StateModel,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        let unresolved = || FootprintRefusal::Unresolved {
            what: "state-graph predicate",
        };
        let predicate = self
            .project_state_graph_predicate(claim, role)
            .map_err(|_| unresolved())?;
        let world = self.world().map_err(|_| unresolved())?;
        crate::value::projection_atom_reads(predicate.projection(), world.types(), model)
    }

    fn evaluate_state_graph_predicate(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
        current: &ObjectEnvironment,
        over: Option<ObjectReference>,
        further: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        let over = over.map_or(
            crate::value::StateGraphOver::None,
            crate::value::StateGraphOver::Bound,
        );
        crate::value::evaluate_bare_state_graph_predicate(
            self, claim, role, current, over, further, meter,
        )
    }

    fn evaluate_state_graph_predicate_at_absent_key(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
        current: &ObjectEnvironment,
        key: ObjectReference,
        binding: String,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_bare_state_graph_predicate(
            self,
            claim,
            role,
            current,
            crate::value::StateGraphOver::Absent { key, binding },
            Vec::new(),
            meter,
        )
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
        predicate: TerminalWhen<'_>,
        current: &ObjectEnvironment,
        self_object: ObjectReference,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        match predicate {
            TerminalWhen::Package(predicate) => crate::value::evaluate_bare_terminal_predicate(
                self,
                predicate,
                current,
                self_object,
                meter,
            ),
            TerminalWhen::Source(_) => Err(SOURCE_FORM),
        }
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

/// The checked predicate of `role` of the source package's state-graph claim
/// at `claim`: a written one, which the port evaluates.
fn source_predicate<'p>(
    package: &'p CheckedPackage,
    claim: &fsl_foundation::source::provenance::OccurrenceKey,
    role: fsl_semantics::check::PredicateRole,
) -> Result<&'p fsl_semantics::check::CheckedPredicateBody, InternalFault> {
    use fsl_semantics::check::{CheckedStatePredicate, PredicateRole, StateGraphForm};
    let fault = |cause: &'static str| InternalFault::new("FR-940", cause);
    let claim = package
        .graph()
        .state_graph_claims()
        .iter()
        .find(|held| held.occurrence() == Some(claim))
        .ok_or_else(|| fault("state-graph-claim-is-in-the-package"))?;
    let (from, target) = match claim.form() {
        StateGraphForm::Possible { target } => (None, target),
        StateGraphForm::AlwaysPossible { from, target } => (Some(from), target),
        StateGraphForm::UniquePath { from, to } => (Some(from), to),
    };
    let predicate = match role {
        PredicateRole::From => from,
        PredicateRole::Target => Some(target),
    };
    match predicate {
        Some(CheckedStatePredicate::Checked(body)) => Ok(body),
        _ => Err(fault("state-graph-predicate-is-written")),
    }
}
