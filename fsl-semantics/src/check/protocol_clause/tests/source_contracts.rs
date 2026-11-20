// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-level control contracts with absolute refusal positions.

use ix_trace_rs::trace;

use super::rules::{located_refusals, protocol};
use super::{CheckCause, HEADER};

const ROLES: &str = "role Courier on M::Actor; role Inspector on M::Actor;";
const ATTEMPT: &str =
    "attempt Dispatch by Courier on M::Actor::op contracts [] as (dispatched: Boolean) { true };";

fn assert_locus(source: &str, code: &str, cause: &str, token: &str, subject: &str) {
    let text = format!("{HEADER}{source}\n");
    let expected = text.rfind(token).expect("the offending source exists");
    let found = located_refusals(source);
    let selected: Vec<_> = found
        .iter()
        .filter(|(refusal, _)| {
            refusal.cause.code().as_str() == code && refusal.cause.cause() == Some(cause)
        })
        .collect();
    assert!(!selected.is_empty(), "missing {code}/{cause}: {found:?}");
    for (refusal, region) in selected {
        assert_eq!(region, token, "{refusal:?}");
        let CheckCause::ProtocolAnchor(anchor) = &refusal.cause else {
            panic!("expected a source protocol refusal: {refusal:?}");
        };
        let span = anchor.span();
        assert_eq!(usize::try_from(span.start()).expect("start"), expected);
        assert_eq!(
            usize::try_from(span.end()).expect("end"),
            expected + token.len()
        );
        assert!(token.len() < source.len());
        assert!(format!("{anchor:?}").contains(subject), "{anchor:?}");
    }
}

#[trace("FR-218-AC-10", "FR-218-AC-13", "FR-884-AC-5")]
#[test]
fn symbolic_capacity_survives_and_unknown_destination_names_its_endpoint() {
    let channel = "channel Notices from Courier to Inspector carries Boolean ordering unordered delivery at-least-once capacity symbolic(Outstanding) overflow reject;";
    let source = protocol(
        "",
        &format!("{ROLES} {channel}"),
        &format!("sequence Main {{ {ATTEMPT} }}"),
    );
    let graph = super::checks(&source);
    let checked = graph.protocol("Flow").expect("checked protocol");
    assert_eq!(checked.channels[0].name, "Notices");
    assert!(
        matches!(&checked.channels[0].capacity, fsl_forms::ChannelCapacity::Symbolic(name) if name.name == "Outstanding")
    );
    let invalid = source.replace("to Inspector", "to MissingReceiver");
    assert_locus(
        &invalid,
        "missing_declaration",
        "missing-name",
        "MissingReceiver",
        "MissingReceiver",
    );
}

#[trace("FR-218-AC-12", "FR-884-AC-5")]
#[test]
fn fifo_key_binder_type_is_checked_independently_of_message_body() {
    let members = format!(
        "{ROLES} channel Notices from Courier to Inspector carries Boolean ordering fifo by (route: Boolean) {{ route }} delivery at-least-once capacity 3 overflow reject;"
    );
    let source = protocol(
        "",
        &members,
        "sequence Main { send Offer via Notices as (offered: Boolean) { true }; receive Accept via Notices of Offer as (accepted: Boolean) { true }; }",
    );
    super::checks(&source);
    let invalid = source.replace("route: Boolean", "route: Integer");
    assert_locus(&invalid, "ill_typed", "type-mismatch", "Integer", "Notices");
}

#[trace("FR-218-AC-2", "FR-218-AC-23", "FR-884-AC-5")]
#[test]
fn three_branch_all_join_resolves_every_label_and_covers_every_branch() {
    let run = "sequence Main { parallel Dispatches { branch north sequence North { } branch south sequence South { } branch west sequence West { } } join all [west, north, south]; }";
    let source = protocol("", ROLES, run);
    let graph = super::checks(&source);
    assert_eq!(
        graph.protocol("Flow").expect("protocol").parallels[0].joined,
        ["west", "north", "south"]
    );
    assert_locus(
        &source.replace("[west, north, south]", "[west, north]"),
        "invalid_package",
        "invalid-value",
        "[west, north]",
        "Dispatches",
    );
    assert_locus(
        &source.replace("[west, north, south]", "[west, north, absent]"),
        "missing_declaration",
        "missing-name",
        "absent",
        "absent",
    );
}

#[trace("FR-218-AC-5", "FR-218-AC-33", "FR-884-AC-5")]
#[test]
fn repeat_with_a_real_step_admits_no_maximum_and_rejects_zero_maximum() {
    let source = protocol(
        "",
        ROLES,
        &format!(
            "sequence Main {{ repeat Deliver by Courier visible () while {{ false }} sequence Body {{ {ATTEMPT} }} }}"
        ),
    );
    super::checks(&source);
    let invalid = source.replace("visible () while", "visible () max 0 while");
    assert_locus(
        &invalid,
        "invalid_package",
        "invalid-value",
        "Deliver",
        "Deliver",
    );
}
