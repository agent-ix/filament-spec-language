// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1030: the native-run refinement-transfer route.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use filament_exact::ValueType;
use filament_semantic_value::fairness::{FairnessConstraint, PropertyForm};
use fsl_analyze::analyze::document::{
    analyze_document, resolve_items, AnalyzeDocumentError, AnalyzeRun, DocumentSelection,
    ItemSelection, ResolvedItem, SubjectSelection,
};
use fsl_analyze::analyze::{AnalyzeLimits, SourceProvision};
use fsl_foundation::Code;
use fsl_replay::planner::{
    read_premise_item, read_result_entry, EntryOrigin, ItemKey, TransferFacts,
    TransferPremiseValue, TransferSettlement,
};
use fsl_replay::spine::CheckedUnit;
use fsl_replay::{Certification, ItemLabel};
use fsl_semantics::check::{
    CheckedGraph, CheckedRefinement, StepClass, StepTarget, TemporalClauseNode, TemporalProfile,
};
use fsl_semantics::model::intake::PackageInput;
use fsl_semantics::model::observation::{
    document_reference, AdmissionFailure, DocumentProvision, DocumentRef, ObservationLimits,
};
use serde_json::{json, Map, Value};

use super::{
    check_sources, intake_with_bytes, package_name, wire, FileCounts, FormalSource, NativeResult,
    Result, RunCause, RunResult, Settle,
};

/// The request and its program source, admitted without compiling or
/// evaluating. Execution compiles the checked package before resolving names.
pub struct PreparedTransfer {
    pub(super) directory: PathBuf,
    pub(super) dependent_bytes: usize,
    pub(super) request: wire::Request,
    pub(super) source: FormalSource,
    pub(super) integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
}

pub(super) fn execute(prepared: PreparedTransfer, settle: &Settle<'_>) -> Result<RunResult> {
    let PreparedTransfer {
        directory,
        dependent_bytes,
        request,
        source,
        integer_division,
    } = prepared;
    let wire::Request {
        models,
        program,
        libraries,
        extensions,
        clocks,
        refinement_transfer,
        ..
    } = request;
    let Some(member) = refinement_transfer else {
        return Err(invalid("refinement_transfer", "missing transfer member"));
    };
    let files = member.observations.values().collect::<BTreeSet<_>>();
    let mut intake = intake_with_bytes(
        &directory,
        FileCounts {
            programs: libraries.len().saturating_add(1),
            models: models.len(),
            observations: files.len(),
            extensions: extensions.len(),
        },
        dependent_bytes,
    )?;
    let observation_limits =
        fsl_replay::spine::ObservationLimitsWire::resolve(member.observation_limits.as_ref())
            .map_err(|error| invalid(&error.member, error.reason))?;
    let mut observation_bytes = BTreeMap::new();
    for file in files {
        let bytes = intake.file(
            file,
            usize::try_from(observation_limits.document_bytes).unwrap_or(usize::MAX),
        )?;
        observation_bytes.insert(file.clone(), bytes);
    }
    let libraries = libraries
        .into_iter()
        .map(|library| wire::Library {
            identity: library.identity,
            refinement_internals: library.refinement_internals,
            source: library.source,
        })
        .collect::<Vec<_>>();
    let program = program.ok_or_else(|| invalid("program", "missing program source"))?;
    let compile = wire::CompileRequest {
        models: Some(models),
        program: wire::CompileProgram {
            sources: vec![program.source],
            extraction: false,
        },
        refinement_internals: None,
        libraries: Some(libraries),
        extensions,
        clocks: Some(clocks),
        output: None,
        handoff: None,
        integer_division: None,
        check_mode: filament_semantic_value::checking::CheckMode::Linked,
    };
    let sources = [source];
    check_sources(
        &compile,
        &mut intake,
        &sources,
        false,
        fsl_replay::spine::SpineLimits::default(),
        integer_division,
        None,
        |_, checked, _, _, packages, _libraries, _, _, _, _| {
            Transfer {
                member: &member,
                checked: &checked,
                packages: &packages,
                settle,
                observation_limits,
                observation_bytes: &observation_bytes,
            }
            .run()
        },
    )
}

/// One transfer request over its compiled package.
struct Transfer<'a> {
    member: &'a wire::RefinementTransfer,
    checked: &'a CheckedUnit,
    packages: &'a PackageInput,
    settle: &'a Settle<'a>,
    observation_limits: ObservationLimits,
    observation_bytes: &'a BTreeMap<String, Vec<u8>>,
}

/// The declarations FR-1031's selection rows resolve.
struct Selected<'g> {
    refinement: &'g CheckedRefinement,
    clauses: Vec<&'g TemporalClauseNode>,
}

/// One subject's selection and the snapshots it reads.
struct Subjects {
    documents: DocumentProvision,
    abstract_subject: SubjectSelection,
    concrete: SubjectSelection,
}

/// An item resolved for settlement: its slot identity and request item wire.
struct Premise {
    resolved: ResolvedItem,
    key: ItemKey,
    identity: Value,
    wire: Value,
}

impl Transfer<'_> {
    fn run(&self) -> Result<RunResult> {
        let package = self.checked.package();
        let graph = package.graph();
        let selected = self.select(graph)?;
        let refinement = selected.refinement;
        let subjects = self.subjects()?;
        self.admit_snapshots(&subjects)?;

        let mut registry = fsl_route::Registry::new();
        for descriptor in self.settle.engines.registrations() {
            registry
                .register(descriptor.clone())
                .map_err(|_| RunCause::AnalyzeFault)?;
        }
        let limits = AnalyzeLimits {
            observation: self.observation_limits,
            ..AnalyzeLimits::unbounded()
        };
        let abstract_binding = subjects
            .abstract_subject
            .binding(self.packages, &subjects.documents);
        let concrete_binding = subjects
            .concrete
            .binding(self.packages, &subjects.documents);
        let units = [self.checked.source().reference().clone()];
        let resolve = |subject: &SubjectSelection,
                       binding: &fsl_analyze::analyze::SubjectBinding<'_>,
                       selections: &[ItemSelection]| {
            resolve_items(
                package,
                subject,
                Some(selections),
                &registry,
                &units,
                binding,
                &limits,
            )
            .into_iter()
            .map(|resolved| self.premise(resolved, subject))
            .collect::<Result<Vec<_>>>()
        };

        let refinement_selection = self.refinement_selection(refinement, &subjects)?;
        let [refinement_premise] = <[Premise; 1]>::try_from(resolve(
            &subjects.concrete,
            &concrete_binding,
            &refinement_selection,
        )?)
        .map_err(|_| RunCause::AnalyzeFault)?;
        let claim_selections = self.claim_selections(&selected, &subjects)?;
        let claim_premises = resolve(
            &subjects.abstract_subject,
            &abstract_binding,
            &claim_selections,
        )?;

        let supplied_refinement = self
            .member
            .premises
            .as_ref()
            .and_then(|premises| premises.refinement.as_ref())
            .map(|supplied| {
                supplied_premise(
                    supplied,
                    &refinement_premise,
                    "/refinement_transfer/premises/refinement",
                )
            })
            .transpose()?;
        let mut supplied_claims = Vec::new();
        for (node, premise) in selected.clauses.iter().zip(&claim_premises) {
            let name = node.clause().name();
            supplied_claims.push(
                self.member
                    .premises
                    .as_ref()
                    .and_then(|premises| premises.claims.get(name))
                    .map(|supplied| {
                        supplied_premise(
                            supplied,
                            premise,
                            &format!("/refinement_transfer/premises/claims/{name}"),
                        )
                    })
                    .transpose()?,
            );
        }

        let settled_refinement = match supplied_refinement {
            Some(supplied) => supplied.value,
            None => {
                let analyzed = self.analyze(
                    package,
                    concrete_binding,
                    vec![refinement_premise.resolved.clone()],
                    limits,
                )?;
                analyze_record(analyzed.first().ok_or(RunCause::AnalyzeFault)?)?
            }
        };
        let in_process: Vec<usize> = supplied_claims
            .iter()
            .enumerate()
            .filter_map(|(index, supplied)| supplied.is_none().then_some(index))
            .collect();
        let analyzed = if in_process.is_empty() {
            Vec::new()
        } else {
            self.analyze(
                package,
                abstract_binding,
                in_process
                    .iter()
                    .map(|index| claim_premises[*index].resolved.clone())
                    .collect(),
                limits,
            )?
        };
        let mut analyzed = analyzed.iter();

        let dense = graph.time_sources().iter().any(|time| {
            time.model == self.member.concrete.model
                && matches!(time.source, fsl_semantics::check::TimeSourceKind::Dense)
        });
        let step_for_step = step_for_step_fact(refinement, dense);
        let concrete_subject = json!({
            "model": self.member.concrete.model,
            "initial": self.member.concrete.initial,
        });
        let refinement_node = refinement
            .v2_emission()
            .map_err(|_| RunCause::AnalyzeFault)?
            .identity;
        let budget = fsl_replay::planner::Budget {
            max_work: self
                .member
                .limits
                .max_work
                .unwrap_or(fsl_replay::planner::DEFAULT_MAX_WORK),
        };
        let mut transfers = Vec::new();
        for ((node, premise), supplied) in selected
            .clauses
            .iter()
            .zip(claim_premises)
            .zip(supplied_claims)
        {
            let (claim, stutter_closure) = match supplied {
                Some(supplied) => (supplied.value, supplied.stutter_closure),
                None => {
                    let record = analyzed.next().ok_or(RunCause::AnalyzeFault)?;
                    (analyze_record(record)?, true)
                }
            };
            let liveness = matches!(node.clause().property_form(), Some(PropertyForm::Liveness));
            let unimplied = node
                .clause()
                .fairness()
                .iter()
                .find(|needed| !fairness_implies(refinement, needed))
                .cloned();
            let facts = TransferFacts {
                package: package_name(self.checked.source().identity()),
                refinement: refinement_node.clone(),
                clause: node.identity(),
                subject: concrete_subject.clone(),
                refinement_identity: refinement_premise.identity.clone(),
                abstract_claim_identity: premise.identity,
                step_for_step: step_for_step.clone(),
                fragment_operator: node.clause().position_counting_operator(),
                fairness_implied: unimplied.is_none(),
                unimplied_constraint: unimplied,
                liveness,
            };
            let settlement = TransferSettlement::from_settled_facts(
                &facts,
                stutter_closure,
                [
                    (facts.refinement_identity.clone(), settled_refinement),
                    (facts.abstract_claim_identity.clone(), claim),
                ],
            );
            let result = fsl_replay::planner::settle_transfer(&settlement, budget)
                .map_err(|_| RunCause::AnalyzeFault)?;
            transfers.push(json!({"clause": node.clause().name(), "result": result}));
        }
        let exit_code = if transfers
            .iter()
            .all(|transfer| transfer["result"]["value"] == "proved")
        {
            0
        } else {
            22
        };
        let refinement_value = json!({
            "name": refinement.name(),
            "node": fsl_foundation::node_wire::node_key_to_json(&refinement_node)
                .map_err(|_| RunCause::AnalyzeFault)?,
            "step_for_step": step_for_step,
        });
        Ok(RunResult {
            exit_code,
            value: NativeResult::analyze_document(json!({
                "format": "native-run-result/2",
                "refinement_transfer": {
                    "refinement": refinement_value,
                    "transfers": transfers,
                }
            })),
            outcome: None,
            requirements: None,
            sources: super::super::Sources::default(),
        })
    }

    /// FR-1031's `select` rows in table order: the model aliases, the
    /// refinement and the clause names, then the declared subjects, then each
    /// clause's family and `over` subject.
    fn select<'g>(&self, graph: &'g CheckedGraph) -> Result<Selected<'g>> {
        let member = self.member;
        require_selected_model(graph, &member.abstract_subject.model, "abstract")?;
        require_selected_model(graph, &member.concrete.model, "concrete")?;
        let refinement = graph
            .refinements()
            .iter()
            .find(|refinement| refinement.name() == member.refinement)
            .ok_or_else(|| missing_name("/refinement_transfer/refinement", &member.refinement))?;
        let named = member
            .clauses
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let pointer = format!("/refinement_transfer/clauses/{index}");
                match graph
                    .temporal_clauses()
                    .iter()
                    .find(|node| node.clause().name() == name)
                {
                    Some(node) => Ok(Some(node)),
                    None if declaration_kind(graph, name).is_some() => Ok(None),
                    None => Err(missing_name(&pointer, name)),
                }
            })
            .collect::<Result<Vec<_>>>()?;
        for (side, selected, declared) in [
            (
                "abstract",
                &member.abstract_subject.model,
                refinement.abstract_model(),
            ),
            (
                "concrete",
                &member.concrete.model,
                refinement.concrete_model(),
            ),
        ] {
            if selected != declared {
                return Err(model_mismatch(
                    &format!("/refinement_transfer/{side}/model"),
                    declared,
                    selected,
                ));
            }
        }
        let mut clauses = Vec::with_capacity(named.len());
        for (index, (name, node)) in member.clauses.iter().zip(named).enumerate() {
            let pointer = format!("/refinement_transfer/clauses/{index}");
            let Some(node) = node else {
                let kind = declaration_kind(graph, name).unwrap_or("declaration");
                return Err(type_mismatch(&pointer, kind, None));
            };
            let profile = node.clause().profile();
            if profile != TemporalProfile::InfiniteTrace {
                return Err(type_mismatch(
                    &pointer,
                    "temporal clause",
                    Some(profile.identity()),
                ));
            }
            clauses.push(node);
        }
        let abstract_package = &refinement.abstract_model_selection().identity;
        for (index, node) in clauses.iter().enumerate() {
            let over_abstract = matches!(
                &node.clause().over().1,
                ValueType::Reference(object) if object.original().package() == abstract_package
            );
            if !over_abstract {
                return Err(RunCause::TransferSelection {
                    stage: "select",
                    member: format!("/refinement_transfer/clauses/{index}"),
                    cause: "invalid-value",
                    code: fsl_foundation::Code::InvalidRuntimeInput,
                    message: format!(
                        "clause {} is not over the abstract model {abstract_package}",
                        node.clause().name()
                    ),
                    details: Map::new(),
                });
            }
        }
        Ok(Selected {
            refinement,
            clauses,
        })
    }

    /// The observations' provisioned documents and both subjects.
    fn subjects(&self) -> Result<Subjects> {
        let member = self.member;
        let mut documents = DocumentProvision::new();
        let mut refs = BTreeMap::new();
        let mut by_file = BTreeMap::<&String, DocumentRef>::new();
        for (key, file) in &member.observations {
            let reference = match by_file.get(file) {
                Some(reference) => reference.clone(),
                None => {
                    let bytes = self.observation_bytes.get(file).ok_or_else(|| {
                        invalid(
                            "/refinement_transfer/observations",
                            "missing observation bytes",
                        )
                    })?;
                    let reference = document_reference(bytes).ok_or_else(|| {
                        invalid(
                            format!("/refinement_transfer/observations/{key}"),
                            "the observation has no document identity",
                        )
                    })?;
                    if documents.insert(reference.clone(), bytes.clone()).is_some() {
                        return Err(invalid(
                            format!("/refinement_transfer/observations/{key}"),
                            "the observation identity is provisioned more than once",
                        ));
                    }
                    by_file.insert(file, reference.clone());
                    reference
                }
            };
            refs.insert(
                key.clone(),
                DocumentSelection {
                    authority: reference.authority,
                    identity: reference.identity,
                },
            );
        }
        let abstract_subject = subject(&member.abstract_subject.initial, &refs, "abstract")?;
        let concrete = subject(&member.concrete.initial, &refs, "concrete")?;
        for subject in [&abstract_subject, &concrete] {
            subject
                .check_universes(&documents)
                .map_err(RunCause::AnalyzeUniverse)?;
        }
        Ok(Subjects {
            documents,
            abstract_subject,
            concrete,
        })
    }

    /// FR-1031's `initial` row: FR-106 admits each initial snapshot of each
    /// subject for its model before any premise or transfer settles.
    fn admit_snapshots(&self, subjects: &Subjects) -> Result<()> {
        let package = self.checked.package();
        for (side, subject) in [
            ("abstract", &subjects.abstract_subject),
            ("concrete", &subjects.concrete),
        ] {
            let binding = subject.binding(self.packages, &subjects.documents);
            for (index, initial) in binding.initial.iter().enumerate() {
                let request = fsl_eval::simulation::model::ModelSimulationRequest {
                    input: fsl_eval::simulation::model::ModelCheckInput::source(package),
                    packages: self.packages,
                    normalization: AnalyzeLimits::unbounded().normalization,
                    snapshots: &subjects.documents,
                    initial: vec![initial.clone()],
                    universes: binding.universes.clone(),
                    observation: self.observation_limits,
                    expansion: fsl_eval::simulation::model::ExpansionLimits {
                        max_candidates: u64::MAX,
                    },
                    meter: AnalyzeLimits::unbounded().accounting,
                    emit: AnalyzeLimits::unbounded().emit,
                };
                match fsl_eval::simulation::model::ModelSystem::new(request) {
                    Ok(_) => {}
                    Err(
                        AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record),
                    ) => {
                        let code = Code::from_code(record.catalog.code())
                            .unwrap_or(Code::InvalidRuntimeInput);
                        return Err(RunCause::TransferSelection {
                            stage: "admit",
                            member: format!("/refinement_transfer/{side}/initial/{index}"),
                            cause: record.catalog.cause(),
                            code,
                            message: format!(
                                "snapshot admission refused: {}/{}",
                                record.catalog.code(),
                                record.catalog.cause()
                            ),
                            details: Map::new(),
                        });
                    }
                    Err(AdmissionFailure::Fault(_)) => return Err(RunCause::AnalyzeFault),
                }
            }
        }
        Ok(())
    }

    /// The FR-144 refinement item, over the concrete subject, with the
    /// abstract subject's initial states.
    fn refinement_selection(
        &self,
        refinement: &CheckedRefinement,
        subjects: &Subjects,
    ) -> Result<Vec<ItemSelection>> {
        selections(json!([{
            "claim": refinement.name(),
            "kind": "refinement",
            "temporal": {
                "item_kind": "refinement",
                "subject": subject_wire(&subjects.concrete),
                "abstract_subject": {
                    "identity": refinement.abstract_model_selection().identity,
                    "initial_states": subjects
                        .abstract_subject
                        .initial_states
                        .iter()
                        .map(selection_wire)
                        .collect::<Vec<_>>(),
                },
            },
        }]))
    }

    /// One abstract claim item per clause, over the abstract subject; a
    /// liveness clause is checked over its stutter closure (FR-529).
    fn claim_selections(
        &self,
        selected: &Selected<'_>,
        subjects: &Subjects,
    ) -> Result<Vec<ItemSelection>> {
        let items = selected
            .clauses
            .iter()
            .map(|node| {
                let mut temporal = json!({"subject": subject_wire(&subjects.abstract_subject)});
                if matches!(node.clause().property_form(), Some(PropertyForm::Liveness)) {
                    temporal["stutter_closure"] = json!(true);
                }
                json!({
                    "claim": node.clause().name(),
                    "kind": "temporal-satisfaction",
                    "temporal": temporal,
                })
            })
            .collect();
        selections(Value::Array(items))
    }

    /// `resolved`'s slot identity and request item wire over `subject`.
    fn premise(&self, resolved: ResolvedItem, subject: &SubjectSelection) -> Result<Premise> {
        let ResolvedItem::Claim(claim) = &resolved else {
            return Err(RunCause::AnalyzeFault);
        };
        let wire = claim
            .item
            .request_item_wire(subject)
            .map_err(|_| RunCause::AnalyzeFault)?;
        let item = read_premise_item(&wire).map_err(|_| RunCause::AnalyzeFault)?;
        let identity = item.key.to_wire().map_err(|_| RunCause::AnalyzeFault)?;
        Ok(Premise {
            resolved,
            key: item.key,
            identity,
            wire,
        })
    }

    /// The item records `items` settle to, in order.
    fn analyze(
        &self,
        package: &fsl_package::CheckedPackage,
        binding: fsl_analyze::analyze::SubjectBinding<'_>,
        items: Vec<ResolvedItem>,
        limits: AnalyzeLimits,
    ) -> Result<Vec<Value>> {
        let mut provision = SourceProvision::default();
        for source in std::iter::once(self.checked.source()).chain(self.checked.libraries()) {
            provision.insert(source.reference(), source.text().as_bytes().to_vec());
        }
        let analyzed = analyze_document(
            AnalyzeRun {
                package,
                binding,
                provision,
                items,
                engines: self.settle.engines,
                limits,
            },
            self.settle.negotiate,
            self.settle.cancel,
        )
        .map_err(|error| match error {
            AnalyzeDocumentError::Refused(refusal) => RunCause::AnalyzeRefused(Box::new(refusal)),
            AnalyzeDocumentError::Fault => RunCause::AnalyzeFault,
        })?;
        analyzed
            .0
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .ok_or(RunCause::AnalyzeFault)
    }
}

fn selections(items: Value) -> Result<Vec<ItemSelection>> {
    serde_json::from_value(items).map_err(|error| invalid("refinement_transfer", error.to_string()))
}

fn subject(
    initial: &[String],
    references: &std::collections::BTreeMap<String, DocumentSelection>,
    side: &str,
) -> Result<fsl_analyze::analyze::document::SubjectSelection> {
    let initial_states = initial
        .iter()
        .enumerate()
        .map(|(index, key)| {
            references.get(key).cloned().ok_or_else(|| {
                invalid(
                    format!("/refinement_transfer/{side}/initial/{index}"),
                    "initial snapshot key is absent from observations",
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(fsl_analyze::analyze::document::SubjectSelection {
        universes: initial_states.clone(),
        initial_states,
        proof_bounds: Vec::new(),
        protocol: None,
        memory: Vec::new(),
    })
}

fn subject_wire(subject: &fsl_analyze::analyze::document::SubjectSelection) -> Value {
    json!({
        "initial_states": subject.initial_states.iter().map(selection_wire).collect::<Vec<_>>(),
        "universes": subject.universes.iter().map(selection_wire).collect::<Vec<_>>(),
        "proof_bounds": subject.proof_bounds,
    })
}

fn selection_wire(selection: &DocumentSelection) -> Value {
    json!({
        "authority": selection.authority,
        "identity": selection.identity,
    })
}

fn require_selected_model(
    graph: &fsl_semantics::check::CheckedGraph,
    model: &str,
    side: &str,
) -> Result<()> {
    if graph.model_package_identity(model).is_none() {
        return Err(missing_name(
            &format!("/refinement_transfer/{side}/model"),
            model,
        ));
    }
    Ok(())
}

fn step_for_step_fact(refinement: &CheckedRefinement, dense: bool) -> Value {
    if refinement.is_step_for_step(dense) {
        return json!({"holds": true});
    }
    // FSpec FR-820: a dense concrete subject is not step-for-step before any
    // row is read.
    if dense {
        return json!({"holds": false, "time": "dense"});
    }
    let row = refinement.steps().iter().find(|row| {
        !matches!(&row.class, StepClass::Operation(_))
            || !matches!(&row.target, StepTarget::Abstract { .. })
    });
    let Some(row) = row else {
        return json!({"holds": false});
    };
    let operation = row.concrete_operation().map(|(ty, name)| {
        let node = ty.original().node();
        format!(
            "{}::{}::{name}",
            refinement.concrete_model(),
            node.rsplit('/').next().unwrap_or(node)
        )
    });
    let arm = match &row.target {
        StepTarget::Stutter => "stutter",
        StepTarget::Any => "any",
        StepTarget::Node { .. } => "node",
        StepTarget::Abstract { .. } => "abstract",
    };
    json!({"holds": false, "row": {"operation": operation, "arm": arm}})
}

fn fairness_implies(refinement: &CheckedRefinement, needed: &FairnessConstraint) -> bool {
    refinement.abstract_operation_fairness().any(|supplied| {
        supplied.operation() == needed.operation()
            && (supplied.kind() == filament_semantic_value::fairness::FairnessKind::Strong
                || needed.kind() == filament_semantic_value::fairness::FairnessKind::Weak)
            && (supplied.granularity() == needed.granularity()
                || (needed.granularity()
                    == filament_semantic_value::fairness::FairnessGranularity::Whole
                    && supplied.granularity()
                        == filament_semantic_value::fairness::FairnessGranularity::Each))
    })
}

/// The slot value an analyze item `record` settled: its result label, and
/// for `proved` its certification.
fn analyze_record(value: &Value) -> Result<TransferPremiseValue> {
    let label: ItemLabel = value
        .pointer("/record/result")
        .cloned()
        .and_then(|result| serde_json::from_value(result).ok())
        .ok_or(RunCause::AnalyzeFault)?;
    if label != ItemLabel::Proved {
        return Ok(TransferPremiseValue::Other(label));
    }
    let certification = value
        .pointer("/record/certification")
        .cloned()
        .and_then(|certification| serde_json::from_value(certification).ok())
        .unwrap_or(Certification::Uncertified);
    Ok(TransferPremiseValue::Proved(certification))
}

/// A supplied premise, admitted: its slot value and whether its item was
/// checked over the stutter closure, which FSpec FR-820 reads from the
/// supplied result.
struct Supplied {
    value: TransferPremiseValue,
    stutter_closure: bool,
}

/// Admit the supplied premise result `value` for `expected`'s slot under
/// FSpec FR-820 "Admission": its item is the slot's, over the slot's subject.
/// The slot key does not fix `stutter_closure`, which the planner reads from
/// the result.
fn supplied_premise(value: &Value, expected: &Premise, pointer: &str) -> Result<Supplied> {
    let refused = |member: String, message: &str| {
        admission(member, "invalid_runtime_input", "invalid-value", message)
    };
    let request = value.get("request").ok_or_else(|| {
        refused(
            pointer.to_owned(),
            "premise result is missing its request item",
        )
    })?;
    let item = read_premise_item(request).map_err(|fault| {
        admission(
            format!("{pointer}/request{}", fault.pointer),
            fault.cause.code(),
            fault.cause.as_str(),
            &fault.to_string(),
        )
    })?;
    let stutter_closure = item
        .key
        .temporal
        .as_ref()
        .is_some_and(|temporal| temporal.stutter_closure);
    let slot_key = |mut key: ItemKey| {
        if let Some(temporal) = key.temporal.as_mut() {
            temporal.stutter_closure = false;
        }
        key
    };
    if slot_key(item.key) != slot_key(expected.key.clone()) {
        return Err(refused(
            pointer.to_owned(),
            "premise item identity does not match its slot",
        ));
    }
    if request.pointer("/temporal/subject") != expected.wire.pointer("/temporal/subject") {
        return Err(refused(
            pointer.to_owned(),
            "premise item subject does not match the selected subject",
        ));
    }
    let result = value.get("result").ok_or_else(|| {
        refused(
            pointer.to_owned(),
            "premise result is missing its result entry",
        )
    })?;
    let entry = read_result_entry(result, 0, EntryOrigin::FirstParty)
        .map_err(|fault| {
            admission(
                format!("{pointer}/result{}", fault.pointer),
                fault.cause.code(),
                fault.cause.as_str(),
                &fault.to_string(),
            )
        })?
        .read_from_bytes();
    Ok(Supplied {
        value: TransferPremiseValue::of(entry.record.value()),
        stutter_closure,
    })
}

fn invalid(member: impl Into<String>, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: member.into(),
        reason: reason.into(),
    }
}

/// An `admit`-stage refusal at `member` with FR-272's or FR-106's `code`.
fn admission(member: String, code: &str, cause: &'static str, message: &str) -> RunCause {
    RunCause::TransferSelection {
        stage: "admit",
        member,
        cause,
        code: Code::from_code(code).unwrap_or(Code::InvalidRuntimeInput),
        message: message.to_owned(),
        details: Map::new(),
    }
}

/// The kind of declaration `name` names in `graph` when it is no temporal
/// clause, for FR-1031's family refusal.
fn declaration_kind(graph: &CheckedGraph, name: &str) -> Option<&'static str> {
    if graph.state_clause(name).is_some() {
        Some("state clause")
    } else if graph.protocol(name).is_some() {
        Some("protocol clause")
    } else if graph
        .hyper_clauses()
        .iter()
        .any(|clause| clause.name() == name)
        || graph
            .finite_hyper_clauses()
            .iter()
            .any(|clause| clause.name() == name)
    {
        Some("hyper clause")
    } else {
        graph
            .temporal_clauses()
            .iter()
            .any(|node| node.clause().name() == name)
            .then_some("temporal clause")
    }
}

fn missing_name(member: &str, name: &str) -> RunCause {
    RunCause::TransferSelection {
        stage: "select",
        member: member.to_owned(),
        cause: "missing-name",
        code: Code::MissingDeclaration,
        message: format!("no declaration named {name}"),
        details: Map::new(),
    }
}

fn model_mismatch(member: &str, declared: &str, given: &str) -> RunCause {
    RunCause::TransferSelection {
        stage: "select",
        member: member.to_owned(),
        cause: "invalid-value",
        code: Code::InvalidRuntimeInput,
        message: format!("selected model is {given}, but refinement declares {declared}"),
        details: Map::from_iter([
            ("declared".to_owned(), json!(declared)),
            ("given".to_owned(), json!(given)),
        ]),
    }
}

fn type_mismatch(member: &str, kind: &str, profile: Option<&str>) -> RunCause {
    let message = match profile {
        Some(profile) => format!("{kind} with profile {profile} is not an infinite-trace clause"),
        None => format!("{kind} is not an infinite-trace temporal clause"),
    };
    RunCause::TransferSelection {
        stage: "select",
        member: member.to_owned(),
        cause: "type-mismatch",
        code: Code::IllTyped,
        message,
        details: Map::new(),
    }
}
