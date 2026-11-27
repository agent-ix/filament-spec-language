// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-267-AC-17: the source extraction record of a `native-run-result/2`
//! clause-run document (FSpec FR-352), the one declared type its writer
//! (`fsl-command`) constructs and the strict reader decodes.

use std::collections::BTreeMap;

use ix_trace_rs::implements;
use quire_canonical::{Node, NodeRef};
use quire_rs::semantic::{
    AvailabilityState, ClauseRef, KindAvailability, SemanticDiagnostic, SemanticSeverity,
    SourceLocus,
};
use serde::de::Error as _;
use serde::{Deserialize, Serialize};

use super::super::NativeRunReadError;
use super::present;

/// FR-267-AC-17: the segments of an extraction map, never empty.
#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
pub struct ExtractionSegments(Vec<ExtractionSegment>);

impl ExtractionSegments {
    /// The segments, or `None` for an empty list.
    pub fn new(segments: Vec<ExtractionSegment>) -> Option<Self> {
        (!segments.is_empty()).then_some(Self(segments))
    }

    /// The segments, at least one.
    pub fn as_slice(&self) -> &[ExtractionSegment] {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ExtractionSegments {
    #[implements("FR-267-AC-17")]
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        Self::new(Vec::deserialize(decoder)?)
            .ok_or_else(|| D::Error::custom("a non-empty segment array"))
    }
}

impl ExtractionShape for ExtractionSegments {
    fn missing_member(node: NodeRef<'_>, path: &str) -> Option<String> {
        <Vec<ExtractionSegment> as ExtractionShape>::missing_member(node, path)
    }
}

trait ExtractionShape {
    fn missing_member(node: NodeRef<'_>, path: &str) -> Option<String>;
}

macro_rules! extraction_leaf_shape {
    ($($type:ty),+ $(,)?) => {
        $(
            impl ExtractionShape for $type {
                fn missing_member(_: NodeRef<'_>, _: &str) -> Option<String> {
                    None
                }
            }
        )+
    };
}

extraction_leaf_shape!(
    String,
    u8,
    u64,
    usize,
    bool,
    AvailabilityState,
    SemanticSeverity
);

impl<K, V> ExtractionShape for BTreeMap<K, V> {
    fn missing_member(_: NodeRef<'_>, _: &str) -> Option<String> {
        None
    }
}

impl<T: ExtractionShape> ExtractionShape for Vec<T> {
    #[implements("FR-267-AC-17")]
    fn missing_member(node: NodeRef<'_>, path: &str) -> Option<String> {
        let Node::Array(entries) = node.node() else {
            return None;
        };
        entries
            .enumerate()
            .find_map(|(index, entry)| T::missing_member(entry, &format!("{path}[{index}]")))
    }
}

impl<T: ExtractionShape> ExtractionShape for Option<T> {
    #[implements("FR-267-AC-17")]
    fn missing_member(node: NodeRef<'_>, path: &str) -> Option<String> {
        if matches!(node.node(), Node::Null) {
            None
        } else {
            T::missing_member(node, path)
        }
    }
}

macro_rules! extraction_record_type {
    (
        $name:ident {
            $($(#[$field_attribute:meta])* $field:ident: $field_type:ty),* $(,)?
            ;
            $($(#[$optional_attribute:meta])* $optional_field:ident: $optional_type:ty),* $(,)?
        }
    ) => {
        #[derive(Clone, Debug, Serialize)]
        #[allow(missing_docs, reason = "fields implement the native extraction wire record")]
        pub struct $name {
            $($(#[$field_attribute])* pub $field: $field_type,)*
            $($(#[$optional_attribute])* pub $optional_field: $optional_type,)*
        }

        // Serde's derived struct decoder also accepts a positional array, so
        // the members are decoded only after object syntax is confirmed.
        const _: () = {
            type Record = $name;

            #[derive(Deserialize)]
            #[serde(remote = "Record", deny_unknown_fields)]
            #[allow(dead_code, reason = "a Serde remote decoder is never constructed")]
            struct Members {
                $($(#[$field_attribute])* $field: $field_type,)*
                $($(#[$optional_attribute])* $optional_field: $optional_type,)*
            }

            struct Decoded(Record);

            impl<'de> Deserialize<'de> for Decoded {
                fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
                    Members::deserialize(decoder).map(Self)
                }
            }

            impl<'de> Deserialize<'de> for $name {
                #[implements("FR-267-AC-17")]
                fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
                    fsl_foundation::serde_object::from_object::<D, Decoded>(decoder)
                        .map(|Decoded(record)| record)
                }
            }
        };

        impl ExtractionShape for $name {
            #[implements("FR-267-AC-17")]
            fn missing_member(node: NodeRef<'_>, path: &str) -> Option<String> {
                if !matches!(node.node(), Node::Object(_)) {
                    return None;
                }
                $(
                    let field_path = format!("{path}.{}", stringify!($field));
                    let Some(value) = node.get(stringify!($field)) else {
                        return Some(field_path);
                    };
                    if let Some(missing) = <$field_type as ExtractionShape>::missing_member(value, &field_path) {
                        return Some(missing);
                    }
                )*
                $(
                    if let Some(value) = node.get(stringify!($optional_field)) {
                        let field_path = format!("{path}.{}", stringify!($optional_field));
                        if let Some(missing) = <$optional_type as ExtractionShape>::missing_member(value, &field_path) {
                            return Some(missing);
                        }
                    }
                )*
                None
            }
        }

    };
}

extraction_record_type! {
    ExtractionRecord {
        original: ExtractionOriginal,
        body: ExtractionBody,
        binding: ExtractionBinding,
        outcome: ExtractionOutcome,
        map: ExtractionMap;
        #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
        original_regions: Option<Vec<ExtractionRegion>>
    }
}

extraction_record_type! {
    ExtractionOriginal {
        authority: String,
        identity: String,
        path: String,
        bytes: Vec<u8>;
    }
}

extraction_record_type! {
    ExtractionBody {
        authority: String,
        identity: String,
        document: String,
        bytes: Vec<u8>;
    }
}

extraction_record_type! {
    ExtractionBinding {
        clause: String,
        package: String;
    }
}

extraction_record_type! {
    ExtractionMap {
        region: ExtractionSpan,
        segments: ExtractionSegments;
    }
}

extraction_record_type! {
    ExtractionSpan {
        start: u64,
        end: u64;
    }
}

extraction_record_type! {
    ExtractionSegment {
        body_start: u64,
        body_end: u64,
        original_start: u64,
        original_end: u64;
    }
}

extraction_record_type! {
    ExtractionRegion {
        start: ExtractionPosition,
        end: ExtractionPosition;
    }
}

extraction_record_type! {
    ExtractionPosition {
        byte: u64,
        line: u64,
        column: u64;
    }
}

extraction_record_type! {
    ExtractionOutcome {
        #[serde(deserialize_with = "closed_object")]
        availability: KindAvailability,
        clause_text: std::collections::BTreeMap<String, String>,
        #[serde(deserialize_with = "closed_objects")]
        diagnostics: Vec<SemanticDiagnostic>;
        #[serde(default, deserialize_with = "present_closed_objects", skip_serializing_if = "Option::is_none")]
        clauses: Option<Vec<ClauseRef>>
    }
}

/// A Quire record this reader decodes as a closed JSON object. Quire's own
/// derived decoders accept a positional array and unknown members.
trait ClosedRecord: Sized {
    fn closed<'de, D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error>;
}

/// The closed decoder and the missing-member walk of one Quire record, which
/// Quire writes with its own encoding. Serde's remote derive builds the Quire
/// type, so a member Quire adds, removes or retypes stops the build.
macro_rules! quire_record {
    (
        $record:ident {
            $($wire:literal $field:ident: $field_type:ty $(=> $with:literal)?),*
            ;
            $($optional_wire:literal $optional_field:ident: $optional_type:ty => $optional_with:literal),*
        }
    ) => {
        const _: () = {
            type Record = $record;

            #[derive(Deserialize)]
            #[serde(remote = "Record", deny_unknown_fields)]
            #[allow(dead_code, reason = "a Serde remote decoder is never constructed")]
            struct Members {
                $(
                    #[serde(rename = $wire $(, deserialize_with = $with)?)]
                    $field: $field_type,
                )*
                $(
                    #[serde(rename = $optional_wire, default, deserialize_with = $optional_with)]
                    $optional_field: Option<$optional_type>,
                )*
            }

            struct Decoded(Record);

            impl<'de> Deserialize<'de> for Decoded {
                fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
                    Members::deserialize(decoder).map(Self)
                }
            }

            impl ClosedRecord for $record {
                #[implements("FR-267-AC-17")]
                fn closed<'de, D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
                    fsl_foundation::serde_object::from_object::<D, Decoded>(decoder)
                        .map(|Decoded(record)| record)
                }
            }
        };

        impl ExtractionShape for $record {
            #[implements("FR-267-AC-17")]
            fn missing_member(node: NodeRef<'_>, path: &str) -> Option<String> {
                if !matches!(node.node(), Node::Object(_)) {
                    return None;
                }
                $(
                    let field_path = format!("{path}.{}", $wire);
                    let Some(value) = node.get($wire) else {
                        return Some(field_path);
                    };
                    if let Some(missing) = <$field_type as ExtractionShape>::missing_member(value, &field_path) {
                        return Some(missing);
                    }
                )*
                $(
                    if let Some(value) = node.get($optional_wire) {
                        let field_path = format!("{path}.{}", $optional_wire);
                        if let Some(missing) = <$optional_type as ExtractionShape>::missing_member(value, &field_path) {
                            return Some(missing);
                        }
                    }
                )*
                None
            }
        }
    };
}

quire_record! {
    KindAvailability {
        "state" state: AvailabilityState => "string_variant",
        "lossy" lossy: bool
        ;
        "reason" reason: String => "present"
    }
}

quire_record! {
    SemanticDiagnostic {
        "code" code: String,
        "severity" severity: SemanticSeverity => "string_variant",
        "message" message: String
        ;
        "line" line: usize => "present",
        "column" column: usize => "present",
        "reason" reason: String => "present",
        "sourceSpan" source_span: SourceLocus => "present_closed_object",
        "section" section: String => "present"
    }
}

quire_record! {
    ClauseRef {
        "language" language: String,
        "clauseId" clause_id: String
        ;
        "sourceSpan" source_span: SourceLocus => "present_closed_object"
    }
}

quire_record! {
    SourceLocus {
        "sourceIdentity" source_identity: String,
        "path" path: String,
        "startLine" start_line: usize,
        "startColumn" start_column: usize
        ;
        "endLine" end_line: usize => "present",
        "endColumn" end_column: usize => "present"
    }
}

/// One closed record, wherever a decoder needs a `Deserialize` type.
struct Closed<T>(T);

impl<'de, T: ClosedRecord> Deserialize<'de> for Closed<T> {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        T::closed(decoder).map(Self)
    }
}

fn closed_object<'de, D: serde::Deserializer<'de>, T: ClosedRecord>(
    decoder: D,
) -> Result<T, D::Error> {
    T::closed(decoder)
}

fn present_closed_object<'de, D: serde::Deserializer<'de>, T: ClosedRecord>(
    decoder: D,
) -> Result<Option<T>, D::Error> {
    T::closed(decoder).map(Some)
}

#[implements("FR-267-AC-17")]
fn closed_objects<'de, D: serde::Deserializer<'de>, T: ClosedRecord>(
    decoder: D,
) -> Result<Vec<T>, D::Error> {
    Vec::<Closed<T>>::deserialize(decoder)
        .map(|entries| entries.into_iter().map(|Closed(entry)| entry).collect())
}

fn present_closed_objects<'de, D: serde::Deserializer<'de>, T: ClosedRecord>(
    decoder: D,
) -> Result<Option<Vec<T>>, D::Error> {
    closed_objects(decoder).map(Some)
}

/// A Quire enum, admitted only as a JSON string: Serde's derived enum decoder
/// also accepts `{"error": null}`.
#[implements("FR-267-AC-17")]
fn string_variant<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    decoder: D,
) -> Result<T, D::Error> {
    let name = String::deserialize(decoder)?;
    T::deserialize(serde::de::value::StringDeserializer::new(name))
}

#[implements("FR-267-AC-17")]
pub(in crate::spine::native_result) fn validate_extraction(
    extraction: Option<NodeRef<'_>>,
) -> Result<(), NativeRunReadError> {
    let Some(value) = extraction.filter(|value| !matches!(value.node(), Node::Null)) else {
        return Ok(());
    };
    if let Some(member) = ExtractionRecord::missing_member(value, "extraction") {
        return Err(NativeRunReadError::MalformedMember {
            member,
            source: serde_json::Error::custom("missing required extraction member"),
        });
    }
    let segments_empty = value
        .get("map")
        .and_then(|map| map.get("segments"))
        .is_some_and(
            |segments| matches!(segments.node(), Node::Array(entries) if entries.len() == 0),
        );
    if segments_empty {
        return Err(NativeRunReadError::Invalid {
            member: "extraction.map.segments",
            reason: "a non-empty segment array",
        });
    }
    let regions_empty = value.get("original_regions").is_some_and(
        |regions| matches!(regions.node(), Node::Array(entries) if entries.len() == 0),
    );
    if regions_empty {
        return Err(NativeRunReadError::Invalid {
            member: "extraction.original_regions",
            reason: "a non-empty region array",
        });
    }
    Ok(())
}
