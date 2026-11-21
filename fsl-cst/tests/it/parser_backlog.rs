// SPDX-License-Identifier: AGPL-3.0-or-later
//! Regressions for the parser and formatter backlog (QSL-2599, QSL-2621,
//! QSL-2681, QSL-2684, QSL-2685, QSL-2687, QSL-2688) whose owning vectors do
//! not already pin the behavior exactly.

use fsl_cst::{CompleteCause, CompleteCode, Limits, ParsedSource};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";

fn parse(text: &str) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity::new("producer-tests", "original:diagnostics"),
        "producer.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("the source reads")
}

/// FR-762-AC-2 (QSL-2681): `$` is its own lexical refusal, `invalid-token` at
/// the `$`, and no parse diagnostic accompanies it. The catalog vector in
/// `complete_grammar` authors the reserved word `value` as a field name, so
/// its first diagnostic is that word; this source uses an ordinary name.
#[trace("TC-2149", "FR-762-AC-2")]
#[test]
fn a_dollar_token_is_an_invalid_token_and_nothing_else() {
    let text = format!("{HEADER}record Broken {{ datum: Integer; }} $");
    let parsed = parse(&text);
    let [refusal] = parsed.diagnostics() else {
        panic!("one refusal: {:?}", parsed.diagnostics());
    };
    assert_eq!(
        (refusal.code, refusal.cause),
        (CompleteCode::InvalidSyntax, CompleteCause::InvalidToken)
    );
    let start = text.rfind('$').expect("the dollar");
    assert_eq!(
        refusal.byte_span(),
        Some(Span {
            start,
            end: start + 1
        })
    );
}

/// FR-762 "Lexical errors" (QSL-2687): an invalid escape is the unit's one
/// diagnostic, `invalid-escape` at the string; the valid profile before it is
/// not reported as a selection-identity refusal.
#[trace("FR-002-AC-6", "FR-762-AC-2")]
#[test]
fn an_invalid_escape_is_not_followed_by_a_selection_cascade() {
    let text = format!("{HEADER}// durée\nenum Caption {{ Label = \"bad\\q\" }}");
    let parsed = parse(&text);
    let [refusal] = parsed.diagnostics() else {
        panic!("one refusal: {:?}", parsed.diagnostics());
    };
    assert_eq!(
        (refusal.code, refusal.cause),
        (CompleteCode::InvalidSyntax, CompleteCause::InvalidEscape)
    );
    let span = refusal.byte_span().expect("located");
    assert_eq!(&text[span.start..span.end], "\"bad\\q\"");
}

/// FSpec FR-339-AC-4 (QSL-2621): a comment between `#` and the key refuses at
/// the complete key token, a solid key is admitted, and a gap before `#`
/// refuses at the `#`.
#[trace("FSpec-FR-339-AC-4")]
#[test]
fn the_reference_key_gap_cases_keep_their_loci() {
    let unit = |value: &str| format!("{HEADER}function inspect(): Integer pure {{ {value} }}");
    let solid = unit("Store::Row#\"r\"");
    assert!(parse(&solid).is_admissible(), "{solid}");
    let gap = unit("Store::Row #\"r\"");
    let parsed = parse(&gap);
    let start = gap.find('#').expect("the hash");
    assert_eq!(
        parsed.diagnostics().first().and_then(|d| d.byte_span()),
        Some(Span {
            start,
            end: start + 1
        }),
        "{:?}",
        parsed.diagnostics()
    );
}

/// FSpec FR-339-AC-5 (QSL-2684): one declaration with a reserved name is one
/// skipped region and gives one diagnostic, at the reserved word, whichever
/// reserved word is used and whatever follows the declaration.
#[trace("FR-762-AC-4", "FSpec-FR-339-AC-5")]
#[test]
fn a_reserved_record_name_is_one_diagnostic_and_the_next_declaration_parses() {
    let text = format!(
        "{HEADER}record tuple {{ datum: Integer; }}\nrecord Fine {{ datum: Integer; }}\nrecord record {{ datum: Integer; }}\n"
    );
    let parsed = parse(&text);
    let starts: Vec<usize> = parsed
        .diagnostics()
        .iter()
        .map(|d| d.byte_span().expect("located").start)
        .collect();
    assert_eq!(
        starts,
        [
            text.find("tuple").expect("first"),
            text.rfind("record record").expect("second") + "record ".len()
        ],
        "{:?}",
        parsed.diagnostics()
    );
}

/// FSpec FR-500-AC-1 (QSL-2685): an unsupported edition is the sole
/// diagnostic even when later declarations hold syntax errors, and the
/// language refusal keeps the same priority.
#[trace("FSpec-FR-500-AC-1")]
#[test]
fn an_unsupported_header_selection_outranks_later_syntax_errors() {
    for (header, code, literal) in [
        (
            "language \"ix:native\" edition \"0-draft\";",
            CompleteCode::UnknownEdition,
            "\"0-draft\"",
        ),
        (
            "language \"ix:other\" edition \"1-draft\";",
            CompleteCode::UnknownLanguage,
            "\"ix:other\"",
        ),
    ] {
        let text = format!("{header}\nrecord A {{ x: Integer y: Integer; }}\n");
        let parsed = parse(&text);
        let [refusal] = parsed.diagnostics() else {
            panic!("one refusal: {:?}", parsed.diagnostics());
        };
        assert_eq!(refusal.code, code);
        assert_eq!(refusal.cause, CompleteCause::UnsupportedSelection);
        let span = refusal.byte_span().expect("located");
        assert_eq!(&text[span.start..span.end], literal);
    }
}
