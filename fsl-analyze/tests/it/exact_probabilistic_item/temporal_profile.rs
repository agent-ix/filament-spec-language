// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-196-AC-17's event-position false-extension bounded products.

use super::*;

#[trace("FR-196-AC-17", "FR-201-AC-13")]
#[test]
fn bounded_eventually_reads_each_position_and_settles_at_three_quarters() {
    let mut model = prob_world::health(
        "self.healthy = not fault",
        "true: 1, false: 1",
        false,
        "probabilistic Bounded using ev on M over (n: M::Node) under every scheduler on origin { probability >= 0.75 [ eventually[0,2] holds(n.healthy) ] }",
    );
    model.initial["healthy"] = serde_json::json!({"boolean": false});
    let world = World::new(&model);
    let record = settle(&world, "Bounded", ModelCheckLimits::default());
    let terminal = record.terminal().expect("the bounded claim settles");
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        certification: Certification::Certified,
    } = terminal.value()
    else {
        panic!(
            "the horizon-2 claim settles at its literal probability: {:#?}",
            terminal.value()
        );
    };
    assert_eq!(value.value(), &ratio(3, 4));
}

#[trace("FR-186-AC-3")]
#[test]
fn bounded_nested_eventually_always_uses_the_literal_horizon_110() {
    let claim = "probabilistic Nested using ev on M over (n: M::Node) under every scheduler on origin { probability >= 0.99 [ eventually[0,100] always[0,10] holds(n.healthy) ] }";
    let mut model = prob_world::health("self.healthy = true", "true: 1", false, claim);
    model.initial["healthy"] = serde_json::json!({"boolean": false});
    let world = World::new(&model);
    let record = settle(&world, "Nested", ModelCheckLimits::default());
    let terminal = record
        .terminal()
        .unwrap_or_else(|| panic!("the horizon-110 claim has a terminal: {record:#?}"));
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        certification: Certification::Certified,
    } = terminal.value()
    else {
        panic!(
            "the checked nested formula settles: {:#?}",
            terminal.value()
        );
    };
    assert_eq!(value.value(), &ratio(1, 1));
}

#[trace("FR-186-AC-7")]
#[test]
fn bounded_open_upper_event_interval_is_refused_as_an_expression_form() {
    let claim = "probabilistic OpenUpper using ev on M over (n: M::Node) under every scheduler on origin { probability >= 0.99 [ eventually[2, *] holds(n.healthy) ] }";
    let refusals = match World::try_new(&prob_world::health(
        "self.healthy = true",
        "true: 1",
        false,
        claim,
    )) {
        Err(refusals) => refusals,
        Ok(_) => panic!("an open upper interval is refused before EN-5"),
    };
    assert_eq!(refusals.len(), 1, "one temporal profile refusal");
    let refusal = &refusals[0];
    assert!(matches!(
        &*refusal.cause,
        fsl_semantics::check::CheckCause::Temporal(cause)
            if matches!(cause.as_ref(), fsl_semantics::check::TemporalCause::OpenInterval { .. })
    ));
    assert_eq!(refusal.cause.cause(), Some("expression-form"));
}

fn bounded_always_world() -> World {
    let package = "test/temporal-profile-always";
    let boolean = prob_world::BOOLEAN.to_owned();
    let operation = |name| prob_world::Operation {
        name,
        parameters: vec![("healthy", boolean.clone())],
        modifies: vec!["healthy"],
        deletes: false,
    };
    let model = prob_world::Model {
        package,
        object: "Node",
        population: "nodes",
        key: "n",
        integers: Vec::new(),
        fields: vec![("healthy", boolean.clone())],
        operations: vec![operation("choose_a"), operation("choose_b")],
        body: "\
            pre ReadyA using v on M::Node::choose_a { true }\n\
            post SetA using v on M::Node::choose_a { self.healthy = healthy }\n\
            pre ReadyB using v on M::Node::choose_b { true }\n\
            post SetB using v on M::Node::choose_b { self.healthy = healthy }\n\
            random Node::choose_a(healthy) ~ { true: 1, false: 3 };\n\
            random Node::choose_b(healthy) ~ { true: 3, false: 1 };\n\
            probabilistic Minimum using ev on M over (n: M::Node) under every scheduler on origin { probability >= 0.25 [ always[0,1] holds(n.healthy) ] }\n\
            probabilistic Maximum using ev on M over (n: M::Node) under every scheduler on origin { probability <= 0.75 [ always[0,1] holds(n.healthy) ] }\n"
            .to_owned(),
        initial: serde_json::json!({"healthy": {"boolean": true}}),
    };
    World::new(&model)
}

fn assert_exact_value(record: &fsl_analyze::analyze::AnalyzeRecord, expected: Rational) {
    let terminal = record.terminal().expect("the bounded claim settles");
    let TerminalValue::Proved {
        basis: ProofBasis::ExactValue { value, .. },
        certification: Certification::Certified,
    } = terminal.value()
    else {
        panic!(
            "the bounded claim settles at its literal bound: {:#?}",
            terminal.value()
        );
    };
    assert_eq!(value.value(), &expected);
}

#[trace("FR-196-AC-17", "FR-201-AC-16")]
#[test]
fn bounded_always_accepts_literal_minimum_and_maximum() {
    let world = bounded_always_world();
    assert_exact_value(
        &settle(&world, "Minimum", ModelCheckLimits::default()),
        ratio(1, 4),
    );
    assert_exact_value(
        &settle(&world, "Maximum", ModelCheckLimits::default()),
        ratio(3, 4),
    );
}
