// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-754: bounded candidate enumeration for value synthesis.

use filament_exact::{Integer, IntegerInterval, ValueType};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, Instant};

const DEFAULT_MAX_CANDIDATES: u64 = 10_000_000;
const DEFAULT_MAX_WORK: u64 = 100_000_000;

/// The two independent limits applied while enumerating a bounded integer
/// domain. Each candidate consumes one unit from each counter.
#[derive(Clone, Copy, Debug)]
pub(super) struct SearchLimits {
    pub(super) max_candidates: Option<u64>,
    pub(super) max_work: Option<u64>,
    deadline: Option<Instant>,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            max_candidates: Some(DEFAULT_MAX_CANDIDATES),
            max_work: Some(DEFAULT_MAX_WORK),
            deadline: None,
        }
    }
}

/// The disposition of a bounded integer search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum SearchDisposition {
    /// The first candidate accepted by the checked satisfaction expression.
    Candidate(Integer),
    /// Every value in the finite domain was visited without a match.
    NoCandidate,
    /// A request limit stopped the search before its finite domain was done.
    Incomplete { cause: &'static str, value: u64 },
}

/// One bounded integer search result, including the number of visited values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SearchResult {
    pub(super) disposition: SearchDisposition,
    pub(super) visited: u64,
    pub(super) work: u64,
}

/// The synthesis outcome's always-present `explored` object (FSpec FR-823).
#[derive(Clone, Debug, Serialize)]
struct Explored {
    visited: u64,
    work: u64,
}

/// The synthesis outcome's always-present `limits` object (FSpec FR-823).
#[derive(Clone, Debug, Serialize)]
struct OutcomeLimits {
    #[serde(rename = "synthesis.max_candidates")]
    max_candidates: u64,
    #[serde(rename = "synthesis.max_work")]
    max_work: u64,
}

/// A lineage entry identifying one candidate and both in-process engines.
#[derive(Clone, Debug, Serialize)]
struct Lineage {
    node: Value,
    index: u64,
    symmetry: String,
    synthesizer: &'static str,
    checker: &'static str,
}

/// The strict value-synthesis outcome document. Optional members serialize as
/// `null`, so each status keeps FR-823's one closed key set.
#[derive(Clone, Debug, Serialize)]
pub(super) struct SynthesisOutcomeDocument {
    format: &'static str,
    package: String,
    declaration: String,
    status: String,
    check: Option<String>,
    candidate: Option<Value>,
    objective: Option<Value>,
    optimal: Option<bool>,
    explored: Explored,
    lineage: Option<Lineage>,
    counterexample: Option<Value>,
    limits: OutcomeLimits,
    cause: Option<Value>,
}

impl SynthesisOutcomeDocument {
    /// Build the common outcome envelope, including the complete limit set.
    pub(super) fn new(
        package: impl Into<String>,
        declaration: impl Into<String>,
        status: impl Into<String>,
        visited: u64,
        work: u64,
        max_candidates: u64,
        max_work: u64,
    ) -> Self {
        Self {
            format: "filament.synthesis-outcome/v1",
            package: package.into(),
            declaration: declaration.into(),
            status: status.into(),
            check: None,
            candidate: None,
            objective: None,
            optimal: None,
            explored: Explored { visited, work },
            lineage: None,
            counterexample: None,
            limits: OutcomeLimits {
                max_candidates,
                max_work,
            },
            cause: None,
        }
    }

    /// Attach the independent check result and its typed candidate value.
    #[cfg(test)]
    pub(super) fn with_candidate(
        mut self,
        value: Value,
        check: impl Into<String>,
        optimal: Option<bool>,
    ) -> Self {
        self.candidate = Some(value);
        self.check = Some(check.into());
        self.optimal = optimal;
        self
    }

    /// Attach the expression and evaluated value for an authored objective.
    pub(super) fn with_objective(mut self, objective: Value) -> Self {
        self.objective = Some(objective);
        self
    }

    /// Attach the source node and deterministic candidate lineage.
    #[cfg(test)]
    pub(super) fn with_lineage(
        mut self,
        node: Value,
        index: u64,
        symmetry: impl Into<String>,
    ) -> Self {
        self.lineage = Some(Lineage {
            node,
            index,
            symmetry: symmetry.into(),
            synthesizer: "filament.synthesis.enumerate/v1",
            checker: "filament.native.value-validity/v1",
        });
        self
    }

    /// Attach a checker's replayable counterexample.
    pub(super) fn with_counterexample(mut self, value: Value) -> Self {
        self.counterexample = Some(value);
        self
    }

    /// Attach a limit, cancellation, or engine cause.
    pub(super) fn with_cause(mut self, value: Value) -> Self {
        self.cause = Some(value);
        self
    }

    /// Serialize the closed document to its JSON value.
    pub(super) fn into_value(self) -> Value {
        serde_json::to_value(self).expect("the synthesis outcome is serializable")
    }
}

/// Enumerate a closed integer interval in ascending value order and evaluate
/// each value once. The evaluator is the caller's native checked-expression
/// path; limits stop before the next candidate is evaluated.
pub(super) fn search_integer_domain(
    domain: &IntegerInterval,
    limits: SearchLimits,
    mut satisfies: impl FnMut(&Integer) -> bool,
) -> SearchResult {
    let lower = domain.lower().clone();
    let upper = domain.upper().clone();
    let mut candidate = lower;
    let mut visited = 0_u64;

    loop {
        if candidate > upper {
            return SearchResult {
                disposition: SearchDisposition::NoCandidate,
                visited,
                work: visited,
            };
        }
        if limits
            .max_candidates
            .is_some_and(|maximum| visited >= maximum)
        {
            return SearchResult {
                disposition: SearchDisposition::Incomplete {
                    cause: "synthesis.max_candidates",
                    value: limits.max_candidates.expect("limit was checked"),
                },
                visited,
                work: visited,
            };
        }
        if limits.max_work.is_some_and(|maximum| visited >= maximum) {
            return SearchResult {
                disposition: SearchDisposition::Incomplete {
                    cause: "synthesis.max_work",
                    value: limits.max_work.expect("limit was checked"),
                },
                visited,
                work: visited,
            };
        }
        if limits
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return SearchResult {
                disposition: SearchDisposition::Incomplete {
                    cause: "deadline_ms",
                    value: 0,
                },
                visited,
                work: visited,
            };
        }

        visited = visited.saturating_add(1);
        let accepted = satisfies(&candidate);
        if limits
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return SearchResult {
                disposition: SearchDisposition::Incomplete {
                    cause: "deadline_ms",
                    value: 0,
                },
                visited,
                work: visited,
            };
        }
        if accepted {
            return SearchResult {
                disposition: SearchDisposition::Candidate(candidate),
                visited,
                work: visited,
            };
        }
        candidate = candidate.add(&Integer::one());
    }
}

/// Run the integer candidate loop and encode its FR-823 result without
/// coupling the search helper to request selection or package admission.
pub(super) fn search_integer_outcome(
    package: &str,
    declaration: &str,
    node: Value,
    domain: &IntegerInterval,
    limits: SearchLimits,
    mut satisfies: impl FnMut(&Integer) -> bool,
    mut validate: impl FnMut(&Integer) -> Result<(), Value>,
) -> (SynthesisOutcomeDocument, u8) {
    let max_candidates = limits.max_candidates.unwrap_or(DEFAULT_MAX_CANDIDATES);
    let max_work = limits.max_work.unwrap_or(DEFAULT_MAX_WORK);
    let result = search_integer_domain(domain, limits, &mut satisfies);
    let mut document = SynthesisOutcomeDocument::new(
        package,
        declaration,
        "",
        result.visited,
        result.work,
        max_candidates,
        max_work,
    );
    let exit = match result.disposition {
        SearchDisposition::Candidate(value) => {
            document.status = "candidate".to_owned();
            document.candidate = Some(integer_value(&value));
            document.lineage = Some(Lineage {
                node,
                index: result.visited.saturating_sub(1),
                symmetry: "identity".to_owned(),
                synthesizer: "filament.synthesis.enumerate/v1",
                checker: "filament.native.value-validity/v1",
            });
            if let Some(maximum) = limits.max_work.filter(|maximum| result.work >= *maximum) {
                document.status = "incomplete".to_owned();
                document.check = Some("unchecked".to_owned());
                document.optimal = Some(false);
                document = document.with_cause(serde_json::json!({
                    "kind": "limit-reached",
                    "limit": "max_work",
                    "value": maximum,
                    "setting": "synthesis.max_work"
                }));
                return (document, 22);
            }
            if limits
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                document.status = "incomplete".to_owned();
                document.check = Some("unchecked".to_owned());
                document.optimal = Some(false);
                document = document.with_cause(serde_json::json!({"kind": "deadline"}));
                return (document, 22);
            }
            let validation = validate(&value);
            document.explored.work = document.explored.work.saturating_add(1);
            if limits
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                document.status = "incomplete".to_owned();
                document.check = Some("unchecked".to_owned());
                document.optimal = Some(false);
                document = document.with_cause(serde_json::json!({"kind": "deadline"}));
                return (document, 22);
            }
            match validation {
                Ok(()) => {
                    document.check = Some("validated".to_owned());
                    0
                }
                Err(counterexample) => {
                    document.check = Some("rejected".to_owned());
                    document = document.with_counterexample(counterexample);
                    30
                }
            }
        }
        SearchDisposition::NoCandidate => {
            document.status = "no-candidate".to_owned();
            10
        }
        SearchDisposition::Incomplete { cause, value } => {
            document.status = "incomplete".to_owned();
            document = if cause == "deadline_ms" {
                document.with_cause(serde_json::json!({"kind": "deadline"}))
            } else {
                let setting = cause;
                let limit = setting
                    .strip_prefix("synthesis.")
                    .expect("a synthesis search limit");
                document.with_cause(serde_json::json!({
                    "kind": "limit-reached",
                    "limit": limit,
                    "value": value,
                    "setting": setting,
                }))
            };
            22
        }
    };
    (document, exit)
}

fn integer_value(value: &Integer) -> Value {
    serde_json::json!({"type": "integer", "value": value.to_string()})
}

struct Selection {
    declaration: Option<String>,
    limits: SearchLimits,
    accounting: super::wire::Accounting,
    deadline_ms: Option<u64>,
}

/// Run the native-run/1 synthesis selection when present.
pub(super) fn run_bytes(
    directory: &Path,
    bytes: &[u8],
    run_limits: super::RunLimits,
    _overrides: &super::RequestOverrides<'_>,
) -> super::Result<Option<super::RunResult>> {
    let started = Instant::now();
    let mut document: Value = serde_json::from_slice(bytes).map_err(super::RunCause::Json)?;
    if document.get("format").and_then(Value::as_str)
        != Some(fsl_foundation::wire_format::WireFormat::RunRequest.as_str())
    {
        return Ok(None);
    }
    let Some(body) = document.get_mut("request").and_then(Value::as_object_mut) else {
        return Ok(None);
    };
    let Some(member) = body.remove("synthesis") else {
        return Ok(None);
    };
    let mut selection = read_selection(member)?;
    let request_bytes = serde_json::to_vec(&document).map_err(super::RunCause::Output)?;
    let request: super::wire::Request = super::request(&request_bytes)?;
    selection.limits.deadline = selection
        .deadline_ms
        .and_then(|milliseconds| started.checked_add(Duration::from_millis(milliseconds)));
    if request.package.is_some() {
        return Err(refused("/request/synthesis", "requires a source program"));
    }
    if request.call.is_some() || request.clause.is_some() || request.items.is_some() {
        return Err(refused(
            "/request/synthesis",
            "is mutually exclusive with call, clause and items",
        ));
    }
    if request.integer_division.is_some()
        || request.libraries.iter().any(|library| {
            library.integer_division.is_some() || library.refinement_internals.is_some()
        })
    {
        return Err(refused(
            "/request/synthesis",
            "this synthesis runner does not admit caller-selected arithmetic laws or refinement internals",
        ));
    }
    if !request.extensions.is_empty() {
        return Err(refused(
            "/request/extensions",
            "synthesis extension inputs are not supported",
        ));
    }
    #[cfg(feature = "quire-extraction")]
    if request
        .program
        .as_ref()
        .is_some_and(|program| program.extraction.is_some())
    {
        return Err(refused(
            "/request/program/extraction",
            "synthesis requires a source program",
        ));
    }
    let program = super::required_program(&request)?;
    let counts = super::FileCounts {
        programs: request.libraries.len().saturating_add(1),
        models: request.models.len(),
        observations: 0,
        extensions: 0,
    };
    let mut intake = super::intake_with_bytes(directory, counts, run_limits.dependent_bytes)?;
    let source = intake.source(&program.source, "/request/program/source")?;
    super::require_complete_edition(source.source())?;
    let model_sources = request
        .models
        .iter()
        .map(|model| intake.source(&model.source, "/request/models"))
        .collect::<super::Result<Vec<_>>>()?;
    let library_sources = request
        .libraries
        .iter()
        .map(|library| intake.source(&library.source, "/request/libraries"))
        .collect::<super::Result<Vec<_>>>()?;
    let models = model_sources
        .iter()
        .map(|model| model.source().text().as_bytes().to_vec())
        .collect();
    let libraries = request
        .libraries
        .iter()
        .zip(&library_sources)
        .map(|(library, source)| {
            crate::emit_checked_v2::supplied_library(
                library.identity.clone(),
                &directory.join(&library.source.file),
                source.source().text().as_bytes().to_vec(),
            )
        })
        .collect();
    let package_name = super::package_name(source.source().identity());
    let compiled = crate::emit_checked_v2::compile_inputs(crate::emit_checked_v2::Inputs {
        source_path: directory.join(&program.source.file),
        source_identity: Some(source.source().identity().clone()),
        source: source.source().text().as_bytes().to_vec(),
        models,
        libraries,
        package_name: package_name.clone(),
        points: Default::default(),
        temporal_selections: Vec::new(),
    })
    .map_err(|error| refused("/request/synthesis", error.to_string()))?;
    let mut model_documents = std::collections::BTreeMap::new();
    for (selection, model) in request.models.iter().zip(&model_sources) {
        model_documents.insert(
            selection.source.identity.identity.clone(),
            model.source().text().as_bytes().to_vec(),
        );
    }
    let admitted = fsl_package::admit_checked_package(
        compiled.emitted.package().bytes(),
        &model_documents,
        fsl_package::V2ReadLimits::bounded(fsl_replay::spine::I2_NATIVE_JOIN_STORAGE_DEFAULT),
    )
    .map_err(|failure| {
        refused(
            "/request/synthesis",
            format!("checked package admission failed: {failure:?}"),
        )
    })?;
    let claim = select_claim(&admitted, selection.declaration.as_deref())?;
    let graph = compiled.checked.package().graph();
    let checked = graph
        .syntheses()
        .iter()
        .find(|synthesis| {
            synthesis.name() == claim.name.as_ref()
                && u64::try_from(synthesis.span().start).ok()
                    == Some(claim.declaration_region.start)
                && u64::try_from(synthesis.span().end).ok() == Some(claim.declaration_region.end)
        })
        .ok_or_else(|| refused("/request/synthesis/declaration", "missing-name"))?;
    let ValueType::Int(domain) = checked.domain() else {
        return Err(refused(
            "/request/synthesis/declaration",
            "the value synthesis runner currently requires a finite integer domain",
        ));
    };
    if checked.slot_names().len() != 1 {
        return Err(refused(
            "/request/synthesis/declaration",
            "the checked synthesis candidate has no single evaluation slot",
        ));
    }
    let node = serde_json::to_value(&claim.declaration).map_err(super::RunCause::Output)?;
    let checked_package = compiled.checked.package();
    let mut meter = std::cell::RefCell::new(filament_exact::Meter::new(super::call_accounting(
        &selection.accounting,
    )));
    let mut evaluation_error = None;
    let mut validation_error = None;
    let (mut outcome, exit_code) = search_integer_outcome(
        &package_name,
        claim.name.as_ref(),
        node,
        domain,
        selection.limits,
        |candidate| match evaluate_integer_body(
            checked_package,
            checked.satisfies(),
            candidate,
            &mut meter.borrow_mut(),
        ) {
            Ok(filament_exact::Value::Boolean(value)) => value,
            Ok(value) => {
                evaluation_error = Some(format!("satisfies expression returned {value:?}"));
                false
            }
            Err(error) => {
                evaluation_error = Some(error);
                false
            }
        },
        |candidate| {
            if candidate >= domain.lower() && candidate <= domain.upper() {
                match evaluate_integer_body(
                    checked_package,
                    checked.satisfies(),
                    candidate,
                    &mut meter.borrow_mut(),
                ) {
                    Ok(filament_exact::Value::Boolean(true)) => Ok(()),
                    Ok(filament_exact::Value::Boolean(false)) => {
                        Err(serde_json::json!({"kind": "candidate-rejected"}))
                    }
                    Ok(value) => {
                        validation_error = Some(format!(
                            "satisfies expression returned {value:?} during candidate validation"
                        ));
                        Err(serde_json::json!({"kind": "candidate-validation-failed"}))
                    }
                    Err(error) => {
                        validation_error = Some(error);
                        Err(serde_json::json!({"kind": "candidate-validation-failed"}))
                    }
                }
            } else {
                Err(serde_json::json!({"kind": "candidate-outside-domain"}))
            }
        },
    );
    if let Some(error) = evaluation_error {
        return Err(refused("/request/synthesis", error));
    }
    if let Some(error) = validation_error {
        return Err(refused("/request/synthesis", error));
    }
    if exit_code == 0 {
        if let Some(objective) = checked.objective() {
            let work_exhausted = selection
                .limits
                .max_work
                .filter(|maximum| outcome.explored.work >= *maximum);
            let deadline_expired = selection
                .limits
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline);
            if work_exhausted.is_some() || deadline_expired {
                outcome.status = "incomplete".to_owned();
                outcome.optimal = Some(false);
                outcome.cause = Some(match work_exhausted {
                    Some(maximum) => serde_json::json!({
                        "kind": "limit-reached",
                        "limit": "max_work",
                        "value": maximum,
                        "setting": "synthesis.max_work"
                    }),
                    None => serde_json::json!({"kind": "deadline"}),
                });
                return Ok(Some(super::RunResult {
                    exit_code: 22,
                    value: super::output::NativeResult::synthesis_document(serde_json::json!({
                        "synthesis": outcome.into_value()
                    })),
                    outcome: None,
                    requirements: None,
                    sources: crate::Sources::from_sources(intake.sources.iter().cloned()),
                }));
            }
            let candidate = outcome
                .candidate
                .as_ref()
                .and_then(|value| value.get("value"))
                .and_then(Value::as_str)
                .and_then(|value| value.parse::<Integer>().ok())
                .ok_or_else(|| {
                    refused(
                        "/request/synthesis",
                        "objective candidate is not an integer",
                    )
                })?;
            let value =
                evaluate_integer_body(checked_package, objective, &candidate, meter.get_mut())
                    .map_err(|error| refused("/request/synthesis", error))?;
            outcome.explored.work = outcome.explored.work.saturating_add(1);
            outcome.optimal = Some(checked.objective_is_candidate());
            let filament_exact::Value::Integer(value) = value else {
                return Err(refused(
                    "/request/synthesis/objective",
                    "integer synthesis objective returned a non-integer",
                ));
            };
            outcome = outcome.with_objective(serde_json::json!({
                "direction": checked.objective_direction(),
                "expression": checked.objective_source().unwrap_or(""),
                "value": integer_value(&value),
            }));
        }
    }
    let sources = crate::Sources::from_sources(intake.sources.iter().cloned());
    Ok(Some(super::RunResult {
        exit_code,
        value: super::output::NativeResult::synthesis_document(serde_json::json!({
            "synthesis": outcome.into_value()
        })),
        outcome: None,
        requirements: None,
        sources,
    }))
}

fn read_selection(member: Value) -> super::Result<Selection> {
    let Some(object) = member.as_object() else {
        return Err(refused("/request/synthesis", "expected an object"));
    };
    if let Some(key) = object
        .keys()
        .find(|key| !["declaration", "limits", "accounting", "deadline_ms"].contains(&key.as_str()))
    {
        return Err(refused(
            &format!("/request/synthesis/{key}"),
            "unknown-member",
        ));
    }
    let declaration = match object.get("declaration") {
        None => None,
        Some(Value::String(name)) if !name.is_empty() => Some(name.clone()),
        Some(_) => {
            return Err(refused(
                "/request/synthesis/declaration",
                "expected a non-empty name",
            ));
        }
    };
    let mut limits = SearchLimits::default();
    if let Some(value) = object.get("limits") {
        let Some(object) = value.as_object() else {
            return Err(refused("/request/synthesis/limits", "expected an object"));
        };
        if let Some(key) = object
            .keys()
            .find(|key| !["max_candidates", "max_work"].contains(&key.as_str()))
        {
            return Err(refused(
                &format!("/request/synthesis/limits/{key}"),
                "unknown-member",
            ));
        }
        limits.max_candidates = Some(
            read_limit(object, "max_candidates", DEFAULT_MAX_CANDIDATES)?
                .unwrap_or(DEFAULT_MAX_CANDIDATES),
        );
        limits.max_work =
            Some(read_limit(object, "max_work", DEFAULT_MAX_WORK)?.unwrap_or(DEFAULT_MAX_WORK));
    }
    let accounting = object
        .get("accounting")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| {
            refused(
                "/request/synthesis/accounting",
                format!("invalid accounting object: {error}"),
            )
        })?
        .unwrap_or_default();
    let deadline_ms = match object.get("deadline_ms") {
        None => None,
        Some(value) => value
            .as_u64()
            .filter(|value| *value <= 9_007_199_254_740_991)
            .map(Some)
            .ok_or_else(|| {
                refused(
                    "/request/synthesis/deadline_ms",
                    "expected an integer from 0 to 9007199254740991",
                )
            })?,
    };
    Ok(Selection {
        declaration,
        limits,
        accounting,
        deadline_ms,
    })
}

fn read_limit(
    object: &serde_json::Map<String, Value>,
    name: &str,
    default: u64,
) -> super::Result<Option<u64>> {
    match object.get(name) {
        None => Ok(Some(default)),
        Some(value) => value
            .as_u64()
            .filter(|value| *value > 0)
            .map(Some)
            .ok_or_else(|| {
                refused(
                    &format!("/request/synthesis/limits/{name}"),
                    "expected a positive integer",
                )
            }),
    }
}

fn select_claim<'p>(
    package: &'p filament_contract_model::CheckedPackageV2,
    requested: Option<&str>,
) -> super::Result<&'p filament_contract_model::ClaimNameRow> {
    let rows = package.claim_names().unwrap_or_default();
    let graph = package.resolved_graph();
    let declarations: std::collections::BTreeSet<_> = graph
        .nodes
        .iter()
        .filter(|node| {
            node.node_tag.as_ref() == "claim" && node.semantic_form.as_ref() == "synthesis_request"
        })
        .map(|node| &node.node_id)
        .collect();
    select_claim_rows(
        rows.iter()
            .filter(|row| declarations.contains(&row.declaration)),
        requested,
    )
}

fn select_claim_rows<'p>(
    rows: impl Iterator<Item = &'p filament_contract_model::ClaimNameRow>,
    requested: Option<&str>,
) -> super::Result<&'p filament_contract_model::ClaimNameRow> {
    let rows: Vec<_> = rows.collect();
    let unique_declarations = |matching: &[&'p filament_contract_model::ClaimNameRow]| {
        let mut declarations = Vec::new();
        for row in matching {
            if !declarations
                .iter()
                .any(|known: &&filament_contract_model::ClaimNameRow| {
                    known.declaration == row.declaration
                })
            {
                declarations.push(*row);
            }
        }
        declarations
    };
    let selected = match requested {
        Some(name) => {
            let path: Vec<_> = name.split("::").collect();
            if path.iter().any(|segment| segment.is_empty()) {
                return Err(refused("/request/synthesis/declaration", "missing-name"));
            }
            let matching: Vec<_> = rows
                .iter()
                .copied()
                .filter(|row| {
                    if path.len() == 1 {
                        row.name.as_ref() == name
                    } else {
                        row.qualified_name
                            .iter()
                            .map(|segment| segment.as_ref())
                            .eq(path.iter().copied())
                    }
                })
                .collect();
            match unique_declarations(&matching).as_slice() {
                [row] => *row,
                [] => return Err(refused("/request/synthesis/declaration", "missing-name")),
                _ => return Err(refused("/request/synthesis/declaration", "ambiguous-name")),
            }
        }
        None => match unique_declarations(&rows).as_slice() {
            [row] => *row,
            [] => return Err(refused("/request/synthesis/declaration", "missing-name")),
            _ => return Err(refused("/request/synthesis/declaration", "missing-member")),
        },
    };
    Ok(selected)
}

fn evaluate_integer_body(
    package: &fsl_package::CheckedPackage,
    body: &fsl_semantics::check::CheckedBody,
    candidate: &Integer,
    meter: &mut filament_exact::Meter,
) -> Result<filament_exact::Value, String> {
    let evaluation = fsl_eval::value::evaluate_protocol_slot(
        package,
        body,
        vec![Some(filament_exact::Value::Integer(candidate.clone()))],
        &fsl_semantics::model::object_environment::ObjectEnvironment::default(),
        meter,
    )
    .map_err(|fault| format!("checked synthesis expression fault: {fault:?}"))?;
    match evaluation.outcome {
        filament_eval::FamilyOutcome::Evaluated(filament_exact::Outcome::Completed(value)) => {
            Ok(value)
        }
        outcome => Err(format!(
            "checked synthesis expression did not complete: {outcome:?}"
        )),
    }
}

fn refused(member: &str, reason: impl Into<String>) -> super::RunCause {
    super::RunCause::ClauseRequest {
        member: "synthesis".to_owned(),
        reason: format!("{}: {}", member, reason.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        search_integer_domain, select_claim_rows, SearchDisposition, SearchLimits,
        SynthesisOutcomeDocument, DEFAULT_MAX_WORK,
    };
    use filament_exact::{Integer, IntegerInterval};
    use ix_trace_rs::trace;
    use serde_json::Value;

    fn digit_domain() -> IntegerInterval {
        IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).expect("digit range")
    }

    fn claim_row(scope: &str) -> filament_contract_model::ClaimNameRow {
        serde_json::from_value(serde_json::json!({
            "declaration": {
                "package": "pkg",
                "path": [scope, "Pick"],
                "ordinal": 0
            },
            "occurrence": {"role": "declaration", "ordinal": 0},
            "name": "Pick",
            "qualified_name": [scope, "Pick"],
            "declaration_region": {
                "source": {"authority": "test", "identity": scope},
                "start": 0,
                "end": 4
            }
        }))
        .expect("a valid claim-name row")
    }

    /// FSpec FR-823-AC-3: a bare name matching two declarations is
    /// ambiguous; a fully qualified name selects exactly one row.
    #[test]
    #[trace("TC-3017", "FR-1037-AC-2")]
    #[trace("FSpec-TC-2038", "FSpec-FR-823-AC-3")]
    fn claim_selection_refuses_ambiguous_bare_names_and_accepts_qualified_names() {
        let a = claim_row("A");
        let b = claim_row("B");
        let ambiguous = select_claim_rows([&a, &b].into_iter(), Some("Pick"))
            .expect_err("two matching declarations are ambiguous");
        assert!(format!("{ambiguous:?}").contains("ambiguous-name"));

        let selected = select_claim_rows([&a, &b].into_iter(), Some("A::Pick"))
            .expect("qualified name selects one declaration");
        assert_eq!(
            selected
                .qualified_name
                .iter()
                .map(|segment| segment.as_ref())
                .collect::<Vec<_>>(),
            ["A", "Pick"]
        );
    }

    #[test]
    #[trace("TC-2104", "FR-754-AC-3")]
    fn visits_values_through_the_first_satisfying_integer() {
        let result = search_integer_domain(&digit_domain(), SearchLimits::default(), |value| {
            value == &Integer::from(7_i64)
        });
        assert_eq!(
            result.disposition,
            SearchDisposition::Candidate(Integer::from(7_i64))
        );
        assert_eq!(result.visited, 8);
    }

    #[test]
    #[trace("TC-2104", "FR-754-AC-3")]
    fn exhausts_a_finite_integer_domain_before_reporting_no_candidate() {
        let result = search_integer_domain(&digit_domain(), SearchLimits::default(), |_| false);
        assert_eq!(result.disposition, SearchDisposition::NoCandidate);
        assert_eq!(result.visited, 10);
    }

    #[test]
    #[trace("TC-2104", "FR-754-AC-3")]
    fn request_limits_stop_before_evaluating_the_next_value() {
        let result = search_integer_domain(
            &digit_domain(),
            SearchLimits {
                max_candidates: Some(3),
                max_work: None,
                deadline: None,
            },
            |_| false,
        );
        assert_eq!(
            result.disposition,
            SearchDisposition::Incomplete {
                cause: "synthesis.max_candidates",
                value: 3
            }
        );
        assert_eq!(result.visited, 3);
    }

    /// FR-823: an elapsed request deadline stops before another candidate is
    /// evaluated and produces an incomplete result.
    #[test]
    #[trace("FSpec-TC-2038", "FSpec-FR-823-AC-2")]
    fn elapsed_deadline_stops_before_the_first_candidate() {
        let result = search_integer_domain(
            &digit_domain(),
            SearchLimits {
                deadline: Some(std::time::Instant::now() - std::time::Duration::from_millis(1)),
                ..SearchLimits::default()
            },
            |_| panic!("no candidate is evaluated after the deadline"),
        );
        assert_eq!(
            result.disposition,
            SearchDisposition::Incomplete {
                cause: "deadline_ms",
                value: 0,
            }
        );
        assert_eq!(result.visited, 0);
        assert_eq!(result.work, 0);
    }

    /// FR-823: an evaluation that runs beyond the request deadline is counted
    /// as consumed work, but cannot produce a candidate result.
    #[test]
    #[trace("FSpec-TC-2038", "FSpec-FR-823-AC-2")]
    fn deadline_expiring_during_evaluation_reports_consumed_work() {
        let result = search_integer_domain(
            &digit_domain(),
            SearchLimits {
                deadline: Some(std::time::Instant::now() + std::time::Duration::from_millis(1)),
                ..SearchLimits::default()
            },
            |_| {
                std::thread::sleep(std::time::Duration::from_millis(10));
                true
            },
        );
        assert_eq!(
            result.disposition,
            SearchDisposition::Incomplete {
                cause: "deadline_ms",
                value: 0,
            }
        );
        assert_eq!(result.visited, 1);
        assert_eq!(result.work, 1);
    }

    #[test]
    #[trace("TC-2104", "FR-754-AC-3")]
    fn search_outcome_carries_typed_candidate_and_limit_cause() {
        let (candidate, exit) = super::search_integer_outcome(
            "pkg",
            "Pick",
            serde_json::json!({"package": "pkg", "path": ["Pick"], "ordinal": 0}),
            &digit_domain(),
            SearchLimits::default(),
            |value| value == &Integer::from(7_i64),
            |_| Ok(()),
        );
        assert_eq!(exit, 0);
        let candidate = candidate.into_value();
        assert_eq!(candidate["status"], "candidate");
        assert_eq!(
            candidate["candidate"],
            serde_json::json!({"type": "integer", "value": "7"})
        );
        assert_eq!(candidate["check"], "validated");
        assert_eq!(candidate["explored"]["work"], 9);
        assert_eq!(candidate["lineage"]["index"], 7);
        assert_eq!(candidate["lineage"]["symmetry"], "identity");

        let (limited, exit) = super::search_integer_outcome(
            "pkg",
            "Pick",
            serde_json::json!({"package": "pkg", "path": ["Pick"], "ordinal": 0}),
            &digit_domain(),
            SearchLimits {
                max_candidates: Some(3),
                max_work: Some(DEFAULT_MAX_WORK),
                deadline: None,
            },
            |_| false,
            |_| Ok(()),
        );
        assert_eq!(exit, 22);
        let limited = limited.into_value();
        assert_eq!(limited["status"], "incomplete");
        assert_eq!(limited["explored"]["visited"], 3);
        assert_eq!(limited["explored"]["work"], 3);
        assert_eq!(
            limited["cause"],
            serde_json::json!({
                "kind": "limit-reached", "limit": "max_candidates", "value": 3,
                "setting": "synthesis.max_candidates"
            })
        );
    }

    /// FR-754-AC-3: native candidate validation consumes one unit of work.
    #[test]
    #[trace("TC-2104", "FR-754-AC-3")]
    fn candidate_is_unchecked_when_work_limit_cannot_cover_validation() {
        let mut validation_called = false;
        let (candidate, exit) = super::search_integer_outcome(
            "pkg",
            "Pick",
            serde_json::json!({"package": "pkg", "path": ["Pick"], "ordinal": 0}),
            &digit_domain(),
            SearchLimits {
                max_candidates: None,
                max_work: Some(8),
                deadline: None,
            },
            |value| value == &Integer::from(7_i64),
            |_| {
                validation_called = true;
                Ok(())
            },
        );
        assert_eq!(exit, 22);
        assert!(!validation_called);
        let candidate = candidate.into_value();
        assert_eq!(candidate["status"], "incomplete");
        assert_eq!(candidate["check"], "unchecked");
        assert_eq!(candidate["explored"]["work"], 8);
    }

    #[test]
    #[trace("TC-3017", "FR-1037-AC-1")]
    #[trace("FSpec-FR-823-AC-1")]
    fn candidate_outcome_serializes_every_fr823_member() {
        let outcome = SynthesisOutcomeDocument::new(
            "pkg",
            "Pick",
            "candidate",
            8,
            8,
            10_000_000,
            100_000_000,
        )
        .with_candidate(
            serde_json::json!({"type": "integer", "value": "7"}),
            "validated",
            None,
        )
        .with_lineage(
            serde_json::json!({"package": "pkg", "path": ["Pick"], "ordinal": 0}),
            7,
            "identity",
        )
        .into_value();
        assert_eq!(
            outcome
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "candidate",
                "cause",
                "check",
                "counterexample",
                "declaration",
                "explored",
                "format",
                "limits",
                "lineage",
                "objective",
                "optimal",
                "package",
                "status"
            ]
        );
        assert_eq!(outcome["format"], "filament.synthesis-outcome/v1");
        assert_eq!(
            outcome["explored"],
            serde_json::json!({"visited": 8, "work": 8})
        );
        assert_eq!(outcome["lineage"]["symmetry"], "identity");
        assert_eq!(outcome["lineage"]["index"], 7);
        assert_eq!(outcome["optimal"], Value::Null);
        assert_eq!(outcome["objective"], Value::Null);
        assert_eq!(outcome["counterexample"], Value::Null);
        assert_eq!(outcome["cause"], Value::Null);
    }
}
