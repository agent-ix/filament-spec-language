// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2198: syntax errors use bounded, user-facing descriptions.
use fsl_cst::{CompleteCode, Limits};
use fsl_foundation::SourceIdentity;
use ix_trace_rs::trace;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";

fn messages(text: &str) -> Vec<String> {
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "test:qsl-2198-syntax-messages"),
        "messages.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("source is valid UTF-8");
    assert!(!parsed.is_admissible(), "{text}");
    parsed
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == CompleteCode::InvalidSyntax)
        .map(|diagnostic| diagnostic.message.to_string())
        .collect()
}

#[trace("TC-2198", "FR-762-AC-1")]
#[test]
fn long_expected_sets_use_a_short_category_phrase() {
    let output = messages(&format!("{HEADER}{{\n"));
    assert!(
        output
            .iter()
            .any(|message| message == "expected a declaration"),
        "{output:?}"
    );
    assert!(
        output.iter().all(|message| message.len() < 80),
        "{output:?}"
    );
}

#[trace("TC-2198", "FR-762-AC-2")]
#[test]
fn population_peek_spelling_and_its_candidates_are_hidden() {
    let output = messages(&format!(
        "{HEADER}function f using v(x: Integer): Integer pure {{ count + }}\n"
    ));
    assert!(
        !output.iter().any(|message| message.contains("Peek")),
        "{output:?}"
    );
    for word in ["count", "exists", "all", "sum", "only", "seq"] {
        assert!(
            !output.iter().any(|message| message.contains(word)),
            "{output:?}"
        );
    }
}

#[trace("TC-2198", "FR-762-AC-3")]
#[test]
fn expression_start_does_not_advertise_decimal_literal() {
    let output = messages(&format!(
        "{HEADER}function f using v(x: Integer): Integer pure {{ ) }}\n"
    ));
    assert!(
        output
            .iter()
            .any(|message| message.starts_with("expected ")),
        "{output:?}"
    );
    assert!(
        !output
            .iter()
            .any(|message| message.contains("decimal literal")),
        "{output:?}"
    );
}

#[trace("TC-2198", "FR-762-AC-4")]
#[test]
fn uppercase_hex_reports_the_lowercase_digit_rule() {
    let output = messages(&format!(
        "{HEADER}function f using v(): Float32 pure {{ float32(bits: 0xFF000000) }}\n"
    ));
    assert!(
        output
            .iter()
            .any(|message| message == "expected a lowercase hex digit"),
        "{output:?}"
    );
}

#[trace("TC-2198", "FR-762-AC-5")]
#[test]
fn reserved_identifier_diagnostic_names_the_reserved_word() {
    for word in ["first", "bounded"] {
        let output = messages(&format!("{HEADER}record {word} {{ datum: Integer; }}\n"));
        assert!(
            output
                .iter()
                .any(|message| message == &format!("`{word}` is a reserved word")),
            "{word}: {output:?}"
        );
    }
}

#[trace("TC-2198", "FR-762-AC-6")]
#[test]
fn missing_semicolon_still_reports_the_single_expected_alternative() {
    let output = messages(&format!("{HEADER}record R {{ datum: Integer }}\n"));
    assert!(
        output.iter().any(|message| message == "expected `;`"),
        "{output:?}"
    );
}
