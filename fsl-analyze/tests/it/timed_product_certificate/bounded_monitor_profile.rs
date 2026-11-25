// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-186-AC-7: the admitted bounded timed Eventually form.

use super::*;
use fsl_analyze::exact_probabilistic::{DeadlineKind, DeadlinePhase, MonitorState};

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

#[trace("FR-204-AC-21")]
#[test]
fn nested_punctual_always_consumes_the_initial_position() {
    let claims = "\
        probabilistic InitialNested using ev on M over (c: M::Call) under every scheduler on origin { probability >= 0.99 [ eventually[0 ms, 1 ms] always[0 ms, 0 ms] holds(c.late) ] }\n";
    let package_document = timed_world::document(&["x"], &[("step", &["late", "x"])]);
    let body = format!(
        "{}\
         pre Step using v on M::Call::step {{ true }}\n\
         post Stepped using v on M::Call::step {{ self.late = true and self.x = 0 ms }}\n\
         delay M::Call::step ~ discrete {{ 1 ms: 1 }};\n\
         time invariant ClockWindow when {{ true }} {{ self.x <= 4 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world(&package_document, super::PROFILES, &body);
    let record = settle(&world, "InitialNested");
    assert_eq!(proved_value(&record), ratio(1, 1));
    assert!(accepted(&world, "InitialNested", &document(&record)));
}

#[trace("FR-204-AC-21")]
#[test]
fn nested_punctual_always_rejects_a_true_hop_before_a_false_same_time_hop() {
    let claims = "\
        probabilistic HopNested using ev on M over (c: M::Call) under every scheduler on origin { probability <= 0.01 [ eventually[0 ms, 1 ms] always[0 ms, 0 ms] holds(c.late) ] }\n";
    let package_document = timed_world::document(
        &["x"],
        &[
            ("first", &["phase", "late", "x"]),
            ("second", &["phase", "late", "x"]),
        ],
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
    assert_eq!(proved_value(&record), ratio(0, 1));
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
    // FR-196's deadline monitor, which stays pending while the target is false.
    let pending = Some(MonitorState::Deadline {
        kind: DeadlineKind::Eventually,
        phase: DeadlinePhase::Pending,
    });
    let pending_at_zero = |state: &fsl_analyze::exact_probabilistic::ProbKey| {
        let bytes = state
            .certificate_state
            .as_ref()
            .expect("a timed state has a ProductKey")
            .as_bytes();
        let key: Value = serde_json::from_slice(bytes).expect("canonical ProductKey");
        state.monitor == pending && key[4][1][5] == json!(["deadline", "4", "0"])
    };
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
        .filter_map(|(id, state)| pending_at_zero(state).then_some(id))
        .collect::<Vec<_>>();
    assert!(
        repeated_keys.len() >= 2,
        "the retry toggle visits distinct states under the same pending monitor at time zero"
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
                && pending_at_zero(&product.states[edge.from.index()])
                && pending_at_zero(&product.states[edge.to.index()])
                && reaches_zero_time(edge.to, edge.from)
        }),
        "the zero-delay cycle returns to a repeated canonical product key"
    );

    assert!(
        product.states.iter().all(|state| {
            matches!(
                state.monitor,
                Some(MonitorState::Deadline {
                    kind: DeadlineKind::Eventually,
                    phase: DeadlinePhase::Pending | DeadlinePhase::Reject,
                })
            )
        }),
        "the literal target is false at every observation, so no state accepts"
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

#[trace("FR-204-AC-21")]
#[test]
fn bounded_always_does_not_accept_its_true_initial_observation() {
    let claims = "\
        probabilistic AlwaysAfterChange using ev on M over (c: M::Call) under every scheduler on origin { probability <= 0.01 [ always[0 ms, 4 ms] holds(c.late) ] }\n";
    let package_document = timed_world::document(&["x"], &[("toggle", &["late", "x"])]);
    let body = format!(
        "{}\
         pre Toggle using v on M::Call::toggle {{ self.x <= 1 ms }}\n\
         post Toggled using v on M::Call::toggle {{ self.late = false and self.x = 0 ms }}\n\
         delay M::Call::toggle ~ discrete {{ 1 ms: 1 }};\n\
         time invariant ClockWindow when {{ true }} {{ self.x <= 4 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world_holding(
        &package_document,
        super::PROFILES,
        &body,
        &json!({ "late": { "boolean": true } }),
    );
    let record = settle(&world, "AlwaysAfterChange");
    assert_eq!(proved_value(&record), ratio(0, 1), "{record:#?}");
}

#[trace("FR-204-AC-21")]
#[test]
fn nested_always_does_not_use_a_transient_true_atom_as_its_witness() {
    let claims = "\
        probabilistic NestedAfterChange using ev on M over (c: M::Call) under every scheduler on origin { probability <= 0.01 [ eventually[0 ms, 4 ms] always[0 ms, 0 ms] holds(c.late) ] }\n\
        probabilistic NestedRefuted using ev on M over (c: M::Call) under every scheduler on origin { probability >= 0.99 [ eventually[0 ms, 4 ms] always[0 ms, 0 ms] holds(c.late) ] }\n";
    let package_document = timed_world::document(
        &["x"],
        &[
            ("rise", &["phase", "late", "x"]),
            ("fall", &["phase", "late", "x"]),
        ],
    );
    let body = format!(
        "{}\
         pre Rise using v on M::Call::rise {{ self.phase = 0 }}\n\
         post Risen using v on M::Call::rise {{ self.phase = 1 and self.late = true }}\n\
         pre Fall using v on M::Call::fall {{ self.phase = 1 }}\n\
         post Fallen using v on M::Call::fall {{ self.phase = 2 and self.late = false }}\n\
         delay M::Call::rise ~ discrete {{ 0 ms: 1 }};\n\
         delay M::Call::fall ~ discrete {{ 0 ms: 1 }};\n\
         time invariant ClockWindow when {{ true }} {{ self.x <= 4 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world(&package_document, super::PROFILES, &body);
    let record = settle(&world, "NestedAfterChange");
    assert_eq!(proved_value(&record), ratio(0, 1), "{record:#?}");
    assert!(accepted(&world, "NestedAfterChange", &document(&record)));
    let refuted = settle(&world, "NestedRefuted");
    assert!(is_refuted(&refuted), "{refuted:#?}");
}

#[trace("FR-204-AC-21")]
#[test]
fn nested_positive_offset_always_checks_all_same_time_target_observations() {
    let claims = "\
        probabilistic NestedPositiveOffset using ev on M over (c: M::Call) under every scheduler on origin { probability <= 0.01 [ eventually[0 ms, 4 ms] always[1 ms, 1 ms] holds(c.late) ] }\n";
    let package_document = timed_world::document(
        &["x"],
        &[
            ("rise", &["phase", "late", "x"]),
            ("fall", &["phase", "late", "x"]),
        ],
    );
    let body = format!(
        "{}\
         pre Rise using v on M::Call::rise {{ self.phase = 0 }}\n\
         post Risen using v on M::Call::rise {{ self.phase = 1 and self.late = true and self.x = 0 ms }}\n\
         pre Fall using v on M::Call::fall {{ self.phase = 1 }}\n\
         post Fallen using v on M::Call::fall {{ self.phase = 2 and self.late = false and self.x = 0 ms }}\n\
         delay M::Call::rise ~ discrete {{ 1 ms: 1 }};\n\
         delay M::Call::fall ~ discrete {{ 0 ms: 1 }};\n\
         time invariant ClockWindow when {{ true }} {{ self.x <= 4 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world_holding(
        &package_document,
        super::PROFILES,
        &body,
        &json!({ "late": { "boolean": true } }),
    );
    let record = settle(&world, "NestedPositiveOffset");
    assert_eq!(proved_value(&record), ratio(0, 1), "{record:#?}");
    assert!(accepted(&world, "NestedPositiveOffset", &document(&record)));
}

/// A dual nested window needs one true observation at each target time: a
/// later false hop at that time does not retract it, and every target time of
/// the outer window is checked.
#[trace("FR-204-AC-21")]
#[test]
fn nested_eventually_inside_always_checks_every_target_time() {
    let claims = "\
        probabilistic FirstTarget using ev on M over (c: M::Call) under every scheduler on origin { probability >= 0.99 [ always[0 ms, 0 ms] eventually[1 ms, 1 ms] holds(c.late) ] }\n\
        probabilistic SecondTarget using ev on M over (c: M::Call) under every scheduler on origin { probability <= 0.01 [ always[0 ms, 1 ms] eventually[1 ms, 1 ms] holds(c.late) ] }\n";
    let world = rise_and_fall(1, claims);
    for (claim, value) in [("FirstTarget", ratio(1, 1)), ("SecondTarget", ratio(0, 1))] {
        let record = settle(&world, claim);
        assert_eq!(proved_value(&record), value, "{record:#?}");
        assert!(accepted(&world, claim, &document(&record)));
    }
}

/// At zero offset the last position at a timestamp can witness only itself,
/// so an earlier true hop does not satisfy a dual nested window.
#[trace("FR-204-AC-21")]
#[test]
fn nested_zero_offset_eventually_is_decided_by_the_last_same_time_hop() {
    let claims = "\
        probabilistic LastHop using ev on M over (c: M::Call) under every scheduler on origin { probability <= 0.01 [ always[0 ms, 0 ms] eventually[0 ms, 0 ms] holds(c.late) ] }\n";
    let world = rise_and_fall(0, claims);
    let record = settle(&world, "LastHop");
    assert_eq!(proved_value(&record), ratio(0, 1), "{record:#?}");
    assert!(accepted(&world, "LastHop", &document(&record)));
}

/// `late` is false, rises after `rise_ms` and falls again at that timestamp.
fn rise_and_fall(rise_ms: u64, claims: &str) -> World {
    let package_document = timed_world::document(
        &["x"],
        &[
            ("rise", &["phase", "late", "x"]),
            ("fall", &["phase", "late", "x"]),
        ],
    );
    let body = format!(
        "{}\
         pre Rise using v on M::Call::rise {{ self.phase = 0 }}\n\
         post Risen using v on M::Call::rise {{ self.phase = 1 and self.late = true and self.x = 0 ms }}\n\
         pre Fall using v on M::Call::fall {{ self.phase = 1 }}\n\
         post Fallen using v on M::Call::fall {{ self.phase = 2 and self.late = false and self.x = 0 ms }}\n\
         delay M::Call::rise ~ discrete {{ {rise_ms} ms: 1 }};\n\
         delay M::Call::fall ~ discrete {{ 0 ms: 1 }};\n\
         time invariant ClockWindow when {{ true }} {{ self.x <= 4 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    timed_world::world(&package_document, super::PROFILES, &body)
}

#[trace("FR-204-AC-21")]
#[test]
fn until_preserves_a_false_left_observation_before_same_time_right_target() {
    let claims = "\
        probabilistic OrderedUntil using ev on M over (c: M::Call) under every scheduler on origin { probability <= 0.01 [ holds(c.late) until[1 ms, 4 ms] holds(c.inflight) ] }\n";
    let package_document = timed_world::document(
        &["x"],
        &[("drop", &["late", "x"]), ("deliver", &["inflight", "x"])],
    );
    let body = format!(
        "{}\
         pre Drop using v on M::Call::drop {{ self.x <= 1 ms }}\
         post Dropped using v on M::Call::drop {{ self.late = false and self.x = 0 ms }}\
         pre Deliver using v on M::Call::deliver {{ not self.late }}\
         post Delivered using v on M::Call::deliver {{ self.inflight = true }}\
         delay M::Call::drop ~ discrete {{ 1 ms: 1 }};\
         delay M::Call::deliver ~ discrete {{ 0 ms: 1 }};\
         time invariant ClockWindow when {{ true }} {{ self.x <= 4 ms }}\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world_holding(
        &package_document,
        super::PROFILES,
        &body,
        &json!({ "late": { "boolean": true }, "inflight": { "boolean": false } }),
    );
    let record = settle(&world, "OrderedUntil");
    assert_eq!(proved_value(&record), ratio(0, 1));
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

/// One `step` from phase 0 sets `late`, with the invariant `x <= 3 ms` until
/// it has. `guard` and `delay` fix when it happens.
fn one_step(guard: &str, delay: &str, claims: &str) -> World {
    let package_document = timed_world::document(&["x"], &[("step", &["phase", "late", "x"])]);
    let body = format!(
        "{}\
         pre Step using v on M::Call::step {{ self.phase = 0{guard} }}\n\
         post Stepped using v on M::Call::step {{ self.phase = 1 and self.late = true }}\n\
         {delay}\
         time invariant Wait when {{ self.phase = 0 }} {{ self.x <= 3 ms }}\n\
         workload One on M {{ weight Call::step = 1; }}\n\
         {claims}",
        timed_world::DENSE,
    );
    timed_world::world(&package_document, super::PROFILES, &body)
}

fn claim_under(name: &str, scheduler: &str, bound: &str, formula: &str) -> String {
    format!(
        "probabilistic {name} using ev on M over (c: M::Call) {scheduler} on origin \
         {{ probability {bound} [ {formula} ] }}\n"
    )
}

/// Claims over [`one_step`] whose `step` fires after 2 ms and no later than
/// 3 ms, each with its value. The positions are the initial state at 0 ms,
/// the post-state of `step` and idle observations after it, so none has a
/// time in [1 ms, 2 ms]: an `eventually` or `until` there is false and an
/// `always` there is true, whatever the state holds during the delay.
const EMPTY_WINDOW: [(&str, &str, bool); 4] = [
    (
        "EventuallyNot",
        "eventually[1 ms, 2 ms] holds(not c.late)",
        false,
    ),
    (
        "UntilNot",
        "holds(not c.late) until[1 ms, 2 ms] holds(not c.late)",
        false,
    ),
    ("AlwaysLate", "always[1 ms, 2 ms] holds(c.late)", true),
    ("AlwaysNever", "always[1 ms, 2 ms] holds(c.retried)", true),
];

/// The claims of [`EMPTY_WINDOW`] under `scheduler` over [`one_step`]: for
/// each, a lower bound `Low` and an upper bound `High`.
fn empty_window_world(scheduler: &str, guard: &str, delay: &str) -> World {
    let claims = EMPTY_WINDOW
        .iter()
        .map(|(name, formula, _)| {
            claim_under(&format!("{name}Low"), scheduler, ">= 0.99", formula)
                + &claim_under(&format!("{name}High"), scheduler, "<= 0.01", formula)
        })
        .collect::<String>();
    one_step(guard, delay, &claims)
}

/// The bound each value of [`EMPTY_WINDOW`] meets is proved with that value
/// and an accepted certificate.
fn assert_window_proofs(world: &World) {
    for (name, _, holds) in EMPTY_WINDOW {
        let claim = format!("{name}{}", if holds { "Low" } else { "High" });
        let record = settle(world, &claim);
        assert_eq!(
            proved_value(&record),
            ratio(i64::from(holds), 1),
            "{claim}: {record:#?}"
        );
        assert!(accepted(world, &claim, &document(&record)), "{claim}");
    }
}

/// The bound each value of [`EMPTY_WINDOW`] misses is refuted.
fn assert_window_refutations(world: &World) {
    for (name, _, holds) in EMPTY_WINDOW {
        let claim = format!("{name}{}", if holds { "High" } else { "Low" });
        let record = settle(world, &claim);
        assert!(is_refuted(&record), "{claim}: {record:#?}");
    }
}

/// A delay the scheduler chooses unit by unit passes through 1 ms and 2 ms
/// without a position there.
#[trace("FR-204-AC-21")]
#[test]
fn a_free_delay_instant_is_not_a_predicate_position() {
    for scheduler in ["under every scheduler", "under One"] {
        let world = empty_window_world(scheduler, " and self.x >= 3 ms", "");
        assert_window_proofs(&world);
        assert_window_refutations(&world);
    }
}

/// A drawn delay of 3 ms passes the window in one race edge, under every
/// scheduler and under a workload.
#[trace("FR-204-AC-21")]
#[test]
fn a_drawn_delay_instant_is_not_a_predicate_position() {
    let delay = "delay M::Call::step ~ discrete { 3 ms: 1 };\n";
    for scheduler in ["under every scheduler", "under One"] {
        assert_window_proofs(&empty_window_world(scheduler, "", delay));
    }
}

/// A strict guard takes the region route, where `step` fires after 2 ms.
#[trace("FR-204-AC-21")]
#[test]
fn a_region_delay_instant_is_not_a_predicate_position() {
    let world = empty_window_world("under every scheduler", " and self.x > 2 ms", "");
    assert_window_proofs(&world);
    assert_window_refutations(&world);
}

/// A step at exactly a window's closed end is a position inside it.
#[trace("FR-204-AC-21")]
#[test]
fn a_step_at_a_closed_window_end_is_read() {
    let claims = claim_under(
        "AtLower",
        "under every scheduler",
        ">= 0.99",
        "eventually[3 ms, 4 ms] holds(c.late)",
    ) + &claim_under(
        "AtUpper",
        "under every scheduler",
        ">= 0.99",
        "eventually[1 ms, 3 ms] holds(c.late)",
    );
    let world = one_step(" and self.x >= 3 ms", "", &claims);
    for claim in ["AtLower", "AtUpper"] {
        let record = settle(&world, claim);
        assert_eq!(proved_value(&record), ratio(1, 1), "{claim}: {record:#?}");
        assert!(accepted(&world, claim, &document(&record)));
    }
}

/// A quiescent state with `late` true has one behaviour per choice of idle
/// observation times: one that observes inside [1 ms, 2 ms] satisfies the
/// event and one that first observes later does not, so the minimum is 0 and
/// the maximum 1, and each bound is refuted.
#[trace("FR-204-AC-21")]
#[test]
fn an_idle_observation_is_a_position_and_the_delay_before_it_is_not() {
    let formula = "eventually[1 ms, 2 ms] holds(c.late)";
    let claims = claim_under("Minimum", "under every scheduler", ">= 0.01", formula)
        + &claim_under("Maximum", "under every scheduler", "<= 0.99", formula);
    let package_document = timed_world::document(&["x"], &[("step", &["late", "x"])]);
    let body = format!(
        "{}\
         pre Step using v on M::Call::step {{ false }}\n\
         post Stepped using v on M::Call::step {{ self.late = false }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world_holding(
        &package_document,
        super::PROFILES,
        &body,
        &json!({ "late": { "boolean": true } }),
    );
    for claim in ["Minimum", "Maximum"] {
        let record = settle(&world, claim);
        assert!(is_refuted(&record), "{claim}: {record:#?}");
    }
}

/// `set` and `clear` tie at 1 ms and fire at that instant in either order, so
/// one of the positions at 1 ms has `late` true whatever follows it: the
/// probability is 1, both in the certificate and in the replayed paths that
/// refute the upper bound.
#[trace("FR-204-AC-21")]
#[test]
fn a_tied_step_does_not_retract_an_event_already_met() {
    let formula = "eventually[0 ms, 1 ms] holds(c.late)";
    let claims = claim_under("Met", "under Both", ">= 0.99", formula)
        + &claim_under("Missed", "under Both", "<= 0.99", formula);
    let package_document =
        timed_world::document(&["x"], &[("set", &["phase", "late"]), ("clear", &["late"])]);
    let body = format!(
        "{}\
         pre CanSet using v on M::Call::set {{ self.phase = 0 }}\n\
         post DidSet using v on M::Call::set {{ self.phase = 1 and self.late = true }}\n\
         pre CanClear using v on M::Call::clear {{ true }}\n\
         post DidClear using v on M::Call::clear {{ self.late = false }}\n\
         delay M::Call::set ~ discrete {{ 1 ms: 1 }};\n\
         delay M::Call::clear ~ discrete {{ 1 ms: 1 }};\n\
         workload Both on M {{ weight Call::set = 1; weight Call::clear = 1; }}\n\
         time invariant ClockWindow when {{ true }} {{ self.x <= 2 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world(&package_document, super::PROFILES, &body);
    let record = settle(&world, "Met");
    assert_eq!(proved_value(&record), ratio(1, 1), "{record:#?}");
    assert!(accepted(&world, "Met", &document(&record)));
    let record = settle(&world, "Missed");
    assert!(is_refuted(&record), "{record:#?}");
}
