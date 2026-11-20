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
