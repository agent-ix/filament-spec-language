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

/// A population belonging to neither selected model cannot disappear while
/// projecting a hyper subject. The universe stays clean so snapshot admission
/// must inspect the foreign population in the original initial bytes.
#[trace("FR-940-AC-17", "FR-179-AC-1", "FR-106-AC-3")]
#[test]
fn two_model_hyper_foreign_population_is_refused() {
    for foreign in ["ix://outside/vaults", "ix://test/vault/not-declared"] {
        let declaration = "hyper Same using inf over behaviours (v: V::Vault, w: N::Vault) \
            forall trace a of V forall trace b of N { always holds(v.h @ a = w.h @ b) }";
        let (unit, model, other, snapshot) =
            crate::hyper_world::two_model_request_files((1, 0), declaration);
        let ws = Workspace::new("package-hyper-two-model-foreign");
        ws.write("other.json", &other);
        let mut models = request_members(None)["models"].clone();
        models.as_array_mut().expect("models").push(json!({
            "format": "semantic-ir/2.0.0", "source": {"file": "other.json",
                "authority": "test", "identity": "vault-other-model", "document": "VaultOther",
                "formal_revision": 1}
        }));
        let path = request_over(
            &ws,
            &unit,
            &model,
            &[snapshot.clone()],
            json!({"models": models, "items": [{"claim": "Same"}]}),
        );
        let mut request: Value =
            serde_json::from_slice(&std::fs::read(path).expect("request bytes"))
                .expect("request JSON");
        let mut initial: Value = serde_json::from_slice(&snapshot).expect("snapshot JSON");
        let mut universe = initial.clone();
        universe["identity"]["identity"] = json!("scope-universe");
        ws.write("scope-universe.json", universe.to_string().as_bytes());
        let mut extra = initial["populations"][0].clone();
        extra["population"] = json!(foreign);
        initial["populations"]
            .as_array_mut()
            .expect("populations")
            .push(extra);
        ws.write("snapshot-0.json", initial.to_string().as_bytes());
        request["request"]["documents"]
            .as_array_mut()
            .expect("documents")
            .push(json!({"file": "scope-universe.json"}));
        request["request"]["subject"] = json!({
            "initial_states": [{"authority": "test", "identity": "s0"}],
            "universes": [{"authority": "test", "identity": "scope-universe"}],
            "proof_bounds": []
        });
        let (members, _) = package_selection(&ws, &request["request"]);
        let source = analyze(&ws.write("foreign-source.json", request.to_string().as_bytes()));
        ws.write("program.fsl", b"not a native unit");
        std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
            .expect("remove retained sources");
        let package = analyze(
            &ws.write(
                "foreign-package.json",
                json!({"format": "native-analyze/1", "request": members})
                    .to_string()
                    .as_bytes(),
            ),
        );
        for run in [&source, &package] {
            let document = run.document();
            assert_eq!(document["category"], "refusal", "{document}");
            let diagnostics: Vec<_> = document["diagnostics"]
                .as_array()
                .expect("diagnostics")
                .iter()
                .chain(
                    document["items"]
                        .as_array()
                        .expect("items")
                        .iter()
                        .filter_map(|item| item["record"].get("diagnostic")),
                )
                .collect();
            assert_eq!(diagnostics.len(), 1, "{document}");
            assert_eq!(
                diagnostics[0]["code"], "invalid_runtime_input",
                "{document}"
            );
            assert_eq!(diagnostics[0]["cause"], "wrong-role-mapping", "{document}");
            let detail = diagnostics[0]
                .get("detail")
                .or_else(|| diagnostics[0].get("details"))
                .expect("native refusal details");
            assert_eq!(detail["population"], foreign, "{document}");
        }
    }
}
