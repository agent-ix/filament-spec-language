// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1902 (FR-364-AC-15): a snapshot's instances of an ordered end carry
//! `positions`, and navigation over the end returns its targets in ascending
//! position. Run end to end through `run_clause`: the snapshot JSON is
//! admitted by FR-106 and an invariant observes the order with a sequence
//! fold, which admits an order-sensitive step.
//!
//! Domain package `test/ordered`: `Sys` with the ordered, non-unique end
//! `parts` `{0,3}` to `Pump`, so `self.parts` is a
//! `Sequence<Reference<Pump>>[0,3]`; `Pump` has the field `n: Integer`.

use super::*;
use filament_semantic_value::catalog::Category;
use fsl_semantics::model::intake::meaning;

const PACKAGE: &str = "test/ordered";
const PARTS: &str = "relationship/Sys-composition-Pump";

fn identity(node: &str) -> String {
    format!("ix://{PACKAGE}/{node}")
}

fn provenance(node: &str) -> serde_json::Value {
    json!({"generated": {
        "generatorIdentity": node, "generatorVersion": "1.0.0", "inputIdentities": [node],
    }})
}

fn origin() -> serde_json::Value {
    json!({"source": {
        "sourceIdentity": identity("spec"), "path": "spec.fspec", "startLine": 1, "startColumn": 1,
    }})
}

fn construct(name: &str, meaning: &str) -> serde_json::Value {
    json!({
        "kind": {"module": PACKAGE, "name": name},
        "moduleVersion": "1.0.0",
        "construct": {"identity": "none", "shape": "record", "members": {}, "meaning": meaning},
    })
}

fn object_type(name: &str, extra: serde_json::Value) -> serde_json::Value {
    let node = identity(name);
    let mut value = json!({
        "identity": node, "displayName": node,
        "kind": {"module": PACKAGE, "name": "object_type"},
        "roles": [], "origin": provenance(&node), "constraints": [], "extensions": [],
        "unknownPolicy": "reject", "supertypes": [], "fields": [], "operations": [],
        "relationships": [],
    });
    for (member, entry) in extra.as_object().expect("an object") {
        value[member] = entry.clone();
    }
    value
}

fn domain_document() -> Vec<u8> {
    let n = identity("Pump/n");
    let sys = object_type(
        "Sys",
        json!({"relationships": [{
            "identity": identity(PARTS),
            "sourceEnd": {
                "type": identity("Sys"), "role": "parts",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            },
            "targetEnd": {
                "type": identity("Pump"), "role": "owner",
                "multiplicity": {"lower": 0, "upper": 3, "ordered": true, "unique": false},
            },
            "category": "structural", "composite": false,
            "direction": "source-to-target", "origin": origin(),
        }]}),
    );
    let pump = object_type(
        "Pump",
        json!({"fields": [{
            "identity": n, "name": "n", "typeRef": "ix://filament/native/Integer",
            "presence": "required", "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "origin": provenance(&n),
        }]}),
    );
    let population = json!({
        "identity": identity("Q"), "displayName": identity("Q"),
        "kind": {"module": PACKAGE, "name": "population"},
        "members": [identity("Sys"), identity("Pump")],
        "extent": "closed", "origin": provenance(&identity("Q")),
    });
    serde_json::to_vec(&json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {"identity": identity("spec"), "version": "1.0.0", "dialect": "spec-bundle"},
        "package": {
            "identity": PACKAGE, "version": "1.0.0",
            "mappingVersions": [], "profileVersions": [],
        },
        "occurrences": [], "extensions": [],
        "constructs": [
            construct("object_type", meaning::OBJECT_TYPE),
            construct("population", meaning::POPULATION),
        ],
        "types": [sys, pump],
        "populations": [population],
    }))
    .expect("valid JSON")
}

/// `Order231` holds when `s1`'s parts, read in navigation order, are the
/// pumps whose `n` spell 2, 3, 1; reference-key order (`p1`, `p2`, `p3`)
/// spells 1, 2, 3.
const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    model M = \"test/ordered\";\n\
    type I = Integer;\n\
    invariant Order231 using v on M::Sys at current { \
    fold<I>(acc, x in self.parts: acc * 10 + deref(x).n, identity: 0) = 231 }\n";

/// A current snapshot where `s1` links to `p1`, `p2` and `p3` (`n` 1, 2, 3)
/// with the instances' `positions` given by `positions` (`None` omits it).
fn snapshot(label: &DocumentRef, positions: [Option<serde_json::Value>; 3]) -> Vec<u8> {
    let end = |key: &str| json!({"population": identity("Q"), "key": key});
    let mut objects = vec![json!({"key": "s1", "type": identity("Sys"), "fields": {}})];
    objects.extend((1..=3).map(|n| {
        json!({
            "key": format!("p{n}"), "type": identity("Pump"),
            "fields": {"n": {"integer": n.to_string()}},
        })
    }));
    let instances: Vec<_> = ["p1", "p2", "p3"]
        .iter()
        .zip(positions)
        .map(|(pump, position)| {
            let mut instance = json!({"source": end("s1"), "target": end(pump)});
            if let Some(position) = position {
                instance["positions"] = position;
            }
            instance
        })
        .collect();
    json!({
        "format": "filament.state.snapshot/v1",
        "identity": document_identity_json(label),
        "observation": "current",
        "anchor": {"kind": "handler", "name": "validate"},
        "model": {"identity": PACKAGE, "version": "1.0.0"},
        "populations": [{"population": identity("Q"), "complete": true, "objects": objects}],
        "relationships": [{
            "relationship": identity(PARTS), "complete": true, "instances": instances,
        }],
    })
    .to_string()
    .into_bytes()
}

fn target(position: u64) -> Option<serde_json::Value> {
    Some(json!({"target": position.to_string()}))
}

/// Runs `Order231` with `self` `s1` over the snapshot of `positions`.
fn run_order(positions: [Option<serde_json::Value>; 3]) -> ClauseDisposition {
    let label = config_version_document_ref("ordered-current");
    let bytes = snapshot(&label, positions);
    let selection = ClauseSelection {
        name: "Order231".to_owned(),
        input: ClauseSelectionInput::Current {
            snapshot: label,
            anchor: SelectedAnchor {
                kind: fsl_semantics::model::observation::AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: Some(SelectedObject {
                population: identity("Q"),
                key: "s1".to_owned(),
            }),
        },
    };
    let packages =
        fsl_semantics::model::intake::package_input([domain_document().as_slice()]).unwrap();
    let mut request = config_version_request_for(
        UNIT.to_owned(),
        packages,
        "ordered-end.native",
        ClauseRunSelection::Clause(selection),
    );
    request.snapshots.insert(
        fsl_semantics::model::observation::document_reference(&bytes)
            .expect("a document names itself"),
        bytes,
    );
    run_clause(request)
        .expect("a well-formed request always reports")
        .disposition
}

/// FR-364-AC-15: target positions 7, 2 and 5 on `p1`, `p2` and `p3` make the
/// navigation order `p2`, `p3`, `p1`, whatever the listing order, so the
/// sequence fold gives 231 and the invariant holds.
#[trace("TC-1902", "FR-364-AC-15")]
#[trace("FSpec-TC-1400", "FSpec-FR-152-AC-13")]
#[test]
#[ignore = "QSL-2179: ordered-end positions and ordered navigation not implemented yet"]
fn an_ordered_end_navigates_in_ascending_position() {
    let disposition = run_order([target(7), target(2), target(5)]);
    assert!(
        matches!(
            disposition,
            ClauseDisposition::Evaluate(super::super::CallOutcome::Completed(
                super::super::CallValue::Boolean(true)
            ))
        ),
        "{disposition:?}"
    );
    assert_eq!(disposition.category(), Category::Success);
}

/// FR-364-AC-15: a missing target position, a position for the unordered
/// source end, and two instances of `s1` sharing a position each refuse at
/// admission `invalid_runtime_input`/`invalid-value`.
#[trace("TC-1902", "FR-364-AC-15")]
#[trace("FSpec-TC-1400", "FSpec-FR-152-AC-13")]
#[test]
#[ignore = "QSL-2179: ordered-end position admission not implemented yet"]
fn malformed_positions_refuse_at_admission() {
    for positions in [
        [target(7), None, target(5)],
        [
            target(7),
            Some(json!({"target": "2", "source": "0"})),
            target(5),
        ],
        [target(2), target(2), target(5)],
    ] {
        let disposition = run_order(positions);
        assert_eq!(
            disposition.stage(),
            ClauseRunStage::Admit,
            "{disposition:?}"
        );
        let ClauseDisposition::Admit(fsl_semantics::model::observation::AdmissionFailure::Refused(
            record,
        )) = &disposition
        else {
            panic!("an admission refusal: {disposition:?}");
        };
        assert_eq!(record.catalog.code(), "invalid_runtime_input", "{record:?}");
        assert_eq!(record.catalog.cause(), "invalid-value", "{record:?}");
    }
}
