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
