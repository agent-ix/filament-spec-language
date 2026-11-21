// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1030: the native-run refinement-transfer route.

use std::path::PathBuf;

use filament_semantic_value::fairness::{FairnessConstraint, PropertyForm};
use fsl_analyze::analyze::document::{
    analyze_document, resolve_items, AnalyzeDocumentError, AnalyzeRun, DocumentSelection,
    ItemSelection,
};
use fsl_analyze::analyze::{AnalyzeLimits, SourceProvision};
use fsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use fsl_foundation::bound::ProofBound;
use fsl_replay::planner::{
    read_premise_item, read_result_entry, ClaimKind, EntryOrigin, ExtentClass, FairnessKey,
    ItemKey, TemporalItemKind, TemporalKey, TransferFacts, TransferPremiseValue,
    TransferSettlement,
};
use fsl_replay::{
    settle_refinement, Certification, ModelCheckLimits, SimulationCertificateRequest, TerminalValue,
};
use fsl_semantics::check::{CheckedRefinement, StepClass, StepTarget, TemporalProfile};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::observation::{document_reference, DocumentProvision, DocumentRef};
use serde_json::{json, Value};

use super::{
    check_sources, intake_with_bytes, package_name, wire, FileCounts, FormalSource, NativeResult,
    Result, RunCause, RunResult,
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

pub(super) fn execute(prepared: PreparedTransfer, settle: &super::Settle<'_>) -> Result<RunResult> {
    let PreparedTransfer {
        directory,
        dependent_bytes,
        request,
        source,
        integer_division,
    } = prepared;
    let member = request
        .refinement_transfer
        .as_ref()
        .ok_or_else(|| invalid("refinement_transfer", "missing transfer member"))?;
    let mut intake = intake_with_bytes(
        &directory,
        FileCounts {
            programs: request.libraries.len().saturating_add(1),
            models: request.models.len(),
            observations: member
                .observations
                .values()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            extensions: request.extensions.len(),
        },
        dependent_bytes,
    )?;
    let wire::Request {
        models,
        program,
        package: _,
        limits: _,
        call: _,
        clause: _,
        items: _,
        libraries,
        extensions,
        clocks,
        tl_lowering: _,
        model_based_test: _,
        verification_plan: _,
        synthesis: _,
        integer_division: _,
        refinement_transfer,
    } = request;
    let member = refinement_transfer.expect("validated transfer member");
    let observation_limits =
        fsl_replay::spine::ObservationLimitsWire::resolve(member.observation_limits.as_ref())
            .map_err(|error| invalid(&error.member, error.reason))?;
    let mut observation_bytes = std::collections::BTreeMap::new();
    for file in member
        .observations
        .values()
        .collect::<std::collections::BTreeSet<_>>()
    {
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
            let checked_package = checked.package();
            let graph = checked_package.graph();
            let refinement = graph
                .refinements()
                .iter()
                .find(|refinement| refinement.name() == member.refinement)
                .ok_or_else(|| {
                    missing_name("/refinement_transfer/refinement", &member.refinement)
                })?;
            require_selected_model(graph, &member.abstract_subject.model, "abstract")?;
            require_selected_model(graph, &member.concrete.model, "concrete")?;
            if member.abstract_subject.model != refinement.abstract_model() {
                return Err(model_mismatch(
                    "/refinement_transfer/abstract/model",
                    refinement.abstract_model(),
                    &member.abstract_subject.model,
                ));
            }
            if member.concrete.model != refinement.concrete_model() {
                return Err(model_mismatch(
                    "/refinement_transfer/concrete/model",
                    refinement.concrete_model(),
                    &member.concrete.model,
                ));
            }
            let clauses = member
                .clauses
                .iter()
                .enumerate()
                .map(|(index, name)| {
                    let clause = graph
                        .temporal_clauses()
                        .iter()
                        .find(|node| node.clause().name() == name)
                        .ok_or_else(|| {
                            missing_name(&format!("/refinement_transfer/clauses/{index}"), name)
                        })?;
                    if clause.clause().profile() != TemporalProfile::InfiniteTrace {
                        return Err(type_mismatch(
                            &format!("/refinement_transfer/clauses/{index}"),
                            "temporal",
                            clause.clause().profile().identity(),
                        ));
                    }
                    Ok(clause)
                })
                .collect::<Result<Vec<_>>>()?;

            let mut documents = DocumentProvision::new();
            let mut refs = std::collections::BTreeMap::new();
            let mut by_file = std::collections::BTreeMap::<String, DocumentRef>::new();
            for (key, file) in &member.observations {
                let reference = if let Some(reference) = by_file.get(file) {
                    reference.clone()
                } else {
                    let bytes = observation_bytes.get(file).ok_or_else(|| {
                        invalid(
                            "refinement_transfer/observations",
                            "missing observation bytes",
                        )
                    })?;
                    let reference = document_reference(&bytes).ok_or_else(|| {
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
                    by_file.insert(file.clone(), reference.clone());
                    reference
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
            let concrete_subject = subject(&member.concrete.initial, &refs, "concrete")?;
            abstract_subject
                .check_universes(&documents)
                .map_err(|error| {
                    invalid("/refinement_transfer/abstract/initial", error.to_string())
                })?;
            concrete_subject
                .check_universes(&documents)
                .map_err(|error| {
                    invalid("/refinement_transfer/concrete/initial", error.to_string())
                })?;

            let dense = graph.time_sources().iter().any(|time| {
                time.model == member.concrete.model
                    && matches!(time.source, fsl_semantics::check::TimeSourceKind::Dense)
            });
            let step_for_step = step_for_step_fact(refinement, dense);
            let source_provision = {
                let mut provision = SourceProvision::default();
                for source in std::iter::once(checked.source()).chain(checked.libraries()) {
                    provision.insert(source.reference(), source.text().as_bytes().to_vec());
                }
                provision
            };
            let mut registry = fsl_route::Registry::new();
            for descriptor in settle.engines.registrations() {
                registry
                    .register(descriptor.clone())
                    .map_err(|_| RunCause::AnalyzeFault)?;
            }
            let refinement_occurrence = refinement
                .requirement_occurrence()
                .ok_or(RunCause::AnalyzeFault)?
                .clone();
            let mut fairness = refinement
                .concrete_operation_fairness()
                .chain(refinement.abstract_operation_fairness())
                .map(|constraint| FairnessKey::of(constraint).map_err(|_| RunCause::AnalyzeFault))
                .collect::<Result<Vec<_>>>()?;
            fairness.sort();
            let subject_bytes = quire_canonical::to_vec(
                &subject_wire(&concrete_subject),
                quire_canonical::Limits::new(u64::MAX),
            )
            .map_err(|_| RunCause::AnalyzeFault)?;
            let subject_identity =
                String::from_utf8(subject_bytes).map_err(|_| RunCause::AnalyzeFault)?;
            let refinement_identity = ItemKey {
                claim: refinement_occurrence,
                kind: ClaimKind::TemporalSatisfaction,
                temporal: Some(TemporalKey {
                    subject: subject_identity,
                    item_kind: TemporalItemKind::Refinement,
                    fairness,
                    stutter_closure: false,
                }),
                correspondence: None,
                premises: Vec::new(),
                extent: ExtentClass::Unbounded,
            }
            .to_wire()
            .map_err(|_| RunCause::AnalyzeFault)?;
            let concrete_binding = concrete_subject.binding(&packages, &documents);
            let concrete_simulation = fsl_eval::simulation::model::ModelSimulationRequest {
                input: fsl_eval::simulation::model::ModelCheckInput::source(checked_package),
                packages: &packages,
                normalization: ModelNormalizationLimits::default(),
                snapshots: &documents,
                initial: concrete_binding.initial.clone(),
                universes: concrete_binding.universes.clone(),
                observation: observation_limits,
                expansion: fsl_eval::simulation::model::ExpansionLimits {
                    max_candidates: u64::MAX,
                },
                meter: fsl_replay::spine::default_accounting(u64::MAX),
                emit: fsl_package::EmitLimits::new(fsl_replay::spine::EMIT_OUTPUT_BYTES_DEFAULT),
            };
            let concrete = fsl_eval::model_subject::ModelSubject {
                simulation: concrete_simulation.clone(),
                proof_bounds: Vec::<ProofBound>::new(),
            };
            let abstract_initial = abstract_subject
                .initial_states
                .iter()
                .map(DocumentSelection::reference)
                .collect::<Vec<_>>();
            let refinement_premise = if let Some(supplied) = member
                .premises
                .as_ref()
                .and_then(|premises| premises.refinement.as_ref())
            {
                supplied_premise(
                    supplied,
                    Some(&refinement_identity),
                    Some(subject_wire(&concrete_subject)),
                    "/refinement_transfer/premises/refinement",
                )?
            } else {
                match check_refinement(
                    RefinementCheckRequest {
                        refinement,
                        concrete,
                        abstract_package: checked_package,
                        abstract_initial: fsl_replay::AbstractInitial::Snapshots(
                            abstract_initial.clone(),
                        ),
                        abstract_protocol_binding: None,
                        max_depth: None,
                        limits: ModelCheckLimits::default(),
                    },
                    settle.cancel,
                ) {
                    Ok(outcome) => {
                        let certificate = SimulationCertificateRequest {
                            refinement,
                            concrete: concrete_simulation,
                            abstract_package: checked_package,
                            abstract_initial: fsl_replay::AbstractInitial::Snapshots(
                                abstract_initial.clone(),
                            ),
                        };
                        premise_value(
                            &settle_refinement(
                                &outcome,
                                &ModelCheckLimits::default(),
                                &certificate,
                                settle.cancel,
                                |_| {
                                    Err(fsl_replay::ReplayRefusal::Fault(
                                        filament_semantic_value::catalog::InternalFault::new(
                                            "refinement-transfer",
                                            "counterexample-replay-unavailable",
                                        ),
                                    ))
                                },
                            )
                            .record
                            .value,
                        )
                    }
                    Err(_) => TransferPremiseValue::Other("failed"),
                }
            };

            let mut item_values = Vec::new();
            for node in &clauses {
                let liveness =
                    matches!(node.clause().property_form(), Some(PropertyForm::Liveness));
                let mut temporal = json!({
                    "subject": subject_wire(&abstract_subject),
                });
                if liveness {
                    temporal["stutter_closure"] = json!(true);
                }
                item_values.push(json!({
                    "claim": node.clause().name(),
                    "kind": "temporal-satisfaction",
                    "temporal": temporal,
                }));
            }
            let selections: Vec<ItemSelection> = serde_json::from_value(Value::Array(item_values))
                .map_err(|error| invalid("refinement_transfer/clauses", error.to_string()))?;
            let abstract_binding = abstract_subject.binding(&packages, &documents);
            let analyze_limits = AnalyzeLimits::unbounded();
            let resolved = resolve_items(
                checked_package,
                &abstract_subject,
                Some(&selections),
                &registry,
                &[checked.source().reference().clone()],
                &abstract_binding,
                &analyze_limits,
            );
            let identities = resolved
                .iter()
                .map(|resolved| match resolved {
                    fsl_analyze::analyze::document::ResolvedItem::Claim(claim) => {
                        let wire = claim
                            .item
                            .request_item_wire(&abstract_subject)
                            .map_err(|_| RunCause::AnalyzeFault)?;
                        let item = fsl_replay::planner::read_premise_item(&wire)
                            .map_err(|_| RunCause::AnalyzeFault)?;
                        item.key.to_wire().map_err(|_| RunCause::AnalyzeFault)
                    }
                    fsl_analyze::analyze::document::ResolvedItem::Settled { .. } => {
                        Err(RunCause::AnalyzeFault)
                    }
                    fsl_analyze::analyze::document::ResolvedItem::Simulation(_) => {
                        Err(RunCause::AnalyzeFault)
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            let inprocess_indices: Vec<usize> = resolved
                .iter()
                .enumerate()
                .filter(|(index, _)| {
                    member
                        .premises
                        .as_ref()
                        .and_then(|premises| premises.claims.get(clauses[*index].clause().name()))
                        .is_none()
                })
                .map(|(index, _)| index)
                .collect();
            let inprocess = inprocess_indices
                .iter()
                .map(|index| resolved[*index].clone())
                .collect();
            let analyzed = analyze_document(
                AnalyzeRun {
                    package: checked_package,
                    binding: abstract_binding,
                    provision: source_provision,
                    items: inprocess,
                    engines: settle.engines,
                    limits: AnalyzeLimits::unbounded(),
                },
                settle.negotiate,
                settle.cancel,
            )
            .map_err(|error| match error {
                AnalyzeDocumentError::Refused(refusal) => {
                    RunCause::AnalyzeRefused(Box::new(refusal))
                }
                AnalyzeDocumentError::Fault => RunCause::AnalyzeFault,
            })?;
            let claim_records = analyzed
                .0
                .get("items")
                .and_then(Value::as_array)
                .ok_or(RunCause::AnalyzeFault)?;
            let mut transfers = Vec::new();
            let transfer_budget = fsl_replay::planner::Budget {
                max_work: member
                    .limits
                    .max_work
                    .unwrap_or(fsl_replay::planner::DEFAULT_MAX_WORK),
            };
            let mut inprocess_by_clause = std::collections::BTreeMap::new();
            for (index, claim_record) in inprocess_indices.iter().zip(claim_records) {
                inprocess_by_clause.insert(
                    clauses[*index].clause().name().to_owned(),
                    (identities[*index].clone(), analyze_record(claim_record)?),
                );
            }
            for (node, identity) in clauses.iter().zip(identities) {
                let claim_terminal = if let Some(supplied) = member
                    .premises
                    .as_ref()
                    .and_then(|premises| premises.claims.get(node.clause().name()))
                {
                    supplied_premise(
                        supplied,
                        Some(&identity),
                        Some(subject_wire(&abstract_subject)),
                        &format!(
                            "/refinement_transfer/premises/claims/{}",
                            node.clause().name()
                        ),
                    )?
                } else {
                    inprocess_by_clause
                        .remove(node.clause().name())
                        .map(|(_, value)| value)
                        .ok_or(RunCause::AnalyzeFault)?
                };
                let liveness =
                    matches!(node.clause().property_form(), Some(PropertyForm::Liveness));
                let fairness = node
                    .clause()
                    .fairness()
                    .iter()
                    .find(|needed| !fairness_implies(refinement, needed))
                    .cloned();
                let subject_wire = json!({
                    "model": member.concrete.model,
                    "initial": member.concrete.initial,
                });
                let facts = TransferFacts {
                    package: package_name(checked.source().identity()),
                    refinement: refinement
                        .v2_emission()
                        .map_err(|_| RunCause::AnalyzeFault)?
                        .identity,
                    clause: node.identity(),
                    subject: subject_wire,
                    refinement_identity: refinement_identity.clone(),
                    abstract_claim_identity: identity.clone(),
                    step_for_step: step_for_step.clone(),
                    fragment_operator: node.clause().position_counting_operator(),
                    fairness_implied: fairness.is_none(),
                    unimplied_constraint: fairness,
                    liveness,
                };
                let settlement = TransferSettlement::from_settled_facts(
                    &facts,
                    liveness,
                    [
                        (facts.refinement_identity.clone(), refinement_premise),
                        (identity, claim_terminal),
                    ],
                );
                let result = fsl_replay::planner::settle_transfer(&settlement, transfer_budget)
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
                "node": fsl_foundation::node_wire::node_key_to_json(
                    &refinement.v2_emission().map_err(|_| RunCause::AnalyzeFault)?.identity,
                )
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
        },
    )
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
    let holds = refinement.is_step_for_step(dense);
    if holds {
        return json!({"holds": true});
    }
    let row = refinement.steps().iter().find(|row| {
        !matches!(&row.class, StepClass::Operation(_))
            || !matches!(&row.target, StepTarget::Abstract { .. })
    });
    let Some(row) = row else {
        return json!({"holds": false});
    };
    let operation = row.concrete_operation().map(|(ty, name)| {
        format!(
            "{}::{}::{name}",
            refinement.concrete_model(),
            ty.original().node()
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

fn premise_value(value: &TerminalValue) -> TransferPremiseValue {
    match value {
        TerminalValue::Proved { certification, .. } => TransferPremiseValue::Proved(*certification),
        TerminalValue::Refuted => TransferPremiseValue::Other("refuted"),
        TerminalValue::Inconclusive(_) => TransferPremiseValue::Other("inconclusive"),
        TerminalValue::Unsupported(_) => TransferPremiseValue::Other("unsupported"),
        TerminalValue::Incomplete(_) => TransferPremiseValue::Other("incomplete"),
        TerminalValue::Failed | TerminalValue::ToolFailed(_) => {
            TransferPremiseValue::Other("failed")
        }
        TerminalValue::Tested => TransferPremiseValue::Other("tested"),
        TerminalValue::Declined { .. } => TransferPremiseValue::Other("declined"),
        TerminalValue::Measured(_) => TransferPremiseValue::Other("measured"),
    }
}

fn analyze_record(value: &Value) -> Result<TransferPremiseValue> {
    let result = value
        .get("record")
        .and_then(Value::as_object)
        .and_then(|record| record.get("result"))
        .and_then(Value::as_str)
        .ok_or(RunCause::AnalyzeFault)?;
    let certification = value
        .pointer("/record/certification")
        .and_then(Value::as_str);
    Ok(match result {
        "proved" => TransferPremiseValue::Proved(match certification {
            Some("certified") => Certification::Certified,
            Some("trusted") => Certification::Trusted,
            _ => Certification::Uncertified,
        }),
        "refuted" => TransferPremiseValue::Other("refuted"),
        "inconclusive" => TransferPremiseValue::Other("inconclusive"),
        "unsupported" => TransferPremiseValue::Other("unsupported"),
        "incomplete" => TransferPremiseValue::Other("incomplete"),
        "failed" => TransferPremiseValue::Other("failed"),
        "tested" => TransferPremiseValue::Other("tested"),
        "declined" => TransferPremiseValue::Other("declined"),
        "measured" => TransferPremiseValue::Other("measured"),
        _ => return Err(RunCause::AnalyzeFault),
    })
}

fn supplied_premise(
    value: &Value,
    expected_identity: Option<&Value>,
    expected_subject: Option<Value>,
    pointer: &str,
) -> Result<TransferPremiseValue> {
    let request = value
        .get("request")
        .ok_or_else(|| invalid(pointer, "premise result is missing its request item"))?;
    let premise = read_premise_item(request).map_err(|error| {
        invalid(
            format!("{pointer}/request/{}", error.pointer),
            error.to_string(),
        )
    })?;
    let identity = premise
        .key
        .to_wire()
        .map_err(|_| invalid(pointer, "premise item key cannot be written"))?;
    if expected_identity.is_some_and(|expected| expected != &identity) {
        return Err(invalid(
            pointer,
            "premise item identity does not match its slot",
        ));
    }
    if let Some(subject) = expected_subject {
        let got = request
            .pointer("/temporal/subject")
            .ok_or_else(|| invalid(pointer, "premise item is missing its subject"))?;
        if got != &subject {
            return Err(invalid(
                pointer,
                "premise item subject does not match the selected subject",
            ));
        }
    }
    let result = value
        .get("result")
        .ok_or_else(|| invalid(pointer, "premise result is missing its result entry"))?;
    let entry = read_result_entry(result, 0, EntryOrigin::FirstParty)
        .map_err(|error| {
            invalid(
                format!("{pointer}/result/{}", error.pointer),
                error.to_string(),
            )
        })?
        .read_from_bytes();
    Ok(premise_value(entry.record.value()))
}

fn invalid(member: impl Into<String>, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: member.into(),
        reason: reason.into(),
    }
}

fn missing_name(member: &str, name: &str) -> RunCause {
    RunCause::TransferSelection {
        member: member.to_owned(),
        cause: "missing-name",
        code: fsl_foundation::Code::MissingDeclaration,
        message: format!("no declaration named {name}"),
    }
}

fn model_mismatch(member: &str, declared: &str, given: &str) -> RunCause {
    RunCause::TransferSelection {
        member: member.to_owned(),
        cause: "invalid-value",
        code: fsl_foundation::Code::InvalidRuntimeInput,
        message: format!("selected model is {given}, but refinement declares {declared}"),
    }
}

fn type_mismatch(member: &str, kind: &str, profile: &str) -> RunCause {
    RunCause::TransferSelection {
        member: member.to_owned(),
        cause: "type-mismatch",
        code: fsl_foundation::Code::IllTyped,
        message: format!("{kind} declaration has profile {profile}"),
    }
}
