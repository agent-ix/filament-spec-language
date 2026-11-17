// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-501 and FR-502: the forms on which the standard grammar and the
//! parser disagreed, each recognized as the standard now states it.
use fsl_cst::{CompleteCause, CompleteCode, Limits, ParsedSource, Production};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

fn unit(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n{declarations}"
    )
}

fn parse(text: &str) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:standard-forms".into(),
        },
        "standard-forms.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn texts<'t>(text: &'t str, parsed: &ParsedSource, production: Production) -> Vec<&'t str> {
    parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == production)
        .map(|node| {
            let Span { start, end } = node.span();
            &text[start..end]
        })
        .collect()
}

/// The first diagnostic's code, cause and start offset.
fn refusal(parsed: &ParsedSource) -> (CompleteCode, CompleteCause, usize) {
    let first = parsed.diagnostics().first().expect("refused");
    (
        first.code,
        first.cause,
        first.byte_span().expect("located").start,
    )
}

/// FSpec FR-501-AC-3: `let x: T = e in b` parses with the annotation on the
/// binder; the unannotated form still parses.
#[trace("FSpec-TC-1004", "FSpec-FR-501-AC-3")]
#[test]
#[ignore = "QSL-2008: the grammar has no let annotation; parsing `let x: T = e in b` is not implemented yet"]
fn an_annotated_let_parses_with_its_type_on_the_binder() {
    let text =
        unit("function f using v(y: Int[0,2]): Integer pure { let x: Int[0,3] = y + 1 in x }");
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert!(texts(&text, &parsed, Production::TypeReference).contains(&"Int[0,3]"));
    let text = unit("function g using v(y: Integer): Integer pure { let x = y + 1 in x }");
    assert!(parse(&text).is_admissible());
}

/// FSpec FR-501-AC-4 (parse half; FR-143 owns typing): `Some(e)` parses as
/// the present-option constructor call in a function body, outside any
/// refinement row.
#[trace("FSpec-TC-1005", "FSpec-FR-501-AC-4")]
#[test]
fn some_parses_in_a_function_body() {
    let text = unit("function s using v(b: Int[0,9]): Option<Int[0,9]> pure { Some(b) }");
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
}

/// FSpec FR-501-AC-5: the source `Decimal[..]` type names its rounding mode;
/// the mode-less spelling refuses at its closing `]`.
#[trace("FSpec-TC-1006", "FSpec-FR-501-AC-5")]
#[test]
fn a_decimal_type_without_a_rounding_mode_refuses_at_its_closing_bracket() {
    for mode in ["exact", "nearest-even"] {
        let text = unit(&format!(
            "function d using v(x: Decimal[0, 9; 0, 2; {mode}]): Boolean pure {{ true }}"
        ));
        assert!(parse(&text).is_admissible(), "{mode}");
    }
    let text = unit("function d using v(x: Decimal[0, 9; 0, 2]): Boolean pure { true }");
    assert_eq!(
        refusal(&parse(&text)),
        (
            CompleteCode::InvalidSyntax,
            CompleteCause::UnexpectedToken,
            text.find("2]").expect("the bound") + 1
        )
    );
}

/// FSpec FR-501-AC-6: a complete temporal clause may omit `clock`.
#[trace("FSpec-TC-1007", "FSpec-FR-501-AC-6")]
#[test]
fn a_temporal_clause_without_clock_parses() {
    for clause in [
        "temporal T using v over (o: M::O) on origin { always holds(true) }",
        "temporal T using v over (o: M::O) clock \"model-steps\" on origin { always holds(true) }",
    ] {
        let parsed = parse(&unit(clause));
        assert!(
            parsed.is_admissible(),
            "{clause}: {:?}",
            parsed.diagnostics()
        );
    }
}

/// FSpec FR-501-AC-7: a two-segment operation name is the source-object form
/// and parses; a one-segment name refuses at the `{`.
#[trace("FSpec-TC-1008", "FSpec-FR-501-AC-7")]
#[test]
fn operation_names_are_a_type_name_and_a_member() {
    for clause in [
        "pre P using v on M::T::op { true }",
        "pre P using v on T::op { true }",
    ] {
        let text = unit(clause);
        let parsed = parse(&text);
        assert!(
            parsed.is_admissible(),
            "{clause}: {:?}",
            parsed.diagnostics()
        );
        assert_eq!(texts(&text, &parsed, Production::OperationName).len(), 1);
    }
    let text = unit("pre P using v on T { true }");
    assert_eq!(
        refusal(&parse(&text)),
        (
            CompleteCode::InvalidSyntax,
            CompleteCause::UnexpectedToken,
            text.find("{ true").expect("the block")
        )
    );
}

/// FSpec FR-501-AC-8: a `join` list names every branch and is never empty.
#[trace("FSpec-TC-1008", "FSpec-FR-501-AC-8")]
#[test]
fn an_empty_join_list_refuses_at_its_closing_bracket() {
    let protocol = |join: &str| {
        unit(&format!(
            "protocol P using v over (o: M::O) on origin {{ role a on M::A; role b on M::B; \
             run parallel p {{ branch x sequence s {{}} branch y sequence t {{}} }} join all [{join}]; \
             finish Done as (r: Boolean) {{ true }}; }}"
        ))
    };
    let parsed = parse(&protocol("x, y"));
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let text = protocol("");
    assert_eq!(
        refusal(&parse(&text)),
        (
            CompleteCode::InvalidSyntax,
            CompleteCause::UnexpectedToken,
            text.find("[]").expect("the list") + 1
        )
    );
}

/// FSpec FR-502-AC-5 and AC-6: `ref(e)` and `Text(...)` are `primary`
/// alternatives; `ref` is reserved and cannot name a function.
#[trace("FSpec-TC-1009", "FSpec-FR-502-AC-5", "FSpec-FR-502-AC-6")]
#[test]
fn ref_and_text_are_primaries_and_ref_is_reserved() {
    for function in [
        "function t using v(): Text[0, 8; nfc] pure { Text(\"abc\") }",
        "function r using v(x: Integer): Boolean pure { ref(x) = ref(x) }",
    ] {
        let parsed = parse(&unit(function));
        assert!(
            parsed.is_admissible(),
            "{function}: {:?}",
            parsed.diagnostics()
        );
    }
    let text = unit("function ref using v(): Integer pure { 1 }");
    assert_eq!(
        refusal(&parse(&text)),
        (
            CompleteCode::InvalidSyntax,
            CompleteCause::UnexpectedToken,
            text.find("ref").expect("the name")
        )
    );
}
