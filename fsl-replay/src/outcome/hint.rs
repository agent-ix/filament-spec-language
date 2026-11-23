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

/// FR-286 "Missing selections": the kind of the thing a
/// `missing_import`/`missing-selection` diagnostic says is missing.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MissingSelection {
    /// The domain package a `model` declaration selects.
    Model,
    /// The library an `import` declaration names.
    Library,
    /// A library, where the import names a sibling unit.
    SiblingUnit,
    /// An `import` of the unit for the library a call site names.
    UnimportedLibrary,
    /// An `import` of the unit for the request's `function_owner`.
    FunctionOwner,
    /// The byte-provision entry for a source the package reference names.
    ByteProvision,
    /// The definition of a selected extension.
    ExtensionDefinition,
    /// A source the package's graph names, absent from the source provision.
    Source,
    /// The domain package or library a checked package's bytes name.
    PackageReference,
}

impl MissingSelection {
    /// The nine kinds, in the order of FR-286 "Missing selections".
    pub const ALL: [Self; 9] = [
        Self::Model,
        Self::Library,
        Self::SiblingUnit,
        Self::UnimportedLibrary,
        Self::FunctionOwner,
        Self::ByteProvision,
        Self::ExtensionDefinition,
        Self::Source,
        Self::PackageReference,
    ];

    /// The wire spelling of the kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Library => "library",
            Self::SiblingUnit => "sibling-unit",
            Self::UnimportedLibrary => "unimported-library",
            Self::FunctionOwner => "function-owner",
            Self::ByteProvision => "byte-provision",
            Self::ExtensionDefinition => "extension-definition",
            Self::Source => "source",
            Self::PackageReference => "package-reference",
        }
    }

    /// The kind of the `missing_import`/`missing-selection` refusal of a
    /// domain package `model` intake, or `None` for any other refusal.
    pub fn of_model_refusal(
        refusal: &fsl_semantics::model::normalize::ModelRefusal,
    ) -> Option<Self> {
        matches!(
            refusal.cause,
            fsl_semantics::model::refusal::ModelRefusalCause::MissingSelection { .. }
        )
        .then_some(Self::Model)
    }

    /// The kind of the `missing_import`/`missing-selection` refusal of an
    /// assembly error, or `None` for any other cause.
    pub fn of_assembly_cause(cause: &fsl_semantics::check::AssemblyCause) -> Option<Self> {
        use fsl_semantics::check::AssemblyCause;
        match cause {
            AssemblyCause::UnadmittedModel { .. } => Some(Self::Model),
            AssemblyCause::UnsuppliedImport { .. } => Some(Self::Library),
            _ => None,
        }
    }
}

/// One catalog entry keyed by diagnostic code and optional cause.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HintRow {
    /// The diagnostic code.
    pub code: Code,
    /// The typed cause, or `None` for a cause-less diagnostic.
    pub cause: Option<&'static str>,
    /// The `selection` of a `missing_import`/`missing-selection` diagnostic,
    /// or `None` for a diagnostic with no `selection`.
    pub selection: Option<&'static str>,
    /// The imperative sentence shown to the user.
    pub text: &'static str,
}

/// The fix-hint catalog for common source and request mistakes (FR-286).
pub const HINTS: &[HintRow] = &[
    HintRow {
        code: Code::InvalidSyntax,
        cause: Some("unexpected-token"),
        selection: None,
        text: "Remove the stray token or replace it with the expected syntax.",
    },
    HintRow {
        code: Code::InvalidSyntax,
        cause: Some("unexpected-end"),
        selection: None,
        text: "Close the unfinished block or complete the declaration.",
    },
    HintRow {
        code: Code::InvalidSyntax,
        cause: Some("invalid-token"),
        selection: None,
        text: "Replace the character with one admitted by the language.",
    },
    HintRow {
        code: Code::InvalidSyntax,
        cause: Some("invalid-escape"),
        selection: None,
        text: "Use a valid escape sequence in the text literal.",
    },
    HintRow {
        code: Code::IllTyped,
        cause: Some("type-mismatch"),
        selection: None,
        text: "Give the expression a value of the required type.",
    },
    HintRow {
        code: Code::IllTyped,
        cause: Some("unit-mismatch"),
        selection: None,
        text: "Use quantities with matching units.",
    },
    HintRow {
        code: Code::IllTyped,
        cause: Some("operator-ineligible"),
        selection: None,
        text: "Use an operator that accepts this type.",
    },
    HintRow {
        code: Code::IllTyped,
        cause: Some("non-boolean-root"),
        selection: None,
        text: "Make the clause body evaluate to a Boolean value.",
    },
    HintRow {
        code: Code::MissingImport,
        cause: Some("missing-selection"),
        selection: None,
        text: "Import the package that supplies the selected name.",
    },
    HintRow {
        code: Code::MissingImport,
        cause: Some("missing-selection"),
        selection: Some("model"),
        text: "Supply the model in the request's models member, or with --model <identity>=<file> on fsl check.",
    },
    HintRow {
        code: Code::MissingImport,
        cause: Some("missing-selection"),
        selection: Some("library"),
        text: "Supply the library in the request's libraries member, or with --library <identity>=<file> on fsl check.",
    },
    HintRow {
        code: Code::MissingImport,
        cause: Some("missing-selection"),
        selection: Some("sibling-unit"),
        text: "Supply the imported unit as a library, not as a sibling unit.",
    },
    HintRow {
        code: Code::MissingImport,
        cause: Some("missing-selection"),
        selection: Some("unimported-library"),
        text: "Call a function of a library that the unit imports directly, or add the import.",
    },
    HintRow {
        code: Code::MissingImport,
        cause: Some("missing-selection"),
        selection: Some("function-owner"),
        text: "Name a function_owner that the program imports directly, or add the import.",
    },
    HintRow {
        code: Code::MissingImport,
        cause: Some("missing-selection"),
        selection: Some("byte-provision"),
        text: "Add a byte_provision entry for the named source to the replay request.",
    },
    HintRow {
        code: Code::MissingImport,
        cause: Some("missing-selection"),
        selection: Some("extension-definition"),
        text: "Add the selected extension's definition to the request's extensions member, or remove the selection.",
    },
    HintRow {
        code: Code::MissingImport,
        cause: Some("missing-selection"),
        selection: Some("source"),
        text: "Add the named source to the source provision the analyze call supplies.",
    },
    HintRow {
        code: Code::MissingImport,
        cause: Some("missing-selection"),
        selection: Some("package-reference"),
        text: "Supply the domain package or library that the package names, in the request's models or libraries member.",
    },
    HintRow {
        code: Code::MissingDeclaration,
        cause: Some("missing-name"),
        selection: None,
        text: "Declare the name or import the package that declares it.",
    },
    HintRow {
        code: Code::MissingDeclaration,
        cause: Some("missing-selection"),
        selection: None,
        text: "Select the required declaration or law.",
    },
    HintRow {
        code: Code::AmbiguousDeclaration,
        cause: Some("ambiguous-name"),
        selection: None,
        text: "Select one import that declares the name.",
    },
    HintRow {
        code: Code::UndefinedExpression,
        cause: Some("unproved-nonzero"),
        selection: None,
        text: "Prove that the divisor is nonzero before using it.",
    },
    HintRow {
        code: Code::UndefinedExpression,
        cause: Some("unproved-presence"),
        selection: None,
        text: "Prove that the value is present before reading it.",
    },
    HintRow {
        code: Code::UnsupportedConstruct,
        cause: Some("declaration-form"),
        selection: None,
        text: "Rewrite the declaration using a form admitted by the selected profile.",
    },
    HintRow {
        code: Code::UnsupportedConstruct,
        cause: Some("expression-form"),
        selection: None,
        text: "Rewrite the expression using a form admitted by the selected profile.",
    },
    HintRow {
        code: Code::UnsupportedConstruct,
        cause: Some("not-yet-implemented"),
        selection: None,
        text: "Choose a supported construct or select an implementation that provides it.",
    },
    HintRow {
        code: Code::UnknownProfile,
        cause: Some("unsupported-selection"),
        selection: None,
        text: "Choose a profile or selection value supported by this request.",
    },
    HintRow {
        code: Code::StageLimitExceeded,
        cause: Some("input-bytes-exceeded"),
        selection: None,
        text: "Reduce the input size or raise the input-bytes limit.",
    },
    HintRow {
        code: Code::StageLimitExceeded,
        cause: Some("work-budget-exceeded"),
        selection: None,
        text: "Reduce the requested work or raise the work-budget limit.",
    },
    HintRow {
        code: Code::InvalidRuntimeInput,
        cause: Some("missing-member"),
        selection: None,
        text: "Add the required member to the request document.",
    },
    HintRow {
        code: Code::InvalidRuntimeInput,
        cause: Some("unknown-member"),
        selection: None,
        text: "Remove the member that this request format does not define.",
    },
    HintRow {
        code: Code::IoError,
        cause: None,
        selection: None,
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
        if rows[..index].iter().any(|earlier| {
            earlier.code == row.code
                && earlier.cause == row.cause
                && earlier.selection == row.selection
        }) {
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

/// Find a hint by the exact `(code, cause, selection)` key.
pub fn hint_for(code: Code, cause: Option<&str>, selection: Option<&str>) -> Option<&'static str> {
    HINTS
        .iter()
        .find(|row| row.code == code && row.cause == cause && row.selection == selection)
        .map(|row| row.text)
}
