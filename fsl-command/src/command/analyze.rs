// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-620: the `native-analyze/1` request reader and the analyze run over
//! it. The reader applies FR-026's envelope and intake rules, compiles the
//! program as FR-027 compiles it, provisions the supplied documents under
//! their own identities, and hands the items to `fsl-analyze`'s resolver
//! and document writer. The caller negotiates (FR-281), so the run takes the
//! caller's [`Negotiate`].

use std::path::Path;

use filament_exact::{Cancel, CancelCause};
use fsl_analyze::analyze::document::{
    resolve_items, settle_package_run, settle_run, AnalyzeDocumentError, AnalyzeRun, ItemSelection,
    Negotiate, PackageRun, PackageRunError, SettledRun, SubjectSelection,
};
use fsl_analyze::analyze::{AnalyzeLimits, DeadlineSupply, Engine, Engines, SourceProvision};
use fsl_analyze::provider::ProviderOptions;
use fsl_eval::simulation::protocol::{MemorySelection, MemorySelectionRefusal, ResolvedMemory};
use fsl_semantics::model::observation::DocumentProvision;

use super::settle::Settle;
use super::{
    check_sources, intake_with_bytes, request, request_integer_division, require_native_header,
    wire, FileCounts, FormalSource, NativeResult, Result, RunCause, RunError, RunLimits, RunResult,
};

fn invalid(member: &str, reason: impl Into<String>) -> RunCause {
    RunCause::AnalyzeRequest {
        member: member.to_owned(),
        reason: reason.into(),
    }
}

fn receipt_failure(
    error: fsl_analyze::analyze::document::effect_decision::ReceiptError,
) -> RunCause {
    match error.into_analyze_error() {
        AnalyzeDocumentError::Refused(refusal) => RunCause::AnalyzeRefused(Box::new(refusal)),
        AnalyzeDocumentError::Fault => RunCause::AnalyzeFault,
        AnalyzeDocumentError::Identity(error) => super::identity_cause(error),
    }
}

fn receipt_identity(
    node: quire_canonical::NodeRef<'_>,
    pointer: &str,
) -> std::result::Result<fsl_analyze::analyze::document::DocumentSelection, RunCause> {
    let malformed = || invalid(pointer, "document identity is malformed");
    let quire_canonical::Node::Object(members) = node.node() else {
        return Err(malformed());
    };
    // DocumentSelection is a closed pair of strings. Extract only that
    // fixed-depth shape; opaque receipt objects never enter serde's walker
    // or an encoder with a pre-canonical, input-derived ceiling.
    let mut value = serde_json::Map::new();
    for (name, node) in members {
        if !matches!(name, "authority" | "identity") {
            return Err(malformed());
        }
        let quire_canonical::Node::String(text) = node.node() else {
            return Err(malformed());
        };
        value.insert(name.to_owned(), serde_json::Value::String(text.to_owned()));
    }
    serde_json::from_value(serde_json::Value::Object(value)).map_err(|_| malformed())
}

#[cfg(test)]
mod receipt_failure_tests {
    use super::*;
    use ix_trace_rs::trace;

    #[test]
    #[trace("FR-259", "FR-1046-AC-7")]
    fn receipt_resource_and_invariant_errors_keep_public_command_categories() {
        use fsl_analyze::analyze::document::effect_decision::ReceiptError;
        // Typed failure-boundary projection; host allocator exhaustion is
        // deliberately not induced. This is the actual intake error adapter.
        let error = RunError {
            cause: receipt_failure(ReceiptError::Allocation { requested: 8193 }),
            sources: crate::Sources::default(),
        };
        assert_eq!(error.category().exit_code(), 22);
        let document =
            serde_json::to_value(error.outcome_document(fsl_replay::Operation::Analyze)).unwrap();
        assert_eq!(document["diagnostics"][0]["code"], "resource_exhausted");
        assert_eq!(document["diagnostics"][0]["cause"], "allocation-failed");
        assert_eq!(
            document["diagnostics"][0]["details"],
            serde_json::json!({"requested": "8193"})
        );
        assert_eq!(document["last_stage"], serde_json::Value::Null);
        let error = RunError {
            cause: receipt_failure(ReceiptError::Fault),
            sources: crate::Sources::default(),
        };
        assert_eq!(error.category().exit_code(), 30);
        let document =
            serde_json::to_value(error.outcome_document(fsl_replay::Operation::Analyze)).unwrap();
        assert_eq!(document["diagnostics"][0]["code"], "runtime_invariant");
    }
}

fn validate_activated_population(
    position: usize,
    population: &serde_json::Value,
) -> std::result::Result<(), (String, String)> {
    let base = format!("/items/{position}/population");
    let invalid_at = |pointer: String, reason: String| Err((pointer, reason));
    let Some(groups) = population.as_array() else {
        return invalid_at(base, "population must be an array".to_owned());
    };
    for (group_index, group) in groups.iter().enumerate() {
        let group_pointer = format!("{base}/{group_index}");
        let Some(group) = group.as_object() else {
            return invalid_at(
                group_pointer,
                "population entry must be an object".to_owned(),
            );
        };
        for key in group.keys() {
            if !matches!(key.as_str(), "role" | "instance" | "members") {
                return invalid_at(
                    format!("{group_pointer}/{}", pointer_token(key)),
                    format!("unknown population entry member `{key}`"),
                );
            }
        }
        let role_pointer = format!("{group_pointer}/role");
        if group
            .get("role")
            .and_then(serde_json::Value::as_str)
            .is_none()
        {
            return invalid_at(role_pointer, "role must be a string".to_owned());
        }
        if let Some(instance) = group.get("instance") {
            if !instance.is_null() && !instance.is_string() {
                return invalid_at(
                    format!("{group_pointer}/instance"),
                    "instance must be a string or null".to_owned(),
                );
            }
        }
        let members_pointer = format!("{group_pointer}/members");
        let Some(members) = group.get("members").and_then(serde_json::Value::as_array) else {
            return invalid_at(members_pointer, "members must be an array".to_owned());
        };
        for (member_index, member) in members.iter().enumerate() {
            let member_pointer = format!("{members_pointer}/{member_index}");
            let Some(member) = member.as_object() else {
                return invalid_at(
                    member_pointer,
                    "population member must be an object".to_owned(),
                );
            };
            for key in member.keys() {
                if !matches!(key.as_str(), "authority" | "identity") {
                    return invalid_at(
                        format!("{member_pointer}/{}", pointer_token(key)),
                        format!("unknown population member field `{key}`"),
                    );
                }
            }
            for field in ["authority", "identity"] {
                let pointer = format!("{member_pointer}/{field}");
                let Some(value) = member.get(field).and_then(serde_json::Value::as_str) else {
                    return invalid_at(pointer, format!("{field} must be a string"));
                };
                if value.is_empty() {
                    return invalid_at(pointer, format!("{field} must be nonempty"));
                }
            }
        }
    }
    Ok(())
}

/// Whether `role` is one of FSpec FR-331's five `premises` roles a
/// composition item names.
fn is_premise_role(role: fsl_replay::planner::SlotRole) -> bool {
    use fsl_replay::planner::SlotRole;
    matches!(
        role,
        SlotRole::ComponentGuarantee
            | SlotRole::Rely
            | SlotRole::LivenessDependency
            | SlotRole::Fairness
            | SlotRole::Environment
    )
}

/// FR-1044: a composition item's `premises` is an array of closed
/// `{role, key}` objects whose `role` is one of FSpec FR-331's five, each
/// `key` an object. Whether the list is the derived one is decided against
/// the checked declaration, per item.
fn validate_composition_premises(
    position: usize,
    premises: &serde_json::Value,
) -> std::result::Result<(), (String, String)> {
    let base = format!("/items/{position}/premises");
    let Some(entries) = premises.as_array() else {
        return Err((base, "premises must be an array".to_owned()));
    };
    for (entry_index, entry) in entries.iter().enumerate() {
        let pointer = format!("{base}/{entry_index}");
        let Some(entry) = entry.as_object() else {
            return Err((pointer, "a premise must be an object".to_owned()));
        };
        if let Some(member) = entry
            .keys()
            .find(|member| !matches!(member.as_str(), "role" | "key"))
        {
            return Err((
                format!("{pointer}/{}", pointer_token(member)),
                format!("unknown premise member `{member}`"),
            ));
        }
        match entry.get("role").and_then(serde_json::Value::as_str) {
            Some(role)
                if fsl_replay::planner::SlotRole::parse(role).is_some_and(is_premise_role) => {}
            Some(role) => {
                return Err((
                    format!("{pointer}/role"),
                    format!("`{role}` is not a premise role"),
                ));
            }
            None => {
                return Err((
                    format!("{pointer}/role"),
                    "role must be a string".to_owned(),
                ))
            }
        }
        if !entry.get("key").is_some_and(serde_json::Value::is_object) {
            return Err((format!("{pointer}/key"), "key must be an object".to_owned()));
        }
    }
    Ok(())
}

fn pointer_token(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

/// A read `native-analyze/1` request: its compiled-from sources, supplied
/// documents, subject, items and limits, with no compiler, evaluator or
/// engine call made.
pub struct PreparedAnalyze {
    input: Box<AnalyzeInput>,
}

/// The owned inputs of one analyze run (FR-620 "Library surface").
pub struct AnalyzeInput {
    /// The program source.
    pub source: FormalSource,
    /// The caller's domain-package selections, compile request members and
    /// libraries, compiled as FR-027 compiles them.
    compile: wire::CompileRequest,
    /// The supplied documents, each under its own identity.
    pub documents: DocumentProvision,
    /// The request's default model subject.
    pub subject: SubjectSelection,
    /// The request's `memory` member (FSL FR-1033, FR-219), also carried by
    /// [`Self::subject`].
    pub memory: Vec<fsl_eval::simulation::protocol::MemorySelection>,
    /// The caller's items; `None` for FR-620's default items.
    pub items: Option<Vec<ItemSelection>>,
    /// The limits every item of the run shares.
    pub limits: AnalyzeLimits,
    /// The run's deadline in milliseconds.
    pub deadline_ms: Option<u64>,
    /// The optional request-supplied integer-division law.
    integer_division: Option<fsl_replay::AdmittedIntegerDivision>,
}

impl PreparedAnalyze {
    /// Transfer the read inputs without running a stage.
    pub fn into_input(self) -> AnalyzeInput {
        *self.input
    }
}

/// Read a `native-analyze/1` request and every file it selects (FR-620,
/// FR-026), performing no compiler, evaluator, engine or provider call.
///
/// # Errors
///
/// `invalid-request` for a malformed, wrong-format or unknown-member
/// request; FR-026's resource causes for a byte or file limit; the source
/// refusal of an unreadable program.
pub fn prepare_analyze(
    path: &Path,
    limits: RunLimits,
) -> std::result::Result<PreparedAnalyze, Box<RunError>> {
    super::with_request_limits(path, limits.request_bytes, |directory, bytes| {
        prepare_bytes(directory, bytes, limits)
    })
}

fn prepare_bytes(directory: &Path, bytes: &[u8], limits: RunLimits) -> Result<PreparedAnalyze> {
    match read(directory, bytes, limits)? {
        Readout::Source(input, _) => Ok(PreparedAnalyze {
            input: Box::new(input),
        }),
        // A prepared run holds the source it compiles; a package request
        // has none to prepare (FR-850).
        Readout::Package(_) => Err(RunCause::AnalyzeNotYetImplemented {
            member: "package",
            detail: ": a prepared run holds a program source".to_owned(),
            locus: None,
        }),
    }
}

/// What [`read`] read: a `program` request's inputs with the intake that
/// still charges compilation, or a `package` request's admitted package.
enum Readout<'d> {
    Source(AnalyzeInput, super::Intake<'d>),
    Package(PackageSelection),
}

/// A `package` request (FR-850): the package the strict reader admitted
/// from the request's bytes, with the request's other members.
struct PackageSelection {
    admitted: AdmittedHandoff,
    members: Box<RunMembers>,
    memory: Vec<fsl_eval::simulation::protocol::MemorySelection>,
    items: Option<Vec<ItemSelection>>,
    deadline_ms: Option<u64>,
}

/// FR-531 "Denial is its own typed refusal": a denied identity join names the
/// `i2.*` setting, never a witness defect.
fn native_join_cause(failure: fsl_package::NativeJoinFailure) -> RunCause {
    let setting = failure
        .setting()
        .map_or_else(|| "i2.work_units".to_owned(), |setting| setting.to_string());
    RunCause::AnalyzePackage {
        code: fsl_foundation::Code::StageLimitExceeded,
        cause: failure
            .limit()
            .map(|limit| limit.kind().catalog_cause().to_owned()),
        pointer: None,
        message: format!("stage_limit_exceeded: {setting}: {failure:?}"),
    }
}

/// FR-1044: a composition whose admitted facts could not be read. A denied
/// native charge names its `i2.*` setting (FR-531); any other refusal of an
/// admitted package's own rows is an internal fault.
fn package_composition_cause(
    node: &fsl_package::CheckedNodeId,
    refusal: &fsl_replay::planner::PackageCompositionRefusal,
) -> RunCause {
    let fsl_replay::planner::PackageCompositionRefusal::Native { denied, .. } = refusal else {
        return RunCause::AnalyzeFault;
    };
    let setting = match denied.kind {
        filament_contract_model::NativeJoinChargeKind::Work => fsl_foundation::Setting::I2WorkUnits,
        filament_contract_model::NativeJoinChargeKind::Storage => {
            fsl_foundation::Setting::I2NativeJoinStorage
        }
    };
    RunCause::AnalyzePackage {
        code: fsl_foundation::Code::StageLimitExceeded,
        cause: None,
        pointer: None,
        message: format!("stage_limit_exceeded: {setting}: composition at {node:?}: {refusal}"),
    }
}

/// FR-850, FR-940: analyze a `package` request over the `BarePackage` of
/// its admitted package, read through `AnalysisView`, never compiling
/// source, with the caller's `negotiate` under `cancel` and its deadline.
/// Each item reads the admitted package form its engine family supports.
fn run_package(
    selection: PackageSelection,
    negotiate: &dyn Negotiate,
    providers: Vec<Engine>,
    options: &ProviderOptions,
    cancel: &Cancel,
) -> Result<RunResult> {
    let engines = engines_with(providers, options)?;
    let PackageSelection {
        admitted,
        members,
        memory,
        items,
        deadline_ms,
    } = selection;
    let AdmittedHandoff {
        package: admitted,
        identities,
        equality,
        mut budget,
        supplied,
        package_bytes,
        identity_bytes,
        equality_bytes,
        dependencies,
    } = admitted;
    let package = fsl_package::BarePackage::new(&admitted)
        .with_canonical_bytes(&package_bytes)
        .with_dependency_artifacts(&dependencies, &mut budget)
        .and_then(|package| {
            package.with_supply(
                fsl_package::BareSupply {
                    identities: &identities,
                    equality: equality.as_ref(),
                },
                &mut budget,
            )
        })
        .map_err(native_join_cause)?;
    let RunMembers {
        documents,
        mut subject,
        limits,
    } = *members;
    check_memory_using(&memory, |protocol, entries| {
        use fsl_package::AnalysisView;
        package
            .protocol_execution_named(protocol)
            .ok()
            .flatten()
            .map(|form| ResolvedMemory::resolve_package(&form, entries))
    })?;
    subject.memory = memory;
    let binding = subject.binding(&supplied, &documents);
    let run = PackageRun {
        package: &package,
        package_bytes: &package_bytes,
        supply: fsl_replay::HandoffSupply {
            dependencies: &dependencies,
            identities: &identity_bytes,
            equality: equality_bytes.as_deref(),
        },
        subject,
        binding,
        items: items.as_deref(),
        engines: &engines,
        limits,
        budget,
    };
    let mut settled = with_deadline(cancel, deadline_ms, || {
        settle_package_run(run, negotiate, cancel)
    })
    .map_err(|failure| match failure {
        PackageRunError::Input(refusal) => package_input_refusal(refusal, package.package().name()),
        PackageRunError::ClauseBuild { node, refusal } => package_clause_refusal(node, refusal),
        PackageRunError::Pending(pending) => RunCause::AnalyzeNotYetImplemented {
            member: "package",
            detail: pending.detail(),
            locus: pending.locus(),
        },
        PackageRunError::Document(AnalyzeDocumentError::Refused(refusal)) => {
            RunCause::AnalyzeRefused(Box::new(refusal))
        }
        PackageRunError::Document(AnalyzeDocumentError::Fault) => RunCause::AnalyzeFault,
        PackageRunError::Composition { node, refusal } => {
            package_composition_cause(&node, &refusal)
        }
        PackageRunError::Document(AnalyzeDocumentError::Identity(error)) => {
            super::identity_cause(error)
        }
    })?;
    settled
        .bind_effect_decisions(items.as_deref().unwrap_or_default(), &documents)
        .map_err(|failure| match failure {
            AnalyzeDocumentError::Refused(refusal) => RunCause::AnalyzeRefused(Box::new(refusal)),
            AnalyzeDocumentError::Fault => RunCause::AnalyzeFault,
            AnalyzeDocumentError::Identity(error) => super::identity_cause(error),
        })?;
    Ok(RunResult {
        exit_code: settled.exit_code,
        value: NativeResult::analyze_document(settled.document),
        outcome: None,
        requirements: None,
        sources: crate::Sources::default(),
    })
}

/// Retain the original admission, limit or invariant class of an input that
/// could not rebuild from the admitted package.
fn package_input_refusal(
    refusal: fsl_eval::simulation::model::PendingPackageForm,
    package: &str,
) -> RunCause {
    use fsl_eval::simulation::model::PendingPackageForm;
    let message = refusal.to_string();
    match refusal {
        PendingPackageForm::Model(failure) => RunCause::AnalyzeRefused(Box::new(
            fsl_analyze::analyze::AnalyzeRefusal::Binding(failure),
        )),
        PendingPackageForm::World(failure) | PendingPackageForm::Reward(failure) => {
            let (code, cause) = match failure.into_view(package) {
                Ok(view) => {
                    let (code, cause) = view.catalog();
                    (
                        fsl_foundation::Code::from_code(code)
                            .unwrap_or(fsl_foundation::Code::RuntimeInvariant),
                        cause,
                    )
                }
                Err(fsl_package::SlotSelectionFailure::Fault(fault)) => {
                    let catalog = fault.catalog_code();
                    (fsl_foundation::Code::RuntimeInvariant, catalog.cause())
                }
                Err(
                    fsl_package::SlotSelectionFailure::MissingNode(_)
                    | fsl_package::SlotSelectionFailure::MalformedNode(_)
                    | fsl_package::SlotSelectionFailure::MissingMember { .. },
                ) => (
                    fsl_foundation::Code::RuntimeInvariant,
                    "established-invariant-broken",
                ),
            };
            RunCause::AnalyzePackage {
                code,
                cause: Some(cause.to_owned()),
                pointer: None,
                message,
            }
        }
        PendingPackageForm::Operations(failure) => {
            let catalog = failure
                .errors
                .first()
                .map(|error| error.cause.catalog_code());
            RunCause::AnalyzePackage {
                code: catalog
                    .and_then(|catalog| fsl_foundation::Code::from_code(catalog.code()))
                    .unwrap_or(fsl_foundation::Code::RuntimeInvariant),
                cause: catalog.map(|catalog| catalog.cause().to_owned()),
                pointer: None,
                message,
            }
        }
        PendingPackageForm::Form { form, node } => RunCause::AnalyzeNotYetImplemented {
            member: "package",
            detail: format!(
                ": the {} form at {node:?} has no package form yet",
                form.name()
            ),
            locus: None,
        },
        refusal @ (PendingPackageForm::ProbabilisticNodes(_)
        | PendingPackageForm::Probabilistic(_)
        | PendingPackageForm::SourceModel(_)) => RunCause::AnalyzePackage {
            code: fsl_foundation::Code::RuntimeInvariant,
            cause: Some("established-invariant-broken".to_owned()),
            pointer: None,
            message: refusal.to_string(),
        },
    }
}

/// A selected clause is distinct from an absent claim name, including when
/// its checked form reaches a stage limit.
fn package_clause_refusal(
    node: fsl_package::CheckedNodeId,
    refusal: fsl_semantics::check::PackageClauseRefusal,
) -> RunCause {
    use fsl_semantics::check::PackageClauseRefusal;
    let message = format!("the selected package clause at {node:?} could not rebuild: {refusal:?}");
    let (code, cause) = match refusal {
        PackageClauseRefusal::Profile(_) => {
            return RunCause::AnalyzeNotYetImplemented {
                member: "package",
                detail: format!(": {message}"),
                locus: None,
            }
        }
        PackageClauseRefusal::Horizon(limit)
        | PackageClauseRefusal::Domains(fsl_semantics::family::ClassifyFailure::Limit(limit)) => (
            fsl_foundation::Code::StageLimitExceeded,
            limit.kind().catalog_cause(),
        ),
        PackageClauseRefusal::Domains(fsl_semantics::family::ClassifyFailure::Fault(fault)) => (
            fsl_foundation::Code::RuntimeInvariant,
            fault.catalog_code().cause(),
        ),
        PackageClauseRefusal::Arena
        | PackageClauseRefusal::Shape
        | PackageClauseRefusal::ClockBinding => (
            fsl_foundation::Code::RuntimeInvariant,
            "established-invariant-broken",
        ),
    };
    RunCause::AnalyzePackage {
        code,
        cause: Some(cause.to_owned()),
        pointer: None,
        message,
    }
}

#[cfg(test)]
mod package_intake_tests {
    use super::*;
    use filament_semantic_value::catalog::{CatalogCode, InternalFault};
    use fsl_eval::simulation::model::PendingPackageForm;
    use fsl_foundation::diagnostic::LimitExceeded;
    use fsl_foundation::setting::Setting;
    use fsl_foundation::Span;
    use fsl_semantics::check::{
        AssemblyCause, AssemblyError, AssemblyRefusal, PackageClauseRefusal,
    };
    use fsl_semantics::model::observation::{AdmissionFailure, AdmissionRecord};
    use ix_trace_rs::trace;

    /// Input admission retains its own class rather than becoming an unnamed fault.
    #[trace("FR-850-AC-3", "FR-940-AC-6")]
    #[test]
    fn package_input_failures_preserve_the_typed_form() {
        let slot = || {
            fsl_package::BareSelectionFailure::Slot(fsl_package::SlotSelectionFailure::Fault(
                InternalFault::new("test", "world-invariant"),
            ))
        };
        let record = || AdmissionRecord {
            catalog: CatalogCode::new("invalid_model_binding", "wrong-model-selection"),
            fields: Default::default(),
            path: Vec::new(),
            kernel: None,
        };
        let failures = [
            (PendingPackageForm::World(slot()), "runtime_invariant"),
            (
                PendingPackageForm::Model(AdmissionFailure::Refused(Box::new(record()))),
                "invalid_model_binding",
            ),
            (
                PendingPackageForm::Operations(AssemblyRefusal {
                    errors: vec![AssemblyError {
                        cause: AssemblyCause::TypeLimit(LimitExceeded::new(Setting::S3Nodes, 7, 8)),
                        span: Span { start: 0, end: 0 },
                    }],
                }),
                "stage_limit_exceeded",
            ),
            (
                PendingPackageForm::ProbabilisticNodes(fsl_package::ProbabilisticReadFault(
                    "node-invariant",
                )),
                "runtime_invariant",
            ),
            (
                PendingPackageForm::Probabilistic(
                    fsl_semantics::check::PackageProbabilisticRefusal::UnknownOperation(
                        "step".to_owned(),
                    ),
                ),
                "runtime_invariant",
            ),
            (PendingPackageForm::Reward(slot()), "runtime_invariant"),
        ];
        for (failure, code) in failures {
            let cause = package_input_refusal(failure, "test");
            assert_eq!(cause.code().as_str(), code, "{cause:?}");
            assert!(!matches!(
                cause,
                RunCause::AnalyzeFault | RunCause::AnalyzeNotYetImplemented { .. }
            ));
        }
        let cause = package_input_refusal(
            PendingPackageForm::Model(AdmissionFailure::Incomplete(Box::new(AdmissionRecord {
                catalog: CatalogCode::new("incomplete_population", "incomplete-scope"),
                ..record()
            }))),
            "test",
        );
        assert_eq!(cause.code().as_str(), "incomplete_population");
    }

    /// A real builder domain limit retains its configured bound and limit class.
    #[trace("FR-850-AC-3", "FR-940-AC-6")]
    #[test]
    fn a_known_package_clause_retains_its_build_refusal() {
        let form = fsl_semantics::check::PackageClauseForm {
            name: "C".to_owned(),
            span: Span { start: 0, end: 0 },
            profile: fsl_semantics::check::TemporalProfile::InfiniteTrace,
            package_name: "test".to_owned(),
            over: ("x".to_owned(), filament_exact::ValueType::Boolean),
            clock: "events".to_owned(),
            activation: fsl_semantics::check::Activation::Origin,
            trigger: None,
            captures: Vec::new(),
            populations: Vec::new(),
            fairness: Vec::new(),
            formula: vec![fsl_semantics::check::PackageFormulaNode::Constant(true)],
        };
        let types = filament_semantic_value::declaration::TypeEnvironment::default();
        assert!(
            fsl_semantics::check::CheckedTemporalClause::of_package_form(form.clone(), &types, 1)
                .is_ok()
        );
        let refusal = fsl_semantics::check::CheckedTemporalClause::of_package_form(form, &types, 0)
            .expect_err("a real domain traversal exceeds its configured bound");
        let PackageClauseRefusal::Domains(fsl_semantics::family::ClassifyFailure::Limit(limit)) =
            &refusal
        else {
            panic!("the domain limit remains typed: {refusal:?}");
        };
        assert_eq!(limit.configured_bound(), 0);
        assert_eq!(limit.actual(), 1);
        let node = fsl_semantics::check::wire_id::structural_id(
            &filament_exact::NodeKey::new(
                "test",
                vec![filament_exact::PathSegment::Text("C".to_owned())],
                0,
            )
            .expect("node"),
        );
        for (refusal, bound) in [
            (
                PackageClauseRefusal::Horizon(LimitExceeded::new(Setting::S3WorkUnits, 7, 8)),
                7,
            ),
            (refusal, 0),
        ] {
            let fsl_analyze::analyze::ClauseItemRefusal::Build(refusal) =
                fsl_analyze::analyze::ClauseItemRefusal::from(
                    fsl_eval::simulation::model::PendingClauseForm::Build(refusal),
                )
            else {
                panic!("a selected clause keeps its builder refusal rather than becoming unknown");
            };
            let cause = package_clause_refusal(node.clone(), refusal);
            assert_eq!(cause.code().as_str(), "stage_limit_exceeded");
            assert_eq!(cause.category().exit_code(), 22);
            let RunCause::AnalyzePackage { message, cause, .. } = cause else {
                panic!("typed limit");
            };
            assert!(
                message.contains(&format!("configured_bound: {bound}")),
                "{message}"
            );
            assert!(matches!(
                cause.as_deref(),
                Some("work-budget-exceeded" | "node-count-exceeded")
            ));
        }
    }

    /// FR-1044 "Current system behavior", FR-531: a native charge denied
    /// while the package route reads a composition's admitted facts refuses
    /// the request whole as `stage_limit_exceeded` (exit 22) naming the
    /// `i2.*` setting of the denied charge; any other refusal of the admitted
    /// rows is a fault.
    #[trace("FR-1044-AC-10")]
    #[test]
    fn a_denied_composition_charge_names_its_native_join_setting() {
        use filament_contract_model::{NativeJoinChargeKind, NativeJoinDeniedCharge};
        use fsl_replay::planner::PackageCompositionRefusal;
        let node = fsl_semantics::check::wire_id::structural_id(
            &filament_exact::NodeKey::new(
                "test",
                vec![filament_exact::PathSegment::Text("Sys".to_owned())],
                0,
            )
            .expect("node"),
        );
        for (kind, setting) in [
            (NativeJoinChargeKind::Work, Setting::I2WorkUnits),
            (NativeJoinChargeKind::Storage, Setting::I2NativeJoinStorage),
        ] {
            let refusal = PackageCompositionRefusal::Native {
                denied: NativeJoinDeniedCharge { kind, amount: 1 },
                locus: None,
                configured_bound: 0,
            };
            let cause = package_composition_cause(&node, &refusal);
            assert_eq!(cause.code().as_str(), "stage_limit_exceeded");
            assert_eq!(cause.category().exit_code(), 22);
            let RunCause::AnalyzePackage { message, .. } = cause else {
                panic!("a typed limit refusal");
            };
            assert!(message.contains(&setting.to_string()), "{message}");
        }
        assert!(matches!(
            package_composition_cause(&node, &PackageCompositionRefusal::Member),
            RunCause::AnalyzeFault
        ));
    }
}

/// The request's inputs and the intake that read them, which still charges
/// the libraries and domain packages compilation reads.
fn read<'d>(directory: &'d Path, bytes: &[u8], limits: RunLimits) -> Result<Readout<'d>> {
    let dependent_bytes = limits.dependent_bytes;
    super::request_shape::admit(super::request_shape::Family::Analyze, bytes)?;
    let replay = fsl_replay::ReplayLimits::default().with_input_bytes(limits.replay_input_bytes);
    let mut request: wire::AnalyzeRequest = request(bytes).map_err(|cause| match cause {
        RunCause::Json(error) => invalid("request", error.to_string()),
        cause => cause,
    })?;
    validate_items(request.items.as_deref())?;
    if let Some(package) = &request.package {
        let (admitted, mut intake) = admit_package(directory, dependent_bytes, &request, package)?;
        let memory = read_memory(&request.memory)?;
        let members = Box::new(read_run_members(&request, &mut intake, replay)?);
        return Ok(Readout::Package(PackageSelection {
            admitted,
            members,
            memory,
            items: request.items,
            deadline_ms: request.deadline_ms,
        }));
    }
    let integer_division = request_integer_division(request.integer_division.take())?;
    let Some(program) = &request.program else {
        return Err(invalid("program", "a program or package is required"));
    };
    let memory = read_memory(&request.memory)?;
    let mut intake = intake_with_bytes(
        directory,
        FileCounts {
            programs: request.libraries.len().saturating_add(1),
            models: request.models.len(),
            observations: request.documents.len(),
            extensions: 0,
        },
        dependent_bytes,
    )?;
    let source = intake.source(&program.source, "program.source")?;
    require_native_header(source.source())?;
    let RunMembers {
        documents,
        mut subject,
        limits,
    } = read_run_members(&request, &mut intake, replay)?;
    subject.memory.clone_from(&memory);
    let compile = wire::CompileRequest {
        models: Some(request.models),
        program: wire::CompileProgram {
            sources: Vec::new(),
            extraction: false,
        },
        refinement_internals: request.refinement_internals,
        libraries: Some(request.libraries),
        extensions: Vec::new(),
        clocks: (!request.clocks.is_empty()).then_some(request.clocks),
        output: None,
        handoff: None,
        integer_division: None,
        check_mode: filament_semantic_value::checking::CheckMode::Linked,
    };
    Ok(Readout::Source(
        AnalyzeInput {
            source,
            compile,
            documents,
            subject,
            memory,
            items: request.items,
            limits,
            deadline_ms: request.deadline_ms,
            integer_division,
        },
        intake,
    ))
}

/// Read the `native-analyze/1` request at `path`, compile its program,
/// resolve its items, have `negotiate` settle them, run them under `cancel`
/// and its deadline, and return FR-621's document with its FR-285 exit code.
///
/// # Errors
///
/// A whole-request refusal (FR-624): the request stage's, a compile stage's,
/// or the subject binding's and source provision's; the caller writes its
/// `analyze` refusal document.
pub fn analyze(
    path: &Path,
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> std::result::Result<RunResult, Box<RunError>> {
    analyze_with_limits(path, RunLimits::default(), negotiate, cancel)
}

/// [`analyze`] under the caller's request and dependent-file byte ceilings.
///
/// # Errors
///
/// As [`analyze`].
pub fn analyze_with_limits(
    path: &Path,
    limits: RunLimits,
    negotiate: &dyn Negotiate,
    cancel: &Cancel,
) -> std::result::Result<RunResult, Box<RunError>> {
    analyze_with_providers_and_limits(
        path,
        limits,
        negotiate,
        Vec::new(),
        &ProviderOptions::default(),
        cancel,
    )
}

/// [`analyze`] with the caller's started process providers (FR-635 "One rule
/// for every provider-routing entry"): each an [`Engine::external`] engine
/// over its `Process` descriptor and the handle its items are sent over.
/// They register beside `Engines::fsl()` before any item is negotiated, so
/// an item reaches one only by negotiation. An empty `providers` is
/// [`analyze`]. `options` carries the run's FSpec FR-462 limits, which every
/// provider request carries (FR-964).
///
/// # Errors
///
/// As [`analyze`]; and `invalid-request` naming `providers` when a provider
/// repeats a registered `BackendId`, before any item runs.
pub fn analyze_with_providers(
    path: &Path,
    negotiate: &dyn Negotiate,
    providers: Vec<Engine>,
    options: &ProviderOptions,
    cancel: &Cancel,
) -> std::result::Result<RunResult, Box<RunError>> {
    analyze_with_providers_and_limits(
        path,
        RunLimits::default(),
        negotiate,
        providers,
        options,
        cancel,
    )
}

/// [`analyze_with_providers`] under the caller's request and dependent-file
/// byte ceilings.
///
/// # Errors
///
/// As [`analyze_with_providers`].
pub fn analyze_with_providers_and_limits(
    path: &Path,
    limits: RunLimits,
    negotiate: &dyn Negotiate,
    providers: Vec<Engine>,
    options: &ProviderOptions,
    cancel: &Cancel,
) -> std::result::Result<RunResult, Box<RunError>> {
    super::with_request_limits(path, limits.request_bytes, |directory, bytes| {
        let (input, mut intake) = match read(directory, bytes, limits)? {
            Readout::Source(input, intake) => (input, intake),
            Readout::Package(selection) => {
                return run_package(selection, negotiate, providers, options, cancel);
            }
        };
        let mut result = run_input(&mut intake, input, negotiate, providers, options, cancel)?;
        result.sources = crate::Sources::from_sources(intake.sources.iter().cloned());
        Ok(result)
    })
}

/// FSL's engines with the caller's process providers registered beside them,
/// under the run's provider `options`.
pub(super) fn engines_with(providers: Vec<Engine>, options: &ProviderOptions) -> Result<Engines> {
    let mut engines = Engines::fsl()
        .map_err(|_| RunCause::AnalyzeFault)?
        .under(*options);
    for provider in providers {
        engines.with(provider).map_err(|duplicate| {
            invalid(
                "providers",
                format!(
                    "a configured provider repeats the registered backend {}",
                    duplicate.backend.as_str()
                ),
            )
        })?;
    }
    Ok(engines)
}

fn run_input(
    intake: &mut super::Intake<'_>,
    input: AnalyzeInput,
    negotiate: &dyn Negotiate,
    providers: Vec<Engine>,
    options: &ProviderOptions,
    cancel: &Cancel,
) -> Result<RunResult> {
    // The registry is fixed before the program is read, so a provider that
    // cannot register refuses the request before any stage runs.
    let engines = engines_with(providers, options)?;
    let AnalyzeInput {
        source,
        compile,
        documents,
        subject,
        memory,
        items,
        limits,
        deadline_ms,
        integer_division,
    } = input;
    let routed = RoutedItems {
        documents,
        subject,
        memory,
        items,
        limits,
        deadline_ms,
    };
    let sources = std::slice::from_ref(&source);
    check_sources(
        &compile,
        intake,
        sources,
        false,
        fsl_replay::spine::SpineLimits::default(),
        integer_division,
        None,
        |_intake, checked, selections, _, packages, libraries, _, _, _, _| {
            let units = std::iter::once(&source).chain(libraries);
            let settled = settle_items(
                checked.package(),
                &packages,
                units,
                selections,
                &routed,
                &Settle {
                    negotiate,
                    engines: &engines,
                    cancel,
                },
            )?;
            Ok(RunResult {
                exit_code: settled.exit_code,
                value: NativeResult::analyze_document(settled.document),
                outcome: None,
                requirements: None,
                sources: crate::Sources::default(),
            })
        },
    )
}

/// The FR-620 items of one request and what they read besides the compiled
/// program: an analyze request's, or a monitor request's provider-routed
/// `items` (FR-644).
pub(super) struct RoutedItems {
    /// The supplied documents, each under its own identity.
    pub documents: DocumentProvision,
    /// The request's default model subject.
    pub subject: SubjectSelection,
    /// The request's `memory` member (FSL FR-1033, FR-219).
    pub memory: Vec<fsl_eval::simulation::protocol::MemorySelection>,
    /// The caller's items; `None` for FR-620's default items.
    pub items: Option<Vec<ItemSelection>>,
    /// The limits every item of the run shares.
    pub limits: AnalyzeLimits,
    /// The run's deadline in milliseconds.
    pub deadline_ms: Option<u64>,
}

/// The FR-620 settlement of `routed` over the compiled `package`: each item
/// resolved against `settle`'s registry, negotiated by its negotiator, the
/// in-process items run on the linked engines and the provider items sent
/// through one FR-950 request per backend (FR-635), all under its cancel
/// handle and the run's deadline. `units` are the request's units in
/// request order, the program then each library as listed.
///
/// # Errors
///
/// A whole-request refusal (FR-624) of the subject binding or source
/// provision, or an internal fault.
pub(super) fn settle_items<'u>(
    package: &fsl_package::CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
    units: impl IntoIterator<Item = &'u FormalSource>,
    selections: &[fsl_package::TemporalClockSelection],
    routed: &RoutedItems,
    settle: &Settle<'_>,
) -> Result<SettledRun> {
    let mut provision = SourceProvision::default().with_temporal_selections(selections.to_vec());
    let mut labels = Vec::new();
    for read in units {
        labels.push(read.source().reference().clone());
        provision.insert(
            read.source().reference(),
            read.source().text().as_bytes().to_vec(),
        );
    }
    let mut registry = fsl_route::Registry::new();
    for descriptor in settle.engines.registrations() {
        registry
            .register(descriptor.clone())
            .map_err(|_| RunCause::AnalyzeFault)?;
    }
    check_memory(package, &routed.memory)?;
    let binding = routed.subject.binding(packages, &routed.documents);
    let resolved = resolve_items(
        package,
        &routed.subject,
        routed.items.as_deref(),
        &registry,
        &labels,
        &binding,
        &routed.limits,
    );
    let run = AnalyzeRun {
        package,
        binding,
        provision,
        items: resolved,
        engines: settle.engines,
        limits: routed.limits,
    };
    let mut settled = with_deadline(settle.cancel, routed.deadline_ms, || {
        settle_run(run, settle.negotiate, settle.cancel)
    })
    .map_err(|error| match error {
        AnalyzeDocumentError::Refused(refusal) => RunCause::AnalyzeRefused(Box::new(refusal)),
        AnalyzeDocumentError::Fault => RunCause::AnalyzeFault,
        AnalyzeDocumentError::Identity(error) => super::identity_cause(error),
    })?;
    settled
        .bind_effect_decisions(
            routed.items.as_deref().unwrap_or_default(),
            &routed.documents,
        )
        .map_err(|error| match error {
            AnalyzeDocumentError::Refused(refusal) => RunCause::AnalyzeRefused(Box::new(refusal)),
            AnalyzeDocumentError::Fault => RunCause::AnalyzeFault,
            AnalyzeDocumentError::Identity(error) => super::identity_cause(error),
        })?;
    Ok(settled)
}

/// Run `work` with `cancel` cancelled for `deadline` once `deadline_ms`
/// elapses (FR-287): a deadline of 0 has passed before any item runs.
fn with_deadline<T>(cancel: &Cancel, deadline_ms: Option<u64>, work: impl FnOnce() -> T) -> T {
    match deadline_ms {
        None => work(),
        Some(0) => {
            cancel.cancel(CancelCause::Deadline);
            work()
        }
        Some(milliseconds) => std::thread::scope(|scope| {
            let (done, finished) = std::sync::mpsc::channel::<()>();
            let timer = cancel.clone();
            scope.spawn(move || {
                let wait = std::time::Duration::from_millis(milliseconds);
                if let Err(std::sync::mpsc::RecvTimeoutError::Timeout) = finished.recv_timeout(wait)
                {
                    timer.cancel(CancelCause::Deadline);
                }
            });
            let value = work();
            // The timer thread ends on this message or on its timeout; a
            // closed channel also ends it.
            let _ = done.send(());
            value
        }),
    }
}

/// FR-850 "Admission": a request that selects `package` is read by the strict
/// checked-package reader against the supplied domain packages, and refuses
/// before any item resolves when a compile member sits beside it, the reader
/// refuses the bytes, a recorded domain package is not supplied, or a
/// supplied one is not recorded.
fn admit_package<'d>(
    directory: &'d Path,
    dependent_bytes: usize,
    request: &wire::AnalyzeRequest,
    package: &wire::HandoffSelection,
) -> Result<(AdmittedHandoff, super::Intake<'d>)> {
    if request.program.is_some() {
        return Err(invalid(
            "program",
            "a program cannot be selected beside a package",
        ));
    }
    if !request.libraries.is_empty() {
        return Err(invalid(
            "libraries",
            "a package already holds its dependency selections",
        ));
    }
    if request.refinement_internals.is_some() {
        return Err(invalid(
            "refinement_internals",
            "a package is already compiled",
        ));
    }
    if request.integer_division.is_some() {
        return Err(invalid("integer_division", "a package is already compiled"));
    }
    if !request.clocks.is_empty() {
        return Err(invalid("clocks", "a package already records its clocks"));
    }
    let mut intake = intake_with_bytes(
        directory,
        FileCounts {
            // The handoff's package, identity and equality files.
            programs: 2,
            models: request.models.len(),
            observations: request.documents.len(),
            extensions: 0,
        },
        dependent_bytes,
    )?;
    let handoff = handoff_directory(&package.directory)?;
    // FR-850 "Admission": an absent handoff directory or package file is a
    // package refusal, not an I/O failure.
    if !directory.join(&handoff).is_dir() {
        return Err(witness_refusal(
            &handoff,
            "missing-file",
            "the request names no handoff directory there",
        ));
    }
    let bytes = super::handoff::read_file(&mut intake, &handoff, PACKAGE_FILE, false)?;
    if request
        .models
        .iter()
        .any(|model| !super::is_model_input_format(model))
    {
        return Err(RunCause::CompleteSelection(
            super::CompleteSelection::NativeModels,
        ));
    }
    let documents = request
        .models
        .iter()
        .enumerate()
        .map(|(index, model)| intake.source(&model.source, &format!("models[{index}].source")))
        .collect::<Result<Vec<_>>>()?;
    let supplied = super::package_input_from_selected_models(&request.models, &documents).map_err(
        |cause| match cause {
            RunCause::ClauseRequest { member, reason } if member == "models" => {
                RunCause::AnalyzeRequest { member, reason }
            }
            cause => cause,
        },
    )?;
    let limits =
        fsl_package::V2ReadLimits::bounded(fsl_replay::spine::I2_NATIVE_JOIN_STORAGE_DEFAULT);
    let loaded = super::handoff::admit(&mut intake, &handoff, bytes, &supplied, limits)
        .map_err(super::handoff::AdmissionFailure::for_analyze)?;
    Ok((
        AdmittedHandoff {
            package: loaded.package,
            identities: loaded.identities,
            equality: loaded.equality,
            budget: loaded.budget,
            supplied,
            package_bytes: loaded.bytes,
            identity_bytes: loaded.identity_bytes,
            equality_bytes: loaded.equality_bytes,
            dependencies: loaded.dependencies,
        },
        intake,
    ))
}

/// The FR-517 files a `package` selection reads from its handoff directory.
const PACKAGE_FILE: &str = "checked-package.json";

/// The admitted handoff of a `package` request: the package the strict
/// reader admitted, its identity rows and its equality rows when present.
struct AdmittedHandoff {
    dependencies: Vec<fsl_package::DependencyArtifact>,
    package: filament_contract_model::CheckedPackageV2,
    identities: fsl_package::ReadIdentityRows,
    equality: Option<fsl_package::ReadEqualityRows>,
    budget: filament_contract_model::NativeJoinBudget,
    /// The request's domain packages, bound to the package's model
    /// selections.
    supplied: fsl_semantics::model::intake::PackageInput,
    /// The admitted package's bytes, which replay reads (FR-850).
    package_bytes: Vec<u8>,
    /// The handoff's identity file bytes, which replay reads.
    identity_bytes: Vec<u8>,
    /// The handoff's equality file bytes, when present.
    equality_bytes: Option<Vec<u8>>,
}

/// The handoff directory a `package` selection names, beneath the request's
/// directory.
fn handoff_directory(directory: &str) -> Result<String> {
    use std::path::Component;
    let path = Path::new(directory);
    if directory.is_empty()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(invalid(
            "package.directory",
            "must name a directory beneath the request directory",
        ));
    }
    Ok(directory.trim_end_matches('/').to_owned())
}

/// FR-850 "Admission": a handoff witness file that is absent or refused.
fn witness_refusal(file: &str, cause: &str, detail: &str) -> RunCause {
    RunCause::AnalyzePackage {
        code: fsl_foundation::Code::InvalidPackage,
        cause: Some(cause.to_owned()),
        pointer: None,
        message: format!("invalid_package/{cause}: {file}: {detail}"),
    }
}

/// FSL FR-1033 "Reading `memory`": each entry is a closed object with exactly
/// `parallel` and `model`, the model `sc`, `tso` or `ra`; a malformed one
/// refuses the whole request naming the member by its pointer.
fn read_memory(
    entries: &[serde_json::Value],
) -> Result<Vec<fsl_eval::simulation::protocol::MemorySelection>> {
    use fsl_foundation::MemoryModelName;

    entries
        .iter()
        .enumerate()
        .map(|(position, entry)| {
            let Some(object) = entry.as_object() else {
                return Err(invalid(
                    &format!("/memory/{position}"),
                    "a memory entry is an object",
                ));
            };
            if let Some(member) = object
                .keys()
                .find(|member| !matches!(member.as_str(), "parallel" | "model"))
            {
                return Err(invalid(
                    &format!("/memory/{position}/{member}"),
                    "a memory entry holds only parallel and model",
                ));
            }
            let parallel = object
                .get("parallel")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    invalid(
                        &format!("/memory/{position}/parallel"),
                        "a memory entry names its parallel as a string",
                    )
                })?;
            let model = match object.get("model").and_then(serde_json::Value::as_str) {
                Some("sc") => MemoryModelName::Sc,
                Some("tso") => MemoryModelName::Tso,
                Some("ra") => MemoryModelName::Ra,
                _ => {
                    return Err(invalid(
                        &format!("/memory/{position}/model"),
                        "a memory model is sc, tso or ra",
                    ));
                }
            };
            Ok(fsl_eval::simulation::protocol::MemorySelection {
                parallel: parallel.to_owned(),
                model,
            })
        })
        .collect()
}

/// FSL FR-1033 "Reading `memory`": an entry naming no outermost `parallel`
/// of a protocol of the package (a nested `parallel`, an attempt, a protocol
/// the package does not declare), and two entries naming one `parallel`,
/// refuse the request before any item runs.
fn check_memory(
    package: &fsl_package::CheckedPackage,
    memory: &[fsl_eval::simulation::protocol::MemorySelection],
) -> Result<()> {
    check_memory_using(memory, |protocol, entries| {
        package
            .graph()
            .protocol(protocol)
            .map(|checked| ResolvedMemory::resolve(checked, entries))
    })
}

/// Admit every request memory entry before resolving any item, using the
/// selected route's protocol facts and the same refusal pointers.
fn check_memory_using(
    memory: &[MemorySelection],
    resolve: impl Fn(
        &str,
        &[MemorySelection],
    ) -> Option<std::result::Result<ResolvedMemory, MemorySelectionRefusal>>,
) -> Result<()> {
    let refused = |member: String, reason: String| RunCause::AnalyzeMemory { member, reason };
    let mut protocols: Vec<&str> = Vec::new();
    for (position, entry) in memory.iter().enumerate() {
        let protocol = entry.parallel.split("::").next().unwrap_or_default();
        if resolve(protocol, &[]).is_none() {
            return Err(refused(
                format!("/memory/{position}/parallel"),
                format!("{} names no protocol of the package", entry.parallel),
            ));
        }
        if !protocols.contains(&protocol) {
            protocols.push(protocol);
        }
    }
    for protocol in protocols {
        let entries: Vec<_> = memory
            .iter()
            .filter(|entry| entry.parallel.split("::").next() == Some(protocol))
            .cloned()
            .collect();
        if let Some(Err(refusal)) = resolve(protocol, &entries) {
            let member = memory
                .iter()
                .position(|entry| entries.first().is_some_and(|first| first == entry))
                .map_or_else(
                    || "/memory".to_owned(),
                    |position| format!("/memory/{position}"),
                );
            return Err(refused(member, refusal.to_string()));
        }
    }
    Ok(())
}

/// The members FR-620 reads beside the program or package selection: the
/// supplied documents under their own identities, the request's default
/// model subject and the limits every item of the run shares.
struct RunMembers {
    documents: DocumentProvision,
    subject: SubjectSelection,
    limits: AnalyzeLimits,
}

/// Read `request`'s [`RunMembers`] through `intake`, refusing a malformed
/// or unprovisioned document, trace or recording and an incomplete
/// universe as FR-620 states.
fn read_run_members(
    request: &wire::AnalyzeRequest,
    intake: &mut super::Intake<'_>,
    replay: fsl_replay::ReplayLimits,
) -> Result<RunMembers> {
    let documents = read_documents(intake, &request.documents, request.items.as_deref())?;
    let mut limits = analyze_limits(&request.limits, request.deadline_ms);
    limits.replay = replay;
    let subject = read_subject(request.subject.clone(), &documents)?;
    Ok(RunMembers {
        documents,
        subject,
        limits,
    })
}

/// FR-620's item checks that need no compiled program: `recording` and
/// `population` belong to a global-conformance item, and FR-634's
/// `simulation` member to a simulation item alone.
pub(super) fn validate_items(items: Option<&[ItemSelection]>) -> Result<()> {
    fsl_analyze::analyze::document::effect_decision::selections(items.unwrap_or_default())
        .map_err(|error| invalid(&error.pointer, error.message))?;
    for (position, item) in items.unwrap_or_default().iter().enumerate() {
        if item.kind.as_deref() == Some(fsl_semantics::check::Capability::Composition.to_wire()) {
            if let Some(premises) = &item.premises {
                validate_composition_premises(position, premises)
                    .map_err(|(pointer, reason)| invalid(&pointer, reason))?;
            }
        }
        if item.recording.is_some()
            && item.kind.as_deref()
                != Some(fsl_semantics::check::Capability::GlobalConformance.to_wire())
        {
            return Err(invalid(
                &format!("/items/{position}/recording"),
                "recording is only valid on a global-conformance item",
            ));
        }
        if let Some(population) = &item.population {
            if item.kind.as_deref()
                != Some(fsl_semantics::check::Capability::GlobalConformance.to_wire())
            {
                return Err(invalid(
                    &format!("/items/{position}/population"),
                    "population is only valid on a global-conformance item",
                ));
            }
            if item.recording.is_none() {
                return Err(invalid(
                    &format!("/items/{position}/population"),
                    "population requires a recording",
                ));
            }
            validate_activated_population(position, population)
                .map_err(|(pointer, reason)| invalid(&pointer, reason))?;
            serde_json::from_value::<Vec<fsl_analyze::analyze::document::ActivatedRolePopulation>>(
                population.clone(),
            )
            .map_err(|error| {
                invalid(&format!("/items/{position}/population"), error.to_string())
            })?;
        }
    }
    // FR-634: a `simulation` item names no claim and carries its
    // `simulation` member, and only it does; `seed` and `samples` belong to
    // `sample` only.
    for (position, item) in items.unwrap_or_default().iter().enumerate() {
        let simulation =
            item.kind.as_deref() == Some(fsl_analyze::analyze::document::SIMULATION_KIND);
        match (&item.simulation, simulation) {
            (Some(_), false) => {
                return Err(invalid(
                    &format!("/items/{position}/simulation"),
                    "simulation is only valid on a simulation item",
                ));
            }
            (None, true) => {
                return Err(invalid(
                    &format!("/items/{position}/simulation"),
                    "a simulation item carries its simulation member",
                ));
            }
            (Some(settings), true) => {
                if item.claim.is_some() {
                    return Err(invalid(
                        &format!("/items/{position}/claim"),
                        "a simulation item settles no claim",
                    ));
                }
                if let Some(member) = settings.misplaced_member() {
                    return Err(invalid(
                        &format!("/items/{position}/simulation/{member}"),
                        format!("{member} belongs to a sample simulation only"),
                    ));
                }
            }
            (None, false) => {}
        }
    }
    Ok(())
}

/// Read the supplied `documents` through `intake`, each under its declared
/// or stated identity (FR-821), and refuse an item `trace` or `recording`
/// naming no provisioned document (FR-636).
pub(super) fn read_documents(
    intake: &mut super::Intake<'_>,
    selections: &[wire::DocumentFileSelection],
    items: Option<&[ItemSelection]>,
) -> Result<DocumentProvision> {
    let mut documents = DocumentProvision::new();
    let mut recording_documents = std::collections::BTreeSet::new();
    let selected =
        fsl_analyze::analyze::document::effect_decision::selections(items.unwrap_or_default())
            .map_err(|error| invalid(&error.pointer, error.message))?;
    let mut receipt_documents = std::collections::BTreeMap::new();
    for (item, entries) in selected.iter().enumerate() {
        for (position, reference) in entries.iter().enumerate() {
            receipt_documents
                .entry(reference)
                .or_insert((item, position));
        }
    }
    for (position, document) in selections.iter().enumerate() {
        let bytes = intake.file(&document.file, intake.remaining)?;
        // Receipt identities are provision metadata, so even malformed JSON
        // reaches the receipt reader without an uncharged second file read.
        let wrapper = document.identity.as_ref().and_then(|value| {
            serde_json::from_value::<fsl_analyze::analyze::document::DocumentSelection>(
                value.clone(),
            )
            .ok()
        });
        if let Some((reference, (item, selector))) = wrapper
            .as_ref()
            .and_then(|reference| receipt_documents.get_key_value(reference))
        {
            fsl_analyze::analyze::document::effect_decision::validate(
                &bytes, *item, *selector, reference,
            )
            .map_err(receipt_failure)?;
        }
        let (format, declared_identity) = if let Some(arena) =
            fsl_analyze::analyze::document::effect_decision::receipt_document(&bytes)
                .map_err(receipt_failure)?
        {
            // Opaque receipt objects never enter a recursive Value parser,
            // even when no item selected this provisioned receipt.
            let root = arena.root();
            let format = root.get("format").and_then(|node| match node.node() {
                quire_canonical::Node::String(value) => Some(value.to_owned()),
                _ => None,
            });
            let identity = root
                .get("identity")
                .map(|node| receipt_identity(node, &format!("/documents/{position}/identity")))
                .transpose()?;
            (format, identity)
        } else {
            let parsed: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
                invalid(
                    &format!("/documents/{position}"),
                    format!("{} is not a JSON document: {error}", document.file),
                )
            })?;
            let format = parsed
                .get("format")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let identity = parsed
                .get("identity")
                .map(|value| {
                    serde_json::from_value::<fsl_analyze::analyze::document::DocumentSelection>(
                        value.clone(),
                    )
                })
                .transpose()
                .map_err(|error| {
                    invalid(
                        &format!("/documents/{position}/identity"),
                        format!("document identity is malformed: {error}"),
                    )
                })?;
            (format, identity)
        };
        let stated_identity = match &document.identity {
            Some(value) => Some(
                serde_json::from_value::<fsl_analyze::analyze::document::DocumentSelection>(
                    value.clone(),
                )
                .map_err(|error| {
                    invalid(
                        &format!("/documents/{position}/identity"),
                        format!("document identity is malformed: {error}"),
                    )
                })?,
            ),
            None => None,
        };
        if format.as_deref() == Some("filament.state.snapshot/v1") && document.identity.is_some() {
            return Err(invalid(
                &format!("/documents/{position}/identity"),
                "a supplied snapshot uses its declared identity and cannot state a wrapper identity",
            ));
        }
        if let (Some(declared), Some(stated)) = (&declared_identity, &stated_identity) {
            if declared != stated {
                return Err(invalid(
                    &format!("/documents/{position}/identity"),
                    format!(
                        "stated identity {{{}, {}}} contradicts declared identity {{{}, {}}}",
                        stated.authority, stated.identity, declared.authority, declared.identity
                    ),
                ));
            }
        }
        let Some(identity) = declared_identity.as_ref().or(stated_identity.as_ref()) else {
            return Err(invalid(
                &format!("/documents/{position}"),
                format!("{} declares no identity and none was stated", document.file),
            ));
        };
        let reference = fsl_semantics::model::observation::DocumentRef {
            authority: identity.authority.clone(),
            identity: identity.identity.clone(),
        };
        if format.as_deref() == Some("filament.protocol-recording/1") {
            recording_documents.insert((reference.authority.clone(), reference.identity.clone()));
        }
        if documents.insert(reference, bytes).is_some() {
            return Err(invalid(
                &format!("/documents/{position}"),
                format!("{} repeats a provisioned document identity", document.file),
            ));
        }
    }
    // FR-636: a `trace` naming no provisioned document refuses the request.
    for (position, trace) in items
        .unwrap_or_default()
        .iter()
        .enumerate()
        .filter_map(|(position, item)| item.trace.as_ref().map(|trace| (position, trace)))
    {
        if !documents.contains_key(&trace.reference()) {
            return Err(invalid(
                "items",
                format!(
                    "items[{position}].trace {{{}, {}}} names no provisioned document",
                    trace.authority, trace.identity
                ),
            ));
        }
    }
    for (position, recording) in
        items
            .unwrap_or_default()
            .iter()
            .enumerate()
            .filter_map(|(position, item)| {
                item.recording
                    .as_ref()
                    .map(|recording| (position, recording))
            })
    {
        let identity = recording.reference();
        if !recording_documents.contains(&(identity.authority.clone(), identity.identity.clone())) {
            return Err(invalid(
                &format!("/items/{position}/recording"),
                format!(
                    "recording {{{}, {}}} does not name a provisioned protocol recording",
                    recording.authority, recording.identity,
                ),
            ));
        }
    }
    for (reference, (item, selector)) in receipt_documents {
        if !documents.contains_key(&reference.reference()) {
            return Err(invalid(
                &format!("/items/{item}/effect_decisions/{selector}"),
                "effect decision names no provisioned document",
            ));
        }
    }
    Ok(documents)
}

/// The limits every item of the run shares: FR-620's `limits` over the
/// unbounded defaults, and the run's deadline.
pub(super) fn analyze_limits(
    request: &wire::AnalyzeLimits,
    deadline_ms: Option<u64>,
) -> AnalyzeLimits {
    let mut limits = AnalyzeLimits::unbounded();
    let mut accounting = fsl_replay::AccountingLimits(limits.accounting);
    for (setting, bound) in request.accounting.entries() {
        accounting.set_bound(setting, bound);
    }
    limits.accounting = accounting.0;
    limits.observation = request.observation.resolve();
    limits.normalization = request.normalization.resolve();
    limits.deadline = deadline_ms.map_or(DeadlineSupply::Unbounded, DeadlineSupply::Milliseconds);
    limits
}

/// The request's model subject: `subject` completed by the default subject
/// over `documents`, or that default.
pub(super) fn read_subject(
    subject: Option<SubjectSelection>,
    documents: &DocumentProvision,
) -> Result<SubjectSelection> {
    // FR-620: an explicit `subject.universes` refuses an unprovisioned or
    // incomplete reference; the default subject's universes are the complete
    // populations its snapshots declare, so it refuses neither.
    let default_subject = SubjectSelection::default_over(documents);
    Ok(match subject {
        // A subject that states only its `protocol` keeps the default model
        // parts (FSL FR-1033).
        Some(subject) => {
            let subject = subject.completed_by(&default_subject);
            subject
                .check_universes(documents)
                .map_err(RunCause::AnalyzeUniverse)?;
            subject
        }
        None => default_subject,
    })
}
