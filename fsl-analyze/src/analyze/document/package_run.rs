// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-850, FR-940: the items of a `package` request, resolved against the
//! admitted package's name tables and settled over its package forms.
//!
//! A temporal clause is resolved by its `temporal_clause_names` row and run
//! over its package form ([`super::super::PackageClauseSubject`]); a
//! derived deadlock-freedom item is resolved by the identity of the model
//! selection it is over and run over the package route's model-check input
//! ([`super::super::PackageDeadlockSubject`]). Both replay a counterexample
//! from the package bytes and the handoff supply. They are negotiated,
//! echoed and written by the source route's own negotiation and writer, so
//! an item settles with the source route's record, counterexample, replay
//! and summary. An item whose family has no package form yet refuses the
//! request whole, typed and naming the family, never settling `unsupported`
//! for a fact the source route reads.

use std::collections::BTreeMap;

use filament_exact::Cancel;
use filament_semantic_value::catalog::CatalogCode;
use fsl_eval::model_subject::{CapacityBinding, RoleBinding};
use fsl_eval::simulation::model::{PackageModelInput, PendingPackageForm};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, RequestIndex};
use fsl_package::{AnalysisView, BarePackage, CheckedNodeId, UnreadModelForm};
use fsl_replay::value_check::ValueTarget;
use fsl_replay::TerminalRecord;
use fsl_route::request::{ExtentClassification, ItemKind};
use fsl_route::{CandidateOutcome, Registry};
use fsl_semantics::check::{Capability, DeclarationFamily};
use serde_json::{json, Map};

use super::super::hyper_route::PackageRelationSubject;
use super::super::package_hyper::PackageHyperSubject;
use super::super::refinement_item::PackageRefinementSubject;
use super::super::request::ProtocolBinding;
use super::super::{
    AnalyzeItem, AnalyzeLimits, ClaimItem, ClauseItemRefusal, ClauseLimits, Engines, ItemForm,
    ItemOptions, NegotiatedItem, PackageClauseSubject, PackageDeadlockSubject,
    PackageInvariantSubject, PackageItemInputs, PackageStateGraphSubject, SubjectBinding,
};
mod probability;
mod schedule;

use super::{
    candidate_list, invalid_kind, negotiate_items, negotiated_routes, temporal_settings,
    unknown_claim, unknown_option, write_run, AnalyzeDocumentError, ItemReads, ItemSelection,
    Negotiate, Negotiation, ResolvedClaim, ResolvedItem, SettledParts, SettledRun,
    SubjectSelection,
};

/// A `package` request (FR-850) whose item names a family the package route
/// does not settle yet (FR-940 "Package forms of the source-checked
/// evaluators"): the request is refused whole rather than settle that item
/// `unsupported`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingPackageFamily {
    /// The request index of the first such item, `None` for the default
    /// items of a request with no `items` member and for a form the
    /// package's model-check input does not read.
    pub request_index: Option<RequestIndex>,
    /// What the item names.
    pub family: PendingFamily,
    /// The package node that carries the pending fact, when one does.
    pub node: Option<CheckedNodeId>,
    /// The selected claim's context for a probabilistic or operation-contract
    /// refusal, including why an unresolved probabilistic claim does not resolve.
    pub note: Option<String>,
}

impl PendingPackageFamily {
    /// The node a probabilistic refusal is located at, in words: `None` for
    /// every other family.
    pub fn locus(&self) -> Option<String> {
        matches!(
            self.family,
            PendingFamily::ProbabilisticSourceModel | PendingFamily::ProbabilisticClaimUnresolved
        )
        .then(|| self.node.as_ref().map(node_words))
        .flatten()
    }

    /// The refusal's detail: the item, the family and the node.
    pub fn detail(&self) -> String {
        let at = self
            .node
            .as_ref()
            .map_or_else(String::new, |node| format!(" at {node:?}"));
        match (self.request_index, self.family) {
            (Some(index), PendingFamily::ProbabilisticSourceModel) => format!(
                ": items[{}] names the probabilistic claim {}, which cannot run on the package route: the model has source objects and the package rows carry no effects, fields or populations to step it{}",
                index.get(),
                self.note.as_deref().unwrap_or_default(),
                self.locus()
                    .map_or_else(String::new, |locus| format!(" at node `{locus}`"))
            ),
            (Some(index), PendingFamily::ProbabilisticClaimUnresolved) => format!(
                ": items[{}] names the probabilistic claim {}, which does not resolve against the package's source objects{}",
                index.get(),
                self.note.as_deref().unwrap_or_default(),
                self.locus()
                    .map_or_else(String::new, |locus| format!(" at node `{locus}`"))
            ),
            (Some(index), PendingFamily::ClauseFact(fact)) => format!(
                ": items[{}] names a temporal clause whose {fact} has no package form yet{at}",
                index.get()
            ),
            (Some(index), PendingFamily::ProbabilityForm(fact)) => format!(
                ": items[{}] names a probabilistic claim whose {fact} has no package form yet{at}",
                index.get()
            ),
            (Some(index), PendingFamily::OperationContractRequirementRecord) => format!(
                ": items[{}] names the operation-contract claim {}, whose operation-contract requirement record has no package form yet{at}",
                index.get(),
                self.note.as_deref().unwrap_or_default()
            ),
            (Some(index), PendingFamily::ValueShape) => format!(
                ": items[{}] value-validity has unsupported shape {}{at}",
                index.get(), self.note.as_deref().unwrap_or("unavailable-type")
            ),
            (Some(index), PendingFamily::CompositionOwningWorkPermission) => format!(
                ": items[{}] composition owning execution is pending missing native before-work permission{at}",
                index.get()
            ),
            (None, PendingFamily::CompositionOwningWorkPermission) => format!(
                ": composition owning execution is pending missing native before-work permission{at}"
            ),
            (Some(index), family) => format!(
                ": items[{}] names the {} family, which has no package form yet{at}",
                index.get(),
                family.name()
            ),
            (None, PendingFamily::ModelForm(form)) => format!(
                ": the package's {} form has no package form in the model-check input yet{at}",
                form.name()
            ),
            (None, family) => format!(": the {} rule has no package form yet{at}", family.name()),
        }
    }
}

/// The words a claim node is located by: `package/claim/probabilistic_claim/name`.
fn node_words(node: &CheckedNodeId) -> String {
    use filament_contract_model::CheckedNodePathSegment;
    let path: Vec<&str> = node
        .path
        .iter()
        .take(3)
        .filter_map(|segment| match segment {
            CheckedNodePathSegment::Name(name) => Some(&**name),
            _ => None,
        })
        .collect();
    format!("{}/{}", node.package, path.join("/"))
}

/// What a pending package-route item names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PendingFamily {
    /// A refinement fact outside the supported byte-backed execution subset.
    RefinementFact(&'static str),
    /// A claimless `simulation` item (FR-634).
    Simulation,
    /// A schedulability item an EN-6 task-product engine decides: the package
    /// route reads no task-product input yet.
    TaskProduct,
    /// A derived item other than a model subject's deadlock-freedom item
    /// (FSpec FR-368).
    Derived,
    /// FR-730's invariant item, pending the package wire contract.
    LoopInvariant,
    /// FR-730's termination item, pending the package wire contract.
    LoopTermination,
    /// A named declaration.
    Declaration,
    /// A value-validity parameter or body form has no package evaluator form.
    ValueShape,
    /// A selected operation contract needs the source requirement record,
    /// whose operation-contract form the package wire contract does not carry.
    OperationContractRequirementRecord,
    /// A name the package carries no row for while it holds a claim whose
    /// name it does not carry ([`fsl_package::AnalysisView::unnamed_claims`]):
    /// the name may be that claim's, so it is not settled `unknown-claim`.
    UnnamedClaim,
    /// A composition request whose runtime route is not yet available.
    Composition,
    /// Owning composition execution requires the missing native before-work
    /// permission hook in the type-environment builder.
    CompositionOwningWorkPermission,
    /// A temporal clause fact its package form does not read yet.
    ClauseFact(&'static str),
    /// A form of the package its model-check input does not read yet.
    ModelForm(UnreadModelForm),
    /// A probabilistic claim over a source-object model (FSL FR-740 "Source
    /// objects", FR-940-AC-6): its probabilistic model rebuilds from the
    /// package's rows, but those rows carry no effect frame, field or
    /// population, so no engine can step the model and the claim cannot run.
    ProbabilisticSourceModel,
    /// A probabilistic claim over a source-object model whose probabilistic
    /// model does not rebuild from the package's rows.
    ProbabilisticClaimUnresolved,
    /// An admitted probabilistic claim has a form outside the package route.
    ProbabilityForm(&'static str),
}

impl PendingFamily {
    /// The family's name in a refusal message.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::RefinementFact(fact) => fact,
            Self::Simulation => "simulation",
            Self::TaskProduct => "task-product",
            Self::Derived => "derived",
            Self::LoopInvariant => "loop-invariant",
            Self::LoopTermination => "loop-termination",
            Self::Declaration => "declaration",
            Self::ValueShape => "value-validity-shape",
            Self::OperationContractRequirementRecord => "operation-contract-requirement-record",
            Self::UnnamedClaim => "unnamed-claim",
            Self::Composition => "composition",
            Self::CompositionOwningWorkPermission => "composition-owning-work-permission",
            Self::ClauseFact(fact) => fact,
            Self::ModelForm(_) => "model-form",
            Self::ProbabilisticSourceModel => "probabilistic-source-model",
            Self::ProbabilisticClaimUnresolved => "probabilistic-claim-unresolved",
            Self::ProbabilityForm(fact) => fact,
        }
    }
}

/// Why a `package` request settled no document.
#[derive(Debug, thiserror::Error)]
pub enum PackageRunError {
    /// An item, or the package, names a family with no package form yet.
    #[error("unsupported_construct/not-yet-implemented{}", .0.detail())]
    Pending(PendingPackageFamily),
    /// The package's model-check input retained its typed refusal.
    #[error(transparent)]
    Input(#[from] PendingPackageForm),
    /// A selected clause's admitted facts could not rebuild its checked form.
    #[error("package clause at {node:?} could not rebuild: {refusal}")]
    ClauseBuild {
        /// The selected clause node.
        node: CheckedNodeId,
        /// The builder's original refusal, including any reached limit.
        refusal: fsl_semantics::check::PackageClauseRefusal,
    },
    /// The run was refused or faulted as the source route's is.
    #[error(transparent)]
    Document(#[from] AnalyzeDocumentError),
}

/// What one `package` request (FR-850) settles over.
pub struct PackageRun<'a> {
    /// The admitted package with the handoff's identity and equality rows
    /// attached (FR-531).
    pub package: &'a BarePackage<'a>,
    /// The admitted `checked-package.json` bytes, which replay reads.
    pub package_bytes: &'a [u8],
    /// The handoff's identity and equality file bytes.
    pub supply: fsl_replay::HandoffSupply<'a>,
    /// The request's default subject.
    pub subject: SubjectSelection,
    /// The request's shared subject binding.
    pub binding: SubjectBinding<'a>,
    /// The request's items; `None` for the default items.
    pub items: Option<&'a [ItemSelection]>,
    /// The engines `analyze` may run.
    pub engines: &'a Engines,
    /// The shared limits.
    pub limits: AnalyzeLimits,
}

/// One item as the package's name tables resolve it.
enum Named<'s> {
    Simulation(super::super::SimulationItem),
    Hyper {
        index: RequestIndex,
        name: &'s str,
        declaration: CheckedNodeId,
        ordinal: u64,
        selection: &'s ItemSelection,
    },
    Probability {
        index: RequestIndex,
        name: &'s str,
        declaration: CheckedNodeId,
        ordinal: u64,
        selection: &'s ItemSelection,
    },
    Refinement {
        index: RequestIndex,
        name: &'s str,
        declaration: CheckedNodeId,
        selection: &'s ItemSelection,
    },
    Relation {
        index: RequestIndex,
        name: &'s str,
        declaration: CheckedNodeId,
        ordinal: u64,
        selection: &'s ItemSelection,
    },
    Settled(ResolvedItem),
    Clause {
        index: RequestIndex,
        name: &'s str,
        claim: OccurrenceKey,
        selection: &'s ItemSelection,
    },
    /// A derived item of the model selection `name`.
    Deadlock {
        index: RequestIndex,
        name: &'s str,
        kind: ItemKind,
        selection: &'s ItemSelection,
    },
    ProtocolFreedom {
        index: RequestIndex,
        name: &'s str,
        protocol: String,
        selection: &'s ItemSelection,
    },
    ProtocolAnalysis {
        index: RequestIndex,
        name: &'s str,
        selection: &'s ItemSelection,
    },
    SuppliedProtocol {
        index: RequestIndex,
        name: &'s str,
        selection: &'s ItemSelection,
    },
    StateClause {
        index: RequestIndex,
        name: &'s str,
        declaration: CheckedNodeId,
        ordinal: u64,
        selection: &'s ItemSelection,
    },
    StateGraph {
        index: RequestIndex,
        name: &'s str,
        declaration: CheckedNodeId,
        ordinal: u64,
        selection: &'s ItemSelection,
    },
    /// A `value-validity` item over the Boolean value clause a function is.
    Value {
        index: RequestIndex,
        name: &'s str,
        selection: &'s ItemSelection,
        claim: Option<OccurrenceKey>,
    },
    /// A `schedulability` item over a task-set claim (FR-246, FR-633).
    Schedule {
        index: RequestIndex,
        name: &'s str,
        declaration: CheckedNodeId,
        ordinal: u64,
        selection: &'s ItemSelection,
    },
}

/// How a resolved package-route claim item runs.
enum Route<'a> {
    Probability,
    Refinement(Box<PackageRefinementSubject<'a>>),
    Clause {
        subject: Box<PackageClauseSubject<'a>>,
        protocol: Option<ProtocolBinding>,
    },
    ProtocolFreedom(ProtocolBinding),
    ProtocolAnalysis,
    SuppliedProtocol(String),
    Deadlock,
    StateClause,
    StateGraph(Box<PackageStateGraphSubject<'a>>),
    Hyper(Box<PackageHyperSubject<'a>>),
    Relation(Box<PackageRelationSubject<'a>>),
    Value(Box<PackageValue<'a>>),
    Schedule(Box<schedule::ScheduleRoute>),
}

/// A Boolean value clause's target over the package and the semantic type of
/// its body, which is the `result_bound` of the clause's provider request item.
struct PackageValue<'a> {
    target: ValueTarget<'a>,
    result_bound: Option<filament_exact::NodeKey>,
    record: Option<fsl_route::request::RequirementItem>,
}

/// An authored clause and the optional protocol subject it runs over.
struct ClauseRoute<'a> {
    subject: Box<PackageClauseSubject<'a>>,
    protocol: Option<ProtocolBinding>,
}

/// FR-850, FR-940: resolve `run`'s items against the package's name tables,
/// negotiate them with `negotiate`, run them over their package forms under
/// `cancel` and settle FR-621's document with the source route's writer.
///
/// # Errors
///
/// [`PackageRunError::Pending`] for the first item whose family has no
/// package form yet, and for a package whose model-check input meets a form
/// it does not read yet, before any engine runs;
/// [`PackageRunError::Document`] as [`super::settle_run`] refuses or faults.
pub fn settle_package_run(
    run: PackageRun<'_>,
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> Result<SettledRun, PackageRunError> {
    settle_package_run_inner(run, None, negotiate, cancel)
}

/// Settle the value-validity records owned by a package call (FR-961).
/// Each call item retains its admitted row index and expression occurrence.
pub fn settle_package_call(
    run: PackageRun<'_>,
    function: &str,
    call_items: &[super::CallItem],
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> Result<SettledRun, PackageRunError> {
    settle_package_run_inner(run, Some((function, call_items)), negotiate, cancel)
}

fn settle_package_run_inner(
    run: PackageRun<'_>,
    call: Option<(&str, &[super::CallItem])>,
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> Result<SettledRun, PackageRunError> {
    let PackageRun {
        package,
        package_bytes,
        supply,
        subject,
        binding,
        items,
        engines,
        limits,
    } = run;
    // FSL FR-1044: admitted planner facts are available, but owning execution
    // still requires native permission before type-environment work. Preserve
    // the whole-request guard until that execution boundary is available.
    if items.is_none() {
        if let Some(node) = admitted_composition(package) {
            return Err(PackageRunError::Pending(PendingPackageFamily {
                request_index: None,
                note: None,
                family: PendingFamily::CompositionOwningWorkPermission,
                node: Some(node),
            }));
        }
    }
    let explicit_items = items.is_some();
    let defaults = items.is_none().then(|| default_selections(package));
    let items: &[ItemSelection] = items.or(defaults.as_deref()).unwrap_or_default();
    if let Some(refusal) =
        probabilistic_source_refusal(package, binding.packages, limits.normalization, items)?
    {
        return Err(PackageRunError::Pending(refusal));
    }
    let named = if let Some((function, call_items)) = call {
        if call_items.len() != items.len() {
            return Err(AnalyzeDocumentError::Fault.into());
        }
        call_items
            .iter()
            .zip(items)
            .map(|(item, selection)| Named::Value {
                index: item.request_index,
                name: function,
                selection,
                claim: Some(item.claim.clone()),
            })
            .collect()
    } else {
        resolve_names(package, items)?
    };
    let runnable = named
        .iter()
        .any(|named| !matches!(named, Named::Settled(_)));
    // The input is built only for an item that runs, so a request whose
    // items all settle by name settles as the source route's does.
    let input = runnable
        .then(|| PackageModelInput::of(package, binding.packages, limits.normalization))
        .transpose()
        .map_err(|refusal| match refusal {
            PendingPackageForm::Form { form, node } => {
                PackageRunError::Pending(PendingPackageFamily {
                    request_index: None,
                    note: None,
                    family: PendingFamily::ModelForm(form),
                    node: Some(node),
                })
            }
            refusal @ (PendingPackageForm::World(_)
            | PendingPackageForm::Model(_)
            | PendingPackageForm::Operations(_)
            | PendingPackageForm::ProbabilisticNodes(_)
            | PendingPackageForm::Probabilistic(_)
            | PendingPackageForm::Reward(_)
            // Only `source_object_probabilistic_model` reads the source-object
            // rows, so `of` never gives this one.
            | PendingPackageForm::SourceModel(_)) => PackageRunError::Input(refusal),
        })?;
    if input
        .as_ref()
        .is_some_and(|input| !input.input().time_sources.is_empty())
    {
        if named
            .iter()
            .any(|item| !matches!(item, Named::Settled(_) | Named::StateGraph { .. }))
        {
            return Err(PackageRunError::Pending(PendingPackageFamily {
                request_index: None,
                note: None,
                family: PendingFamily::ModelForm(UnreadModelForm::TimedModel),
                node: package
                    .first_unread_model_form()
                    .map(|(_, node)| node.clone()),
            }));
        }
    }
    let mut registry = Registry::new();
    for descriptor in engines.registrations() {
        registry
            .register(descriptor.clone())
            .map_err(|_| AnalyzeDocumentError::Fault)?;
    }
    let reads = ClaimReads {
        package,
        input: input.as_ref(),
        package_bytes,
        supply,
        binding: &binding,
        limits,
        subject: &subject,
        registry: &registry,
        explicit_items,
    };
    let mut resolved = Vec::with_capacity(named.len());
    let mut routes: BTreeMap<RequestIndex, Route<'_>> = BTreeMap::new();
    let mut owners: BTreeMap<OccurrenceKey, Option<String>> = BTreeMap::new();
    let mut clause_claims: Vec<OccurrenceKey> = Vec::new();
    let mut invariant_claims: Vec<OccurrenceKey> = Vec::new();
    let mut graph_claims: Vec<OccurrenceKey> = Vec::new();
    let mut protocol_claims: Vec<OccurrenceKey> = Vec::new();
    let mut value_claims: Vec<OccurrenceKey> = Vec::new();
    let mut hyper_claims: Vec<OccurrenceKey> = Vec::new();
    let mut relation_claims: Vec<OccurrenceKey> = Vec::new();
    let mut schedule_claims: Vec<OccurrenceKey> = Vec::new();
    let mut probability_claims: Vec<OccurrenceKey> = Vec::new();
    for item in named {
        match item {
            Named::Simulation(item) => resolved.push(ResolvedItem::Simulation(Box::new(item))),
            Named::Hyper {
                index,
                name,
                declaration,
                ordinal,
                selection,
            } => match reads.hyper(index, name, &declaration, ordinal, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, subject) => {
                    hyper_claims.push(claim.item.claim.clone());
                    routes.insert(index, Route::Hyper(subject));
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::Probability {
                index,
                name,
                declaration,
                ordinal,
                selection,
            } => match reads.probability(index, name, &declaration, ordinal, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, ()) => {
                    probability_claims.push(claim.item.claim.clone());
                    routes.insert(index, Route::Probability);
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::Refinement {
                index,
                name,
                declaration,
                selection,
            } => match reads.refinement(index, name, &declaration, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, subject) => {
                    relation_claims.push(claim.item.claim.clone());
                    routes.insert(index, Route::Refinement(subject));
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::Relation {
                index,
                name,
                declaration,
                ordinal,
                selection,
            } => match reads.model_relation(index, name, &declaration, ordinal, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, subject) => {
                    relation_claims.push(claim.item.claim.clone());
                    routes.insert(index, Route::Relation(subject));
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::Settled(settled) => resolved.push(settled),
            Named::Clause {
                index,
                name,
                claim,
                selection,
            } => match reads.clause(index, name, &claim, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, ClauseRoute { subject, protocol }) => {
                    let occurrence = subject.occurrence().clone();
                    owners.insert(
                        occurrence.clone(),
                        owner_of_clause(package, subject.clause()),
                    );
                    clause_claims.push(occurrence);
                    routes.insert(index, Route::Clause { subject, protocol });
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::Deadlock {
                index,
                name,
                kind,
                selection,
            } => match reads.deadlock(index, name, kind, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, ()) => {
                    routes.insert(index, Route::Deadlock);
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::ProtocolFreedom {
                index,
                name,
                protocol,
                selection,
            } => match reads.protocol_freedom(index, name, &protocol, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, protocol) => {
                    routes.insert(index, Route::ProtocolFreedom(protocol));
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::ProtocolAnalysis {
                index,
                name,
                selection,
            } => match reads.protocol_analysis(index, name, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, ()) => {
                    protocol_claims.push(claim.item.claim.clone());
                    routes.insert(index, Route::ProtocolAnalysis);
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::SuppliedProtocol { index, name, selection } => match reads.supplied_protocol(index, name, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, ()) => {
                    protocol_claims.push(claim.item.claim.clone());
                    routes.insert(index, Route::SuppliedProtocol(name.to_owned()));
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::StateClause {
                index,
                name,
                declaration,
                ordinal,
                selection,
            } => match reads.state_clause(index, name, &declaration, ordinal, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, ()) => {
                    invariant_claims.push(claim.item.claim.clone());
                    routes.insert(index, Route::StateClause);
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::Value {
                index,
                name,
                selection,
                claim,
            } => match reads.value(index, name, selection, claim.as_ref())? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, target) => {
                    if let Some(target) = target {
                        value_claims.push(claim.item.claim.clone());
                        routes.insert(index, Route::Value(Box::new(target)));
                    }
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::Schedule {
                index,
                name,
                declaration,
                ordinal,
                selection,
            } => match reads.schedule(index, name, &declaration, ordinal, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, route) => {
                    schedule_claims.push(claim.item.claim.clone());
                    routes.insert(index, Route::Schedule(Box::new(route)));
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
            Named::StateGraph {
                index,
                name,
                declaration,
                ordinal,
                selection,
            } => match reads.state_graph(index, name, &declaration, ordinal, selection)? {
                Resolution::Settled(settled) => resolved.push(settled),
                Resolution::Claim(claim, subject) => {
                    let occurrence = claim.item.claim.clone();
                    owners.insert(occurrence.clone(), owner_of_graph(package, subject.claim()));
                    graph_claims.push(occurrence);
                    routes.insert(index, Route::StateGraph(subject));
                    resolved.push(ResolvedItem::Claim(claim));
                }
            },
        }
    }
    // A default derived item is named by no claim (FR-620), as the source
    // route writes it; an authored one is named by its model's identity.
    if defaults.is_some() {
        for (selection, item) in items.iter().zip(resolved.iter_mut()) {
            let derived = selection
                .temporal
                .as_ref()
                .is_some_and(|temporal| temporal.item_kind.is_some());
            if let (true, ResolvedItem::Claim(claim)) = (derived, item) {
                claim.claim_name.clear();
            }
        }
    }
    let model_input = input.as_ref().map(PackageModelInput::input);
    let admit = |item: &mut ClaimItem| match &model_input {
        Some(model) => super::admit_reductions(model, &binding, &limits, item),
        None => Ok(()),
    };
    // As the source route's owning model: a clause's `over` object type's
    // package, else the package's one model selection.
    let owner = |claim: &OccurrenceKey| {
        owners
            .get(claim)
            .cloned()
            .unwrap_or_else(|| sole_model(package))
    };
    let family = |kind: ItemKind, claim: &OccurrenceKey| match kind {
        ItemKind::Formula if probability_claims.contains(claim) => {
            Some(DeclarationFamily::Probabilistic)
        }
        ItemKind::Formula if protocol_claims.contains(claim) => Some(DeclarationFamily::Protocol),
        ItemKind::Formula if value_claims.contains(claim) => Some(DeclarationFamily::Value),
        ItemKind::Formula if schedule_claims.contains(claim) => Some(DeclarationFamily::RealTime),
        ItemKind::Formula if relation_claims.contains(claim) => {
            Some(DeclarationFamily::RelationClaim)
        }
        ItemKind::Formula if invariant_claims.contains(claim) => {
            Some(DeclarationFamily::StateModel)
        }
        ItemKind::Formula => (clause_claims.contains(claim)
            || graph_claims.contains(claim)
            || hyper_claims.contains(claim))
        .then_some(DeclarationFamily::TemporalTrace),
        other => crate::claim_family::kind_family(other),
    };
    let Negotiation {
        components: _,
        gates: _,
        negotiated,
        unrouted_items,
        settled,
        echoes,
        simulations,
    } = negotiate_items(
        resolved,
        negotiate,
        &ItemReads {
            admit: &admit,
            owner: &owner,
            family: &family,
        },
    )?;
    let mut analyzed = Vec::with_capacity(negotiated.len());
    if let (Some(input), Some(model)) = (input.as_ref(), model_input.as_ref()) {
        // FR-624: the shared subject binding is admitted before any item
        // runs, as on the source route.
        fsl_eval::simulation::model::ModelSystem::new(super::super::request::simulation(
            model, &binding, &limits,
        ))
        .map_err(|failure| {
            AnalyzeDocumentError::Refused(super::super::AnalyzeRefusal::Binding(failure))
        })?;
        // A provider's request over the package carries the admitted bytes
        // and their dependency closure, read once for every item it decides.
        let provider_decides = |item: &NegotiatedItem| {
            matches!(routes.get(&item.item.request_index), Some(Route::Value(_)))
                && item.disposition.target().is_some_and(|backend| {
                    engines
                        .get(backend.id())
                        .is_some_and(|engine| engine.provider().is_some())
                })
        };
        let admitted = negotiated.iter().any(provider_decides).then(|| {
            std::sync::Arc::new(crate::provider::AdmittedPackage {
                name: input.package().package().name().to_owned(),
                bytes: package_bytes.to_vec(),
                dependencies: supply
                    .dependencies
                    .iter()
                    .map(|dependency| fsl_replay::HybridProcessDependency {
                        identity: dependency.identity.clone(),
                        package: dependency.bytes.to_vec(),
                    })
                    .collect(),
            })
        });
        // EN-6 decides a task set over a task-product input the package
        // route does not read yet.
        for item in &negotiated {
            let zone = matches!(
                routes.get(&item.item.request_index),
                Some(Route::Schedule(_))
            ) && item.disposition.target().is_some_and(|backend| {
                backend.id().as_str() == crate::zone_check::manifest::BACKEND_ID
            });
            if zone {
                return Err(PackageRunError::Pending(PendingPackageFamily {
                    request_index: Some(item.item.request_index),
                    note: None,
                    family: PendingFamily::TaskProduct,
                    node: None,
                }));
            }
        }
        let work = WorkInputs {
            engines,
            admitted,
            input,
            package_bytes,
            supply,
            binding: &binding,
            limits,
            family: &family,
        };
        for item in &negotiated {
            let route = routes.remove(&item.item.request_index);
            analyzed.push(work.item(route, item));
        }
    }
    let records = super::super::analyze_items(engines, analyzed, cancel);
    let details = records.iter().map(super::super::ItemDetail::of).collect();
    let protocol_conformance = records
        .iter()
        .filter_map(|record| {
            record
                .protocol_conformance
                .clone()
                .map(|output| (record.request_index, output))
        })
        .collect();
    let (records, counterexamples) = super::super::split_records_over(records, None);
    let (targets, item_routes) = negotiated_routes(&negotiated, &unrouted_items, engines);
    let mut simulated = Vec::new();
    if let Some(model) = model_input.as_ref() {
        for item in &simulations {
            match super::super::simulation_item::simulate(model, &binding, &limits, item, cancel) {
                Ok(written) => simulated.push(written),
                Err(super::super::simulation_item::SimulationFault::Binding(failure)) => {
                    return Err(AnalyzeDocumentError::Refused(
                        super::super::AnalyzeRefusal::Binding(failure),
                    )
                    .into());
                }
                Err(super::super::simulation_item::SimulationFault::Fault) => {
                    return Err(AnalyzeDocumentError::Fault.into());
                }
            }
        }
    }
    Ok(write_run(
        SettledParts {
            records,
            counterexamples,
            details,
            protocol_conformance,
            graphs: super::super::DocumentGraphs {
                own: super::super::DocumentGraphs::of_bytes(package_bytes),
                dependencies: BTreeMap::new(),
            },
        },
        &settled,
        &echoes,
        &targets,
        &item_routes,
        &simulated,
        engines,
    )?)
}

/// The model package owning a temporal clause: its `over` object type's
/// package, else the package's one model selection.
fn owner_of_clause(
    package: &BarePackage<'_>,
    clause: &fsl_semantics::check::CheckedTemporalClause,
) -> Option<String> {
    match &clause.over().1 {
        filament_exact::ValueType::Reference(object_type) => {
            Some(object_type.original().package().to_owned())
        }
        _ => sole_model(package),
    }
}

/// The model package owning a state-graph claim: its `over` object type's
/// package, else the package's one model selection.
fn owner_of_graph(
    package: &BarePackage<'_>,
    claim: &fsl_semantics::check::CheckedStateGraphClaim,
) -> Option<String> {
    match claim.over() {
        Some(over) => Some(over.object.original().package().to_owned()),
        None => sole_model(package),
    }
}

/// Whether the package node `node` is a `claim`/`state_graph` node (FR-410).
fn is_state_graph(view: &dyn AnalysisView, node: &CheckedNodeId) -> bool {
    view.package()
        .resolved_graph()
        .nodes
        .iter()
        .find(|candidate| candidate.node_id == *node)
        .is_some_and(|candidate| {
            &*candidate.node_tag == "claim" && candidate.semantic_form.as_ref() == "state_graph"
        })
}

/// The package's one model selection, when it has exactly one.
fn sole_model(package: &BarePackage<'_>) -> Option<String> {
    match AnalysisView::model_selections(package).as_slice() {
        [selection] => Some(selection.identity.clone()),
        _ => None,
    }
}

/// FSL FR-740, FR-940-AC-6: the located refusal of the first item that names
/// a probabilistic claim of a package with source objects.
///
/// The package's source-object rows rebuild the probabilistic model
/// ([`PackageModelInput::source_object_probabilistic_model`]) but carry no
/// effect frame, field or population, so no engine steps the model and no
/// probabilistic item runs: a claim whose model rebuilds is refused
/// [`PendingFamily::ProbabilisticSourceModel`], and one whose model does not
/// [`PendingFamily::ProbabilisticClaimUnresolved`], each at the claim's node.
/// `None` for a package with no source objects and for a request that names
/// no probabilistic claim, whose items settle as every other item does. A
/// package fault the model-check input would give is the same fault here.
fn probabilistic_source_refusal(
    package: &BarePackage<'_>,
    packages: &fsl_semantics::model::intake::PackageInput,
    normalization: fsl_semantics::model::accounting::ModelNormalizationLimits,
    items: &[ItemSelection],
) -> Result<Option<PendingPackageFamily>, PackageRunError> {
    if !matches!(
        package.first_unread_model_form(),
        Some((UnreadModelForm::SourceObject, _))
    ) {
        return Ok(None);
    }
    let view: &dyn AnalysisView = package;
    let declared = view.declared_names();
    let Some((index, name, node)) = items.iter().enumerate().find_map(|(position, item)| {
        let name = item.claim.as_deref()?;
        let node = declared
            .iter()
            .find(|(declared, _)| declared == name)
            .map(|(_, node)| (*node).clone())
            .or_else(|| view.claim_name(name).map(|found| found.declaration))?;
        view.package()
            .probabilistic_claims()
            .iter()
            .any(|claim| same_claim_node(&claim.node, &node))
            .then(|| (RequestIndex::new(position), name, node))
    }) else {
        return Ok(None);
    };
    let (family, note) = match PackageModelInput::source_object_probabilistic_model(
        package,
        packages,
        normalization,
    ) {
        Ok(_) => (PendingFamily::ProbabilisticSourceModel, format!("`{name}`")),
        Err(refusal) => match unresolved_reason(&refusal) {
            Some(reason) => (
                PendingFamily::ProbabilisticClaimUnresolved,
                format!("`{name}` ({reason})"),
            ),
            // The same faults `PackageModelInput::of` gives settle as a
            // fault, not as a gap of the route.
            None => return Err(PackageRunError::Document(AnalyzeDocumentError::Fault)),
        },
    };
    Ok(Some(PendingPackageFamily {
        request_index: Some(index),
        note: Some(note),
        family,
        node: Some(node),
    }))
}

/// Whether two ids name one claim node: the read of a node's path spells its
/// operands by index in one place and by node in another, so ids compare by
/// the package, ordinal and the leading `claim/probabilistic_claim/<name>`
/// segments, which no operand spelling reaches.
fn same_claim_node(left: &CheckedNodeId, right: &CheckedNodeId) -> bool {
    left.package == right.package
        && left.ordinal == right.ordinal
        && left.path.iter().take(3).eq(right.path.iter().take(3))
}

/// Why a package's probabilistic model does not rebuild, in words, for a
/// refusal that is a gap of the route; `None` for a fault of the package
/// (an unreadable environment, domain packages not admitted as a model,
/// untyped operations or unreadable probabilistic nodes), which
/// [`settle_package_run`] settles as a fault as it does for `of`.
fn unresolved_reason(refusal: &PendingPackageForm) -> Option<String> {
    match refusal {
        PendingPackageForm::Form { .. } => {
            Some("the package holds a form its model-check input does not read yet".to_owned())
        }
        PendingPackageForm::SourceModel(fault) => Some(fault.to_string()),
        PendingPackageForm::Probabilistic(_) => Some(
            "the package's random parameters and workloads do not rebuild against its model"
                .to_owned(),
        ),
        PendingPackageForm::Reward(_) => {
            Some("a reward's clause function does not project from the package".to_owned())
        }
        PendingPackageForm::World(_)
        | PendingPackageForm::Model(_)
        | PendingPackageForm::Operations(_)
        | PendingPackageForm::ProbabilisticNodes(_) => None,
    }
}

/// FR-850 "Building the analysis input": each item resolved by name against
/// the package, read only through the view (FR-940). A temporal clause is
/// named by its `temporal_clause_names` row; a derived deadlock-freedom item
/// by the identity of the model selection the package records. A name
/// neither carries settles `invalid-request` `invalid_capability`/
/// `unknown-claim` as the source route settles it, but only when the
/// package holds no claim whose name it does not carry (an `invariant`
/// state clause, a refinement, a state-graph claim) and no component
/// composition, which the package form does not read yet: the name could be
/// that claim's or composition's.
fn resolve_names<'s>(
    view: &dyn AnalysisView,
    selections: &'s [ItemSelection],
) -> Result<Vec<Named<'s>>, PackageRunError> {
    let pending = |request_index, family| {
        PackageRunError::Pending(PendingPackageFamily {
            request_index,
            note: None,
            family,
            node: None,
        })
    };
    let clauses = view.clause_names();
    let declared = view.declared_names();
    let unnamed = !view.unnamed_claims().is_empty() || admitted_composition(view).is_some();
    let models: Vec<String> = view
        .model_selections()
        .into_iter()
        .map(|selection| selection.identity)
        .collect();
    selections
        .iter()
        .enumerate()
        .map(|(position, selection)| {
            let index = RequestIndex::new(position);
            let pending = |family| pending(Some(index), family);
            let item_kind = selection
                .temporal
                .as_ref()
                .and_then(|temporal| temporal.item_kind)
                .map(super::ItemKindSelection::kind);
            match item_kind {
                Some(ItemKind::LoopInvariant) => {
                    if let Some(name) = selection.claim.as_deref() {
                        if let Some(protocol) = protocol_literal_repeat_name(view, name) {
                            return Ok(Named::ProtocolFreedom {
                                index,
                                name,
                                protocol,
                                selection,
                            });
                        }
                    }
                    return Err(pending(PendingFamily::LoopInvariant));
                }
                Some(ItemKind::LoopTermination) => {
                    return Err(pending(PendingFamily::LoopTermination));
                }
                Some(
                    ItemKind::Formula
                    | ItemKind::DeadlockFreedom
                    | ItemKind::TimeLockFreedom
                    | ItemKind::RaceFreedom
                    | ItemKind::Refinement,
                )
                | None => {}
            }
            let Some(name) = selection.claim.as_deref() else {
                if let (Some(super::SIMULATION_KIND), Some(settings)) =
                    (selection.kind.as_deref(), selection.simulation.as_ref())
                {
                    return Ok(Named::Simulation(super::super::SimulationItem {
                        index,
                        settings: settings.clone(),
                        subject: selection
                            .temporal
                            .as_ref()
                            .and_then(|temporal| temporal.subject.clone()),
                    }));
                }
                return Err(pending(PendingFamily::Simulation));
            };
            let derived = item_kind.is_some_and(|kind| kind != ItemKind::Formula);
            let protocol = if matches!(
                selection.kind.as_deref(),
                Some("local-projection" | "realizability" | "finite-replay" | "global-conformance")
            ) || (item_kind == Some(ItemKind::DeadlockFreedom)
                && selection.kind.as_deref() == Some(Capability::TemporalSatisfaction.to_wire()))
            {
                match view.protocol_execution_named(name) {
                    Ok(form) => form.is_some(),
                    Err(fsl_package::protocol_view::ViewRefusal::MalformedMember {
                        node,
                        member: "operator",
                    }) => {
                        let mut detail = Map::new();
                        detail.insert("fact".to_owned(), json!("operator"));
                        detail.insert("node".to_owned(), json!(node));
                        return Ok(Named::Settled(ResolvedItem::Settled {
                            claim_name: Some(name.to_owned()),
                            record: Box::new(TerminalRecord::disposition_unsupported(
                                index,
                                CatalogCode::new("unsupported_construct", "not-yet-implemented"),
                                detail,
                            )),
                        }));
                    }
                    Err(_) => return Err(PackageRunError::Document(AnalyzeDocumentError::Fault)),
                }
            } else {
                false
            };
            if matches!(selection.kind.as_deref(), Some("finite-replay" | "global-conformance")) && protocol {
                return Ok(Named::SuppliedProtocol { index, name, selection });
            }
            if matches!(
                selection.kind.as_deref(),
                Some("local-projection" | "realizability")
            ) && protocol
            {
                return Ok(Named::ProtocolAnalysis {
                    index,
                    name,
                    selection,
                });
            }
            if item_kind == Some(ItemKind::DeadlockFreedom)
                && selection.kind.as_deref() == Some(Capability::TemporalSatisfaction.to_wire())
                && protocol
            {
                return Ok(Named::ProtocolFreedom {
                    index,
                    name,
                    protocol: name.to_owned(),
                    selection,
                });
            }
            if item_kind == Some(ItemKind::RaceFreedom) {
                if let Some(protocol) = protocol_parallel_name(view, name) {
                    return Ok(Named::ProtocolFreedom {
                        index,
                        name,
                        protocol,
                        selection,
                    });
                }
            }
            if let Some(row) = clauses.iter().find(|row| row.name.as_ref() == name) {
                if derived {
                    return Err(pending(PendingFamily::Derived));
                }
                let key =
                    fsl_package::checked_views::temporal_subject(view.package(), &row.declaration)
                        .ok()
                        .and_then(|subject| subject.declaration_key())
                        .ok_or(PackageRunError::Document(AnalyzeDocumentError::Fault))?;
                // W58: the declaration is its own `claim` occurrence of a
                // node structurally equal clauses may share.
                let claim = OccurrenceKey::new(
                    key,
                    filament_exact::Origin::new(filament_exact::Role::new("claim"), row.occurrence),
                );
                return Ok(Named::Clause {
                    index,
                    name,
                    claim,
                    selection,
                });
            }
            // FR-850-AC-7, FR-940-AC-7: any other claim by its `claim_names`
            // row. An `invariant` state clause runs over the package route's
            // input; a claim of another family has no package form yet.
            if let Some(found) = view.claim_name(name) {
                if matches!(
                    view.package().term(&found.declaration),
                    Some(filament_contract_model::CheckedTerm::Refinement { .. })
                ) {
                    // Derived deadlock and time-lock items name a model alias,
                    // not a refinement declaration, as on the source route.
                    if matches!(
                        item_kind,
                        Some(ItemKind::DeadlockFreedom | ItemKind::TimeLockFreedom)
                    ) {
                        return Ok(Named::Settled(unknown_claim(index, name)));
                    }
                    return Ok(Named::Refinement {
                        index,
                        name,
                        declaration: found.declaration,
                        selection,
                    });
                }
                if derived {
                    return Err(pending(PendingFamily::Derived));
                }
                if view.package().resolved_graph().nodes.iter().any(|node| {
                    node.node_id == found.declaration
                        && node.semantic_form.as_ref() == "probabilistic_claim"
                }) {
                    return Ok(Named::Probability {
                        index,
                        name,
                        declaration: found.declaration,
                        ordinal: found.occurrence.ordinal,
                        selection,
                    });
                }
                if view.package().resolved_graph().nodes.iter().any(|node| {
                    node.node_id == found.declaration && node.step_relation_body().is_some()
                }) {
                    return Ok(Named::Relation {
                        index,
                        name,
                        declaration: found.declaration,
                        ordinal: found.occurrence.ordinal,
                        selection,
                    });
                }
                if view.package().resolved_graph().nodes.iter().any(|node| {
                    node.node_id == found.declaration
                        && &*node.node_tag == "claim"
                        && node.semantic_form.as_ref() == "hyperproperty"
                }) {
                    return Ok(Named::Hyper {
                        index,
                        name,
                        declaration: found.declaration,
                        ordinal: found.occurrence.ordinal,
                        selection,
                    });
                }
                if is_schedulability(view, &found.declaration) {
                    return Ok(Named::Schedule {
                        index,
                        name,
                        declaration: found.declaration,
                        ordinal: found.occurrence.ordinal,
                        selection,
                    });
                }
                if is_state_graph(view, &found.declaration) {
                    return Ok(Named::StateGraph {
                        index,
                        name,
                        declaration: found.declaration,
                        ordinal: found.occurrence.ordinal,
                        selection,
                    });
                }
                if !is_state_clause(view, &found.declaration) {
                    return Err(pending(declaration_pending_family(
                        view,
                        &found.declaration,
                    )));
                }
                return Ok(Named::StateClause {
                    index,
                    name,
                    declaration: found.declaration,
                    ordinal: found.occurrence.ordinal,
                    selection,
                });
            }
            if let Some((_, declaration)) = declared.iter().find(|(declared, _)| declared == name) {
                if !derived
                    && selection.kind.as_deref() == Some(Capability::ValueValidity.to_wire())
                {
                    return Ok(Named::Value {
                        index,
                        name,
                        selection,
                        claim: None,
                    });
                }
                return Err(pending(declaration_pending_family(view, declaration)));
            }
            if derived {
                let selected = models.iter().any(|identity| identity == name);
                return match (item_kind, selected, models.len()) {
                    (
                        Some(
                            kind @ (ItemKind::DeadlockFreedom
                            | ItemKind::TimeLockFreedom
                            | ItemKind::RaceFreedom),
                        ),
                        true,
                        _,
                    ) => Ok(Named::Deadlock {
                        index,
                        name,
                        kind,
                        selection,
                    }),
                    (_, true, _) => Err(pending(PendingFamily::Derived)),
                    (_, false, _) => Ok(Named::Settled(model_alias(index, name, &models))),
                };
            }
            if unnamed {
                return Err(pending(PendingFamily::UnnamedClaim));
            }
            Ok(Named::Settled(unknown_claim(index, name)))
        })
        .collect()
}

/// Refine only the pending diagnostic of an already resolved declaration.
fn declaration_pending_family(view: &dyn AnalysisView, node: &CheckedNodeId) -> PendingFamily {
    if matches!(
        view.package().node_kind(node),
        Some(filament_contract_model::CheckedNodeKind::Correspondence(
            filament_contract_model::CorrespondenceForm::ComponentComposition
        ))
    ) {
        PendingFamily::CompositionOwningWorkPermission
    } else {
        PendingFamily::Declaration
    }
}

/// The protocol name of `name` when it names one of the package protocol's
/// memory-governed parallel controls (`Protocol::path`).
fn protocol_parallel_name(view: &dyn AnalysisView, name: &str) -> Option<String> {
    let (protocol, path) = name.split_once("::")?;
    let form = view.protocol_execution_named(protocol).ok().flatten()?;
    form.controls
        .iter()
        .any(|control| {
            control.operation == fsl_package::protocol_view::ControlOperation::Parallel
                && control.path.join("::") == path
        })
        .then(|| protocol.to_owned())
}

/// Resolve only literal Boolean invariants, before model/type work begins.
fn protocol_literal_repeat_name(view: &dyn AnalysisView, name: &str) -> Option<String> {
    fn member<'a>(
        term: &'a fsl_package::CheckedTerm,
        selected: &str,
    ) -> Option<&'a fsl_package::CheckedTerm> {
        use fsl_package::CheckedTerm;
        let terms = match term {
            CheckedTerm::Application { arguments, .. } => arguments,
            CheckedTerm::Aggregate { members } => members,
            _ => return None,
        };
        terms.iter().find_map(|term| match term {
            CheckedTerm::Binding { name, value, .. } if &**name == selected => Some(&**value),
            _ => None,
        })
    }
    let (protocol, path) = name.split_once("::")?;
    let form = view.protocol_execution_named(protocol).ok().flatten()?;
    let repeat = form.controls.iter().find(|control| {
        control.operation == fsl_package::protocol_view::ControlOperation::Repeat
            && control.path.join("::") == path
    })?;
    let invariant =
        member(&repeat.node.body, "proof").and_then(|proof| member(proof, "invariant"))?;
    let invariant = match invariant {
        fsl_package::CheckedTerm::Reference { target } => {
            if view.package().node_kind(target)
                != Some(fsl_package::CheckedNodeKind::Value(
                    fsl_package::ValueForm::Literal,
                ))
            {
                return None;
            }
            view.package().term(target)?
        }
        term => term,
    };
    matches!(
        invariant.literal_read()?.value,
        fsl_package::CheckedLiteralValue::Boolean(_)
    )
    .then(|| protocol.to_owned())
}

/// The checked protocol binding carried by an item subject, if it is a
/// protocol subject. `None` distinguishes a model subject; malformed
/// capacities or role bindings name the member for the request's ordinary
/// invalid-value disposition.
fn protocol_binding(
    subject: &SubjectSelection,
    input: &PackageModelInput<'_>,
) -> Result<Option<ProtocolBinding>, &'static str> {
    let Some(selection) = subject.protocol.as_ref() else {
        return Ok(None);
    };
    let capacities = selection
        .capacities
        .iter()
        .map(|(name, value)| {
            value.parse().ok().map(|value| CapacityBinding {
                name: name.clone(),
                value,
            })
        })
        .collect::<Option<Vec<_>>>()
        .ok_or("capacities")?;
    let memory: Vec<_> = subject
        .memory
        .iter()
        .filter(|entry| entry.parallel.split("::").next() == Some(selection.protocol.as_str()))
        .cloned()
        .collect();
    let effective = input
        .package()
        .protocol_execution_named(&selection.protocol)
        .ok()
        .flatten()
        .and_then(|form| {
            fsl_eval::simulation::protocol::ResolvedMemory::resolve_package(&form, &memory).ok()
        })
        .map(|resolved| {
            resolved
                .parallels()
                .iter()
                .map(|parallel| (parallel.parallel.clone(), parallel.model))
                .collect()
        })
        .unwrap_or_default();
    Ok(Some(ProtocolBinding {
        protocol: selection.protocol.clone(),
        over: selection.over.as_ref().map(|over| {
            (
                over.reference.population.clone(),
                over.reference.key.clone(),
            )
        }),
        roles: selection
            .roles
            .iter()
            .map(|(role, binding)| RoleBinding {
                role: role.clone(),
                model: binding.selection.clone(),
            })
            .collect(),
        capacities,
        memory,
        effective,
    }))
}

/// Whether the package node `node` names a state clause. The kind determines
/// whether FR-489 routes it to `contract-checker` or EN-1.
fn is_state_clause(view: &dyn AnalysisView, node: &CheckedNodeId) -> bool {
    view.package()
        .resolved_graph()
        .nodes
        .iter()
        .find(|candidate| candidate.node_id == *node)
        .is_some_and(|candidate| candidate.semantic_form.as_ref() == "state_clause")
}

/// FR-850: a derived item whose `claim` names no claim row and no model
/// identity the package records settles `invalid-request`
/// `invalid_capability`/`model-alias`, naming the claim and every recorded
/// identity in model-selection order.
fn model_alias(index: RequestIndex, name: &str, identities: &[String]) -> ResolvedItem {
    let mut detail = Map::new();
    detail.insert("name".to_owned(), json!(name));
    detail.insert("identities".to_owned(), json!(identities));
    ResolvedItem::Settled {
        claim_name: Some(name.to_owned()),
        record: Box::new(TerminalRecord::disposition_invalid_request(
            index,
            CatalogCode::new("invalid_capability", "model-alias"),
            detail,
        )),
    }
}

/// What a resolved claim reads beside its name.
enum Resolution<R> {
    Settled(ResolvedItem),
    Claim(Box<ResolvedClaim>, R),
}

/// What the package route's claim resolution reads.
struct ClaimReads<'r, 'a> {
    package: &'a BarePackage<'a>,
    input: Option<&'r PackageModelInput<'a>>,
    package_bytes: &'a [u8],
    supply: fsl_replay::HandoffSupply<'a>,
    binding: &'r SubjectBinding<'a>,
    limits: AnalyzeLimits,
    subject: &'r SubjectSelection,
    registry: &'r Registry,
    explicit_items: bool,
}

impl<'r, 'a: 'r> ClaimReads<'r, 'a> {
    fn refinement(
        &self,
        index: RequestIndex,
        name: &str,
        declaration: &CheckedNodeId,
        selection: &ItemSelection,
    ) -> Result<Resolution<Box<PackageRefinementSubject<'r>>>, PackageRunError> {
        use filament_contract_model::{
            CapabilityKind, DomainKey as WireDomain, DomainKind as WireKind, RequirementExtent,
        };
        use fsl_foundation::bound::{DomainKey, DomainKind};
        use fsl_semantics::family::{ClaimExtent, Requirements};
        let fault = || PackageRunError::Document(AnalyzeDocumentError::Fault);
        let pending = |fact| {
            PackageRunError::Pending(PendingPackageFamily {
                request_index: Some(index),
                family: PendingFamily::RefinementFact(fact),
                node: Some(declaration.clone()),
                note: None,
            })
        };
        let input = self.input.ok_or_else(fault)?;
        let package = input.package();
        let key = package.node_key(declaration).ok_or_else(fault)?;
        let claim = OccurrenceKey::declaration(key.clone());
        let recorded = package
            .package()
            .requirement_items()
            .iter()
            .find(|row| {
                row.claim == *declaration
                    && row.occurrence.role
                        == filament_contract_model::CheckedOccurrenceRole::Declaration
                    && row.occurrence.ordinal == 0
                    && row.kind == CapabilityKind::TemporalSatisfaction
            })
            .ok_or_else(|| pending("refinement requirement record"))?;
        let extent = match &recorded.extent {
            RequirementExtent::Unbounded(domains)
                if matches!(domains.as_slice(),
                [(WireDomain::Node { node, path }, WireKind::InfiniteTrace)] if node == declaration && path.is_empty()) =>
            {
                ClaimExtent::from_domains(
                    [(
                        DomainKey::Node {
                            node: key.clone(),
                            path: Vec::new(),
                        },
                        DomainKind::InfiniteTrace,
                    )]
                    .into_iter()
                    .collect(),
                )
            }
            _ => return Err(pending("refinement requirement extent")),
        };
        let subject = PackageRefinementSubject::new(
            input,
            declaration,
            self.binding.clone(),
            self.limits,
            claim.clone(),
        )
        .map_err(|refusal| refusal.pending_fact().map_or_else(fault, pending))?;
        let kind = match selection.kind.as_deref() {
            Some("refinement") | None => Capability::TemporalSatisfaction,
            Some(label) => match Capability::from_wire(label) {
                Ok(kind) => kind,
                Err(_) => {
                    return Ok(Resolution::Settled(invalid_kind(
                        index,
                        Some(name),
                        "unknown-kind",
                    )))
                }
            },
        };
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if let Some(refusal) = unknown_option(
            index,
            Some(name),
            self.registry,
            named.as_ref(),
            &backend_options,
        ) {
            return Ok(Resolution::Settled(refusal));
        }
        let requirements = Requirements::new(Capability::TemporalSatisfaction, extent)
            .over_model_subject()
            .with_property_form(
                filament_semantic_value::fairness::PropertyForm::Refinement { liveness: false },
            );
        let candidates = self.registry.restrict_to_timing(
            if kind == Capability::TemporalSatisfaction {
                fsl_route::request::requirement_candidates(
                    &requirements,
                    self.registry,
                    named.as_ref(),
                )
            } else {
                self.registry.candidates(kind, named.as_ref())
            },
            false,
        );
        let temporal = selection.temporal.clone().unwrap_or_default();
        let mut form = temporal_settings(&temporal);
        form.pending_reductions.clone_from(&selection.reductions);
        let own_subject = self.own_subject(index, selection)?;
        let item = ClaimItem {
            request_index: index,
            claim,
            kind,
            extent: ExtentClassification::Unbounded {
                finite_bound_available: false,
            },
            backend: named,
            candidates: candidate_list(&candidates),
            options: options(selection, backend_options),
            form: (kind == Capability::TemporalSatisfaction).then_some(ItemForm::Temporal(form)),
        };
        let unrouted = selection.unrouted()
            || reads_no_clocks(selection)
            || temporal
                .limits
                .is_some_and(|limits| limits.sets_memory_limits())
            || !own_subject.same_model(self.subject);
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                subject: own_subject,
                unrouted,
            }),
            Box::new(subject),
        ))
    }

    /// The item's own subject over the request's, including a protocol
    /// subject selected by this item.
    fn own_subject(
        &self,
        _index: RequestIndex,
        selection: &ItemSelection,
    ) -> Result<SubjectSelection, PackageRunError> {
        let own = selection
            .temporal
            .as_ref()
            .and_then(|temporal| temporal.subject.clone())
            .map_or_else(
                || self.subject.clone(),
                |own| own.completed_by(self.subject),
            );
        Ok(own)
    }

    /// FR-620's item over the temporal clause `name` at `key`, as the
    /// source route's `resolve_one` resolves an authored clause: its claim
    /// occurrence from the clause's name row, the record's kind, extent and
    /// candidates from the package form's claim.
    fn clause(
        &self,
        index: RequestIndex,
        name: &str,
        claim: &OccurrenceKey,
        selection: &ItemSelection,
    ) -> Result<Resolution<ClauseRoute<'r>>, PackageRunError> {
        let input = self
            .input
            .ok_or(PackageRunError::Document(AnalyzeDocumentError::Fault))?;
        let own_subject = self.own_subject(index, selection)?;
        let temporal = selection.temporal.clone().unwrap_or_default();
        let protocol = match protocol_binding(&own_subject, input) {
            Ok(protocol) => protocol,
            Err(member) => {
                return Ok(Resolution::Settled(super::invalid_value(
                    index, name, member,
                )));
            }
        };
        if temporal
            .item_kind
            .is_some_and(|kind| kind.kind() != ItemKind::Formula)
            && protocol.is_none()
        {
            return Err(PackageRunError::Pending(PendingPackageFamily {
                request_index: Some(index),
                note: None,
                family: PendingFamily::Derived,
                node: None,
            }));
        }
        let mut form = temporal_settings(&temporal);
        let clause_subject = PackageClauseSubject::new(
            input,
            self.package_bytes,
            self.supply,
            self.binding.clone(),
            self.limits,
            claim,
            ClauseLimits {
                limits: form.limits.limits,
                max_depth: form.max_depth,
                max_candidates: form.max_candidates,
            },
        )
        .map_err(|refusal| match refusal {
            ClauseItemRefusal::Build(refusal) => PackageRunError::ClauseBuild {
                node: fsl_semantics::check::wire_id::structural_id(claim.node()),
                refusal,
            },
            ClauseItemRefusal::Pending { fact, node } => {
                PackageRunError::Pending(PendingPackageFamily {
                    request_index: Some(index),
                    note: None,
                    family: PendingFamily::ClauseFact(fact),
                    node: Some(node),
                })
            }
            ClauseItemRefusal::UnknownClause | ClauseItemRefusal::UnnamedClause => {
                PackageRunError::Document(AnalyzeDocumentError::Fault)
            }
        })?;
        let kind = match selection.kind.as_deref() {
            Some(label) => match Capability::from_wire(label) {
                Ok(kind) => kind,
                Err(_) => {
                    return Ok(Resolution::Settled(invalid_kind(
                        index,
                        Some(name),
                        "unknown-kind",
                    )));
                }
            },
            // A temporal clause's one record is `temporal-satisfaction`.
            None => Capability::TemporalSatisfaction,
        };
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if kind.applies_to_declaration(DeclarationFamily::TemporalTrace) {
            if let Some(refused) = unknown_option(
                index,
                Some(name),
                self.registry,
                named.as_ref(),
                &backend_options,
            ) {
                return Ok(Resolution::Settled(refused));
            }
        }
        let clause = clause_subject.clause();
        let record = kind == Capability::TemporalSatisfaction;
        let (extent, candidates) = if record {
            (
                clause_extent(clause),
                self.registry.restrict_to_timing(
                    fsl_route::request::requirement_candidates(
                        &clause
                            .claim()
                            .requirements_over(fsl_semantics::family::ClaimExtent::Bounded),
                        self.registry,
                        named.as_ref(),
                    ),
                    false,
                ),
            )
        } else {
            (
                ExtentClassification::Bounded,
                tie(
                    self.registry.candidates(kind, named.as_ref()),
                    kind,
                    named.as_ref(),
                ),
            )
        };
        form.fairness = clause.fairness().to_vec();
        form.pending_reductions.clone_from(&selection.reductions);
        form.protocol.clone_from(&protocol);
        let item_form = (kind == Capability::TemporalSatisfaction
            || kind == Capability::GlobalConformance)
            .then(|| ItemForm::Temporal(form));
        let item = ClaimItem {
            request_index: index,
            claim: clause_subject.occurrence().clone(),
            kind,
            extent,
            backend: named,
            candidates: candidate_list(&candidates),
            options: options(selection, backend_options),
            form: item_form,
        };
        let unrouted = selection.unrouted()
            || (selection.reductions.is_some() && kind != Capability::TemporalSatisfaction)
            || reads_no_clocks(selection)
            || temporal
                .limits
                .is_some_and(|limits| limits.sets_memory_limits())
            || !own_subject.same_model(self.subject);
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                subject: own_subject,
                unrouted,
            }),
            ClauseRoute {
                subject: Box::new(clause_subject),
                protocol,
            },
        ))
    }

    /// A Boolean value clause or an explicitly selected scalar record.
    /// Non-Boolean function names retain their declaration's inapplicable
    /// family; unsupported parameter shapes keep their typed refusal.
    fn value(
        &self,
        index: RequestIndex,
        name: &str,
        selection: &ItemSelection,
        selected_claim: Option<&OccurrenceKey>,
    ) -> Result<Resolution<Option<PackageValue<'a>>>, PackageRunError> {
        let pending = || {
            PackageRunError::Pending(PendingPackageFamily {
                request_index: Some(index),
                note: None,
                family: PendingFamily::Declaration,
                node: None,
            })
        };
        let functions = self
            .package
            .package()
            .resolved_graph()
            .nodes
            .iter()
            .filter(|node| {
                node.node_tag.as_ref() == "function"
                    && node.semantic_form.as_ref() == "pure_function"
                    && node.declaration.as_ref().is_some_and(|declared| {
                        declared.qualified_name.len() == 1
                            && declared.qualified_name[0].as_ref() == name
                    })
            })
            .collect::<Vec<_>>();
        let [function] = functions.as_slice() else {
            return Err(pending());
        };
        let inapplicable = || {
            let declarations = function
                .occurrences
                .iter()
                .filter(|occurrence| {
                    occurrence.role == filament_contract_model::CheckedOccurrenceRole::Declaration
                })
                .collect::<Vec<_>>();
            let [occurrence] = declarations.as_slice() else {
                return Err(pending());
            };
            let claim = OccurrenceKey::new(
                self.package
                    .node_key(&function.node_id)
                    .ok_or_else(pending)?,
                filament_exact::Origin::new(
                    filament_exact::Role::new("declaration"),
                    occurrence.ordinal,
                ),
            );
            let kind = Capability::ValueValidity;
            let named = selection.backend.as_deref().map(BackendId::new);
            let candidates = tie(
                self.registry.candidates(kind, named.as_ref()),
                kind,
                named.as_ref(),
            );
            Ok(Resolution::Claim(
                Box::new(ResolvedClaim {
                    item: ClaimItem {
                        request_index: index,
                        claim,
                        kind,
                        extent: ExtentClassification::Bounded,
                        backend: named,
                        candidates: candidate_list(&candidates),
                        options: options(selection, selection.options.clone().unwrap_or_default()),
                        form: None,
                    },
                    claim_name: name.to_owned(),
                    candidates,
                    subject: self.own_subject(index, selection)?,
                    unrouted: selection.unrouted(),
                }),
                None,
            ))
        };
        if selected_claim.is_none()
            && self.package.package().node_kind(&function.semantic_type)
                != Some(filament_contract_model::CheckedNodeKind::ScalarType(
                    filament_contract_model::ScalarTypeForm::Boolean,
                ))
        {
            return inapplicable();
        }
        let body = self
            .package
            .function_body(name)
            .ok()
            .flatten()
            .ok_or_else(pending)?;
        let mut clauses: Vec<&OccurrenceKey> = body
            .occurrences
            .iter()
            .filter(|occurrence| {
                matches!(occurrence.origin().role().as_str(), "claim" | "declaration")
            })
            .collect();
        clauses.sort();
        let claim = selected_claim
            .or_else(|| clauses.first().copied())
            .ok_or_else(pending)?;
        let target = match ValueTarget::select_package(self.package, name, claim) {
            Ok(target) => target,
            Err(fsl_replay::value_check::PackageSelection::Selection(
                fsl_replay::value_check::Selection::NoForm,
            )) if selected_claim.is_none() => {
                return inapplicable();
            }
            Err(refusal) => {
                return Err(PackageRunError::Pending(PendingPackageFamily {
                    request_index: Some(index),
                    note: Some(format!("{refusal:?}")),
                    family: PendingFamily::ValueShape,
                    node: None,
                }));
            }
        };
        // Enum values use the admitted world's complete member identities.
        if !target.parameters().iter().all(|parameter| {
            matches!(
                parameter.domain(),
                fsl_replay::value_check::ParameterDomain::Boolean
                    | fsl_replay::value_check::ParameterDomain::Integers(_)
                    | fsl_replay::value_check::ParameterDomain::Unbounded
                    | fsl_replay::value_check::ParameterDomain::Values(_)
            )
        }) {
            let shape = target
                .parameters()
                .iter()
                .find_map(|parameter| match parameter.domain() {
                    fsl_replay::value_check::ParameterDomain::Union(_) => Some("union".to_owned()),
                    fsl_replay::value_check::ParameterDomain::Unsupported(_) => {
                        let nodes = &self.package.package().resolved_graph().nodes;
                        nodes
                            .iter()
                            .find(|node| {
                                self.package.node_key(&node.node_id).as_ref()
                                    == Some(&parameter.node())
                            })
                            .and_then(|node| {
                                nodes.iter().find(|ty| ty.node_id == node.semantic_type)
                            })
                            .map(|ty| ty.semantic_form.to_string())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| "unavailable-type".to_owned());
            return Err(PackageRunError::Pending(PendingPackageFamily {
                request_index: Some(index),
                note: Some(shape),
                family: PendingFamily::ValueShape,
                node: None,
            }));
        }
        let kind = Capability::ValueValidity;
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if kind.applies_to_declaration(DeclarationFamily::Value) {
            if let Some(refused) = unknown_option(
                index,
                Some(name),
                self.registry,
                named.as_ref(),
                &backend_options,
            ) {
                return Ok(Resolution::Settled(refused));
            }
        }
        let record = if selected_claim.is_some() {
            let rows = self
                .package
                .requirement_items()
                .iter()
                .filter(|row| {
                    super::package_value::occurrence(self.package, row).as_ref() == Some(claim)
                })
                .collect::<Vec<_>>();
            let [row] = rows.as_slice() else {
                return Err(pending());
            };
            Some(
                super::package_value::requirement_item(
                    self.package,
                    row,
                    index,
                    self.registry,
                    named.as_ref(),
                )
                .map_err(|_| pending())?,
            )
        } else {
            None
        };
        let extent = record.as_ref().map_or_else(
            || {
                target
                    .clause_extent()
                    .map_or(ExtentClassification::Bounded, ExtentClassification::of)
            },
            |record| record.extent(),
        );
        let candidates = tie(
            self.registry.candidates(kind, named.as_ref()),
            kind,
            named.as_ref(),
        );
        let own_subject = self.own_subject(index, selection)?;
        let temporal = selection.temporal.clone().unwrap_or_default();
        let item = ClaimItem {
            request_index: index,
            claim: claim.clone(),
            kind,
            extent,
            backend: named,
            candidates: candidate_list(&candidates),
            options: options(selection, backend_options),
            form: None,
        };
        let unrouted = selection.unrouted()
            || selection.reductions.is_some()
            || reads_no_clocks(selection)
            || temporal
                .limits
                .is_some_and(|limits| limits.sets_memory_limits())
            || !own_subject.same_model(self.subject);
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                subject: own_subject,
                unrouted,
            }),
            Some(PackageValue {
                target,
                result_bound: body.semantic_type.clone(),
                record,
            }),
        ))
    }

    /// Supplied finite replay or recording over an admitted protocol declaration.
    fn supplied_protocol(
        &self,
        index: RequestIndex,
        name: &str,
        selection: &ItemSelection,
    ) -> Result<Resolution<()>, PackageRunError> {
        let fault = || PackageRunError::Document(AnalyzeDocumentError::Fault);
        let input = self.input.ok_or_else(fault)?;
        let kind = match selection.kind.as_deref().map(Capability::from_wire) {
            Some(Ok(kind @ (Capability::FiniteReplay | Capability::GlobalConformance))) => kind,
            _ => return Ok(Resolution::Settled(invalid_kind(index, Some(name), "unknown-kind"))),
        };
        let form = input.package().protocol_execution_named(name).map_err(|_| fault())?.ok_or_else(fault)?;
        let declaration = form.protocol.clause().clone();
        let key = input.package().node_key(&declaration).ok_or_else(fault)?;
        let claim = OccurrenceKey::declaration(key);
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if let Some(refused) = unknown_option(index, Some(name), self.registry, named.as_ref(), &backend_options) {
            return Ok(Resolution::Settled(refused));
        }
        if (kind == Capability::GlobalConformance && selection.recording.is_none())
            || !crate::protocol_analysis::recording::package_recording_supported(&input.input(), name)
        {
            return Err(PackageRunError::Pending(PendingPackageFamily {
                request_index: Some(index), family: PendingFamily::Declaration, node: Some(declaration),
                note: Some(format!("`{name}`: supplied protocol slice requires origin activation, static roles, unordered literal-capacity channels and sequence/choice/message/check controls; temporal requirements, captures, negative replay and behavioral conformance remain pending")),
            }));
        }
        let candidates = self.registry.candidates(kind, named.as_ref());
        let temporal = selection.temporal.clone().unwrap_or_default();
        let own_subject = self.own_subject(index, selection)?;
        let item_form = (kind == Capability::GlobalConformance).then(|| ItemForm::Temporal(temporal_settings(&temporal)));
        Ok(Resolution::Claim(Box::new(ResolvedClaim {
            item: ClaimItem {
                request_index: index, claim, kind, extent: ExtentClassification::Bounded,
                backend: named, candidates: candidate_list(&candidates), options: options(selection, backend_options), form: item_form,
            },
            claim_name: name.to_owned(), candidates,
            unrouted: selection.unrouted() || selection.reductions.is_some() || reads_no_clocks(selection) || !own_subject.same_model(self.subject),
            subject: own_subject,
        }), ()))
    }

    /// A native projection or realizability item over an admitted protocol declaration.
    fn protocol_analysis(
        &self,
        index: RequestIndex,
        name: &str,
        selection: &ItemSelection,
    ) -> Result<Resolution<()>, PackageRunError> {
        let fault = || PackageRunError::Document(AnalyzeDocumentError::Fault);
        let input = self.input.ok_or_else(fault)?;
        let kind = match selection.kind.as_deref().map(Capability::from_wire) {
            Some(Ok(Capability::LocalProjection)) => Capability::LocalProjection,
            Some(Ok(Capability::Realizability)) => Capability::Realizability,
            _ => {
                return Ok(Resolution::Settled(invalid_kind(
                    index,
                    Some(name),
                    "unknown-kind",
                )))
            }
        };
        let named = selection.backend.as_deref().map(BackendId::new);
        let mut backend_options = selection.options.clone().unwrap_or_default();
        let role = (kind == Capability::LocalProjection)
            .then(|| backend_options.remove("role"))
            .flatten();
        if let Some(refused) = unknown_option(
            index,
            Some(name),
            self.registry,
            named.as_ref(),
            &backend_options,
        ) {
            return Ok(Resolution::Settled(refused));
        }
        let mut settings =
            match super::protocol_analysis_selection(index, name, selection, kind, role) {
                Ok(settings) => settings,
                Err(refused) => return Ok(Resolution::Settled(refused)),
            };
        let own_subject = self.own_subject(index, selection)?;
        if !own_subject
            .protocol
            .as_ref()
            .is_some_and(|selected| selected.protocol == name)
        {
            return Ok(Resolution::Settled(super::invalid_member(
                index,
                name,
                "temporal.subject.protocol",
            )));
        }
        let protocol = match protocol_binding(&own_subject, input) {
            Ok(Some(protocol)) => protocol,
            Ok(None) => {
                return Ok(Resolution::Settled(super::invalid_member(
                    index,
                    name,
                    "temporal.subject.protocol",
                )))
            }
            Err(member) => {
                return Ok(Resolution::Settled(super::invalid_value(
                    index, name, member,
                )))
            }
        };
        let form = input
            .package()
            .protocol_execution_named(name)
            .map_err(|_| fault())?
            .ok_or_else(fault)?;
        let key = input
            .package()
            .node_key(form.protocol.clause())
            .ok_or_else(fault)?;
        let claim = OccurrenceKey::declaration(key);
        settings.protocol = Some(protocol.clone());
        let candidates = self.registry.candidates(kind, named.as_ref());
        let mut item_options = options(selection, backend_options);
        item_options.protocol_analysis = Some(settings);
        let temporal = selection.temporal.clone().unwrap_or_default();
        let mut form = temporal_settings(&temporal);
        form.protocol = Some(protocol);
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item: ClaimItem {
                    request_index: index,
                    claim,
                    kind,
                    extent: ExtentClassification::Unbounded {
                        finite_bound_available: false,
                    },
                    backend: named,
                    candidates: candidate_list(&candidates),
                    options: item_options,
                    form: Some(ItemForm::Temporal(form)),
                },
                claim_name: name.to_owned(),
                candidates,
                unrouted: selection.unrouted_routing_options(false, false)
                    || selection.reductions.is_some()
                    || reads_no_clocks(selection)
                    || !own_subject.same_model(self.subject),
                subject: own_subject,
            }),
            (),
        ))
    }

    /// A protocol's deadlock-freedom item or a parallel's race-freedom item,
    /// anchored at that protocol's declaration occurrence.
    fn protocol_freedom(
        &self,
        index: RequestIndex,
        name: &str,
        protocol_name: &str,
        selection: &ItemSelection,
    ) -> Result<Resolution<ProtocolBinding>, PackageRunError> {
        let input = self
            .input
            .ok_or(PackageRunError::Document(AnalyzeDocumentError::Fault))?;
        let own_subject = self.own_subject(index, selection)?;
        if selection
            .temporal
            .as_ref()
            .and_then(|temporal| temporal.item_kind)
            == Some(super::ItemKindSelection::DeadlockFreedom)
            && !own_subject
                .protocol
                .as_ref()
                .is_some_and(|selected| selected.protocol == protocol_name)
        {
            return Ok(Resolution::Settled(super::invalid_value(
                index,
                name,
                "temporal.subject.protocol",
            )));
        }
        let Some(selected) = own_subject.protocol.as_ref() else {
            let mut detail = Map::new();
            detail.insert("member".to_owned(), json!("temporal.subject.protocol"));
            return Ok(Resolution::Settled(ResolvedItem::Settled {
                claim_name: Some(name.to_owned()),
                record: Box::new(
                    TerminalRecord::disposition_declined(
                        index,
                        fsl_replay::ProofRefusalCause::InvalidInput,
                        fsl_replay::DeclineCode::Fsl(fsl_foundation::Code::InvalidRuntimeInput),
                        CatalogCode::new("invalid_runtime_input", "missing-member"),
                    )
                    .with_diagnostic_detail(detail),
                ),
            }));
        };
        if selected.protocol != protocol_name {
            return Ok(Resolution::Settled(unknown_claim(index, name)));
        }
        let protocol = match protocol_binding(&own_subject, input) {
            Ok(Some(protocol)) => protocol,
            Ok(None) => return Ok(Resolution::Settled(unknown_claim(index, name))),
            Err(member) => {
                return Ok(Resolution::Settled(super::invalid_value(
                    index, name, member,
                )));
            }
        };
        let Some(form) = input
            .package()
            .protocol_execution_named(protocol_name)
            .ok()
            .flatten()
        else {
            return Ok(Resolution::Settled(unknown_claim(index, name)));
        };
        let key = input
            .package()
            .node_key(form.protocol.clause())
            .ok_or(PackageRunError::Document(AnalyzeDocumentError::Fault))?;
        let claim = if selection
            .temporal
            .as_ref()
            .and_then(|temporal| temporal.item_kind)
            == Some(super::ItemKindSelection::LoopInvariant)
        {
            if selection
                .temporal
                .as_ref()
                .is_some_and(|temporal| temporal.max_depth.is_some())
            {
                return Ok(Resolution::Settled(super::invalid_value(
                    index,
                    name,
                    "temporal.max_depth",
                )));
            }
            let Some((_, path)) = name.split_once("::") else {
                return Ok(Resolution::Settled(unknown_claim(index, name)));
            };
            let Some(repeat) = form.controls.iter().find(|control| {
                control.operation == fsl_package::protocol_view::ControlOperation::Repeat
                    && control.path.join("::") == path
            }) else {
                return Ok(Resolution::Settled(unknown_claim(index, name)));
            };
            let ordinal = u64::try_from(repeat.id.0)
                .map_err(|_| PackageRunError::Document(AnalyzeDocumentError::Fault))?;
            OccurrenceKey::new(
                key,
                filament_exact::Origin::new(filament_exact::Role::new("repeat"), ordinal),
            )
        } else {
            OccurrenceKey::declaration(key)
        };
        let temporal = selection.temporal.clone().unwrap_or_default();
        let form = temporal_settings(&temporal);
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if let Some(refused) = unknown_option(
            index,
            Some(name),
            self.registry,
            named.as_ref(),
            &backend_options,
        ) {
            return Ok(Resolution::Settled(refused));
        }
        let candidates = self.registry.restrict_to_timing(
            self.registry
                .candidates(Capability::TemporalSatisfaction, named.as_ref()),
            false,
        );
        let mut temporal_form = form;
        temporal_form.protocol = Some(protocol.clone());
        let extent = if temporal_form.item_kind == ItemKind::DeadlockFreedom {
            ExtentClassification::Bounded
        } else {
            ExtentClassification::Unbounded {
                finite_bound_available: false,
            }
        };
        let item = ClaimItem {
            request_index: index,
            claim: claim.clone(),
            kind: Capability::TemporalSatisfaction,
            extent,
            backend: named,
            candidates: candidate_list(&candidates),
            options: options(selection, backend_options),
            form: Some(ItemForm::Temporal(temporal_form)),
        };
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                subject: own_subject.clone(),
                unrouted: selection.unrouted() || !own_subject.same_model(self.subject),
            }),
            protocol,
        ))
    }

    /// Resolve universal hyper analysis from its authenticated package form.
    fn hyper(
        &self,
        index: RequestIndex,
        name: &str,
        declaration: &CheckedNodeId,
        ordinal: u64,
        selection: &ItemSelection,
    ) -> Result<Resolution<Box<PackageHyperSubject<'r>>>, PackageRunError> {
        let fault = || PackageRunError::Document(AnalyzeDocumentError::Fault);
        let input = self.input.ok_or_else(fault)?;
        let package = input.package();
        let key = package.node_key(declaration).ok_or_else(fault)?;
        let claim = OccurrenceKey::new(
            key.clone(),
            filament_exact::Origin::new(filament_exact::Role::new("claim"), ordinal),
        );
        let pending = |fact| {
            PackageRunError::Pending(PendingPackageFamily {
                request_index: Some(index),
                family: PendingFamily::ClauseFact(fact),
                node: Some(declaration.clone()),
                note: None,
            })
        };
        let form = package
            .hyper_clause_form(&claim)
            .map_err(|refusal| match refusal {
                fsl_package::BareClauseRefusal::Pending { fact, .. } => pending(fact),
                _ => fault(),
            })?;
        let world = package.world().map_err(|_| fault())?;
        let clause = fsl_semantics::check::CheckedHyperClause::of_package_form(
            form,
            world.types(),
            crate::plan::MODEL_BASED_POSITIONS,
        )
        .map_err(|_| fault())?;
        use fsl_eval::simulation::model::ModelEvaluator;
        package
            .hyper_reference_literals(&clause)
            .map_err(|_| pending("hyper-reference-bindings"))?;
        // Hyper requirements are reconstructed from the authenticated form.
        // requirement_items currently carries HP-1 rows only. Every admitted
        // object parameter is a Reference (a bounded type position); the
        // universal infinite-trace clause itself is the unbounded domain.
        let requirements = clause
            .claim()
            .requirements(key, &[], &[], &[])
            .map_err(|_| fault())?;
        let own_subject = self.own_subject(index, selection)?;
        let temporal = selection.temporal.clone().unwrap_or_default();
        let mut settings = temporal_settings(&temporal);
        settings
            .pending_reductions
            .clone_from(&selection.reductions);
        let kind = match selection.kind.as_deref() {
            Some(label) => match Capability::from_wire(label) {
                Ok(kind) => kind,
                Err(_) => {
                    return Ok(Resolution::Settled(invalid_kind(
                        index,
                        Some(name),
                        "unknown-kind",
                    )))
                }
            },
            None => Capability::TemporalSatisfaction,
        };
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if let Some(refused) = unknown_option(
            index,
            Some(name),
            self.registry,
            named.as_ref(),
            &backend_options,
        ) {
            return Ok(Resolution::Settled(refused));
        }
        let candidates = self.registry.restrict_to_timing(
            if kind == Capability::TemporalSatisfaction {
                fsl_route::request::requirement_candidates(
                    &requirements,
                    self.registry,
                    named.as_ref(),
                )
            } else {
                self.registry.candidates(kind, named.as_ref())
            },
            false,
        );
        let item = ClaimItem {
            request_index: index,
            claim,
            kind,
            extent: ExtentClassification::Unbounded {
                finite_bound_available: false,
            },
            backend: named,
            candidates: candidate_list(&candidates),
            options: options(selection, backend_options),
            form: (kind == Capability::TemporalSatisfaction)
                .then_some(ItemForm::Temporal(settings)),
        };
        let unrouted = selection.unrouted_beyond_sampling()
            || reads_no_clocks(selection)
            || temporal
                .limits
                .is_some_and(|limits| limits.sets_memory_limits())
            || !own_subject.same_model(self.subject);
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                subject: own_subject,
                unrouted,
            }),
            Box::new(PackageHyperSubject::new(
                input,
                clause,
                self.binding.clone(),
                self.limits,
            )),
        ))
    }

    fn model_relation(
        &self,
        index: RequestIndex,
        name: &str,
        declaration: &CheckedNodeId,
        ordinal: u64,
        selection: &ItemSelection,
    ) -> Result<Resolution<Box<PackageRelationSubject<'r>>>, PackageRunError> {
        use filament_contract_model::{
            CapabilityKind, DomainKey as WireDomain, DomainKind as WireKind, RequirementExtent,
        };
        use fsl_foundation::bound::{DomainKey, DomainKind};
        use fsl_semantics::family::{ClaimExtent, Requirements};
        let fault = || PackageRunError::Document(AnalyzeDocumentError::Fault);
        let input = self.input.ok_or_else(fault)?;
        let package = input.package();
        let key = package.node_key(declaration).ok_or_else(fault)?;
        let claim = OccurrenceKey::new(
            key.clone(),
            filament_exact::Origin::new(filament_exact::Role::new("claim"), ordinal),
        );
        let selected = fsl_package::ClaimOccurrence {
            declaration: declaration.clone(),
            occurrence: filament_contract_model::CheckedOccurrence {
                role: filament_contract_model::CheckedOccurrenceRole::Claim,
                ordinal,
            },
        };
        let settled = |record| {
            Resolution::Settled(ResolvedItem::Settled {
                claim_name: Some(name.to_owned()),
                record: Box::new(record),
            })
        };
        let malformed = || {
            settled(
                TerminalRecord::failed_with(
                    index,
                    BackendId::new(crate::model_check::manifest::BACKEND_ID),
                    super::super::items::BROKEN,
                )
                .with_diagnostic_detail(
                    serde_json::json!({"node": declaration, "selection": "malformed-node"})
                        .as_object()
                        .cloned()
                        .unwrap_or_default(),
                ),
            )
        };
        let form = match package.model_relation_form(&selected) {
            Ok(form) => form,
            Err(fsl_package::ModelRelationFormRefusal::MissingSubjectDeclarationOrder {
                package,
                node,
            }) => {
                let locus =
                    fsl_foundation::node_wire::node_key_to_json(&key).map_err(|_| fault())?;
                let detail = serde_json::json!({"package": package, "node": node, "locus": locus, "reason": "missing-subject-declaration-order"});
                return Ok(settled(TerminalRecord::disposition_unsupported(
                    index,
                    CatalogCode::new("unknown_required_feature", "unsupported-feature"),
                    detail.as_object().cloned().unwrap_or_default(),
                )));
            }
            Err(_) => return Ok(malformed()),
        };
        let relation = match fsl_eval::simulation::model::ModelRelationDescriptor::of_package(
            package, &form,
        ) {
            Ok(relation) => relation,
            Err(failure) => {
                let refused = super::super::items::package_projection_refusal(
                    package.package().name(),
                    "model-relation-body",
                    serde_json::json!(declaration),
                    &failure,
                );
                let record = match refused {
                    super::super::Refusal::UnsupportedWithDetail(code, details) => {
                        TerminalRecord::disposition_unsupported(index, code, details)
                    }
                    super::super::Refusal::FailedWithDetail(code, details) => {
                        TerminalRecord::failed_with(
                            index,
                            BackendId::new(crate::model_check::manifest::BACKEND_ID),
                            code,
                        )
                        .with_diagnostic_detail(details)
                    }
                    super::super::Refusal::Failed(code) => TerminalRecord::failed_with(
                        index,
                        BackendId::new(crate::model_check::manifest::BACKEND_ID),
                        code,
                    ),
                    _ => return Ok(malformed()),
                };
                return Ok(settled(record));
            }
        };
        let model_record = form.requirement();
        let actual_extent = match &model_record.extent {
            RequirementExtent::Unbounded(domains)
                if domains.len() == 1
                    && matches!(&domains[0],
                (WireDomain::Node { node, path }, WireKind::InfiniteTrace) if node == declaration && path.is_empty()) =>
            {
                ClaimExtent::from_domains(
                    [(
                        DomainKey::Node {
                            node: key.clone(),
                            path: Vec::new(),
                        },
                        DomainKind::InfiniteTrace,
                    )]
                    .into_iter()
                    .collect(),
                )
            }
            _ => return Ok(malformed()),
        };
        if model_record.kind != CapabilityKind::TemporalSatisfaction {
            return Ok(malformed());
        }
        let requirements =
            Requirements::new(Capability::TemporalSatisfaction, actual_extent).over_model_subject();
        let own_subject = self.own_subject(index, selection)?;
        let temporal = selection.temporal.clone().unwrap_or_default();
        let mut settings = temporal_settings(&temporal);
        settings
            .pending_reductions
            .clone_from(&selection.reductions);
        let kind = match selection.kind.as_deref() {
            Some(label) => match Capability::from_wire(label) {
                Ok(kind) => kind,
                Err(_) => {
                    return Ok(Resolution::Settled(invalid_kind(
                        index,
                        Some(name),
                        "unknown-kind",
                    )))
                }
            },
            None => Capability::TemporalSatisfaction,
        };
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if let Some(refused) = unknown_option(
            index,
            Some(name),
            self.registry,
            named.as_ref(),
            &backend_options,
        ) {
            return Ok(Resolution::Settled(refused));
        }
        let candidates = self.registry.restrict_to_timing(
            if kind == Capability::TemporalSatisfaction {
                fsl_route::request::requirement_candidates(
                    &requirements,
                    self.registry,
                    named.as_ref(),
                )
            } else {
                self.registry.candidates(kind, named.as_ref())
            },
            false,
        );
        let item = ClaimItem {
            request_index: index,
            claim,
            kind,
            extent: ExtentClassification::Unbounded {
                finite_bound_available: false,
            },
            backend: named,
            candidates: candidate_list(&candidates),
            options: options(selection, backend_options),
            form: (kind == Capability::TemporalSatisfaction)
                .then_some(ItemForm::Temporal(settings)),
        };
        let unrouted = selection.unrouted_beyond_sampling()
            || reads_no_clocks(selection)
            || temporal
                .limits
                .is_some_and(|limits| limits.sets_memory_limits())
            || !own_subject.same_model(self.subject);
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                subject: own_subject,
                unrouted,
            }),
            Box::new(PackageRelationSubject::new(
                input,
                relation,
                self.binding.clone(),
                self.limits,
            )),
        ))
    }

    /// FR-620's item over the state-graph claim `name`, whose `claim_names`
    /// row is on `declaration` at `claim` occurrence `ordinal`, as the source
    /// route's `resolve_one` resolves it: the record's extent and candidates
    /// from the package form's requirement record, narrowed to the claim's
    /// form (FR-628).
    fn state_graph(
        &self,
        index: RequestIndex,
        name: &str,
        declaration: &CheckedNodeId,
        ordinal: u64,
        selection: &ItemSelection,
    ) -> Result<Resolution<Box<PackageStateGraphSubject<'r>>>, PackageRunError> {
        let fault = || PackageRunError::Document(AnalyzeDocumentError::Fault);
        let input = self.input.ok_or_else(fault)?;
        let key = input.package().node_key(declaration).ok_or_else(fault)?;
        let claim = OccurrenceKey::new(
            key.clone(),
            filament_exact::Origin::new(filament_exact::Role::new("claim"), ordinal),
        );
        let own_subject = self.own_subject(index, selection)?;
        let temporal = selection.temporal.clone().unwrap_or_default();
        let mut form = temporal_settings(&temporal);
        let subject = PackageStateGraphSubject::new(
            input,
            self.package_bytes,
            self.supply,
            self.binding.clone(),
            self.limits,
            &claim,
        )
        .map_err(|refusal| {
            PackageRunError::Pending(PendingPackageFamily {
                request_index: Some(index),
                note: None,
                family: refusal
                    .pending_fact()
                    .map_or(PendingFamily::Declaration, |(fact, _)| {
                        PendingFamily::ClauseFact(fact)
                    }),
                node: Some(declaration.clone()),
            })
        })?;
        let kind = match selection.kind.as_deref() {
            Some(label) => match Capability::from_wire(label) {
                Ok(kind) => kind,
                Err(_) => {
                    return Ok(Resolution::Settled(invalid_kind(
                        index,
                        Some(name),
                        "unknown-kind",
                    )));
                }
            },
            None => Capability::TemporalSatisfaction,
        };
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if kind.applies_to_declaration(DeclarationFamily::TemporalTrace) {
            if let Some(refused) = unknown_option(
                index,
                Some(name),
                self.registry,
                named.as_ref(),
                &backend_options,
            ) {
                return Ok(Resolution::Settled(refused));
            }
        }
        let record = kind == Capability::TemporalSatisfaction;
        let (extent, candidates) = if record {
            (
                ExtentClassification::Unbounded {
                    finite_bound_available: false,
                },
                self.registry.restrict_to_state_graph_form(
                    self.registry.restrict_to_timing(
                        fsl_route::request::requirement_candidates(
                            &subject.claim().claim().requirements(key),
                            self.registry,
                            named.as_ref(),
                        ),
                        !input.input().time_sources.is_empty(),
                    ),
                    fsl_route::StateGraphFormKind::of(subject.claim().form()),
                ),
            )
        } else {
            (
                ExtentClassification::Bounded,
                // FR-962: `resolve_tie` is the identity on every kind but
                // `operation-contract`, which excludes EN-1 for a claim that
                // is no `invariant` state clause.
                tie(
                    self.registry.candidates(kind, named.as_ref()),
                    kind,
                    named.as_ref(),
                ),
            )
        };
        if let Some(refused) = super::sampling_refusal(index, name, subject.claim(), &temporal) {
            return Ok(Resolution::Settled(refused));
        }
        form.pending_reductions.clone_from(&selection.reductions);
        let item_form = (kind == Capability::TemporalSatisfaction
            || kind == Capability::GlobalConformance)
            .then(|| ItemForm::Temporal(form));
        let item = ClaimItem {
            request_index: index,
            claim,
            kind,
            extent,
            backend: named,
            candidates: candidate_list(&candidates),
            options: options(selection, backend_options),
            form: item_form,
        };
        let unrouted = selection.unrouted_beyond_sampling()
            || (selection.reductions.is_some() && kind != Capability::TemporalSatisfaction)
            || reads_no_clocks(selection)
            || temporal
                .limits
                .is_some_and(|limits| limits.sets_memory_limits())
            || !own_subject.same_model(self.subject);
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                subject: own_subject,
                unrouted,
            }),
            Box::new(subject),
        ))
    }

    /// FR-626's derived item (`deadlock-freedom`, or a kind the untimed subject does
    /// not derive) over the one model selection the subject reads,
    /// whose identity `name` is, as the source route's `resolve_derived`
    /// resolves it: at the selection's `model_root` anchor with the
    /// `generated` occurrence 0.
    fn deadlock(
        &self,
        index: RequestIndex,
        name: &str,
        kind: ItemKind,
        selection: &ItemSelection,
    ) -> Result<Resolution<()>, PackageRunError> {
        let temporal = selection.temporal.clone().unwrap_or_default();
        if temporal.max_depth.is_some() {
            // FSpec FR-368-AC-20: a derived item carries no `max_depth`.
            let mut detail = Map::new();
            detail.insert("member".to_owned(), json!("temporal.max_depth"));
            return Ok(Resolution::Settled(ResolvedItem::Settled {
                claim_name: Some(name.to_owned()),
                record: Box::new(TerminalRecord::disposition_invalid_request(
                    index,
                    CatalogCode::new("invalid_runtime_input", "invalid-value"),
                    detail,
                )),
            }));
        }
        let root = fsl_semantics::check::model_root_key(
            &fsl_semantics::model::domain_package::DomainPackageRef {
                identity: name.to_owned(),
            },
        )
        .map_err(|_| AnalyzeDocumentError::Fault)?;
        let claim = OccurrenceKey::new(
            root,
            filament_exact::Origin::new(filament_exact::Role::new("generated"), 0),
        );
        // FSpec FR-368-AC-22, FR-124-AC-12: as on the source route's
        // `resolve_derived`, a subject that reads several model selections has
        // no anchor, so its item settles `unsupported` with no request item,
        // and a subject that reads one selection has the item of that
        // selection only.
        let all: Vec<String> = AnalysisView::model_selections(self.package)
            .into_iter()
            .map(|selection| selection.identity)
            .collect();
        let read: Vec<&String> = all
            .iter()
            .filter(|identity| {
                self.binding
                    .universes
                    .iter()
                    .any(|universe| universe.population.package == **identity)
            })
            .collect();
        let read = if read.is_empty() {
            all.iter().collect()
        } else {
            read
        };
        match read.as_slice() {
            // The package route's subject is untimed (a timed model is refused
            // with the model input), so it derives no time-lock or race item.
            [_] if kind != ItemKind::DeadlockFreedom => {
                return Ok(Resolution::Settled(invalid_kind(
                    index,
                    Some(name),
                    "inapplicable-kind",
                )));
            }
            [only] if *only == name => {
                // FR-124: a model the subject reads that declares `terminal any`
                // opts out of the item, so the source route writes none and
                // settles `inapplicable-kind` (`SubjectReading::opted_out`).
                let opted_out = self
                    .package
                    .terminal_declarations()
                    .map_err(|_| AnalyzeDocumentError::Fault)?
                    .iter()
                    .any(|terminal| {
                        terminal.model == name
                            && matches!(
                                terminal.declaration,
                                fsl_package::BareTerminalDeclaration::Any
                            )
                    });
                if opted_out {
                    return Ok(Resolution::Settled(invalid_kind(
                        index,
                        Some(name),
                        "inapplicable-kind",
                    )));
                }
            }
            [_] => {
                return Ok(Resolution::Settled(invalid_kind(
                    index,
                    Some(name),
                    "inapplicable-kind",
                )));
            }
            _ => {
                return Ok(Resolution::Settled(ResolvedItem::Settled {
                    claim_name: Some(name.to_owned()),
                    record: Box::new(TerminalRecord::disposition_unsupported(
                        index,
                        fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code(),
                        Map::new(),
                    )),
                }));
            }
        }
        let own_subject = self.own_subject(index, selection)?;
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if let Some(refused) = unknown_option(
            index,
            Some(name),
            self.registry,
            named.as_ref(),
            &backend_options,
        ) {
            return Ok(Resolution::Settled(refused));
        }
        let candidates = self.registry.restrict_to_timing(
            self.registry
                .candidates(Capability::TemporalSatisfaction, named.as_ref()),
            false,
        );
        let mut form = temporal_settings(&temporal);
        form.pending_reductions.clone_from(&selection.reductions);
        let item = ClaimItem {
            request_index: index,
            claim,
            kind: Capability::TemporalSatisfaction,
            extent: ExtentClassification::Unbounded {
                finite_bound_available: false,
            },
            backend: named,
            candidates: candidate_list(&candidates),
            options: ItemOptions {
                required: selection.required.unwrap_or(true),
                max_cases: None,
                max_task_product_work: None,
                max_check_work: None,
                trace: None,
                traces: selection
                    .traces
                    .as_ref()
                    .map(super::DocumentSelection::reference),
                recording: None,
                population: None,
                backend_options,
                protocol_analysis: None,
            },
            form: Some(ItemForm::Temporal(form)),
        };
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                unrouted: selection.unrouted()
                    || reads_no_clocks(selection)
                    || !own_subject.same_model(self.subject),
                subject: own_subject,
            }),
            (),
        ))
    }

    /// FR-625's item over the `invariant` state clause `name`, whose
    /// `claim_names` row is on `declaration`, as the source route's
    /// `resolve_one` resolves it: at the clause's node and `claim`
    /// occurrence, kind `operation-contract` unless the item names another,
    /// and, over a model subject, FR-489's tie narrowed to EN-1 (FR-962). An
    /// item with no model subject is decided by EN-2's contract check, which
    /// has no package form yet.
    fn state_clause(
        &self,
        index: RequestIndex,
        name: &str,
        declaration: &CheckedNodeId,
        ordinal: u64,
        selection: &ItemSelection,
    ) -> Result<Resolution<()>, PackageRunError> {
        let pending = |family| {
            PackageRunError::Pending(PendingPackageFamily {
                request_index: Some(index),
                note: None,
                family,
                node: Some(declaration.clone()),
            })
        };
        let input = self
            .input
            .ok_or(PackageRunError::Document(AnalyzeDocumentError::Fault))?;
        let key = input
            .package()
            .node_key(declaration)
            .ok_or(PackageRunError::Document(AnalyzeDocumentError::Fault))?;
        let origin = filament_exact::Origin::new(filament_exact::Role::new("claim"), ordinal);
        let clause = input
            .state_clause(&key, &origin)
            .ok_or(PackageRunError::Document(AnalyzeDocumentError::Fault))?;
        let claim = OccurrenceKey::new(clause.identity.clone(), clause.claim.clone());
        // FR-290: the operation-contract record's extent from its domain
        // kinds, as S3 classifies the selected state clause.
        let kinds = input
            .invariant_domain_kinds(
                clause,
                filament_semantic_value::checking::CheckingLimits::default().nodes(),
            )
            .ok_or_else(|| pending(PendingFamily::Declaration))?;
        let extent = if kinds.is_empty() {
            ExtentClassification::Bounded
        } else {
            ExtentClassification::Unbounded {
                finite_bound_available: kinds.iter().all(|kind| kind.finite_kind().is_some()),
            }
        };
        let own_subject = self.own_subject(index, selection)?;
        let kind = match selection.kind.as_deref() {
            Some(label) => match Capability::from_wire(label) {
                Ok(kind) => kind,
                Err(_) => {
                    return Ok(Resolution::Settled(invalid_kind(
                        index,
                        Some(name),
                        "unknown-kind",
                    )));
                }
            },
            None => Capability::OperationContract,
        };
        let named = selection.backend.as_deref().map(BackendId::new);
        let backend_options = selection.options.clone().unwrap_or_default();
        if kind.applies_to_declaration(DeclarationFamily::StateModel) {
            if let Some(refused) = unknown_option(
                index,
                Some(name),
                self.registry,
                named.as_ref(),
                &backend_options,
            ) {
                return Ok(Resolution::Settled(refused));
            }
        }
        let model_subject = !own_subject.initial_states.is_empty();
        if !self.explicit_items && kind == Capability::OperationContract && !model_subject {
            return Err(pending(PendingFamily::Declaration));
        }
        let candidates = state_clause_tie(
            self.registry.candidates(kind, named.as_ref()),
            kind,
            named.as_ref(),
            model_subject,
            clause.invariant,
        );
        if self.explicit_items && kind == Capability::OperationContract {
            if let CandidateOutcome::Candidates(set) = &candidates {
                let native_invariant = clause.invariant
                    && model_subject
                    && matches!(set.candidates(), [candidate] if candidate.id().as_str() == crate::model_check::manifest::BACKEND_ID);
                if matches!(set.candidates(), [_]) && !native_invariant {
                    return Err(PackageRunError::Pending(PendingPackageFamily {
                        request_index: Some(index),
                        family: PendingFamily::OperationContractRequirementRecord,
                        node: Some(declaration.clone()),
                        note: Some(format!("`{name}` (claim occurrence {ordinal})")),
                    }));
                }
            }
        }
        let temporal = selection.temporal.clone().unwrap_or_default();
        let item = ClaimItem {
            request_index: index,
            claim,
            kind,
            extent,
            backend: named,
            candidates: candidate_list(&candidates),
            options: options(selection, backend_options),
            form: None,
        };
        let unrouted = selection.unrouted()
            || selection.reductions.is_some()
            || reads_no_clocks(selection)
            || temporal
                .limits
                .is_some_and(|limits| limits.sets_memory_limits())
            || !own_subject.same_model(self.subject);
        Ok(Resolution::Claim(
            Box::new(ResolvedClaim {
                item,
                claim_name: name.to_owned(),
                candidates,
                subject: own_subject,
                unrouted,
            }),
            (),
        ))
    }
}

/// FR-962's operation-contract tie rule for a package state clause. EN-1
/// decides an invariant over a model subject; pre/post clauses go to
/// `contract-checker` regardless of whether the subject has a model.
fn state_clause_tie(
    candidates: CandidateOutcome,
    kind: Capability,
    named: Option<&BackendId>,
    model_subject: bool,
    invariant: bool,
) -> CandidateOutcome {
    if named.is_some() || kind != Capability::OperationContract {
        return candidates;
    }
    if !model_subject || !invariant {
        return fsl_route::tie::exclude_en1(candidates, kind, named, false);
    }
    let CandidateOutcome::Candidates(set) = &candidates else {
        return candidates;
    };
    let mut members: Vec<&str> = set
        .candidates()
        .iter()
        .map(|candidate| candidate.id().as_str())
        .collect();
    members.sort_unstable();
    let mut tied = [
        crate::contract_check::manifest::BACKEND_ID,
        crate::model_check::manifest::BACKEND_ID,
    ];
    tied.sort_unstable();
    if members != tied {
        return candidates;
    }
    match set.narrowed_to(&BackendId::new(crate::model_check::manifest::BACKEND_ID)) {
        Some(narrowed) => CandidateOutcome::Candidates(narrowed),
        None => candidates,
    }
}

/// FR-620's item options of `selection`.
fn options(selection: &ItemSelection, backend_options: BTreeMap<String, String>) -> ItemOptions {
    ItemOptions {
        required: selection.required.unwrap_or(true),
        max_cases: selection.check.map(|check| check.limits.max_cases),
        max_task_product_work: selection
            .temporal
            .as_ref()
            .and_then(|temporal| temporal.limits)
            .and_then(|limits| limits.max_task_product_work),
        max_check_work: selection
            .temporal
            .as_ref()
            .and_then(|temporal| temporal.limits)
            .and_then(|limits| limits.max_check_work),
        trace: selection
            .trace
            .as_ref()
            .map(super::DocumentSelection::reference),
        traces: selection
            .traces
            .as_ref()
            .map(super::DocumentSelection::reference),
        recording: selection
            .recording
            .as_ref()
            .map(super::DocumentSelection::reference),
        population: selection.population.clone(),
        backend_options,
        protocol_analysis: None,
    }
}

/// An untimed subject reads no clock limit; the package route's
/// model-check input reads no timed model, so its subject is untimed.
fn reads_no_clocks(selection: &ItemSelection) -> bool {
    selection
        .temporal
        .as_ref()
        .and_then(|temporal| temporal.limits)
        .is_some_and(|limits| limits.max_clocks.is_some())
}

/// FR-290: the extent classification of a package-form clause's record,
/// from the kinds of its domains. The populations its `holds` atoms name
/// are population domains, which have a finite bound kind, and a package
/// form is under the infinite-trace profile, whose formula domain makes the
/// extent unbounded already, so they do not change it.
fn clause_extent(clause: &fsl_semantics::check::CheckedTemporalClause) -> ExtentClassification {
    let kinds = clause.claim().domain_kinds();
    if kinds.is_empty() {
        return ExtentClassification::Bounded;
    }
    ExtentClassification::Unbounded {
        finite_bound_available: kinds.iter().all(|kind| kind.finite_kind().is_some()),
    }
}

/// FR-962's tie rule for an item over a temporal clause: EN-1 is excluded
/// from an unnamed `operation-contract` item, since a temporal clause is no
/// `invariant` state clause, and the rule selects no provider for it.
fn tie(
    candidates: CandidateOutcome,
    kind: Capability,
    named: Option<&BackendId>,
) -> CandidateOutcome {
    if named.is_some() || kind != Capability::OperationContract {
        return candidates;
    }
    fsl_route::tie::exclude_en1(candidates, kind, named, false)
}

/// What every package-route analysis item reads.
struct WorkInputs<'w, 'a> {
    engines: &'w Engines,
    /// The admitted bytes a provider's request carries, held when a provider
    /// decides an item.
    admitted: Option<std::sync::Arc<crate::provider::AdmittedPackage>>,
    input: &'a PackageModelInput<'a>,
    package_bytes: &'a [u8],
    supply: fsl_replay::HandoffSupply<'a>,
    binding: &'w SubjectBinding<'a>,
    limits: AnalyzeLimits,
    family: &'w dyn Fn(ItemKind, &OccurrenceKey) -> Option<DeclarationFamily>,
}

impl<'a> WorkInputs<'_, 'a> {
    /// The analysis item of the negotiated `negotiated` on the package
    /// route, as the source route's `work` builds it: a temporal clause
    /// over its package form, a derived deadlock-freedom item over the
    /// package route's input, and any other kind its unadapted item.
    fn item(&self, route: Option<Route<'a>>, negotiated: &NegotiatedItem) -> AnalyzeItem<'a> {
        let item = &negotiated.item;
        let temporal_kind = match &item.form {
            Some(ItemForm::Temporal(form)) => form.item_kind,
            Some(
                ItemForm::FiniteHyper(_)
                | ItemForm::Probabilistic(_)
                | ItemForm::Hybrid(_)
                | ItemForm::Composition(_),
            )
            | None => ItemKind::Formula,
        };
        let family = (self.family)(temporal_kind, &item.claim);
        let mut built = match (&item.form, route) {
            (Some(ItemForm::Probabilistic(form)), Some(Route::Probability))
                if item.kind == Capability::ProbabilisticSatisfaction =>
            {
                probability::item(self.input, self.binding, &self.limits, item, form)
            }
            (Some(ItemForm::Temporal(form)), Some(Route::Refinement(subject)))
                if item.kind == Capability::TemporalSatisfaction
                    && matches!(form.item_kind, ItemKind::Formula | ItemKind::Refinement) =>
            {
                subject.configured(form.clone()).item(item.request_index)
            }
            (_, Some(Route::ProtocolAnalysis)) => super::super::request::protocol_analysis_item(
                Box::new(self.input.package().package().clone()),
                &self.input.input(),
                self.input.package().package().name(),
                self.binding,
                item,
                &self.limits,
            ),
            (_, Some(Route::SuppliedProtocol(protocol))) if item.kind == Capability::FiniteReplay => {
                let trace = item.options.trace.as_ref().and_then(|reference| self.binding.snapshots.get(reference)).map(Vec::as_slice).map_or(
                    Err(crate::protocol_analysis::TraceDefect::unreadable()), crate::protocol_analysis::SuppliedTrace::read,
                );
                crate::protocol_analysis::finite_replay::package_finite_replay_item(item.request_index, self.input.input(), self.binding.clone(), self.limits, protocol, trace)
            }
            (_, Some(Route::SuppliedProtocol(protocol)))
                if item.kind == Capability::GlobalConformance =>
            {
                let recording = item
                    .options
                    .recording
                    .as_ref()
                    .and_then(|reference| self.binding.snapshots.get(reference))
                    .map(Vec::as_slice);
                crate::protocol_analysis::conformance::package_global_recording_item(
                    item.request_index,
                    self.input.input(),
                    protocol,
                    item.options.recording.clone(),
                    recording,
                    item.options.population.clone(),
                    self.limits.replay,
                )
            }
            (
                Some(ItemForm::Temporal(form)),
                Some(Route::Clause {
                    subject,
                    protocol: Some(protocol),
                }),
            ) if item.kind == Capability::TemporalSatisfaction => subject
                .configured(form.clone())
                .protocol_item(item.request_index, protocol),
            (
                Some(ItemForm::Temporal(form)),
                Some(Route::Clause {
                    subject,
                    protocol: None,
                }),
            ) if item.kind == Capability::TemporalSatisfaction => {
                subject.configured(form.clone()).item(item.request_index)
            }
            (Some(ItemForm::Temporal(form)), Some(Route::Hyper(subject)))
                if item.kind == Capability::TemporalSatisfaction =>
            {
                subject.configured(form.clone()).item(item.request_index)
            }
            (Some(ItemForm::Temporal(form)), Some(Route::Relation(subject)))
                if item.kind == Capability::TemporalSatisfaction =>
            {
                subject.configured(form.clone()).item(item.request_index)
            }
            (Some(ItemForm::Temporal(form)), Some(Route::ProtocolFreedom(protocol)))
                if item.kind == Capability::TemporalSatisfaction =>
            {
                super::super::clause_item::package_protocol_freedom_item(
                    self.input,
                    self.package_bytes,
                    self.supply,
                    self.binding.clone(),
                    self.limits,
                    super::super::clause_item::PackageProtocolSelection {
                        claim: &item.claim,
                        form,
                        protocol,
                        index: item.request_index,
                    },
                )
            }
            (Some(ItemForm::Temporal(form)), Some(Route::Deadlock))
                if item.kind == Capability::TemporalSatisfaction
                    && form.item_kind == ItemKind::DeadlockFreedom =>
            {
                PackageDeadlockSubject::new(
                    self.input,
                    self.package_bytes,
                    self.supply,
                    self.binding.clone(),
                    self.limits,
                    &item.claim,
                    form,
                )
                .item(item.request_index)
            }
            (Some(ItemForm::Temporal(form)), Some(Route::StateGraph(subject)))
                if item.kind == Capability::TemporalSatisfaction =>
            {
                subject.configured(form.clone()).item(item.request_index)
            }
            // FR-489's tie routed the invariant over the model subject to
            // EN-1 (FR-625, FR-951), as on the source route.
            (_, Some(Route::StateClause))
                if item.kind == Capability::OperationContract
                    && negotiated.disposition.target().is_some_and(|backend| {
                        backend.id().as_str() == crate::model_check::manifest::BACKEND_ID
                    }) =>
            {
                let form = match &item.form {
                    Some(ItemForm::Temporal(form)) => form.clone(),
                    Some(
                        ItemForm::FiniteHyper(_)
                        | ItemForm::Probabilistic(_)
                        | ItemForm::Hybrid(_)
                        | ItemForm::Composition(_),
                    )
                    | None => super::super::TemporalItem::default(),
                };
                PackageInvariantSubject::new(
                    PackageItemInputs {
                        input: self.input,
                        package_bytes: self.package_bytes,
                        supply: self.supply,
                        binding: self.binding.clone(),
                        limits: self.limits,
                    },
                    &item.claim,
                    &form,
                )
                .map_or_else(
                    || AnalyzeItem::unadapted(item.request_index, item.kind, family),
                    |subject| subject.item(item.request_index),
                )
            }
            (_, Some(Route::Schedule(route))) if item.kind == Capability::Schedulability => {
                route.item(item.request_index)
            }
            (_, Some(Route::Value(value))) if item.kind == Capability::ValueValidity => {
                let PackageValue {
                    target,
                    result_bound,
                    record,
                } = *value;
                let target =
                    target.with_limits(fsl_replay::AccountingLimits(self.limits.accounting));
                let backend = negotiated.disposition.target().filter(|backend| {
                    self.engines
                        .get(backend.id())
                        .is_some_and(|engine| engine.provider().is_some())
                });
                if let (Some(backend), Some(admitted)) = (backend, &self.admitted) {
                    // FR-644-AC-8: the provider is sent the request the
                    // source route sends, over the admitted bytes.
                    let routed = record
                        .or_else(|| {
                            target.clause_extent().zip(result_bound).and_then(|bounds| {
                                crate::provider::routed_package_clause_item(
                                    self.engines.registrations(),
                                    item.request_index,
                                    &item.claim,
                                    bounds,
                                    item.backend.as_ref(),
                                )
                            })
                        })
                        .map(|routed| routed.with_options(item.options.backend_options.clone()));
                    crate::provider::package_provider_item(
                        item.request_index,
                        backend.id(),
                        std::sync::Arc::clone(admitted),
                        crate::provider::PackageProviderInput {
                            family,
                            packages: self.binding.packages,
                            routed,
                            emit: self.limits.emit,
                            target,
                        },
                    )
                } else {
                    crate::value_check::value_item_in(
                        item.request_index,
                        family,
                        Ok(crate::value_check::ValueRequest {
                            target,
                            max_cases: item.options.max_cases,
                            reported: None,
                        }),
                    )
                }
            }
            _ => AnalyzeItem::unadapted(item.request_index, item.kind, family),
        };
        let expected = match &item.form {
            _ if matches!(
                item.kind,
                Capability::GlobalConformance
                    | Capability::LocalProjection
                    | Capability::Realizability
            ) =>
            {
                Some(crate::protocol_analysis::manifest::BACKEND_ID)
            }
            // A timed model's graph item is EN-6's, as the source route's
            // `state_graph_item` selects it; an untimed one is EN-1's.
            Some(ItemForm::Temporal(_)) if !self.input.input().time_sources.is_empty() => {
                Some(crate::zone_check::manifest::BACKEND_ID)
            }
            Some(ItemForm::Temporal(_)) => Some(crate::model_check::manifest::BACKEND_ID),
            Some(ItemForm::Probabilistic(form)) => match form.evidence {
                fsl_route::EvidenceKind::Exact => {
                    Some(crate::exact_probabilistic::manifest::BACKEND_ID)
                }
                fsl_route::EvidenceKind::Statistical => {
                    Some(crate::statistical::manifest::BACKEND_ID)
                }
            },
            Some(ItemForm::FiniteHyper(_) | ItemForm::Hybrid(_) | ItemForm::Composition(_))
            | None => None,
        };
        if let (Some(expected), fsl_route::routing::Disposition::Supported(candidate)) =
            (expected, &negotiated.disposition)
        {
            if candidate.id().as_str() != expected {
                built.work = Some(super::super::ItemWork::Engine(Box::new(move |_, _| {
                    super::super::EngineReport::Refused(super::super::Refusal::Unsupported(
                        fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code(),
                    ))
                })));
            }
        }
        built.kind = item.kind;
        built.family = family;
        built.claim = Some(item.claim.clone());
        built.mode = match item.extent {
            ExtentClassification::Bounded => fsl_route::Mode::Bounded,
            ExtentClassification::Unbounded { .. } => fsl_route::Mode::Unbounded,
        };
        built.required(item.options.required).disposed(
            negotiated.disposition.clone(),
            negotiated.diagnostic.clone(),
        )
    }
}

/// The first component-composition declaration in package graph order.
/// The default runtime guard cannot omit it while owning execution is pending.
fn admitted_composition(view: &dyn AnalysisView) -> Option<CheckedNodeId> {
    view.package()
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| crate::claim_family::is_component_composition(&node.semantic_form))
        .map(|node| node.node_id.clone())
}

/// FR-620 "Subject, compilation and default items" over a package, as the
/// source route's `default_items` writes them: one item per kind each claim
/// admits, in source order (a unit's position in the package's source list,
/// then the start byte of the claim's declaration span), then each task-set
/// schedulability claim, then the derived deadlock-freedom item of each model
/// selection. Each item carries the published default of every setting.
fn default_selections(view: &dyn AnalysisView) -> Vec<ItemSelection> {
    use filament_contract_model::CheckedSourceRegion;

    let units = &view.package().lock().sources;
    let span = |region: &CheckedSourceRegion| {
        (
            units
                .iter()
                .position(|unit| {
                    unit.authority == region.source.authority
                        && unit.identity == region.source.identity
                })
                .unwrap_or(usize::MAX),
            region.start,
        )
    };
    let mut claims: Vec<((usize, u64), String, Vec<Capability>)> = view
        .clause_names()
        .iter()
        .map(|row| {
            (
                span(&row.declaration_region),
                row.name.to_string(),
                vec![Capability::TemporalSatisfaction],
            )
        })
        .collect();
    let mut schedulability: Vec<((usize, u64), String)> = Vec::new();
    let mut declarations: Vec<&CheckedNodeId> = Vec::new();
    for row in view.claim_names() {
        if declarations.contains(&&row.declaration) {
            continue;
        }
        declarations.push(&row.declaration);
        let name = row.qualified_name.join("::");
        let at = span(&row.declaration_region);
        if is_schedulability(view, &row.declaration) {
            schedulability.push((at, name));
            continue;
        }
        let kinds = if is_state_graph(view, &row.declaration) {
            vec![Capability::TemporalSatisfaction]
        } else if is_state_clause(view, &row.declaration) {
            vec![Capability::OperationContract]
        } else {
            Vec::new()
        };
        claims.push((at, name, kinds));
    }
    claims.sort_by_key(|(left, ..)| *left);
    schedulability.sort_by_key(|(left, _)| *left);
    let item = |name: String, kind: Option<Capability>| ItemSelection {
        claim: Some(name),
        kind: kind.map(|kind| kind.to_wire().to_owned()),
        ..ItemSelection::default()
    };
    let mut items: Vec<ItemSelection> = Vec::new();
    for (_, name, kinds) in claims {
        if kinds.is_empty() {
            // A claim form with no package record of its kinds yet: the
            // name alone, which the name tables settle.
            items.push(item(name, None));
            continue;
        }
        for kind in Capability::ALL.iter().filter(|kind| kinds.contains(kind)) {
            items.push(item(name.clone(), Some(*kind)));
        }
    }
    items.extend(
        schedulability
            .into_iter()
            .map(|(_, name)| item(name, Some(Capability::Schedulability))),
    );
    items.extend(
        view.model_selections()
            .into_iter()
            .map(|selection| ItemSelection {
                claim: Some(selection.identity),
                temporal: Some(super::TemporalSelection {
                    item_kind: Some(super::ItemKindSelection::DeadlockFreedom),
                    ..super::TemporalSelection::default()
                }),
                ..ItemSelection::default()
            }),
    );
    items
}

/// Whether the package node `node` is a `claim`/`schedulability` node (FR-246).
fn is_schedulability(view: &dyn AnalysisView, node: &CheckedNodeId) -> bool {
    view.package()
        .resolved_graph()
        .nodes
        .iter()
        .find(|candidate| candidate.node_id == *node)
        .is_some_and(|candidate| {
            &*candidate.node_tag == "claim" && candidate.semantic_form.as_ref() == "schedulability"
        })
}

/// The refusal of a declaration whose package rows the route cannot read as
/// a closed-form subject: typed, naming the declaration.
fn pending_declaration(index: RequestIndex, declaration: &CheckedNodeId) -> PackageRunError {
    PackageRunError::Pending(PendingPackageFamily {
        request_index: Some(index),
        note: None,
        family: PendingFamily::Declaration,
        node: Some(declaration.clone()),
    })
}
