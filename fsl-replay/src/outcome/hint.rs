// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-286 fix-hint catalog types and lookup key.

use filament_semantic_value::catalog::CatalogCode;

/// External semantic-value cause pairs needed by outcome diagnostics.
///
/// The upstream crate does not yet publish its own cause-pair catalog.
pub const SEMANTIC_VALUE_PAIRS: &[CatalogCode] = &[
    CatalogCode::new("invalid_runtime_input", "invalid-value"),
    CatalogCode::new("invalid_runtime_input", "missing-member"),
    CatalogCode::new("runtime_invariant", "established-invariant-broken"),
];
use fsl_foundation::diagnostic::Code;

/// One catalog entry keyed by diagnostic code and optional cause.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HintRow {
    /// The diagnostic code.
    pub code: Code,
    /// The typed cause, or `None` for a cause-less diagnostic.
    pub cause: Option<&'static str>,
    /// The imperative sentence shown to the user.
    pub text: &'static str,
}

/// The fix-hint catalog for common source and request mistakes (FR-286).
pub const HINTS: &[HintRow] = &[
    HintRow {
        code: Code::InvalidSyntax,
        cause: Some("unexpected-token"),
        text: "Remove the stray token or replace it with the expected syntax.",
    },
    HintRow {
        code: Code::InvalidSyntax,
        cause: Some("unexpected-end"),
        text: "Close the unfinished block or complete the declaration.",
    },
    HintRow {
        code: Code::InvalidSyntax,
        cause: Some("invalid-token"),
        text: "Replace the character with one admitted by the language.",
    },
    HintRow {
        code: Code::InvalidSyntax,
        cause: Some("invalid-escape"),
        text: "Use a valid escape sequence in the text literal.",
    },
    HintRow {
        code: Code::IllTyped,
        cause: Some("type-mismatch"),
        text: "Give the expression a value of the required type.",
    },
    HintRow {
        code: Code::IllTyped,
        cause: Some("unit-mismatch"),
        text: "Use quantities with matching units.",
    },
    HintRow {
        code: Code::IllTyped,
        cause: Some("operator-ineligible"),
        text: "Use an operator that accepts this type.",
    },
    HintRow {
        code: Code::IllTyped,
        cause: Some("non-boolean-root"),
        text: "Make the clause body evaluate to a Boolean value.",
    },
    HintRow {
        code: Code::MissingImport,
        cause: Some("missing-selection"),
        text: "Import the package that supplies the selected name.",
    },
    HintRow {
        code: Code::MissingDeclaration,
        cause: Some("missing-name"),
        text: "Declare the name or import the package that declares it.",
    },
    HintRow {
        code: Code::MissingDeclaration,
        cause: Some("missing-selection"),
        text: "Select the required declaration or law.",
    },
    HintRow {
        code: Code::AmbiguousDeclaration,
        cause: Some("ambiguous-name"),
        text: "Select one import that declares the name.",
    },
    HintRow {
        code: Code::UndefinedExpression,
        cause: Some("unproved-nonzero"),
        text: "Prove that the divisor is nonzero before using it.",
    },
    HintRow {
        code: Code::UndefinedExpression,
        cause: Some("unproved-presence"),
        text: "Prove that the value is present before reading it.",
    },
    HintRow {
        code: Code::UnsupportedConstruct,
        cause: Some("declaration-form"),
        text: "Rewrite the declaration using a form admitted by the selected profile.",
    },
    HintRow {
        code: Code::UnsupportedConstruct,
        cause: Some("expression-form"),
        text: "Rewrite the expression using a form admitted by the selected profile.",
    },
    HintRow {
        code: Code::UnsupportedConstruct,
        cause: Some("not-yet-implemented"),
        text: "Choose a supported construct or select an implementation that provides it.",
    },
    HintRow {
        code: Code::UnknownProfile,
        cause: Some("unsupported-selection"),
        text: "Choose a profile or selection value supported by this request.",
    },
    HintRow {
        code: Code::StageLimitExceeded,
        cause: Some("input-bytes-exceeded"),
        text: "Reduce the input size or raise the input-bytes limit.",
    },
    HintRow {
        code: Code::StageLimitExceeded,
        cause: Some("work-budget-exceeded"),
        text: "Reduce the requested work or raise the work-budget limit.",
    },
    HintRow {
        code: Code::InvalidRuntimeInput,
        cause: Some("missing-member"),
        text: "Add the required member to the request document.",
    },
    HintRow {
        code: Code::InvalidRuntimeInput,
        cause: Some("unknown-member"),
        text: "Remove the member that this request format does not define.",
    },
    HintRow {
        code: Code::IoError,
        cause: None,
        text: "Check that the path exists and that the process can read or write it.",
    },
];

/// A defect in the static fix-hint catalog.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HintCatalogError {
    /// A cause does not belong to any typed cause in the supplied catalog.
    #[error("unknown hint pair {code}/{cause}")]
    UnknownPair {
        /// Diagnostic code.
        code: Code,
        /// Catalog cause.
        cause: &'static str,
    },
    /// Two rows use the same exact key.
    #[error("duplicate hint pair {code}")]
    DuplicatePair {
        /// Diagnostic code.
        code: Code,
        /// Catalog cause, when present.
        cause: Option<&'static str>,
    },
    /// The text is empty, too long, contains a control, or lacks a final period.
    #[error("malformed hint text for {code}")]
    MalformedText {
        /// Diagnostic code.
        code: Code,
        /// Catalog cause, when present.
        cause: Option<&'static str>,
    },
}

/// Check every hint key against the typed causes and validate its one-line text.
pub fn check_hint_catalog(rows: &[HintRow], known: &[CatalogCode]) -> Result<(), HintCatalogError> {
    for (index, row) in rows.iter().enumerate() {
        if let Some(cause) = row.cause {
            let pair = CatalogCode::new(row.code.as_str(), cause);
            if !known.contains(&pair) {
                return Err(HintCatalogError::UnknownPair {
                    code: row.code,
                    cause,
                });
            }
        }
        if rows[..index]
            .iter()
            .any(|earlier| earlier.code == row.code && earlier.cause == row.cause)
        {
            return Err(HintCatalogError::DuplicatePair {
                code: row.code,
                cause: row.cause,
            });
        }
        let bytes = row.text.as_bytes();
        if bytes.is_empty()
            || bytes.len() > 200
            || !row.text.ends_with('.')
            || row
                .text
                .chars()
                .any(|character| matches!(u32::from(character), 0x0000..=0x001f | 0x007f..=0x009f))
        {
            return Err(HintCatalogError::MalformedText {
                code: row.code,
                cause: row.cause,
            });
        }
    }
    Ok(())
}

/// Find a hint by the exact `(code, cause)` key.
pub fn hint_for(code: Code, cause: Option<&str>) -> Option<&'static str> {
    HINTS
        .iter()
        .find(|row| row.code == code && row.cause == cause)
        .map(|row| row.text)
}
