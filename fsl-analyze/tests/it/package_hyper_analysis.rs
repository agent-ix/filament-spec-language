// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: universal infinite-trace analysis from package facts alone.

use fsl_package::{AnalysisView, BarePackage, BareSupply, ReadEqualityRows, ReadIdentityRows};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, counter_request, label, package_selection, request_members, Workspace, HANDOFF,
};

fn universal_parity(bound: u64, expected: &str) {
    let ws = Workspace::new(&format!("package-universal-{expected}"));
    let unit = format!("hyper Equal using t0 over behaviours (c: M::Counter) forall trace a of M forall trace b of M {{ always holds(c.value @ a <= {bound} and c.value @ b <= {bound}) }}");
    let items = json!([{"claim": "Equal"}]);
    let source = analyze(&counter_request(&ws, &unit, items.clone()));
    assert_eq!(label(&source.item(0)), expected, "{}", source.document());
    let (members, bytes) = package_selection(&ws, &request_members(Some(items)));
    let model = std::fs::read(ws.path("model.json")).expect("domain fixture");
    let models =
        fsl_semantics::model::intake::package_input([model.as_slice()]).expect("domain admits");
    let limits = fsl_package::V2ReadLimits::bounded(1 << 24);
    let admitted =
        fsl_package::admit_checked_package(&bytes, &models, limits).expect("package admits");
    let mut budget = limits.native_join_limits().budget();
    let identities = ReadIdentityRows::read(
        &std::fs::read(ws.path(&format!("{HANDOFF}/identities.json"))).expect("identity rows"),
        &admitted,
        &mut budget,
    )
    .expect("identities admit");
    let equality = std::fs::read(ws.path(&format!("{HANDOFF}/equality.json")))
        .ok()
        .map(|bytes| ReadEqualityRows::read(&bytes, &admitted).expect("equality admits"));
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: equality.as_ref(),
            },
            &mut budget,
        )
        .expect("supply joins");
    let selected = bare.claim_name("Equal").expect("named hyper claim");
    let occurrence = fsl_foundation::source::provenance::OccurrenceKey::new(
        bare.node_key(&selected.declaration)
            .expect("structural claim key"),
        filament_exact::Origin::new(
            filament_exact::Role::new("claim"),
            selected.occurrence.ordinal,
        ),
    );
    let form = bare
        .hyper_clause_form(&occurrence)
        .expect("existing universal package form is readable");
    let rebuilt = fsl_semantics::check::CheckedHyperClause::of_package_form(
        form,
        bare.world().expect("package world").types(),
        1_000_000,
    )
    .expect("existing source-free constructor accepts Equal");
    assert_eq!(
        rebuilt.form(),
        Some(&fsl_semantics::check::HyperForm::Universal)
    );
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
    let document: Value = package.document();
    assert_eq!(package.exit, source.exit, "{document}");
    assert_eq!(package.item(0)["record"]["result"], expected, "{document}");
    for member in ["record", "certificate", "counterexample", "replay"] {
        assert_eq!(
            package.item(0)[member],
            source.item(0)[member],
            "{member}: {document}"
        );
    }
    if expected == "proved" {
        assert_eq!(
            package.item(0)["certificate"]["accepted"],
            true,
            "{document}"
        );
    } else {
        assert_eq!(package.item(0)["replay"]["reproduced"], true, "{document}");
    }
}

#[trace("FR-940-AC-17", "FR-940-AC-18", "FR-176-AC-1")]
#[test]
fn universal_package_proof_matches_source_without_source() {
    universal_parity(3, "proved");
}

#[trace("FR-940-AC-17", "FR-940-AC-18", "FR-176-AC-2")]
#[test]
fn universal_package_refutation_replays_without_source() {
    universal_parity(2, "refuted");
}

/// Prepare genuine inputs for the separately run production-driver smoke
/// check. The driver must create the handoff itself from these originals.
#[trace("FR-940-AC-24", "FR-631-AC-1")]
#[test]
fn prepare_hp1_production_cli_fixture() {
    let ws = Workspace::new("hp1-production-cli-inputs");
    let (unit, model, snapshots) = crate::hyper_world::request_files_over_starts(
        crate::hyper_world::Vault::Secure,
        false,
        &crate::hyper_world::STARTS,
        crate::relation_claims::DETERMINISTIC,
    );
    let path = crate::analyze_cli_support::request_over(
        &ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Det", "kind": "temporal-satisfaction", "backend": "en-1"}]}),
    );
    assert_eq!(
        snapshots.len(),
        2,
        "the CLI command block copies both initial snapshots"
    );
    let source = analyze(&path);
    assert_eq!(source.exit, 0, "{}", source.document());
    assert_eq!(source.item(0)["record"]["result"], "proved");
    assert_eq!(source.item(0)["record"]["certification"], "certified");
    assert_eq!(source.item(0)["certificate"]["accepted"], true);
    let source_request: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("source request"))
            .expect("request JSON");
    ws.write("analyze-source.json", source_request.to_string().as_bytes());
    let request = &source_request["request"];
    ws.write(
        "compile.json",
        json!({"format": "native-compile/1", "request": {
            "models": request["models"], "program": request["program"],
            "handoff": {"directory": "handoff", "package_name": "analyze-unit",
                "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}}
        }})
        .to_string()
        .as_bytes(),
    );
    let mut package = source_request;
    package["request"]
        .as_object_mut()
        .expect("request members")
        .remove("program");
    package["request"]["package"] = json!({"directory": "handoff"});
    ws.write("analyze-package.json", package.to_string().as_bytes());
    assert!(
        !ws.path(HANDOFF).exists(),
        "only the production CLI creates this acceptance handoff"
    );
    println!("HP1_FIXTURE={}", ws.path(".").display());
}
