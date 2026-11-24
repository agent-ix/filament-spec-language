// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-710's layer-2 parsing seam for model-unit intake.
//!
//! The forms crate owns access to layer-1 parsing for downstream layers. These
//! wrappers expose only the source, token and diagnostic facts model intake
//! needs; no layer-1 CST type appears in this module's public API.

use fsl_foundation::{Code, Phase, Source, SourceIdentity, Span};
use std::fmt;

/// The stable complete-source diagnostic code.
pub type CompleteCode = Code;

/// A complete-source diagnostic cause, retained by its stable catalog tag.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompleteCause(&'static str);

#[allow(non_upper_case_globals)]
impl CompleteCause {
    /// An unexpected token.
    pub const UnexpectedToken: Self = Self("unexpected-token");
    /// A required token was absent at end of input.
    pub const UnexpectedEnd: Self = Self("unexpected-end");
    /// The source contains bytes that are not a token.
    pub const InvalidToken: Self = Self("invalid-token");
    /// The parser reached its diagnostic-list bound.
    pub const DiagnosticListCut: Self = Self("diagnostic-list-cut");
    /// A string contains an invalid escape.
    pub const InvalidEscape: Self = Self("invalid-escape");
    /// A declaration form is not admitted.
    pub const DeclarationForm: Self = Self("declaration-form");
    /// A selection is unsupported.
    pub const UnsupportedSelection: Self = Self("unsupported-selection");
    /// A resource budget denied the next charge.
    pub const InsufficientNextCharge: Self = Self("insufficient-next-charge");
    /// A selected definition is missing.
    pub const MissingSelection: Self = Self("missing-selection");
    /// A name is ambiguous.
    pub const AmbiguousName: Self = Self("ambiguous-name");
    /// An identity has conflicting authorities.
    pub const ConflictingAuthority: Self = Self("conflicting-authority");
    /// A package member is duplicated.
    pub const DuplicateMember: Self = Self("duplicate-member");
    /// A package value is invalid.
    pub const InvalidValue: Self = Self("invalid-value");
    /// A definition graph contains a cycle.
    pub const DefinitionCycle: Self = Self("definition-cycle");
    /// A selected feature set is inconsistent.
    pub const FeatureSetMismatch: Self = Self("feature-set-mismatch");
    /// A required feature is unknown.
    pub const UnknownFeature: Self = Self("unknown-feature");
    /// A required feature is unsupported.
    pub const UnsupportedFeature: Self = Self("unsupported-feature");
    /// A model selection is wrong.
    pub const WrongModelSelection: Self = Self("wrong-model-selection");
    /// A model declaration is malformed.
    pub const MalformedDeclaration: Self = Self("malformed-declaration");
    /// A model reference has the wrong exported kind.
    pub const WrongExport: Self = Self("wrong-export");
    /// A model declaration is missing.
    pub const MissingDeclaration: Self = Self("missing-declaration");
    /// A model identity has conflicting bindings.
    pub const ConflictingBinding: Self = Self("conflicting-binding");
    /// An established runtime invariant failed.
    pub const EstablishedInvariantBroken: Self = Self("established-invariant-broken");

    /// The stable complete-source cause tag.
    pub const fn as_str(self) -> &'static str {
        self.0
    }

    fn from_layer_one(cause: fsl_cst::CompleteCause) -> Self {
        Self(cause.as_str())
    }
}

/// A located source diagnostic, with only the fields needed by model intake.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompleteDiagnostic {
    /// Phase that observed the failure.
    pub phase: Phase,
    /// Stable machine-readable classification.
    pub code: CompleteCode,
    /// Human-readable detail.
    pub message: String,
    cause: CompleteCause,
    span: Option<Span>,
}

impl CompleteDiagnostic {
    /// The stable typed cause tag.
    pub const fn cause(&self) -> CompleteCause {
        self.cause
    }

    /// The byte span of the diagnostic, or `None` if it names no source region.
    pub const fn byte_span(&self) -> Option<Span> {
        self.span
    }

    fn from_layer_one(diagnostic: fsl_cst::CompleteDiagnostic) -> Self {
        let span = diagnostic.byte_span();
        Self {
            phase: diagnostic.phase,
            code: diagnostic.code,
            message: diagnostic.message,
            cause: CompleteCause::from_layer_one(diagnostic.cause),
            span,
        }
    }
}

impl fmt::Display for CompleteDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.message)
    }
}

/// The layer-1 limits used for model-unit parsing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    inner: fsl_cst::Limits,
}

impl Default for Limits {
    fn default() -> Self {
        let inner = fsl_cst::Limits::default();
        Self { inner }
    }
}

impl Limits {
    /// Maximum source size accepted by the parser.
    pub const fn source_bytes(self) -> usize {
        self.inner.source_bytes
    }

    /// These limits with a different source-byte bound.
    pub fn with_source_bytes(mut self, source_bytes: usize) -> Self {
        self.inner = self.inner.with_source_bytes(source_bytes);
        self
    }
}

/// An admitted or recovering source parse, without exposing its CST type.
#[derive(Clone, Debug)]
pub struct ParsedSource(fsl_cst::ParsedSource);

impl ParsedSource {
    /// The exact source bytes and identity.
    pub fn source(&self) -> &Source {
        self.0.source()
    }

    /// A read-only token view of the lossless CST.
    pub fn cst(&self) -> CstView<'_> {
        CstView(self.0.cst())
    }

    /// Whether the parse has no syntax diagnostics or recovery.
    pub fn is_admissible(&self) -> bool {
        self.0.is_admissible()
    }

    /// The parse diagnostics in source order.
    pub fn diagnostics(&self) -> Vec<CompleteDiagnostic> {
        self.0
            .diagnostics()
            .iter()
            .cloned()
            .map(CompleteDiagnostic::from_layer_one)
            .collect()
    }
}

/// A read-only view of the token leaves needed by model intake.
pub struct CstView<'a>(&'a fsl_cst::LosslessCst);

impl CstView<'_> {
    /// Token leaves, retaining their source order and exact spelling.
    pub fn tokens(&self) -> Vec<CstToken<'_>> {
        self.0.tokens().iter().map(CstToken).collect()
    }
}

/// A borrowed token leaf without exposing the layer-1 token type.
#[derive(Clone, Copy)]
pub struct CstToken<'a>(&'a fsl_cst::CstToken);

impl CstToken<'_> {
    /// Its significant-token/trivia classification.
    pub fn class(&self) -> TokenClass {
        if self.0.class() == fsl_cst::TokenClass::Token {
            TokenClass::Token
        } else {
            TokenClass::Other
        }
    }

    /// Exact token bytes.
    pub fn spelling(&self) -> &[u8] {
        self.0.spelling()
    }

    /// Source byte span.
    pub fn span(&self) -> Span {
        self.0.span()
    }
}

/// The token distinction model intake needs from a lossless CST.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenClass {
    /// A significant grammar token.
    Token,
    /// Any retained trivia or invalid token.
    Other,
}

/// Parse a source identity and byte buffer, retaining its lossless parse.
pub fn parse(
    identity: SourceIdentity,
    path: impl Into<String>,
    bytes: &[u8],
    limits: Limits,
) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    fsl_cst::parse(identity, path, bytes, limits.inner)
        .map(ParsedSource)
        .map_err(|diagnostic| Box::new(CompleteDiagnostic::from_layer_one(*diagnostic)))
}

/// Parse an already-labelled source.
pub fn parse_source(
    source: Source,
    limits: Limits,
) -> Result<ParsedSource, Box<CompleteDiagnostic>> {
    fsl_cst::parse_source(source, limits.inner)
        .map(ParsedSource)
        .map_err(|diagnostic| Box::new(CompleteDiagnostic::from_layer_one(*diagnostic)))
}

/// Layer-1 diagnostic constructors needed by model intake.
pub mod diagnostic {
    use super::{CompleteCause, CompleteCode, CompleteDiagnostic};
    use fsl_foundation::{Phase, Source, SourceIdentity, Span, SyntaxLimit};

    /// Read exact source bytes under the supplied source-byte bound.
    pub fn read_source(
        identity: SourceIdentity,
        path: impl Into<String>,
        bytes: &[u8],
        byte_limit: usize,
    ) -> Result<Source, Box<CompleteDiagnostic>> {
        fsl_cst::diagnostic::read_source(identity, path, bytes, byte_limit)
            .map_err(|diagnostic| Box::new(CompleteDiagnostic::from_layer_one(*diagnostic)))
    }

    /// Build a located source diagnostic.
    pub fn error(
        source: &Source,
        code: CompleteCode,
        cause: CompleteCause,
        phase: Phase,
        start: usize,
        end: usize,
        message: impl Into<String>,
    ) -> Box<CompleteDiagnostic> {
        let cause = layer_one_cause(cause);
        let diagnostic =
            fsl_cst::diagnostic::error(source, code, cause, phase, start, end, message);
        Box::new(CompleteDiagnostic::from_layer_one(*diagnostic))
    }

    /// Build a source-byte resource refusal at an exact span.
    pub fn resource_exhausted(
        source: &Source,
        phase: Phase,
        span: Span,
        limit: SyntaxLimit,
    ) -> Box<CompleteDiagnostic> {
        let diagnostic = fsl_cst::diagnostic::resource_exhausted(source, phase, span, limit);
        Box::new(CompleteDiagnostic::from_layer_one(*diagnostic))
    }

    fn layer_one_cause(cause: CompleteCause) -> fsl_cst::CompleteCause {
        use fsl_cst::CompleteCause as LayerOne;
        match cause.as_str() {
            "unexpected-token" => LayerOne::UnexpectedToken,
            "unexpected-end" => LayerOne::UnexpectedEnd,
            "invalid-token" => LayerOne::InvalidToken,
            "diagnostic-list-cut" => LayerOne::DiagnosticListCut,
            "invalid-escape" => LayerOne::InvalidEscape,
            "declaration-form" => LayerOne::DeclarationForm,
            "unsupported-selection" => LayerOne::UnsupportedSelection,
            "insufficient-next-charge" => LayerOne::InsufficientNextCharge,
            "missing-selection" => LayerOne::MissingSelection,
            "ambiguous-name" => LayerOne::AmbiguousName,
            "conflicting-authority" => LayerOne::ConflictingAuthority,
            "duplicate-member" => LayerOne::DuplicateMember,
            "invalid-value" => LayerOne::InvalidValue,
            "definition-cycle" => LayerOne::DefinitionCycle,
            "feature-set-mismatch" => LayerOne::FeatureSetMismatch,
            "unknown-feature" => LayerOne::UnknownFeature,
            "unsupported-feature" => LayerOne::UnsupportedFeature,
            "wrong-model-selection" => LayerOne::WrongModelSelection,
            "malformed-declaration" => LayerOne::MalformedDeclaration,
            "wrong-export" => LayerOne::WrongExport,
            "missing-declaration" => LayerOne::MissingDeclaration,
            "conflicting-binding" => LayerOne::ConflictingBinding,
            "established-invariant-broken" => LayerOne::EstablishedInvariantBroken,
            _ => unreachable!("the forms seam admits only closed layer-1 causes"),
        }
    }
}
