// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-850 "Equality with the source route", FR-940, FR-124-AC-12 (QSL-2785):
//! a derived deadlock-freedom item of a `native-analyze/1` request that
//! selects a v2 checked package whose unit selects two models settles from
//! the package bytes exactly as the request over the source settles it: over
//! the one model the subject reads it is proved or refuted with a replayed
//! counterexample, and over a subject that reads both there is no anchor, so
//! the item is `unsupported` (FSpec FR-368-AC-22), never a whole-request
//! `not-yet-implemented` refusal.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, label, package_selection, request_members, request_over, Run, Workspace,
};
use crate::hyper_world::{self, Vault, KEY, PACKAGE};

const OTHER: &str = "test/vault-other";

/// The item `{claim, temporal: {item_kind: deadlock-freedom}}` over the
/// model selection `identity`.
fn deadlock_item(identity: &str) -> Value {
    json!({"claim": identity, "temporal": {"item_kind": "deadlock-freedom"}})
}

/// Which populations the initial snapshot (hence the subject) covers.
#[derive(Clone, Copy)]
enum Reads {
    First,
    Second,
    Both,
}

/// Both routes' runs over a unit that selects `test/vault` as `V` and
/// `test/vault-other` as `N`, whose subject reads `reads`, for `items`; the
/// authored unit text is deleted before the package run.
fn both_routes(name: &str, vault: Vault, reads: Reads, items: &Value) -> (Run, Run) {
    let first = hyper_world::document(false, false);
    let other = hyper_world::other_document(&first);
    let unit = hyper_world::unit_text(Some(&other), vault, false, "");
    let mut snapshots = fsl_semantics::model::observation::DocumentProvision::new();
    let at = [(KEY, (0, 0))];
    let (one, two) = match reads {
        Reads::First => (&at[..], None),
        Reads::Second => (&[][..], Some(&at[..])),
        Reads::Both => (&at[..], Some(&at[..])),
    };
    hyper_world::snapshot(&mut snapshots, "s0", one, None, two);
    let snapshot = snapshots.into_values().next().expect("one snapshot");

    let ws = Workspace::new(name);
    ws.write("other.json", &other);
    let mut models = request_members(None)["models"].clone();
    models.as_array_mut().expect("models").push(
        json!({"format": "semantic-ir/2.0.0", "source": {"file": "other.json",
            "authority": "test", "identity": "vault-other-model", "document": "VaultOther",
            "formal_revision": 1}}),
    );
    let path = request_over(
        &ws,
        &unit,
        &first,
        &[snapshot],
        json!({"models": models, "items": items}),
    );
    let source = analyze(&path);
    let request: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("JSON");
    let (members, _) = package_selection(&ws, &request["request"]);
    for file in ["program.fsl"] {
        std::fs::remove_file(ws.path(file)).expect("the authored file is removed");
    }
    let document = json!({"format": "native-analyze/1", "request": members});
    let package = analyze(&ws.write("analyze-package.json", document.to_string().as_bytes()));
    (source, package)
}

/// The package run's whole result equals the source run's.
fn assert_equal(source: &Run, package: &Run) {
    for member in ["items", "aggregate", "last_stage", "category", "registry"] {
        assert_eq!(
            package.document()[member],
            source.document()[member],
            "{member}: {}",
            package.document()
        );
    }
    assert_eq!(package.exit, source.exit, "{}", package.document());
}

/// FR-124-AC-12: a model's reachable deadlock is refuted with the same
/// counterexample and a reproduced replay on both routes, whichever of the
/// unit's two models the subject reads.
#[trace("TC-519", "FR-124-AC-12")]
#[trace("TC-2410", "FR-850-AC-2")]
#[test]
fn a_reachable_deadlock_over_one_of_two_models_refutes_as_the_source_route() {
    for (reads, identity) in [(Reads::First, PACKAGE), (Reads::Second, OTHER)] {
        let (source, package) = both_routes(
            "package-deadlock-two-models-refuted",
            Vault::Terminal,
            reads,
            &json!([deadlock_item(identity)]),
        );
        assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
        assert_eq!(label(&package.item(0)), "refuted", "{}", package.document());
        assert_eq!(package.item(0)["replay"]["reproduced"], true);
        assert_equal(&source, &package);
    }
}

/// FR-124-AC-12: with no reachable deadlock the item is proved on both routes.
#[trace("TC-519", "FR-124-AC-12")]
#[trace("TC-2410", "FR-850-AC-2")]
#[test]
fn a_deadlock_free_model_of_two_proves_as_the_source_route() {
    let (source, package) = both_routes(
        "package-deadlock-two-models-proved",
        Vault::Secure,
        Reads::First,
        &json!([deadlock_item(PACKAGE)]),
    );
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    assert_equal(&source, &package);
}

/// FR-124-AC-12, FSpec FR-368-AC-22: a subject that reads both models has no
/// anchor, so its item settles `unsupported` on both routes with equal results,
/// and the items beside it settle as they do.
#[trace("TC-519", "FR-124-AC-12")]
#[trace("TC-2410", "FR-850-AC-2")]
#[test]
fn a_subject_reading_both_models_settles_unsupported_as_the_source_route() {
    let (source, package) = both_routes(
        "package-deadlock-two-models-both",
        Vault::Terminal,
        Reads::Both,
        &json!([deadlock_item(PACKAGE), deadlock_item(OTHER)]),
    );
    assert_eq!(
        source.item(0)["record"]["diagnostic"]["code"],
        "unsupported_projection",
        "{}",
        source.document()
    );
    assert_equal(&source, &package);
}
