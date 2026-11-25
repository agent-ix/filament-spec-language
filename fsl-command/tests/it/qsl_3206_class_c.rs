// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-3206: the native-run/provider path emits the complete coll-cart source
//! with the same source selection and identity used by the example request.

use fsl_command::command::{self, RequestOverrides, RunLimits};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const REQUEST: &str =
    include_str!("../fixtures/qsl3206/shop/requests/values/collections/coll-cart.json");
const SOURCE: &str =
    include_str!("../fixtures/qsl3206/shop/rules/values/collections/coll-cart.fsl");

fn copy_example_fixture(directory: &Path) -> PathBuf {
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/qsl3206/shop");
    let request = directory.join("shop/requests/values/collections/coll-cart.json");
    let source = directory.join("shop/rules/values/collections/coll-cart.fsl");
    std::fs::create_dir_all(request.parent().expect("request parent exists"))
        .expect("request directory creates");
    std::fs::create_dir_all(source.parent().expect("source parent exists"))
        .expect("source directory creates");
    std::fs::copy(
        fixture_root.join("requests/values/collections/coll-cart.json"),
        &request,
    )
    .expect("the exact native-run request copies");
    std::fs::copy(
        fixture_root.join("rules/values/collections/coll-cart.fsl"),
        &source,
    )
    .expect("the complete example source copies");
    request
}

/// The exact coll-cart request declares no libraries and one program source.
/// The source contains the full example unit (nine functions calling prices),
/// so the driver-equivalent provider path must admit exactly that one file.
#[test]
#[trace("QSL-3206", "FSpec-FR-322")]
fn driver_provider_run_emits_the_complete_coll_cart_example_unit() {
    let request: Value = serde_json::from_str(REQUEST).expect("the checked-in request is JSON");
    assert_eq!(request["format"], "native-run/1");
    assert_eq!(request["request"]["models"], json!([]));
    assert!(request["request"].get("libraries").is_none());
    assert_eq!(
        request["request"]["program"]["source"]["file"],
        "../../../rules/values/collections/coll-cart.fsl"
    );

    let directory = tempfile::tempdir().expect("temporary example tree creates");
    let request_path = copy_example_fixture(directory.path());
    let engines = vec![
        fsl_analyze::analyze::Engine::hybrid_reference()
            .expect("the driver reference-hybrid provider registers"),
    ];
    let result = command::run_with_providers_and_limits_and_overrides(
        &request_path,
        RunLimits::default(),
        &RequestOverrides::default(),
        &crate::negotiation::Fr290,
        engines,
        &fsl_analyze::provider::ProviderOptions::default(),
        &filament_exact::Cancel::new(),
    )
    .expect("the real coll-cart native-run request completes through the provider-backed run path");

    assert_eq!(result.exit_code, 0, "{}", result.value);
    assert_eq!(
        result.value.as_value()["outcome"]["value"],
        json!({"type": "integer", "value": "3"})
    );
    let admitted = result.sources().as_slice();
    assert_eq!(
        admitted.len(),
        1,
        "the request selects no libraries or other rule files"
    );
    assert_eq!(
        admitted[0].path(),
        "../../../rules/values/collections/coll-cart.fsl"
    );
    assert_eq!(admitted[0].identity().authority, "example");
    assert_eq!(
        admitted[0].identity().identity,
        "example:shop/values/collections/coll-cart"
    );
    assert_eq!(
        admitted[0].text(),
        SOURCE,
        "the entire example unit is loaded"
    );
}
