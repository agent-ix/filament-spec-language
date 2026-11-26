// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1051 mutation registrations for R-27 and R-28. These routes carry
//! `ZoneCertificate` values through a typed adapter because the zone wire
//! reader does not read the `state_graph` member.

use filament_exact::{Cancel, ValueType};
use fsl_certificate_mutation::{Verdict, check_typed_with_adapter};
use fsl_eval::simulation::model::{DEFAULT_MAX_CLOCKS, TimedSystem};
use fsl_replay::{
    CertificateCheckLimits, CertifiedItem, StateGraphCertificateForm, StateGraphPart,
    ZoneCertificate, ZoneCertificateRequest, ZoneCertificateVerdict, check_zone_certificate,
};
use fsl_route::deadlock::{ItemObligation, ObligationKind};
use fsl_semantics::check::StateGraphForm;
use ix_trace_rs::trace;
use serde_json::{Value, json};

const TIMED: &str = "profile tt = \"filament.temporal.timed/v1\";\n";
const CLAIMS: &str = "possible ReplyPossible using v over (c: M::Call) { c.phase = 2 }\n\
    always possible BackToIdle using v over (c: M::Call) { c.phase = 0 }\n\
    unique path OneReply using v over (c: M::Call) from (c.phase = 0) to (c.phase = 2);\n";

#[derive(Clone)]
struct AdapterCertificate {
    certificate: ZoneCertificate<ItemObligation<()>>,
    malformed: bool,
}

fn body(reset: bool) -> String {
    let reset_pre = if reset {
        "not self.inflight and (self.phase = 2 or self.phase = 3)"
    } else {
        "false"
    };
    format!(
        "{}\
         pre SendIdle using v on M::Call::send {{ self.phase = 0 }}\n\
         post Send using v on M::Call::send {{ self.phase = 1 and self.inflight and not self.late and self.x = 0 ms }}\n\
         pre Reply using v on M::Call::reply {{ self.inflight and self.x >= 1 ms }}\n\
         post Replied using v on M::Call::reply {{ not self.inflight and self.late = (pre(self.phase) = 3) and (pre(self.phase) = 1 implies self.phase = 2) and (pre(self.phase) != 1 implies self.phase = pre(self.phase)) }}\n\
         pre Timeout using v on M::Call::timeout {{ self.phase = 1 and self.x >= 4 ms }}\n\
         post TimedOut using v on M::Call::timeout {{ self.phase = 3 }}\n\
         pre Reset using v on M::Call::reset {{ {reset_pre} }}\n\
         post ResetIdle using v on M::Call::reset {{ self.phase = 0 }}\n\
         time invariant ServerDeadline when {{ self.inflight }} {{ self.x <= 3 ms }}\n\
         time invariant ClientDeadline when {{ self.phase = 1 }} {{ self.x <= 4 ms }}\n\
         {CLAIMS}",
        crate::timed_world::DENSE
    )
}

/// Trace: FR-1051-AC-11, FR-1051-AC-13.
#[trace("FR-1051-AC-11", "FR-1051-AC-13")]
#[test]
fn r27_always_possible_zone_certificate_rejects_single_mutations() {
    check_fixture("BackToIdle", "R-27 always possible");
}

/// Trace: FR-1051-AC-11, FR-1051-AC-13.
#[trace("FR-1051-AC-11", "FR-1051-AC-13")]
#[test]
fn r28_unique_path_zone_certificate_rejects_single_mutations() {
    check_fixture("OneReply", "R-28 unique path");
}

fn check_fixture(claim_name: &str, route: &str) {
    let document = crate::timed_world::document(
        &["x"],
        &[
            ("send", &["phase", "inflight", "late", "x"]),
            ("reply", &["phase", "inflight", "late"]),
            ("timeout", &["phase"]),
            ("reset", &["phase"]),
        ],
    );
    let world = crate::timed_world::world(&document, TIMED, &body(claim_name == "BackToIdle"));
    let system = world.system();
    let timed = TimedSystem::with_max_clocks(&system, "M", DEFAULT_MAX_CLOCKS)
        .expect("the fixture admits the timed model");
    let claim = world
        .package
        .graph()
        .state_graph_claim(claim_name)
        .expect("the named authored state-graph claim");
    let occurrence = claim
        .occurrence()
        .expect("authored state-graph occurrence")
        .clone();
    let parameter = claim.over().expect("the claim binds its call object");
    let instance = system
        .instances(&ValueType::Reference(parameter.object.clone()))
        .expect("the fixture's call instance")
        .into_iter()
        .next()
        .expect("one call instance");
    let over = system
        .written_reference(parameter.object.clone(), &instance)
        .expect("the written call reference");
    let graph = crate::zone_check::explore_timed_state_graph(
        &timed,
        crate::zone_check::ZoneCheckLimits::default(),
        &Cancel::new(),
    )
    .expect("the timed state graph is explored");
    let decision = crate::zone_check::decide_timed_state_graph(
        &system,
        &graph,
        claim,
        Some(&over),
        &Cancel::new(),
    )
    .expect("the timed state graph claim is decided");
    let crate::zone_check::TimedStateGraphDecision::Holds { ef_nodes, .. } = decision else {
        panic!("fixture claim {claim_name} must hold");
    };
    let form = match claim.form() {
        StateGraphForm::AlwaysPossible { .. } => StateGraphCertificateForm::AlwaysPossible,
        StateGraphForm::UniquePath { .. } => StateGraphCertificateForm::UniquePath,
        StateGraphForm::Possible { .. } => StateGraphCertificateForm::Possible,
    };
    let obligation = ItemObligation {
        subject: (),
        kind: ObligationKind::StateGraphAuthored(occurrence.clone()),
        reductions: Default::default(),
    };
    let certificate = crate::zone_check::state_graph_certificate(
        &timed,
        &graph,
        obligation.clone(),
        form,
        &ef_nodes,
    )
    .expect("the typed zone certificate is built");
    let identity = fsl_replay::certificate_identity_wire(&occurrence, "M")
        .expect("the certificate identity is encoded");
    let fixture = AdapterCertificate {
        certificate,
        malformed: false,
    };

    let mutations = check_typed_with_adapter(
        route,
        claim_name,
        &fixture,
        Vec::new(),
        |adapter| {
            let mut base = adapter.certificate.clone();
            let state_graph = base.state_graph.take();
            let mut document = base.to_wire_with_source(identity.clone(), &timed).ok()?;
            if let Some(part) = state_graph {
                document.as_object_mut()?.insert(
                    "state_graph".into(),
                    json!({
                        "form": part.form.as_str(),
                        "ef_nodes": part.ef_nodes,
                    }),
                );
            }
            Some(document)
        },
        |document| {
            let decoded = (|| {
                let mut base = document.clone();
                let state_graph = match base.as_object_mut()?.remove("state_graph") {
                    Some(value) => Some(read_state_graph(value)?),
                    None => None,
                };
                let (mut certificate, found_identity) =
                    ZoneCertificate::from_wire_with_source(&base, obligation.clone(), &timed)
                        .ok()?;
                if found_identity != identity {
                    return None;
                }
                certificate.state_graph = state_graph;
                Some(certificate)
            })();
            Some(match decoded {
                Some(certificate) => AdapterCertificate {
                    certificate,
                    malformed: false,
                },
                None => AdapterCertificate {
                    certificate: fixture.certificate.clone(),
                    malformed: true,
                },
            })
        },
        |mutated| {
            if mutated.malformed {
                return Verdict::Rejected;
            }
            match check_zone_certificate(
                ZoneCertificateRequest {
                    simulation: world.request(),
                    model: "M",
                    max_clocks: DEFAULT_MAX_CLOCKS,
                    item: CertifiedItem::StateGraph {
                        claim,
                        over: Some(&over),
                    },
                },
                &obligation,
                crate::zone_check::certificate_item,
                &mutated.certificate,
                CertificateCheckLimits::default(),
                &Cancel::new(),
            ) {
                Ok(ZoneCertificateVerdict::Accepted) => Verdict::Accepted,
                _ => Verdict::Rejected,
            }
        },
    )
    .expect("all unlisted typed mutations reject through check_zone_certificate");
    assert!(
        mutations > 0,
        "the zone certificate has generated mutations"
    );
}

fn read_state_graph(value: Value) -> Option<StateGraphPart> {
    let object = value.as_object()?;
    let form = match object.get("form")?.as_str()? {
        "possible" => StateGraphCertificateForm::Possible,
        "always-possible" => StateGraphCertificateForm::AlwaysPossible,
        "unique-path" => StateGraphCertificateForm::UniquePath,
        _ => return None,
    };
    let ef_nodes = object
        .get("ef_nodes")?
        .as_array()?
        .iter()
        .map(|node| usize::try_from(node.as_u64()?).ok())
        .collect::<Option<Vec<_>>>()?;
    Some(StateGraphPart { form, ef_nodes })
}
