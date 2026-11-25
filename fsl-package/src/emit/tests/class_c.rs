// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-3206: shared collection applications retain valid FR-322 occurrence
//! rows when package-wide numbering visits several owning functions.

use super::*;

const COLL_CART: &str = include_str!(
    "../../../../fsl-command/tests/fixtures/qsl3206/shop/rules/values/collections/coll-cart.fsl"
);

#[trace("QSL-3206", "FSpec-FR-322")]
#[test]
fn coll_cart_shared_application_emits_with_nine_function_owners_and_source_rows() {
    let identity =
        fsl_foundation::SourceIdentity::new("example", "example:shop/values/collections/coll-cart");
    let parsed = fsl_cst::parse(
        identity.clone(),
        "../../../rules/values/collections/coll-cart.fsl",
        COLL_CART.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the coll-cart unit parses");
    assert_eq!(parsed.diagnostics(), []);

    let raw = parsed.source().reference().clone();
    let unit = fsl_forms::build_unit(&parsed).expect("the coll-cart unit builds");
    let graph = PackageDeclarations::assemble(
        raw,
        identity
            .label(fsl_foundation::source::SourceLabel::Identity)
            .to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the coll-cart program assembles")
    .check(
        CheckMode::Linked,
        CheckingLimits::default(),
        &fsl_semantics::check::test_support::ZeroRewardEvaluator,
    )
    .expect("the coll-cart program checks");
    let package = CheckedPackage::link(graph);

    let emission = emit_checked(&package)
        .expect("shared prices() Application has valid owner-grouped occurrence ordinals");
    let wire = wire(&emission);
    let prices = nodes(&wire)
        .iter()
        .find(|node| {
            node["node_tag"] == "function"
                && node["declaration"]["qualified_name"] == json!(["prices"])
        })
        .expect("prices is one declared function node");
    let prices_id = &prices["node_id"];
    let shared_prices = nodes(&wire)
        .iter()
        .find(|node| {
            node["body"]["term"] == "application"
                && node["body"]["operation"]["identity"] == "filament.op.function.call"
                && node["body"]["arguments"][0]["target"] == *prices_id
        })
        .expect("calls to prices share one Application node");
    let mut expression_rows: Vec<_> = shared_prices["occurrences"]
        .as_array()
        .expect("occurrences are rows")
        .iter()
        .filter(|row| row["role"] == "expression")
        .collect();
    expression_rows.sort_by_key(|row| row["ordinal"].as_u64().expect("ordinal is an integer"));
    assert_eq!(
        expression_rows.len(),
        9,
        "the nine coll-cart calls to prices() each own one expression occurrence"
    );
    assert_eq!(
        expression_rows
            .iter()
            .map(|row| row["ordinal"].as_u64().expect("ordinal is an integer"))
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 4, 5, 6, 7, 8],
        "shared call occurrences have consecutive ordinals"
    );

    let mut source_rows: Vec<_> = wire["source_map"]
        .as_array()
        .expect("source map is a list")
        .iter()
        .filter(|row| row["node_id"] == shared_prices["node_id"] && row["role"] == "expression")
        .collect();
    source_rows.sort_by_key(|row| row["ordinal"].as_u64().expect("ordinal is an integer"));
    assert_eq!(
        source_rows.len(),
        9,
        "every prices() call has a source-map row"
    );
    let actual_starts: Vec<_> = source_rows
        .iter()
        .map(|row| {
            row["regions"][0]["start"]
                .as_u64()
                .expect("source offset is an integer")
        })
        .collect();
    let expected_starts: Vec<_> = COLL_CART
        .match_indices("prices()")
        .map(|(start, _)| u64::try_from(start).expect("fixture source offset fits u64"))
        .collect();
    assert_eq!(
        actual_starts, expected_starts,
        "source-map rows correspond to each prices() call"
    );

    let read = read_v2(
        emission.package.bytes(),
        LibraryName::new("example:shop/values/collections/coll-cart")
            .expect("the source identity is a library name"),
        V2ReadLimits::fixture(),
        &read_evidence(&emission),
    );
    let admitted = match read {
        Read::Verified { admitted, .. } => admitted,
        other => panic!("Contract IR must admit the coll-cart package: {other:?}"),
    };
    let mut admitted_rows: Vec<_> = admitted
        .source_map()
        .iter()
        .filter(|row| {
            serde_json::to_value(&row.node_id).ok().as_ref() == Some(&shared_prices["node_id"])
                && row.role == filament_contract_model::CheckedOccurrenceRole::Expression
        })
        .collect();
    admitted_rows.sort_by_key(|row| row.ordinal);
    assert_eq!(
        admitted_rows
            .iter()
            .map(|row| row.ordinal)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 4, 5, 6, 7, 8],
        "verified Contract IR readback retains all shared call occurrences"
    );
    let admitted_starts: Vec<_> = admitted_rows
        .iter()
        .map(|row| row.regions[0].start)
        .collect();
    assert_eq!(
        admitted_starts, expected_starts,
        "verified Contract IR source-map rows correspond to each prices() call"
    );
}

#[trace("QSL-3206", "FSpec-FR-322")]
#[test]
fn recursive_drop_first_measure_and_body_emit_with_owner_grouped_ordinals() {
    let identity = fsl_foundation::SourceIdentity::new("qsl-3206-test", "recursive-drop-first");
    let source = r#"language "ix:native" edition "1-draft";
profile v = "filament.value.complete/v1";
function tally using v(s: Sequence<Int[0, 9]>): Integer pure decreases(size(s)) { if size(s) > 0 then 1 + tally(dropFirst(s)) else 0 }
function example using v(): Boolean pure { let q: Sequence<Int[0, 9]> = sequence[7, 8, 7] in tally(q) = 3 }"#;
    let parsed = fsl_cst::parse(
        identity.clone(),
        "../rules/recursion-size-measure-dropfirst-with-fact.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the recursive example unit parses");
    assert_eq!(parsed.diagnostics(), []);

    let raw = parsed.source().reference().clone();
    let unit = fsl_forms::build_unit(&parsed).expect("the recursive example unit builds");
    let graph = PackageDeclarations::assemble(
        raw,
        identity
            .label(fsl_foundation::source::SourceLabel::Identity)
            .to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the recursive example program assembles")
    .check(
        CheckMode::Linked,
        CheckingLimits::default(),
        &fsl_semantics::check::test_support::ZeroRewardEvaluator,
    )
    .expect("the recursive example program checks");
    let emission = emit_checked(&CheckedPackage::link(graph))
        .expect("recursive body and termination-measure applications retain their occurrences");
    let wire = wire(&emission);
    let parameter = nodes(&wire)
        .iter()
        .find(|node| {
            node["node_tag"] == "value"
                && node["semantic_form"] == "parameter"
                && node["body"]["members"][0]["value"]["value"] == "s"
        })
        .expect("tally's s parameter is emitted");
    let size = nodes(&wire)
        .iter()
        .find(|node| {
            node["body"]["term"] == "application"
                && node["body"]["operation"]["identity"] == "filament.op.collection.size"
                && node["dependencies"]
                    .as_array()
                    .is_some_and(|dependencies| dependencies.contains(&parameter["node_id"]))
        })
        .expect("size(s) is one shared Application");
    let size_ordinals: Vec<_> = size["occurrences"]
        .as_array()
        .expect("occurrences are rows")
        .iter()
        .filter(|occurrence| occurrence["role"] == "expression")
        .map(|occurrence| {
            occurrence["ordinal"]
                .as_u64()
                .expect("ordinal is an integer")
        })
        .collect();
    assert_eq!(
        size_ordinals,
        [0, 1],
        "the measure and body each retain their size(s) occurrence"
    );
    let mut size_source_rows: Vec<_> = wire["source_map"]
        .as_array()
        .expect("source map is a list")
        .iter()
        .filter(|row| row["node_id"] == size["node_id"] && row["role"] == "expression")
        .collect();
    size_source_rows.sort_by_key(|row| row["ordinal"].as_u64().expect("ordinal is an integer"));
    assert_eq!(size_source_rows.len(), 2, "one source row per size(s) use");
    let actual_starts: Vec<_> = size_source_rows
        .iter()
        .map(|row| {
            row["regions"][0]["start"]
                .as_u64()
                .expect("a size(s) source span has an integer start")
        })
        .collect();
    let mut source_order_starts: Vec<_> = source
        .match_indices("size(s)")
        .map(|(start, _)| u64::try_from(start).expect("fixture source offset fits u64"))
        .collect();
    source_order_starts.reverse();
    assert_eq!(
        actual_starts, source_order_starts,
        "body traversal assigns ordinal 0 and decreases-measure traversal ordinal 1"
    );
    for name in ["tally", "example"] {
        assert!(
            nodes(&wire).iter().any(|node| {
                node["node_tag"] == "function"
                    && node["declaration"]["qualified_name"] == json!([name])
            }),
            "the emitted package retains function {name}"
        );
    }
    let read = read_v2(
        emission.package.bytes(),
        LibraryName::new("recursive-drop-first")
            .expect("the test-owned identity is a library name"),
        V2ReadLimits::fixture(),
        &read_evidence(&emission),
    );
    assert!(
        matches!(read, Read::Verified { .. }),
        "Contract IR admits the emitted recursive package and source map, got {read:?}"
    );
}
