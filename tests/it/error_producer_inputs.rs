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
        (
            "enum Caption { Label = \"bad\\q\" }",
            CompleteCause::InvalidEscape,
            Phase::Lex,
            "\\q",
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
fn a_cancelled_real_parse_stops_at_its_public_poll() {
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let text = source("function total using p(): Integer pure { 5 }");
    let diagnostic = fsl_cst::parse_with_cancel(
        identity(),
        "producer.native",
        text.as_bytes(),
        Limits::default(),
        &cancel,
    )
    .expect_err("caller poll stops parse");
    produced(
        &diagnostic,
        CompleteCode::ResourceExhausted,
        CompleteCause::InsufficientNextCharge,
        Phase::Lex,
    );
    assert!(cancel.tripped().is_some());
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
