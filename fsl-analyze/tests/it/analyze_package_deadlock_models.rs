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
    analyze, label, package_selection, request_members, request_over, Run, Workspace, HANDOFF,
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
    /// A unit that selects one model, which the subject reads.
    One,
}

/// The source run of a unit that selects `test/vault` as `V` and
/// `test/vault-other` as `N`, whose subject reads `reads`, for `items`, and
/// its package request.
fn world(name: &str, vault: Vault, reads: Reads, terminals: &str, items: &Value) -> World {
    let first = hyper_world::document(false, false);
    let other = hyper_world::other_document(&first);
    let unit = hyper_world::unit_text(
        (!matches!(reads, Reads::One)).then_some(&other[..]),
        vault,
        false,
        terminals,
    );
    let mut snapshots = fsl_semantics::model::observation::DocumentProvision::new();
    let at = [(KEY, (0, 0))];
    let (one, two) = match reads {
        Reads::First | Reads::One => (&at[..], None),
        Reads::Second => (&[][..], Some(&at[..])),
        Reads::Both => (&at[..], Some(&at[..])),
    };
    hyper_world::snapshot(&mut snapshots, "s0", one, None, two);
    let mut snapshot = snapshots.into_values().next().expect("one snapshot");
    if matches!(reads, Reads::Second) {
        // The subject reads `test/vault-other` alone: its snapshot holds no
        // population of `test/vault`.
        let mut body: Value = serde_json::from_slice(&snapshot).expect("snapshot JSON");
        body["populations"]
            .as_array_mut()
            .expect("populations")
            .remove(0);
        snapshot = body.to_string().into_bytes();
    }

    let ws = Workspace::new(name);
    ws.write("other.json", &other);
    let mut models = request_members(None)["models"].clone();
    if !matches!(reads, Reads::One) {
        models.as_array_mut().expect("models").push(
            json!({"format": "semantic-ir/2.0.0", "source": {"file": "other.json",
            "authority": "test", "identity": "vault-other-model", "document": "VaultOther",
            "formal_revision": 1}}),
        );
    }
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
    std::fs::remove_file(ws.path("program.fsl")).expect("the authored unit is removed");
    World {
        ws,
        source,
        members,
    }
}

/// A unit's source run, its compiled package request and its workspace.
struct World {
    ws: Workspace,
    source: Run,
    members: Value,
}

impl World {
    fn package_run(&self) -> Run {
        let document = json!({"format": "native-analyze/1", "request": self.members});
        analyze(
            &self
                .ws
                .write("analyze-package.json", document.to_string().as_bytes()),
        )
    }
}

/// Both routes' runs over `world`, the authored unit text deleted before the
/// package run.
fn both_routes(
    name: &str,
    vault: Vault,
    reads: Reads,
    terminals: &str,
    items: &Value,
) -> (Run, Run) {
    let world = world(name, vault, reads, terminals, items);
    let package = world.package_run();
    (world.source, package)
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
/// counterexample and a reproduced replay on both routes when the subject
/// reads the first of the unit's two models.
#[trace("TC-519", "FR-124-AC-12")]
#[trace("TC-2410", "FR-850-AC-2")]
#[test]
fn a_reachable_deadlock_over_one_of_two_models_refutes_as_the_source_route() {
    let (source, package) = both_routes(
        "package-deadlock-two-models-refuted",
        Vault::Terminal,
        Reads::First,
        "",
        &json!([deadlock_item(PACKAGE)]),
    );
    assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
    assert_eq!(label(&package.item(0)), "refuted", "{}", package.document());
    assert_eq!(package.item(0)["replay"]["reproduced"], true);
    assert_equal(&source, &package);
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
        "",
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
        "",
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

/// A subject that reads the second model alone settles as the source route
/// settles it (the source route declines `invalid_model_binding` for want of
/// per-model snapshot provisions), so the package route adds no outcome.
#[trace("TC-2410", "FR-850-AC-2")]
#[test]
fn a_subject_reading_the_second_model_alone_settles_as_the_source_route() {
    let (source, package) = both_routes(
        "package-deadlock-two-models-second",
        Vault::Terminal,
        Reads::Second,
        "",
        &json!([deadlock_item(OTHER)]),
    );
    assert_equal(&source, &package);
}

/// FR-850-AC-3 (negative): a package that lacks the rows of one of its two
/// models is refused `invalid_package` before any item resolves, with no
/// internal failure.
#[trace("TC-2410", "FR-850-AC-3")]
#[test]
fn a_package_missing_one_models_rows_refuses_invalid_package() {
    let world = world(
        "package-deadlock-two-models-missing",
        Vault::Terminal,
        Reads::First,
        "",
        &json!([deadlock_item(PACKAGE)]),
    );
    let path = world.ws.path(&format!("{HANDOFF}/checked-package.json"));
    let mut package: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("the package")).expect("JSON");
    let mut removed = 0;
    for rows in package
        .as_object_mut()
        .expect("a package object")
        .values_mut()
    {
        if let Some(rows) = rows.as_array_mut() {
            let before = rows.len();
            rows.retain(|row| !row.to_string().contains("vault-other"));
            removed += before - rows.len();
        }
    }
    assert!(removed > 0, "the package carries rows of the second model");
    std::fs::write(&path, package.to_string()).expect("the package is written");
    let run = world.package_run();
    let diagnostic = run.document()["diagnostics"][0].clone();
    assert_eq!(diagnostic["code"], "invalid_package", "{}", run.document());
    assert_eq!(run.document()["items"], json!([]));
    assert_eq!(run.exit, 20);
}

/// FR-124: a subject that reads a model declaring `terminal any` has no
/// deadlock-freedom item (`inapplicable-kind`) on both routes, whether the
/// unit selects one model or two; a `terminal any` of a model the subject
/// does not read changes nothing. The item naming a model alias settles as
/// on the source route.
#[trace("TC-519", "FR-124-AC-12")]
#[trace("TC-2410", "FR-850-AC-2")]
#[test]
fn a_terminal_any_of_the_read_model_opts_out_as_the_source_route() {
    let item = json!([deadlock_item(PACKAGE)]);
    for (name, reads, terminals, expected) in [
        (
            "package-deadlock-any-read",
            Reads::First,
            "terminal Halt using v on V any;\n",
            "inapplicable-kind",
        ),
        (
            "package-deadlock-any-other",
            Reads::First,
            "terminal Halt using v on N any;\n",
            "refuted",
        ),
        (
            "package-deadlock-any-one",
            Reads::One,
            "terminal Halt using v on V any;\n",
            "inapplicable-kind",
        ),
    ] {
        let (source, package) = both_routes(name, Vault::Terminal, reads, terminals, &item);
        let settled = |run: &Run| {
            run.item(0)["record"]["diagnostic"]["cause"]
                .as_str()
                .map_or_else(|| label(&run.item(0)).to_owned(), str::to_owned)
        };
        assert_eq!(settled(&source), expected, "{name}: {}", source.document());
        assert_eq!(
            settled(&package),
            expected,
            "{name}: {}",
            package.document()
        );
        assert!(
            package.item(0)["request"].is_null() == (expected != "refuted"),
            "{name}: {}",
            package.document()
        );
        assert_equal(&source, &package);
    }
    let (source, package) = both_routes(
        "package-deadlock-alias",
        Vault::Terminal,
        Reads::First,
        "",
        &json!([deadlock_item("V")]),
    );
    assert_equal(&source, &package);
}
