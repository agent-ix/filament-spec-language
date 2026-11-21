// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact Await selection transport and adverse sidecar cases (FR-517).

use super::*;
use crate::checked_handoff::{self, CheckedHandoffRequest, Corpus};
use ix_trace_rs::trace;
use serde_json::{json, Value};

struct Fixture {
    directory: tempfile::TempDir,
    request: CheckedHandoffRequest,
}

fn awaiting(name: &str) -> String {
    format!(
        "await {name} after Started using t clock \"line-clock\" within [0, 1] \
        match event Seen{name} by R as (seen{name}: Boolean) {{ true }}; \
        then sequence Good{name} {{}} timeout sequence Late{name} {{}}"
    )
}

impl Fixture {
    fn new(names: &[&str]) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("await.native");
        let controls = names
            .iter()
            .map(|name| awaiting(name))
            .collect::<Vec<_>>()
            .join("\n");
        let temporal = if names.len() == 2 {
            "temporal Due using t over (view: M::Widget) clock \"model-steps\" on origin { always[0,1] true }"
        } else {
            ""
        };
        std::fs::write(
            &source,
            format!(
                r#"language "ix:native" edition "1-draft";
profile v = "filament.model.complete/v1";
profile p = "filament.protocol.finite-global/v1";
profile t = "filament.temporal.event-position.false-extension/v1";
model M = "acme/orders";
{temporal}
protocol Flow using p over (input: M::Widget) on origin {{
    role R on M::Widget;
    run sequence Main {{ event Started by R as (started: Boolean) {{ true }}; {controls} }}
    finish End as (outcome: Boolean) {{ true }};
}}
"#
            ),
        )
        .unwrap();
        let request = CheckedHandoffRequest {
            emit: Request {
                source,
                package_name: "await-unit".into(),
                models: vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .unwrap()
                    .join("tests/fixtures/spine-model.semantic-ir.json")],
                libraries: vec![],
                execution_points: None,
                out: None,
                temporal_selections: names
                    .iter()
                    .map(|name| ClockSelection {
                        clause: format!("await Flow::{name}"),
                        clock: ClockConfiguration::EventPosition {
                            sequence_authority: "caller-events".into(),
                        },
                    })
                    .chain((names.len() == 2).then(|| ClockSelection {
                        clause: "Due".into(),
                        clock: ClockConfiguration::EventPosition {
                            sequence_authority: "caller-events".into(),
                        },
                    }))
                    .collect(),
            },
            corpus: Corpus::None,
            native_join: fsl_package::NativeJoinLimits::new(1 << 40, 1 << 28),
        };
        Self { directory, request }
    }

    fn write(&self, name: &str) -> (Value, Box<CheckedPackageV2>) {
        let path = self.directory.path().join(name);
        checked_handoff::write(&self.request, &path).expect("the actual Await acquisition writes");
        let sidecar =
            serde_json::from_slice(&std::fs::read(path.join("selection.json")).unwrap()).unwrap();
        let compiled = checked_handoff::compile(&path).expect("the exact clocks recompile");
        assert_eq!(
            compiled.sources().temporal_selections().len(),
            self.request.emit.temporal_selections.len()
        );
        assert!(compiled
            .sources()
            .temporal_selections()
            .iter()
            .all(|selection| selection.clock
                == ClockConfiguration::EventPosition {
                    sequence_authority: "caller-events".into()
                }));
        assert_eq!(
            compiled.compiled.emitted.package().bytes(),
            std::fs::read(path.join("checked-package.json")).unwrap()
        );
        (
            sidecar,
            checked_handoff::verify(&path).expect("the package and sidecar agree"),
        )
    }
}

fn departure(sidecar: &Value, package: &CheckedPackageV2, axis: SelectionAxis, name: Option<&str>) {
    let selected =
        Selection::read(&serde_json::to_vec(sidecar).unwrap()).expect("a shaped sidecar");
    let refused = selected
        .check(package)
        .expect_err("the exact axis must disagree");
    assert_eq!(refused.axis, axis);
    assert_eq!(refused.clause.as_deref(), name);
}

#[test]
#[trace("FR-517-AC-29", "FR-517-AC-30", "TC-1312")]
fn await_rows_roundtrip_the_supplied_clocks_in_source_order() {
    let fixture = Fixture::new(&["WaitZ", "WaitA"]);
    let (sidecar, package) = fixture.write("one");
    let (again, _) = fixture.write("two");
    assert_eq!(sidecar, again);
    assert_eq!(
        std::fs::read(fixture.directory.path().join("one/selection.json")).unwrap(),
        std::fs::read(fixture.directory.path().join("two/selection.json")).unwrap()
    );
    assert_eq!(sidecar["awaits"].as_array().unwrap().len(), 2);
    for (index, name) in ["WaitZ", "WaitA"].iter().enumerate() {
        let row = &sidecar["awaits"][index];
        assert_eq!(row["name"], format!("await Flow::{name}"));
        assert_eq!(row["binding"]["clock_name"], "line-clock");
        assert_eq!(
            row["binding"]["clock"]["sequence_authority"],
            "caller-events"
        );
        let node: CheckedNodeId = serde_json::from_value(row["node"].clone()).unwrap();
        let binding = package
            .temporal_bindings()
            .unwrap()
            .iter()
            .find(|binding| binding.declaration == node)
            .unwrap();
        assert_eq!(row["occurrence"], binding.occurrence);
        assert!(binding.source_unit.is_none() && binding.trace.is_none());
    }
    assert_eq!(
        package.temporal_bindings().unwrap().len(),
        3,
        "both Await bindings count alongside the temporal clause binding"
    );
}

#[test]
#[trace("FR-517-AC-30", "TC-1312")]
fn await_absence_keeps_the_old_sidecar_encoding() {
    let fixture = Fixture::new(&[]);
    let (mut sidecar, package) = fixture.write("old");
    assert!(sidecar.get("awaits").is_none());
    let old = Selection::read(&serde_json::to_vec(&sidecar).unwrap())
        .unwrap()
        .to_bytes()
        .unwrap();
    sidecar["awaits"] = json!([]);
    let selected = Selection::read(&serde_json::to_vec(&sidecar).unwrap()).unwrap();
    selected.check(&package).unwrap();
    assert_eq!(selected.to_bytes().unwrap(), old);
}

#[test]
#[trace("FR-517-AC-31", "TC-1312")]
fn await_binding_identity_and_clock_departures_are_named() {
    let fixture = Fixture::new(&["WaitZ", "WaitA"]);
    let (sidecar, package) = fixture.write("axes");
    for (member, value, axis) in [
        ("clock_name", json!("other-clock"), SelectionAxis::ClockName),
        (
            "clock",
            json!({"kind":"timestamped_event", "timestamp_unit":"s"}),
            SelectionAxis::ClockKind,
        ),
        (
            "clock",
            json!({"kind":"event_position", "sequence_authority":"different-events"}),
            SelectionAxis::ClockSequenceAuthority,
        ),
    ] {
        let mut changed = sidecar.clone();
        changed["awaits"][0]["binding"][member] = value;
        departure(&changed, &package, axis, Some("await Flow::WaitZ"));
    }
    for (member, value, axis) in [
        // Both are real Await nodes with equal clock configurations.
        (
            "node",
            sidecar["awaits"][1]["node"].clone(),
            SelectionAxis::ClauseNode,
        ),
        (
            "occurrence",
            json!(sidecar["awaits"][0]["occurrence"].as_u64().unwrap() + 1),
            SelectionAxis::ClauseBinding,
        ),
        ("binding", Value::Null, SelectionAxis::ClauseBinding),
    ] {
        let mut changed = sidecar.clone();
        changed["awaits"][0][member] = value;
        if member == "node" {
            // Keep the wrong-node expectation readable: duplicating the other
            // row's (node, occurrence) would be a distinct read-time defect.
            changed["awaits"].as_array_mut().unwrap().truncate(1);
        }
        departure(&changed, &package, axis, Some("await Flow::WaitZ"));
    }
    let mut non_await = sidecar.clone();
    let sequence = package
        .resolved_graph()
        .nodes
        .iter()
        .find(|node| {
            fsl_package::protocol_view::control_operation(node).is_ok_and(|operation| {
                operation == fsl_package::protocol_view::ControlOperation::Sequence
            })
        })
        .unwrap();
    non_await["awaits"][0]["node"] = serde_json::to_value(&sequence.node_id).unwrap();
    departure(
        &non_await,
        &package,
        SelectionAxis::ClauseNode,
        Some("await Flow::WaitZ"),
    );
    let mut fewer = sidecar.clone();
    fewer["awaits"].as_array_mut().unwrap().pop();
    departure(&fewer, &package, SelectionAxis::ClauseBinding, None);
}

#[test]
#[trace("FR-517-AC-32", "TC-1312")]
fn await_rows_require_each_member_and_unique_occurrences() {
    let fixture = Fixture::new(&["WaitZ"]);
    let (sidecar, _) = fixture.write("shape");
    for member in ["node", "name", "occurrence", "binding"] {
        let mut changed = sidecar.clone();
        changed["awaits"][0].as_object_mut().unwrap().remove(member);
        assert!(
            Selection::read(&serde_json::to_vec(&changed).unwrap()).is_err(),
            "missing {member}"
        );
    }
    for (member, value) in [
        ("extra", json!(true)),
        ("node", json!(false)),
        ("occurrence", json!(-1)),
    ] {
        let mut changed = sidecar.clone();
        changed["awaits"][0][member] = value;
        assert!(
            Selection::read(&serde_json::to_vec(&changed).unwrap()).is_err(),
            "{member}"
        );
    }
    let mut duplicate = sidecar.clone();
    duplicate["awaits"]
        .as_array_mut()
        .unwrap()
        .push(sidecar["awaits"][0].clone());
    assert!(Selection::read(&serde_json::to_vec(&duplicate).unwrap()).is_err());
    let bytes = serde_json::to_string(&sidecar).unwrap();
    let repeated = bytes.replace("\"binding\":", "\"binding\":null,\"binding\":");
    assert!(Selection::read(repeated.as_bytes()).is_err());
    let mut nullable = sidecar;
    nullable["awaits"][0]["binding"] = Value::Null;
    assert!(Selection::read(&serde_json::to_vec(&nullable).unwrap()).is_ok());
}

#[test]
#[trace("FR-517-AC-31", "TC-1312")]
fn await_native_binding_mutations_remain_strictly_admitted() {
    use filament_contract_model::{CheckedPackageDispatchResult, CheckedPackageReadLimits};
    let fixture = Fixture::new(&["WaitZ", "WaitA"]);
    let (sidecar, _) = fixture.write("native");
    let original: Value = serde_json::from_slice(
        &std::fs::read(fixture.directory.path().join("native/checked-package.json")).unwrap(),
    )
    .unwrap();
    let evidence = checked_handoff::evidence(&fixture.request).unwrap();
    let target = &sidecar["awaits"][0]["node"];
    for (case, axis) in [
        ("authority", SelectionAxis::ClockSequenceAuthority),
        ("name", SelectionAxis::ClockName),
        ("occurrence", SelectionAxis::ClauseBinding),
        ("extra", SelectionAxis::ClauseBinding),
    ] {
        let mut wire = original.clone();
        let binding_at = wire["temporal_bindings"]
            .as_array()
            .unwrap()
            .iter()
            .position(|row| &row["declaration"] == target)
            .unwrap();
        let control_at = wire["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|node| &node["node_id"] == target)
            .unwrap();
        match case {
            "authority" => {
                wire["temporal_bindings"][binding_at]["clock"]["sequence_authority"] =
                    json!("other-events")
            }
            "name" => {
                wire["temporal_bindings"][binding_at]["clock_name"] = json!("other-clock");
                let clock = wire["semantic_graph"]["nodes"][control_at]["body"]["arguments"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|arg| arg["name"] == "clock")
                    .unwrap();
                clock["value"]["value"] = json!("other-clock");
            }
            "occurrence" | "extra" => {
                let ordinal = sidecar["awaits"][0]["occurrence"].as_u64().unwrap();
                let next = ordinal + 1;
                let mut binding = wire["temporal_bindings"][binding_at].clone();
                binding["occurrence"] = json!(next);
                let occurrences = wire["semantic_graph"]["nodes"][control_at]["occurrences"]
                    .as_array_mut()
                    .unwrap();
                let at = occurrences
                    .iter()
                    .position(|item| item["role"] == "declaration" && item["ordinal"] == ordinal)
                    .unwrap();
                let mut occurrence = occurrences[at].clone();
                occurrence["ordinal"] = json!(next);
                if case == "extra" {
                    occurrences.insert(at + 1, occurrence);
                } else {
                    occurrences[at] = occurrence;
                }
                let source_map = wire["source_map"].as_array_mut().unwrap();
                let at = source_map
                    .iter()
                    .position(|row| {
                        &row["node_id"] == target
                            && row["role"] == "declaration"
                            && row["ordinal"] == ordinal
                    })
                    .unwrap();
                let mut region = source_map[at].clone();
                region["ordinal"] = json!(next);
                if case == "extra" {
                    source_map.insert(at + 1, region);
                } else {
                    source_map[at] = region;
                }
                if case == "extra" {
                    wire["temporal_bindings"]
                        .as_array_mut()
                        .unwrap()
                        .insert(binding_at + 1, binding);
                } else {
                    wire["temporal_bindings"][binding_at] = binding;
                }
            }
            _ => unreachable!(),
        }
        let result = filament_contract_model::read_checked_package(
            &serde_json::to_vec(&wire).unwrap(),
            CheckedPackageReadLimits::bounded(),
            &evidence,
        );
        let CheckedPackageDispatchResult::AdmittedV2(package) = result else {
            panic!("{case} must reach the selection comparison after strict admission: {result:?}");
        };
        departure(&sidecar, &package, axis, Some("await Flow::WaitZ"));
    }
}

#[test]
#[trace("FR-517-AC-32", "TC-1312")]
fn await_selectors_refuse_unknown_duplicate_missing_and_ambiguous_requests() {
    let mut fixture = Fixture::new(&["WaitZ"]);
    let selection = fixture.request.emit.temporal_selections[0].clone();
    fixture
        .request
        .emit
        .temporal_selections
        .push(selection.clone());
    assert!(matches!(
        checked_handoff::write(
            &fixture.request,
            &fixture.directory.path().join("duplicate")
        ),
        Err(Error::Compile(
            crate::emit_checked_v2::Error::DuplicateClause { .. }
        ))
    ));
    fixture.request.emit.temporal_selections = vec![ClockSelection {
        clause: "await Flow::Missing".into(),
        ..selection.clone()
    }];
    assert!(matches!(
        checked_handoff::write(&fixture.request, &fixture.directory.path().join("unknown")),
        Err(Error::Compile(
            crate::emit_checked_v2::Error::UnknownClause { .. }
        ))
    ));
    fixture.request.emit.temporal_selections.clear();
    let missing =
        checked_handoff::write(&fixture.request, &fixture.directory.path().join("missing"))
            .expect_err("the clock selection remains required");
    let Error::Compile(crate::emit_checked_v2::Error::Producer(refusal)) = missing else {
        panic!("the emitter must name the missing selection: {missing:?}");
    };
    assert_eq!(refusal.code().as_str(), "missing_declaration");
    assert_eq!(refusal.cause(), Some("missing-selection"));
    fixture.request.emit.temporal_selections = vec![selection];
    let source = std::fs::read_to_string(&fixture.request.emit.source).unwrap();
    let original = awaiting("WaitZ");
    let nested = original.replace("after Started", "after Main::Started");
    let right = nested.replace("seenWaitZ", "seenOther");
    let ambiguous = source.replace(
        &original,
        &format!("sequence Left {{ {nested} }} sequence Right {{ {right} }}"),
    );
    std::fs::write(&fixture.request.emit.source, ambiguous).unwrap();
    assert!(matches!(
        checked_handoff::write(
            &fixture.request,
            &fixture.directory.path().join("ambiguous")
        ),
        Err(Error::Compile(
            crate::emit_checked_v2::Error::AmbiguousClause { .. }
        ))
    ));
    for name in ["duplicate", "unknown", "missing", "ambiguous"] {
        assert!(!fixture.directory.path().join(name).exists());
    }
}
