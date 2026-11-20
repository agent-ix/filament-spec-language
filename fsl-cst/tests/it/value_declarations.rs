// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original value-declaration near-misses at real grammar boundaries.
//!
//! FSpec FR-339's pinned shared grammar owns the six productions. Expected
//! messages spell their required terminals; a missing type uses the parser's
//! bounded category for the large TypeReference alternative set.

use fsl_cst::{CompleteCause, CompleteCode, Limits, ParsedSource, Production};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

const PREFIX: &str = "language \"ix:native\" edition \"1-draft\";\r\n// μ and café distinguish byte offsets from character offsets.\r\nprofile values = \"filament.value.complete/v1\";\r\n";
const PATH: &str = "value-declarations.native";

fn identity() -> SourceIdentity {
    SourceIdentity::new("tests", "test:value-declaration-near-misses")
}

fn parse(source: &str) -> ParsedSource {
    fsl_cst::parse(identity(), PATH, source.as_bytes(), Limits::default())
        .expect("original UTF-8 input and valid source labels")
}

fn accepted(body: &str, production: Production) {
    let source = format!("{PREFIX}{body}");
    let parsed = parse(&source);
    assert!(parsed.is_admissible(), "{body}: {:?}", parsed.diagnostics());
    assert!(parsed.diagnostics().is_empty());
    assert_eq!(parsed.cst().render(), source.as_bytes());
    let mut nodes = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == production);
    let node = nodes.next().expect("the intended declaration production");
    assert!(nodes.next().is_none());
    assert_eq!(
        node.span(),
        Span {
            start: PREFIX.len(),
            end: source.len()
        }
    );
    assert_eq!(parsed.cst().render_node(node).unwrap(), body.as_bytes());
}

fn rejected(body: &str, token: &str, expected: &str) {
    rejected_with_width(body, token, token.len(), expected);
}

fn rejected_with_width(body: &str, marker: &str, width: usize, expected: &str) {
    let start = body.rfind(marker).expect("written offending token");
    refused(
        body,
        CompleteCause::UnexpectedToken,
        Span {
            start,
            end: start + width,
        },
        expected,
    );
}

fn unfinished(body: &str, expected: &str) {
    refused(
        body,
        CompleteCause::UnexpectedEnd,
        Span {
            start: body.len(),
            end: body.len(),
        },
        expected,
    );
}

fn refused(body: &str, cause: CompleteCause, relative: Span, expected: &str) {
    let source = format!("{PREFIX}{body}");
    let parsed = parse(&source);
    assert!(!parsed.is_admissible(), "near-miss admitted: {body}");
    assert_eq!(parsed.cst().render(), source.as_bytes());
    let diagnostic = parsed
        .diagnostics()
        .first()
        .expect("the first malformed declaration token");
    assert_eq!(
        diagnostic.code,
        CompleteCode::InvalidSyntax,
        "{body}: {diagnostic:?}"
    );
    assert_eq!(diagnostic.cause, cause, "{body}: {diagnostic:?}");
    assert_eq!(diagnostic.source, identity());
    assert_eq!(diagnostic.path, PATH);
    assert_eq!(
        diagnostic.byte_span(),
        Some(Span {
            start: PREFIX.len() + relative.start,
            end: PREFIX.len() + relative.end
        }),
        "{body}: {diagnostic:?}"
    );
    assert_eq!(diagnostic.message, expected, "{body}");
}

mod dimension_equivalence {
    use super::*;

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
    fn accepted_counterpart() {
        accepted(
            "dimension ClockTime ~ ObservedTime by 1 second = rational(1, 10) tick;",
            Production::DimensionEquivalence,
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn a_different_keyword_cannot_replace_by() {
        rejected(
            "dimension ClockTime ~ ObservedTime via 1 second = rational(1, 10) tick;",
            "via",
            "expected `by`",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn a_fractional_literal_is_not_an_exact_ratio_constructor() {
        rejected(
            "dimension ClockTime ~ ObservedTime by 1 second = 0.1 tick;",
            "0.1",
            "expected `decimal` or `rational` or unsigned integer",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn the_left_ratio_requires_its_unit() {
        rejected(
            "dimension ClockTime ~ ObservedTime by 1 = rational(1, 10) tick;",
            "=",
            "expected identifier",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn two_ratios_cannot_omit_the_equals_separator() {
        rejected(
            "dimension ClockTime ~ ObservedTime by 1 second rational(1, 10) tick;",
            "rational",
            "expected `=`",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn eof_cannot_replace_the_terminating_semicolon() {
        unfinished(
            "dimension ClockTime ~ ObservedTime by 1 second = rational(1, 10) tick",
            "expected `;`",
        );
    }
}

mod unit_declaration {
    use super::*;

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
    fn accepted_counterpart() {
        accepted(
            "unit minute: Time = rational(60, 1);",
            Production::UnitDeclaration,
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn a_plain_number_cannot_replace_the_exact_constructor() {
        rejected(
            "unit minute: Time = 60;",
            "60",
            "expected `decimal` or `rational`",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn a_unit_cannot_omit_its_dimension_separator() {
        rejected("unit minute = rational(60, 1);", "=", "expected `:`");
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn a_dimension_requires_a_name_rather_than_a_number() {
        rejected(
            "unit minute: 9 = rational(60, 1);",
            "9",
            "expected identifier",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn an_exact_constructor_cannot_replace_the_equals_separator() {
        rejected(
            "unit minute: Time rational(60, 1);",
            "rational",
            "expected `=`",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn eof_cannot_replace_the_terminating_semicolon() {
        unfinished("unit minute: Time = rational(60, 1)", "expected `;`");
    }
}

mod enum_declaration {
    use super::*;

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
    fn accepted_counterpart_includes_the_admitted_trailing_comma() {
        accepted(
            "ordered enum Shade { pale, dark, }",
            Production::EnumDeclaration,
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn the_body_requires_a_member() {
        rejected("ordered enum Shade { }", "}", "expected identifier");
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn adjacent_members_require_the_comma_separator() {
        rejected("ordered enum Shade { pale dark }", "dark", "expected `}`");
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn ordered_cannot_follow_the_name() {
        rejected(
            "enum Shade ordered { pale, dark, }",
            "ordered",
            "expected `{`",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn eof_cannot_close_the_body() {
        unfinished("ordered enum Shade { pale, dark,", "expected `}`");
    }
}

mod record_declaration {
    use super::*;

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
    fn accepted_counterpart() {
        accepted(
            "record Parcel { mass: Integer; label: Boolean; }",
            Production::RecordDeclaration,
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn the_body_requires_a_field() {
        rejected("record Parcel { }", "}", "expected identifier");
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn a_field_requires_its_type() {
        rejected_with_width(
            "record Parcel { mass: ; label: Boolean; }",
            "; label",
            1,
            "expected a declaration",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn equals_cannot_replace_the_field_colon() {
        rejected(
            "record Parcel { mass = Integer; label: Boolean; }",
            "=",
            "expected `:`",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn the_next_field_cannot_replace_the_field_semicolon() {
        rejected(
            "record Parcel { mass: Integer label: Boolean; }",
            "label",
            "expected `;`",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn eof_cannot_replace_the_closing_brace() {
        unfinished(
            "record Parcel { mass: Integer; label: Boolean;",
            "expected `}`",
        );
    }
}

mod tuple_declaration {
    use super::*;

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
    fn accepted_counterpart() {
        accepted(
            "tuple Coordinates(Integer, Boolean);",
            Production::TupleDeclaration,
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn the_tuple_requires_an_element_type() {
        rejected("tuple Coordinates();", ")", "expected a declaration");
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn a_tuple_cannot_have_a_trailing_comma() {
        rejected(
            "tuple Coordinates(Integer, Boolean,);",
            ")",
            "expected a declaration",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn a_semicolon_cannot_separate_element_types() {
        rejected_with_width(
            "tuple Coordinates(Integer; Boolean);",
            "; Boolean",
            1,
            "expected `)`",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn eof_cannot_replace_the_tuple_terminator() {
        unfinished("tuple Coordinates(Integer, Boolean)", "expected `;`");
    }
}

mod alias_declaration {
    use super::*;

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
    fn accepted_counterpart() {
        accepted("type Capacity = Int[0, 9];", Production::AliasDeclaration);
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn the_type_cannot_replace_the_equals_separator() {
        rejected("type Capacity Int[0, 9];", "Int", "expected `=`");
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn the_alias_requires_a_type() {
        rejected("type Capacity = ;", ";", "expected a declaration");
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn equals_cannot_replace_the_alias_name() {
        rejected(
            "type = Int[0, 9];",
            "=",
            "expected type name other than `Clock`",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn a_second_equals_cannot_replace_the_type() {
        rejected(
            "type Capacity = = Int[0, 9];",
            "=",
            "expected a declaration",
        );
    }

    #[test]
    #[trace("FSpec-FR-339-AC-1", "FR-002-AC-6")]
    fn eof_cannot_replace_the_alias_terminator() {
        unfinished("type Capacity = Int[0, 9]", "expected `;`");
    }
}
