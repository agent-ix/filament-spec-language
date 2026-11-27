// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-663 and FR-096: the refusal locus of a `native-run-result/2` disposition,
//! its six tagged forms (FR-267 "Refusal locus wire").

use serde::{Deserialize, Serialize};

use super::{present, RunSpan};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
/// Decoded locus attached to a compile or selection refusal.
pub enum RefusalLocus {
    /// Region within an authored source.
    Region {
        /// Source identity.
        source: SourceRef,
        /// First byte of the region.
        start: u64,
        /// Byte after the region.
        end: u64,
    },
    /// Source path and line-column span.
    Located(Located),
    /// Named occurrence.
    Occurrence {
        /// Occurrence identifier.
        occurrence: String,
    },
    /// Pointer into an identified artifact.
    Artifact {
        /// Artifact identity.
        identity: SourceRef,
        /// Pointer within the artifact.
        pointer: String,
    },
    /// A position in a caller-named input operand.
    Input {
        /// The operand exactly as the caller wrote it.
        operand: String,
        /// Member path relative to a directory operand, present as null when absent.
        #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
        member: Option<Option<String>>,
        /// Pointer into the input, present as null when absent.
        #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
        pointer: Option<Option<String>>,
    },
    /// A member in a JSON input document.
    Document {
        /// Document identity, present as null when the caller supplied none.
        #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
        document: Option<Option<SourceRef>>,
        /// Caller path, present as null when the caller supplied none.
        #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
        path: Option<Option<String>>,
        /// Pointer to the failing member or object holding a missing member.
        pointer: String,
        /// Object identity, present as null when no snapshot object applies.
        #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
        object: Option<Option<DocumentObject>>,
        /// Failing member, present as null when none is named.
        #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
        member: Option<Option<String>>,
    },
}

/// The two closed located forms: native bytes or imported coordinates.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Located {
    /// A native source span with real byte offsets and no imported URI.
    Native {
        /// Authored path.
        file: String,
        /// Native byte-bearing span.
        span: RunSpan,
    },
    /// An imported origin, without any native label pair or byte offsets.
    Imported {
        /// Authored path.
        file: String,
        /// Exact admitted FCD source identity URI.
        #[serde(rename = "sourceIdentity", deserialize_with = "source_identity_uri")]
        source_identity: String,
        /// Byte-free authored coordinates.
        span: fsl_foundation::SourceCoordinates,
    },
}

pub(in crate::spine::native_result) fn source_identity_uri<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if fsl_semantics::model::intake::is_source_identity_uri(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(
            "an admitted FCD source identity URI",
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Decoded snapshot object identity in a document locus.
pub struct DocumentObject {
    /// Model population name.
    pub population: String,
    /// Object key within the population.
    pub key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Decoded identity of an authored source.
pub struct SourceRef {
    /// Source authority.
    pub authority: String,
    /// Source identity.
    pub identity: String,
}

impl From<crate::outcome::OutcomeLocus> for RefusalLocus {
    /// The one conversion from the `filament-outcome/1` diagnostic locus the
    /// producers build to this wire type; every variant is matched.
    fn from(locus: crate::outcome::OutcomeLocus) -> Self {
        use crate::outcome::OutcomeLocus as Outcome;
        let source = |source: fsl_foundation::source::provenance::RawSourceRef| SourceRef {
            authority: source.authority().to_owned(),
            identity: source.identity().to_owned(),
        };
        match locus {
            Outcome::Region { source: s, start, end } => Self::Region {
                source: source(s),
                start,
                end,
            },
            Outcome::Located { file, span } => Self::Located(Located::Native {
                file,
                span: RunSpan::from(span),
            }),
            Outcome::ImportedLocated {
                file,
                source_identity,
                span,
            } => Self::Located(Located::Imported {
                file,
                source_identity,
                span,
            }),
            Outcome::Occurrence { occurrence } => Self::Occurrence { occurrence },
            Outcome::Artifact { identity, pointer } => Self::Artifact {
                identity: source(identity),
                pointer,
            },
            Outcome::Input {
                operand,
                member,
                pointer,
            } => Self::Input {
                operand,
                member: Some(member),
                pointer: Some(pointer),
            },
            Outcome::Document {
                document,
                path,
                pointer,
                object,
                member,
            } => Self::Document {
                document: Some(document.map(source)),
                path: Some(path),
                pointer,
                object: Some(object.map(|object| DocumentObject {
                    population: object.population,
                    key: object.key,
                })),
                member: Some(member),
            },
        }
    }
}
