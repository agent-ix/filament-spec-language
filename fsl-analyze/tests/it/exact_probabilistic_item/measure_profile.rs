// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-3152: finite-horizon products, independent payoff/rank checking and limits.
use super::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde_json::{json, Value};

fn retry(claims: &str) -> prob_world::Model {
    prob_world::Model {
        package:"test/retry",object:"Msg",population:"msgs",key:"m",
        integers:vec![("Lost",0,1)],
        fields:vec![("delivered",prob_world::BOOLEAN.into())],
        operations:["fast","slow"].into_iter().map(|name| prob_world::Operation { name,parameters:vec![("lost","ix://test/retry/Lost".into())],modifies:vec!["delivered"],deletes:false }).collect(),
        body:format!("pre Fast using v on M::Msg::fast {{ not self.delivered }}\npost FastDone using v on M::Msg::fast {{ self.delivered = (lost = 1) }}\npre Slow using v on M::Msg::slow {{ not self.delivered }}\npost SlowDone using v on M::Msg::slow {{ self.delivered = (lost = 1) }}\nrandom Msg::fast(lost) ~ {{ 0:1,1:1 }};\nrandom Msg::slow(lost) ~ {{ 0:3,1:1 }};\nworkload Steady on M {{ weight Msg::fast=1; weight Msg::slow=0; }}\nterminal Done using v on M when {{ self.delivered }};\n{claims}"),
        initial:json!({"delivered":false}),
    }
}
fn claim(name: &str, scheduler: &str, body: &str) -> String {
    format!("probabilistic {name} using ev on M over (m:M::Msg) under {scheduler} on origin {{ {body} }}\n")
}
fn steps() -> &'static str {
    "steps from holds(true) until holds(m.delivered) within 4"
}
fn proof(world: &World, name: &str, expected: Rational) -> Value {
    let record = settle(world, name, ModelCheckLimits::default());
    let terminal = record
        .terminal()
        .unwrap_or_else(|| panic!("expected proof: {:?}", record.settlement));
    assert!(
        matches!(terminal.value(), TerminalValue::Proved { .. }),
        "{:?}",
        terminal.value()
    );
    assert_eq!(record.exact_probabilistic.as_ref().unwrap().value, expected);
    serde_json::from_slice(
        terminal
            .exact_certificate_document()
            .expect("a checked document"),
    )
    .unwrap()
}
fn check(
    world: &World,
    name: &str,
    document: &Value,
) -> Result<ExactProbabilityCertificateCheck, fsl_replay::ProbabilityDocumentError> {
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
    fsl_replay::check_probability_certificate_document(
        &system,
        world.package.graph().probabilistic_model().claims(),
        &item,
        &serde_json::to_vec(document).unwrap(),
        fsl_replay::ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes: 8_000_000,
            states: 128,
            edges: 1024,
            scalar_bits: 128,
            accounting: fsl_replay::AccountingLimits::default().0,
        },
        &Cancel::new(),
    )
}
fn key(row: &Value) -> Value {
    serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(row["state"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap()
}
fn refuted(world: &World, name: &str) {
    let record = settle(world, name, ModelCheckLimits::default());
    let terminal = record
        .terminal()
        .unwrap_or_else(|| panic!("expected refutation: {:?}", record.settlement));
    assert_eq!(
        terminal.value(),
        &TerminalValue::Refuted,
        "{:?}",
        terminal.value()
    );
    assert!(record.probabilistic_counterexample.is_some());
}

#[trace("TC-3152", "FR-1043-AC-1")]
#[test]
fn workload_quantile_has_independently_checked_numerator_denominator_and_ranks() {
    let world = World::new(&retry(
        &(claim(
            "Q",
            "Steady",
            &format!("quantile 0.5 of ({}) <= 2", steps()),
        ) + &claim(
            "No",
            "Steady",
            &format!("quantile 0.8 of ({}) <= 2", steps()),
        )),
    ));
    let doc = proof(&world, "Q", ratio(3, 4));
    assert_eq!(doc["kind"], "conditional");
    assert_eq!(doc["conditional"]["mode"], "ratio");
    let numerator = &doc["conditional"]["numerator"];
    let denominator = &doc["conditional"]["denominator"];
    assert_eq!(numerator["objective"]["payoff"], "satisfied");
    assert_eq!(denominator["objective"]["payoff"], "activated");
    for (remaining, amount, value) in [
        (4, "0", ratio(3, 4)),
        (3, "1", ratio(1, 2)),
        (2, "2", ratio(0, 1)),
    ] {
        let row = numerator["values"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                let key = key(row);
                key[5][1] == remaining.to_string() && key[5][2] == "active" && key[6][2] == amount
            })
            .expect("an active amount row");
        assert_eq!(
            row["value"],
            json!({"numerator":value.numerator().to_string(),"denominator":value.denominator().to_string()})
        );
        assert!(denominator["values"]
            .as_array()
            .unwrap()
            .iter()
            .any(|other| other["state"] == row["state"]
                && other["value"] == json!({"numerator":"1","denominator":"1"})));
    }
    let accepted = check(&world, "Q", &doc);
    assert!(
        matches!(accepted, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
        "{accepted:?}"
    );
    let mut altered = doc.clone();
    let ranks = altered["conditional"]["numerator"]["ranking"]
        .as_array_mut()
        .unwrap();
    let row = ranks.iter_mut().find(|row| row["rank"] == 3).unwrap();
    row["rank"] = json!(4);
    let checked = check(&world, "Q", &altered).unwrap();
    assert!(
        matches!(
            checked,
            ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::RankBroken,
                at: ProbabilityCertificateLocus::ProductPart {
                    part: fsl_replay::ProbabilityCertificatePart::Numerator,
                    ..
                }
            })
        ),
        "{checked:?}"
    );
    refuted(&world, "No");
}

#[trace("TC-3152", "FR-1043-AC-2", "FR-1043-AC-3")]
#[test]
fn every_scheduler_checks_shifted_extrema_and_the_strict_zero_edge() {
    let claims = claim(
        "Min",
        "every scheduler",
        &format!("quantile 0.4 of ({}) <= 2", steps()),
    ) + &claim(
        "MinNo",
        "every scheduler",
        &format!("quantile 0.5 of ({}) <= 2", steps()),
    ) + &claim(
        "Max",
        "every scheduler",
        &format!("quantile 0.6 of ({}) >= 2", steps()),
    ) + &claim(
        "MaxNo",
        "every scheduler",
        &format!("quantile 0.5 of ({}) >= 2", steps()),
    );
    let world = World::new(&retry(&claims));
    let doc = proof(&world, "Min", ratio(3, 80));
    assert_eq!(doc["objective"]["payoff"], "shifted");
    assert_eq!(doc["objective"]["optimization"], "minimum");
    proof(&world, "Max", ratio(-1, 10));
    refuted(&world, "MinNo");
    refuted(&world, "MaxNo");
    let mut altered = doc.clone();
    let initial = altered["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["rank"] == 4)
        .unwrap()["state"]
        .clone();
    altered["values"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["state"] == initial)
        .unwrap()["value"] = json!({"numerator":"9","denominator":"40"});
    let checked = check(&world, "Min", &altered).unwrap();
    assert!(
        matches!(
            checked,
            ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::InequalityFails,
                ..
            })
        ),
        "{checked:?}"
    );
}

#[trace("TC-3152", "FR-1043-AC-5")]
#[test]
fn comparison_event_and_fraction_mean_use_distinct_exact_payoffs() {
    let world = World::new(&retry(
        &(claim(
            "Event",
            "Steady",
            &format!("probability >= 0.7 [{} <= 2]", steps()),
        ) + &claim(
            "Mean",
            "Steady",
            "mean of fraction holds(m.delivered) over [0,2] >= rational(2,5)",
        )),
    ));
    let event = proof(&world, "Event", ratio(3, 4));
    assert_eq!(event["objective"]["payoff"], "event");
    let mean = proof(&world, "Mean", ratio(5, 12));
    assert_eq!(mean["objective"]["payoff"], "fraction");
    let checked = check(&world, "Mean", &mean);
    assert!(
        matches!(checked, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
        "{checked:?}"
    );
}

#[trace("TC-3152", "FR-1043-AC-7")]
#[test]
fn changed_objectives_full_keys_and_shifted_range_never_alias() {
    let world = World::new(&retry(&claim(
        "Q",
        "every scheduler",
        &format!("quantile 0.4 of ({}) <= 2", steps()),
    )));
    let doc = proof(&world, "Q", ratio(3, 80));
    for (member, value) in [
        ("window", json!(5)),
        ("threshold", json!({"numerator":"3","denominator":"1"})),
        ("comparison", json!("lt")),
        ("reward", json!([{}, "duration"])),
        ("unit", json!(["declared", {}])),
        ("level", json!({"numerator":"1","denominator":"2"})),
    ] {
        let mut altered = doc.clone();
        altered["objective"]["measure"][member] = value;
        let actual = check(&world, "Q", &altered);
        assert!(
            !matches!(actual, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
            "{member}: {actual:?}"
        );
    }
    let mut changed = doc.clone();
    changed["objective"]["payoff"] = json!("activated");
    let actual = check(&world, "Q", &changed);
    assert!(
        matches!(
            actual,
            Err(fsl_replay::ProbabilityDocumentError::ObjectiveMismatch)
        ),
        "{actual:?}"
    );
    let mut changed = doc.clone();
    changed["values"][0]["value"] = json!({"numerator":"-1","denominator":"1"});
    let actual = check(&world, "Q", &changed);
    assert!(
        matches!(
            actual,
            Ok(ExactProbabilityCertificateCheck::Rejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::Malformed,
                    at: ProbabilityCertificateLocus::Member("values")
                }
            ))
        ),
        "{actual:?}"
    );
    let mut changed = doc.clone();
    let mut decoded = key(&changed["values"][0]);
    decoded[5][2] = json!("unactivated");
    changed["values"][0]["state"] =
        json!(URL_SAFE_NO_PAD.encode(serde_json::to_vec(&decoded).unwrap()));
    let actual = check(&world, "Q", &changed);
    assert!(
        !matches!(actual, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
        "{actual:?}"
    );
}

#[trace("TC-3152", "FR-1043-AC-7", "FR-196-AC-8", "FR-196-AC-12")]
#[test]
fn a_draw_with_multiple_post_states_retains_its_full_pending_control() {
    let claim = claim(
        "Q",
        "every scheduler",
        "quantile 0.4 of (steps from holds(true) until holds(m.delivered) within 2) >= 1",
    );
    let mut model = retry(&claim);
    model.body = model.body.replace("self.delivered = (lost = 1)", "true");
    let world = World::new(&model);
    let doc = proof(&world, "Q", ratio(-2, 5));
    let pending = doc["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(key)
        .filter(|key| !key[7].is_null())
        .collect::<Vec<_>>();
    assert!(
        !pending.is_empty(),
        "a genuine residual post-state choice must enter the graph"
    );
    assert!(pending
        .iter()
        .all(|key| key[7][0] == "drawn" && key[5][0] == "measure"));
    assert!(
        pending
            .iter()
            .any(|key| key[7][2][0] == json!({"type":"integer","value":"0"})),
        "{pending:?}"
    );
    assert!(
        pending
            .iter()
            .any(|key| key[7][2][0] == json!({"type":"integer","value":"1"})),
        "{pending:?}"
    );
    let checked = check(&world, "Q", &doc);
    assert!(
        matches!(checked, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
        "{checked:?}"
    );
}

#[trace("TC-3152", "FR-1043-AC-5", "FR-196-AC-11")]
#[test]
fn a_completed_fraction_does_not_read_an_outside_window_deleted_object() {
    let model=prob_world::coin("probabilistic Mean using ev on M over (c:M::Coin) under Even on origin { mean of fraction holds(c.v = 0) over [0,0] >= rational(1,2) }");
    let world = World::new(&model);
    proof(&world, "Mean", ratio(1, 1));
}

#[trace("TC-3152", "FR-1043-AC-7", "FR-196-AC-13")]
#[test]
fn measure_and_fraction_undefined_reads_replay_the_unadvanced_control() {
    for form in [
        "probability >= 0.5 [steps from holds(c.v = 0) until holds(c.v = 1) within 2 <= 1]",
        "quantile 0.5 of (steps from holds(c.v = 0) until holds(c.v = 1) within 2) <= 1",
        "mean of fraction holds(c.v = 0) over [0,2] >= rational(1,2)",
    ] {
        let source = format!(
            "probabilistic Q using ev on M over (c:M::Coin) under Even on origin {{ {form} }}"
        );
        let world = World::new(&prob_world::coin(&source));
        let record = settle(&world, "Q", ModelCheckLimits::default());
        let terminal = record
            .terminal()
            .unwrap_or_else(|| panic!("{:?}", record.settlement));
        assert_eq!(
            terminal.value(),
            &TerminalValue::Refuted,
            "{form}: {:?}",
            terminal.value()
        );
        let reproduced = record
            .probabilistic_counterexample
            .as_ref()
            .expect("native undefined evidence");
        let undefined = reproduced
            .undefined()
            .expect("an independently reproduced undefined observation");
        assert_eq!(undefined.evaluation.site.position, 1);
        let decoded: Value = serde_json::from_slice(undefined.product_state.as_bytes()).unwrap();
        assert!(!decoded[5].is_null() && !decoded[6].is_null(), "{decoded}");
        let system = ModelSystem::new(world.subject().simulation).unwrap();
        let replay = fsl_replay::replay_probabilistic_undefined(
            &world.package,
            &system,
            &undefined.path.item,
            undefined,
            &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
            &Cancel::new(),
        );
        assert!(
            matches!(
                replay,
                Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
            ),
            "{replay:?}"
        );
    }
}

#[trace("TC-3152", "FR-1043-AC-7")]
#[test]
fn the_independent_measure_checker_preserves_caller_limits_and_cancellation() {
    let world = World::new(&retry(&claim(
        "Q",
        "every scheduler",
        &format!("quantile 0.4 of ({}) <= 2", steps()),
    )));
    let doc = proof(&world, "Q", ratio(3, 80));
    let bytes = serde_json::to_vec(&doc).unwrap();
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("Q")).unwrap();
    let limits = fsl_replay::ProbabilityDocumentLimits {
        bytes: 1_000_000,
        storage_bytes: 8_000_000,
        states: 128,
        edges: 1024,
        scalar_bits: 128,
        accounting: fsl_replay::AccountingLimits::default().0,
    };
    let run = |limits, cancel: &Cancel| {
        fsl_replay::check_probability_certificate_document(
            &system,
            world.package.graph().probabilistic_model().claims(),
            &item,
            &bytes,
            limits,
            cancel,
        )
    };
    let actual = run(limits, &Cancel::new());
    assert!(
        matches!(actual, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
        "valid baseline: {actual:?}"
    );
    for (offered, expected) in [
        (
            fsl_replay::ProbabilityDocumentLimits { bytes: 0, ..limits },
            fsl_replay::ProbabilityDocumentLimit::Bytes,
        ),
        (
            fsl_replay::ProbabilityDocumentLimits {
                storage_bytes: 0,
                ..limits
            },
            fsl_replay::ProbabilityDocumentLimit::StorageBytes,
        ),
        (
            fsl_replay::ProbabilityDocumentLimits {
                states: 0,
                ..limits
            },
            fsl_replay::ProbabilityDocumentLimit::States,
        ),
        (
            fsl_replay::ProbabilityDocumentLimits { edges: 0, ..limits },
            fsl_replay::ProbabilityDocumentLimit::Edges,
        ),
    ] {
        let actual = run(offered, &Cancel::new());
        assert!(
            matches!(actual,Err(fsl_replay::ProbabilityDocumentError::Limit { limit,.. }) if limit==expected),
            "{expected:?}: {actual:?}"
        );
    }
    for accounting in [
        filament_exact::ScalarLimits {
            work_units: 0,
            ..limits.accounting
        },
        filament_exact::ScalarLimits {
            integer_bits: 0,
            ..limits.accounting
        },
        filament_exact::ScalarLimits {
            result_units: 0,
            ..limits.accounting
        },
    ] {
        let actual = run(
            fsl_replay::ProbabilityDocumentLimits {
                accounting,
                ..limits
            },
            &Cancel::new(),
        );
        assert!(
            matches!(
                actual,
                Err(fsl_replay::ProbabilityDocumentError::Accounting(_))
            ),
            "{actual:?}"
        );
    }
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Deadline);
    let actual = run(limits, &cancel);
    assert!(
        matches!(
            actual,
            Err(fsl_replay::ProbabilityDocumentError::Cancelled(
                CancelCause::Deadline
            ))
        ),
        "{actual:?}"
    );
}

#[trace("TC-2066", "FR-1043-AC-4", "FR-203-AC-1")]
#[test]
fn p95_quantity_ratio_is_accepted_from_its_emitted_package_and_two_ms_refutes() {
    let claims="probabilistic P95 using ev on M over (s:M::Server) under Steady on origin { quantile 0.95 of (accumulate duration from holds(s.phase = 1) until holds(s.phase = 2) within 10) <= 5 ms }\nprobabilistic TooFast using ev on M over (s:M::Server) under Steady on origin { quantile 0.95 of (accumulate duration from holds(s.phase = 1) until holds(s.phase = 2) within 10) <= 2 ms }";
    let mut model = prob_world::quantity_service("1 ms:90,3 ms:7,12 ms:3", claims);
    model.body = model.body.replace(
        "random Server::attempt(ok) ~ { true: 1 };",
        "random Server::attempt(ok) ~ { true: 98, false: 2 };",
    );
    let world = World::new(&model);
    let document = proof(&world, "P95", ratio(24233, 25000));
    let numerator = &document["conditional"]["numerator"];
    assert_eq!(numerator["objective"]["measure"]["kind"], "accumulate");
    assert_eq!(numerator["objective"]["measure"]["reward"][1], "duration");
    assert!(!numerator["objective"]["measure"]["unit"].is_null());
    let emitted =
        fsl_package::emit_checked(&world.package, fsl_package::EmitLimits::default()).unwrap();
    let simulation = world.subject().simulation;
    let mut documents = vec![model.document(false)];
    documents.extend(simulation.snapshots.values().cloned());
    let provision = fsl_replay::ByteProvision::admit(
        world
            .provision
            .entries()
            .map(|(authority, identity, bytes)| {
                (authority.to_owned(), identity.to_owned(), bytes.to_vec())
            })
            .collect(),
        documents,
        std::iter::empty(),
    )
    .unwrap();
    let item = fsl_replay::probabilistic_claim_identity(world.claim("P95")).unwrap();
    let limits = fsl_replay::ProbabilitySourceLimits {
        spine: fsl_replay::spine::SpineLimits::default(),
        observation: simulation.observation,
        expansion: simulation.expansion,
        scalar: simulation.meter,
        document: fsl_replay::ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes: 8_000_000,
            states: 128,
            edges: 1024,
            scalar_bits: 128,
            accounting: fsl_replay::AccountingLimits::default().0,
        },
    };
    let check = |document: &Value| {
        let bytes = serde_json::to_vec(document).unwrap();
        fsl_replay::check_probability_certificate_package(
            fsl_replay::ProbabilityPackageRequest {
                package: emitted.package().bytes(),
                package_name: Some("package"),
                provision: &provision,
                item: &item,
                initial: simulation.initial.clone(),
                universes: simulation.universes.clone(),
                certificate: &bytes,
            },
            limits,
            &Cancel::new(),
        )
    };
    let checked = check(&document);
    assert!(
        matches!(checked, Ok(ExactProbabilityCertificateCheck::Accepted(_))),
        "{checked:?}"
    );
    let mut altered = document.clone();
    altered["conditional"]["numerator"]["values"][0]["value"] =
        json!({"numerator":"1","denominator":"7"});
    let checked = check(&altered);
    assert!(
        matches!(
            checked,
            Ok(ExactProbabilityCertificateCheck::Rejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::InequalityFails,
                    at: ProbabilityCertificateLocus::ProductPart {
                        part: fsl_replay::ProbabilityCertificatePart::Numerator,
                        ..
                    }
                }
            ))
        ),
        "{checked:?}"
    );
    let too_fast = settle(&world, "TooFast", ModelCheckLimits::default());
    assert_eq!(
        too_fast.terminal().unwrap().value(),
        &TerminalValue::Refuted,
        "{:?}",
        too_fast.settlement
    );
    assert_eq!(
        too_fast
            .probabilistic_counterexample
            .as_ref()
            .unwrap()
            .evidence()
            .paths
            .len(),
        1
    );
}

#[trace("TC-3152", "FR-1043-AC-7", "FR-196-AC-13")]
#[test]
fn an_undefined_measure_after_merged_draws_replays_a_real_positive_draw() {
    let source="probabilistic Q using ev on M over (c:M::Coin) under Even on origin { probability >= 0.5 [steps from holds(c.v = 0) until holds(false) within 3 <= 2] }";
    let mut model = prob_world::coin("");
    model.operations.push(prob_world::Operation {
        name: "advance",
        parameters: vec![("bit", "ix://test/coin/Bit".into())],
        modifies: vec!["v"],
        deletes: false,
    });
    model.body = model
        .body
        .replace(
            "post Drop using v",
            "pre CanDrop using v on M::Coin::drop { self.v = 1 }\npost Drop using v",
        )
        .replace(
            "weight Coin::drop = 1;",
            "weight Coin::drop = 1; weight Coin::advance = 1;",
        );
    model.body.push_str("\npre CanAdvance using v on M::Coin::advance { self.v = 0 }\npost Advanced using v on M::Coin::advance { self.v = 1 }\nrandom Coin::advance(bit) ~ { 0:1,1:1 };\n");
    model.body.push_str(source);
    let world = World::new(&model);
    let record = settle(&world, "Q", ModelCheckLimits::default());
    assert_eq!(
        record.terminal().unwrap().value(),
        &TerminalValue::Refuted,
        "{:?}",
        record.settlement
    );
    let token = record
        .probabilistic_counterexample
        .as_ref()
        .expect("a native reproduced path");
    let undefined = token.undefined().unwrap();
    assert_eq!(undefined.evaluation.site.position, 2);
    assert_eq!(undefined.path.paths[0].steps.len(), 2);
    assert!(undefined.path.paths[0].steps[0].draw.is_some());
    assert_eq!(token.mass(), &ratio(1, 4));
}

#[trace("TC-3152", "FR-197-AC-4", "FR-1043-AC-7")]
#[test]
fn unreachable_workload_activation_has_a_certified_value_free_vacuity_basis() {
    let world = World::new(&retry(
        &(claim(
            "Le",
            "Steady",
            "quantile 0.9 of (steps from holds(false) until holds(m.delivered) within 4) <= 2",
        ) + &claim(
            "Ge",
            "Steady",
            "quantile 0.9 of (steps from holds(false) until holds(m.delivered) within 4) >= 2",
        )),
    ));
    for name in ["Le", "Ge"] {
        let record = settle(&world, name, ModelCheckLimits::default());
        let terminal = record
            .terminal()
            .unwrap_or_else(|| panic!("{:?}", record.settlement));
        assert!(
            matches!(
                terminal.value(),
                TerminalValue::Proved {
                    basis: ProofBasis::Exhaustive,
                    certification: Certification::Certified
                }
            ),
            "{:?}",
            terminal.value()
        );
        assert!(
            record.exact_probabilistic.is_none(),
            "vacuity has no invented exact value"
        );
        let document: Value =
            serde_json::from_slice(terminal.exact_certificate_document().unwrap()).unwrap();
        assert_eq!(document["kind"], "conditional");
        assert_eq!(document["conditional"], json!({"mode":"vacuous"}));
        assert_eq!(document["values"], json!([]));
        let checked = check(&world, name, &document);
        assert!(
            matches!(
                checked,
                Ok(ExactProbabilityCertificateCheck::AcceptedVacuous(_))
            ),
            "{checked:?}"
        );
    }
    let activated = World::new(&retry(&claim(
        "Q",
        "Steady",
        &format!("quantile 0.5 of ({}) <= 2", steps()),
    )));
    let mut forged = proof(&activated, "Q", ratio(3, 4));
    forged["conditional"] = json!({"mode":"vacuous"});
    let checked = check(&activated, "Q", &forged);
    assert!(
        matches!(
            checked,
            Ok(ExactProbabilityCertificateCheck::Rejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::InequalityFails,
                    ..
                }
            ))
        ),
        "{checked:?}"
    );
}

fn document_limits() -> fsl_replay::ProbabilityDocumentLimits {
    fsl_replay::ProbabilityDocumentLimits {
        bytes: 1_000_000,
        storage_bytes: 8_000_000,
        states: 128,
        edges: 1024,
        scalar_bits: 128,
        accounting: fsl_replay::AccountingLimits::default().0,
    }
}

#[trace(
    "TC-3152",
    "FR-1043-AC-5",
    "FR-202-AC-27",
    "FR-1043-AC-9",
    "FR-202-AC-29"
)]
#[test]
fn workload_mean_refutation_uses_a_checked_subsystem_and_equality_never_refutes() {
    let claims = claim(
        "No",
        "Steady",
        "mean of fraction holds(m.delivered) over [0,2] >= rational(1,2)",
    ) + &claim(
        "EqualGe",
        "Steady",
        "mean of fraction holds(m.delivered) over [0,2] >= rational(5,12)",
    ) + &claim(
        "EqualLe",
        "Steady",
        "mean of fraction holds(m.delivered) over [0,2] <= rational(5,12)",
    );
    let world = World::new(&retry(&claims));
    let record = settle(&world, "No", ModelCheckLimits::default());
    assert_eq!(
        record.terminal().unwrap().value(),
        &TerminalValue::Refuted,
        "{:?}",
        record.settlement
    );
    let token = record
        .probabilistic_counterexample
        .as_ref()
        .expect("a checked subsystem");
    assert!(token.evidence().paths.is_empty());
    let subsystem = token
        .evidence()
        .subsystem
        .as_ref()
        .expect("fraction expectation certificate");
    let document: Value = serde_json::from_slice(subsystem.as_bytes()).unwrap();
    assert_eq!(document["kind"], "exact");
    assert_eq!(document["objective"]["payoff"], "fraction");
    assert_eq!(
        token.wire.as_ref().unwrap()["evidence"]["type"],
        "subsystem"
    );
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let replay = fsl_replay::replay_probabilistic_witness(
        &world.package,
        &system,
        &token.evidence().item,
        token.evidence(),
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        &Cancel::new(),
    );
    assert!(
        matches!(
            replay,
            Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
        ),
        "{replay:?}"
    );
    for name in ["EqualGe", "EqualLe"] {
        proof(&world, name, ratio(5, 12));
        let mut offered = token.evidence().clone();
        offered.item = fsl_replay::probabilistic_claim_identity(world.claim(name)).unwrap();
        let mut document = fsl_replay::read_measure_probability_certificate(
            subsystem.as_bytes(),
            document_limits(),
            &Cancel::new(),
        )
        .unwrap();
        document.identity = offered.item.clone();
        offered.subsystem = Some(Box::new(
            fsl_replay::ProbabilitySubsystemCertificate::from_document(
                &document,
                document_limits(),
                &Cancel::new(),
            )
            .unwrap(),
        ));
        let replay = fsl_replay::replay_probabilistic_witness(
            &world.package,
            &system,
            &offered.item,
            &offered,
            &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
            &Cancel::new(),
        );
        assert!(
            matches!(
                replay,
                Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                    ProbabilityCertificateRejection {
                        rule: ProbabilityCertificateRule::ThresholdMissed,
                        ..
                    }
                ))
            ),
            "equality: {replay:?}"
        );
    }
    let mut document = fsl_replay::read_measure_probability_certificate(
        subsystem.as_bytes(),
        document_limits(),
        &Cancel::new(),
    )
    .unwrap();
    document.values[0].value = ratio(1, 7);
    let mut offered = token.evidence().clone();
    offered.subsystem = Some(Box::new(
        fsl_replay::ProbabilitySubsystemCertificate::from_document(
            &document,
            document_limits(),
            &Cancel::new(),
        )
        .unwrap(),
    ));
    let replay = fsl_replay::replay_probabilistic_witness(
        &world.package,
        &system,
        &offered.item,
        &offered,
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        &Cancel::new(),
    );
    assert!(
        matches!(
            replay,
            Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::InequalityFails,
                    ..
                }
            ))
        ),
        "altered exact value: {replay:?}"
    );
}

#[trace("TC-3152", "FR-1043-AC-8", "FR-1043-AC-9", "FR-202-AC-29")]
#[test]
fn every_scheduler_exact_mean_witness_attains_the_deciding_extremum() {
    let claims = claim(
        "MinNo",
        "every scheduler",
        "mean of fraction holds(m.delivered) over [0,2] >= rational(1,2)",
    ) + &claim(
        "MaxNo",
        "every scheduler",
        "mean of fraction holds(m.delivered) over [0,2] <= rational(1,3)",
    );
    let world = World::new(&retry(&claims));
    for name in ["MinNo", "MaxNo"] {
        refuted(&world, name);
    }
    let baseline = settle(&world, "MinNo", ModelCheckLimits::default());
    let token = baseline.probabilistic_counterexample.as_ref().unwrap();
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    let initial = system.initial().remove(0);
    let fast = system
        .actions(&initial)
        .unwrap()
        .unwrap()
        .remove(0)
        .identity;
    let fast =
        fsl_replay::ProductAction::Identity(fsl_replay::Schedule::of(&system, &fast).unwrap());
    let mut changed = token.evidence().clone();
    let row = changed
        .scheduler
        .iter_mut()
        .find(|(key, _)| key == &changed.initial_key)
        .unwrap();
    assert_ne!(
        row.1, fast,
        "the adverse action must differ from the actual minimizing policy"
    );
    row.1 = fast;
    let replay = fsl_replay::replay_probabilistic_witness(
        &world.package,
        &system,
        &changed.item,
        &changed,
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        &Cancel::new(),
    );
    assert!(
        matches!(replay, Ok(fsl_replay::ProbabilityWitnessReplay::Parity)),
        "{replay:?}"
    );
}

fn mean_bound(
    witness: &fsl_replay::ProbabilisticCounterexample,
    lower: bool,
    every: bool,
) -> Value {
    let mut document: Value =
        serde_json::from_slice(witness.subsystem.as_ref().unwrap().as_bytes()).unwrap();
    let base = document["objective"].take();
    document["objective"] =
        json!({"profile":"filament.probability-objective/value-bounds/v1","base":base});
    document["kind"] = json!(if lower { "lower" } else { "upper" });
    if lower {
        // The least unnamed value is zero. Retain all positive rows, including
        // their actual horizon ranks, without offering an upper counterpart.
        document["values"]
            .as_array_mut()
            .unwrap()
            .retain(|row| row["value"]["numerator"] != "0");
        let states = document["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["state"].clone())
            .collect::<Vec<_>>();
        document["ranking"]
            .as_array_mut()
            .unwrap()
            .retain(|row| states.contains(&row["state"]));
    } else {
        document.as_object_mut().unwrap().remove("ranking");
    }
    if every {
        let states = document["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["state"].clone())
            .collect::<Vec<_>>();
        document["policy"] = Value::Array(witness.scheduler.iter().filter_map(|(key, action)| {
            let state = json!(URL_SAFE_NO_PAD.encode(key.as_bytes()));
            if !states.contains(&state) { return None; }
            let fsl_replay::ProductAction::Identity(schedule) = action else { return None; };
            let triple: Value = serde_json::from_slice(schedule.as_bytes()).unwrap();
            Some(json!({"state":state,"choice":{"type":"transition","operation":serde_json::to_string(&triple[0]).unwrap(),"receiver":triple[1],"arguments":triple[2]}}))
        }).collect());
    }
    document
}

fn replay_mean_document(
    world: &World,
    baseline: &fsl_replay::ProbabilisticCounterexample,
    document: &Value,
) -> Result<fsl_replay::ProbabilityWitnessReplay, fsl_replay::ProbabilityWitnessError> {
    let mut offered = baseline.clone();
    offered.subsystem = Some(Box::new(
        fsl_replay::ProbabilitySubsystemCertificate::from_bytes(
            &serde_json::to_vec(document).unwrap(),
            document_limits(),
            &Cancel::new(),
        )
        .unwrap(),
    ));
    let system = ModelSystem::new(world.subject().simulation).unwrap();
    fsl_replay::replay_probabilistic_witness(
        &world.package,
        &system,
        &offered.item,
        &offered,
        &fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        &Cancel::new(),
    )
}

#[trace("FR-1043-AC-8", "FR-1043-AC-9", "FR-202-AC-28", "FR-202-AC-29")]
#[test]
fn bounded_mean_refutation_roles_accept_upper_minimum_and_partial_lower_maximum() {
    for scheduler in ["Steady", "every scheduler"] {
        let world = World::new(&retry(
            &(claim(
                "Min",
                scheduler,
                "mean of fraction holds(m.delivered) over [0,2] >= rational(1,2)",
            ) + &claim(
                "Max",
                scheduler,
                "mean of fraction holds(m.delivered) over [0,2] <= rational(1,3)",
            )),
        ));
        for (name, lower) in [("Min", false), ("Max", true)] {
            let baseline = settle(&world, name, ModelCheckLimits::default());
            assert_eq!(
                baseline.terminal().unwrap().value(),
                &TerminalValue::Refuted,
                "{:?}",
                baseline.settlement
            );
            let token = baseline.probabilistic_counterexample.as_ref().unwrap();
            let document = mean_bound(token.evidence(), lower, scheduler == "every scheduler");
            let replay = replay_mean_document(&world, token.evidence(), &document);
            assert!(
                matches!(
                    replay,
                    Ok(fsl_replay::ProbabilityWitnessReplay::Reproduced(_))
                ),
                "{name}/{scheduler}: {replay:?}"
            );
            let root = URL_SAFE_NO_PAD.encode(token.evidence().initial_key.as_bytes());
            let mut equality = document.clone();
            let row = equality["values"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|row| row["state"] == root)
                .expect("the initial bound");
            row["value"] = if lower {
                json!({"numerator":"1","denominator":"3"})
            } else {
                json!({"numerator":"1","denominator":"2"})
            };
            let replay = replay_mean_document(&world, token.evidence(), &equality);
            assert!(
                matches!(
                    replay,
                    Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                        ProbabilityCertificateRejection {
                            rule: ProbabilityCertificateRule::ThresholdMissed,
                            ..
                        }
                    ))
                ),
                "valid non-refuting equality: {replay:?}"
            );
            let mut invalid = document.clone();
            let row = invalid["values"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|row| row["state"] == root)
                .unwrap();
            row["value"] = json!({"numerator":if lower { "1" } else { "0" },"denominator":"1"});
            let replay = replay_mean_document(&world, token.evidence(), &invalid);
            assert!(
                matches!(
                    replay,
                    Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                        ProbabilityCertificateRejection {
                            rule: ProbabilityCertificateRule::InequalityFails,
                            ..
                        }
                    ))
                ),
                "invalid bound: {replay:?}"
            );
            if lower {
                let original: Value =
                    serde_json::from_slice(token.evidence().subsystem.as_ref().unwrap().as_bytes())
                        .unwrap();
                assert!(
                    document["values"].as_array().unwrap().len()
                        < original["values"].as_array().unwrap().len(),
                    "the lower must really omit states"
                );
                let mut changed = document.clone();
                changed["ranking"][0]["rank"] = json!(99);
                let replay = replay_mean_document(&world, token.evidence(), &changed);
                assert!(
                    matches!(
                        replay,
                        Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                            ProbabilityCertificateRejection {
                                rule: ProbabilityCertificateRule::RankBroken,
                                ..
                            }
                        ))
                    ),
                    "{replay:?}"
                );
            } else {
                let mut changed = document.clone();
                changed["values"].as_array_mut().unwrap().pop();
                let replay = replay_mean_document(&world, token.evidence(), &changed);
                assert!(
                    matches!(
                        replay,
                        Err(fsl_replay::ProbabilityWitnessError::CertificateRejected(
                            ProbabilityCertificateRejection {
                                rule: ProbabilityCertificateRule::Malformed,
                                at: fsl_replay::ProbabilityCertificateLocus::Member("values")
                            }
                        ))
                    ),
                    "{replay:?}"
                );
            }
            let mut changed = document.clone();
            changed["objective"]["base"]["measure"]["window"] = json!(3);
            let replay = replay_mean_document(&world, token.evidence(), &changed);
            assert!(
                matches!(replay, Err(fsl_replay::ProbabilityWitnessError::Refused)),
                "{replay:?}"
            );
        }
    }
}
