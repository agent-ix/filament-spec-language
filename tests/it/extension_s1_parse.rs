// SPDX-License-Identifier: AGPL-3.0-or-later
//! S1 parse of a selected extension's declaration forms and the
//! declaration-form refusal (FR-354 "Admission"; TC-882 parse steps,
//! TC-883 steps 5 and 6; FSpec FR-467 "Production").
use std::collections::BTreeSet;

use filament_spec_language::complete::parse_with_extensions;
use fsl_cst::{CompleteCause, CompleteCode, CompleteDiagnostic, Limits, ParsedSource, Production};
use fsl_foundation::selection::DefinitionRef;
use fsl_foundation::SourceIdentity;
use fsl_semantics::library::bundle::{
    Definition, DefinitionCatalog, DefinitionRole, ReaderAuthority,
};
use fsl_semantics::library::extension::{CoreVocabulary, HeldExtensions};
use ix_trace_rs::trace;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"filament.value.complete/v1\";\n";
const FORM: &str = "retention Audit { subject: \"audit\", days: 30 }";

fn identity() -> SourceIdentity {
    SourceIdentity {
        authority: "test".into(),
        identity: "test:extension-s1".into(),
    }
}

/// TC-882's `retention` extension, held after catalog validation.
fn held(keywords: &[&str]) -> HeldExtensions {
    let reader = ReaderAuthority::fixture();
    let definitions: Vec<Definition> = keywords
        .iter()
        .map(|keyword| {
            let document = serde_json::json!({
                "grammar_schema": {"productions": [
                    {"keyword": keyword, "clauses": ["subject", "days"]}
                ]},
                "typed_node_schema": {"nodes": [
                    {"keyword": keyword, "node_kind": keyword, "clauses": [
                        {"name": "subject", "type": "Text"},
                        {"name": "days", "type": "Int[1, 3650]"}
                    ]}
                ]}
            });
            Definition::from_exact_bytes(
                &reader,
                "agent-ix",
                format!("ext.{keyword}"),
                DefinitionRole::Source,
                BTreeSet::new(),
                BTreeSet::new(),
                document.to_string().as_bytes(),
            )
            .unwrap()
        })
        .collect();
    let selected: Vec<_> = keywords
        .iter()
        .map(|keyword| DefinitionRef::new("agent-ix", format!("ext.{keyword}")).unwrap())
        .collect();
    DefinitionCatalog::new(definitions)
        .unwrap()
        .hold_extensions(
            &selected,
            &CoreVocabulary::new(["function", "record"], ["record"]),
        )
        .unwrap()
}

fn parse(held: &HeldExtensions, declarations: &str) -> ParsedSource {
    parse_with_extensions(
        identity(),
        "unit.native",
        format!("{HEADER}{declarations}\n").as_bytes(),
        held,
        Limits::default(),
    )
    .unwrap()
}

fn only_diagnostic(parsed: &ParsedSource) -> &CompleteDiagnostic {
    assert_eq!(parsed.diagnostics().len(), 1, "{:?}", parsed.diagnostics());
    &parsed.diagnostics()[0]
}

/// The first diagnostic is at the first occurrence of `token` after `from`.
fn refused_at(declarations: &str, from: &str, token: &str) -> (usize, usize) {
    let source = format!("{HEADER}{declarations}\n");
    let start = source.find(from).unwrap();
    let at = start + source[start..].find(token).unwrap();
    (at, at + token.len())
}

fn invalid_syntax(declarations: &str, from: &str, token: &str, expected: &str) {
    let parsed = parse(&held(&["retention"]), declarations);
    let diagnostic = only_diagnostic(&parsed);
    assert_eq!(
        diagnostic.code,
        CompleteCode::InvalidSyntax,
        "{declarations}"
    );
    assert_eq!(
        diagnostic.cause,
        CompleteCause::UnexpectedToken,
        "{declarations}"
    );
    let (start, end) = refused_at(declarations, from, token);
    let span = diagnostic.byte_span().unwrap();
    assert_eq!((span.start, span.end), (start, end), "{declarations}");
    assert!(
        diagnostic.message.contains(expected),
        "{declarations}: {}",
        diagnostic.message
    );
}

#[trace("TC-882", "FR-354-AC-1")]
#[trace("FSpec-TC-949", "FSpec-FR-467-AC-1")]
#[test]
fn a_selected_extension_form_parses_as_one_node_spanning_the_whole_form() {
    let parsed = parse(&held(&["retention"]), FORM);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let forms: Vec<_> = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::ExtensionDeclaration)
        .collect();
    assert_eq!(forms.len(), 1);
    assert_eq!(parsed.cst().render_node(forms[0]).unwrap(), FORM.as_bytes());
    let values = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::Expression)
        .count();
    assert_eq!(values, 2, "one expression per clause value");
}

#[trace("TC-882", "FR-354-AC-2")]
#[test]
fn a_form_that_misses_its_production_refuses_at_s1_naming_the_expected_clause() {
    // `days` absent: the `}` where the production expects the next clause.
    invalid_syntax(
        "retention Audit { subject: \"audit\" }",
        "retention",
        "}",
        "clause `days`",
    );
    // Out of order: `days` where the production expects `subject`.
    invalid_syntax(
        "retention Audit { days: 30, subject: \"audit\" }",
        "retention",
        "days",
        "clause `subject`",
    );
    // Trailing separator: the `}` after the last `,`.
    for expected in ["clause-name", "clause `days`"] {
        invalid_syntax(
            "retention Audit { subject: \"audit\", }",
            "retention",
            "}",
            expected,
        );
    }
    // An unknown clause name and a repeated clause.
    invalid_syntax(
        "retention Audit { subject: \"a\", period: 3 }",
        "retention",
        "period",
        "clause `days`",
    );
    invalid_syntax(
        "retention Audit { subject: \"a\", subject: \"b\" }",
        "\"a\", ",
        "subject",
        "clause `days`",
    );
}

#[trace("TC-882", "FR-354-AC-2")]
#[test]
fn a_form_missing_a_name_brace_or_value_refuses_naming_the_actual_token() {
    invalid_syntax(
        "retention { subject: \"audit\", days: 30 }",
        "retention",
        "{",
        "identifier",
    );
    invalid_syntax(
        "retention Audit subject: \"audit\", days: 30",
        "retention",
        "subject",
        "`{`",
    );
    invalid_syntax(
        "retention Audit { subject: , days: 30 }",
        "retention",
        ",",
        "expected",
    );
}

#[trace("TC-882", "FR-354-AC-2")]
#[test]
fn a_form_cut_off_at_the_end_of_the_input_refuses_unexpected_end() {
    let parsed = parse(
        &held(&["retention"]),
        "retention Audit { subject: \"audit\",",
    );
    let diagnostic = only_diagnostic(&parsed);
    assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax);
    assert_eq!(diagnostic.cause, CompleteCause::UnexpectedEnd);
    assert!(
        diagnostic.message.contains("clause `days`"),
        "{}",
        diagnostic.message
    );
}

#[trace("TC-883", "FR-354-AC-4")]
#[test]
fn an_unselected_leading_keyword_refuses_declaration_form_whatever_the_catalog_holds() {
    let selections = "profile Complete = \"filament.value.complete/v1\"";
    let none = parse(&HeldExtensions::default(), FORM);
    let other = parse(&held(&["audit"]), FORM);
    for parsed in [&none, &other] {
        let diagnostic = only_diagnostic(parsed);
        assert_eq!(diagnostic.code, CompleteCode::UnsupportedConstruct);
        assert_eq!(diagnostic.cause, CompleteCause::DeclarationForm);
        assert_eq!(diagnostic.cause.as_str(), "declaration-form");
        assert!(diagnostic.cause.is_cause_of(diagnostic.code));
        let (start, end) = refused_at(FORM, "retention", "retention");
        let span = diagnostic.byte_span().unwrap();
        assert_eq!((span.start, span.end), (start, end));
        assert!(
            diagnostic.message.contains("`retention`"),
            "{}",
            diagnostic.message
        );
        assert!(
            diagnostic.message.contains(selections),
            "{}",
            diagnostic.message
        );
    }
    assert_eq!(
        none.diagnostics()[0].message,
        other.diagnostics()[0].message
    );
    assert_eq!(
        none.diagnostics()[0].byte_span(),
        other.diagnostics()[0].byte_span()
    );
}

#[trace("TC-883", "FR-354-AC-4")]
#[test]
fn a_word_after_another_declaration_refuses_declaration_form_at_its_own_span() {
    let declarations = "record R { datum: Integer; }\nbogus R { datum: 1 }";
    let parsed = parse(&held(&["retention"]), declarations);
    let diagnostic = only_diagnostic(&parsed);
    assert_eq!(diagnostic.cause, CompleteCause::DeclarationForm);
    let (start, end) = refused_at(declarations, "bogus", "bogus");
    let span = diagnostic.byte_span().unwrap();
    assert_eq!((span.start, span.end), (start, end));
}

#[trace("TC-883", "FR-354-AC-4")]
#[test]
fn a_leading_keyword_is_an_ordinary_identifier_outside_a_top_level_declaration_start() {
    let declarations = concat!(
        "record Policy { retention: Integer; }\n",
        "function Keep using Complete (retention: Integer): Integer pure { retention }\n",
        "retention Audit { subject: \"audit\", days: 30 }\n",
    );
    let with = parse(&held(&["retention"]), declarations);
    assert!(with.is_admissible(), "{:?}", with.diagnostics());
    let forms = with
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::ExtensionDeclaration)
        .count();
    assert_eq!(forms, 1, "only the top-level form is an extension form");

    // Without the form the same source parses identically with and without
    // the extension selected.
    let plain = declarations.replace("retention Audit { subject: \"audit\", days: 30 }\n", "");
    let selected = parse(&held(&["retention"]), &plain);
    let unselected = parse(&HeldExtensions::default(), &plain);
    assert!(selected.is_admissible(), "{:?}", selected.diagnostics());
    assert!(unselected.is_admissible(), "{:?}", unselected.diagnostics());
    let shape = |parsed: &ParsedSource| {
        parsed
            .cst()
            .nodes()
            .iter()
            .map(|node| (node.production(), node.span()))
            .collect::<Vec<_>>()
    };
    assert_eq!(shape(&selected), shape(&unselected));
}

/// FR-660-AC-1 (TC-1740): a `native-compile/1` request's `extensions`
/// member supplies the definition catalog the unit selects `ext.retention`
/// from, and the compile admits the `retention` form.
#[test]
#[trace("TC-1740", "FR-660-AC-1")]
#[ignore = "QSL-2189: the request `extensions` member is not implemented yet"]
fn compile_applies_request_extension_definitions() {
    let directory = tempfile::tempdir().unwrap();
    let definition = serde_json::json!({
        "grammar_schema": {"productions": [
            {"keyword": "retention", "clauses": ["subject", "days"]}
        ]},
        "typed_node_schema": {"nodes": [
            {"keyword": "retention", "node_kind": "retention", "clauses": [
                {"name": "subject", "type": "Text"},
                {"name": "days", "type": "Int[1, 3650]"}
            ]}
        ]}
    });
    std::fs::write(
        directory.path().join("retention.json"),
        definition.to_string(),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("unit.native"),
        format!("{HEADER}profile r = \"ext.retention\";\n{FORM}\n"),
    )
    .unwrap();
    let request = serde_json::json!({"format": "native-compile/1", "request": {
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:extension-request", "document": "Ext", "formal_revision": 1}},
        "extensions": [{"authority": "agent-ix", "identity": "ext.retention",
            "file": "retention.json"}]
    }});
    let path = directory.path().join("compile.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let package = fsl_command::command::compile(&path)
        .unwrap_or_else(|error| panic!("{}", error.value().unwrap()));
    let package = String::from_utf8(package).unwrap();
    assert!(package.contains("\"retention\""), "{package}");
    assert!(package.contains("ext.retention"), "{package}");
}

/// FR-660-AC-6 (TC-1740): with the catalog holding `ext.retention`, the
/// parse admits one `retention` form; the `parse` operation given no catalog
/// refuses the same form as an unselected extension form.
#[test]
#[trace("TC-1740", "FR-660-AC-6")]
fn the_parse_operation_admits_a_form_only_with_its_catalog() {
    let with = parse(&held(&["retention"]), FORM);
    assert!(with.is_admissible(), "{:?}", with.diagnostics());
    let identity = identity();
    let bytes = format!("{HEADER}{FORM}\n");
    let without = fsl_replay::spine::parse(
        &fsl_replay::spine::ParseRequest {
            source: &identity,
            path: "unit.native",
            bytes: bytes.as_bytes(),
        },
        Limits::default(),
        &filament_exact::Cancel::new(),
    );
    let document: serde_json::Value = serde_json::from_slice(
        &fsl_replay::OutcomeDocument::from_parse(&without)
            .to_bytes()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(document["category"], "unsupported", "{document}");
    assert_eq!(
        document["diagnostics"][0]["code"], "unsupported_construct",
        "{document}"
    );
    assert_eq!(
        document["diagnostics"][0]["cause"], "declaration-form",
        "{document}"
    );
}

/// FR-660-AC-6 (TC-1740): the request reader's exact-byte catalog drives S1
/// extension selection without a fixture authority.
#[test]
#[trace("TC-1740", "FR-660-AC-6")]
fn request_catalog_parses_selected_extension_definition() {
    let identity = identity();
    let definition = serde_json::json!({
        "grammar_schema": {"productions": [
            {"keyword": "retention", "clauses": ["subject", "days"]}
        ]},
        "typed_node_schema": {"nodes": [
            {"keyword": "retention", "node_kind": "retention", "clauses": [
                {"name": "subject", "type": "Text"},
                {"name": "days", "type": "Int[1, 3650]"}
            ]}
        ]}
    });
    let exact_bytes = serde_json::to_vec(&definition).unwrap();
    let catalog = DefinitionCatalog::read_extensions(vec![
        fsl_semantics::library::bundle::ExtensionDefinitionSource {
            authority: "agent-ix".into(),
            identity: "ext.retention".into(),
            exact_bytes,
            dependencies: BTreeSet::new(),
            capabilities: BTreeSet::new(),
        },
    ])
    .unwrap();
    let bytes = format!("{HEADER}profile r = \"ext.retention\";\n{FORM}\n");
    let parsed = fsl_replay::spine::parse_selecting_extensions(
        &fsl_replay::spine::ParseRequest {
            source: &identity,
            path: "unit.native",
            bytes: bytes.as_bytes(),
        },
        fsl_replay::spine::ProfileCatalogInput {
            catalog: &catalog,
            limits: fsl_semantics::library::bundle::PackageLimits::default(),
        },
        Limits::default(),
        &filament_exact::Cancel::new(),
    );
    assert!(parsed.is_ok(), "{parsed:?}");
}
