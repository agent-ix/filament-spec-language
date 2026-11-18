//! FR-218 "Observe-only boundaries" (TC-2989): an observe-only role or
//! control that reads and recommends checks, and every side-effecting action
//! behind the boundary refuses at the action itself.

use ix_trace_rs::trace;

use super::rules::{located_refusals, protocol, TRIED};
use crate::check::CheckRefusal;

fn catalog(refusal: &CheckRefusal) -> (&'static str, Option<&'static str>) {
    (refusal.cause.code().as_str(), refusal.cause.cause())
}

const ADVISE: &str =
    "recommend Advise by R on M::Actor::op contracts [] as (r: Boolean) { true };";

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
    super::checks(&by_role);

    let by_control = protocol(
        "",
        "role R on M::Actor;",
        &format!("observe sequence Review {{ {gate} {ADVISE} }}"),
    );
    assert_eq!(located_refusals(&by_control).len(), 0, "{by_control}");
    super::checks(&by_control);

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
        assert_eq!(found.len(), 1, "no refusal at the role or the control: {found:?}");
    }
}
