//! FR-218 "Observe-only boundaries" (TC-2989): an observe-only role or
//! control that reads and recommends checks, and every side-effecting action
//! behind the boundary refuses at the action itself.

use ix_trace_rs::trace;

use super::rules::{TRIED, located_refusals, protocol};
use crate::check::{CheckCause, CheckRefusal};

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
    let checked_role = graph.protocol("Flow").expect("checked protocol");
    assert!(
        checked_role
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
    let checked_control = graph.protocol("Flow").expect("checked protocol");
    assert!(
        checked_control
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

    let plain_graph = super::checks(&protocol(
        "",
        "role R on M::Actor;",
        &format!("sequence Review {{ {gate} {ADVISE} }}"),
    ));
    let plain = plain_graph
        .protocol("Flow")
        .expect("plain protocol checked");
    let mut observed_roles = checked_role.roles.clone();
    for role in &mut observed_roles {
        role.observe_only = false;
    }
    assert_eq!(
        observed_roles, plain.roles,
        "marking the role changes only its mark"
    );
    let mut observed_controls = checked_control.controls.clone();
    for control in &mut observed_controls.controls {
        control.observe_only = false;
    }
    assert_eq!(
        observed_controls, plain.controls,
        "marking the control changes only its mark"
    );
    assert_eq!(checked_control.recommendations, plain.recommendations);

    let source = protocol(
        "",
        "role R on M::Actor;",
        "observe observe sequence Review { check Gate using v { true }; }",
    );
    let source = format!("{}{}", super::HEADER, source);
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("observe-only", "unit.native"),
        "unit.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source reads");
    assert!(!parsed.is_admissible(), "a duplicated prefix is rejected");
    let [diagnostic] = parsed.diagnostics() else {
        panic!("one parser diagnostic: {:?}", parsed.diagnostics());
    };
    assert_eq!(diagnostic.code, fsl_cst::CompleteCode::InvalidSyntax);
    assert_eq!(diagnostic.cause.as_str(), "unexpected-token");
    let span = diagnostic
        .byte_span()
        .expect("the second marker is located");
    assert_eq!(
        &source[span.start..span.end],
        "observe",
        "the refusal is on the duplicate marker"
    );

    let contextual_names = super::checks(&protocol(
        "",
        "role observe on M::Actor;",
        "sequence Main { sequence EventScope { event recommend by observe as (seen: Boolean) { true }; } sequence RecommendScope { recommend recommend by observe on M::Actor::op contracts [] as (advice: Boolean) { true }; } }",
    ));
    let contextual = contextual_names.protocol("Flow").expect("protocol checked");
    assert_eq!(contextual.roles[0].name, "observe");
    assert!(!contextual.roles[0].observe_only);
    assert_eq!(contextual.recommendations.len(), 1);
    assert_eq!(contextual.recommendations[0].role, "observe");
    assert!(contextual.controls.controls.iter().any(|control| {
        control.kind == fsl_forms::ProtocolNodeKind::Event && control.name == "recommend"
    }));
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
        "observe sequence Review { send Sent via Note as (sent: Boolean) { true }; }",
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
    super::checks(&sent.replace("observe sequence Review", "sequence Review"));

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

    let nested = protocol(
        "",
        "role R on M::Actor;",
        &format!(
            "observe sequence Review {{ choice Pick by R visible () {{ case alpha when {{ true }} sequence First {{ {TRIED} }} case beta when {{ false }} sequence Second {{ commit Committed by R as (committed: Boolean) {{ true }}; }} }} }}"
        ),
    );
    let found = located_refusals(&nested);
    let actions: Vec<_> = found
        .iter()
        .filter_map(|(refusal, at)| match refusal.cause.as_ref() {
            CheckCause::ProtocolAnchor(cause) => match cause.as_ref() {
                crate::check::refusal::ProtocolAnchorCause::ObserveOnlyAction {
                    action,
                    boundary,
                    ..
                } => Some((*action, boundary.as_str(), at.as_str())),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(
        actions,
        [
            ("attempt", "Review", TRIED),
            (
                "commit",
                "Review",
                "commit Committed by R as (committed: Boolean) { true };"
            ),
        ],
        "nested-choice actions refuse at their own declarations in source order"
    );

    let actions = super::recovery_flow("Main::Committed", "");
    super::checks(&actions);

    let observed_role = actions.replace("role R on", "observe role R on");
    let found = located_refusals(&observed_role);
    let refusals: Vec<_> = found
        .iter()
        .filter_map(|(refusal, at)| match refusal.cause.as_ref() {
            CheckCause::ProtocolAnchor(cause) => match cause.as_ref() {
                crate::check::refusal::ProtocolAnchorCause::ObserveOnlyAction {
                    action,
                    boundary,
                    ..
                } => Some((*action, boundary.as_str(), at.as_str())),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(
        refusals,
        [
            ("compensate", "R", refusals[0].2),
            ("attempt", "R", TRIED),
            (
                "effect",
                "R",
                "effect Applied of Tried as (applied: Boolean) { true };"
            ),
            (
                "event-for",
                "R",
                "event Recovered by R for Undo as (notice: Boolean) { true };"
            ),
            (
                "commit",
                "R",
                "commit Committed by R as (committed: Boolean) { true };"
            ),
        ],
        "the role boundary reports each performed action once in source order"
    );
    assert!(
        refusals[0]
            .2
            .starts_with("compensate Undo for Main::Applied")
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

    let unknown_recommendation = protocol(
        "",
        "role R on M::Actor;",
        &format!(
            "sequence Main {{ {} }}",
            ADVISE.replace("M::Actor::op", "M::Actor::unknown")
        ),
    );
    let unknown_attempt = protocol(
        "",
        "role R on M::Actor;",
        &format!(
            "sequence Main {{ {} }}",
            TRIED.replace("M::Actor::op", "M::Actor::unknown")
        ),
    );
    let recommend_errors = super::assembly_errors(&unknown_recommendation);
    let attempt_errors = super::assembly_errors(&unknown_attempt);
    assert!(
        recommend_errors.iter().any(|error| matches!(
            error.cause,
            crate::check::assemble::AssemblyCause::UnresolvedOperation { ref operation, .. }
                if operation == "unknown"
        )),
        "an unknown recommended operation refuses: {recommend_errors:?}"
    );
    assert_eq!(recommend_errors.len(), attempt_errors.len());

    let foreign_contract = format!(
        "pre OtherOperation using v on M::Actor::other {{ true }}\n{}",
        protocol(
            "",
            "role R on M::Actor;",
            &format!(
                "sequence Main {{ {} }}",
                ADVISE.replace("contracts []", "contracts [OtherOperation]")
            ),
        )
    );
    let found = located_refusals(&foreign_contract);
    assert!(
        found
            .iter()
            .any(|(refusal, _)| refusal.cause.code().as_str() == "wrong_snapshot"),
        "a recommendation cannot select a foreign operation contract: {found:?}"
    );

    let while_guard = protocol(
        "",
        "role R on M::Actor;",
        &format!(
            "sequence Main {{ {} repeat Loop by R visible () while {{ r }} sequence Body {{ }} }}",
            ADVISE
        ),
    );
    let found = located_refusals(&while_guard);
    assert!(
        found.iter().any(|(refusal, at)| {
            catalog(refusal) == ("ill_typed", Some("type-mismatch")) && at == "r"
        }),
        "the recommendation result cannot be read in a while guard: {found:?}"
    );
}
