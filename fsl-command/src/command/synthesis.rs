// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-754: bounded candidate enumeration for value synthesis.

use filament_exact::{Integer, IntegerInterval, ValueType};
use fsl_foundation::source::SourceLabel;
use fsl_semantics::check::SCALAR_LIMITS_UNLIMITED;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

const DEFAULT_MAX_CANDIDATES: u64 = 10_000_000;
const DEFAULT_MAX_WORK: u64 = 100_000_000;

/// The two independent limits applied while enumerating a bounded integer
/// domain. Each candidate consumes one unit from each counter.
#[derive(Clone, Copy, Debug)]
pub(super) struct SearchLimits {
    pub(super) max_candidates: Option<u64>,
    pub(super) max_work: Option<u64>,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            max_candidates: Some(DEFAULT_MAX_CANDIDATES),
            max_work: Some(DEFAULT_MAX_WORK),
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
            };
        }
        if limits.max_work.is_some_and(|maximum| visited >= maximum) {
            return SearchResult {
                disposition: SearchDisposition::Incomplete {
                    cause: "synthesis.max_work",
                    value: limits.max_work.expect("limit was checked"),
                },
                visited,
            };
        }

        visited = visited.saturating_add(1);
        if satisfies(&candidate) {
            return SearchResult {
                disposition: SearchDisposition::Candidate(candidate),
                visited,
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
        result.visited,
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
            match validate(&value) {
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
            let setting = cause;
            let limit = setting
                .strip_prefix("synthesis.")
                .expect("a synthesis search limit");
            document.status = "incomplete".to_owned();
            document = document.with_cause(serde_json::json!({
                "kind": "limit-reached",
                "limit": limit,
                "value": value,
                "setting": setting,
            }));
            22
        }
    };
    (document, exit)
}

fn integer_value(value: &Integer) -> Value {
    serde_json::json!({"type": "integer", "value": value.to_string()})
}

#[derive(Clone, Debug)]
struct Selection {
    declaration: Option<String>,
    limits: SearchLimits,
}

/// Run the native-run/1 synthesis selection when present.
pub(super) fn run_bytes(
    directory: &Path,
    bytes: &[u8],
    run_limits: super::RunLimits,
    _overrides: &super::RequestOverrides<'_>,
) -> super::Result<Option<super::RunResult>> {
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
    let selection = read_selection(member)?;
    let request_bytes = serde_json::to_vec(&document).map_err(super::RunCause::Output)?;
    let request: super::wire::Request = super::request(&request_bytes)?;
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
        fsl_package::V2ReadLimits::default(),
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
                && synthesis.span().start == claim.declaration_region.start
                && synthesis.span().end == claim.declaration_region.end
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
    let mut evaluation_error = None;
    let (mut outcome, exit_code) = search_integer_outcome(
        &package_name,
        claim.name.as_ref(),
        node,
        domain,
        selection.limits,
        |candidate| match evaluate_integer_body(checked_package, checked.satisfies(), candidate) {
            Ok(Value::Boolean(value)) => value,
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
                Ok(())
            } else {
                Err(serde_json::json!({"kind": "candidate-outside-domain"}))
            }
        },
    );
    if let Some(error) = evaluation_error {
        return Err(refused("/request/synthesis", error));
    }
    if exit_code == 0 {
        if let Some(objective) = checked.objective() {
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
            let value = evaluate_integer_body(checked_package, objective, &candidate)
                .map_err(|error| refused("/request/synthesis", error))?;
            outcome.optimal = Some(checked.objective_is_candidate());
            outcome = outcome.with_objective(serde_json::json!({
                "expression": checked.objective_source().unwrap_or(""),
                "value": value,
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
        .find(|key| !["declaration", "limits"].contains(&key.as_str()))
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
        limits.max_candidates = read_limit(object, "max_candidates")?;
        limits.max_work = read_limit(object, "max_work")?;
    }
    Ok(Selection {
        declaration,
        limits,
    })
}

fn read_limit(object: &serde_json::Map<String, Value>, name: &str) -> super::Result<Option<u64>> {
    match object.get(name) {
        None => Ok(None),
        Some(value) => value.as_u64().map(Some).ok_or_else(|| {
            refused(
                &format!("/request/synthesis/limits/{name}"),
                "expected a non-negative integer",
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
    let synthesis_rows: Vec<_> = rows
        .iter()
        .filter(|row| {
            graph.nodes.iter().any(|node| {
                node.node_id == row.declaration
                    && node.node_tag.as_ref() == "claim"
                    && node.semantic_form.as_ref() == "synthesis_request"
            })
        })
        .collect();
    let selected = match requested {
        Some(name) => {
            let path: Vec<_> = name.split("::").collect();
            if path.iter().any(|segment| segment.is_empty()) {
                return Err(refused("/request/synthesis/declaration", "missing-name"));
            }
            let matches: Vec<_> = synthesis_rows
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
            match matches.as_slice() {
                [row] => *row,
                [] => return Err(refused("/request/synthesis/declaration", "missing-name")),
                _ => return Err(refused("/request/synthesis/declaration", "ambiguous-name")),
            }
        }
        None => match synthesis_rows.as_slice() {
            [row] => *row,
            [] => return Err(refused("/request/synthesis/declaration", "missing-member")),
            _ => return Err(refused("/request/synthesis/declaration", "missing-member")),
        },
    };
    Ok(selected)
}

fn evaluate_integer_body(
    package: &fsl_package::CheckedPackage,
    body: &fsl_semantics::check::CheckedBody,
    candidate: &Integer,
) -> Result<filament_exact::Value, String> {
    let mut meter = filament_exact::Meter::new(SCALAR_LIMITS_UNLIMITED);
    let evaluation = fsl_eval::value::expression::evaluate_protocol_slot(
        package,
        body,
        vec![Some(filament_exact::Value::Integer(candidate.clone()))],
        &fsl_semantics::model::object_environment::ObjectEnvironment::default(),
        &mut meter,
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
        DEFAULT_MAX_WORK, SearchDisposition, SearchLimits, SynthesisOutcomeDocument,
        search_integer_domain,
    };
    use filament_exact::{Integer, IntegerInterval};
    use ix_trace_rs::trace;
    use serde_json::Value;

    fn digit_domain() -> IntegerInterval {
        IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).expect("digit range")
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
        assert_eq!(candidate["explored"]["work"], 8);
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
