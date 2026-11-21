// SPDX-License-Identifier: AGPL-3.0-or-later
//! Terminal emission, occurrence identity and independent reader boundaries.

use super::*;

const MIRROR: &str = "model Mirror = \"example/config-version\";\n";

fn terminal_rows(compiled: &Compiled) -> Vec<crate::checked_v2::ReadTerminal> {
    read_terminals(&admit(compiled)).expect("the emitted terminal rows read")
}

fn graph_bodies(compiled: &Compiled) -> BTreeMap<NodeKey, Value> {
    nodes(&compiled.wire)
        .iter()
        .map(|node| (key(&compiled.wire, &node["node_id"]), node["body"].clone()))
        .collect()
}

#[trace("FR-411-AC-1", "FSpec-FR-476-AC-6")]
#[test]
fn equal_alias_terminals_keep_both_claim_spans_and_one_semantic_row() {
    let first = "terminal First using v on Config any;";
    let second = "terminal Second using v on Mirror any;";
    for declarations in [format!("{first}\n{second}"), format!("{second}\n{first}")] {
        let compiled = compile(&format!("{MIRROR}{declarations}"));
        let terminals = terminal_nodes(&compiled.wire);
        let [node] = terminals.as_slice() else {
            panic!("equal declarations must share one terminal node");
        };
        let rows = terminal_rows(&compiled);
        let [row] = rows.as_slice() else {
            panic!("one semantic terminal row: {rows:?}");
        };
        assert_eq!(row.ordinal, 0);
        assert_eq!(row.declaration, ReadTerminalDeclaration::Any);
        assert_eq!(row.node, key(&compiled.wire, &node["node_id"]));
        let mut claims = compiled.wire["source_map"]
            .as_array()
            .expect("source map")
            .iter()
            .filter(|entry| entry["node_id"] == node["node_id"] && entry["role"] == "claim")
            .collect::<Vec<_>>();
        claims.sort_by_key(|entry| entry["ordinal"].as_u64().expect("ordinal"));
        assert_eq!(claims.len(), 2, "neither declaration's location is lost");
        let expected = if declarations.starts_with(first) {
            [first, second]
        } else {
            [second, first]
        };
        for (ordinal, (claim, declaration)) in claims.iter().zip(expected).enumerate() {
            assert_eq!(
                claim["ordinal"].as_u64(),
                Some(u64::try_from(ordinal).unwrap())
            );
            let [region] = claim["regions"].as_array().expect("regions").as_slice() else {
                panic!("one declaration region");
            };
            let start = usize::try_from(region["start"].as_u64().unwrap()).unwrap();
            let end = usize::try_from(region["end"].as_u64().unwrap()).unwrap();
            assert_eq!(&compiled.text[start..end], declaration);
        }
    }
}

#[trace("FR-411-AC-1", "FR-411-AC-2", "FSpec-FR-476-AC-6")]
#[test]
fn distinct_alias_terminals_keep_key_order_when_source_declarations_are_reordered() {
    let when = "terminal Ready using v on Config when { self.versionNumber = 3 };";
    let any = "terminal Halt using v on Mirror any;";
    let forward = compile(&format!("{MIRROR}{when}\n{any}"));
    let reverse = compile(&format!("{MIRROR}{any}\n{when}"));
    assert_eq!(graph_bodies(&forward), graph_bodies(&reverse));
    let rows = terminal_rows(&forward);
    assert_eq!(rows, terminal_rows(&reverse));
    let [first, second] = rows.as_slice() else {
        panic!("two distinct terminal rows: {rows:?}");
    };
    assert_eq!(
        first.model, second.model,
        "aliases select one model identity"
    );
    assert!(
        first.node < second.node,
        "row order follows structural terminal keys"
    );
    assert_eq!([first.ordinal, second.ordinal], [0, 1]);
    assert_eq!(
        rows.iter()
            .filter(|row| row.declaration == ReadTerminalDeclaration::Any)
            .count(),
        1
    );
    assert_eq!(
        rows.iter()
            .filter(|row| matches!(row.declaration, ReadTerminalDeclaration::When { .. }))
            .count(),
        1
    );
    for row in &rows {
        assert_eq!(
            read_terminal(&admit(&forward), &row.model, row.ordinal).unwrap(),
            row.declaration
        );
    }
    assert_eq!(
        read_terminal(&admit(&forward), &first.model, 2).unwrap(),
        ReadTerminalDeclaration::None
    );
}

#[trace("FR-411-AC-1", "FSpec-FR-476-AC-6")]
#[test]
fn a_terminal_row_names_the_selected_model_root_instead_of_its_object_type() {
    let compiled = compile(&format!(
        "{MIRROR}{WHEN}\nterminal Halt using v on Mirror any;"
    ));
    let expected = NodeKey::new(
        "example/config-version",
        vec![
            PathSegment::Text("model".into()),
            PathSegment::Text("model_root".into()),
        ],
        0,
    )
    .expect("the selected model's structural key");
    let rows = terminal_rows(&compiled);
    assert_eq!(rows.len(), 2);
    for row in rows {
        assert_eq!(row.model, expected);
    }
}

#[trace("FR-411-AC-1", "FR-411-AC-2", "FSpec-FR-476-AC-5")]
#[test]
fn constant_terminal_predicates_keep_their_boolean_literal_without_a_self_parameter() {
    let any = compile(ANY);
    let any_key = terminal_rows(&any)[0].node.clone();
    let mut predicate_keys = BTreeSet::new();
    for truth in [false, true] {
        let compiled = compile(&format!(
            "terminal Constant using v on Config when {{ {truth} }};"
        ));
        let node = terminal_node(&compiled.wire);
        assert_eq!(binding(&node["body"], "parameters")["members"], json!([]));
        let condition_id = target(binding(&node["body"], "condition"));
        let literal = node_with(&compiled.wire, condition_id);
        assert_eq!(literal["node_tag"], "value");
        assert_eq!(literal["semantic_form"], "literal");
        assert_eq!(literal["body"]["value"], json!(truth));
        let rows = terminal_rows(&compiled);
        let [row] = rows.as_slice() else {
            panic!("one constant row")
        };
        assert_eq!(
            row.declaration,
            ReadTerminalDeclaration::When {
                parameter: None,
                condition: key(&compiled.wire, condition_id),
            }
        );
        assert_ne!(row.node, any_key);
        assert!(
            predicate_keys.insert(row.node.clone()),
            "true and false have distinct keys"
        );
    }
    assert_eq!(predicate_keys.len(), 2);
}

fn refusal_with_unique_dependencies(
    compiled: &Compiled,
    edit: impl FnOnce(&mut Value),
) -> (filament_contract_model::CheckedPackageRefusal, usize) {
    let mut envelope = compiled.wire.clone();
    let graph = fsl_semantics::check::wire_id::WireGraph::of_package_value(&compiled.wire).unwrap();
    let position = envelope["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node| node["semantic_form"] == "terminal")
        .expect("terminal node");
    let node = &mut envelope["semantic_graph"]["nodes"][position];
    edit(node);
    let mut targets = BTreeMap::new();
    for member in node["body"]["members"].as_array().expect("body members") {
        let value = &member["value"];
        if value["term"] == "reference" {
            let target = value["target"].clone();
            targets.insert(graph.key_of_value(&target).unwrap(), target);
        } else if let Some(members) = value["members"].as_array() {
            for reference in members {
                let target = reference["target"].clone();
                targets.insert(graph.key_of_value(&target).unwrap(), target);
            }
        }
    }
    node["dependencies"] = Value::Array(targets.into_values().collect());
    match read_envelope(&envelope, &compiled.wire) {
        Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) => (*refusal, position),
        other => panic!("a typed terminal refusal: {other:?}"),
    }
}

#[trace("FR-411-AC-4", "FSpec-FR-476-AC-4")]
#[test]
fn an_any_terminal_with_a_parameter_and_condition_refuses_the_parameter_first() {
    let compiled = compile(WHEN);
    let (refusal, position) = refusal_with_unique_dependencies(&compiled, |node| {
        node["body"]["members"][1]["value"]["value"] = json!("any");
    });
    assert_eq!(refusal.code, RefusalCode::InvalidSemanticGraph);
    assert_eq!(
        refusal.loci[0].path.as_ref().map(|path| path.as_str()),
        Some(format!("/semantic_graph/nodes/{position}/body/members/2").as_str())
    );
}

#[trace("FR-411-AC-4", "FSpec-FR-476-AC-4")]
#[test]
fn two_self_parameters_refuse_the_parameter_binding_with_unique_dependencies() {
    let compiled = compile(WHEN);
    let (refusal, position) = refusal_with_unique_dependencies(&compiled, |node| {
        let parameters = node["body"]["members"][2]["value"]["members"]
            .as_array_mut()
            .unwrap();
        parameters.push(parameters[0].clone());
    });
    assert_eq!(refusal.code, RefusalCode::InvalidSemanticGraph);
    assert_eq!(
        refusal.loci[0].path.as_ref().map(|path| path.as_str()),
        Some(format!("/semantic_graph/nodes/{position}/body/members/2").as_str())
    );
}

#[trace("FR-411-AC-4", "FSpec-FR-476-AC-4")]
#[test]
fn wrong_binding_order_is_refused_before_invalid_mode() {
    let compiled = compile(WHEN);
    let (refusal, position) = refusal_with_unique_dependencies(&compiled, |node| {
        node["body"]["members"][1]["value"]["value"] = json!("unsupported");
        node["body"]["members"].as_array_mut().unwrap().swap(1, 3);
    });
    assert_eq!(refusal.code, RefusalCode::InvalidSemanticGraph);
    assert_eq!(
        refusal.loci[0].path.as_ref().map(|path| path.as_str()),
        Some(format!("/semantic_graph/nodes/{position}/body").as_str())
    );
}
