// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-026: typed native result views; upstream identities retain their serializers.

use filament_contract_model as ir;
use fsl_foundation::{LocatedSpan, SourceIdentity};
use serde::Serialize;
use std::borrow::Cow;

pub(super) use fsl_foundation::wire_format::WireFormat as Format;

pub(super) enum Stage {
    File,
    Request,
    Output,
    Envelope,
    Intake,
    Native(fsl_foundation::Phase),
    /// The spine stage that refused a `1-draft` program.
    Spine(fsl_replay::spine::SpineStage),
    /// The stage a `1-draft` spine `run` refused at: the wrapped compile
    /// refusal's own stage, or `call` (FR-100).
    SpineRun(&'static str),
}

impl Serialize for Stage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::File => "file",
            Self::Request => "request",
            Self::Output => "output",
            Self::Envelope => "envelope",
            Self::Intake => "intake",
            Self::Native(phase) => phase.as_str(),
            Self::Spine(stage) => stage.as_str(),
            Self::SpineRun(stage) => stage,
        })
    }
}

#[derive(Serialize)]
pub(super) struct Diagnostic<'a> {
    pub phase: &'static str,
    pub code: &'static str,
    pub message: &'a str,
    pub source: &'a SourceIdentity,
    pub path: &'a str,
    pub span: LocatedSpan,
    pub upstream: Option<&'a ir::Diagnostic>,
    /// FR-001: `cause` and `label`, on an `invalid_source_identity`
    /// refusal only.
    #[serde(flatten)]
    pub identity: Option<fsl_foundation::source::IdentityCauseFields>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum Details<'a> {
    None,
    /// FR-960: the claim of a `call.requirements` entry naming no item of
    /// the call.
    Claim {
        claim: &'a serde_json::Value,
    },
    Io {
        path: Cow<'a, str>,
        os_code: Option<i32>,
    },
    Json {
        line: usize,
        column: usize,
    },
    FileBytes {
        limit: &'static str,
    },
    ConfiguredBytes {
        limit: &'static str,
        maximum: usize,
        option: &'static str,
    },
    FixedBytes {
        limit: &'static str,
        maximum: usize,
    },
    FileCount {
        limit: &'static str,
        category: &'static str,
        requested: usize,
        remaining: usize,
        maximum: usize,
    },
    Identifier(&'a ir::Diagnostic),
    Native(Diagnostic<'a>),
    Spine {
        source: &'a SourceIdentity,
        path: &'a str,
        span: Option<LocatedSpan>,
    },
    /// FR-640: a `clocks` selection that does not fit the program, naming
    /// the cause, the clock and, where one declaration is at fault, its
    /// name and profile, with the program locus.
    ClockSelection {
        cause: &'static str,
        clock: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        declaration: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        profile: Option<&'static str>,
        source: &'a SourceIdentity,
        path: &'a str,
        span: Option<LocatedSpan>,
    },
    /// FR-286 "Diagnostic details": a stage-`call` refusal's closed object,
    /// member for member what the outcome diagnostic carries.
    Object(serde_json::Map<String, serde_json::Value>),
    /// FR-049-AC-2: the call's entry refused the object environment,
    /// naming the object and, when one attribute is at fault, the attribute.
    ObjectEnvironment {
        object: Option<&'a str>,
        field: Option<&'a str>,
    },
}

/// FR-100: the program source as `spine-run-result/1` renders one -- its
/// four FR-001 labels and authored path. Narrower
/// than [`Source`]: no formal document/revision member.
#[derive(Serialize)]
pub(super) struct RunSource<'a> {
    #[serde(flatten)]
    pub identity: &'a SourceIdentity,
    pub path: &'a str,
}

/// FR-096/FR-100: a `refused` record's or cause's resolved locus -- the
/// path of the source the region belongs to, and its span rendered over that
/// source.
#[derive(Serialize)]
pub(super) struct SpineLocus {
    pub file: String,
    pub span: LocatedSpan,
}

/// FR-096/FR-100: the declaration and child-index path a `refused` outcome
/// arose at (`Evaluation.location`).
#[derive(Serialize)]
pub(super) struct SpineLocation {
    pub origin: SpineOrigin,
    pub path: Vec<usize>,
}

/// FR-096/FR-100's `location.origin`.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(super) enum SpineOrigin {
    Body {
        function: String,
        index: usize,
    },
    Measure {
        function: String,
        index: usize,
    },
    Expression,
    TypeDeclaration {
        name: String,
    },
    StateClause {
        clause: String,
        index: usize,
    },
    ProtocolAttempt {
        protocol: usize,
        attempt: usize,
    },
    ModelOperationClause {
        operation: String,
        clause: &'static str,
        position: usize,
    },
}

/// FR-100's `outcome` member. `Refused`'s members follow the refusal
/// table's two rows exactly, by which of `fields`/`locus` are present:
/// `code` and `cause` are on every refused outcome (FR-100's refusal table
/// gives both members to a record row and a family-no-record row alike; no
/// row omits either), a record row also carries `fields`/`locus` (plus
/// `location` when known), and a family-no-record row carries `location`
/// alone besides `code`/`cause`.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum SpineOutcome {
    Completed {
        /// FR-650's run value form.
        value: serde_json::Value,
    },
    Refused {
        code: &'static str,
        cause: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        fields: Option<std::collections::BTreeMap<&'static str, String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        locus: Option<SpineLocus>,
        #[serde(skip_serializing_if = "Option::is_none")]
        location: Option<SpineLocation>,
    },
    Undefined {
        reason: &'static str,
    },
    Incomplete {
        limit: &'static str,
    },
}

/// The `spine-run-result/1` outcome document (FR-100).
#[derive(Serialize)]
pub(super) struct SpineRunReport<'a> {
    pub format: Format,
    pub source: RunSource<'a>,
    pub function: &'a str,
    pub outcome: SpineOutcome,
    /// FR-963: one FR-621 entry per settled requirement item, present
    /// exactly when the call carried `requirements`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirements: Option<Vec<serde_json::Value>>,
    /// FR-963: FR-635's registry the items were negotiated against.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub(super) struct Failure<'a> {
    pub format: Format,
    pub stage: Stage,
    pub code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<&'a str>,
    pub message: String,
    pub details: Details<'a>,
    pub basis: &'static str,
}

/// Artifact commands retain their completion status on stderr. The `run` command uses the closed command-error shape above.
#[derive(Serialize)]
pub(super) struct ArtifactFailure<'a> {
    #[serde(flatten)]
    pub failure: Failure<'a>,
    pub status: ArtifactFailureStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ArtifactFailureStatus {
    Refused,
    Incomplete,
}
