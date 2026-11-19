// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1033 (TC-2959): a `native-analyze/1` request selects a protocol
//! subject through `subject.protocol`, reads the `memory` member, requests
//! race freedom over a `parallel`, and refuses bad selections and bindings.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, litmus_request, request_over, Workspace};
use crate::litmus::{sb_run, sb_unit};

fn binding() -> Value {
    json!({"type": "model-binding", "selection": "test/litmus"})
}

fn sb_subject() -> Value {
    json!({"protocol": {"protocol": "SB",
        "over": {"reference": {"population": "ix://test/litmus/cores", "key": "core"}},
        "roles": {"w": binding(), "c": binding()}}})
}

/// FR-1033-AC-1: the claim holds over `SB` under `sc` and is refuted under a
/// `tso` selection, the counterexample replayed.
#[trace("TC-2959", "FR-1033-AC-1")]
#[trace("FSpec-TC-2005", "FSpec-FR-520-AC-6")]
#[test]
fn a_protocol_subject_is_decided_under_the_selected_memory_model() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let ws = Workspace::new("protocol-subject-sc");
    let sc = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": sb_subject(), "items": [{"claim": "NoBothZero"}]}),
    ));
    assert_eq!(label(&sc.item(0)), "proved", "{}", sc.document());
    let ws = Workspace::new("protocol-subject-tso");
    let tso = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": sb_subject(), "memory": [{"parallel": "SB::Both", "model": "tso"}],
            "items": [{"claim": "NoBothZero"}]}),
    ));
    assert_eq!(label(&tso.item(0)), "refuted", "{}", tso.document());
    assert_eq!(tso.item(0)["replay"]["reproduced"], true);
    assert_eq!(tso.exit, 10);
    assert_eq!(
        tso.item(0)["request"]["temporal"]["subject"]["protocol"]["protocol"],
        "SB",
        "{}",
        tso.document()
    );
    assert_ne!(sc.item(0)["request"], tso.item(0)["request"]);
}

/// FR-1033-AC-2: a race-freedom item over two unsynchronised non-atomic
/// writes is refuted naming both; naming another protocol is unknown-claim.
#[trace("TC-2959", "FR-1033-AC-2")]
#[test]
fn a_race_freedom_item_over_a_parallel_is_refuted() {
    let body = sb_unit(&sb_run("memory tso", "", "acquire"));
    let ws = Workspace::new("protocol-race");
    let run = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": sb_subject(),
            "items": [{"claim": "SB::Both", "temporal": {"item_kind": "race-freedom"}}]}),
    ));
    assert_eq!(label(&run.item(0)), "refuted", "{}", run.document());
    let race = &run.item(0)["counterexample"]["kind"];
    assert!(race.to_string().contains("race"), "{}", run.document());
    let ws = Workspace::new("protocol-race-default-items");
    let defaults = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": sb_subject()}),
    ));
    let items = defaults.document()["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        items
            .iter()
            .any(|item| item["request"]["temporal"]["item_kind"] == "race-freedom"),
        "{}",
        defaults.document()
    );
}

/// FR-1033-AC-4 (negative): an unknown model and a selection naming no
/// outermost `parallel` refuse the whole request.
#[trace("TC-2959", "FR-1033-AC-4")]
#[test]
fn bad_memory_selections_refuse_the_request() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    for (n, memory) in [
        json!([{"parallel": "SB::Both", "model": "pso"}]),
        json!([{"parallel": "SB::Both::Sx", "model": "tso"}]),
        json!([{"parallel": "Nope::Both", "model": "tso"}]),
        json!([{"parallel": "SB::Both", "model": "tso"}, {"parallel": "SB::Both", "model": "ra"}]),
    ]
    .into_iter()
    .enumerate()
    {
        let ws = Workspace::new(&format!("protocol-memory-{n}"));
        let run = analyze(&litmus_request(
            &ws,
            &body,
            json!({"subject": sb_subject(), "memory": memory, "items": [{"claim": "NoBothZero"}]}),
        ));
        assert_eq!(run.exit, 20, "{}", run.document());
        assert!(
            run.document()["items"]
                .as_array()
                .is_some_and(Vec::is_empty),
            "{}",
            run.document()
        );
        assert!(
            !run.document()
                .to_string()
                .contains("not read by this build"),
            "{}",
            run.document()
        );
    }
}

/// FR-1033-AC-5 (negative): a protocol the package does not declare
/// declines the item; an unknown binding member refuses the request.
#[trace("TC-2959", "FR-1033-AC-5")]
#[test]
fn bad_protocol_bindings_decline_or_refuse() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let ws = Workspace::new("protocol-binding-missing");
    let run = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": {"protocol": {"protocol": "Missing"}}, "items": [{"claim": "NoBothZero"}]}),
    ));
    assert_eq!(label(&run.item(0)), "declined", "{}", run.document());
    let ws = Workspace::new("protocol-binding-extra");
    let run = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": {"protocol": {"protocol": "SB", "extra": 1}},
            "items": [{"claim": "NoBothZero"}]}),
    ));
    assert_eq!(run.exit, 20, "{}", run.document());
}

/// FR-225-AC-4's domain: `Cell` with `v` (the `Level` range 0 to 2, since a `Bit` type is not assignable to `r`) (and an
/// `id` selecting it), universe `{x, y, z}` at 0, `set()` storing 1; `Core`
/// with one register `r: Int[0, 2]` and `read(c)`, which loads `c.v` into it.
fn ac_domain() -> Vec<u8> {
    let mut doc: Value = serde_json::from_slice(&crate::litmus::resulting_document())
        .expect("the litmus package is JSON");
    let types = doc["types"].as_array_mut().expect("types");
    let level = types
        .iter()
        .position(|node| node["identity"] == "ix://test/litmus/Level")
        .expect("Level");
    types[level] = serde_json::from_str(
        &types[level]
            .to_string()
            .replace("\"value\":\"3\"", "\"value\":\"2\""),
    )
    .expect("Level");
    for node in types.iter_mut() {
        let identity = node["identity"].as_str().unwrap_or_default().to_owned();
        if identity == "ix://test/litmus/Cell" {
            let fields = node["fields"].as_array_mut().expect("fields");
            fields.retain(|field| field["name"] != "w");
            for field in fields.iter_mut().filter(|field| field["name"] == "v") {
                field["typeRef"] = json!("ix://test/litmus/Level");
            }
            node["operations"]
                .as_array_mut()
                .expect("operations")
                .retain(|op| op["name"] == "set");
        }
        if identity == "ix://test/litmus/Core" {
            let fields = node["fields"].as_array_mut().expect("fields");
            fields.truncate(1);
            fields[0] = serde_json::from_str(
                &fields[0]
                    .to_string()
                    .replace("Core/r1", "Core/r")
                    .replace("\"r1\"", "\"r\""),
            )
            .expect("r");
            let operations = node["operations"].as_array_mut().expect("operations");
            operations.retain(|op| op["name"] == "read1");
            operations[0] = serde_json::from_str(
                &operations[0]
                    .to_string()
                    .replace("Core/read1", "Core/read")
                    .replace("\"read1\"", "\"read\"")
                    .replace("Core/r1", "Core/r"),
            )
            .expect("read");
        }
    }
    doc.to_string().into_bytes()
}

/// The AC's contracts: `IsX`, `IsY` and `IsZ` select a cell, `StoreOne`
/// stores 1, `ReadsX`, `ReadsY` and `ReadsZ` select the cell `read` loads.
fn ac_contracts() -> String {
    let mut text = String::from("post StoreOne using v on M::Cell::set { self.v = 1 }\n");
    for (suffix, id) in [("X", 0), ("Y", 1), ("Z", 2)] {
        text.push_str(&format!(
            "pre Is{suffix} using v on M::Cell::set {{ self.id = {id} }}\n\
             pre Reads{suffix} using v on M::Core::read {{ c.id = {id} }}\n"
        ));
    }
    text + "post Loaded using v on M::Core::read { self.r = pre(c.v) }\n"
}

/// The AC's request: three cells at 0, `core` with `r` at 2.
fn three_cell_request(ws: &Workspace, body: &str, extra: Value) -> std::path::PathBuf {
    let int = |value: i64| json!({"integer": value.to_string()});
    let cell = |key: &str, id: i64| {
        json!({"key": key, "type": "ix://test/litmus/Cell",
            "fields": {"id": int(id), "v": int(0)}})
    };
    let snapshot = json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "s0"},
        "observation": "current", "model": {"identity": "test/litmus"},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [
            {"population": "ix://test/litmus/spares", "complete": true, "objects": []},
            {"population": "ix://test/litmus/cells", "complete": true,
                "objects": [cell("x", 0), cell("y", 1), cell("z", 2)]},
            {"population": "ix://test/litmus/cores", "complete": true,
                "objects": [{"key": "core", "type": "ix://test/litmus/Core",
                    "fields": {"r": int(2)}}]},
        ],
    });
    request_over(
        ws,
        &crate::litmus::unit_text(body),
        &ac_domain(),
        &[snapshot.to_string().into_bytes()],
        extra,
    )
}

/// FR-1033-AC-6 and FR-225-AC-4: FR-225-AC-2's three unfenced stores under
/// `tso` with `max_store_buffer` 2 and the state constraint `Zero` settle
/// `ConstraintReached` with no counterexample, stating the store-buffer
/// bound as reached; without `Zero` they settle `MemoryBoundReached`.
#[trace("TC-2959", "FR-1033-AC-6")]
#[trace("TC-670", "FR-225-AC-4")]
#[test]
fn a_constrained_store_buffer_run_settles_constraint_reached() {
    use crate::litmus::attempt;
    let run = format!(
        "parallel Both {{ branch left sequence L {{ {} {} {} }} \
         branch right sequence R {{ {} {} {} }} }} join all [left, right];",
        attempt("Sx", "w", "Cell::set", "release", "IsX, StoreOne"),
        attempt("Sy", "w", "Cell::set", "release", "IsY, StoreOne"),
        attempt("Sz", "w", "Cell::set", "release", "IsZ, StoreOne"),
        attempt("Lx", "c", "Core::read", "acquire", "ReadsX, Loaded"),
        attempt("Ly", "c", "Core::read", "acquire", "ReadsY, Loaded"),
        attempt("Lz", "c", "Core::read", "acquire", "ReadsZ, Loaded"),
    );
    let body = format!(
        "{}constraint Zero using v on M::Cell at current {{ self.v <= 0 }}\n\
         temporal Holds using t0 over (c: M::Cell) clock \"model-steps\" on origin \
         {{ always holds(c.v <= 1) }}\n",
        ac_contracts() + &crate::litmus::protocol("SB", &run)
    );
    let memory = json!([{"parallel": "SB::Both", "model": "tso"}]);
    let item = |constraint: bool| {
        let mut item = json!({"claim": "Holds", "temporal": {"limits": {"max_store_buffer": 2}}});
        if constraint {
            item["reductions"] = json!({"constraint": "Zero"});
        }
        item
    };
    let ws = Workspace::new("protocol-constraint-reached");
    let run_with = analyze(&three_cell_request(
        &ws,
        &body,
        json!({"subject": sb_subject(), "memory": memory, "items": [item(true)]}),
    ));
    let record = &run_with.item(0)["record"];
    assert_eq!(
        label(&run_with.item(0)),
        "inconclusive",
        "{}",
        run_with.document()
    );
    assert!(
        record.to_string().contains("constraint-reached"),
        "{record}"
    );
    assert!(run_with.item(0).get("counterexample").is_none(), "{record}");
    assert!(record.to_string().contains("max_store_buffer"), "{record}");
    assert_eq!(
        record["memory_bound"],
        json!({"bound": "max_store_buffer", "value": 2, "reached": true}),
        "{record}"
    );
    // 21 states: the pre-fork initial state, 12 expanded states with no store
    // flushed (`left` at 0, 1 or 2 buffered stores, `right` at 0 to 3 loads), and
    // 8 boundary states (`left`'s first flush from a state holding a store).
    // FR-225-AC-4 counts 20 because it omits the pre-fork initial state.
    assert_eq!(
        run_with.item(0)["exploration"]["states"],
        json!("21"),
        "{}",
        run_with.document()
    );
    assert_eq!(record["cause"]["boundary_states"], 8, "{record}");
    let ws = Workspace::new("protocol-memory-bound-reached");
    let run_without = analyze(&three_cell_request(
        &ws,
        &body,
        json!({"subject": sb_subject(), "memory": memory, "items": [item(false)]}),
    ));
    assert!(
        run_without.item(0)["record"]
            .to_string()
            .contains("memory-bound-reached"),
        "{}",
        run_without.document()
    );
}

/// FR-1033-AC-3: an item with no `subject.protocol` is decided over the model
/// subject, and its echoed request names no protocol.
#[trace("TC-2959", "FR-1033-AC-3")]
#[test]
fn an_item_without_a_protocol_subject_is_decided_over_the_model_subject() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let ws = Workspace::new("protocol-subject-model");
    let run = analyze(&litmus_request(
        &ws,
        &body,
        json!({"items": [{"claim": "NoBothZero"}]}),
    ));
    let item = run.item(0);
    assert!(
        item["request"]["temporal"]["subject"]
            .get("protocol")
            .is_none(),
        "{}",
        run.document()
    );
    assert!(
        !item["record"].to_string().contains("max_store_buffer"),
        "{}",
        run.document()
    );
}

/// FR-1033-AC-5: a role binding the protocol does not admit settles the item
/// `declined` with `wrong-role-mapping`.
#[trace("TC-2959", "FR-1033-AC-5")]
#[test]
fn a_role_binding_the_protocol_does_not_admit_declines_the_item() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let ws = Workspace::new("protocol-binding-role");
    let run = analyze(&litmus_request(
        &ws,
        &body,
        json!({"subject": {"protocol": {"protocol": "SB",
            "over": {"reference": {"population": "ix://test/litmus/cores", "key": "core"}},
            "roles": {"w": binding(), "c": binding(), "zz": binding()}}},
            "items": [{"claim": "NoBothZero"}]}),
    ));
    assert_eq!(label(&run.item(0)), "declined", "{}", run.document());
    assert!(
        run.document().to_string().contains("wrong-role-mapping"),
        "{}",
        run.document()
    );
}

/// Analyze `extra` over the `SB` unit `body` from source and from the
/// handoff directory its program compiles into; returns both runs.
fn sb_both_routes(
    name: &str,
    body: &str,
    extra: Value,
) -> (
    crate::analyze_cli_support::Run,
    crate::analyze_cli_support::Run,
) {
    let ws = Workspace::new(name);
    let path = litmus_request(&ws, body, extra);
    let source = analyze(&path);
    let document: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("the request")).expect("JSON");
    let (members, _) = crate::analyze_cli_support::package_selection(&ws, &document["request"]);
    let package_request = json!({"format": "native-analyze/1", "request": members});
    let package = analyze(&ws.write(
        "analyze-package.json",
        package_request.to_string().as_bytes(),
    ));
    (source, package)
}

/// FR-850-AC-11, FR-940-AC-14: `SB` under `tso`, `SB` with no `memory` and
/// the race-freedom item over its racing variant settle from the package
/// with the source route's records, and the package request is not refused
/// for carrying `memory`.
#[trace("TC-3050", "FR-850-AC-11")]
#[trace("TC-3050", "FR-940-AC-14")]
#[test]
#[ignore = "QSL-2405: the package-route protocol subject is not implemented yet"]
fn a_protocol_subject_settles_from_the_package_as_from_source() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let tso = json!({"subject": sb_subject(), "memory": [{"parallel": "SB::Both", "model": "tso"}],
        "items": [{"claim": "NoBothZero"}]});
    let sc = json!({"subject": sb_subject(), "items": [{"claim": "NoBothZero"}]});
    let race_body = sb_unit(&sb_run("memory tso", "", "acquire"));
    let race = json!({"subject": sb_subject(),
        "items": [{"claim": "SB::Both", "temporal": {"item_kind": "race-freedom"}}]});
    for (name, unit, extra, expected) in [
        ("package-protocol-tso", &body, tso, "refuted"),
        ("package-protocol-sc", &body, sc, "proved"),
        ("package-protocol-race", &race_body, race, "refuted"),
    ] {
        let (source, package) = sb_both_routes(name, unit, extra);
        assert_eq!(label(&source.item(0)), expected, "{}", source.document());
        assert!(
            !package.document().to_string().contains("unsupported"),
            "{}",
            package.document()
        );
        for member in ["items", "aggregate", "last_stage", "category", "registry"] {
            assert_eq!(
                package.document()[member],
                source.document()[member],
                "{name} {member}: {}",
                package.document()
            );
        }
        assert_eq!(package.exit, source.exit, "{name}");
    }
}
