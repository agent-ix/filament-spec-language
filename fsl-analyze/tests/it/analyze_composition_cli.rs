// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1044 (and FSpec FR-348-AC-3, FR-348-AC-4): a `native-analyze/1` item
//! naming a checked `compose` declaration selects its composition, and the
//! composition's component-guarantee premises settle in the same request,
//! each over its own component's checked package.
//!
//! The components are the checked packages `test/part-a` and `test/part-b`,
//! supplied as request libraries, and the unit `test/system` imports them
//! over the Counter domain package. The registry is `Engines::fsl()`, which
//! registers no composition backend, so a composition settles `unsupported`
//! (FSpec FR-348-AC-4). FSpec FR-348-AC-1 and AC-2 are not delivered here.

use std::path::PathBuf;

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, counter_snapshot, label, package_selection, request_members, Workspace, HANDOFF,
};
use crate::counter::{clause_text, document};

/// The clause `test/part-a` and, with `name` `Capped`, `test/part-b` declare.
const BOUNDED: &str = "always holds(c.value <= 3)";

/// `SysRely`'s peer: `Capped` relies on `Bounded`.
const RELY: &str = "rely PB::Capped on PA::Bounded;";

const COMPONENTS: &str = "component PA from A using M; component PB from B using M; \
     guarantee PA::Bounded; guarantee PB::Capped; ";

fn library(formula: &str, name: &str) -> String {
    crate::counter::unit_text(&clause_text(name, "", formula))
}

fn program(body: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         profile t0 = \"filament.temporal.infinite-trace/v1\";\n\
         import \"test/part-a\" as A;\n\
         import \"test/part-b\" as B;\n\
         model M = \"test/counter\";\n\
         function Noop using v(): Boolean pure {{ true }}\n{body}"
    )
}

fn compose(name: &str, rule: &str, members: &str) -> String {
    format!("compose {name} rule {rule} {{ {COMPONENTS}{members}}}\n")
}

/// The declarations of `test/system`.
fn system() -> String {
    format!(
        "{}{}{}{}{}{}{}",
        clause_text("Local", "", BOUNDED),
        compose("Sys", "interface", ""),
        compose("SysPR", "protocol-refinement", ""),
        compose("SysRT", "refinement-transfer", ""),
        compose("SysAG", "assume-guarantee", ""),
        compose("SysRely", "interface", RELY),
        compose(
            "SysCycle",
            "interface",
            &format!("{RELY} rely PA::Bounded on PB::Capped;"),
        ),
    )
}

/// A request over `body` with the component `Capped` clause `capped`, and
/// `items` (none for the default items).
fn request_for(ws: &Workspace, body: &str, capped: &str, items: Option<Value>) -> PathBuf {
    ws.write("part-a.fsl", library(BOUNDED, "Bounded").as_bytes());
    ws.write("part-b.fsl", library(capped, "Capped").as_bytes());
    ws.write("program.fsl", program(body).as_bytes());
    ws.write("model.json", &document(false));
    ws.write("start.json", &counter_snapshot(&[("a", 0)]));
    let mut request = request_members(items);
    let member = |identity: &str, file: &str| {
        json!({"identity": identity, "source": {"file": file, "authority": "test",
            "identity": file, "document": "Component", "formal_revision": 1}})
    };
    request["libraries"] = json!([
        member("test/part-a", "part-a.fsl"),
        member("test/part-b", "part-b.fsl")
    ]);
    let document = json!({"format": "native-analyze/1", "request": request});
    ws.write("analyze.json", document.to_string().as_bytes())
}

fn request(ws: &Workspace, body: &str, items: Value) -> PathBuf {
    request_for(ws, body, BOUNDED, Some(items))
}

fn composition(name: &str) -> Value {
    json!({"claim": name, "kind": "composition"})
}

/// The item's `record` diagnostic `cause`.
fn cause(item: &Value) -> &str {
    item["record"]["diagnostic"]["cause"]
        .as_str()
        .unwrap_or_else(|| panic!("a diagnostic cause in {item}"))
}

/// Asserts `item` is a composition settled `unsupported` by disposition,
/// naming kind `composition`, and returns its `request` member, when it has
/// one.
fn assert_unsupported_composition(item: &Value) {
    assert_eq!(label(item), "unsupported", "{item}");
    assert_eq!(item["record"]["settled_by"], "disposition", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["code"], "unsupported_projection",
        "{item}"
    );
    assert_eq!(cause(item), "unsupported-requested-capability", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["kind"], "composition",
        "{item}"
    );
    assert!(item["record"]["backend"].is_null(), "{item}");
}

/// The roles of an item's `request.premises`.
fn roles(item: &Value) -> Vec<&str> {
    item["request"]["premises"]
        .as_array()
        .unwrap_or_else(|| panic!("premises in {item}"))
        .iter()
        .map(|premise| premise["role"].as_str().expect("a role"))
        .collect()
}

/// A routed composition item: the derived slot list, no candidates, and an
/// `unsupported` disposition.
fn assert_routed(item: &Value) {
    assert_unsupported_composition(item);
    assert_eq!(roles(item), ["component-guarantee"; 2], "{item}");
    assert_eq!(
        item["request"]["candidates"],
        json!({"type": "set", "backends": []}),
        "{item}"
    );
}

/// FR-1044-AC-1: a routed composition settles `unsupported`, its premise
/// items `proved` by EN-1, each over its own component, and the composition
/// is not `proved`.
#[trace("FR-1044-AC-1")]
#[trace("FSpec-FR-348-AC-4")]
#[test]
fn a_routed_composition_is_unsupported_beside_its_proved_premises() {
    let ws = Workspace::new("compose-ac1");
    let run = analyze(&request(&ws, &system(), json!([composition("Sys")])));
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(items.len(), 3, "{document}");
    assert_routed(&items[0]);
    assert_eq!(items[1]["claim_name"], "Bounded", "{document}");
    assert_eq!(label(&items[1]), "proved", "{document}");
    assert_eq!(items[2]["claim_name"], "Capped", "{document}");
    assert_eq!(label(&items[2]), "proved", "{document}");
    assert_eq!(run.exit, 21, "{document}");
}

/// FSpec FR-348-AC-4: the production registry advertises no composition
/// backend, so the item's candidates are empty before any negotiation.
#[trace("FSpec-FR-348-AC-4")]
#[test]
fn the_production_registry_has_no_composition_candidate() {
    let mut registry = fsl_route::Registry::new();
    let engines = fsl_analyze::analyze::Engines::fsl().expect("the FSL engines register");
    for descriptor in engines.registrations() {
        registry
            .register(descriptor.clone())
            .expect("a descriptor registers");
    }
    assert!(matches!(
        registry.candidates(fsl_semantics::check::Capability::Composition, None),
        fsl_route::CandidateOutcome::Candidates(set) if set.candidates().is_empty()
    ));
}

/// FR-1044-AC-2: the other routed rules write AC-1's items; an
/// `assume-guarantee` selection is planner-local, with no `request` member.
#[trace("FR-1044-AC-2")]
#[test]
fn the_other_rules_and_the_planner_local_item_settle_unsupported() {
    let ws = Workspace::new("compose-ac2");
    for name in ["SysPR", "SysRT"] {
        let run = analyze(&request(&ws, &system(), json!([composition(name)])));
        let document = run.document();
        assert_routed(&document["items"][0]);
        assert_eq!(document["items"].as_array().expect("items").len(), 3);
        assert_eq!(run.exit, 21, "{document}");
    }
    let run = analyze(&request(&ws, &system(), json!([composition("SysAG")])));
    let document = run.document();
    let item = &document["items"][0];
    assert_unsupported_composition(item);
    assert_eq!(item["claim_name"], "SysAG", "{document}");
    assert!(item["request"].is_null(), "{item}");
    assert_eq!(document["items"][1]["claim_name"], "Bounded", "{document}");
    assert_eq!(document["items"][2]["claim_name"], "Capped", "{document}");
    assert_eq!(run.exit, 21, "{document}");
}

/// FR-1044-AC-3: each selector settles by its own rule, and the request's
/// other items still run.
#[trace("FR-1044-AC-3")]
#[test]
fn each_selector_settles_by_its_own_rule() {
    let ws = Workspace::new("compose-ac3");
    let run = analyze(&request(
        &ws,
        &system(),
        json!([
            composition("Sys"),
            composition("Missing"),
            composition("Local"),
            {"claim": "Sys", "kind": "temporal-satisfaction"},
            {"claim": "Sys"},
            {"claim": "SysAG", "kind": "temporal-satisfaction"},
        ]),
    ));
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_unsupported_composition(&items[0]);
    assert_eq!(cause(&items[1]), "unknown-claim", "{document}");
    assert_eq!(
        items[1]["record"]["diagnostic"]["detail"]["name"],
        "Missing"
    );
    let inapplicable = |item: &Value, index: u64, capability: &str, family: Option<&str>| {
        assert_eq!(cause(item), "inapplicable-kind", "{item}");
        let detail = &item["record"]["diagnostic"]["detail"];
        assert_eq!(detail["request_index"], index, "{item}");
        assert_eq!(detail["capability"], capability, "{item}");
        match family {
            Some(family) => assert_eq!(detail["family"], family, "{item}"),
            None => assert!(detail["family"].is_null(), "{item}"),
        }
    };
    inapplicable(&items[2], 2, "composition", Some("temporal-trace"));
    inapplicable(&items[3], 3, "temporal-satisfaction", Some("protocol"));
    assert_eq!(cause(&items[4]), "absent-kind", "{document}");
    inapplicable(&items[5], 5, "temporal-satisfaction", None);
    assert_eq!(run.exit, 20, "{document}");
}

/// The roles and keys of `item`'s `request.premises`, as the caller restates
/// them, each object's members written in reverse.
fn restated(item: &Value) -> Value {
    Value::Array(
        item["request"]["premises"]
            .as_array()
            .expect("premises")
            .iter()
            .map(|premise| {
                let mut reversed = serde_json::Map::new();
                reversed.insert("key".to_owned(), premise["key"].clone());
                reversed.insert("role".to_owned(), premise["role"].clone());
                Value::Object(reversed)
            })
            .collect(),
    )
}

fn with_premises(name: &str, premises: &Value) -> Value {
    json!({"claim": name, "kind": "composition", "premises": premises})
}

/// FR-1044-AC-4: a caller may restate the derived slot list and nothing
/// else; a mistyped member refuses the request, a different list refuses
/// the item.
#[trace("FR-1044-AC-4")]
#[test]
fn a_caller_may_only_restate_the_derived_premise_keys() {
    let ws = Workspace::new("compose-ac4");
    let derived = analyze(&request(&ws, &system(), json!([composition("Sys")]))).item(0);
    let listed = restated(&derived);
    // (a) the derived list, with members reversed, is the item without it.
    for name in ["Sys", "SysAG"] {
        let plain = analyze(&request(&ws, &system(), json!([composition(name)])));
        let run = analyze(&request(
            &ws,
            &system(),
            json!([with_premises(name, &listed)]),
        ));
        assert_eq!(run.exit, 21, "{name}");
        assert_eq!(
            run.document()["items"].as_array().map(Vec::len),
            plain.document()["items"].as_array().map(Vec::len),
            "{name}"
        );
        assert_eq!(label(&run.item(1)), label(&plain.item(1)), "{name}");
    }
    // (b) a slot removed, two swapped or one relabelled refuses that item.
    let slots = listed.as_array().expect("the list").clone();
    let mut relabelled = slots.clone();
    relabelled[1]["role"] = json!("rely");
    let variants = [
        json!([slots[0]]),
        json!([slots[1], slots[0]]),
        Value::Array(relabelled),
    ];
    for name in ["Sys", "SysAG"] {
        for premises in &variants {
            let run = analyze(&request(
                &ws,
                &system(),
                json!([with_premises(name, premises), {"claim": "Local"}]),
            ));
            let document = run.document();
            let items = document["items"].as_array().expect("items");
            assert_eq!(items.len(), 2, "no premise item: {document}");
            assert_eq!(
                items[0]["record"]["diagnostic"]["code"], "invalid_runtime_input",
                "{document}"
            );
            assert_eq!(cause(&items[0]), "invalid-value", "{document}");
            assert_eq!(
                items[0]["record"]["diagnostic"]["detail"]["pointer"], "/items/0/premises",
                "{document}"
            );
            assert_eq!(label(&items[1]), "proved", "{document}");
            assert_eq!(run.exit, 20, "{document}");
        }
    }
    // The item that settles alone settles at the request stage `S4`.
    let alone = analyze(&request(
        &ws,
        &system(),
        json!([with_premises("Sys", &json!([]))]),
    ));
    assert_eq!(alone.document()["last_stage"], "S4");
    // (c) a mistyped member refuses the whole request.
    for (premises, pointer) in [
        (json!({}), "/items/0/premises"),
        (json!([1]), "/items/0/premises/0"),
        (
            json!([{"role": "nope", "key": {}}]),
            "/items/0/premises/0/role",
        ),
    ] {
        let run = analyze(&request(
            &ws,
            &system(),
            json!([with_premises("Sys", &premises)]),
        ));
        let document = run.document();
        assert_eq!(run.exit, 20, "{document}");
        assert!(document.to_string().contains(pointer), "{document}");
        assert!(
            document["items"].as_array().is_none_or(Vec::is_empty),
            "{document}"
        );
    }
    // (d) a name that is no composition keeps its own refusal, with no
    // premise item.
    let run = analyze(&request(
        &ws,
        &system(),
        json!([
            with_premises("Missing", &json!([])),
            with_premises("Local", &json!([]))
        ]),
    ));
    let document = run.document();
    assert_eq!(cause(&document["items"][0]), "unknown-claim", "{document}");
    assert_eq!(
        cause(&document["items"][1]),
        "inapplicable-kind",
        "{document}"
    );
    assert_eq!(document["items"].as_array().map(Vec::len), Some(2));
}

/// FR-1044-AC-5: a guarantee checked under peer guarantees is not run as
/// the clause alone.
#[trace("FR-1044-AC-5")]
#[test]
fn a_premise_under_assumptions_is_not_run_as_the_clause_alone() {
    let ws = Workspace::new("compose-ac5");
    let run = analyze(&request(&ws, &system(), json!([composition("SysRely")])));
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(items.len(), 3, "no item for the rely slot: {document}");
    assert_eq!(roles(&items[0]).len(), 3, "{document}");
    assert_eq!(label(&items[1]), "proved", "{document}");
    let assumed = &items[2];
    assert_eq!(assumed["claim_name"], "Capped", "{document}");
    assert_eq!(label(assumed), "unsupported", "{document}");
    assert_eq!(assumed["record"]["settled_by"], "disposition", "{document}");
    assert_eq!(
        cause(assumed),
        "unsupported-requested-capability",
        "{document}"
    );
    assert_eq!(
        assumed["record"]["diagnostic"]["detail"]["kind"], "temporal-satisfaction",
        "{document}"
    );
    assert_eq!(
        assumed["request"]["candidates"],
        json!({"type": "set", "backends": []}),
        "{document}"
    );
    assert_unsupported_composition(&items[0]);
    assert_eq!(run.exit, 21, "{document}");
    // The clause alone, in its own package, is a different request.
    let alone = analyze(&request(&ws, &system(), json!([composition("Sys")])));
    assert_eq!(label(&alone.item(2)), "proved");
}

/// FR-1044-AC-6: a dependency cycle with no lattice profile refuses the
/// composition item and derives no premise item.
#[trace("FR-1044-AC-6")]
#[test]
fn a_cycle_without_a_lattice_refuses_the_item() {
    let ws = Workspace::new("compose-ac6");
    let run = analyze(&request(&ws, &system(), json!([composition("SysCycle")])));
    let document = run.document();
    assert_eq!(
        document["items"].as_array().map(Vec::len),
        Some(1),
        "{document}"
    );
    let item = &document["items"][0];
    assert_eq!(
        item["record"]["diagnostic"]["code"], "invalid_model_binding",
        "{item}"
    );
    assert_eq!(cause(item), "malformed-declaration", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["cycles"],
        json!([["PB::Capped", "PA::Bounded"]]),
        "{item}"
    );
    assert_eq!(run.exit, 20, "{document}");
}

/// FR-1044-AC-8: premise items are written once however many selections
/// derive them, and a guarantee under another assumption set is another
/// item with an equal request.
#[trace("FR-1044-AC-8")]
#[test]
fn a_shared_premise_is_written_once() {
    let ws = Workspace::new("compose-ac8");
    let run = analyze(&request(
        &ws,
        &system(),
        json!([composition("Sys"), composition("Sys")]),
    ));
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(items.len(), 4, "{document}");
    assert_eq!(items[2]["claim_name"], "Bounded", "{document}");
    assert_eq!(items[3]["claim_name"], "Capped", "{document}");
    let run = analyze(&request(
        &ws,
        &system(),
        json!([
            composition("Sys"),
            composition("Sys"),
            composition("SysRely")
        ]),
    ));
    let document = run.document();
    let items = document["items"].as_array().expect("items");
    assert_eq!(items.len(), 6, "{document}");
    for index in 0..3 {
        assert_unsupported_composition(&items[index]);
    }
    assert_eq!(items[3]["claim_name"], "Bounded", "{document}");
    assert_eq!(label(&items[4]), "proved", "{document}");
    assert_eq!(label(&items[5]), "unsupported", "{document}");
    assert_eq!(items[5]["request"], {
        let mut equal = items[4]["request"].clone();
        equal["request_index"] = json!(5);
        equal["candidates"] = items[5]["request"]["candidates"].clone();
        equal
    });
}

/// FR-1044-AC-9: a refuted component guarantee keeps its own result, and the
/// composition is neither `proved` nor `refuted`.
#[trace("FR-1044-AC-9")]
#[test]
fn a_refuted_premise_keeps_its_own_record() {
    let ws = Workspace::new("compose-ac9");
    let run = analyze(&request_for(
        &ws,
        &system(),
        "always holds(c.value <= 2)",
        Some(json!([composition("Sys")])),
    ));
    let document = run.document();
    assert_unsupported_composition(&document["items"][0]);
    assert_eq!(label(&document["items"][1]), "proved", "{document}");
    let capped = &document["items"][2];
    assert_eq!(label(capped), "refuted", "{document}");
    assert!(
        capped["counterexample"].to_string().contains("\"3\""),
        "the counterexample reaches value = 3: {capped}"
    );
    assert_eq!(run.exit, 21, "{document}");
}

/// FR-1044-AC-10: with no `items`, the composition is a default item in
/// source order, then the subject's derived item, then the premise items.
#[trace("FR-1044-AC-10")]
#[test]
fn a_composition_is_a_default_item() {
    let ws = Workspace::new("compose-ac10");
    let body = format!(
        "{}{}",
        clause_text("Local", "", BOUNDED),
        compose("Sys", "interface", "")
    );
    let run = analyze(&request_for(&ws, &body, BOUNDED, None));
    let document = run.document();
    let names: Vec<&str> = document["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| item["claim_name"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(names[..2], ["Local", "Sys"], "{document}");
    assert_eq!(names[3..], ["Bounded", "Capped"], "{document}");
    assert_eq!(run.exit, 21, "{document}");
    assert_eq!(label(&document["items"][0]), "proved", "{document}");
    assert_unsupported_composition(&document["items"][1]);
}

/// FR-1044-AC-10, second half: over the unit's `package` handoff the same
/// items, records and exit code. The package route reads no composition
/// declaration (it refuses the family typed, FR-940), so this is the
/// request's current outcome.
#[test]
#[ignore = "QSL-2739: the package route has no composition form"]
fn the_package_route_settles_the_same_items() {
    let ws = Workspace::new("compose-ac10-package");
    let body = format!(
        "{}{}",
        clause_text("Local", "", BOUNDED),
        compose("Sys", "interface", "")
    );
    let path = request_for(&ws, &body, BOUNDED, None);
    let source: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("the request reads")).expect("JSON");
    let (selected, _) = package_selection(&ws, &source["request"]);
    let handoff = ws.write(
        "analyze-package.json",
        json!({"format": "native-analyze/1", "request": selected})
            .to_string()
            .as_bytes(),
    );
    let _ = HANDOFF;
    let by_source = analyze(&path);
    let by_package = analyze(&handoff);
    assert_eq!(by_package.exit, by_source.exit);
    assert_eq!(
        by_package.document()["items"],
        by_source.document()["items"]
    );
}
