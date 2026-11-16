// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230-AC-50, FR-230-AC-54, FSpec FR-322 "Clause nodes"
//! (FR-322-AC-261, FR-322-AC-263, FR-322-AC-265), FR-342-AC-11: a dense
//! model's guards, resets, time invariants and `Clock` parameters reach S4
//! as `clock_guard`, `clock_reset`, `time_invariant` and `clock_constraint`
//! nodes, each located at its source and listed in declared order, and the
//! members S4 does not write yet refuse `not-yet-implemented` at the member.

use super::clock_places::{clocked_document_with, clocked_unit, member, vector_document};
use super::*;
use filament_semantic_value::checking::CheckMode;

const CLOCK: &str = "ix://filament/native/Clock";

/// ConfigVersion with the clock `x`, `attemptUpdate`'s frame naming `x`
/// too, and, when `parameter` holds, `attemptUpdate(c: Clock)`.
fn document(parameter: bool) -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&clocked_document_with(false)).expect("JSON");
    let operation = &mut document["types"][1]["operations"][0];
    operation["frame"]["modifies"]
        .as_array_mut()
        .expect("modifies")
        .push(json!("ix://example/config-version/ConfigVersion/x"));
    if parameter {
        let identity = "ix://example/config-version/ConfigVersion/attemptUpdate/c";
        operation["params"] = json!([{
            "identity": identity, "name": "c", "typeRef": CLOCK, "presence": "required",
            "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false},
            "origin": {"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
                "inputIdentities": [identity]}},
        }]);
    }
    crate::emit::tests::wire_bytes(&document).expect("JSON")
}

/// The checked unit over `document` with `body`, and the unit's text.
fn checked(
    document: &[u8],
    body: &str,
) -> (
    String,
    Result<fsl_semantics::check::CheckedGraph, Vec<fsl_semantics::check::CheckRefusal>>,
) {
    let packages = fsl_semantics::model::intake::package_input([document]).unwrap();
    let unit = clocked_unit(body);
    let checked = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckMode::Linked, CheckingLimits::default());
    (unit, checked)
}

/// The emitted wire of the unit over `document` with `body`, and the
/// unit's text.
fn emitted(document: &[u8], body: &str) -> (String, Value) {
    let (unit, checked) = checked(document, body);
    let graph = checked.unwrap_or_else(|refusals| panic!("{body}: the unit checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    (unit, wire(&emission))
}

/// The node of `wire` that `reference` names.
fn target<'w>(wire: &'w Value, reference: &Value) -> &'w Value {
    let key = &reference["target"];
    nodes(wire)
        .iter()
        .find(|node| &node["node_id"] == key)
        .unwrap_or_else(|| panic!("{reference} names a node"))
}

/// Every node of `wire` of the timed form `form`.
fn timed<'w>(wire: &'w Value, form: &str) -> Vec<&'w Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["node_tag"] == "timed_model" && node["semantic_form"] == form)
        .collect()
}

/// The text a node's first source-map region covers in `unit`.
fn located<'u>(wire: &Value, node: &Value, unit: &'u str) -> &'u str {
    let entry = wire["source_map"]
        .as_array()
        .expect("source map")
        .iter()
        .find(|entry| entry["node_id"] == node["node_id"])
        .unwrap_or_else(|| panic!("{} is located", node["semantic_form"]));
    let region = &entry["regions"][0];
    let at = |name: &str| region[name].as_u64().expect("a position") as usize;
    &unit[at("start")..at("end")]
}

/// A literal member's value, as text.
fn literal(node: &Value, name: &str) -> String {
    match &member(node, name)["value"] {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// The names of a node's body members, in order.
fn names(body: &Value) -> Vec<&str> {
    body["members"]
        .as_array()
        .expect("an aggregate")
        .iter()
        .map(|member| member["name"].as_str().unwrap_or("-"))
        .collect()
}

/// The `true` predicate: an `expression`/`reference` node over the
/// `value`/`literal` node `true`.
fn assert_true(wire: &Value, node: &Value) {
    assert_eq!(node["node_tag"], "expression");
    assert_eq!(node["semantic_form"], "reference");
    let literal = target(wire, &node["body"]);
    assert_eq!(
        (
            &literal["node_tag"],
            &literal["semantic_form"],
            &literal["body"]["value"]
        ),
        (&json!("value"), &json!("literal"), &json!(true))
    );
}

/// `self`: a level-0 `parameter` node.
fn assert_self(wire: &Value, reference: &Value) {
    let node = target(wire, reference);
    assert_eq!(node["semantic_form"], "parameter");
    assert_eq!(literal(&node["body"], "name"), "self");
    assert_eq!(literal(&node["body"], "level"), "0");
}

/// The emitted package's timed aggregates are in FSpec FR-322's declared
/// order.
fn assert_declared_order(wire: &Value) {
    let (nodes, source_map) = admitted_graph(wire);
    assert_eq!(
        crate::emit::timed_order::first_out_of_order(&nodes, &source_map),
        Ok(None)
    );
}

/// The typed nodes and source map the last emission on this thread built.
fn admitted_graph(
    _wire: &Value,
) -> (
    Vec<filament_contract_model::CheckedSemanticNodeV2>,
    Vec<filament_contract_model::CheckedSourceMapEntry>,
) {
    crate::emit::TIMED_GRAPH
        .with(|held| held.borrow().clone())
        .expect("the emission built a timed graph")
}

/// FR-230-AC-54, FSpec FR-322-AC-261, FR-322-AC-263 and
/// FR-342-AC-11: over `x: Clock`, the precondition `self.x >= 1 ms and
/// self.versionNumber > 0 or self.x < 5 ms` is one `clock_guard` of
/// `attemptUpdate`'s no-point `operation_anchor`, with two parts: the first
/// the expression node of `self.versionNumber > 0` and the constraint `ge
/// 1/1`, the second `true` and `lt 5/1`. The postcondition `self.x = 0 ms`
/// is one `clock_reset` of kind `place`, owner `self`, no slot or binder and
/// the condition `true`. `time invariant Bounded when {
/// self.versionNumber >= 0 } { self.x <= 9 ms }` is one `time_invariant`
/// with the bound `le 9/1`, and `urgent Always when { true }` names the `true` expression. A
/// place is exactly `model`, `path`, `resolved_path`. The root lists every
/// clause and every constraint once, in declared order, and each clause
/// node is located at its own source.
#[trace("TC-685", "FR-230-AC-50", "FR-230-AC-54")]
#[trace(
    "FSpec-FR-322-AC-261",
    "FSpec-FR-322-AC-263",
    "FSpec-FR-322-AC-265",
    "FSpec-FR-342-AC-11"
)]
#[test]
fn guards_resets_and_invariants_emit_clause_nodes() {
    let (unit, wire) = emitted(
        &document(false),
        "pre Late using v on Config::ConfigVersion::attemptUpdate { \
         self.x >= 1 ms and self.versionNumber > 0 or self.x < 5 ms }\n\
         post Reset using v on Config::ConfigVersion::attemptUpdate { self.x = 0 ms }\n\
         time invariant Bounded when { self.versionNumber >= 0 } { self.x <= 9 ms }\n\
         urgent Always when { true };\n",
    );
    let [place] = timed(&wire, "clock_place")[..] else {
        panic!("one place");
    };
    assert_eq!(names(&place["body"]), ["model", "path", "resolved_path"]);

    let [guard] = timed(&wire, "clock_guard")[..] else {
        panic!("one guard");
    };
    assert_eq!(names(&guard["body"]), ["operation", "parts"]);
    assert!(located(&wire, guard, &unit).starts_with("pre Late"));
    let anchor = target(&wire, member(&guard["body"], "operation"));
    assert_eq!(anchor["semantic_form"], "operation_anchor");
    assert_eq!(literal(&anchor["body"], "operation"), "attemptUpdate");
    assert_eq!(names(&anchor["body"]), ["context", "operation", "frame"]);
    let parts = member(&guard["body"], "parts")["members"]
        .as_array()
        .expect("parts");
    let [first, second] = parts.as_slice() else {
        panic!("two parts: {parts:?}");
    };
    let part = |part: &Value| -> (Value, Vec<Value>) {
        let members = part["members"].as_array().expect("a part");
        assert_eq!(members[0]["name"], "predicate");
        (
            target(&wire, &members[0]["value"]).clone(),
            members[1..]
                .iter()
                .map(|reference| target(&wire, reference).clone())
                .collect(),
        )
    };
    let (data, constraints) = part(first);
    assert_eq!(data["node_tag"], "expression");
    assert_eq!(data["body"]["term"], "application");
    assert_eq!(located(&wire, &data, &unit), "self.versionNumber > 0");
    let [constraint] = constraints.as_slice() else {
        panic!("one constraint");
    };
    assert_eq!(
        names(&constraint["body"]),
        ["kind", "place", "owner", "slots", "comparison", "bound"]
    );
    assert_eq!(literal(&constraint["body"], "kind"), "place");
    assert_eq!(literal(&constraint["body"], "comparison"), "ge");
    assert_eq!(literal(&constraint["body"], "bound"), "1/1");
    assert_eq!(
        member(&constraint["body"], "place")["target"],
        place["node_id"]
    );
    assert_self(&wire, member(&constraint["body"], "owner"));
    assert_eq!(member(&constraint["body"], "slots")["members"], json!([]));
    assert_eq!(located(&wire, constraint, &unit), "self.x >= 1 ms");
    let (data, constraints) = part(second);
    assert_true(&wire, &data);
    let [constraint] = constraints.as_slice() else {
        panic!("one constraint");
    };
    assert_eq!(literal(&constraint["body"], "comparison"), "lt");
    assert_eq!(literal(&constraint["body"], "bound"), "5/1");

    let [reset] = timed(&wire, "clock_reset")[..] else {
        panic!("one reset");
    };
    assert_eq!(
        names(&reset["body"]),
        [
            "kind",
            "operation",
            "place",
            "owner",
            "slots",
            "binders",
            "condition",
            "value"
        ]
    );
    assert_eq!(
        member(&reset["body"], "operation"),
        member(&guard["body"], "operation"),
        "a guard and a reset name one anchor"
    );
    assert_self(&wire, member(&reset["body"], "owner"));
    assert_eq!(member(&reset["body"], "binders")["members"], json!([]));
    assert_true(&wire, target(&wire, member(&reset["body"], "condition")));
    assert_eq!(literal(&reset["body"], "value"), "0/1");
    assert_eq!(located(&wire, reset, &unit), "self.x = 0 ms");

    let [invariant] = timed(&wire, "time_invariant")[..] else {
        panic!("one invariant");
    };
    assert_eq!(names(&invariant["body"]), ["predicate", "bounds"]);
    let predicate = target(&wire, member(&invariant["body"], "predicate"));
    assert_eq!(located(&wire, predicate, &unit), "self.versionNumber >= 0");
    let [bound] = member(&invariant["body"], "bounds")["members"]
        .as_array()
        .expect("bounds")
        .as_slice()
    else {
        panic!("one bound");
    };
    let bound = target(&wire, bound);
    assert_eq!(literal(&bound["body"], "comparison"), "le");
    assert_eq!(literal(&bound["body"], "bound"), "9/1");
    assert!(located(&wire, invariant, &unit).starts_with("time invariant Bounded"));

    let [urgency] = timed(&wire, "urgency")[..] else {
        panic!("one urgency");
    };
    assert_true(&wire, target(&wire, member(&urgency["body"], "predicate")));

    let [root] = timed(&wire, "timed_model")[..] else {
        panic!("one root");
    };
    let listed = |name: &str| {
        member(&root["body"], name)["members"]
            .as_array()
            .unwrap()
            .len()
    };
    assert_eq!(
        ["resets", "guards", "invariants", "urgencies", "constraints"].map(listed),
        [1, 1, 1, 1, 3]
    );
    assert_eq!(timed(&wire, "clock_constraint").len(), 3);
    assert_declared_order(&wire);
}

/// FSpec FR-322 "Clause nodes": the comparison `eq` is a guard's, and a
/// constraint written twice is one node with an occurrence at each source,
/// listed once by the root.
#[trace("TC-685", "FR-230-AC-54", "FSpec-FR-322-AC-261")]
#[test]
fn an_equal_constraint_is_one_node_located_at_each_source() {
    let (unit, wire) = emitted(
        &document(false),
        "pre Exact using v on Config::ConfigVersion::attemptUpdate { \
         self.x = 2 ms or self.versionNumber > 0 and self.x = 2 ms }\n",
    );
    let [constraint] = timed(&wire, "clock_constraint")[..] else {
        panic!("one constraint");
    };
    assert_eq!(literal(&constraint["body"], "comparison"), "eq");
    let regions: Vec<&str> = wire["source_map"]
        .as_array()
        .expect("source map")
        .iter()
        .filter(|entry| entry["node_id"] == constraint["node_id"])
        .map(|entry| {
            let region = &entry["regions"][0];
            &unit[region["start"].as_u64().unwrap() as usize
                ..region["end"].as_u64().unwrap() as usize]
        })
        .collect();
    assert_eq!(regions, ["self.x = 2 ms", "self.x = 2 ms"]);
    assert_eq!(constraint["occurrences"].as_array().unwrap().len(), 2);
    assert_declared_order(&wire);
}

/// FR-230 "Operation parameters", FSpec FR-322 "Clause nodes" and
/// FR-342-AC-11: over `attemptUpdate(c: Clock)`, `c >= 1 ms` is a
/// `clock_constraint` of kind `parameter` naming the operation's anchor and
/// `c`, and `c = 0 ms` a `clock_reset` of kind `parameter` with no
/// `condition`: the `Clock` parameter is an inline name, no node.
#[trace("TC-685", "FR-230-AC-54", "FSpec-FR-322-AC-261", "FSpec-FR-322-AC-262")]
#[test]
fn a_clock_parameter_is_an_inline_name_of_its_operation() {
    let (_, wire) = emitted(
        &document(true),
        "pre Late using v on Config::ConfigVersion::attemptUpdate { c >= 1 ms }\n\
         post Reset using v on Config::ConfigVersion::attemptUpdate { c = 0 ms }\n",
    );
    let [constraint] = timed(&wire, "clock_constraint")[..] else {
        panic!("one constraint");
    };
    assert_eq!(
        names(&constraint["body"]),
        ["kind", "operation", "parameter", "comparison", "bound"]
    );
    assert_eq!(literal(&constraint["body"], "kind"), "parameter");
    assert_eq!(literal(&constraint["body"], "parameter"), "c");
    let anchor = target(&wire, member(&constraint["body"], "operation"));
    assert_eq!(anchor["semantic_form"], "operation_anchor");
    let [reset] = timed(&wire, "clock_reset")[..] else {
        panic!("one reset");
    };
    assert_eq!(
        names(&reset["body"]),
        ["kind", "operation", "parameter", "value"]
    );
    assert_eq!(literal(&reset["body"], "parameter"), "c");
    assert_eq!(
        member(&reset["body"], "operation"),
        member(&constraint["body"], "operation")
    );
    let [guard] = timed(&wire, "clock_guard")[..] else {
        panic!("one guard");
    };
    assert_eq!(
        member(&guard["body"], "operation"),
        member(&constraint["body"], "operation")
    );
    assert_declared_order(&wire);
}

/// FSpec FR-322 "Clause nodes": a model has one `clock_guard` for each
/// operation anchor, so two guarded preconditions of one operation,
/// `self.x >= 1 ms or self.x > 7 ms` and `self.versionNumber > 0 and
/// self.x < 5 ms`,
/// are the one guard of their conjunction: each part of the one a part of
/// each part of the other, its predicate the `and` of theirs (the data
/// predicate alone where the other's is `true`) and its constraints both
/// lists, in declared order.
#[trace("TC-685", "FR-230-AC-50", "FR-230-AC-54", "FSpec-FR-322-AC-261")]
#[test]
fn two_guarded_preconditions_are_one_guard_of_their_conjunction() {
    let (unit, wire) = emitted(
        &document(false),
        "pre A using v on Config::ConfigVersion::attemptUpdate { \
         self.x >= 1 ms or self.x > 7 ms }\n\
         pre B using v on Config::ConfigVersion::attemptUpdate { \
         self.versionNumber > 0 and self.x < 5 ms }\n",
    );
    let [guard] = timed(&wire, "clock_guard")[..] else {
        panic!("one guard");
    };
    assert!(located(&wire, guard, &unit).starts_with("pre A"));
    let parts = member(&guard["body"], "parts")["members"]
        .as_array()
        .expect("parts");
    let read: Vec<(String, Vec<String>)> = parts
        .iter()
        .map(|part| {
            let members = part["members"].as_array().expect("a part");
            let predicate = target(&wire, &members[0]["value"]);
            (
                located(&wire, predicate, &unit).to_owned(),
                members[1..]
                    .iter()
                    .map(|reference| located(&wire, target(&wire, reference), &unit).to_owned())
                    .collect(),
            )
        })
        .collect();
    let data = "self.versionNumber > 0".to_owned();
    assert_eq!(
        read,
        [
            (
                data.clone(),
                vec!["self.x >= 1 ms".to_owned(), "self.x < 5 ms".to_owned()]
            ),
            (
                data,
                vec!["self.x > 7 ms".to_owned(), "self.x < 5 ms".to_owned()]
            ),
        ]
    );
    let [root] = timed(&wire, "timed_model")[..] else {
        panic!("one root");
    };
    assert_eq!(
        member(&root["body"], "guards")["members"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(timed(&wire, "clock_constraint").len(), 3);
    assert_declared_order(&wire);

    let (unit, wire) = emitted(
        &document(false),
        "pre A using v on Config::ConfigVersion::attemptUpdate { \
         self.versionNumber > 0 and self.x >= 1 ms }\n\
         pre B using v on Config::ConfigVersion::attemptUpdate { \
         self.versionNumber < 9 and self.x < 5 ms }\n",
    );
    let [guard] = timed(&wire, "clock_guard")[..] else {
        panic!("one guard");
    };
    let [part] = member(&guard["body"], "parts")["members"]
        .as_array()
        .expect("parts")
        .as_slice()
    else {
        panic!("one part");
    };
    let predicate = target(&wire, &part["members"][0]["value"]);
    assert_eq!(
        predicate["body"]["operation"]["identity"],
        "filament.op.boolean.and"
    );
    let operands: Vec<&str> = predicate["body"]["arguments"]
        .as_array()
        .expect("arguments")
        .iter()
        .map(|argument| located(&wire, target(&wire, argument), &unit))
        .collect();
    assert_eq!(
        operands,
        ["self.versionNumber > 0", "self.versionNumber < 9"]
    );
    assert_eq!(part["members"].as_array().map(Vec::len), Some(3));
    assert_declared_order(&wire);
}

/// [`document`] with the required field `peer: ConfigVersion`, a
/// reference to another object of the type.
fn peer_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&document(false)).expect("JSON");
    let config = &mut document["types"][1];
    let owner = config["identity"].as_str().expect("an identity").to_owned();
    let identity = format!("{owner}/peer");
    let mut peer = config["fields"][0].clone();
    peer["identity"] = json!(identity);
    peer["name"] = json!("peer");
    peer["typeRef"] = json!(owner);
    peer["presence"] = json!("required");
    peer["multiplicity"] = json!({"lower": 1, "upper": 1, "ordered": false, "unique": true});
    peer["origin"]["generated"]["generatorIdentity"] = json!(identity);
    peer["origin"]["generated"]["inputIdentities"] = json!([identity]);
    config["fields"].as_array_mut().expect("fields").push(peer);
    crate::emit::tests::wire_bytes(&document).expect("JSON")
}

/// FR-230-AC-10, FSpec FR-322 "Clause nodes" and "Presence": a constraint
/// on another object's clock, `peer_clock(self) <= 2 ms` with `peer_clock`
/// reading `o.peer.x`, is a `clock_constraint`
/// of kind `place` on the template of `x` whose owner term is the
/// expression `self.peer`: the `record.project` of `peer` over the
/// `filament.op.model.deref` of `self`, as S3 lowers an attribute read.
#[trace("TC-685", "FR-230-AC-10", "FR-230-AC-54", "FSpec-FR-322-AC-261")]
#[test]
fn a_constraint_on_another_objects_clock_names_its_owner_term() {
    let (unit, wire) = emitted(
        &peer_document(),
        "function peer_clock using v(o: Config::ConfigVersion): Clock pure { o.peer.x }\n\
         pre Near using v on Config::ConfigVersion::attemptUpdate { peer_clock(self) <= 2 ms }\n",
    );
    let [place] = timed(&wire, "clock_place")[..] else {
        panic!("one place");
    };
    let [constraint] = timed(&wire, "clock_constraint")[..] else {
        panic!("one constraint");
    };
    assert_eq!(literal(&constraint["body"], "kind"), "place");
    assert_eq!(
        member(&constraint["body"], "place")["target"],
        place["node_id"]
    );
    assert_eq!(
        located(&wire, constraint, &unit),
        "peer_clock(self) <= 2 ms"
    );
    let owner = target(&wire, member(&constraint["body"], "owner"));
    assert_eq!(owner["node_tag"], "expression");
    assert_eq!(
        owner["body"]["operation"]["identity"],
        "filament.op.record.project"
    );
    assert_eq!(owner["body"]["operation"]["member"]["name"], "peer");
    let [dereferenced] = owner["body"]["arguments"]
        .as_array()
        .expect("arguments")
        .as_slice()
    else {
        panic!("one argument");
    };
    let dereferenced = target(&wire, dereferenced);
    assert_eq!(
        dereferenced["body"]["operation"]["identity"],
        "filament.op.model.deref"
    );
    assert_self(&wire, &dereferenced["body"]["arguments"][0]);
    assert_declared_order(&wire);
}

/// [`peer_document`] with `v: Option<Clock>`.
fn presence_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&peer_document()).expect("JSON");
    let config = &mut document["types"][1];
    let owner = config["identity"].as_str().expect("an identity").to_owned();
    let fields = config["fields"].as_array_mut().expect("fields");
    let mut v = fields
        .iter()
        .find(|field| field["name"] == "x")
        .expect("x")
        .clone();
    let identity = format!("{owner}/v");
    v["identity"] = json!(identity);
    v["name"] = json!("v");
    v["presence"] = json!("optional");
    v["origin"]["generated"]["generatorIdentity"] = json!(identity);
    v["origin"]["generated"]["inputIdentities"] = json!([identity]);
    fields.push(v);
    crate::emit::tests::wire_bytes(&document).expect("JSON")
}

/// `present(p)` of the place `name`: a `presence_read` application of
/// `filament.op.option.present`, typed Boolean, over the place's template and
/// the owner term; returns the owner term's node.
fn assert_presence<'w>(wire: &'w Value, read: &Value, place: &Value) -> &'w Value {
    assert_eq!(
        (&read["node_tag"], &read["semantic_form"]),
        (&json!("expression"), &json!("presence_read"))
    );
    assert_eq!(
        read["body"]["operation"]["identity"],
        "filament.op.option.present"
    );
    assert_eq!(read["body"]["operator"], "present");
    assert_eq!(
        target(wire, &json!({"target": read["body"]["result_type"]}))["semantic_form"],
        "boolean"
    );
    let [template, owner, ..] = read["body"]["arguments"]
        .as_array()
        .expect("arguments")
        .as_slice()
    else {
        panic!("a place and an owner term: {read}");
    };
    assert_eq!(template["target"], place["node_id"]);
    target(wire, owner)
}

/// The `clock_place` of `wire` whose path is the one field `name`.
fn place_of<'w>(wire: &'w Value, name: &str) -> &'w Value {
    timed(wire, "clock_place")
        .into_iter()
        .find(|place| {
            member(&place["body"], "path")["members"][0]["members"][1]["value"]["value"] == name
        })
        .unwrap_or_else(|| panic!("the place {name}"))
}

/// FSpec FR-322 "Presence" (FR-322-AC-155), FR-230 "Presence is derived":
/// `present(self.v)` over `v: Option<Clock>` is a `presence_read` of
/// `filament.op.option.present` over `v`'s `clock_place` template and the
/// owner `self`, in a guard part beside the part's constraint on `v`, as a
/// time invariant's whole predicate and conjoined with an `urgent when`'s
/// data predicate; `present(self.peer.v)` takes the owner term `self.peer`.
/// Each place is S3's template, keyed as IR keys it, and no place or model
/// node carries a presence member.
#[trace("TC-685", "FR-230-AC-50", "FR-230-AC-53", "FR-230-AC-54")]
#[trace("FSpec-FR-322-AC-155", "FSpec-FR-322-AC-261")]
#[test]
fn a_presence_reads_the_place_template_at_its_owner() {
    let (unit, wire) = emitted(
        &presence_document(),
        "pre Armed using v on Config::ConfigVersion::attemptUpdate { \
         present(self.v) and value(self.v) >= 1 ms }\n\
         time invariant Held when { present(self.v) } { value(self.v) <= 9 ms }\n\
         urgent Due when { present(self.v) and self.versionNumber > 0 };\n\
         urgent Peer when { present(self.peer.v) };\n",
    );
    let places = timed(&wire, "clock_place");
    assert_eq!(places.len(), 2);
    let graph = fsl_semantics::check::wire_id::WireGraph::of_package_value(&wire).unwrap();
    for place in &places {
        // The held timed graph is the emitter's, in structural ids.
        let id = crate::emit::checked_node_id_of(&graph.key_of_value(&place["node_id"]).unwrap());
        let node = admitted_graph(&wire)
            .0
            .into_iter()
            .find(|node| node.node_id == id)
            .expect("a node");
        assert_eq!(
            crate::emit::timed_model_node_id(&StructuralIds::default(), &node).expect("a key"),
            node.node_id,
            "S3's place key is the key IR derives"
        );
        assert_eq!(names(&place["body"]), ["model", "path", "resolved_path"]);
    }
    let v = place_of(&wire, "v");

    let [guard] = timed(&wire, "clock_guard")[..] else {
        panic!("one guard");
    };
    let part = &member(&guard["body"], "parts")["members"][0]["members"];
    let read = target(&wire, &part[0]["value"]);
    assert_self(
        &wire,
        &json!({"target": assert_presence(&wire, read, v)["node_id"]}),
    );
    let constraint = target(&wire, &part[1]);
    assert_eq!(member(&constraint["body"], "place")["target"], v["node_id"]);
    assert_eq!(literal(&constraint["body"], "comparison"), "ge");

    let [invariant] = timed(&wire, "time_invariant")[..] else {
        panic!("one invariant");
    };
    assert_eq!(
        member(&invariant["body"], "predicate")["target"],
        read["node_id"],
        "one presence node"
    );

    let urgencies = timed(&wire, "urgency");
    let predicate = |name: &str| {
        let urgency = urgencies
            .iter()
            .find(|urgency| located(&wire, urgency, &unit).starts_with(name))
            .unwrap_or_else(|| panic!("urgency {name}"));
        target(&wire, member(&urgency["body"], "predicate"))
    };
    let due = predicate("urgent Due");
    assert_eq!(
        due["body"]["operation"]["identity"],
        "filament.op.boolean.and"
    );
    assert_eq!(due["body"]["arguments"][0]["target"], read["node_id"]);
    assert_eq!(
        located(&wire, target(&wire, &due["body"]["arguments"][1]), &unit),
        "self.versionNumber > 0"
    );
    let owner = assert_presence(&wire, predicate("urgent Peer"), v);
    assert_eq!(
        owner["body"]["operation"]["identity"],
        "filament.op.record.project"
    );
    assert_eq!(owner["body"]["operation"]["member"]["name"], "peer");

    for node in nodes(&wire) {
        if node["node_tag"] == "timed_model" || node["semantic_form"] == "model" {
            assert!(
                !node.to_string().contains("\"presence\""),
                "no node carries a presence member: {node}"
            );
        }
    }
    assert_declared_order(&wire);
}

/// The member a refused model names, and the text of its span in `unit`.
fn unwritten(document: &[u8], body: &str) -> (fsl_semantics::check::UnwrittenTimedMember, String) {
    let (unit, checked) = checked(document, body);
    let refusals = checked.expect_err("the member is not written yet");
    let found = refusals.iter().find_map(|refusal| match &refusal.cause {
        fsl_semantics::check::CheckCause::Temporal(cause) => match &**cause {
            fsl_semantics::check::TemporalCause::TimedModelNotEmitted { member, span, .. } => {
                Some((*member, unit[span.start..span.end].to_owned()))
            }
            _ => None,
        },
        _ => None,
    });
    found.unwrap_or_else(|| panic!("{body}: {refusals:?}"))
}

/// FR-230-AC-55, FSpec FR-322-AC-295 and FR-415-AC-24: bounded `forall`
/// guards encode each live length as a conjunction with one slot binder per
/// constraint, while `exists` encodes selected slots as disjunctions and has
/// no matching branch for an empty collection.
#[trace("TC-685", "FR-230-AC-55", "FSpec-FR-322-AC-295", "FSpec-FR-415-AC-24")]
#[test]
fn quantified_clock_guards_emit_ordered_slot_binders() {
    use fsl_foundation::node_wire::node_key_from_json;
    use fsl_semantics::check::builtin_integer_type_key;

    for (quantifier, empty_part) in [("forall", true), ("exists", false)] {
        let query = format!("{quantifier}(e in self.ts: e.x < 5 ms)");
        let (unit, wire) = emitted(
            &timer_document(),
            &format!("pre Timed using v on Config::ConfigVersion::attemptUpdate {{ {query} }}\n"),
        );
        let [guard] = timed(&wire, "clock_guard")[..] else {
            panic!("one guard");
        };
        let parts = member(&guard["body"], "parts")["members"]
            .as_array()
            .expect("guard parts");
        assert!(!parts.is_empty(), "the collection sizes have branches");
        let mut saw_empty = false;
        let integer = builtin_integer_type_key("pkg").unwrap();

        for part in parts {
            let members = part["members"].as_array().expect("part members");
            let binder_member = members.iter().find(|member| member["name"] == "binders");
            let binders = binder_member
                .and_then(|member| member["value"]["members"].as_array())
                .cloned()
                .unwrap_or_default();
            let constraints: Vec<&Value> = members
                .iter()
                .filter(|member| {
                    member["term"] == "reference"
                        && target(&wire, member)["semantic_form"] == "clock_constraint"
                })
                .map(|reference| target(&wire, reference))
                .collect();

            assert_eq!(constraints.len(), binders.len());
            if !binders.is_empty() {
                let binder_index = members
                    .iter()
                    .position(|member| member["name"] == "binders")
                    .expect("binder member is in the aggregate");
                let first_constraint = members
                    .iter()
                    .position(|member| {
                        member["term"] == "reference"
                            && target(&wire, member)["semantic_form"] == "clock_constraint"
                    })
                    .expect("a binder-bearing part has a constraint");
                assert!(
                    binder_index < first_constraint,
                    "binders precede constraints"
                );
            }
            if constraints.is_empty() {
                saw_empty = true;
                assert_eq!(quantifier, "forall", "exists has no empty branch");
                let predicate_reference = members
                    .iter()
                    .find(|member| member["name"] == "predicate")
                    .expect("the vacuous branch retains its size predicate");
                let predicate = target(&wire, &predicate_reference["value"]);
                assert_eq!(
                    predicate["body"]["operation"]["identity"],
                    "filament.op.integer.eq"
                );
                let [size, zero] = predicate["body"]["arguments"]
                    .as_array()
                    .expect("size-zero equality")
                    .as_slice()
                else {
                    panic!("size-zero equality has two operands: {predicate}");
                };
                let size = target(&wire, size);
                assert_eq!(
                    size["body"]["operation"]["identity"],
                    "filament.op.collection.size"
                );
                let [collection] = size["body"]["arguments"]
                    .as_array()
                    .expect("collection size")
                    .as_slice()
                else {
                    panic!("collection size has one operand: {size}");
                };
                let collection = target(&wire, collection);
                assert_eq!(
                    collection["body"]["operation"]["identity"],
                    "filament.op.record.project"
                );
                assert_eq!(collection["body"]["operation"]["member"]["name"], "ts");
                assert_eq!(zero["term"], "literal");
                assert_eq!(zero["value"], "0");
            }
            for (binder_reference, constraint) in binders.iter().zip(constraints) {
                let binder = target(&wire, binder_reference);
                assert_eq!(binder["semantic_form"], "parameter");
                assert_eq!(
                    node_key_from_json(&binder["semantic_type"]).unwrap(),
                    integer
                );
                let slots = &member(&constraint["body"], "slots")["members"];
                assert_eq!(slots, &json!([binder_reference]));
            }
        }
        assert_eq!(
            saw_empty, empty_part,
            "{quantifier} empty-collection branch semantics"
        );
        assert_declared_order(&wire);
        assert!(unit.contains(&query));
    }
}

/// FR-230-AC-56, FR-393-AC-9 and FSpec FR-322-AC-295: a universal quantified
/// invariant emits one family slot binder and an upper-bound constraint
/// using that same binder. The family term bounds only live slots, so the
/// invariant has no constraint to violate when the collection is empty.
#[trace(
    "TC-685",
    "FR-230-AC-56",
    "FR-393-AC-9",
    "FSpec-FR-322-AC-295",
    "FSpec-FR-415-AC-24"
)]
#[test]
fn quantified_time_invariant_emits_binder_and_live_slot_bound() {
    let (unit, wire) = emitted(
        &timer_document(),
        "time invariant All when { true } { forall(e in self.ts: e.x <= 9 ms) }\n",
    );
    let [invariant] = timed(&wire, "time_invariant")[..] else {
        panic!("one time invariant");
    };
    assert_eq!(
        names(&invariant["body"]),
        ["predicate", "binders", "bounds"]
    );
    let predicate = target(&wire, member(&invariant["body"], "predicate"));
    assert_true(&wire, predicate);
    let [binder_reference] = member(&invariant["body"], "binders")["members"]
        .as_array()
        .expect("invariant binders")
        .as_slice()
    else {
        panic!("one slot binder");
    };
    let binder = target(&wire, binder_reference);
    assert_eq!(binder["semantic_form"], "parameter");
    assert_eq!(literal(&binder["body"], "name"), "e");
    let [bound_reference] = member(&invariant["body"], "bounds")["members"]
        .as_array()
        .expect("invariant bounds")
        .as_slice()
    else {
        panic!("one upper bound");
    };
    let bound = target(&wire, bound_reference);
    assert_eq!(literal(&bound["body"], "comparison"), "le");
    assert_eq!(literal(&bound["body"], "bound"), "9/1");
    let place = target(&wire, member(&bound["body"], "place"));
    let path = member(&place["body"], "path")["members"]
        .as_array()
        .expect("template clock path");
    assert_eq!(
        path.iter()
            .map(|step| literal(step, "kind"))
            .collect::<Vec<_>>(),
        ["field", "slot", "field"]
    );
    assert_eq!(literal(&path[0], "name"), "ts");
    assert_eq!(literal(&path[2], "name"), "x");
    assert_eq!(
        member(&bound["body"], "slots")["members"],
        json!([binder_reference]),
        "the constraint slot is the invariant binder"
    );
    assert!(unit.contains("forall(e in self.ts: e.x <= 9 ms)"));
    assert_declared_order(&wire);
}

/// FR-230-AC-56: existential quantified invariant bounds and quantified
/// element-presence invariants refuse as a whole at the owning invariant.
#[trace("TC-685", "FR-230-AC-56", "FR-230-AC-50")]
#[test]
fn unsupported_quantified_invariant_forms_refuse_without_partial_output() {
    use fsl_foundation::diagnostic::Code;
    use fsl_semantics::check::{CheckCause, TemporalCause};

    for (document, source, expected) in [
        (
            timer_document(),
            "time invariant Some when { true } { exists(e in self.ts: e.x <= 9 ms) }",
            "time invariant Some",
        ),
        (
            slot_presence_document(),
            "time invariant HasElement when { forall(e in self.q: present(e)) } { self.x <= 9 ms }",
            "time invariant HasElement",
        ),
    ] {
        let (unit, result) = checked(&document, &format!("{source}\n"));
        let refusals = result.expect_err("the unsupported invariant is refused");
        let Some(refusal) = refusals.iter().find(|refusal| {
            matches!(
                &refusal.cause,
                CheckCause::Temporal(cause)
                    if matches!(&**cause, TemporalCause::TimeInvariantShape { .. }
                        | TemporalCause::TimedModelNotEmitted { .. })
            )
        }) else {
            panic!("the invariant refusal is reported: {refusals:?}");
        };
        let expected_code = if source.contains("exists(") {
            Code::InvalidTimedModel
        } else {
            Code::UnsupportedConstruct
        };
        assert_eq!(refusal.cause.code(), expected_code, "{source}");
        let span = match &refusal.cause {
            CheckCause::Temporal(cause) => match &**cause {
                TemporalCause::TimeInvariantShape { span }
                | TemporalCause::TimedModelNotEmitted { span, .. } => *span,
                other => panic!("invariant refusal: {other:?}"),
            },
            other => panic!("temporal refusal: {other:?}"),
        };
        assert!(
            unit[span.start..span.end].starts_with(expected),
            "{source}: refusal span {:?} is {:?}, expected prefix {expected:?}",
            span,
            &unit[span.start..span.end]
        );
    }
}

/// FR-230-AC-55, FR-230-AC-56, FR-230-AC-53: element-presence reads remain
/// unavailable in timed guards, invariants and urgency predicates until their
/// quantified wire form is admitted. Each model refuses as a whole instead of
/// emitting a guard with the presence requirement missing.
#[trace("TC-685", "FR-230-AC-53", "FR-230-AC-55", "FR-230-AC-56")]
#[test]
fn element_presence_in_timed_predicates_refuses_without_emitting_partial_nodes() {
    use fsl_semantics::check::UnwrittenTimedMember as Member;

    let quantifier = "forall(e in self.q: present(e))";
    let guard = format!(
        "pre HasElement using v on Config::ConfigVersion::attemptUpdate {{ \
         self.x >= 1 ms and {quantifier} }}"
    );
    let (member, text) = unwritten(&slot_presence_document(), &format!("{guard}\n"));
    assert_eq!(
        (member, text.as_str()),
        (Member::ElementPresence, guard.as_str()),
        "FR-230-AC-55: a quantified guard that reads element presence is refused at \
         the owning guard as ElementPresence"
    );

    let invariant =
        format!("time invariant HasElement when {{ {quantifier} }} {{ self.x <= 9 ms }}");
    let (member, text) = unwritten(&slot_presence_document(), &format!("{invariant}\n"));
    assert_eq!(member, Member::ElementPresence);
    assert!(text.starts_with("time invariant HasElement"), "{text}");

    let urgency = format!("urgent HasElement when {{ {quantifier} }};");
    let (member, text) = unwritten(&slot_presence_document(), &format!("{urgency}\n"));
    assert_eq!(member, Member::ElementPresence);
    assert!(text.starts_with("urgent HasElement"), "{text}");
}

/// FR-230-AC-57 / TC-685 step 38: a mixed quantified and ordinary bound
/// list refuses at the invariant before it can emit a partially scoped node.
/// The fixture's `ts` range includes zero, where one global binder would
/// otherwise make the ordinary bound vacuous.
#[trace("TC-685", "FR-230-AC-57")]
#[test]
fn a_mixed_quantified_invariant_bound_refuses_before_partial_emission() {
    use fsl_foundation::diagnostic::Code;
    use fsl_semantics::check::{CheckCause, TemporalCause};

    let source = "time invariant Mixed when { true } { forall(e in self.ts: e.x <= 9 ms) and self.x < 12 ms }";
    let (unit, result) = checked(&timer_document(), &format!("{source}\n"));
    let refusals = result.expect_err("mixed bound scopes cannot be represented");
    assert_eq!(refusals.len(), 1, "no partial timed model is produced");
    let refusal = &refusals[0];
    assert_eq!(refusal.cause.code(), Code::InvalidTimedModel);
    let CheckCause::Temporal(cause) = &refusal.cause else {
        panic!("temporal refusal: {:?}", refusal.cause);
    };
    let TemporalCause::TimeInvariantShape { span } = &**cause else {
        panic!("time-invariant-shape refusal: {cause:?}");
    };
    assert!(unit[span.start..span.end].starts_with(source));
}

/// FR-230-AC-50, FSpec FR-342-AC-11: a clock-reading clause bound to an
/// execution point refuses `not-yet-implemented` at the clause and is never
/// written with a point-qualified anchor.
#[trace("TC-685", "FR-230-AC-50", "FSpec-FR-342-AC-11")]
#[test]
fn a_point_qualified_clock_clause_refuses_at_the_clause() {
    use filament_contract_model::{AnchorName, ExecutionPoint};
    let document = document(false);
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = clocked_unit(
        "pre Late using v on Config::ConfigVersion::attemptUpdate { self.x >= 1 ms }\n",
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:spine-model"),
        "program.native",
        unit.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    let raw = parsed.source().reference().clone();
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = fsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("intake admits the package");
    let point = ExecutionPoint::Pre {
        operation: AnchorName::new("attemptUpdate").unwrap().into(),
    };
    let refusals = PackageDeclarations::assemble_with_execution_points_and_cancel(
        raw,
        "pkg".to_owned(),
        built,
        models,
        Vec::new(),
        Default::default(),
        fsl_semantics::check::AssemblyRun {
            cancel: &filament_exact::Cancel::new(),
            execution_points: BTreeMap::from([("Late".to_owned(), point)]),
        },
    )
    .expect("the unit assembles")
    .check(CheckMode::Linked, CheckingLimits::default())
    .expect_err("a point-qualified clock clause is not written yet");
    let found = refusals.iter().find_map(|refusal| match &refusal.cause {
        fsl_semantics::check::CheckCause::Temporal(cause) => match &**cause {
            fsl_semantics::check::TemporalCause::TimedModelNotEmitted { member, span, .. } => {
                Some((*member, &unit[span.start..span.end]))
            }
            _ => None,
        },
        _ => None,
    });
    let Some((member, text)) = found else {
        panic!("{refusals:?}");
    };
    assert_eq!(
        member,
        fsl_semantics::check::UnwrittenTimedMember::PointQualifiedClause
    );
    assert!(member.description().contains("execution point"));
    assert!(text.starts_with("pre Late"), "{text}");
}

/// [`vector_document`] whose `attemptUpdate` frame names `ts`.
fn timer_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&vector_document()).expect("JSON");
    document["types"][1]["operations"][0]["frame"]["modifies"]
        .as_array_mut()
        .expect("modifies")
        .push(json!("ix://example/config-version/ConfigVersion/ts"));
    crate::emit::tests::wire_bytes(&document).expect("JSON")
}

/// FR-230 "Resets by selection", FSpec FR-322 "Clause nodes"
/// (FR-322-AC-261, FR-322-AC-264): over `ts: Sequence<Timer>[0, 3]`,
/// `forall(e in self.ts: e.x = 0 ms)` is one `clock_reset` of the place
/// `ts[slot].x` whose one slot term and one binder are the `parameter` node
/// `e` of the selector domain `Integer`, under the condition `true`. A
/// selection that reads the element's data refuses `not-yet-implemented`
/// at the reset.
#[trace("TC-685", "FR-230-AC-54", "FSpec-FR-322-AC-261", "FSpec-FR-322-AC-264")]
#[test]
fn a_reset_by_selection_binds_its_slot() {
    let (unit, wire) = emitted(
        &timer_document(),
        "post R using v on Config::ConfigVersion::attemptUpdate { \
         forall(e in self.ts: e.x = 0 ms) }\n",
    );
    let [reset] = timed(&wire, "clock_reset")[..] else {
        panic!("one reset");
    };
    let place = target(&wire, member(&reset["body"], "place"));
    let steps: Vec<String> = member(&place["body"], "path")["members"]
        .as_array()
        .expect("a path")
        .iter()
        .map(|step| literal(step, "kind"))
        .collect();
    assert_eq!(steps, ["field", "slot", "field"]);
    let slots = &member(&reset["body"], "slots")["members"];
    let binders = &member(&reset["body"], "binders")["members"];
    assert_eq!(slots, binders, "the binder is the slot term");
    let [binder] = binders.as_array().expect("binders").as_slice() else {
        panic!("one binder");
    };
    let binder = target(&wire, binder);
    assert_eq!(binder["semantic_form"], "parameter");
    assert_eq!(literal(&binder["body"], "name"), "e");
    let integer = fsl_semantics::check::builtin_integer_type_key("pkg").unwrap();
    assert_eq!(
        fsl_foundation::node_wire::node_key_from_json(&binder["semantic_type"]).unwrap(),
        integer
    );
    assert_true(&wire, target(&wire, member(&reset["body"], "condition")));
    assert_eq!(
        located(&wire, reset, &unit),
        "forall(e in self.ts: e.x = 0 ms)"
    );
    assert_declared_order(&wire);
}

/// The excluded form a refused model names, and the text of its span in
/// `unit`.
fn excluded(document: &[u8], body: &str) -> (fsl_semantics::check::ExcludedClauseForm, String) {
    let (unit, checked) = checked(document, body);
    let refusals = checked.expect_err("the form is excluded");
    let found = refusals.iter().find_map(|refusal| match &refusal.cause {
        fsl_semantics::check::CheckCause::Temporal(cause) => match &**cause {
            fsl_semantics::check::TemporalCause::ExcludedClauseForm { form, span, .. } => {
                assert_eq!(
                    (cause.code(), cause.tag()),
                    (Code::UnsupportedConstruct, "expression-form")
                );
                Some((*form, unit[span.start..span.end].to_owned()))
            }
            _ => None,
        },
        _ => None,
    });
    found.unwrap_or_else(|| panic!("{body}: {refusals:?}"))
}

/// FSpec FR-322-AC-267 `CW-06`: an indexed element write `self.ts[0] =
/// Config::Timer { x: 0 ms, n: 1 }` has no package form. S2 builds `self.ts[0]`
/// as a selection (FR-230 "Selection"), and the checker refuses the write
/// `unsupported_construct`, so no package is emitted.
#[trace("TC-685", "FSpec-FR-322-AC-267")]
#[test]
fn an_indexed_element_write_is_refused() {
    let (_, checked) = checked(
        &dropping_document(),
        "post P using v on Config::ConfigVersion::attemptUpdate { \
         self.ts[0] = Config::Timer { x: 0 ms, n: 1 } }\n",
    );
    let refusals = checked.expect_err("the write has no package form");
    assert!(
        refusals.iter().any(|refusal| refusal
            .cause
            .catalog_code()
            .is_some_and(|code| code.to_string().starts_with("unsupported_construct/"))),
        "{refusals:?}"
    );
}

/// [`dropping_document`] with `q: Sequence<Option<Clock>>[0, 2]`, which
/// `attemptUpdate`'s frame names too.
fn slot_presence_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&dropping_document()).expect("JSON");
    let config = &mut document["types"][1];
    let owner = config["identity"].as_str().expect("an identity").to_owned();
    let fields = config["fields"].as_array_mut().expect("fields");
    let mut q = fields
        .iter()
        .find(|field| field["name"] == "v")
        .expect("v")
        .clone();
    let identity = format!("{owner}/q");
    q["identity"] = json!(identity);
    q["name"] = json!("q");
    q["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
    q["origin"]["generated"]["generatorIdentity"] = json!(identity);
    q["origin"]["generated"]["inputIdentities"] = json!([identity]);
    fields.push(q);
    config["operations"][0]["frame"]["modifies"]
        .as_array_mut()
        .expect("modifies")
        .push(json!(identity));
    crate::emit::tests::wire_bytes(&document).expect("JSON")
}

/// FSpec FR-322-AC-267 `SD-02`: `forall(e in self.q: present(e) implies e
/// = 0 ms)` over `q: Sequence<Option<Clock>>[0, 2]` is one `clock_reset`
/// of `q`'s element place whose one binder `e` is its slot term and whose
/// condition is the `presence_read` of that place at `self` and `e`.
#[trace("TC-685", "FR-230-AC-54", "FSpec-FR-322-AC-267", "FSpec-FR-322-AC-155")]
#[test]
fn a_reset_selected_by_presence_conditions_on_its_presence_read() {
    let (unit, wire) = emitted(
        &slot_presence_document(),
        "post R using v on Config::ConfigVersion::attemptUpdate { \
         forall(e in self.q: present(e) implies e = 0 ms) }\n",
    );
    let [reset] = timed(&wire, "clock_reset")[..] else {
        panic!("one reset");
    };
    let place = target(&wire, member(&reset["body"], "place"));
    assert_eq!(place, place_of(&wire, "q"));
    let binders = &member(&reset["body"], "binders")["members"];
    assert_eq!(binders, &member(&reset["body"], "slots")["members"]);
    let [binder] = binders.as_array().expect("binders").as_slice() else {
        panic!("one binder");
    };
    let condition = target(&wire, member(&reset["body"], "condition"));
    assert_self(
        &wire,
        &json!({"target": assert_presence(&wire, condition, place)["node_id"]}),
    );
    assert_eq!(condition["body"]["arguments"][2], *binder);
    assert_eq!(
        located(&wire, reset, &unit),
        "forall(e in self.q: present(e) implies e = 0 ms)"
    );
    assert_declared_order(&wire);
}

/// FSpec FR-322-AC-267 `CW-05`: `post present(self.v)` is a postcondition
/// predicate, a `presence_read` of `v`'s place at `self`, and not a write:
/// the package carries no clause node for it.
#[trace("TC-685", "FR-230-AC-50", "FSpec-FR-322-AC-267")]
#[test]
fn a_postcondition_presence_requirement_is_a_presence_read_with_no_clause_node() {
    let (_, wire) = emitted(
        &dropping_document(),
        "post P using v on Config::ConfigVersion::attemptUpdate { present(self.v) }\n",
    );
    for form in [
        "clock_reset",
        "clock_guard",
        "clock_constraint",
        "time_invariant",
    ] {
        assert!(timed(&wire, form).is_empty(), "no {form}");
    }
    let v = place_of(&wire, "v");
    let (_, without) = emitted(&dropping_document(), "");
    // Two packages: their reads compare structurally (FR-322).
    let reads = |wire: &Value| -> Vec<(Value, Value)> {
        let graph = fsl_semantics::check::wire_id::WireGraph::of_package_value(wire).unwrap();
        nodes(wire)
            .iter()
            .filter(|node| node["semantic_form"] == "presence_read")
            .map(|node| (graph.structural_value(node).unwrap(), node.clone()))
            .collect()
    };
    let before: Vec<Value> = reads(&without).into_iter().map(|(key, _)| key).collect();
    let added: Vec<Value> = reads(&wire)
        .into_iter()
        .filter(|(key, _)| !before.contains(key))
        .map(|(_, read)| read)
        .collect();
    let [read] = &added[..] else {
        panic!("one presence read: {added:?}");
    };
    assert_self(
        &wire,
        &json!({"target": assert_presence(&wire, read, v)["node_id"]}),
    );
    assert_declared_order(&wire);
}

/// [`presence_document`] whose `attemptUpdate` frame names `v`.
fn dropping_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&presence_document()).expect("JSON");
    document["types"][1]["operations"][0]["frame"]["modifies"]
        .as_array_mut()
        .expect("modifies")
        .push(json!("ix://example/config-version/ConfigVersion/v"));
    crate::emit::tests::wire_bytes(&document).expect("JSON")
}

/// FSpec FR-322-AC-267 (STD-560, `excluded_forms`): the forms the checked
/// package excludes refuse `unsupported_construct`/`expression-form` at the
/// form, naming the construct, and no package is emitted: a guard disjunct
/// that reads no clock at that disjunct (`GJ-01`), a clock collection
/// written by a literal or a `filter` at the write (`CW-01`, `CW-02`), a
/// drop `self.v = absent` at the write (`CW-03`) and a reset by selection
/// that reads the element's data at the condition (`SD-01`). The controls
/// build: `self.x >= 0 ms or self.versionNumber > 0` is the one part of
/// `x >= 0 ms`, which every clock meets (`GJ-03`), and a constant write of
/// `v` is its reset (`CW-04`).
#[trace("TC-685", "FR-230-AC-50", "FSpec-FR-322-AC-267")]
#[test]
fn excluded_clause_forms_refuse_at_the_form() {
    use fsl_semantics::check::ExcludedClauseForm as Form;
    let attempt = |body: &str| {
        format!("post P using v on Config::ConfigVersion::attemptUpdate {{ {body} }}\n")
    };
    for (document, body, form, text) in [
        (
            document(false),
            "pre Or using v on Config::ConfigVersion::attemptUpdate { \
             self.x >= 1 ms or self.versionNumber > 0 }\n"
                .to_owned(),
            Form::DataOnlyDisjunct,
            "self.versionNumber > 0",
        ),
        (
            timer_document(),
            attempt("self.ts = sequence[Config::Timer { x: 0 ms, n: 1 }]"),
            Form::CollectionWrite,
            "self.ts = sequence[Config::Timer { x: 0 ms, n: 1 }]",
        ),
        (
            timer_document(),
            attempt("self.ts = filter(e in pre(self.ts): e.n > 1)"),
            Form::CollectionWrite,
            "self.ts = filter(e in pre(self.ts): e.n > 1)",
        ),
        (
            dropping_document(),
            attempt("self.v = absent"),
            Form::PresenceDrop,
            "self.v = absent",
        ),
        (
            timer_document(),
            attempt("forall(e in self.ts: e.n > 1 implies e.x = 0 ms)"),
            Form::ElementDataSelection,
            "e.n > 1",
        ),
    ] {
        let (found, located) = excluded(&document, &body);
        assert_eq!((found, located.as_str()), (form, text), "{body}");
        assert!(!found.description().is_empty());
    }
    let (_, wire) = emitted(
        &document(false),
        "pre Zero using v on Config::ConfigVersion::attemptUpdate { \
         self.x >= 0 ms or self.versionNumber > 0 }\n",
    );
    let [guard] = timed(&wire, "clock_guard")[..] else {
        panic!("one guard");
    };
    let parts = member(&guard["body"], "parts")["members"]
        .as_array()
        .expect("parts");
    assert_eq!(parts.len(), 1, "the disjunct x >= 0 ms alone");
    let (_, wire) = emitted(&dropping_document(), &attempt("self.v = 0 ms"));
    assert_eq!(timed(&wire, "clock_reset").len(), 1);
}

/// The body of every `presence_read` application in `wire`.
fn presence_read_bodies(wire: &Value) -> Vec<Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == "filament.op.option.present")
        .map(|node| node["body"].clone())
        .collect()
}

/// FR-230-AC-66: a presence requirement compared with `absent` writes the
/// `presence_read` of its `present` form, and a timed predicate reading the
/// presences of a clock collection's elements writes a `presence_read` at
/// the binder.
#[trace("TC-2390", "FR-230-AC-66")]
#[test]
#[ignore = "QSL-2144: element presences in a guard, time invariant or urgency predicate have no specified binder scope beside a non-slot constraint (spec gap)"]
fn compared_and_element_presences_are_written() {
    let post = |body: &str| {
        emitted(
            &dropping_document(),
            &format!("post P using v on Config::ConfigVersion::attemptUpdate {{ {body} }}\n"),
        )
        .1
    };
    let compared = presence_read_bodies(&post("self.v != absent"));
    assert!(!compared.is_empty(), "a presence_read is written");
    assert_eq!(compared, presence_read_bodies(&post("present(self.v)")));
    let quantifier = "forall(e in self.q: present(e))";
    for member in [
        format!(
            "pre HasElement using v on Config::ConfigVersion::attemptUpdate {{ \
             self.x >= 1 ms and {quantifier} }}"
        ),
        format!("time invariant HasElement when {{ {quantifier} }} {{ self.x <= 9 ms }}"),
        format!("urgent HasElement when {{ {quantifier} }};"),
    ] {
        let (_, wire) = emitted(&slot_presence_document(), &format!("{member}\n"));
        assert!(!presence_read_bodies(&wire).is_empty(), "{member}");
    }
}

/// FR-230-AC-66: a presence requirement compared with `absent` writes the
/// `presence_read` of its `present` form: `self.v != absent` and
/// `not (self.v = absent)` write the body `present(self.v)` writes.
#[trace("TC-2390", "FR-230-AC-66")]
#[test]
fn a_presence_compared_with_absent_writes_its_present_read() {
    let post = |body: &str| {
        emitted(
            &dropping_document(),
            &format!("post P using v on Config::ConfigVersion::attemptUpdate {{ {body} }}\n"),
        )
        .1
    };
    let present = presence_read_bodies(&post("present(self.v)"));
    assert!(!present.is_empty(), "a presence_read is written");
    assert_eq!(presence_read_bodies(&post("self.v != absent")), present);
    assert_eq!(
        presence_read_bodies(&post("not (self.v = absent)")),
        present
    );
}

/// [`timer_document`] whose `Timer` also holds `ys: Sequence<Clock>[0, 2]`.
fn nested_timer_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&timer_document()).expect("JSON");
    let timer = document["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|node| node["identity"] == "ix://example/config-version/Timer")
        .expect("Timer");
    let fields = timer["fields"].as_array_mut().expect("fields");
    let mut ys = fields[0].clone();
    let identity = "ix://example/config-version/Timer/ys";
    ys["identity"] = json!(identity);
    ys["name"] = json!("ys");
    ys["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
    ys["origin"]["generated"]["generatorIdentity"] = json!(identity);
    ys["origin"]["generated"]["inputIdentities"] = json!([identity]);
    fields.push(ys);
    crate::emit::tests::wire_bytes(&document).expect("JSON")
}

/// FR-230-AC-67: a reset of a clock below two collections writes one
/// `clock_reset` with a slot term per collection level.
#[trace("TC-2391", "FR-230-AC-67")]
#[test]
#[ignore = "QSL-2144: an element reset no single binder selects is refused TimedModelNotEmitted, not written"]
fn a_reset_below_two_collections_holds_a_slot_per_level() {
    const RESET: &str =
        "post R using v on Config::ConfigVersion::attemptUpdate { self.ts[0].ys[1] = 0 ms }\n";
    let pre = |condition: &str| {
        format!("pre S using v on Config::ConfigVersion::attemptUpdate {{ {condition} }}\n{RESET}")
    };
    // Each slot keeps its own presence obligation (FR-230 "Selection").
    for (body, at) in [(RESET.to_owned(), "0"), (pre("size(self.ts) >= 1"), "1")] {
        let (unit, checked) = checked(&nested_timer_document(), &body);
        let refusals = checked.expect_err("an unproved slot is refused");
        let [refusal] = refusals.as_slice() else {
            panic!("one refusal: {refusals:?}");
        };
        assert!(
            format!("{refusal:?}").contains("nproved"),
            "unproved-presence: {refusal:?}"
        );
        let fsl_semantics::check::CheckCause::Temporal(cause) = &refusal.cause else {
            panic!("a timed refusal: {refusal:?}");
        };
        let span = cause.span();
        assert_eq!(&unit[span.start..span.end], at, "refused at the slot {at}");
    }
    let (_, wire) = emitted(
        &nested_timer_document(),
        &pre("size(self.ts) >= 1 and size(self.ts[0].ys) >= 2"),
    );
    let [reset] = timed(&wire, "clock_reset")[..] else {
        panic!("one reset");
    };
    let place = target(&wire, member(&reset["body"], "place"));
    let steps: Vec<String> = member(&place["body"], "path")["members"]
        .as_array()
        .expect("a path")
        .iter()
        .map(|step| literal(step, "kind"))
        .collect();
    assert_eq!(steps, ["field", "slot", "field", "slot"]);
    let slots = member(&reset["body"], "slots")["members"]
        .as_array()
        .expect("slots")
        .len();
    assert_eq!(slots, 2);
}

/// FR-230-AC-50, FSpec FR-342-AC-13, FR-322-AC-454: a clock-reading `pre`
/// bound to an execution point emits its guard with `operation` the no-point
/// anchor and `point_anchor` the point-qualified anchor.
#[trace("TC-2393", "FR-230-AC-50")]
#[trace("FSpec-TC-1721", "FSpec-FR-342-AC-13")]
#[trace("FSpec-TC-1722", "FSpec-FR-322-AC-454")]
#[test]
#[ignore = "QSL-2144: blocked on filament-contract-model (filament-contract-ir) admitting the FR-322-AC-454 point_anchor binding on clock_guard and clock_reset"]
fn a_point_bound_clock_clause_names_its_point_anchor() {
    use filament_contract_model::{AnchorName, ExecutionPoint};
    let document = document(false);
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = clocked_unit(
        "pre Late using v on Config::ConfigVersion::attemptUpdate { self.x >= 1 ms }\n",
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:spine-model"),
        "program.native",
        unit.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    let raw = parsed.source().reference().clone();
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = fsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("intake admits the package");
    let point = ExecutionPoint::Pre {
        operation: AnchorName::new("attemptUpdate").unwrap().into(),
    };
    let graph = PackageDeclarations::assemble_with_execution_points_and_cancel(
        raw,
        "pkg".to_owned(),
        built,
        models,
        Vec::new(),
        Default::default(),
        fsl_semantics::check::AssemblyRun {
            cancel: &filament_exact::Cancel::new(),
            execution_points: BTreeMap::from([("Late".to_owned(), point)]),
        },
    )
    .expect("the unit assembles")
    .check(CheckMode::Linked, CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("the point-bound clause checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    let wire = wire(&emission);
    let [guard] = timed(&wire, "clock_guard")[..] else {
        panic!("one guard");
    };
    let operation = target(&wire, member(&guard["body"], "operation"));
    let anchor = target(&wire, member(&guard["body"], "point_anchor"));
    assert_eq!(operation["semantic_form"], "operation_anchor");
    assert_eq!(anchor["semantic_form"], "operation_anchor");
    assert_ne!(operation["node_id"], anchor["node_id"]);
    assert!(
        anchor["body"].to_string().contains("execution_point"),
        "{anchor}"
    );
    assert!(
        !operation["body"].to_string().contains("execution_point"),
        "{operation}"
    );
}

/// Reads `bytes`, an emitted package over `document`, through I2.
fn read_presence_package(document: &[u8], bytes: &[u8], wire: &Value) -> Result<(), String> {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"]
        .as_array()
        .expect("features")
    {
        evidence.support_feature(feature.as_str().expect("a feature"));
    }
    evidence
        .insert_domain_package_document(package_identity(document), document.to_vec())
        .expect("a fresh evidence key");
    crate::checked_v2::read_checked_package_v2(bytes, library(), V2ReadLimits::fixture(), &evidence)
        .map(|_| ())
        .map_err(|failure| format!("{failure:?}"))
}

/// The members of a clause body or guard part named `name`.
fn members_named<'v>(aggregate: &'v Value, name: &str) -> Vec<&'v Value> {
    aggregate["members"]
        .as_array()
        .expect("an aggregate")
        .iter()
        .filter(|member| member["name"] == name)
        .collect()
}

/// The node a predicate reference names, followed through an
/// `expression`/`reference` wrapper to its application.
fn predicate_operation(wire: &Value, reference: &Value) -> String {
    let node = target(wire, &reference["value"]);
    node["body"]["operation"]["identity"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| node["body"].to_string())
}

/// FR-930-AC-1 to AC-4, FSpec FR-760-AC-1 to AC-3: a quantifier that reads
/// only element presences is written in the clause predicate with its own
/// binder, beside an unquantified constraint, and its binder is never a part
/// or invariant binder.
#[trace("TC-2600", "FR-930-AC-1", "FR-930-AC-2", "FR-930-AC-3", "FR-930-AC-4")]
#[trace(
    "FSpec-TC-1830",
    "FSpec-FR-760-AC-1",
    "FSpec-FR-760-AC-2",
    "FSpec-FR-760-AC-3"
)]
#[test]
#[ignore = "QSL-2144: an element-presence quantifier is not yet written in its clause predicate with its own binder"]
fn an_element_presence_quantifier_is_scoped_in_its_predicate() {
    let document = slot_presence_document();
    let emit = |body: &str| {
        let (unit, checked) = checked(&document, &format!("{body}\n"));
        let graph = checked.unwrap_or_else(|refusals| panic!("{body}: checks: {refusals:?}"));
        let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
        let wire = wire(&emission);
        (unit, emission, wire)
    };
    let guard = |quantifier: &str| {
        format!(
            "pre HasElement using v on Config::ConfigVersion::attemptUpdate {{ \
             self.x >= 1 ms and {quantifier}(e in self.q: present(e)) }}"
        )
    };
    for (quantifier, operation) in [
        ("forall", "filament.op.collection.forall"),
        ("exists", "filament.op.collection.exists"),
    ] {
        let (_, emission, wire) = emit(&guard(quantifier));
        let [clock_guard] = timed(&wire, "clock_guard")[..] else {
            panic!("one guard");
        };
        let parts = member(&clock_guard["body"], "parts")["members"]
            .as_array()
            .expect("parts");
        assert_eq!(
            parts.len(),
            1,
            "{quantifier}: one part, never one per size or slot"
        );
        let part = &parts[0];
        assert!(
            members_named(part, "binders").is_empty(),
            "{quantifier}: no binders"
        );
        let [predicate] = members_named(part, "predicate")[..] else {
            panic!("one predicate");
        };
        assert_eq!(predicate_operation(&wire, predicate), operation);
        assert!(
            !presence_read_bodies(&wire).is_empty(),
            "a presence_read at the binder"
        );
        read_presence_package(&document, emission.package.bytes(), &wire)
            .unwrap_or_else(|failure| panic!("{quantifier}: I2 admits: {failure}"));
    }

    let quantifier = "forall(e in self.q: present(e))";
    let (_, emission, wire) = emit(&format!(
        "time invariant HasElement when {{ {quantifier} }} {{ self.x <= 9 ms }}"
    ));
    let [invariant] = timed(&wire, "time_invariant")[..] else {
        panic!("one invariant");
    };
    assert!(members_named(&invariant["body"], "binders").is_empty());
    let [predicate] = members_named(&invariant["body"], "predicate")[..] else {
        panic!("one predicate");
    };
    assert_eq!(
        predicate_operation(&wire, predicate),
        "filament.op.collection.forall"
    );
    read_presence_package(&document, emission.package.bytes(), &wire)
        .unwrap_or_else(|failure| panic!("the invariant reads back: {failure}"));

    let (_, emission, wire) = emit(&format!("urgent HasElement when {{ {quantifier} }};"));
    let [urgency] = timed(&wire, "urgency")[..] else {
        panic!("one urgency");
    };
    assert!(members_named(&urgency["body"], "binders").is_empty());
    read_presence_package(&document, emission.package.bytes(), &wire)
        .unwrap_or_else(|failure| panic!("the urgency reads back: {failure}"));

    // FR-930-AC-3: a body that constrains its binder's clock is a quantified
    // part whose binders hold the binder (FR-230-AC-55).
    let (_, _, wire) = emit(
        "pre Shared using v on Config::ConfigVersion::attemptUpdate { \
         forall(e in self.q: present(e) implies e >= 1 ms) }",
    );
    let [shared] = timed(&wire, "clock_guard")[..] else {
        panic!("one guard");
    };
    assert!(
        member(&shared["body"], "parts")["members"]
            .as_array()
            .expect("parts")
            .iter()
            .any(|part| !members_named(part, "binders").is_empty()),
        "a constrained binder is a part binder"
    );

    // FR-930-AC-4, FSpec FR-760-AC-2: lifting the presence binder into the
    // part's binders is refused at the binder target.
    let (_, _, mut wire) = emit(&guard("forall"));
    let parameter = nodes(&wire)
        .iter()
        .find(|node| node["semantic_form"] == "parameter" && node.to_string().contains("\"e\""))
        .map(|node| node["node_id"].clone())
        .expect("the quantifier's parameter");
    let guard_id = timed(&wire, "clock_guard")[0]["node_id"].clone();
    let node = wire["semantic_graph"]["nodes"]
        .as_array_mut()
        .expect("nodes")
        .iter_mut()
        .find(|node| node["node_id"] == guard_id)
        .expect("the guard");
    let part = &mut node["body"]["members"]
        .as_array_mut()
        .expect("members")
        .iter_mut()
        .find(|member| member["name"] == "parts")
        .expect("parts")["value"]["members"][0];
    part["members"]
        .as_array_mut()
        .expect("part members")
        .insert(
            1,
            json!({"name": "binders", "value": {"term": "aggregate",
            "members": [{"term": "reference", "target": parameter}]}}),
        );
    let bytes = crate::emit::tests::wire_bytes(&wire).expect("JSON");
    let failure = read_presence_package(&document, &bytes, &wire)
        .expect_err("a binder no constraint reads is refused");
    assert!(failure.contains("timed-node-reference"), "{failure}");
}
