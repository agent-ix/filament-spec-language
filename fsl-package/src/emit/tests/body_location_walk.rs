// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-322 body-location production through option values.

use super::*;

fn body_rows(wire: &Value) -> &[Value] {
    wire["checked_term_data"]["body_locations"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn add_dependency(node: &mut Value, target: &Value) {
    let mut dependencies: Vec<CheckedNodeId> = serde_json::from_value(node["dependencies"].clone())
        .expect("the node dependencies are structural ids");
    let target: CheckedNodeId = serde_json::from_value(target.clone()).unwrap();
    if !dependencies.contains(&target) {
        dependencies.push(target);
    }
    dependencies.sort();
    node["dependencies"] = serde_json::to_value(dependencies).unwrap();
}

fn set_first_reference(term: &mut Value, target: &Value) -> bool {
    match term {
        Value::Object(object) if object.get("term") == Some(&json!("reference")) => {
            object.insert("target".to_owned(), target.clone());
            true
        }
        Value::Object(object) => object
            .values_mut()
            .any(|value| set_first_reference(value, target)),
        Value::Array(items) => items
            .iter_mut()
            .any(|value| set_first_reference(value, target)),
        _ => false,
    }
}

fn assert_cycle_refused(emission: &Emission, wire: &Value, cycle: &[Value]) {
    let bytes = jcs(wire);
    let outcome = read_v2(
        &bytes,
        library(),
        V2ReadLimits::fixture(),
        &read_evidence(emission),
    );
    let Read::Refused(crate::checked_v2::V2ReadRefusal::Envelope {
        refusal,
        locus: Some(locus),
    }) = outcome
    else {
        panic!("the strict package-byte reader must refuse the cycle: {outcome:?}");
    };
    let expected = cycle
        .iter()
        .map(|id| serde_json::from_value::<CheckedNodeId>(id.clone()).unwrap())
        .min()
        .unwrap();
    let at = nodes(wire)
        .iter()
        .position(|node| node["node_id"] == json!(expected))
        .unwrap();
    assert_eq!(nodes(wire)[at]["node_id"], json!(expected));
    let pointer = format!("/semantic_graph/nodes/{at}");
    assert_eq!(
        refusal.code,
        filament_contract_model::CheckedPackageRefusalCode::InvalidSemanticGraph,
        "a body-walk cycle is a graph-shape refusal"
    );
    assert_eq!(refusal.cause, None);
    assert_eq!(
        refusal.loci.len(),
        1,
        "no partial locations accompany refusal"
    );
    assert_eq!(
        refusal.loci[0].path.as_ref().map(|path| path.as_str()),
        Some(pointer.as_str())
    );
    let fsl_foundation::diagnostic::Locus::Artifact {
        pointer: reported, ..
    } = *locus
    else {
        panic!("the refusal pointer is an artifact locus");
    };
    assert_eq!(reported.as_str(), pointer);
}

/// FSpec FR-322-AC-540/541: option values and their record/tuple payloads
/// contribute no rows, while each reached application visit keeps its exact
/// occurrence and term ordinal pairing.
#[trace("FSpec-FR-322-AC-540", "FSpec-FR-322-AC-541")]
#[test]
fn nested_option_record_tuple_walk_emits_one_row_per_application_visit() {
    let (_, package) = package_from_text(
        "record Cell { left: Option<Integer>; right: Integer; }\n\
         tuple Pair(Cell, Option<Integer>);\n\
         function nested using v(x: Integer): Option<Pair> pure {\n\
             Some(Pair(Cell { left: Some(x + 1), right: x + 1 }, Some(x + 2)))\n\
         }\n",
    );
    let emission = emit_checked(&package).expect("the package emits");
    let wire = wire(&emission);
    let rows = body_rows(&wire);
    assert_eq!(
        rows.len(),
        3,
        "each application reached below options is retained"
    );

    let nodes = nodes(&wire);
    let option_ids: Vec<_> = nodes
        .iter()
        .filter(|node| node["semantic_form"] == "option_value")
        .map(|node| &node["node_id"])
        .collect();
    assert_eq!(option_ids.len(), 3, "outer and field options are emitted");
    assert!(rows
        .iter()
        .all(|row| !option_ids.contains(&&row["application"])));

    for row in rows {
        let app = nodes
            .iter()
            .find(|node| node["node_id"] == row["application"])
            .expect("every emitted body-location row names an emitted node");
        assert_eq!(
            app["node_tag"], "expression",
            "rows belong to application terms"
        );
        assert_ne!(app["semantic_form"], "reference");
    }

    let repeated_app = rows
        .iter()
        .find(|row| {
            rows.iter()
                .filter(|other| other["application"] == row["application"])
                .count()
                == 2
        })
        .expect("the repeated x + 1 reaches share an application identity");
    let same_app: Vec<_> = rows
        .iter()
        .filter(|row| row["application"] == repeated_app["application"])
        .collect();
    let keys: BTreeSet<_> = same_app
        .iter()
        .map(|row| {
            (
                row["occurrence"]["ordinal"].as_u64().unwrap(),
                row["term_ordinal"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        keys.len(),
        2,
        "each visit keeps its occurrence/term ordinal key"
    );
}

/// FSpec FR-322-AC-543: direct and nested option constructors with no
/// application terms produce no body-location rows.
#[trace("FSpec-FR-322-AC-543")]
#[test]
fn nested_option_values_without_applications_emit_no_body_location_rows() {
    let (_, package) = package_from_text(
        "function direct using v(x: Integer): Option<Integer> pure { Some(x) }\n\
         function nested using v(x: Integer): Option<Option<Integer>> pure { Some(Some(x)) }\n",
    );
    let emission = emit_checked(&package).expect("the package emits");
    let wire = wire(&emission);
    assert!(
        body_rows(&wire).is_empty(),
        "option value nodes without application terms have no body-location rows"
    );
}

/// FSpec FR-322-AC-549: a source-built record with an option edge is emitted
/// without a value cycle, while a record/option cycle in package bytes is
/// refused at its first structural node, without a cause or partial rows.
#[trace("FSpec-FR-322-AC-549")]
#[test]
fn record_value_self_cycle_is_refused_without_partial_body_locations() {
    let (_, package) = package_from_text(
        "record Cell { next: Option<Cell>; }\n\
         function cell using v(x: Cell): Cell pure { Cell { next: Some(x) } }\n\
         function calc using v(x: Integer): Integer pure { x + 1 }\n",
    );
    let emission = emit_checked(&package).expect("the acyclic producer package emits");
    let mut wire = wire(&emission);
    assert!(matches!(read_back(&emission), Read::Verified { .. }));
    assert!(!body_rows(&wire).is_empty(), "the producer retains its row");
    let semantic_nodes = wire["semantic_graph"]["nodes"].as_array_mut().unwrap();
    let record = semantic_nodes
        .iter()
        .position(|node| node["semantic_form"] == "record_value")
        .expect("the source produces one record value");
    let option = semantic_nodes
        .iter()
        .position(|node| node["semantic_form"] == "option_value")
        .expect("Some(x) produces an option value");
    let record_id = semantic_nodes[record]["node_id"].clone();
    let option_id = semantic_nodes[option]["node_id"].clone();
    assert!(set_first_reference(
        &mut semantic_nodes[option]["body"],
        &record_id
    ));
    add_dependency(&mut semantic_nodes[option], &record_id);
    assert_cycle_refused(&emission, &wire, &[record_id, option_id]);
}

/// FSpec FR-322-AC-550: source-built tuple, record and option values remain
/// acyclic, while their tuple/option cycle in package bytes is refused at its
/// first structural node, without a cause or partial body-location rows.
#[trace("FSpec-FR-322-AC-550")]
#[test]
fn tuple_option_cycle_is_refused_without_partial_body_locations() {
    let (_, package) = package_from_text(
        "tuple Pair(Cell);\n\
         record Cell { maybe: Option<Pair>; }\n\
         function pair using v(x: Cell): Pair pure { Pair(x) }\n\
         function cell using v(x: Pair): Cell pure { Cell { maybe: Some(x) } }\n\
         function calc using v(x: Integer): Integer pure { x + 1 }\n",
    );
    let emission = emit_checked(&package).expect("the acyclic producer package emits");
    let mut wire = wire(&emission);
    assert!(matches!(read_back(&emission), Read::Verified { .. }));
    assert!(!body_rows(&wire).is_empty(), "the producer retains its row");
    let semantic_nodes = wire["semantic_graph"]["nodes"].as_array_mut().unwrap();
    let option = semantic_nodes
        .iter()
        .position(|node| node["semantic_form"] == "option_value")
        .expect("the source produces an option value");
    let tuple = semantic_nodes
        .iter()
        .position(|node| node["semantic_form"] == "tuple_value")
        .expect("the source produces a tuple value");
    let option_id = semantic_nodes[option]["node_id"].clone();
    let tuple_id = semantic_nodes[tuple]["node_id"].clone();
    let record = semantic_nodes
        .iter()
        .position(|node| node["semantic_form"] == "record_value")
        .expect("the source produces a record value");
    let record_id = semantic_nodes[record]["node_id"].clone();
    assert!(set_first_reference(
        &mut semantic_nodes[tuple]["body"],
        &record_id
    ));
    add_dependency(&mut semantic_nodes[tuple], &record_id);
    assert!(set_first_reference(
        &mut semantic_nodes[record]["body"],
        &option_id
    ));
    add_dependency(&mut semantic_nodes[record], &option_id);
    assert!(set_first_reference(
        &mut semantic_nodes[option]["body"],
        &tuple_id
    ));
    add_dependency(&mut semantic_nodes[option], &tuple_id);
    assert_cycle_refused(&emission, &wire, &[option_id, tuple_id, record_id]);
}
