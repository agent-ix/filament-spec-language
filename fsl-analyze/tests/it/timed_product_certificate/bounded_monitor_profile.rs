// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-186-AC-7: the admitted bounded timed Eventually form.

use super::*;

#[trace("FR-186-AC-7")]
#[test]
fn timed_eventually_with_closed_horizon_settles_at_ninety_nine_percent() {
    let claims = "\
        probabilistic Eventually using ev on M over (c: M::Call) under every scheduler on origin { probability >= 0.99 [ eventually[0 ms, 4 ms] holds(c.late) ] }\n";
    let delay_two_ms = "delay M::Call::send ~ discrete { 2 ms: 1 };\n";
    let world = retx(GUARD, true, delay_two_ms, claims);
    let record = settle(&world, "Eventually");
    let terminal = record
        .terminal()
        .unwrap_or_else(|| panic!("timed bounded claim has a terminal: {record:#?}"));
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        certification: fsl_replay::Certification::Certified,
    } = terminal.value()
    else {
        panic!("timed bounded claim settles with a checked proof: {record:#?}");
    };
    assert_eq!(value.value(), &ratio(99, 100));

    let document = document(&record);
    let terminal_timeout = document["values"]
        .as_array()
        .expect("certificate values")
        .iter()
        .any(|row| {
            let encoded = row["state"].as_str().expect("encoded ProductKey");
            let bytes = URL_SAFE_NO_PAD.decode(encoded).expect("base64 ProductKey");
            let key: Value = serde_json::from_slice(&bytes).expect("canonical ProductKey");
            let clocks = key[4][1][4].as_array().expect("digital clocks");
            let x = clocks
                .iter()
                .find(|clock| clock[0][1] == "x")
                .expect("clock x");
            x[1] == "2" && key[4][1][5] == json!(["deadline", "4", "5"])
        });
    assert!(
        terminal_timeout,
        "producer's t=5 rejecting state retains x=2 ms"
    );
    assert!(matches!(
        check(&world, "Eventually", &document),
        Ok(ExactProbabilityCertificateCheck::Accepted(_))
    ));
}

#[trace("FR-186-AC-3")]
#[test]
fn nested_punctual_always_consumes_the_initial_position() {
    let claims = "\
        probabilistic InitialNested using ev on M over (c: M::Call) under every scheduler on origin { probability >= 0.99 [ eventually[0 ms, 1 ms] always[0 ms, 0 ms] holds(c.late) ] }\n";
    let package_document = timed_world::document(&["x"], &[("step", &["late", "x"])]);
    let body = format!(
        "{}\
         pre Step using v on M::Call::step {{ true }}\n\
         post Stepped using v on M::Call::step {{ self.late = false and self.x = 0 ms }}\n\
         delay M::Call::step ~ discrete {{ 0 ms: 1 }};\n\
         time invariant ZeroClock when {{ true }} {{ self.x <= 0 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world_holding(
        &package_document,
        super::PROFILES,
        &body,
        &json!({ "late": { "boolean": true } }),
    );
    let record = settle(&world, "InitialNested");
    assert_eq!(proved_value(&record), ratio(1, 1));
    assert!(accepted(&world, "InitialNested", &document(&record)));
}

#[trace("FR-186-AC-3")]
#[test]
fn nested_punctual_always_keeps_a_true_zero_delay_hop_before_a_false_hop() {
    let claims = "\
        probabilistic HopNested using ev on M over (c: M::Call) under every scheduler on origin { probability >= 0.99 [ eventually[0 ms, 1 ms] always[0 ms, 0 ms] holds(c.late) ] }\n";
    let package_document = timed_world::document(
        &["x"],
        &[("first", &["phase", "late", "x"]), ("second", &["phase", "late", "x"])],
    );
    let body = format!(
        "{}\
         pre First using v on M::Call::first {{ self.phase = 0 }}\n\
         post FirstDone using v on M::Call::first {{ self.phase = 1 and self.late = true and self.x = 0 ms }}\n\
         pre Second using v on M::Call::second {{ self.phase = 1 }}\n\
         post SecondDone using v on M::Call::second {{ self.phase = 2 and self.late = false and self.x = 0 ms }}\n\
         delay M::Call::first ~ discrete {{ 0 ms: 1 }};\n\
         delay M::Call::second ~ discrete {{ 0 ms: 1 }};\n\
         time invariant ZeroClock when {{ true }} {{ self.x <= 0 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world(&package_document, super::PROFILES, &body);
    let record = settle(&world, "HopNested");
    assert_eq!(proved_value(&record), ratio(1, 1));
    assert!(accepted(&world, "HopNested", &document(&record)));
}

/// A zero-delay cycle with an unchanged false observation has a finite key.
#[trace("FR-186-AC-7")]
#[test]
fn timed_eventually_zero_delay_cycle_has_a_finite_repeated_key() {
    let claims = "\
        probabilistic ZeroCycle using ev on M over (c: M::Call) under every scheduler on origin { probability <= 0.5 [ eventually[0 ms, 4 ms] holds(c.inflight) ] }\n";
    let package_document = timed_world::document(&["x"], &[("toggle", &["retried", "x"])]);
    let body = format!(
        "{}\
         pre Toggle using v on M::Call::toggle {{ self.x <= 1 ms }}\n\
         post Toggled using v on M::Call::toggle {{ self.retried = not pre(self.retried) and self.x = 0 ms }}\n\
         delay M::Call::toggle ~ discrete {{ 0 ms: 1, 1 ms: 1 }};\n\
         time invariant ClockWindow when {{ true }} {{ self.x <= 4 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world(&package_document, super::PROFILES, &body);
    let checked_claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim("ZeroCycle")
        .expect("the bounded timed claim checks");
    assert!(matches!(
        &checked_claim.form,
        fsl_semantics::check::ProbForm::Probability {
            event: fsl_semantics::check::Event::Formula {
                timed_horizon: Some(_),
                ..
            },
            ..
        }
    ));
    let mut request = request(&world, "ZeroCycle");
    request.limits.model.limits.max_states = 128;
    let product = match fsl_analyze::exact_probabilistic::build_product(request, &Cancel::new()) {
        Ok(fsl_analyze::exact_probabilistic::ProbBuild::Built(product)) => product,
        other => panic!("the zero-delay timed product closes its finite key cycle: {other:?}"),
    };
    let repeated_history = vec![fsl_analyze::exact_probabilistic::TimedObservation {
        time: 0,
        values: vec![false],
    }];
    assert!(
        product.states.len() < 128,
        "the product closes below its state bound"
    );
    let zero = ratio(0, 1);
    let one = ratio(1, 1);
    assert!(
        product.edges.iter().any(|edge| edge.elapsed == zero),
        "the mixed delay law has a literal zero-ms edge"
    );
    assert!(
        product.edges.iter().any(|edge| edge.elapsed == one),
        "the mixed delay law has a literal one-ms edge"
    );
    let repeated_keys = product
        .ids()
        .zip(&product.states)
        .filter_map(|(id, state)| {
            (state.timed_history.as_ref() == Some(&repeated_history)).then_some(id)
        })
        .collect::<Vec<_>>();
    assert!(
        repeated_keys.len() >= 2,
        "the retry toggle visits distinct states under the same same-time Eventually summary"
    );
    let reaches_zero_time =
        |start: fsl_analyze::exact_probabilistic::ProbStateId,
         target: fsl_analyze::exact_probabilistic::ProbStateId| {
            let mut seen = vec![false; product.states.len()];
            let mut pending = vec![start];
            while let Some(state) = pending.pop() {
                if state == target {
                    return true;
                }
                if seen[state.index()] {
                    continue;
                }
                seen[state.index()] = true;
                pending.extend(
                    product
                        .outgoing(state)
                        .iter()
                        .filter(|edge| edge.elapsed == zero)
                        .map(|edge| edge.to),
                );
            }
            false
        };
    assert!(
        product.edges.iter().any(|edge| {
            edge.elapsed == zero
                && product.states[edge.from.index()].timed_history.as_ref()
                    == Some(&repeated_history)
                && product.states[edge.to.index()].timed_history.as_ref() == Some(&repeated_history)
                && reaches_zero_time(edge.to, edge.from)
        }),
        "the zero-delay cycle returns to a repeated canonical TimedTrace product key"
    );

    assert!(
        product.states.iter().all(|state| {
            state.timed_history.as_ref().is_some_and(|history| {
                history
                    .iter()
                    .all(|observation| observation.values == [false])
            })
        }),
        "the literal target is false at every reconstructed observation"
    );
    let exact = fsl_analyze::exact_probabilistic::decide_bounded_event(
        &product,
        &fsl_semantics::check::Bound::AtMost(ratio(1, 2)),
        &ExactProbLimits::default(),
        &Cancel::new(),
    )
    .expect("the mixed-delay finite product has an exact bounded value");
    let fsl_analyze::exact_probabilistic::FhResult::Exact { values, .. } = exact else {
        panic!("exact zero-cycle value: {exact:?}");
    };
    assert_eq!(
        values[product.initial[0].2.index()],
        ratio(0, 1),
        "the false target gives literal bounded Eventually probability zero"
    );
    let record = settle(&world, "ZeroCycle");
    assert_eq!(proved_value(&record), ratio(0, 1));
    assert!(accepted(&world, "ZeroCycle", &document(&record)));
}

#[trace("FR-186-AC-7")]
#[test]
fn forged_always_value_cannot_merge_false_and_true_same_time_reads() {
    let claims = "\
        probabilistic Always using ev on M over (c: M::Call) under every scheduler on origin { probability <= 0.5 [ always[0 ms, 4 ms] holds(c.late) ] }\n";
    let package_document = timed_world::document(&["x"], &[("toggle", &["late", "x"])]);
    let body = format!(
        "{}\
         pre Toggle using v on M::Call::toggle {{ self.x <= 1 ms }}\n\
         post Toggled using v on M::Call::toggle {{ self.late = true and self.x = 0 ms }}\n\
         delay M::Call::toggle ~ discrete {{ 0 ms: 1, 1 ms: 1 }};\n\
         time invariant ClockWindow when {{ true }} {{ self.x <= 4 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world(&package_document, super::PROFILES, &body);
    let record = settle(&world, "Always");
    assert_eq!(proved_value(&record), ratio(0, 1));

    let mut forged = document(&record);
    for row in forged["values"].as_array_mut().expect("certificate values") {
        row["value"] = json!({ "numerator": "1", "denominator": "1" });
    }
    assert!(
        !matches!(
            check(&world, "Always", &forged),
            Ok(ExactProbabilityCertificateCheck::Accepted(_))
        ),
        "replay preserves the initial false Always read despite a same-time true successor"
    );
}

#[trace("FR-186-AC-7")]
#[test]
fn timed_eventually_latches_initial_hit_before_immediate_time_lock() {
    let claims = "\
        probabilistic InitialHit using ev on M over (c: M::Call) under every scheduler on origin { probability >= 0.99 [ eventually[0 ms, 4 ms] holds(c.late) ] }\n";
    let package_document = timed_world::document(&["x"], &[("toggle", &["late", "x"])]);
    let body = format!(
        "{}\
         pre Toggle using v on M::Call::toggle {{ false }}\n\
         post Toggled using v on M::Call::toggle {{ self.late = true and self.x = 0 ms }}\n\
         delay M::Call::toggle ~ discrete {{ 1 ms: 1 }};\n\
         time invariant ClockWindow when {{ true }} {{ self.x <= 0 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world_holding(
        &package_document,
        super::PROFILES,
        &body,
        &json!({ "late": { "boolean": true } }),
    );
    let record = settle(&world, "InitialHit");
    assert_eq!(proved_value(&record), ratio(1, 1));
    assert!(accepted(&world, "InitialHit", &document(&record)));
}

#[trace("FR-186-AC-7")]
#[test]
fn timed_eventually_latches_in_window_hit_before_time_lock() {
    let claims = "\
        probabilistic TimedHit using ev on M over (c: M::Call) under every scheduler on origin { probability >= 0.99 [ eventually[0 ms, 4 ms] holds(c.late) ] }\n";
    let package_document = timed_world::document(&["x"], &[("toggle", &["late", "x"])]);
    let body = format!(
        "{}\
         pre Toggle using v on M::Call::toggle {{ not self.late }}\n\
         post Toggled using v on M::Call::toggle {{ self.late = true and self.x = 1 ms }}\n\
         delay M::Call::toggle ~ discrete {{ 1 ms: 1 }};\n\
         time invariant ClockWindow when {{ self.late }} {{ self.x <= 1 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world(&package_document, super::PROFILES, &body);
    let record = settle(&world, "TimedHit");
    let terminal = record
        .terminal()
        .unwrap_or_else(|| panic!("the bounded claim has a terminal: {record:#?}"));
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        ..
    } = terminal.value()
    else {
        panic!("the t=1 lock settles with an exact proof: {record:#?}");
    };
    assert_eq!(value.value(), &ratio(1, 1));
    assert!(accepted(&world, "TimedHit", &document(&record)));
}
