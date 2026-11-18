// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-026: construct typed JSON views of native outcomes and actual provenance.

mod types;

use super::{LimitKind, RunCause, RunError, RunResult};
use crate::formal_source::FormalSource;
use fsl_foundation::source::SourceReadCause;
use fsl_foundation::Diagnostic;
use serde_json::Value;

/// Immutable JSON produced from the typed native result schema.
/// Construction stays inside the output adapter; callers can inspect or serialize it.
#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
pub struct NativeResult(Value);

impl NativeResult {
    /// The FR-661 build inventory emitted by `version`.
    pub(super) fn version_document(document: Value) -> Self {
        Self(document)
    }

    /// The in-process `check` command's native check outcome document.
    pub(super) fn check_document(document: Value) -> Self {
        Self(document)
    }

    /// The FR-621 `analyze` document, written by `fsl-analyze`.
    pub(super) fn analyze_document(document: Value) -> Self {
        Self(document)
    }

    /// Mark generated flow items whose bounded tuple expansion stopped
    /// before analysis. The result remains the ordinary analyze document;
    /// only those item records and their aggregate gaps change.
    pub(super) fn mark_flow_items_incomplete(
        &mut self,
        claim_names: &std::collections::BTreeSet<String>,
    ) {
        let Some(items) = self.0.get_mut("items").and_then(Value::as_array_mut) else {
            return;
        };
        let mut stopped = std::collections::BTreeSet::new();
        for item in items {
            if !item
                .get("claim_name")
                .and_then(Value::as_str)
                .is_some_and(|name| claim_names.contains(name))
            {
                continue;
            }
            let Some(index) = item.get("request_index").and_then(Value::as_u64) else {
                continue;
            };
            stopped.insert(index);
            item["category"] = Value::String("incomplete".to_owned());
            let Some(record) = item.get_mut("record").and_then(Value::as_object_mut) else {
                continue;
            };
            let request_index = record.get("request_index").cloned();
            record.clear();
            if let Some(request_index) = request_index {
                record.insert("request_index".to_owned(), request_index);
            }
            record.insert("result".to_owned(), Value::String("incomplete".to_owned()));
            record.insert(
                "cause".to_owned(),
                serde_json::json!({"kind": "limit-reached"}),
            );
            record.insert(
                "category".to_owned(),
                Value::String("incomplete".to_owned()),
            );
        }
        if stopped.is_empty() {
            return;
        }
        if let Some(gaps) = self
            .0
            .pointer_mut("/aggregate/gaps")
            .and_then(Value::as_array_mut)
        {
            for gap in gaps {
                if gap
                    .get("request_index")
                    .and_then(Value::as_u64)
                    .is_some_and(|i| stopped.contains(&i))
                {
                    gap["result"] = Value::String("incomplete".to_owned());
                }
            }
        }
        self.0["category"] = Value::String("incomplete".to_owned());
    }

    /// The FR-644 `filament.monitor-run/1` document.
    pub(super) fn monitor_document(document: Value) -> Self {
        Self(document)
    }

    /// The FR-931 lowering document, its `tl_lowering` record.
    pub(super) fn lowering_document(document: Value) -> Self {
        Self(document)
    }

    /// The FR-862 replay result: FR-323's `results` disposition.
    pub(super) fn replay_document(document: Value) -> Self {
        Self(document)
    }

    /// The FR-651 result document for several independently settled items.
    pub(super) fn items_document(document: Value) -> Self {
        Self(document)
    }

    /// Inspect the encoded observations without changing the admitted shape.
    pub fn as_value(&self) -> &Value {
        &self.0
    }
}

impl std::fmt::Display for NativeResult {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

fn diagnostic(value: &Diagnostic) -> types::Diagnostic<'_> {
    types::Diagnostic {
        phase: value.phase.as_str(),
        code: value.code.as_str(),
        message: &value.message,
        source: &value.source,
        path: &value.path,
        span: value.span,
        upstream: None,
        identity: value
            .identity_cause()
            .and_then(SourceReadCause::identity_fields),
    }
}

pub(super) fn error(error: &RunError, artifact: bool) -> Result<Value, serde_json::Error> {
    let (stage, details) = match &error.cause {
        RunCause::Io { path, error } => (
            types::Stage::File,
            types::Details::Io {
                path: path.to_string_lossy(),
                os_code: error.raw_os_error(),
            },
        ),
        RunCause::Json(error) => (
            types::Stage::Request,
            types::Details::Json {
                line: error.line(),
                column: error.column(),
            },
        ),
        RunCause::Output(_) => (types::Stage::Output, types::Details::None),
        RunCause::Format => (types::Stage::Envelope, types::Details::None),
        RunCause::Limit(kind) => (
            types::Stage::Intake,
            match kind {
                LimitKind::FileBytes => types::Details::FileBytes {
                    limit: kind.as_str(),
                },
                LimitKind::RequestBytes { maximum } => types::Details::ConfiguredBytes {
                    limit: kind.as_str(),
                    maximum: *maximum,
                    option: "--request-bytes",
                },
                LimitKind::FixedRequestBytes { maximum } => types::Details::FixedBytes {
                    limit: kind.as_str(),
                    maximum: *maximum,
                },
                LimitKind::DependentBytes { maximum } => types::Details::ConfiguredBytes {
                    limit: kind.as_str(),
                    maximum: *maximum,
                    option: "--dependent-bytes",
                },
                LimitKind::FixedDependentBytes { maximum } => types::Details::FixedBytes {
                    limit: kind.as_str(),
                    maximum: *maximum,
                },
                LimitKind::ObservationDocumentBytes { maximum } => {
                    types::Details::ConfiguredBytes {
                        limit: kind.as_str(),
                        maximum: *maximum,
                        option: "observation_limits.document_bytes",
                    }
                }
                LimitKind::SelectedFiles {
                    category,
                    requested,
                    remaining,
                    maximum,
                } => types::Details::FileCount {
                    limit: kind.as_str(),
                    category: category.as_str(),
                    requested: *requested,
                    remaining: *remaining,
                    maximum: *maximum,
                },
            },
        ),
        RunCause::ClauseRequest { .. }
        | RunCause::ClocksGivenTwice
        | RunCause::OutputGivenTwice
        | RunCause::OutputExists { .. } => (types::Stage::Request, types::Details::None),
        RunCause::ExtensionCatalog(_) => (
            types::Stage::Spine(fsl_replay::spine::SpineStage::Forms),
            types::Details::None,
        ),
        RunCause::ClauseOutput(_) => (types::Stage::Output, types::Details::None),
        RunCause::Identifier { diagnostic, .. } => (
            types::Stage::Request,
            types::Details::Identifier(diagnostic),
        ),
        RunCause::Native(error) => (
            types::Stage::Native(error.phase),
            types::Details::Native(diagnostic(error)),
        ),
        RunCause::Source(error) => (
            types::Stage::Native(error.native.phase),
            types::Details::Native(diagnostic(&error.native)),
        ),
        RunCause::CompleteSelection(_)
        | RunCause::Libraries(_)
        | RunCause::CompleteRunSelection(_)
        | RunCause::ExtractionRequiresRun
        | RunCause::EmptyCompileProgram
        | RunCause::AnalyzeRequest { .. }
        | RunCause::AnalyzeUniverse(_)
        | RunCause::AnalyzePackage { .. }
        | RunCause::AnalyzeNotYetImplemented { .. }
        | RunCause::AnalyzeRefused(_)
        | RunCause::MonitorRefused(_)
        | RunCause::TlCorrespondence { .. }
        | RunCause::IntegerDivisionLaw { .. }
        | RunCause::TlClause { .. }
        | RunCause::TlNotYetImplemented { .. } => (types::Stage::Request, types::Details::None),
        RunCause::TlFault(_) => (types::Stage::Output, types::Details::None),
        RunCause::AnalyzeFault | RunCause::MonitorFault | RunCause::MonitorCancelled => {
            (types::Stage::Output, types::Details::None)
        }
        RunCause::ReplayRequest(_) | RunCause::ReplayBytes { .. } => {
            (types::Stage::Request, types::Details::None)
        }
        RunCause::ReplayOutput(_) => (types::Stage::Output, types::Details::None),
        RunCause::Replay(failure) => (
            match &failure.refusal {
                fsl_replay::ReplayRefusal::Recompile(refusal)
                | fsl_replay::ReplayRefusal::RecompileWithProtocolResults { refusal, .. } => {
                    types::Stage::Spine(refusal.stage())
                }
                _ => types::Stage::Request,
            },
            types::Details::None,
        ),
        RunCause::ItemsFault => (types::Stage::Output, types::Details::None),
        RunCause::Spine(failure) => (
            types::Stage::Spine(failure.refusal.stage()),
            match clock_selection_refusal(&failure.refusal) {
                Some(refusal) => types::Details::ClockSelection {
                    cause: refusal.cause.as_str(),
                    clock: &refusal.clock,
                    declaration: refusal.declaration.as_ref().map(|at| at.name.as_str()),
                    profile: refusal.declaration.as_ref().map(|at| at.profile.identity()),
                    source: &failure.source,
                    path: &failure.path,
                    span: failure.span,
                },
                None => types::Details::Spine {
                    source: &failure.source,
                    path: &failure.path,
                    span: failure.span,
                },
            },
        ),
        RunCause::UnknownRequirementClaim { claim } => (
            types::Stage::SpineRun("call"),
            types::Details::Claim { claim },
        ),
        RunCause::TemporalCorrection(refusal) => (
            types::Stage::SpineRun("evaluate"),
            types::Details::Correction {
                cause: refusal.cause,
                member: refusal.member,
                observation: refusal.observation.as_deref(),
            },
        ),
        RunCause::SpineRun(refusal) => (
            // A cancel has no stage (FR-286); the command never cancels its
            // own handle, so the envelope names the request that owns it.
            refusal
                .stage()
                .map_or(types::Stage::Request, types::Stage::SpineRun),
            match refusal.as_ref() {
                // FR-049-AC-2: the object and the attribute the call's entry
                // refused; no member of FR-286's closed `details` vocabulary.
                fsl_replay::spine::RunRefusal::ObjectEnvironment(record) => {
                    types::Details::ObjectEnvironment {
                        object: record.fields.get("object").map(String::as_str),
                        field: record.fields.get("field").map(String::as_str),
                    }
                }
                // FR-286 "Diagnostic details": the one closed object the
                // outcome diagnostic carries too.
                other => other
                    .details()
                    .map_or(types::Details::None, types::Details::Object),
            },
        ),
    };
    let failure = types::Failure {
        format: types::Format::RunResult,
        stage,
        code: error.cause.code().as_str(),
        cause: match &error.cause {
            RunCause::Spine(failure) => failure.refusal.cause(),
            RunCause::Replay(failure) => failure.refusal.cause(),
            RunCause::SpineRun(refusal) => match refusal.as_ref() {
                fsl_replay::spine::RunRefusal::Compile(refusal) => refusal.cause(),
                fsl_replay::spine::RunRefusal::Argument { defect, .. } => Some(defect.cause),
                fsl_replay::spine::RunRefusal::UnsupportedResult { .. }
                | fsl_replay::spine::RunRefusal::UnsupportedParameter { .. } => {
                    Some("unsupported-feature")
                }
                fsl_replay::spine::RunRefusal::Fault(_) => Some("established-invariant-broken"),
                fsl_replay::spine::RunRefusal::Admission { refusal, .. } => match &refusal.cause {
                    fsl_semantics::model::normalize::ModelRefusalCause::ForeignUniverse {
                        ..
                    } => Some("foreign-universe"),
                    fsl_semantics::model::normalize::ModelRefusalCause::ForeignModelSelection {
                        ..
                    } => Some("foreign-model-selection"),
                    fsl_semantics::model::normalize::ModelRefusalCause::ForeignType { .. } => {
                        Some("foreign-type")
                    }
                    _ => None,
                },
                fsl_replay::spine::RunRefusal::DanglingReference { .. } => {
                    Some("absent-target-in-complete-population")
                }
                fsl_replay::spine::RunRefusal::Storage { failure, .. } => {
                    fsl_semantics::value::environment_stage::membership_failure_cause(failure)
                }
                _ => None,
            },
            RunCause::TemporalCorrection(_) => Some("invalid-value"),
            RunCause::TlCorrespondence { .. } | RunCause::IntegerDivisionLaw { .. } => {
                Some("unsupported-selection")
            }
            RunCause::TlClause { .. } => Some("missing-name"),
            RunCause::TlNotYetImplemented { .. } => Some("not-yet-implemented"),
            _ => None,
        },
        message: error.to_string(),
        details,
        basis: "unavailable",
    };
    if artifact {
        serde_json::to_value(types::ArtifactFailure {
            failure,
            status: if error.cause.is_incomplete() {
                types::ArtifactFailureStatus::Incomplete
            } else {
                types::ArtifactFailureStatus::Refused
            },
        })
    } else {
        serde_json::to_value(failure)
    }
}

/// The FR-286 `monitor` document of clause runs, which reach S6a. A run no
/// item can carry (an evaluator fault fails the whole request, FR-283)
/// settles internal-failure with no stage (FR-286 `last_stage`).
pub(super) fn monitor_outcome<'a>(
    runs: impl IntoIterator<Item = (&'a str, &'a fsl_replay::spine::ClauseRunReport)>,
) -> fsl_replay::OutcomeDocument {
    use filament_semantic_value::catalog::Category;
    fsl_replay::OutcomeDocument::from_clause_runs(Some(fsl_replay::OutcomeStage::S6a), runs)
        .unwrap_or_else(|_| {
            fsl_replay::OutcomeDocument::new(
                fsl_replay::Operation::Monitor,
                None,
                Category::InternalFailure,
            )
        })
}

/// FR-267: expose the library's canonical clause report to the CLI adapter.
pub(super) fn clause_report(
    mut report: fsl_replay::spine::ClauseRunReport,
    selection: &fsl_replay::spine::ClauseRunSelection,
) -> super::Result<RunResult> {
    // FR-526: a timestamped run settles its correction relation before any
    // result document is written; a refused relation writes none.
    fsl_replay::spine::settle_run_correction(&mut report, selection)
        .map_err(RunCause::TemporalCorrection)?;
    let report = &report;
    let bytes = fsl_replay::spine::write_clause_run_result_for(report, Some(selection))
        .map_err(RunCause::ClauseOutput)?;
    let outcome = monitor_outcome(std::iter::once((&selection.clause_name()[..], report)));
    let value = serde_json::from_slice(&bytes).map_err(RunCause::Output)?;
    Ok(RunResult {
        // FR-285: a finite-window `pending` exits 0 and `indeterminate` 11.
        exit_code: report.disposition.exit_code(),
        value: NativeResult(value),
        outcome: Some(outcome),
        requirements: None,
        sources: crate::Sources::default(),
    })
}

/// FR-096/FR-100: `Evaluation.location`'s `origin`, rendered by kind.
fn spine_origin(origin: &filament_semantic_value::location::ExpressionOwner) -> types::SpineOrigin {
    match origin {
        filament_semantic_value::location::ExpressionOwner::Body { function, index } => {
            types::SpineOrigin::Body {
                function: function.clone(),
                index: *index,
            }
        }
        filament_semantic_value::location::ExpressionOwner::Measure { function, index } => {
            types::SpineOrigin::Measure {
                function: function.clone(),
                index: *index,
            }
        }
        filament_semantic_value::location::ExpressionOwner::Expression => {
            types::SpineOrigin::Expression
        }
        filament_semantic_value::location::ExpressionOwner::TypeDeclaration { name } => {
            types::SpineOrigin::TypeDeclaration { name: name.clone() }
        }
        filament_semantic_value::location::ExpressionOwner::StateClause { clause, index } => {
            types::SpineOrigin::StateClause {
                clause: clause.clone(),
                index: *index,
            }
        }
        filament_semantic_value::location::ExpressionOwner::ProtocolAttempt {
            protocol,
            attempt,
        } => types::SpineOrigin::ProtocolAttempt {
            protocol: *protocol,
            attempt: *attempt,
        },
        filament_semantic_value::location::ExpressionOwner::ModelOperationClause {
            operation,
            kind,
            position,
        } => types::SpineOrigin::ModelOperationClause {
            operation: operation.clone(),
            clause: match kind {
                filament_semantic_value::location::OperationClauseKind::Requires => "requires",
                filament_semantic_value::location::OperationClauseKind::Ensures => "ensures",
            },
            position: *position,
        },
    }
}

/// FR-096/FR-100: `Evaluation.location`, rendered as `{origin, path}`.
fn spine_location(
    location: &filament_semantic_value::location::ExpressionLocation,
) -> types::SpineLocation {
    types::SpineLocation {
        origin: spine_origin(&location.origin),
        path: location.path(),
    }
}

/// FR-096/FR-100: a record's resolved locus, rendered as
/// `{file, span}`.
fn spine_locus(locus: fsl_replay::spine::CallLocus) -> types::SpineLocus {
    types::SpineLocus {
        file: locus.file,
        span: locus.span,
    }
}

/// FR-100: render `spine-run-result/1`, the outcome mapping's stdout
/// document and FR-301 exit status, from `fsl_replay::spine::run`'s result.
pub(super) fn spine_run_result(
    source: &FormalSource,
    function: &str,
    outcome: fsl_replay::spine::CallOutcome,
    settled: Option<fsl_analyze::analyze::document::SettledRun>,
) -> super::Result<RunResult> {
    spine_run_result_for_source(source.source(), function, outcome, settled)
}

pub(super) fn spine_run_result_for_source(
    source: &fsl_foundation::Source,
    function: &str,
    outcome: fsl_replay::spine::CallOutcome,
    settled: Option<fsl_analyze::analyze::document::SettledRun>,
) -> super::Result<RunResult> {
    use fsl_replay::spine::{CallOutcome, CallRefusal};
    let call_category = outcome.category();
    let lifecycle = fsl_replay::OutcomeDocument::from_call(&outcome);
    // FR-963: the run's category is the most severe of the call's and
    // every settled item's, the call's on a tie.
    let category = settled.as_ref().map_or(call_category, |settled| {
        let categories: Vec<filament_semantic_value::catalog::Category> =
            std::iter::once(call_category)
                .chain(
                    settled
                        .records
                        .iter()
                        .map(|record| record.value().category()),
                )
                .collect();
        filament_semantic_value::catalog::Category::most_severe(
            categories.iter().map(|category| category.exit_code()),
        )
        .and_then(|worst| {
            categories
                .iter()
                .copied()
                .find(|category| category.exit_code() == worst)
        })
        .unwrap_or(call_category)
    });
    let exit_code = category.exit_code();
    let (lifecycle, requirements, registry, results) = match settled {
        None => (lifecycle, None, None, None),
        Some(settled) => {
            let registry = serde_json::Value::Array(
                settled
                    .registry
                    .iter()
                    .map(fsl_analyze::analyze::document::RegistryEntry::wire)
                    .collect(),
            );
            (
                lifecycle.with_requirement_items(category, settled.items.clone(), registry.clone()),
                Some(settled.items),
                Some(registry),
                Some(super::RequirementResults {
                    records: settled.records,
                    counterexamples: settled.counterexamples,
                    registry: settled.registry,
                }),
            )
        }
    };
    let outcome = match outcome {
        CallOutcome::Completed(value) => types::SpineOutcome::Completed {
            value: value.form(),
        },
        CallOutcome::Refused(CallRefusal::Record {
            code,
            fields,
            locus,
            location,
        }) => types::SpineOutcome::Refused {
            code: code.code(),
            cause: code.cause(),
            fields: Some(fields),
            locus: locus.map(spine_locus),
            location: location.as_ref().map(spine_location),
        },
        CallOutcome::Refused(CallRefusal::Family { code, location }) => {
            types::SpineOutcome::Refused {
                code: code.code(),
                cause: code.cause(),
                fields: None,
                locus: None,
                location: location.as_ref().map(spine_location),
            }
        }
        CallOutcome::Undefined { reason } => types::SpineOutcome::Undefined { reason },
        CallOutcome::Incomplete(incomplete) => types::SpineOutcome::Incomplete {
            limit: incomplete.limits_field(),
        },
    };
    let document = types::SpineRunReport {
        format: types::Format::SpineRunResult,
        source: types::RunSource {
            identity: source.identity(),
            path: source.path(),
        },
        function,
        outcome,
        requirements,
        registry,
    };
    Ok(RunResult {
        exit_code,
        value: NativeResult(serde_json::to_value(document).map_err(RunCause::Output)?),
        outcome: Some(lifecycle),
        requirements: results,
        sources: crate::Sources::from_sources([source.clone()]),
    })
}

/// FR-640: the clock-selection refusal `refusal` carries, the program's own
/// or one of a library unit it compiles with (FR-099).
fn clock_selection_refusal(
    refusal: &fsl_replay::spine::CompileRefusal,
) -> Option<&fsl_replay::spine::ClockSelectionRefusal> {
    match refusal {
        fsl_replay::spine::CompileRefusal::Emit(
            fsl_replay::spine::EmitRefusal::ClockSelection(refusal),
            _,
        ) => Some(refusal.as_ref()),
        fsl_replay::spine::CompileRefusal::Dependency { refusal, .. } => {
            clock_selection_refusal(refusal)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formal_source::FormalSource;
    use filament_contract_model as ir;
    use filament_semantic_value::catalog::Category;
    use fsl_foundation::SourceIdentity;
    use fsl_replay::spine::CallOutcome;
    use ix_trace_rs::trace;

    fn formal_source() -> FormalSource {
        let source = fsl_foundation::Source::read(
            SourceIdentity {
                authority: "agent-ix".into(),
                identity: "test:spine-run".into(),
            },
            "program.native",
            b"",
            1,
        )
        .unwrap();
        let identity = ir::SourceIdentity::new(
            ir::SourceDocumentId::new("SpineRun").unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        );
        FormalSource::new(source, identity)
    }

    /// Assert `spine_run_result`'s whole stdout document and exit code for
    /// an `Undefined` outcome carrying `reason`, isolated from any specific
    /// kernel or family origin (FR-100-AC-9, FND-006). The renderer
    /// (`fsl-command/src/command/output.rs`) does not distinguish where a `reason`
    /// string came from; `fsl_replay::spine::call::convert_outcome`'s own
    /// tests cover that distinction.
    fn assert_undefined_renders(reason: &'static str) {
        let source = formal_source();
        let result =
            spine_run_result(&source, "seven", CallOutcome::Undefined { reason }, None).unwrap();
        assert_eq!(result.exit_code, 10, "{reason}");
        assert_eq!(
            result.value.as_value(),
            &serde_json::json!({
                "format": "spine-run-result/1",
                "source": {
                    "authority": "agent-ix",
                    "identity": "test:spine-run",
                    "path": "program.native",
                },
                "function": "seven",
                "outcome": {"kind": "undefined", "reason": reason},
            }),
            "{reason}"
        );
    }

    /// FR-100-AC-9 (TC-452 step 4): each kernel `Undefined` reason
    /// (`fsl_replay::spine::call::kernel_undefined_reason`) renders
    /// `{"kind":"undefined","reason":...}` and exits 10, FR-285's code for
    /// undefined.
    #[test]
    #[trace("TC-452", "FR-100-AC-9", "TC-769", "FR-285-AC-4")]
    fn undefined_kernel_reasons_render_and_exit_10() {
        for reason in [
            "division-by-zero",
            "ieee-not-finite",
            "empty-reduction",
            "none-value",
            "sum-out-of-domain",
        ] {
            assert_undefined_renders(reason);
        }
    }

    /// FR-100-AC-9 (TC-452 step 4): each family `Undefined` reason
    /// (`filament_semantic_value::catalog::UndefinedReason::as_str`) renders
    /// `{"kind":"undefined","reason":...}` and exits 10, FR-285's code for
    /// undefined.
    #[test]
    #[trace("TC-452", "FR-100-AC-9", "TC-786", "FR-100-AC-11")]
    fn undefined_family_reasons_render_and_exit_10() {
        for reason in ["precondition-false", "absent-key"] {
            assert_undefined_renders(reason);
        }
    }

    /// FR-090-AC-21 (TC-227): incomplete allInstances coverage renders as
    /// undefined with its exact reason and exits 10. The structured `[T]`
    /// witness is asserted at the CheckedPackage boundary; CLI output carries
    /// the undefined reason only.
    #[test]
    #[trace("FR-090-AC-21", "TC-227")]
    fn incomplete_closure_undefined_renders_and_exits_10() {
        assert_undefined_renders("incomplete-closure");
    }

    fn render(outcome: CallOutcome) -> RunResult {
        spine_run_result(&formal_source(), "seven", outcome, None).unwrap()
    }

    /// FR-100-AC-9 (TC-452 step 4, FND-013): a record refusal renders every
    /// FR-096 member and exits by its catalog code's mapping -- `resource_exhausted`
    /// (`insufficient-next-charge`'s own code) is incomplete, so it exits 22, not the
    /// kernel-refusal row's hardcoded 20.
    #[test]
    #[trace("TC-452", "FR-100-AC-9")]
    fn refused_record_renders_and_exits_by_its_code() {
        use fsl_replay::spine::CallRefusal;
        let fields = std::collections::BTreeMap::from([
            ("setting", "model.ancestor_steps".to_owned()),
            ("limit", "5".to_owned()),
            ("actual", "6".to_owned()),
        ]);
        let result = render(CallOutcome::Refused(CallRefusal::Record {
            code: filament_semantic_value::catalog::CatalogCode::new(
                "resource_exhausted",
                "insufficient-next-charge",
            ),
            fields: fields.clone(),
            locus: None,
            location: None,
        }));
        assert_eq!(result.exit_code, 22);
        assert_eq!(
            result.value.as_value()["outcome"],
            serde_json::json!({
                "kind": "refused",
                "code": "resource_exhausted",
                "cause": "insufficient-next-charge",
                "fields": fields,
            })
        );
    }

    /// FR-100-AC-9 (TC-452 step 4): a kernel `ForeignReference`
    /// record renders `foreign_reference`/`foreign-universe` with its
    /// `required`/`supplied` universes and exits 20, no longer the bare
    /// `{"kind":"refused"}` a kernel-no-record row renders. Goes through the
    /// real conversion (`kernel_refusal_record`) rather than a hand-built
    /// `CallRefusal::Record`. Each universe renders as its root type node.
    #[test]
    #[trace("TC-452", "FR-100-AC-9")]
    fn refused_foreign_reference_renders_record_and_exits_20() {
        use fsl_replay::spine::CallRefusal;
        let root = |name: &str| {
            filament_exact::NodeKey::new(
                "pkg",
                vec![filament_exact::PathSegment::Text(name.to_owned())],
                0,
            )
            .expect("a fixture root type key is well formed")
        };
        let (required, supplied) = (root("first"), root("second"));
        let record = fsl_foundation::diagnostic::kernel_refusal_record(
            &filament_exact::Refusal::ForeignReference {
                required: filament_exact::UniverseId::new(required.clone()),
                supplied: filament_exact::UniverseId::new(supplied.clone()),
            },
            None,
        )
        .expect("foreign_reference has a code");
        let result = render(CallOutcome::Refused(CallRefusal::Record {
            code: record.code(),
            fields: record.fields().clone(),
            locus: None,
            location: None,
        }));
        assert_eq!(result.exit_code, 20);
        assert_eq!(
            result.value.as_value()["outcome"],
            serde_json::json!({
                "kind": "refused",
                "code": "foreign_reference",
                "cause": "foreign-universe",
                "fields": {
                    "required": required.to_string(),
                    "supplied": supplied.to_string(),
                },
            })
        );
    }

    #[test]
    fn typed_admission_command_details_preserve_model_and_type_causes() {
        use filament_checked::key::DeclarationKey;
        use fsl_replay::spine::RunRefusal;
        use fsl_semantics::model::normalize::{ModelRefusal, ModelRefusalCause};
        use fsl_semantics::model::refusal::OfferedSelection;

        let expected = fsl_semantics::model::domain_package::DomainPackageRef {
            identity: "test/orders".to_owned(),
        };
        let model = RunError {
            cause: RunCause::SpineRun(Box::new(RunRefusal::Admission {
                position: 2,
                refusal: ModelRefusal {
                    code: fsl_foundation::diagnostic::Code::ForeignReference,
                    cause: ModelRefusalCause::ForeignModelSelection {
                        actual: OfferedSelection::Document("foreign-model".to_owned()),
                        expected: expected.clone(),
                    },
                    detail: "foreign model".to_owned(),
                },
            })),
            sources: crate::Sources::default(),
        };
        let model_value = error(&model, false).unwrap();
        assert_eq!(
            model_value["details"],
            serde_json::json!({
                "position": 2,
                "model_identity": "foreign-model",
                "expected": {
                    "identity": expected.identity,
                },
            })
        );

        let type_name = DeclarationKey::fixture("foreign/W");
        let type_refusal = RunError {
            cause: RunCause::SpineRun(Box::new(RunRefusal::Admission {
                position: 3,
                refusal: ModelRefusal {
                    code: fsl_foundation::diagnostic::Code::ForeignReference,
                    cause: ModelRefusalCause::ForeignType {
                        member: "w".to_owned(),
                        type_name: type_name.clone(),
                    },
                    detail: "foreign type".to_owned(),
                },
            })),
            sources: crate::Sources::default(),
        };
        let type_value = error(&type_refusal, false).unwrap();
        assert_eq!(
            type_value["details"],
            serde_json::json!({
                "position": 3,
                "member": "w",
                "type": {"package": type_name.package, "node": type_name.node},
            })
        );
    }

    /// FR-100-AC-9 (TC-452 step 4, FND-013): a family refusal with no FR-096
    /// record carries its code and cause but no `fields`/`locus`.
    #[test]
    #[trace("TC-452", "FR-100-AC-9")]
    fn refused_family_with_no_record_renders_code_and_cause_only() {
        use fsl_replay::spine::CallRefusal;
        let result = render(CallOutcome::Refused(CallRefusal::Family {
            code: filament_semantic_value::catalog::CatalogCode::new("ill_typed", "type-mismatch"),
            location: None,
        }));
        assert_eq!(result.exit_code, 20);
        assert_eq!(
            result.value.as_value()["outcome"],
            serde_json::json!({"kind": "refused", "code": "ill_typed", "cause": "type-mismatch"})
        );
    }

    /// FR-100-AC-9 (TC-452 step 4, FND-013): every `location.origin` kind
    /// renders its kebab-case tag, including `type-declaration`.
    #[test]
    #[trace("TC-452", "FR-100-AC-9")]
    fn location_origin_kinds_render_kebab_case() {
        use filament_semantic_value::location::{
            ExpressionLocation as Location, ExpressionOwner as Origin,
        };
        use fsl_replay::spine::CallRefusal;
        let cases = [
            (
                Origin::Body {
                    function: "seven".to_owned(),
                    index: 0,
                },
                serde_json::json!({"kind": "body", "function": "seven", "index": 0}),
            ),
            (
                Origin::Measure {
                    function: "seven".to_owned(),
                    index: 1,
                },
                serde_json::json!({"kind": "measure", "function": "seven", "index": 1}),
            ),
            (
                Origin::Expression,
                serde_json::json!({"kind": "expression"}),
            ),
            (
                Origin::TypeDeclaration {
                    name: "Point".to_owned(),
                },
                serde_json::json!({"kind": "type-declaration", "name": "Point"}),
            ),
            (
                Origin::StateClause {
                    clause: "ParentOrder".to_owned(),
                    index: 0,
                },
                serde_json::json!({"kind": "state-clause", "clause": "ParentOrder", "index": 0}),
            ),
        ];
        for (origin, expected) in cases {
            let result = render(CallOutcome::Refused(CallRefusal::Family {
                code: filament_semantic_value::catalog::CatalogCode::new(
                    "ill_typed",
                    "type-mismatch",
                ),
                location: Some(Location::at(origin, &[2, 0])),
            }));
            assert_eq!(
                result.value.as_value()["outcome"]["location"],
                serde_json::json!({"origin": expected, "path": [2, 0]}),
                "{expected}"
            );
        }
    }

    /// FR-100-AC-9 (TC-452 step 4, FND-019): a record refusal's `locus`
    /// renders `{file, span}` exactly -- the TC-452 fixture-F
    /// values -- and `location` renders on both a record refusal and a
    /// family refusal.
    #[test]
    #[trace("TC-452", "FR-100-AC-9")]
    fn refused_locus_and_location_render() {
        use filament_semantic_value::location::{
            ExpressionLocation as Location, ExpressionOwner as Origin,
        };
        use fsl_replay::spine::{CallLocus, CallRefusal};
        let span = fsl_foundation::LocatedSpan {
            start: fsl_foundation::Position {
                byte: 225,
                line: 3,
                column: 54,
            },
            end: fsl_foundation::Position {
                byte: 226,
                line: 3,
                column: 55,
            },
        };
        let locus = CallLocus {
            file: "fixture.native".to_owned(),
            span,
        };
        let location = Location::at(
            Origin::Body {
                function: "f".to_owned(),
                index: 0,
            },
            &[1],
        );
        let fields = std::collections::BTreeMap::from([("binding", "people".to_owned())]);
        let expected_location = serde_json::json!({"origin": {"kind": "body", "function": "f", "index": 0}, "path": [1]});

        let record = render(CallOutcome::Refused(CallRefusal::Record {
            code: filament_semantic_value::catalog::CatalogCode::new(
                "invalid_runtime_input",
                "absent-key",
            ),
            fields: fields.clone(),
            locus: Some(locus),
            location: Some(location.clone()),
        }));
        assert_eq!(
            record.value.as_value()["outcome"],
            serde_json::json!({
                "kind": "refused",
                "code": "invalid_runtime_input",
                "cause": "absent-key",
                "fields": fields,
                "locus": {
                    "file": "fixture.native",
                    "span": {
                        "start": {"byte": 225, "line": 3, "column": 54},
                        "end": {"byte": 226, "line": 3, "column": 55},
                    },
                },
                "location": expected_location,
            })
        );

        let family = render(CallOutcome::Refused(CallRefusal::Family {
            code: filament_semantic_value::catalog::CatalogCode::new("ill_typed", "type-mismatch"),
            location: Some(location),
        }));
        assert_eq!(
            family.value.as_value()["outcome"],
            serde_json::json!({
                "kind": "refused",
                "code": "ill_typed",
                "cause": "type-mismatch",
                "location": expected_location,
            })
        );
    }

    #[trace("TC-470", "FR-096-AC-12")]
    #[test]
    fn a_report_combines_exit_codes_by_fr_301_severity() {
        for (pair, expected) in [
            ([30, 20], 30),
            ([30, 21], 30),
            ([30, 22], 30),
            ([20, 21], 20),
            ([21, 22], 21),
            ([20, 22], 20),
        ] {
            assert_eq!(Category::most_severe(pair), Some(expected));
            assert_eq!(
                Category::most_severe(pair.into_iter().rev()),
                Some(expected)
            );
        }
        assert_eq!(Category::most_severe(std::iter::empty()), None);
    }
}
