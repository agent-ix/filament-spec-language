// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: per-model subjects in universal package hyper analysis.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, label, package_selection, request_members, request_over, Workspace, HANDOFF,
};

fn two_model_parity(equal: bool) {
    let (expected, exit, operator) = if equal {
        ("proved", 0, "=")
    } else {
        ("refuted", 10, "!=")
    };
    let declaration = format!(
        "hyper Same using inf over behaviours (v: V::Vault, w: N::Vault) \
        forall trace a of V forall trace b of N \
        {{ always holds(v.h @ a {operator} w.h @ b) }}"
    );
    let (unit, model, other, snapshot) =
        crate::hyper_world::two_model_request_files((1, 0), &declaration);
    let ws = Workspace::new(&format!("package-hyper-two-model-{expected}"));
    ws.write("other.json", &other);
    let mut models = request_members(None)["models"].clone();
    models.as_array_mut().expect("models").push(json!({
        "format": "semantic-ir/2.0.0", "source": {"file": "other.json",
            "authority": "test", "identity": "vault-other-model", "document": "VaultOther",
            "formal_revision": 1}
    }));
    let source_path = request_over(
        &ws,
        &unit,
        &model,
        &[snapshot],
        json!({"models": models, "items": [{"claim": "Same"}]}),
    );
    let source = analyze(&source_path);
    assert_eq!(source.exit, exit, "{}", source.document());
    assert_eq!(label(&source.item(0)), expected, "{}", source.document());
    let request: Value =
        serde_json::from_slice(&std::fs::read(&source_path).expect("request bytes"))
            .expect("request JSON");
    let (members, _) = package_selection(&ws, &request["request"]);
    ws.write("program.fsl", b"not a native unit");
    std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
        .expect("remove retained sources");
    let package = analyze(
        &ws.write(
            "package-analyze.json",
            json!({"format": "native-analyze/1", "request": members})
                .to_string()
                .as_bytes(),
        ),
    );
    assert_eq!(package.exit, exit, "{}", package.document());
    for member in ["record", "certificate", "counterexample", "replay"] {
        assert_eq!(
            package.item(0)[member],
            source.item(0)[member],
            "{member}: {}",
            package.document()
        );
    }
    if equal {
        assert_eq!(package.item(0)["certificate"]["accepted"], true);
    } else {
        assert_eq!(package.item(0)["replay"]["reproduced"], true);
    }
}

#[trace("FR-940-AC-17", "FR-940-AC-18", "FR-176-AC-1", "FR-163-AC-1")]
#[test]
fn two_model_package_hyper_proof_matches_source_without_source() {
    two_model_parity(true);
}

#[trace("FR-940-AC-17", "FR-940-AC-18", "FR-176-AC-2", "FR-183-AC-1")]
#[test]
fn two_model_package_hyper_refutation_replays_without_source() {
    two_model_parity(false);
}
