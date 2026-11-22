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
