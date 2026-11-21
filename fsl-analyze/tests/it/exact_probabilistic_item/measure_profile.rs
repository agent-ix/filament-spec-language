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
            Err(fsl_replay::ProbabilityDocumentError::MalformedMember(
                "values"
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
