// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real source and host inputs that reach stable diagnostic producers.

use filament_exact::{Cancel, CancelCause};
use filament_spec_language::complete::{self, DocumentBinding, SourceEdit};
use fsl_cst::{CompleteCause, CompleteCode, CompleteDiagnostic, HostCause, Limits, ParsedSource};
use fsl_foundation::selection::ProfileCatalog;
use fsl_foundation::source::SourceLabel;
use fsl_foundation::{Phase, SourceIdentity, Span};
use ix_trace_rs::trace;

const DOCUMENTATION: &str = include_str!("../../docs/native-error-codes.md");
const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile p = \"filament.value.complete/v1\";\n";

fn identity() -> SourceIdentity {
    SourceIdentity::new("producer-tests", "original:diagnostics")
}
fn source(body: &str) -> String {
    format!("{HEADER}// durée\n{body}")
}
fn parsed(text: &str) -> ParsedSource {
    fsl_cst::parse(
        identity(),
        "producer.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("readable source")
}
fn refused(text: &[u8], limits: Limits) -> CompleteDiagnostic {
    match fsl_cst::parse(identity(), "producer.native", text, limits) {
        Err(diagnostic) => *diagnostic,
        Ok(parsed) => parsed
            .diagnostics()
            .first()
            .expect("input must be refused")
            .clone(),
    }
}
fn produced(
    diagnostic: &CompleteDiagnostic,
    code: CompleteCode,
    cause: CompleteCause,
    phase: Phase,
) {
    assert_eq!(
        (diagnostic.code, diagnostic.cause, diagnostic.phase),
        (code, cause, phase),
        "{diagnostic:?}"
    );
    let prefix = format!("| {} |", code.as_str());
    let row = DOCUMENTATION
        .lines()
        .find(|line| line.starts_with(&prefix))
        .expect("an actually produced code must be documented");
    assert!(!row
        .strip_prefix(&prefix)
        .expect("selected row")
        .trim()
        .trim_matches('|')
        .trim()
        .is_empty());
    assert_eq!(diagnostic.source, identity());
    assert_eq!(diagnostic.path, "producer.native");
}
fn editor_source() -> ParsedSource {
    let text = source("function bill using p(items: Integer): Integer pure { items + 4 }");
    let value = parsed(&text);
    assert!(value.is_admissible(), "{:?}", value.diagnostics());
    value
}
fn catalog() -> ProfileCatalog {
    ProfileCatalog::new(["filament.value.complete/v1"]).expect("valid exact profile")
}
fn binding() -> DocumentBinding {
    DocumentBinding {
        source: identity(),
        profile: "filament.value.complete/v1".into(),
    }
}

/// Provenance: QSL-2079; grammar token/end and string-escape producers.
#[test]
#[trace("FR-002-AC-6", "FSpec-FR-339-AC-4")]
fn source_refusals_reach_their_typed_producers_and_documented_rows() {
    for (body, cause, phase, token) in [
        (
            "record Crate { amount: Integer }",
            CompleteCause::UnexpectedToken,
            Phase::Parse,
            "}",
        ),
        (
            "record Crate { amount: Integer;",
            CompleteCause::UnexpectedEnd,
            Phase::Parse,
            "",
        ),
    ] {
        let text = source(body);
        let diagnostic = refused(text.as_bytes(), Limits::default());
        produced(&diagnostic, CompleteCode::InvalidSyntax, cause, phase);
        let span = diagnostic.byte_span().expect("source refusal is located");
        if token.is_empty() {
            assert_eq!(
                span,
                Span {
                    start: text.len(),
                    end: text.len()
                }
            );
        } else {
            assert!(text[span.start..span.end].contains(token), "{diagnostic:?}");
        }
    }
}

#[test]
#[trace("FR-354-AC-1")]
fn an_unselected_declaration_form_is_a_real_unsupported_construct() {
    let text = source("invented_coating Shield { true }");
    let diagnostic = refused(text.as_bytes(), Limits::default());
    produced(
        &diagnostic,
        CompleteCode::UnsupportedConstruct,
        CompleteCause::DeclarationForm,
        Phase::Parse,
    );
    let start = text.find("invented_coating").expect("authored head");
    assert_eq!(
        diagnostic.byte_span(),
        Some(Span {
            start,
            end: start + "invented_coating".len()
        })
    );
}

#[test]
#[trace("FR-762-AC-6")]
#[ignore = "QSL-2688: the syntax diagnostic cut must name s1.diagnostics"]
fn the_diagnostic_limit_is_reached_by_independent_source_errors() {
    let declarations = (0..101)
        .map(|index| format!("record Item{index} {{ amount: ; }}\n"))
        .collect::<String>();
    let text = source(&declarations);
    let value = fsl_cst::parse(
        identity(),
        "producer.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("readable");
    assert_eq!(value.diagnostics().len(), 101);
    let diagnostic = value.diagnostics().last().expect("bounded cut diagnostic");
    produced(
        diagnostic,
        CompleteCode::InvalidSyntax,
        CompleteCause::DiagnosticListCut,
        Phase::Parse,
    );
    assert_eq!(
        diagnostic.byte_span().expect("cut location").start,
        text.rfind(';').expect("second field")
    );
    assert!(diagnostic.message.contains("s1.diagnostics"));
}

#[test]
#[trace("FR-001-AC-11")]
fn source_preflight_inputs_reach_retained_host_causes_before_parsing() {
    for (offered, path, bytes, cause, code) in [
        (
            SourceIdentity::new(" ", "named"),
            "producer.native",
            b"bad".as_slice(),
            HostCause::BlankLabel {
                label: SourceLabel::Authority,
            },
            CompleteCode::InvalidSourceIdentity,
        ),
        (
            identity(),
            "",
            b"bad".as_slice(),
            HostCause::EmptyPath,
            CompleteCode::InvalidSourceIdentity,
        ),
        (
            identity(),
            "producer.native",
            &[0xff],
            HostCause::InvalidUtf8,
            CompleteCode::InvalidUtf8,
        ),
    ] {
        let diagnostic = fsl_cst::parse(offered.clone(), path, bytes, Limits::default())
            .expect_err("preflight rejects input");
        assert_eq!(diagnostic.code, code);
        assert_eq!(diagnostic.cause, CompleteCause::Host(cause));
        assert_eq!(diagnostic.phase, Phase::Source);
        assert_eq!(diagnostic.source, offered);
        assert_eq!(diagnostic.path, path);
        assert!(DOCUMENTATION.contains(&format!("| {} |", code.as_str())));
    }
}

#[test]
#[trace("FR-001-AC-12")]
fn real_host_edits_reach_predecessor_and_range_causes() {
    let value = editor_source();
    let original = value.source().text().as_bytes().to_vec();
    let wrong = complete::apply_edits(
        &value,
        SourceIdentity::new("other", "foreign"),
        &[],
        Limits::default(),
    )
    .expect_err("wrong immutable predecessor");
    produced(
        &wrong,
        CompleteCode::InvalidSourceMap,
        CompleteCause::Host(HostCause::EditPredecessor),
        Phase::SourceMap,
    );
    assert!(wrong.region.is_none());
    let split = value.source().text().find('é').expect("multibyte source") + 1;
    let malformed = SourceEdit {
        range: Span {
            start: split,
            end: split,
        },
        replacement: " ".into(),
    };
    let wrong = complete::apply_edits(&value, identity(), &[malformed], Limits::default())
        .expect_err("edit splits UTF-8");
    produced(
        &wrong,
        CompleteCode::InvalidSourceMap,
        CompleteCause::Host(HostCause::EditRanges),
        Phase::SourceMap,
    );
    assert_eq!(value.source().text().as_bytes(), original);
}

#[test]
#[trace("FR-001-AC-12")]
fn real_editor_requests_reach_revision_profile_and_cancellation_producers() {
    let value = editor_source();
    let supported = catalog();
    let mut offered = binding();
    offered.source = SourceIdentity::new("other", "document");
    let wrong = complete::analyze_document(&value, offered, false, &supported)
        .expect_err("foreign request identity");
    produced(
        &wrong,
        CompleteCode::InvalidSourceMap,
        CompleteCause::Host(HostCause::RequestRevision),
        Phase::SourceMap,
    );
    assert!(wrong.region.is_none());
    let mut offered = binding();
    offered.profile = "filament.value.future/v99".into();
    let wrong = complete::analyze_document(&value, offered, false, &supported)
        .expect_err("unselected profile");
    produced(
        &wrong,
        CompleteCode::UnknownProfile,
        CompleteCause::UnsupportedSelection,
        Phase::Profile,
    );
    let wrong = complete::analyze_document(&value, binding(), true, &supported)
        .expect_err("cancelled host request");
    produced(
        &wrong,
        CompleteCode::Cancelled,
        CompleteCause::CallerCancelled,
        Phase::Check,
    );
}

#[test]
#[trace("FR-276-AC-1")]
fn a_cancelled_real_parse_stops_at_its_public_entry() {
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let text = source("function total using p(): Integer pure { 5 }");
    let offered = identity();
    let result = fsl_replay::spine::parse(
        &fsl_replay::spine::ParseRequest {
            source: &offered,
            path: "producer.native",
            bytes: text.as_bytes(),
        },
        Limits::default(),
        &cancel,
    );
    assert!(
        matches!(
            &result,
            Err(fsl_foundation::diagnostic::StageFailure::Cancelled(
                CancelCause::Requested
            ))
        ),
        "{result:?}"
    );
    let document = fsl_replay::OutcomeDocument::from_parse(&result);
    let outcome: serde_json::Value =
        serde_json::from_slice(&document.to_bytes().expect("public cancellation outcome"))
            .expect("JSON");
    assert_eq!(outcome["category"], "incomplete");
    assert!(outcome["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .any(|diagnostic| diagnostic["code"] == CompleteCode::Cancelled.as_str()));
}

#[test]
#[ignore = "QSL-2687: malformed escapes must not produce unrelated selection-identity refusals"]
fn an_invalid_escape_reaches_its_lexical_producer_before_selection_cascades() {
    let text = source("enum Caption { Label = \"bad\\q\" }");
    let diagnostic = refused(text.as_bytes(), Limits::default());
    produced(
        &diagnostic,
        CompleteCode::InvalidSyntax,
        CompleteCause::InvalidEscape,
        Phase::Lex,
    );
    let span = diagnostic
        .byte_span()
        .expect("the actual string is located");
    assert!(text[span.start..span.end].contains("\\q"));
}

#[test]
#[trace("FSpec-FR-302-AC-2")]
fn a_real_node_from_another_parse_reaches_the_foreign_node_host_cause() {
    let value = editor_source();
    let other = fsl_cst::parse(
        SourceIdentity::new("other", "original:node"),
        "other.native",
        source("record Abroad { item: Boolean; }").as_bytes(),
        Limits::default(),
    )
    .expect("readable");
    assert!(other.is_admissible());
    let node = other
        .cst()
        .nodes()
        .iter()
        .find(|node| node.production() == fsl_cst::Production::RecordDeclaration)
        .expect("real foreign declaration");
    let diagnostic = value
        .cst()
        .render_node(node)
        .expect_err("foreign node cannot render here");
    produced(
        &diagnostic,
        CompleteCode::InvalidSourceMap,
        CompleteCause::Host(HostCause::ForeignNode),
        Phase::SourceMap,
    );
    assert!(diagnostic.region.is_none());
    assert_eq!(
        other.cst().render_node(node).expect("own node renders"),
        b"record Abroad { item: Boolean; }"
    );
}

/// Provenance: QSL-2079; real layer-3 resolution, not synthetic CompleteCause conversion.
#[test]
fn exact_definition_inputs_reach_bundle_refusal_producers() {
    use fsl_foundation::selection::DefinitionRef;
    use fsl_semantics::library::bundle::{
        link_bundle, DefinitionCatalog, ExtensionDefinitionSource, PackageLimits, ResolutionCause,
    };
    use std::collections::BTreeSet;
    let first = DefinitionRef::new("producer.example", "extensions/one").expect("exact selection");
    let second = DefinitionRef::new("producer.example", "extensions/two").expect("exact selection");
    let definition = |reference: &DefinitionRef, dependencies: BTreeSet<DefinitionRef>| {
        ExtensionDefinitionSource {
            authority: reference.authority().to_owned(),
            identity: reference.identity().to_owned(),
            exact_bytes: b"{}".to_vec(),
            dependencies,
            capabilities: BTreeSet::new(),
        }
    };
    let absent_dependency = DefinitionCatalog::read_extensions(vec![definition(
        &first,
        BTreeSet::from([second.clone()]),
    )])
    .expect("real selected root with an unresolved dependency");
    let missing = link_bundle(
        std::slice::from_ref(&first),
        &absent_dependency,
        PackageLimits::default(),
    )
    .expect_err("missing real dependency");
    assert_eq!(
        (missing.code, missing.cause_tag),
        (
            CompleteCode::MissingImport,
            ResolutionCause::MissingSelection
        )
    );
    let mut facet_only = definition(&first, BTreeSet::new());
    facet_only.capabilities = fsl_semantics::library::bundle::CapabilityId::complete_inventory()
        .into_iter()
        .collect();
    let catalog = DefinitionCatalog::read_extensions(vec![facet_only])
        .expect("the public object reader admits exact extension input");
    let incomplete = link_bundle(
        std::slice::from_ref(&first),
        &catalog,
        PackageLimits::default(),
    )
    .expect_err("an extension alone does not supply complete facets");
    assert_eq!(
        (incomplete.code, incomplete.cause_tag),
        (
            CompleteCode::InvalidPackage,
            ResolutionCause::FeatureSetMismatch
        )
    );
    let cyclic = DefinitionCatalog::read_extensions(vec![
        definition(&first, BTreeSet::from([second.clone()])),
        definition(&second, BTreeSet::from([first.clone()])),
    ])
    .expect("graph input is admitted before closure resolution");
    let cycle = link_bundle(
        std::slice::from_ref(&first),
        &cyclic,
        PackageLimits::default(),
    )
    .expect_err("real dependency cycle");
    assert_eq!(
        (cycle.code, cycle.cause_tag),
        (
            CompleteCode::InvalidPackage,
            ResolutionCause::DefinitionCycle
        )
    );
    for refusal in [missing, incomplete, cycle] {
        assert!(DOCUMENTATION.contains(&format!("| {} |", refusal.code.as_str())));
    }
}

#[test]
#[trace("FR-010-AC-7")]
fn public_compile_intake_inputs_reach_io_json_and_wire_codes() {
    use fsl_command::command;
    let directory = tempfile::tempdir().expect("owned temporary input directory");
    let missing =
        command::compile(&directory.path().join("absent-job.json")).expect_err("missing real file");
    assert_eq!(missing.cause.code(), CompleteCode::IoError);
    assert!(DOCUMENTATION.contains("| io-error |"));
    for (name, bytes, code) in [
        ("broken.json", b"{".as_slice(), CompleteCode::InvalidRequest),
        (
            "wire.json",
            br#"{"format":"producer.future/99","request":{}}"#.as_slice(),
            CompleteCode::UnknownWire,
        ),
    ] {
        let path = directory.path().join(name);
        std::fs::write(&path, bytes).expect("original request bytes");
        let refusal = command::compile(&path).expect_err("invalid public request");
        assert_eq!(refusal.cause.code(), code, "{refusal:?}");
        assert!(DOCUMENTATION.contains(&format!("| {} |", code.as_str())));
    }
}

fn compile_original(body: &str) -> Result<Vec<u8>, Box<fsl_command::command::RunError>> {
    compile_text(&source(body))
}

fn compile_text(text: &str) -> Result<Vec<u8>, Box<fsl_command::command::RunError>> {
    let directory = tempfile::tempdir().expect("owned compile input");
    std::fs::write(directory.path().join("producer.fsl"), text).expect("original source");
    let request = serde_json::json!({"format":"native-compile/1","request":{"models":[],"program":{"source":{"file":"producer.fsl","authority":"producer-tests","identity":"original:diagnostics","document":"Producer","formal_revision":1}}}});
    std::fs::write(
        directory.path().join("job.json"),
        serde_json::to_vec(&request).expect("request encodes"),
    )
    .expect("original request");
    fsl_command::command::compile(&directory.path().join("job.json"))
}

#[test]
#[trace("FR-286-AC-2")]
fn public_compile_semantic_inputs_keep_missing_typing_and_definedness_codes() {
    for (body, code) in [
        (
            "function bill using p(): Integer pure { missing_amount }",
            CompleteCode::MissingDeclaration,
        ),
        (
            "function bill using p(): Integer pure { \"not a number\" }",
            CompleteCode::IllTyped,
        ),
        (
            "function shifted using p(items: Int[0, 8]): Int[0, 8] pure { items + 2 }",
            CompleteCode::UndefinedExpression,
        ),
    ] {
        let refusal = compile_original(body).expect_err("semantic defect refuses before emission");
        assert_eq!(refusal.cause.code(), code, "{body}: {refusal:?}");
        let fsl_command::command::RunCause::Spine(failure) = &refusal.cause else {
            panic!("expected a real compiler-stage refusal: {refusal:?}");
        };
        let stage: fsl_replay::OutcomeStage = failure.refusal.stage().into();
        assert_eq!(stage, fsl_replay::OutcomeStage::S3);
        assert!(failure.span.is_some(), "the original source is located");
        assert_eq!(failure.source, identity());
        assert!(failure.path.ends_with("producer.fsl"));
        assert!(DOCUMENTATION.contains(&format!("| {} |", code.as_str())));
    }
}

#[test]
#[trace("FR-056-AC-9")]
fn actual_model_document_inputs_reach_malformed_and_noncanonical_codes() {
    use fsl_foundation::IntakeLimits;
    use fsl_semantics::model::intake::PackageDocument;
    let original = crate::support::spine_model::DOCUMENT;
    PackageDocument::parse(original, IntakeLimits::default())
        .expect("authoritative same-repository model document is valid");
    let malformed = PackageDocument::parse(b"{", IntakeLimits::default())
        .expect_err("actual malformed document");
    assert_eq!(malformed.code, CompleteCode::InvalidModelBinding);
    let mut document: serde_json::Value =
        serde_json::from_slice(original).expect("original document JSON");
    *document
        .pointer_mut("/types/0/fields/0/multiplicity/lower")
        .expect("existing numeric model member") = serde_json::json!(9_007_199_254_740_993_u64);
    let bytes = serde_json::to_vec(&document).expect("caller bytes encode the offered integer");
    let inexact = PackageDocument::parse(&bytes, IntakeLimits::default())
        .expect_err("integer lacks an exact RFC8785 spelling");
    assert_eq!(inexact.code, CompleteCode::NoncanonicalWire, "{inexact:?}");
    for code in [malformed.code, inexact.code] {
        assert!(DOCUMENTATION.contains(&format!("| {} |", code.as_str())));
    }
}

#[test]
fn source_selection_inputs_reach_language_and_identifier_codes() {
    let text =
        "language \"producer:unknown\" edition \"1-draft\";\nrecord Boxed { amount: Integer; }";
    let refusal = compile_text(text).expect_err("unsupported compile language selection");
    assert_eq!(
        refusal.cause.code(),
        CompleteCode::UnknownLanguage,
        "{refusal:?}"
    );
    assert_eq!(refusal.category().exit_code(), 20);
    assert!(DOCUMENTATION.contains("| unknown_language |"));
    let empty = "language \"ix:native\" edition \"1-draft\";\nprofile p = \"\";\nrecord Boxed { amount: Integer; }";
    let diagnostic = refused(empty.as_bytes(), Limits::default());
    produced(
        &diagnostic,
        CompleteCode::InvalidIdentifier,
        CompleteCause::Host(HostCause::SelectionIdentity),
        Phase::Profile,
    );
}

#[test]
#[trace("FR-255-AC-1")]
fn source_lexer_and_parser_limits_reach_actual_stage_limit_causes() {
    use fsl_foundation::diagnostic::LimitKind;
    let text = source("function total using p(): Integer pure { 5 }");
    assert!(parsed(&text).is_admissible());
    for (limits, kind, phase) in [
        (
            Limits::default().with_source_bytes(1),
            LimitKind::InputBytes,
            Phase::Source,
        ),
        (
            Limits::default().with_tokens(0),
            LimitKind::TokenCount,
            Phase::Lex,
        ),
        (
            Limits::default().with_nodes(0),
            LimitKind::NodeCount,
            Phase::Parse,
        ),
        (
            Limits::default().with_work_units(1),
            LimitKind::WorkBudget,
            Phase::Parse,
        ),
    ] {
        let diagnostic = refused(text.as_bytes(), limits);
        produced(
            &diagnostic,
            CompleteCode::StageLimitExceeded,
            CompleteCause::StageLimit(kind),
            phase,
        );
        assert!(
            diagnostic.limit().is_some(),
            "the actual denied budget is retained"
        );
    }
}

#[test]
#[trace("FR-111-AC-3")]
fn real_bundle_budget_and_authority_inputs_keep_distinct_codes() {
    use fsl_foundation::selection::DefinitionRef;
    use fsl_semantics::library::bundle::{
        link_bundle, DefinitionCatalog, ExtensionDefinitionSource, PackageLimits, ResolutionCause,
    };
    use std::collections::BTreeSet;
    let selected = DefinitionRef::new("producer.example", "extensions/budget").expect("selection");
    let offered = ExtensionDefinitionSource {
        authority: selected.authority().into(),
        identity: selected.identity().into(),
        exact_bytes: b"{}".to_vec(),
        dependencies: BTreeSet::new(),
        capabilities: BTreeSet::new(),
    };
    let catalog = DefinitionCatalog::read_extensions(vec![offered]).expect("real caller document");
    let limited = link_bundle(
        std::slice::from_ref(&selected),
        &catalog,
        PackageLimits::default().with_definitions(0),
    )
    .expect_err("caller denies first selected definition");
    assert_eq!(
        (limited.code, limited.cause_tag),
        (
            CompleteCode::ResourceExhausted,
            ResolutionCause::InsufficientNextCharge
        )
    );
    let conflict =
        DefinitionRef::new("another.authority", selected.identity()).expect("distinct authority");
    let ambiguous = link_bundle(&[selected, conflict], &catalog, PackageLimits::default())
        .expect_err("two authorities select one logical identity");
    assert_eq!(
        (ambiguous.code, ambiguous.cause_tag),
        (
            CompleteCode::AmbiguousDeclaration,
            ResolutionCause::ConflictingAuthority
        )
    );
    for code in [limited.code, ambiguous.code] {
        assert!(DOCUMENTATION.contains(&format!("| {} |", code.as_str())));
    }
}

#[path = "../../fsl-semantics/tests/support/named_population.rs"]
mod population_fixture;

fn population_domain() -> fsl_semantics::model::domain_package::DomainPackage {
    use fsl_semantics::model::intake::{admit, package_input, read_domain_package};
    let document = population_fixture::document(false);
    let input =
        package_input([document.as_slice()]).expect("authoritative same-repository package input");
    let (selection, document) = admit(
        population_fixture::PACKAGE,
        &input,
        fsl_foundation::IntakeLimits::default(),
    )
    .expect("exact public model selection");
    read_domain_package(selection, &document).expect("valid source-owned domain records")
}
fn population_view(
    domain: &fsl_semantics::model::domain_package::DomainPackage,
) -> fsl_semantics::model::normalize::EffectiveView {
    use fsl_semantics::model::normalize::{normalize, NormalizeOutcome};
    match normalize(
        domain,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    ) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("model must normalize before runtime input tests: {other:?}"),
    }
}

#[test]
#[trace("FSpec-FR-153-AC-2", "FSpec-FR-153-AC-3")]
fn real_population_inputs_reach_foreign_incomplete_and_cardinality_codes() {
    use fsl_semantics::model::dispatch::GeneralizationClosure;
    use fsl_semantics::model::population::{
        admit_binding, all_instances, AdmissionMeter, AdmissionOutcome, AllInstancesOutcome,
        PopulationAdmissionLimits, PopulationDocument, PopulationMember,
    };
    let domain = population_domain();
    let view = population_view(&domain);
    let population = population_fixture::key("counters");
    let mut document = PopulationDocument {
        model_identity: population_fixture::PACKAGE.into(),
        members: Vec::new(),
    };
    let meter = || AdmissionMeter::new(PopulationAdmissionLimits::default());
    let foreign = PopulationDocument {
        model_identity: "producer/foreign-model".into(),
        members: Vec::new(),
    };
    let AdmissionOutcome::Refused(refusal) = admit_binding(
        &view,
        &foreign,
        &population,
        GeneralizationClosure::Closed,
        Some(1),
        &mut meter(),
    ) else {
        panic!("a foreign model identity must refuse")
    };
    assert_eq!(refusal.code, CompleteCode::ForeignReference, "{refusal:?}");
    let AdmissionOutcome::UnknownClosure(refusal) = admit_binding(
        &view,
        &document,
        &population,
        GeneralizationClosure::Open,
        Some(1),
        &mut meter(),
    ) else {
        panic!("unknown subtype closure cannot admit a population")
    };
    assert_eq!(
        refusal.code,
        CompleteCode::IncompletePopulation,
        "{refusal:?}"
    );
    document.members = ["parcel-a", "parcel-b"]
        .into_iter()
        .map(|object| PopulationMember {
            object: object.into(),
            type_identity: population_fixture::key("Counter"),
            field_values: Vec::new(),
        })
        .collect();
    let AdmissionOutcome::Admitted(binding) = admit_binding(
        &view,
        &document,
        &population,
        GeneralizationClosure::Closed,
        Some(1),
        &mut meter(),
    ) else {
        panic!("well-formed closed members admit independently of a later query bound")
    };
    let mut scalar = filament_exact::Meter::new(filament_exact::ScalarLimits {
        integer_bits: 256,
        decimal_digits: 256,
        scale_expansion: 256,
        text_input_bytes: 4096,
        text_scalars: 1024,
        normalized_scalars: 1024,
        unit_edges: 256,
        value_occurrences: 1024,
        work_units: 10_000,
        result_units: 4096,
    });
    let AllInstancesOutcome::Refused(refusal) =
        all_instances(&binding, &population_fixture::key("Counter"), &mut scalar)
    else {
        panic!("two selected members exceed the declared maximum of one")
    };
    assert_eq!(
        refusal.code,
        CompleteCode::CardinalityOutOfBound,
        "{refusal:?}"
    );
    for code in [
        CompleteCode::ForeignReference,
        CompleteCode::IncompletePopulation,
        CompleteCode::CardinalityOutOfBound,
    ] {
        assert!(DOCUMENTATION.contains(&format!("| {} |", code.as_str())));
    }
}

#[test]
fn an_actual_abstract_instance_produces_invalid_runtime_input() {
    use fsl_semantics::model::dispatch::GeneralizationClosure;
    use fsl_semantics::model::domain_package::DomainPackageRecord;
    use fsl_semantics::model::population::{
        admit_binding, AdmissionMeter, AdmissionOutcome, PopulationAdmissionLimits,
        PopulationDocument, PopulationMember,
    };
    let mut domain = population_domain();
    let record = domain
        .records
        .iter_mut()
        .find_map(|record| match record {
            DomainPackageRecord::ObjectType(record)
                if record.key == population_fixture::key("Counter") =>
            {
                Some(record)
            }
            _ => None,
        })
        .expect("actual object-type input record");
    record.abstract_type = true;
    let view = population_view(&domain);
    let document = PopulationDocument {
        model_identity: population_fixture::PACKAGE.into(),
        members: vec![PopulationMember {
            object: "parcel-a".into(),
            type_identity: population_fixture::key("Counter"),
            field_values: Vec::new(),
        }],
    };
    let mut meter = AdmissionMeter::new(PopulationAdmissionLimits::default());
    let AdmissionOutcome::Refused(refusal) = admit_binding(
        &view,
        &document,
        &population_fixture::key("counters"),
        GeneralizationClosure::Closed,
        Some(1),
        &mut meter,
    ) else {
        panic!("an abstract object type has no direct instance")
    };
    assert_eq!(
        refusal.code,
        CompleteCode::InvalidRuntimeInput,
        "{refusal:?}"
    );
    assert!(DOCUMENTATION.contains("| invalid_runtime_input |"));
}

#[test]
fn duplicate_real_model_selections_produce_duplicate_selection() {
    use fsl_semantics::model::intake::{admit_selections, package_input};
    let document = population_fixture::document(false);
    let input = package_input([document.as_slice()]).expect("real package bytes");
    let selected = vec![
        population_fixture::PACKAGE.to_owned(),
        population_fixture::PACKAGE.to_owned(),
    ];
    let refusal = admit_selections(&selected, &input, fsl_foundation::IntakeLimits::default())
        .expect_err("a domain identity cannot be selected twice");
    assert_eq!(
        refusal.code,
        CompleteCode::DuplicateSelection,
        "{refusal:?}"
    );
    assert!(DOCUMENTATION.contains("| duplicate_selection |"));
}

#[test]
fn real_invocations_distinguish_recorded_delta_from_frame_permission() {
    use fsl_semantics::model::dispatch::GeneralizationClosure;
    use fsl_semantics::model::domain_package::OperationEffect;
    use fsl_semantics::model::population::{
        admit_invocation, AdmissionMeter, AdmissionOutcome, InvocationContext, InvocationDelta,
        PopulationAdmissionLimits, PopulationDocument, PopulationMember,
    };
    // FSpec FR-046-AC-3: complete pre/post inventories and frame permission are
    // independent checks; an empty frame grants no creation.
    let domain = population_domain();
    let view = population_view(&domain);
    let population = population_fixture::key("counters");
    let pre = PopulationDocument {
        model_identity: population_fixture::PACKAGE.into(),
        members: Vec::new(),
    };
    let mut post = pre.clone();
    let effect = OperationEffect::default();
    let context = || InvocationContext {
        view: &view,
        population: &population,
        subtype_closure: GeneralizationClosure::Closed,
        declared_maximum: Some(2),
    };
    let meter = || AdmissionMeter::new(PopulationAdmissionLimits::default());
    let invoke = |post: &PopulationDocument, declared_created: &[String]| {
        admit_invocation(
            context(),
            &pre,
            post,
            &InvocationDelta {
                effect: &effect,
                declared_created,
                declared_deleted: &[],
            },
            &mut meter(),
            &mut meter(),
        )
    };
    assert!(matches!(invoke(&post, &[]), AdmissionOutcome::Admitted(_)));
    let AdmissionOutcome::Refused(refusal) = invoke(&post, &["parcel-new".into()]) else {
        panic!("recording a creation absent from complete observations must refuse")
    };
    assert_eq!(
        refusal.code,
        CompleteCode::PopulationDeltaMismatch,
        "{refusal:?}"
    );
    post.members.push(PopulationMember {
        object: "parcel-new".into(),
        type_identity: population_fixture::key("Counter"),
        field_values: Vec::new(),
    });
    let AdmissionOutcome::Refused(refusal) = invoke(&post, &["parcel-new".into()]) else {
        panic!("an accurate delta does not authorize creation outside the frame")
    };
    assert_eq!(refusal.code, CompleteCode::FrameViolation, "{refusal:?}");
    for code in [
        CompleteCode::PopulationDeltaMismatch,
        CompleteCode::FrameViolation,
    ] {
        assert!(DOCUMENTATION.contains(&format!("| {} |", code.as_str())));
    }
}

#[test]
fn real_snapshot_provisions_distinguish_absence_from_wrong_role() {
    use filament_semantic_value::declaration::TypeEnvironment;
    use fsl_semantics::model::accounting::ModelNormalizationLimits;
    use fsl_semantics::model::observation::{
        admit_current_snapshot, AdmissionFailure, DocumentRef, ModelSelections, ObservationLimits,
    };
    use std::collections::BTreeMap;
    // FR-106 read check 1.1 and current-snapshot admission: no selected
    // models means no invented model authority is needed for an empty state.
    let selected = DocumentRef {
        authority: "producer-tests".into(),
        identity: "original:parcel-snapshot".into(),
    };
    let types = TypeEnvironment::default();
    let packages = BTreeMap::new();
    let admit = |provision: &BTreeMap<DocumentRef, Vec<u8>>| {
        admit_current_snapshot(
            ModelSelections::new(&[], &[]),
            &types,
            &packages,
            ModelNormalizationLimits::default(),
            provision,
            &selected,
            ObservationLimits::default(),
        )
    };
    let AdmissionFailure::Incomplete(record) =
        admit(&BTreeMap::new()).expect_err("absent artifact")
    else {
        panic!("an absent snapshot is incomplete rather than an empty population")
    };
    assert_eq!(
        record.catalog.code(),
        CompleteCode::UnavailableObservation.as_str()
    );
    assert_eq!(record.catalog.cause(), "missing-required-artifact");
    let snapshot = |role| {
        serde_json::to_vec(&serde_json::json!({
            "format": "filament.state.snapshot/v1",
            "identity": {"authority": selected.authority, "identity": selected.identity},
            "observation": role,
            "populations": []
        }))
        .expect("original snapshot bytes")
    };
    let current = BTreeMap::from([(selected.clone(), snapshot("current"))]);
    assert!(
        admit(&current).is_ok(),
        "an available empty current state admits"
    );
    let pre = BTreeMap::from([(selected.clone(), snapshot("pre"))]);
    let AdmissionFailure::Refused(record) = admit(&pre).expect_err("wrong selected role") else {
        panic!("an available pre snapshot is a refusal, not missing evidence")
    };
    assert_eq!(record.catalog.code(), CompleteCode::WrongSnapshot.as_str());
    assert_eq!(record.catalog.cause(), "wrong-observation");
    for code in [
        CompleteCode::UnavailableObservation,
        CompleteCode::WrongSnapshot,
    ] {
        assert!(DOCUMENTATION.contains(&format!("| {} |", code.as_str())));
    }
}

#[test]
#[trace("FR-031-AC-6")]
fn original_extraction_requests_reach_combination_refusal_producers() {
    let directory = tempfile::tempdir().expect("owned extraction requests");
    let source = serde_json::json!({"file":"original.md","authority":"producer-tests","identity":"original:markdown","document":"ParcelGuide","formal_revision":1});
    let program = serde_json::json!({"source":source,"extraction":{"body":{"authority":"producer-tests","identity":"original:body","document":"ParcelRule"}},"clauses":[{"clause":"ParcelRule","package":"original-parcels"}]});
    let base =
        serde_json::json!({"format":"native-run/1","request":{"models":[],"program":program}});
    let mut cases = Vec::new();
    let mut package = base.clone();
    package["request"]["package"] = serde_json::json!({"file":"absent.package.json"});
    cases.push((package, CompleteCode::ExtractionPackageConflict));
    let mut call = base.clone();
    call["request"]["call"] = serde_json::json!({"function":"parcel_cost","arguments":[]});
    cases.push((call, CompleteCode::ExtractionRequiresRun));
    for bindings in [
        serde_json::json!([]),
        serde_json::json!([
            {"clause":"First","package":"original-parcels"},
            {"clause":"Second","package":"original-parcels"}
        ]),
    ] {
        let mut request = base.clone();
        request["request"]["program"]["clauses"] = bindings;
        cases.push((request, CompleteCode::ExtractionClauseCount));
    }
    for (index, (request, code)) in cases.into_iter().enumerate() {
        let path = directory.path().join(format!("request-{index}.json"));
        std::fs::write(
            &path,
            serde_json::to_vec(&request).expect("request encodes"),
        )
        .expect("own input");
        let refusal = fsl_command::command::read_clause_run_request(&path)
            .err()
            .expect("incompatible extraction request");
        assert_eq!(refusal.cause.code(), code, "{refusal:?}");
        if let fsl_command::command::RunCause::ExtractionClauseCount { actual } = refusal.cause {
            assert_eq!(
                actual,
                request["request"]["program"]["clauses"]
                    .as_array()
                    .expect("bindings")
                    .len()
            );
        }
        assert!(DOCUMENTATION.contains(&format!("| {} |", code.as_str())));
    }
    let compile = serde_json::json!({"format":"native-compile/1","request":{"models":[],"program":{"source":source,"extraction":{}}}});
    let path = directory.path().join("compile.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&compile).expect("compile encodes"),
    )
    .expect("own input");
    let refusal =
        fsl_command::command::compile(&path).expect_err("extraction requires a clause run");
    assert_eq!(refusal.cause.code(), CompleteCode::ExtractionRequiresRun);
    assert!(
        !directory.path().join("original.md").exists(),
        "request defects settle before opening source bytes"
    );
}

#[test]
fn an_original_extraction_request_preserves_quire_context_diagnostics() {
    let directory = tempfile::tempdir().expect("owned Markdown input");
    std::fs::write(directory.path().join("original.md"), "# Parcel guide\n")
        .expect("original Markdown");
    let request = serde_json::json!({"format":"native-run/1","request":{
        "models":[],
        "program":{
            "source":{"file":"original.md","authority":"producer-tests","identity":"original:markdown","document":"ParcelGuide","formal_revision":1},
            "extraction":{"body":{"authority":"producer-tests","identity":"original:body","document":"ParcelRule"}},
            "clauses":[{"clause":"ParcelRule","package":""}]
        },
        "clause":{"selection":{},"observations":{}}
    }});
    let path = directory.path().join("request.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&request).expect("request encodes"),
    )
    .expect("own request");
    let refusal = fsl_command::command::read_clause_run_request(&path)
        .err()
        .expect("empty semantic package");
    assert_eq!(
        refusal.cause.code(),
        CompleteCode::InvalidQuireContext,
        "{refusal:?}"
    );
    let fsl_command::command::RunCause::InvalidQuireContext { diagnostics } = refusal.cause else {
        panic!("the original Quire diagnostics must be retained")
    };
    assert!(!diagnostics.is_empty());
    assert!(DOCUMENTATION.contains("| invalid-quire-context |"));
}

#[test]
fn a_source_clock_without_time_reaches_the_timed_model_producer() {
    // FR-230, source-model time member: selected-model time cannot supply a
    // source clock, and a source clock without dense time is a model defect.
    let text = format!("{HEADER}profile t = \"filament.temporal.complete/v1\";\nobject ParcelTimer {{ elapsed: Clock; population timers; }}\n");
    let refusal = compile_text(&text).expect_err("a source clock needs source dense time");
    assert_eq!(
        refusal.cause.code(),
        CompleteCode::InvalidTimedModel,
        "{refusal:?}"
    );
    let fsl_command::command::RunCause::Spine(failure) = &refusal.cause else {
        panic!("timed checking owns this located refusal: {refusal:?}")
    };
    let stage: fsl_replay::OutcomeStage = failure.refusal.stage().into();
    assert_eq!(stage, fsl_replay::OutcomeStage::S3);
    assert!(failure.span.is_some());
    assert!(DOCUMENTATION.contains("| invalid_timed_model |"));
}

#[test]
#[trace("FR-185-AC-2")]
fn an_original_zero_weight_reaches_the_probabilistic_model_producer() {
    let text = format!("{HEADER}profile q = \"filament.model.complete/v1\";\nobject ParcelSorter {{ population sorters; operation route(left: Boolean); }}\nrandom ParcelSorter::route(left) ~ {{ true: 0, false: 1 }};\n");
    let refusal = compile_text(&text).expect_err("each authored random weight must be positive");
    assert_eq!(
        refusal.cause.code(),
        CompleteCode::InvalidProbabilisticModel,
        "{refusal:?}"
    );
    let fsl_command::command::RunCause::Spine(failure) = &refusal.cause else {
        panic!("probabilistic checking owns this located refusal: {refusal:?}")
    };
    let stage: fsl_replay::OutcomeStage = failure.refusal.stage().into();
    assert_eq!(stage, fsl_replay::OutcomeStage::S3);
    assert!(failure.span.is_some());
    assert!(DOCUMENTATION.contains("| invalid_probabilistic_model |"));
}

#[test]
fn an_original_sequence_bound_reaches_fixed_representability_refusal() {
    // FR-139's seq maximum is max(0, hi_max - lo_min + 1), FSpec
    // FR-376-AC-2. These endpoints fit Integer; the cardinality exceeds u64.
    let body = "object Parcel { population parcels; } invariant Ordered using p on Parcel at current { size(seq(i in 0 .. 18446744073709551615: i)) >= 0 }";
    let refusal = compile_original(body).expect_err("finite maximum cannot be represented");
    assert_eq!(
        refusal.cause.code(),
        CompleteCode::UnrepresentableConstraint,
        "{refusal:?}"
    );
    let fsl_command::command::RunCause::Spine(failure) = &refusal.cause else {
        panic!("the checker owns the fixed representability policy: {refusal:?}")
    };
    let stage: fsl_replay::OutcomeStage = failure.refusal.stage().into();
    assert_eq!(stage, fsl_replay::OutcomeStage::S3);
    assert!(failure.span.is_some());
    assert!(DOCUMENTATION.contains("| unrepresentable_constraint |"));
}

#[test]
#[trace("FR-302-AC-7")]
fn original_duplicate_operation_bodies_reach_conflicting_declaration() {
    let body = "profile m = \"filament.model.complete/v1\"; object ParcelScanner { population scanners; operation ready(): Boolean; } body First using m on ParcelScanner::ready() { true } body Second using m on ParcelScanner::ready() { false }";
    let refusal = compile_original(body).expect_err("one resolved operation has at most one body");
    assert_eq!(
        refusal.cause.code(),
        CompleteCode::ConflictingDeclaration,
        "{refusal:?}"
    );
    assert!(DOCUMENTATION.contains("| conflicting_declaration |"));
}

#[test]
fn a_real_called_family_without_a_body_reaches_ambiguous_dispatch() {
    // FSpec FR-151-AC-3 and FSL FR-302: a declared operation without an
    // applicable authored body cannot be supplied a dispatch target.
    let body = "profile m = \"filament.model.complete/v1\"; object ParcelScanner { population scanners; operation ready(): Boolean; } invariant Scanned using m on ParcelScanner at current { self.ready() }";
    let refusal = compile_original(body).expect_err("the called family has no applicable body");
    assert_eq!(
        refusal.cause.code(),
        CompleteCode::AmbiguousDispatch,
        "{refusal:?}"
    );
    assert!(DOCUMENTATION.contains("| ambiguous_dispatch |"));
}

#[test]
#[trace("FR-296-AC-2")]
fn an_original_call_request_reaches_unsupplied_execution_capability() {
    let mut call = fsl_replay::spine::Call::new(
        "parcel_cost".into(),
        Vec::new(),
        fsl_replay::spine::default_accounting(1000),
        Default::default(),
    );
    call.backend = filament_eval::ExecutionBackendChoice::Jit;
    let text = source("function parcel_cost using p(): Integer pure { 17 }");
    let refusal = fsl_replay::spine::run(
        identity(),
        "original:diagnostics".into(),
        "producer.native",
        text.as_bytes(),
        &std::collections::BTreeMap::new(),
        &fsl_replay::spine::DependencyInput::default(),
        fsl_replay::spine::SpineLimits::default(),
        &call,
        &fsl_semantics::model::object_environment::ObjectEnvironment::default(),
        &Cancel::new(),
    )
    .expect_err("JIT is not supplied to the public default entry");
    assert_eq!(
        refusal.code(),
        CompleteCode::UnimplementedCapability,
        "{refusal:?}"
    );
    let fsl_replay::spine::RunRefusal::Unsupported(entry) = refusal.as_ref() else {
        panic!("backend admission owns the refusal")
    };
    assert_eq!(entry.backend, filament_eval::ExecutionBackendChoice::Jit);
    assert_eq!(entry.cause, filament_eval::UnsupportedCause::NotSupplied);
    assert!(DOCUMENTATION.contains("| unimplemented_capability |"));
}

#[test]
#[ignore = "QSL-2692: public observation inputs omit source-declared models"]
fn an_admitted_complete_population_distinguishes_dangling_protocol_seeds() {
    use fsl_semantics::model::accounting::ModelNormalizationLimits;
    use fsl_semantics::model::domain_package::DomainPackageRecord;
    use fsl_semantics::model::observation::{
        admit_protocol_reference_seeds, admit_value_invariants_snapshot,
        declared_type_universe_for, AdmissionFailure, DocumentRef, ObservationLimits,
    };
    use std::collections::BTreeMap;
    // FR-106-AC-65: protocol reference admission first requires completeness,
    // then checks actual targets. All identities below come from the checked
    // original source and its admitted snapshot rather than a synthetic arena.
    let text =
        source("profile m = \"filament.model.complete/v1\"; object Parcel { population parcels; }");
    let cancel = Cancel::new();
    let source_identity = identity();
    let parsed = fsl_replay::spine::parse(
        &fsl_replay::spine::ParseRequest {
            source: &source_identity,
            path: "producer.native",
            bytes: text.as_bytes(),
        },
        Limits::default(),
        &cancel,
    )
    .expect("original source parses")
    .into_value();
    let packages = BTreeMap::new();
    let models = fsl_replay::spine::select(
        &parsed,
        &packages,
        ModelNormalizationLimits::default(),
        &cancel,
    )
    .expect("source model selection")
    .into_value();
    let checked = fsl_replay::spine::check(
        &parsed,
        "original:diagnostics".into(),
        &models,
        &fsl_replay::spine::DependencyInput::default(),
        fsl_replay::spine::SpineLimits::default(),
        &cancel,
    )
    .expect("original source checks")
    .into_value();
    let graph = checked.package().graph();
    let domain = graph
        .source_packages()
        .first()
        .expect("actual source model");
    let population = domain
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::Population(record) => Some(record),
            _ => None,
        })
        .expect("authored population");
    let member_type = &population.member_types[0];
    let (effective, universe) = declared_type_universe_for(
        graph.model_inputs(),
        &packages,
        ModelNormalizationLimits::default(),
        member_type,
    )
    .expect("admitted model view")
    .expect("authored member type");
    let selected = DocumentRef {
        authority: "producer-tests".into(),
        identity: "original:parcel-state".into(),
    };
    for complete in [true, false] {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "format":"filament.state.snapshot/v1",
            "identity":{"authority":selected.authority,"identity":selected.identity},
            "observation":"current",
            "model":{"identity":domain.model_selection.identity},
            "populations":[{"population":population.key.node,"complete":complete,
                "objects":[{"key":"parcel-present","type":member_type.node,"fields":{}}]}]
        }))
        .expect("original state encodes");
        let provision = BTreeMap::from([(selected.clone(), bytes)]);
        let (observation, _) = admit_value_invariants_snapshot(
            graph.model_inputs(),
            graph.scope().types(),
            &packages,
            ModelNormalizationLimits::default(),
            &provision,
            &selected,
            ObservationLimits::default(),
        )
        .expect("structurally valid snapshot admits");
        let node = observation
            .environment
            .population_node(&population.key.node)
            .expect("admitted provenance");
        let seed = |object| {
            (
                filament_exact::ObjectReference::new(
                    universe.clone(),
                    effective.clone(),
                    filament_exact::ObjectId::new(object).expect("authored object key"),
                ),
                node.clone(),
                Vec::new(),
            )
        };
        let present = admit_protocol_reference_seeds(
            graph.scope().types(),
            &observation,
            &[seed("parcel-present")],
        );
        let missing = admit_protocol_reference_seeds(
            graph.scope().types(),
            &observation,
            &[seed("parcel-missing")],
        );
        if complete {
            assert!(
                present.is_ok(),
                "an admitted target is not dangling: {present:?}"
            );
            let AdmissionFailure::Refused(record) =
                missing.expect_err("absent target in complete population")
            else {
                panic!("a complete population settles an absent reference as a refusal")
            };
            assert_eq!(
                record.catalog.code(),
                CompleteCode::DanglingReference.as_str()
            );
            assert_eq!(
                record.catalog.cause(),
                "absent-target-in-complete-population"
            );
        } else {
            for result in [present, missing] {
                let AdmissionFailure::Incomplete(record) =
                    result.expect_err("open population cannot establish targets")
                else {
                    panic!("incomplete evidence cannot prove a dangling target")
                };
                assert_eq!(
                    record.catalog.code(),
                    CompleteCode::IncompletePopulation.as_str()
                );
                assert_eq!(record.catalog.cause(), "incomplete-scope");
            }
        }
    }
    assert!(DOCUMENTATION.contains("| dangling_reference |"));
}
