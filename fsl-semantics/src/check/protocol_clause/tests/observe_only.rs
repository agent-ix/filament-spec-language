//! FR-218 "Observe-only boundaries" (TC-2989): an observe-only role or
//! control that reads and recommends checks, and every side-effecting action
//! behind the boundary refuses at the action itself.

use ix_trace_rs::trace;

use super::rules::{TRIED, located_refusals, protocol};
use crate::check::CheckRefusal;

fn catalog(refusal: &CheckRefusal) -> (&'static str, Option<&'static str>) {
    (refusal.cause.code().as_str(), refusal.cause.cause())
}

const ADVISE: &str = "recommend Advise by R on M::Actor::op contracts [] as (r: Boolean) { true };";

/// FR-218-AC-45: an observe-only role and an observe-only `sequence` that
/// read (`check`) and recommend both check, and a capture named `observe`
/// keeps its meaning.
#[trace("TC-2989", "FR-218-AC-45", "FSpec-TC-2016", "FSpec-FR-822-AC-1")]
#[test]
fn an_observe_only_step_checks() {
    let gate = "check Gate using v { true };";
    let by_role = protocol(
        "",
        "observe role R on M::Actor;",
        &format!("sequence Main {{ {gate} {ADVISE} }}"),
    );
    assert_eq!(located_refusals(&by_role).len(), 0, "{by_role}");
    let graph = super::checks(&by_role);
    let checked = graph.protocol("Flow").expect("checked protocol");
    assert!(
        checked
            .roles
            .iter()
            .find(|role| role.name == "R")
            .unwrap()
            .observe_only
    );

    let by_control = protocol(
        "",
        "role R on M::Actor;",
        &format!("observe sequence Review {{ {gate} {ADVISE} }}"),
    );
    assert_eq!(located_refusals(&by_control).len(), 0, "{by_control}");
    let graph = super::checks(&by_control);
    let checked = graph.protocol("Flow").expect("checked protocol");
    assert!(
        checked
            .controls
            .controls
            .iter()
            .find(|control| control.name == "Review")
            .unwrap()
            .observe_only
    );

    let named = protocol(
        "capture observe: Boolean = true;",
        "role R on M::Actor;",
        "sequence Main { check Gate using v { observe }; }",
    );
    super::checks(&named);
}

/// FR-218-AC-46: an `attempt` inside `observe sequence Review`, and one by an
/// observe-only role, each refuses `ill_typed`/`operator-ineligible` whose
/// region is exactly the attempt's declaration; the same attempt outside the
/// boundary checks.
#[trace("TC-2989", "FR-218-AC-46", "FSpec-TC-2016", "FSpec-FR-822-AC-2")]
#[test]
fn a_side_effect_under_observe_only_refuses() {
    super::checks(&protocol(
        "",
        "role R on M::Actor;",
        &format!("sequence Main {{ {TRIED} }}"),
    ));

    for (roles, run) in [
        (
            "role R on M::Actor;",
            format!("observe sequence Review {{ {TRIED} }}"),
        ),
        (
            "observe role R on M::Actor;",
            format!("sequence Main {{ {TRIED} }}"),
        ),
    ] {
        let found = located_refusals(&protocol("", roles, &run));
        let at: Vec<&String> = found
            .iter()
            .filter(|(refusal, _)| catalog(refusal) == ("ill_typed", Some("operator-ineligible")))
            .map(|(_, at)| at)
            .collect();
        assert_eq!(at, [TRIED], "{roles} {run}: {found:?}");
        assert_eq!(
            found.len(),
            1,
            "no refusal at the role or the control: {found:?}"
        );
    }

    let channel = "channel Note from R to S carries Boolean ordering unordered delivery at-least-once capacity 1 overflow block;";
    let sent = protocol(
        "",
        "role R on M::Actor; role S on M::Actor;",
        &format!("observe sequence Review {{ send Sent via Note as (sent: Boolean) {{ true }}; }}"),
    )
    .replace("run observe", &format!("{channel}\nrun observe"));
    let found = located_refusals(&sent);
    assert!(
        found.iter().any(|(refusal, at)| {
            catalog(refusal) == ("ill_typed", Some("operator-ineligible"))
                && at == "send Sent via Note as (sent: Boolean) { true };"
        }),
        "a send refuses at the action: {found:?}"
    );

    let effect = protocol(
        "",
        "role R on M::Actor;",
        &format!(
            "sequence Main {{ {TRIED} observe sequence Review {{ effect Applied of Main::Tried as (applied: Boolean) {{ true }}; }} }}"
        ),
    );
    let found = located_refusals(&effect);
    assert!(
        found.iter().any(|(refusal, at)| {
            catalog(refusal) == ("ill_typed", Some("operator-ineligible"))
                && at == "effect Applied of Main::Tried as (applied: Boolean) { true };"
        }),
        "an effect refuses at the action: {found:?}"
    );
}

/// FR-218-AC-47: a recommendation's result binder is never a decision fact.
#[trace("TC-2989", "FR-218-AC-47", "FSpec-TC-2016", "FSpec-FR-822-AC-3")]
#[test]
fn a_recommendation_result_is_not_a_decision_fact() {
    let run = format!(
        "sequence Main {{ {ADVISE} repeat Loop by R visible (r) while {{ true }} sequence Body {{ }} }}"
    );
    let found = located_refusals(&protocol("", "role R on M::Actor;", &run));
    let at_read: Vec<&String> = found
        .iter()
        .filter(|(refusal, _)| catalog(refusal) == ("ill_typed", Some("type-mismatch")))
        .map(|(_, at)| at)
        .collect();
    assert_eq!(
        at_read,
        ["r"],
        "the recommendation result is unreadable: {found:?}"
    );
}
