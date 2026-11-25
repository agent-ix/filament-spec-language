// SPDX-License-Identifier: AGPL-3.0-or-later
//! Claims over timed subjects whose truth depends on a window holding no position,
//! over closed subjects. A dense scheduler can delay a non-integer time, so
//! the integer-time product does not give these claims' extrema.

use super::*;

/// The exact value of a proof, or the terminal or whole record otherwise.
fn outcome(record: &AnalyzeRecord) -> String {
    match record.terminal().map(|terminal| terminal.value()) {
        Some(TerminalValue::Proved {
            basis: ProofBasis::ExactValue { value, .. },
            ..
        }) => format!(
            "proved {}/{}",
            value.value().numerator().as_big(),
            value.value().denominator().as_big()
        ),
        Some(value) => format!("{value:?}"),
        None => format!("{record:?}"),
    }
}

/// The three every-scheduler claims over `formula`, all false when the dense
/// extrema are 0 and 1: a route whose maximum stops short proves `Max` with it,
/// and one whose minimum stops short proves `Min`.
fn extrema_claims(formula: &str) -> String {
    [("Max", "<= 0.999"), ("Min", ">= 0.001"), ("Half", "<= 0.5")]
        .map(|(name, bound)| {
            format!(
                "probabilistic {name} using ev on M over (c: M::Call) under every scheduler \
                 on origin {{ probability {bound} [ {formula} ] }}\n"
            )
        })
        .concat()
}

fn observed(world: &World) -> [(&'static str, String); 3] {
    ["Max", "Min", "Half"].map(|claim| (claim, outcome(&settle(world, claim))))
}

/// The dense values both counterexamples share: minimum 0, maximum 1.
fn dense_extrema() -> [(&'static str, String); 3] {
    [
        ("Max", "Refuted".to_owned()),
        ("Min", "Refuted".to_owned()),
        ("Half", "Refuted".to_owned()),
    ]
}

/// `Retx` holds exactly when no position follows another, the earlier one at
/// most 4 ms, by exactly 2 ms. Attempts at 1.5, 2.6, 3.7, 4.8 and 6.0 ms, and
/// idle observations clear of every earlier time plus 2 ms, satisfy it on
/// every draw, so the maximum is 1. A `send` at exactly 2 ms breaks it on
/// every draw, so the minimum is 0. At integer times the second attempt after
/// a loss at 1 ms falls on 2 or 3 ms, both 2 ms after a position.
#[trace("FR-204-AC-26")]
#[test]
fn nested_always_avoids_every_exact_offset_only_at_dense_times() {
    let claims = extrema_claims("always[0 ms, 4 ms] always[2 ms, 2 ms] holds(false)");
    let world = retx(GUARD, true, "", &claims);
    assert_eq!(observed(&world), dense_extrema());
}

/// `first` runs by 1 ms and resets `x`; `second` runs exactly 2 ms later,
/// after which time is free. `first` strictly inside (0, 1) ms puts the
/// positions at 0, `t` and `t + 2` ms, none of them in [1, 2] ms, so the
/// maximum is 1; `first` at 1 ms puts one there, so the minimum is 0. At
/// integer times `first` runs at 0 or 1 ms and a position always falls in the
/// window.
#[trace("FR-204-AC-26")]
#[test]
fn single_always_over_an_empty_window_needs_a_non_integer_delay() {
    let package_document = timed_world::document(
        &["x"],
        &[("first", &["phase", "x"]), ("second", &["phase"])],
    );
    let claims = extrema_claims("always[1 ms, 2 ms] holds(false)");
    let body = format!(
        "{}\
         pre First using v on M::Call::first {{ self.phase = 0 }}\n\
         post FirstDone using v on M::Call::first {{ self.phase = 1 and self.x = 0 ms }}\n\
         pre Second using v on M::Call::second {{ self.phase = 1 and self.x >= 2 ms }}\n\
         post SecondDone using v on M::Call::second {{ self.phase = 2 }}\n\
         time invariant Early when {{ self.phase = 0 }} {{ self.x <= 1 ms }}\n\
         time invariant Exact when {{ self.phase = 1 }} {{ self.x <= 2 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world(&package_document, super::PROFILES, &body);
    assert_eq!(observed(&world), dense_extrema());
}

/// A strict subject, so already decided over the region product. `first`
/// resets `x` strictly inside (0, 1) ms while `y` runs on; `third` needs
/// `x < 1 ms` and `y > 1 ms`, which `first` at 0.5 ms and `third` at 1.2 ms
/// meet. The maximum is 1, and a scheduler that never takes `third` gives the
/// minimum 0.
#[test]
fn region_delay_reaches_the_region_between_two_clocks_integer_crossings() {
    let package_document = timed_world::document(
        &["x", "y"],
        &[("first", &["phase", "x"]), ("third", &["phase", "late"])],
    );
    let claims = extrema_claims("eventually[0 ms, 4 ms] holds(c.late)");
    let body = format!(
        "{}\
         pre First using v on M::Call::first {{ self.phase = 0 and self.x > 0 ms and self.x < 1 ms }}\n\
         post FirstDone using v on M::Call::first {{ self.phase = 1 and self.x = 0 ms }}\n\
         pre Third using v on M::Call::third {{ self.phase = 1 and self.x < 1 ms and self.y > 1 ms }}\n\
         post ThirdDone using v on M::Call::third {{ self.phase = 2 and self.late = true }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world(&package_document, super::PROFILES, &body);
    for claim in ["Max", "Half"] {
        super::settle_tampered(&world, claim, move |certificate| {
            let one = super::ratio(1, 1);
            eprintln!(
                "{claim}: producer states {}, states of value one {}",
                certificate.values.len(),
                certificate
                    .values
                    .iter()
                    .filter(|entry| entry.value == one)
                    .count()
            );
        });
    }
    assert_eq!(observed(&world), dense_extrema());
}

/// `first` runs at a time `t` in [0, 1] ms, resets `x` and draws `lost`.
/// When lost, the measure starts at `t` and ends at 2 ms: `M = 2 - t`. When
/// not, it starts at 1 ms and ends at `t + 2` ms: `M = t + 1`. A `t`
/// strictly inside (0, 1) makes `M > 1 ms` on both draws, so the minimum of
/// `P(M <= 1 ms)` is 0 and the 0.4-quantile bound fails. At integer times one
/// draw always meets it, a minimum of one half.
#[test]
fn elapsed_upper_quantile_minimum_needs_a_non_integer_delay() {
    let claims = "probabilistic Quantile using ev on M over (c: M::Call) under every scheduler on origin \
         { quantile 0.4 of (elapsed from holds(c.inflight) until holds(c.late) within 4 ms) <= 1 ms }\n";
    let mut package: Value = serde_json::from_slice(&timed_world::document(
        &["x", "y"],
        &[
            ("first", &["phase", "inflight", "x"]),
            ("start", &["phase", "inflight"]),
            ("left", &["phase", "late"]),
            ("right", &["phase", "late"]),
        ],
    ))
    .expect("JSON");
    package["types"][1]["operations"][0]["params"] = json!([{
        "identity": timed_world::ident("Call/first/lost"),
        "name": "lost",
        "typeRef": "ix://filament/native/Boolean",
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {"source": {"sourceIdentity": timed_world::ident("spec"),
            "path": "spec.fspec", "startLine": 1, "startColumn": 1}},
    }]);
    let body = format!(
        "{}\
         pre First using v on M::Call::first {{ self.phase = 0 }}\n\
         post FirstDone using v on M::Call::first {{ self.phase = 1 and self.inflight = lost and self.x = 0 ms }}\n\
         random M::Call::first(lost) ~ {{ true: 1, false: 1 }};\n\
         pre Start using v on M::Call::start {{ self.phase = 1 and not self.inflight and self.y >= 1 ms }}\n\
         post Started using v on M::Call::start {{ self.phase = 2 and self.inflight = true }}\n\
         pre Left using v on M::Call::left {{ self.phase = 1 and self.inflight and self.y >= 2 ms }}\n\
         post LeftDone using v on M::Call::left {{ self.phase = 3 and self.late = true }}\n\
         pre Right using v on M::Call::right {{ self.phase = 2 and self.x >= 2 ms }}\n\
         post RightDone using v on M::Call::right {{ self.phase = 3 and self.late = true }}\n\
         time invariant Initial when {{ self.phase = 0 }} {{ self.y <= 1 ms }}\n\
         time invariant Lost when {{ self.phase = 1 and self.inflight }} {{ self.y <= 2 ms }}\n\
         time invariant Kept when {{ self.phase = 1 and not self.inflight }} {{ self.y <= 1 ms }}\n\
         time invariant Started when {{ self.phase = 2 }} {{ self.x <= 2 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world(
        &serde_json::to_vec(&package).expect("serializes"),
        PROFILES,
        &body,
    );
    let record = settle(&world, "Quantile");
    assert_eq!(outcome(&record), "Refuted", "{record:?}");
}

/// The two-step subject above, read by a temporal clause that every
/// behaviour has a position in [1, 2] ms. `first` strictly inside (0, 1) ms
/// leaves none, so both the zone route and the digital-clock route must
/// refute it.
#[test]
fn a_clause_violated_by_an_empty_window_is_refuted_on_both_routes() {
    let body = format!(
        "{}\
         pre First using v on M::Call::first {{ self.phase = 0 }}\n\
         post FirstDone using v on M::Call::first {{ self.phase = 1 and self.x = 0 ms }}\n\
         pre Second using v on M::Call::second {{ self.phase = 1 and self.x >= 2 ms }}\n\
         post SecondDone using v on M::Call::second {{ self.phase = 2 }}\n\
         time invariant Early when {{ self.phase = 0 }} {{ self.x <= 1 ms }}\n\
         time invariant Exact when {{ self.phase = 1 }} {{ self.x <= 2 ms }}\n\
         temporal Occupied using tt over (c: M::Call) clock \"model-time\" on origin \
         {{ eventually[1 ms, 2 ms] holds(c.phase >= 0) }}\n",
        timed_world::DENSE,
    );
    let unit = timed_world::unit_text(timed_world::PACKAGE, crate::zone_search::TIMED, &body);
    let document = timed_world::document(
        &["x"],
        &[("first", &["phase", "x"]), ("second", &["phase"])],
    );
    let workspace = crate::analyze_cli_support::Workspace::new("timed-empty-window");
    let run = crate::analyze_cli_support::analyze(&crate::analyze_cli_support::request_over(
        &workspace,
        &unit,
        &document,
        &[crate::analyze_family_support::rpc_snapshot()],
        json!({"items": [
            {"claim": "Occupied"},
            {"claim": "Occupied", "options": {"method": "digital-clocks"}},
        ]}),
    ));
    let items = run.document()["items"].clone();
    let observed = [0, 1].map(|at| {
        (
            items[at]["record"]["backend"].clone(),
            crate::analyze_cli_support::label(&items[at]).to_owned(),
        )
    });
    assert_eq!(
        observed,
        [
            (json!("en-6"), "refuted".to_owned()),
            (json!("en-1"), "refuted".to_owned()),
        ],
        "{items}"
    );
}

/// `send` is drawn at exactly 5 ms and `x <= 5 ms` forces time to stop
/// there, so the positions are at 0 and 5 ms and none is at 4 ms:
/// `eventually[4, 4]` is false on the only behaviour and the `<= 0.5` claim
/// holds. A sampler that adds its own position at the 4 ms horizon reads the
/// state there and rejects it.
#[test]
fn a_sampled_window_without_a_step_holds_no_position() {
    let claims = "delay M::Call::send ~ discrete { 5 ms: 1 };\n\
         workload Even on M { weight Call::send = 1; }\n\
         probabilistic Empty using ev on M over (c: M::Call) under Even on origin \
         with alpha 0.1, beta 0.1, indifference 0.1 \
         { probability <= 0.5 [ eventually[4 ms, 4 ms] holds(c.phase = 0) ] }\n";
    let body = format!(
        "{}\
         pre Send using v on M::Call::send {{ self.phase = 0 }}\n\
         post Sent using v on M::Call::send {{ self.phase = 1 }}\n\
         time invariant Wait when {{ self.phase = 0 }} {{ self.x <= 5 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world(
        &timed_world::document(&["x"], &[("send", &["phase"])]),
        PROFILES,
        &body,
    );
    let outcome = fsl_analyze::statistical::check_statistical(
        fsl_analyze::statistical::StatisticalRequest {
            workers: 1,
            subject: ModelSubject {
                simulation: world.request(),
                proof_bounds: Vec::new(),
            },
            claim: world
                .package
                .graph()
                .probabilistic_model()
                .claim("Empty")
                .expect("the claim checks"),
            method: fsl_replay::statistical::StatisticalMethod::Sprt,
            seed: 7,
            sampler: fsl_foundation::selection::DefinitionRef::new(
                "agent-ix",
                "filament.simulation.sampler/v1",
            )
            .expect("a valid DefinitionRef"),
            limits: fsl_replay::statistical::StatisticalLimits::default(),
            max_witnesses: 1,
            symmetry: Vec::new(),
            timed_sampling: fsl_replay::statistical::TimedSampling::default(),
        },
        &Cancel::new(),
    );
    let Ok(fsl_replay::statistical::StatisticalOutcome::Completed(verdict)) = outcome else {
        panic!("the sampled claim completes: {outcome:?}");
    };
    assert_eq!(
        verdict.decision,
        fsl_replay::statistical::Decision::Accepted,
        "{verdict:?}"
    );
}

/// `send` runs exactly at 2 ms, so the positions are 0 and 2 ms and then the
/// idle tail's. No position is at 1 ms, so `eventually[1, 1]` is false on
/// every behaviour and the `<= 0.5` claim holds. A product that reads a
/// position at the instant a delay passes 1 ms refutes it.
#[test]
fn a_nested_window_reads_no_position_inside_a_delay() {
    let claims = "probabilistic Nested using ev on M over (c: M::Call) under every scheduler \
         on origin { probability <= 0.5 [ eventually[1 ms, 1 ms] always[0 ms, 0 ms] \
         holds(c.phase = 0) ] }\n";
    let world = send_at_two(claims);
    assert_eq!(outcome(&settle(&world, "Nested")), "proved 0/1");
}

fn send_at_two(claims: &str) -> World {
    let body = format!(
        "{}\
         pre Send using v on M::Call::send {{ self.phase = 0 and self.x >= 2 ms }}\n\
         post Sent using v on M::Call::send {{ self.phase = 1 }}\n\
         time invariant Wait when {{ self.phase = 0 }} {{ self.x <= 2 ms }}\n\
         {claims}",
        timed_world::DENSE,
    );
    timed_world::world(
        &timed_world::document(&["x"], &[("send", &["phase"])]),
        PROFILES,
        &body,
    )
}

/// The timed measures a window of time or a long run reads, over a closed
/// subject: the exact engine decides none of them, so no integer-time value
/// can settle one.
#[test]
fn timed_measures_get_no_integer_time_verdict() {
    let forms = [
        ("Steps", "{ probability >= 0.5 [ steps from holds(not c.late) until holds(c.late) within 5 ms >= 1 ] }"),
        ("Quantile", "{ quantile 0.5 of (steps from holds(not c.late) until holds(c.late) within 5 ms) <= 3 }"),
        ("Fraction", "{ probability >= 0.5 [ fraction holds(c.late) over [0, 5 ms] weighted by delay <= rational(1, 2) ] }"),
        ("MeanFraction", "{ mean of (fraction holds(c.late) over [0, 5 ms] weighted by delay) >= 0.5 }"),
        ("LongRunDelay", "{ long_run_fraction holds(c.late) weighted by delay <= 0.5 }"),
        ("LongRun", "{ long_run_fraction holds(c.late) <= 0.5 }"),
        ("PositionFraction", "{ probability >= 0.5 [ fraction holds(c.late) over [0, 5 ms] <= rational(1, 2) ] }"),
    ];
    let mut observed = Vec::new();
    for (name, form) in forms {
        for scheduler in ["every scheduler", "One"] {
            let workload = if scheduler == "One" {
                "workload One on M { weight Call::send = 1; }\n"
            } else {
                ""
            };
            let claims = format!(
                "{workload}probabilistic {name} using ev on M over (c: M::Call) under {scheduler} \
                 on origin {form}\n"
            );
            let world = retx(GUARD, true, "", &claims);
            let record = settle(&world, name);
            let mut text = outcome(&record);
            text.truncate(160);
            observed.push(format!("{name} under {scheduler}: {text}"));
        }
    }
    eprintln!("{}", observed.join("\n"));
    assert!(
        observed
            .iter()
            .all(|line| !line.contains(": proved") && !line.contains(": Refuted")),
        "{observed:#?}"
    );
}

/// `send` is drawn at 2 ms and sets `late`, so the window holds position 0,
/// where `late` is false, and the `send` position, where it is true: at least
/// one half of the positions in [0, 5] ms satisfy it, and the claim that the
/// fraction reaches one tenth holds on the only behaviour, whether the window's
/// count is divided by its positions or by `h + 1`.
#[test]
fn a_sampled_position_fraction_over_a_time_window_reads_the_window() {
    let claims = "delay M::Call::send ~ discrete { 2 ms: 1 };\n\
         workload Even on M { weight Call::send = 1; }\n\
         probabilistic Share using ev on M over (c: M::Call) under Even on origin \
         with alpha 0.1, beta 0.1, indifference 0.1 \
         { probability >= 0.5 [ fraction holds(c.late) over [0, 5 ms] >= rational(1, 10) ] }\n";
    let body = format!(
        "{}\
         pre Send using v on M::Call::send {{ not self.late }}\n\
         post Sent using v on M::Call::send {{ self.late = true }}\n\
         {claims}",
        timed_world::DENSE,
    );
    let world = timed_world::world(
        &timed_world::document(&["x"], &[("send", &["late"])]),
        PROFILES,
        &body,
    );
    let outcome = fsl_analyze::statistical::check_statistical(
        fsl_analyze::statistical::StatisticalRequest {
            workers: 1,
            subject: ModelSubject {
                simulation: world.request(),
                proof_bounds: Vec::new(),
            },
            claim: world
                .package
                .graph()
                .probabilistic_model()
                .claim("Share")
                .expect("the claim checks"),
            method: fsl_replay::statistical::StatisticalMethod::Sprt,
            seed: 7,
            sampler: fsl_foundation::selection::DefinitionRef::new(
                "agent-ix",
                "filament.simulation.sampler/v1",
            )
            .expect("a valid DefinitionRef"),
            limits: fsl_replay::statistical::StatisticalLimits::default(),
            max_witnesses: 1,
            symmetry: Vec::new(),
            timed_sampling: fsl_replay::statistical::TimedSampling::default(),
        },
        &Cancel::new(),
    );
    let Ok(fsl_replay::statistical::StatisticalOutcome::Completed(verdict)) = outcome else {
        panic!("the sampled claim completes: {outcome:?}");
    };
    assert_eq!(
        verdict.decision,
        fsl_replay::statistical::Decision::Accepted,
        "{verdict:?}"
    );
}
