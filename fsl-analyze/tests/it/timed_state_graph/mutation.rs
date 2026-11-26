// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 mutation registrations for timed state-graph zone certificates.

use super::*;
use filament_exact::{Cancel, ValueType};
use fsl_certificate_mutation::{Fixture, Verdict, check_document};
use fsl_replay::{
    CertificateCheckLimits, CertifiedItem, StateGraphCertificateForm, StateGraphPart,
    ZoneCertificate, ZoneCertificateRequest, ZoneCertificateVerdict,
};
use fsl_route::deadlock::{ItemObligation, ObligationKind};
use ix_trace_rs::trace;

/// Trace: FR-1051-AC-11, FR-1051-AC-13.
#[trace("FR-1051-AC-11", "FR-1051-AC-13")]
#[test]
fn always_possible_zone_certificate_rejects_single_mutations() {
    check_zone_state_graph_fixture("BackToIdle", true, "R-27 always-possible");
}

/// Trace: FR-1051-AC-11, FR-1051-AC-13.
#[trace("FR-1051-AC-11", "FR-1051-AC-13")]
#[test]
fn unique_path_zone_certificate_rejects_single_mutations() {
    check_zone_state_graph_fixture("OneReply", false, "R-28 unique-path");
}

fn check_zone_state_graph_fixture(claim_name: &str, reset: bool, route: &str) {
    let items = json!([{"claim": claim_name}]);
    let run = super::run(&format!("mutation-{claim_name}"), reset, items);
    let item = run.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["certificate"]["accepted"], true, "{item}");
    let document = item["certificate"]["document"].clone();

    let text = super::unit(reset);
    let marker = "model M = \"test/timed\";\n";
    let (_, body) = text.split_once(marker).expect("the fixture's model header");
    let world = crate::timed_world::world(
        &crate::zone_search::rpc_document(),
        crate::zone_search::TIMED,
        body,
    );
    let claim = world
        .package
        .graph()
        .state_graph_claim(claim_name)
        .expect("the checked state-graph claim");
    let occurrence = claim
        .occurrence()
        .expect("authored claim occurrence")
        .clone();
    let obligation = ItemObligation {
        subject: (),
        kind: ObligationKind::StateGraphAuthored(occurrence),
        reductions: Default::default(),
    };
    let system = world.system();
    let over_parameter = claim.over().expect("the claim binds the call object");
    let instance = system
        .instances(&ValueType::Reference(over_parameter.object.clone()))
        .expect("the admitted instances")
        .into_iter()
        .next()
        .expect("one call instance");
    let over = system
        .written_reference(over_parameter.object.clone(), &instance)
        .expect("the instance reference");
    let mut base_document = document.clone();
    base_document
        .as_object_mut()
        .expect("the certificate is an object")
        .remove("state_graph");
    let (mut baseline_certificate, baseline_identity) =
        ZoneCertificate::from_wire(&base_document, obligation.clone())
            .expect("the route's codec reads its accepted certificate");
    baseline_certificate.state_graph = Some(state_graph_part(&document["state_graph"]));
    assert_eq!(
        fsl_replay::check_zone_certificate(
            ZoneCertificateRequest {
                simulation: world.request(),
                model: "M",
                max_clocks: fsl_eval::simulation::model::DEFAULT_MAX_CLOCKS,
                item: CertifiedItem::StateGraph {
                    claim,
                    over: Some(&over),
                },
            },
            &obligation,
            fsl_analyze::zone_check::certificate_item,
            &baseline_certificate,
            CertificateCheckLimits::default(),
            &Cancel::new(),
        )
        .expect("the baseline typed certificate checks"),
        ZoneCertificateVerdict::Accepted
    );

    let changes = check_document(
        Fixture {
            route,
            name: claim_name,
            document,
            allowed_to_survive: Vec::new(),
        },
        |bytes| {
            let changed: Value = serde_json::from_slice(bytes).expect("canonical zone certificate");
            let mut base = changed.clone();
            let Some(state_graph) = base
                .as_object_mut()
                .and_then(|object| object.remove("state_graph"))
            else {
                return Verdict::Rejected;
            };
            let Ok((mut certificate, identity)) =
                ZoneCertificate::from_wire(&base, obligation.clone())
            else {
                return Verdict::Rejected;
            };
            certificate.state_graph = Some(state_graph_part(&state_graph));
            if identity != baseline_identity {
                return Verdict::Rejected;
            }
            match fsl_replay::check_zone_certificate(
                ZoneCertificateRequest {
                    simulation: world.request(),
                    model: "M",
                    max_clocks: fsl_eval::simulation::model::DEFAULT_MAX_CLOCKS,
                    item: CertifiedItem::StateGraph {
                        claim,
                        over: Some(&over),
                    },
                },
                &obligation,
                fsl_analyze::zone_check::certificate_item,
                &certificate,
                CertificateCheckLimits::default(),
                &Cancel::new(),
            ) {
                Ok(ZoneCertificateVerdict::Accepted) => Verdict::Accepted,
                _ => Verdict::Rejected,
            }
        },
    )
    .expect("all unlisted zone-certificate mutations reject");
    assert!(changes > 0, "the zone certificate has generated mutations");
}

fn state_graph_part(value: &Value) -> StateGraphPart {
    let form = match value["form"].as_str().expect("state-graph form") {
        "possible" => StateGraphCertificateForm::Possible,
        "always-possible" => StateGraphCertificateForm::AlwaysPossible,
        "unique-path" => StateGraphCertificateForm::UniquePath,
        other => panic!("unexpected state-graph form {other}"),
    };
    let ef_nodes = value["ef_nodes"]
        .as_array()
        .expect("E(P) node positions")
        .iter()
        .map(|node| {
            usize::try_from(node.as_u64().expect("node position"))
                .expect("node position fits usize")
        })
        .collect();
    StateGraphPart { form, ef_nodes }
}
