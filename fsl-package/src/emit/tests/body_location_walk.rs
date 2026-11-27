// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-322 body-location production through option values.

use super::*;

fn body_rows(wire: &Value) -> &[Value] {
    wire["checked_term_data"]["body_locations"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
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
    assert!(
        rows.iter()
            .all(|row| !option_ids.contains(&&row["application"]))
    );

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
