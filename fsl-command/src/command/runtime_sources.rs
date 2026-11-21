// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1039: closed caller source selections, validated before selected-file I/O.

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::fmt;

use super::{wire, Result, RunCause};

/// Validate this member before dispatch to a run member's own strict reader,
/// including readers that strip `export` before decoding the run request.
pub(super) fn validate_bytes(bytes: &[u8]) -> Result<()> {
    use fsl_foundation::serde_object::Object;

    let Ok(Object(envelope)) = serde_json::from_slice::<Object<wire::Envelope<'_>>>(bytes) else {
        // The owning envelope reader retains malformed-envelope diagnostics.
        return Ok(());
    };
    if envelope.format != "native-run/1" {
        return Ok(());
    }
    struct Provisions {
        sources: Option<Vec<wire::SourceFile>>,
        program: bool,
        package: bool,
    }
    struct ProvisionsSeed<'a> {
        duplicate_sources: &'a mut bool,
    }
    impl<'de> DeserializeSeed<'de> for ProvisionsSeed<'_> {
        type Value = Provisions;
        fn deserialize<D: de::Deserializer<'de>>(
            self,
            decoder: D,
        ) -> std::result::Result<Self::Value, D::Error> {
            struct ProvisionsVisitor<'a> {
                duplicate_sources: &'a mut bool,
            }
            impl<'de> Visitor<'de> for ProvisionsVisitor<'_> {
                type Value = Provisions;
                fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.write_str("a run request object")
                }
                fn visit_map<M: MapAccess<'de>>(
                    self,
                    mut map: M,
                ) -> std::result::Result<Self::Value, M::Error> {
                    let mut provisions = Provisions {
                        sources: None,
                        program: false,
                        package: false,
                    };
                    while let Some(key) = map.next_key::<String>()? {
                        match key.as_str() {
                            "sources" => {
                                if provisions.sources.is_some() {
                                    *self.duplicate_sources = true;
                                    return Err(de::Error::custom(
                                        "/request/sources: repeated member",
                                    ));
                                }
                                let mut pointer = "/request/sources".to_owned();
                                provisions.sources = map
                                    .next_value_seed(SourcesSeed {
                                        pointer: &mut pointer,
                                    })
                                    .map(Some)
                                    .map_err(|error| {
                                        de::Error::custom(format!("{pointer}: {error}"))
                                    })?;
                            }
                            _ => {
                                provisions.program |= key == "program";
                                provisions.package |= key == "package";
                                map.next_value::<de::IgnoredAny>()?;
                            }
                        }
                    }
                    Ok(provisions)
                }
            }
            decoder.deserialize_map(ProvisionsVisitor {
                duplicate_sources: self.duplicate_sources,
            })
        }
    }
    let mut duplicate_sources = false;
    let mut decoder = serde_json::Deserializer::from_str(envelope.request.get());
    let provisions = ProvisionsSeed {
        duplicate_sources: &mut duplicate_sources,
    }
    .deserialize(&mut decoder)
    .map_err(|error| {
        if duplicate_sources {
            RunCause::ClauseRequest {
                member: "/request/sources".to_owned(),
                reason: "repeated member".to_owned(),
            }
        } else {
            RunCause::Json(error)
        }
    })?;
    decoder.end()?;
    validate_provisions(
        provisions.sources.as_deref(),
        provisions.program,
        provisions.package,
    )
}

pub(super) fn deserialize<'de, D: de::Deserializer<'de>>(
    decoder: D,
) -> std::result::Result<Option<Vec<wire::SourceFile>>, D::Error> {
    let mut pointer = "/request/sources".to_owned();
    SourcesSeed {
        pointer: &mut pointer,
    }
    .deserialize(decoder)
    .map(Some)
    .map_err(|error| de::Error::custom(format!("{pointer}: {error}")))
}

struct SourcesSeed<'a> {
    pointer: &'a mut String,
}

impl<'de> DeserializeSeed<'de> for SourcesSeed<'_> {
    type Value = Vec<wire::SourceFile>;

    fn deserialize<D: de::Deserializer<'de>>(
        self,
        decoder: D,
    ) -> std::result::Result<Self::Value, D::Error> {
        decoder.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for SourcesSeed<'_> {
    type Value = Vec<wire::SourceFile>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an array of source selections")
    }

    fn visit_seq<S: SeqAccess<'de>>(
        self,
        mut seq: S,
    ) -> std::result::Result<Self::Value, S::Error> {
        let mut sources = Vec::new();
        loop {
            let index = sources.len();
            *self.pointer = format!("/request/sources/{index}");
            let Some(source) = seq.next_element_seed(SourceSeed {
                pointer: self.pointer,
                index,
            })?
            else {
                return Ok(sources);
            };
            sources.push(source);
        }
    }
}

struct SourceSeed<'a> {
    pointer: &'a mut String,
    index: usize,
}

impl<'de> DeserializeSeed<'de> for SourceSeed<'_> {
    type Value = wire::SourceFile;

    fn deserialize<D: de::Deserializer<'de>>(
        self,
        decoder: D,
    ) -> std::result::Result<Self::Value, D::Error> {
        decoder.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for SourceSeed<'_> {
    type Value = wire::SourceFile;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a five-member source selection object")
    }

    fn visit_map<M: MapAccess<'de>>(
        self,
        mut map: M,
    ) -> std::result::Result<Self::Value, M::Error> {
        const FIELDS: [&str; 5] = [
            "file",
            "authority",
            "identity",
            "document",
            "formal_revision",
        ];
        let base = format!("/request/sources/{}", self.index);
        let mut fields = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            *self.pointer = format!("{base}/{}", key.replace('~', "~0").replace('/', "~1"));
            if !FIELDS.contains(&key.as_str()) || fields.contains_key(&key) {
                return Err(de::Error::custom("unknown or repeated member"));
            }
            let value = if key == "formal_revision" {
                serde_json::Value::from(map.next_value::<u64>()?)
            } else {
                serde_json::Value::String(map.next_value::<String>()?)
            };
            fields.insert(key, value);
        }
        for field in FIELDS {
            if !fields.contains_key(field) {
                *self.pointer = format!("{base}/{field}");
                return Err(de::Error::missing_field(field));
            }
        }
        serde_json::from_value(serde_json::Value::Object(fields)).map_err(de::Error::custom)
    }
}

pub(super) fn validate(request: &wire::Request) -> Result<()> {
    validate_provisions(
        request.sources.as_deref(),
        request.program.is_some(),
        request.package.is_some(),
    )
}

fn validate_provisions(
    sources: Option<&[wire::SourceFile]>,
    program: bool,
    package: bool,
) -> Result<()> {
    let Some(sources) = sources else {
        return Ok(());
    };
    let invalid = |member, reason: &str| RunCause::ClauseRequest {
        member,
        reason: reason.to_owned(),
    };
    if program || !package {
        return Err(invalid(
            "/request/sources".to_owned(),
            "sources requires package without program",
        ));
    }
    let mut references = std::collections::BTreeSet::new();
    for (index, source) in sources.iter().enumerate() {
        let base = format!("/request/sources/{index}");
        if !references.insert((&source.identity.authority, &source.identity.identity)) {
            return Err(invalid(base, "repeated native source reference"));
        }
        filament_contract_model::SourceDocumentId::new(&source.identity.document).map_err(
            |mut diagnostic| {
                diagnostic.path = format!("{base}/document");
                RunCause::from(diagnostic)
            },
        )?;
        filament_contract_model::SourceRevision::new(source.identity.formal_revision).map_err(
            |mut diagnostic| {
                diagnostic.path = format!("{base}/formal_revision");
                RunCause::from(diagnostic)
            },
        )?;
    }
    Ok(())
}
