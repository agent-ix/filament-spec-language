// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-884-AC-5: minimal accepted/rejected pairs for the protocol constructs
//! whose local S3 refusals name a subject span.

use ix_trace_rs::trace;

use super::rules::{located_refusals, protocol, TRIED};
use crate::check::refusal::ProtocolAnchorCause;

fn only_locus(source: &str, expected_code: &str, expected_cause: Option<&str>) -> String {
    let found = located_refusals(source);
    let selected: Vec<_> = found
        .into_iter()
        .filter(|(refusal, _)| {
            refusal.cause.code().as_str() == expected_code
                && refusal.cause.cause() == expected_cause
        })
        .collect();
    assert_eq!(selected.len(), 1, "one expected refusal: {selected:?}");
    selected.into_iter().next().expect("one refusal").1
}

/// FR-884-AC-5: each small accepted control below checks, and its paired
/// rejection is located at the name, control header or join list it names.
/// QSL-2409's attempt-result reads are excluded because their FR-218 AC-44
/// behavior is not implemented yet.
#[trace("TC-2509", "FR-884-AC-5")]
#[test]
fn minimal_protocol_construct_pairs_check_and_refuse_at_their_subject() {
    let tried = format!("sequence Main {{ {TRIED} }}");

    // Roles: a declared role checks; a duplicate role is refused at each name.
    super::checks(&protocol("", "role R on M::Actor;", &tried));
    let duplicate_roles = protocol("", "role R on M::Actor; role R on M::Actor;", &tried);
    let role_regions: Vec<_> = located_refusals(&duplicate_roles)
        .into_iter()
        .filter(|(refusal, _)| {
            refusal.cause.code().as_str() == "ambiguous_declaration"
                && refusal.cause.cause() == Some("ambiguous-name")
        })
        .map(|(_, region)| region)
        .collect();
    assert_eq!(role_regions, ["R", "R"]);
    // Channels: a complete declaration checks; an undeclared endpoint is the
    // refusal subject, while the channel name remains outside the locus.
    let channel = "channel C from R to R carries Boolean ordering unordered delivery at-least-once capacity 2 overflow block;";
    super::checks(&protocol(
        "",
        &format!("role R on M::Actor; {channel}"),
        &tried,
    ));
    let missing_endpoint = protocol(
        "",
        "role R on M::Actor; channel C from R to Buyer carries Boolean ordering unordered delivery at-least-once capacity 2 overflow block;",
        &tried,
    );
    assert_eq!(
        only_locus(
            &missing_endpoint,
            "missing_declaration",
            Some("missing-name")
        ),
        "Buyer"
    );
    let duplicate_channels = protocol(
        "",
        &format!("role R on M::Actor; {channel} {channel}"),
        &tried,
    );
    let channel_regions: Vec<_> = located_refusals(&duplicate_channels)
        .into_iter()
        .filter(|(refusal, _)| {
            refusal.cause.code().as_str() == "ambiguous_declaration"
                && refusal.cause.cause() == Some("ambiguous-name")
        })
        .map(|(_, region)| region)
        .collect();
    assert_eq!(channel_regions, ["C", "C"]);
    let message_members = format!("role R on M::Actor; {channel}");
    let ordered_messages = "sequence Main { send Tx via C as (tx: Boolean) { true }; receive Rx via C of Tx as (rx: Boolean) { true }; }";
    super::checks(&protocol("", &message_members, ordered_messages));
    let cyclic_messages = "sequence Main { receive Rx via C of Tx as (rx: Boolean) { true }; send Tx via C as (tx: Boolean) { true }; }";
    assert_eq!(
        only_locus(
            &protocol("", &message_members, cyclic_messages),
            "invalid_package",
            Some("definition-cycle"),
        ),
        "Rx"
    );

    // Choice: complementary guards check; duplicate coverage refuses at the
    // choice header rather than at its protocol declaration.
    let choice = |first: &str, second: &str| {
        format!(
            "sequence Main {{ {TRIED} choice Pick by R visible () {{ \
             case yes when {{ {first} }} sequence Yes {{ }} \
             case no when {{ {second} }} sequence No {{ }} }} }}"
        )
    };
    super::checks(&protocol(
        "",
        "role R on M::Actor;",
        &choice("true", "false"),
    ));
    let missing_owner = protocol(
        "",
        "role R on M::Actor;",
        &choice("true", "false").replace("by R", "by Absent"),
    );
    let owner_regions: Vec<_> = located_refusals(&missing_owner)
        .into_iter()
        .map(|(_, region)| region)
        .collect();
    assert_eq!(owner_regions, ["Absent", "Absent"]);
    assert_eq!(
        only_locus(
            &protocol("", "role R on M::Actor;", &choice("true", "true")),
            "undefined_expression",
            Some("unproved-exhaustiveness"),
        ),
        "choice Pick by R"
    );

    // Repeat: an attempt body takes a step; an empty body refuses at the
    // repeat control that fails to make progress.
    let repeat = |body: &str| {
        format!(
            "sequence Main {{ repeat Loop by R visible () while {{ false }} \
             sequence Body {{ {body} }} }}"
        )
    };
    super::checks(&protocol("", "role R on M::Actor;", &repeat(TRIED)));
    assert_eq!(
        only_locus(
            &protocol("", "role R on M::Actor;", &repeat("")),
            "unsupported_construct",
            Some("declaration-form"),
        ),
        "Loop"
    );
    // A loop proof's variant: an integer checks; a Boolean is refused at the
    // variant.
    let variant = |variant: &str| {
        protocol(
            "",
            "role R on M::Actor;",
            &format!(
                "sequence Main {{ repeat Loop by R visible () \
                 invariant {{ true }} variant {{ {variant} }} while {{ false }} \
                 sequence Body {{ {TRIED} }} }}"
            ),
        )
    };
    super::checks(&variant("3"));
    assert_eq!(
        only_locus(&variant("true"), "ill_typed", Some("type-mismatch")),
        "true"
    );

    // Parallel and join: all joined branches check; an incomplete all-list
    // refuses at the join list.
    let parallel = |joined: &str| {
        format!(
            "parallel Both {{ branch a sequence A {{ }} \
             branch b sequence B {{ }} }} join all [{joined}];"
        )
    };
    let parallel_run = |joined: &str| format!("sequence Main {{ {} }}", parallel(joined));
    super::checks(&protocol("", "role R on M::Actor;", &parallel_run("a, b")));
    assert_eq!(
        only_locus(
            &protocol("", "role R on M::Actor;", &parallel_run("a")),
            "invalid_package",
            Some("invalid-value"),
        ),
        "[a]"
    );
    let duplicate_branches = protocol(
        "",
        "role R on M::Actor;",
        "sequence Main { parallel Both { branch east sequence A { } branch east sequence B { } } join all [east]; }",
    );
    let branch_regions: Vec<_> = located_refusals(&duplicate_branches)
        .into_iter()
        .filter(|(refusal, _)| {
            refusal.cause.code().as_str() == "ambiguous_declaration"
                && refusal.cause.cause() == Some("ambiguous-name")
        })
        .map(|(_, region)| region)
        .collect();
    assert_eq!(branch_regions, ["east", "east"]);

    // Protocol names: different names check; equal names are refused at each
    // declaration name. These are also the smallest accepted/rejected units.
    let flow = protocol("", "role R on M::Actor;", &tried);
    super::checks(&format!(
        "{flow}\n{}",
        flow.replace("protocol Flow", "protocol Drain")
    ));
    let duplicates = located_refusals(&format!("{flow}\n{flow}"));
    let protocol_regions: Vec<_> = duplicates
        .into_iter()
        .filter(|(refusal, _)| {
            refusal.cause.code().as_str() == "ambiguous_declaration"
                && refusal.cause.cause() == Some("ambiguous-name")
        })
        .map(|(_, region)| region)
        .collect();
    assert_eq!(protocol_regions, ["Flow", "Flow"]);

    // The subject names above come from typed causes, not source-text
    // retention: the direct over-read carries its owning slot and span.
    let over = protocol(
        "",
        "role R on M::Actor;",
        &choice("input.ready", "not input.ready"),
    );
    let causes: Vec<_> = located_refusals(&over)
        .into_iter()
        .filter_map(|(refusal, region)| match refusal.cause.as_ref() {
            crate::check::CheckCause::ProtocolAnchor(cause)
                if matches!(cause.as_ref(), ProtocolAnchorCause::SlotModelState { node, .. } if node == "Pick") =>
            {
                Some((cause.span(), region))
            }
            _ => None,
        })
        .collect();
    assert_eq!(causes.len(), 2);
    assert!(causes.iter().all(|(span, region)| {
        region == "input.ready" && span.end - span.start == "input.ready".len()
    }));
}
