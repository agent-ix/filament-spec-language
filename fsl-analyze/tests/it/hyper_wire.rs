// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hyperproperty checks through the command adapter and its wire document.

use ix_trace_rs::trace;
use serde_json::json;

use crate::analyze_cli_support::{Workspace, analyze, label, request_over};
use crate::hyper_world::{
    EQUAL_INPUTS, STARTS, Vault, non_interference, opaque, request_files_over_starts,
};

#[trace("TC-1591", "FR-631-AC-1", "FR-163-AC-1")]
#[test]
fn analyze_hyper_holds_with_a_checked_product_certificate() {
    let (unit, model, snapshots) = request_files_over_starts(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let ws = Workspace::new("hyper-wire-holds");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "NonInterference"}]}),
    ));
    assert_eq!(run.exit, 0, "{}", run.document());
    assert_eq!(label(&run.item(0)), "proved");
    assert_eq!(run.item(0)["record"]["certification"], "certified");
    assert!(run.item(0)["counterexample"].is_null());
}

#[trace("TC-1591", "FR-631-AC-1", "FR-177-AC-1", "FR-163-AC-1")]
#[test]
fn analyze_hyper_forall_exists_holds_and_refutes() {
    for (vault, expected, exit) in [(Vault::Secure, "proved", 0), (Vault::Leaky, "refuted", 10)] {
        let (unit, model, snapshots) =
            request_files_over_starts(vault, false, &STARTS, &opaque(""));
        let ws = Workspace::new(&format!("hyper-wire-forall-exists-{expected}"));
        let run = analyze(&request_over(
            &ws,
            &unit,
            &model,
            &snapshots,
            json!({"items": [{"claim": "Opaque"}]}),
        ));
        assert_eq!(run.exit, exit, "{}", run.document());
        assert_eq!(label(&run.item(0)), expected);
        if exit == 10 {
            assert_eq!(run.item(0)["counterexample"]["kind"], "witness-exhausted");
            assert_eq!(run.item(0)["replay"]["reproduced"], true);
        } else {
            assert_eq!(run.item(0)["certificate"]["accepted"], true);
        }
    }
}

#[trace("TC-1591", "FR-631-AC-1", "FR-178-AC-1", "FR-163-AC-1")]
#[test]
fn analyze_hyper_projected_holds_and_refutes() {
    for (vault, expected, exit) in [
        (Vault::Busy, "proved", 0),
        (Vault::BusyLeaky, "refuted", 10),
    ] {
        let (unit, model, snapshots) =
            request_files_over_starts(vault, false, &STARTS, &crate::hyper_projected::aligned(&[]));
        let ws = Workspace::new(&format!("hyper-wire-projected-{expected}"));
        let run = analyze(&request_over(
            &ws,
            &unit,
            &model,
            &snapshots,
            json!({"items": [{"claim": "NonInterference"}]}),
        ));
        assert_eq!(run.exit, exit, "{}", run.document());
        assert_eq!(label(&run.item(0)), expected);
        if exit == 10 {
            assert_eq!(run.item(0)["counterexample"]["kind"], "projected");
            assert_eq!(run.item(0)["replay"]["reproduced"], true);
        } else {
            assert_eq!(run.item(0)["certificate"]["accepted"], true);
        }
    }
}

#[trace("TC-645", "FR-163-AC-1", "FR-163-AC-2", "FR-163-AC-4", "FR-163-AC-5")]
#[test]
fn hyper_product_certificate_is_recomputed_and_rejects_tampering() {
    use crate::hyper_world::World;
    use filament_exact::Cancel;
    use fsl_replay::{
        CertificateLocus, CertificateMember, CertificateRule, HyperOutcome, ModelCheckLimits,
        ProductClosureRequest, TemporalCertificateCheck, check_product_closure,
    };
    let world = World::new(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let cancel = Cancel::new();
    let outcome = world
        .run(ModelCheckLimits::default(), &cancel)
        .expect("secure product search");
    let HyperOutcome::HoldsReduced { certificates, .. } = outcome else {
        panic!("secure copy-swap proof");
    };
    let subjects = vec![world.subject()];
    let request = ProductClosureRequest {
        package: &world.package,
        clause: &world.clause,
        subjects: &subjects,
        meter: crate::counter::UNLIMITED,
    };
    let checked = |proof, limits| {
        check_product_closure(&request, proof, limits, &cancel).expect("core check")
    };
    let proof = certificates.first().expect("one instance certificate");
    let limits = ModelCheckLimits::default();
    assert_eq!(checked(proof, &limits), TemporalCertificateCheck::Accepted);
    assert_eq!(checked(proof, &limits), TemporalCertificateCheck::Accepted);
    let mut malformed = proof.clone();
    malformed.components = None;
    assert!(
        matches!(checked(&malformed, &limits), TemporalCertificateCheck::Rejected(rejection)
        if rejection.rule == CertificateRule::Malformed
            && rejection.at == CertificateLocus::CertificateMember(CertificateMember::Components))
    );
    let mut duplicate = proof.clone();
    duplicate.closure.insert(0, duplicate.closure[0].clone());
    assert!(
        matches!(checked(&duplicate, &limits), TemporalCertificateCheck::Rejected(rejection)
        if rejection.rule == CertificateRule::Malformed)
    );
    let mut missing = proof.clone();
    missing.closure.clear();
    assert!(
        matches!(checked(&missing, &limits), TemporalCertificateCheck::Rejected(rejection)
        if rejection.rule == CertificateRule::InitialMissing)
    );
    let tight = ModelCheckLimits {
        limits: fsl_eval::simulation::Limits {
            max_states: 1,
            ..limits.limits
        },
        ..limits
    };
    assert!(matches!(
        checked(proof, &tight),
        TemporalCertificateCheck::Stopped(_)
    ));
}

#[trace("TC-1591", "FR-631-AC-1", "FR-183-AC-1")]
#[test]
fn analyze_hyper_refuted_with_pair() {
    let (unit, model, snapshots) = request_files_over_starts(
        Vault::Leaky,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let ws = Workspace::new("hyper-wire-refuted");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "NonInterference"}]}),
    ));
    assert_eq!(run.exit, 10, "{}", run.document());
    let item = run.item(0);
    assert_eq!(label(&item), "refuted");
    let found = &item["counterexample"];
    assert_eq!(found["type"], "hyper-counterexample");
    assert_eq!(found["kind"], "lockstep");
    assert_eq!(found["traces"].as_array().map(Vec::len), Some(2));
    assert_eq!(found["traces"][0]["variable"], "a");
    assert_eq!(found["traces"][1]["variable"], "b");
    assert_eq!(item["replay"]["reproduced"], true);
}

#[trace("TC-1591", "FR-631-AC-3", "FR-184-AC-2")]
#[test]
fn analyze_hyper_witness_budget_is_incomplete() {
    let declaration = "hyper Opaque using inf over behaviours (v: V::Vault) \
        forall trace a of V exists trace b of V { always holds(v.l @ a = v.l @ b) }";
    let (unit, model, snapshots) =
        request_files_over_starts(Vault::Secure, false, &STARTS, declaration);
    let ws = Workspace::new("hyper-wire-witness-budget");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Opaque", "temporal": {"limits": {"max_witness_set": 1}}}]}),
    ));
    assert_eq!(label(&run.item(0)), "incomplete", "{}", run.document());
    assert_eq!(run.item(0)["record"]["cause"]["limit"], "max_witness_set");
    assert_eq!(run.exit, 22);
}
