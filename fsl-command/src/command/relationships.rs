// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1045: request-shape admission for native-run relationship correlation.

use fsl_semantics::model::domain_package::DomainPackageRecord;
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt;

use super::{Result, RunCause, wire};

/// Validates the closed selection before opening any selected file.
pub(super) fn validate_request(call: Option<&wire::Call>) -> Result<()> {
    let Some(call) = call else { return Ok(()) };
    let Some(selection) = call.relationships.as_ref() else {
        return Ok(());
    };
    if call.snapshot.is_none() {
        return Err(invalid(
            "/call/relationships",
            "relationship correlation requires call.snapshot",
        ));
    }

    fsl_replay::spine::RelationshipSelection::read(selection)
        .map(|_| ())
        .map_err(|error| invalid(&error.path, error.reason))
}

/// Detects duplicate members before deserializing into `serde_json::Value`,
/// which would otherwise discard the earlier spelling. Only the relationship
/// selection is closed by FR-1045; unrelated request objects keep their
/// existing duplicate-member behavior.
pub(super) fn duplicate_selection_member(
    bytes: &[u8],
) -> std::result::Result<Option<String>, serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let duplicate = DuplicateSeed {
        path: String::new(),
    }
    .deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(duplicate.map(|path| path.strip_prefix("/request").unwrap_or(&path).to_owned()))
}

struct DuplicateSeed {
    path: String,
}

impl<'de> DeserializeSeed<'de> for DuplicateSeed {
    type Value = Option<String>;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(DuplicateVisitor { path: self.path })
    }
}

struct DuplicateVisitor {
    path: String,
}

impl<'de> Visitor<'de> for DuplicateVisitor {
    type Value = Option<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, _: bool) -> std::result::Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_i64<E>(self, _: i64) -> std::result::Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_u64<E>(self, _: u64) -> std::result::Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_f64<E>(self, _: f64) -> std::result::Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_str<E>(self, _: &str) -> std::result::Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_string<E>(self, _: String) -> std::result::Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_unit<E>(self) -> std::result::Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_none<E>(self) -> std::result::Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_some<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    fn visit_seq<A>(self, mut sequence: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut first = None;
        let mut index = 0usize;
        while let Some(child) = sequence.next_element_seed(DuplicateSeed {
            path: format!("{}/{index}", self.path),
        })? {
            first = first.or(child);
            index += 1;
        }
        Ok(first)
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut seen = BTreeSet::new();
        let mut first = None;
        while let Some(key) = map.next_key::<String>()? {
            let child_path = format!("{}/{}", self.path, pointer_escape(&key));
            let duplicate_is_in_selection = self.path == "/request/call" && key == "relationships"
                || self.path == "/request/call/relationships"
                || self.path.starts_with("/request/call/relationships/");
            if duplicate_is_in_selection && !seen.insert(key.clone()) && first.is_none() {
                first = Some(child_path.clone());
            } else {
                seen.insert(key.clone());
            }
            let child = map.next_value_seed(DuplicateSeed { path: child_path })?;
            first = first.or(child);
        }
        Ok(first)
    }
}

fn pointer_escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

/// Preserves the producer's relationship identities while preparing the same
/// snapshot for FR-106's pre-existing object admission reader.
pub(super) fn snapshot_document(
    bytes: &[u8],
    selection: Option<&Value>,
) -> Result<(Vec<u8>, Option<Value>)> {
    if selection.is_none() {
        return Ok((bytes.to_vec(), None));
    }
    let Ok(mut document) = serde_json::from_slice::<Value>(bytes) else {
        // Let the owning FR-106 reader report malformed snapshot JSON.
        return Ok((bytes.to_vec(), None));
    };
    let Some(extents) = document.get("relationships").cloned() else {
        return Ok((bytes.to_vec(), None));
    };
    if let (Some(extents), Some(sanitized)) = (
        extents.as_array(),
        document
            .get_mut("relationships")
            .and_then(Value::as_array_mut),
    ) {
        for extent in sanitized {
            let Some(instances) = extent.get_mut("instances").and_then(Value::as_array_mut) else {
                continue;
            };
            for instance in instances {
                if let Some(object) = instance.as_object_mut() {
                    object.remove("identity");
                }
            }
        }
        debug_assert_eq!(
            extents.len(),
            document["relationships"].as_array().map_or(0, Vec::len)
        );
    } else {
        return Ok((bytes.to_vec(), Some(extents)));
    }
    let sanitized = serde_json::to_vec(&document).map_err(|error| {
        invalid(
            "/call/snapshot",
            format!("could not prepare snapshot: {error}"),
        )
    })?;
    Ok((sanitized, Some(extents)))
}

fn invalid(path: &str, reason: impl Into<String>) -> RunCause {
    RunCause::ClauseRequest {
        member: path.to_owned(),
        reason: reason.into(),
    }
}

/// Checks caller mappings against the navigation relationships of the
/// selected, admitted domain packages before the snapshot path is opened.
pub(super) fn validate_package_mappings(
    selection: &Value,
    model_identities: impl IntoIterator<Item = String>,
    packages: &fsl_semantics::model::intake::PackageInput,
) -> Result<std::collections::BTreeSet<String>> {
    let selection = fsl_replay::spine::RelationshipSelection::read(selection)
        .map_err(|error| invalid(&error.path, error.reason))?;
    let mut domain_packages = Vec::new();
    for identity in model_identities {
        let (package_ref, document) = fsl_semantics::model::intake::admit(
            &identity,
            packages,
            fsl_foundation::IntakeLimits::default(),
        )
        .map_err(|refusal| RunCause::ModelPackageInput(Box::new(refusal)))?;
        let package = fsl_semantics::model::intake::read_domain_package(package_ref, &document)
            .map_err(|mut refusals| {
                let refusal = refusals.remove(0);
                RunCause::ModelPackageInput(Box::new(refusal))
            })?;
        domain_packages.push(package);
    }
    let object_types: std::collections::BTreeSet<_> = domain_packages
        .iter()
        .flat_map(|package| package.records.iter())
        .filter_map(|record| match record {
            DomainPackageRecord::ObjectType(object_type) => Some(object_type.key.clone()),
            _ => None,
        })
        .collect();
    let navigation_relationships: std::collections::BTreeSet<String> = domain_packages
        .iter()
        .flat_map(|package| package.records.iter())
        .filter_map(|record| match record {
            DomainPackageRecord::Relationship(relationship)
                if object_types.contains(&relationship.source.type_identity)
                    && object_types.contains(&relationship.target.type_identity) =>
            {
                Some(relationship.key.node.clone())
            }
            _ => None,
        })
        .collect();
    for (index, (relationship_identity, _)) in selection.relationship_mappings().enumerate() {
        if !navigation_relationships.contains(relationship_identity) {
            return Err(invalid(
                &format!("/call/relationships/relationship_identities/{index}/relationship"),
                "relationship does not name a navigation relationship of the selected package",
            ));
        }
    }
    Ok(navigation_relationships)
}

#[cfg(test)]
mod tests {
    use super::{duplicate_selection_member, snapshot_document};
    use filament_observation::{
        PRODUCER_INTERFACE_VERSION, ProducerConfiguration, ProducerDigest, Revision,
        StaticProducerBundle,
    };
    use ix_trace_rs::trace;
    use serde_json::json;

    fn selection(relationship_identities: serde_json::Value) -> serde_json::Value {
        let mut producer = StaticProducerBundle {
            bundle_identity: Some("ix://test/producer".to_owned()),
            bundle_revision: Some(Revision {
                namespace: "test-revision".to_owned(),
                value: "1".to_owned(),
            }),
            interface_version: Some(PRODUCER_INTERFACE_VERSION.to_owned()),
            configuration: Some(ProducerConfiguration {
                configuration_identity: "ix://test/configuration".to_owned(),
                digest: ProducerDigest {
                    label: "filament-canonical-json-1/1".to_owned(),
                    value: format!("sha256:{}", "00".repeat(32)),
                },
            }),
            ..StaticProducerBundle::default()
        };
        let digest = producer
            .canonical_digest_selection()
            .expect("producer candidate has a canonical digest");
        producer.digest = Some(digest.clone());
        let mut producer = serde_json::to_value(producer).expect("producer serializes");
        let wire_digest = |value: &str| {
            json!({
                "algorithm": "sha256",
                "domain": "filament-canonical-json-1",
                "value": value,
                "version": "1"
            })
        };
        producer["digest"] = wire_digest(&digest.value);
        producer["configuration"]["digest"] = wire_digest(&format!("sha256:{}", "00".repeat(32)));
        json!({
            "producer": producer,
            "binding": {
                "identity": "binding",
                "source_identity": "source",
                "schema_identity": "schema",
                "signal_identity": "signal",
                "trigger_identity": "trigger",
                "unit": "unit",
                "subject_kind": "type",
                "required": true
            },
            "expected_subject": {
                "authority": {
                    "bundle_identity": "ix://test/producer",
                    "bundle_revision": {"namespace": "test-revision", "value": "1"},
                    "digest": {"label": "filament-canonical-json-1/1", "value": format!("sha256:{}", "00".repeat(32))}
                },
                "subject_kind": "type",
                "authority_local_object_identity": "01"
            },
            "required_relationships": [],
            "relationship_identities": relationship_identities,
            "scope": {
                "population_identity": "population",
                "membership_rule_identity": "membership-rule",
                "membership_digest": "00".repeat(32),
                "membership_document": "{}",
                "required_member_identities": [],
                "observation_sources": [],
                "completeness_dependencies": [],
                "progress_dependencies": [],
                "clock_identity": "clock",
                "clock_revision": "1",
                "membership_complete": true,
                "closure_identity": null,
                "closure_digest": null,
                "kind": {"kind": "snapshot", "snapshot_identity": "snapshot"},
                "range": {"clock_family": "event-position", "start": "0", "end_exclusive": "1"},
                "members": []
            },
            "records": [],
            "limits": {
                "max_records": "1",
                "max_members": "1",
                "max_relationships": "1",
                "max_required_relationships": "1"
            }
        })
    }

    #[trace("FR-1045-AC-2", "FR-106-AC-74")]
    #[test]
    fn relationship_identity_is_retained_for_correlation_and_stripped_for_snapshot_admission() {
        let snapshot = json!({
            "relationships": [{
                "relationship": "ix://test/package/relationship/R",
                "complete": true,
                "instances": [{
                    "identity": "00ff",
                    "source": {"population": "orders", "key": "o1"},
                    "target": {"population": "payments", "key": "p1"}
                }]
            }]
        });
        let bytes = serde_json::to_vec(&snapshot).expect("snapshot encodes");
        let selected = selection(json!([{
            "relationship": "ix://test/package/relationship/R",
            "declaration_identity": "ix://test/producer/relationship/R"
        }]));
        let (sanitized, extents) =
            snapshot_document(&bytes, Some(&selected)).expect("snapshot prepares");
        let sanitized: serde_json::Value =
            serde_json::from_slice(&sanitized).expect("sanitized snapshot parses");
        assert!(
            sanitized["relationships"][0]["instances"][0]
                .get("identity")
                .is_none()
        );
        assert_eq!(
            extents.expect("extents retained")[0]["instances"][0]["identity"],
            "00ff"
        );
    }

    #[trace("FR-1045-AC-2", "FR-106-AC-74")]
    #[test]
    fn malformed_relationship_identity_is_preserved_for_post_admission_validation() {
        let snapshot = json!({
            "relationships": [{
                "relationship": "ix://test/package/relationship/R",
                "complete": true,
                "instances": [{
                    "identity": "00FF",
                    "source": {"population": "orders", "key": "o1"},
                    "target": {"population": "payments", "key": "p1"}
                }]
            }]
        });
        let bytes = serde_json::to_vec(&snapshot).expect("snapshot encodes");
        let selected = selection(json!([{
            "relationship": "ix://test/package/relationship/R",
            "declaration_identity": "ix://test/producer/relationship/R"
        }]));
        let (sanitized, extents) =
            snapshot_document(&bytes, Some(&selected)).expect("snapshot prepares");
        let sanitized: serde_json::Value =
            serde_json::from_slice(&sanitized).expect("sanitized snapshot parses");
        assert!(
            sanitized["relationships"][0]["instances"][0]
                .get("identity")
                .is_none()
        );
        assert_eq!(
            extents.expect("original extent retained")[0]["instances"][0]["identity"],
            "00FF"
        );
    }

    #[trace("FR-1045-AC-12", "FR-106-AC-74")]
    #[test]
    fn unmapped_relationship_identity_is_ignored() {
        let snapshot = json!({
            "relationships": [{
                "relationship": "ix://test/package/relationship/unmapped",
                "complete": false,
                "instances": [{"identity": "NOT-HEX"}]
            }]
        });
        let bytes = serde_json::to_vec(&snapshot).expect("snapshot encodes");
        let selected = selection(json!([]));
        let (sanitized, extents) = snapshot_document(&bytes, Some(&selected))
            .expect("an unmapped entry contributes no candidate and refuses no identity");
        let sanitized: serde_json::Value =
            serde_json::from_slice(&sanitized).expect("sanitized snapshot parses");
        assert!(
            sanitized["relationships"][0]["instances"][0]
                .get("identity")
                .is_none()
        );
        assert_eq!(extents.expect("extents retained")[0]["complete"], false);
    }

    #[trace("FR-1045-AC-13")]
    #[test]
    fn relationship_identity_mapping_duplicates_refuse_at_the_repeated_member() {
        let duplicate_relationships = selection(json!([
            {
                "relationship": "ix://test/package/relationship/R",
                "declaration_identity": "ix://test/producer/relationship/R"
            },
            {
                "relationship": "ix://test/package/relationship/R",
                "declaration_identity": "ix://test/producer/relationship/S"
            }
        ]));
        let error = fsl_replay::spine::RelationshipSelection::read(&duplicate_relationships)
            .expect_err("a package relationship maps once");
        assert_eq!(
            error.path,
            "/call/relationships/relationship_identities/1/relationship"
        );

        let duplicate_declarations = selection(json!([
            {
                "relationship": "ix://test/package/relationship/R",
                "declaration_identity": "ix://test/producer/relationship/R"
            },
            {
                "relationship": "ix://test/package/relationship/S",
                "declaration_identity": "ix://test/producer/relationship/R"
            }
        ]));
        let error = fsl_replay::spine::RelationshipSelection::read(&duplicate_declarations)
            .expect_err("a Producer declaration maps once");
        assert_eq!(
            error.path,
            "/call/relationships/relationship_identities/1/declaration_identity"
        );
    }

    #[trace("FR-1045-AC-8")]
    #[test]
    fn duplicate_object_member_survives_raw_request_preflight() {
        let request = br#"{"format":"native-run/1","request":{"call":{"relationships":{"records":[{"identity":"first","identity":"second"}]}}}}"#;
        assert_eq!(
            duplicate_selection_member(request).unwrap().as_deref(),
            Some("/call/relationships/records/0/identity")
        );
        let unrelated =
            br#"{"format":"native-run/1","request":{"call":{"function":"x","function":"y"}}}"#;
        assert_eq!(duplicate_selection_member(unrelated).unwrap(), None);
    }
}
