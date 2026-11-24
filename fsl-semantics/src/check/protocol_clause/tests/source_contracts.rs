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
        let CheckCause::ProtocolAnchor(anchor) = refusal.cause.as_ref() else {
            panic!("expected a source protocol refusal: {refusal:?}");
        };
        let span = anchor.span();
        assert_eq!(span.start, expected);
        assert_eq!(span.end, expected + token.len());
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
        matches!(&checked.channels[0].capacity, fsl_forms::ChannelCapacity::Symbolic(name) if name == "Outstanding")
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
        &source.replace("[west, north, south]", "[west, north, missingBranch]"),
        "missing_declaration",
        "missing-name",
        "missingBranch",
        "missingBranch",
    );
}

#[trace("FR-218-AC-5", "FR-218-AC-33", "FR-884-AC-5")]
#[test]
fn repeat_with_a_real_step_admits_no_maximum_and_rejects_zero_maximum() {
    let source = protocol(
        "",
        ROLES,
        &format!(
            "sequence Main {{ repeat Deliver by Courier visible (dispatched) while {{ not dispatched }} sequence Body {{ {ATTEMPT} }} }}"
        ),
    );
    super::checks(&source);
    let invalid = protocol(
        "",
        ROLES,
        &format!(
            "sequence Main {{ repeat Deliver by Courier visible (dispatched) max 0 while {{ not dispatched }} sequence Body {{ {ATTEMPT} }} exhausted sequence Drained {{ }} }}"
        ),
    );
    assert_locus(
        &invalid,
        "invalid_package",
        "invalid-value",
        "Deliver",
        "Deliver",
    );
}

#[trace("FR-218-AC-17", "FR-218-AC-19", "FR-884-AC-5")]
#[test]
fn receiving_role_uses_a_visible_fact_but_sender_cannot_use_its_guard() {
    let members = format!(
        "{ROLES} channel Notices from Courier to Inspector carries Boolean ordering unordered delivery at-most-once capacity 1 overflow reject;"
    );
    let run = "sequence Main { send Offer via Notices as (offered: Boolean) { true }; receive Accept via Notices of Offer as (accepted: Boolean) { true }; choice Route by Inspector visible (accepted) { case proceed when { accepted } sequence Proceed { } case stop when { not accepted } sequence Stop { } } }";
    let source = protocol("", &members, run);
    super::checks(&source);
    let wrong_owner = source.replace("Route by Inspector", "Route by Courier");
    let found = located_refusals(&wrong_owner);
    let reads: Vec<_> = found
        .iter()
        .filter(|(refusal, _)| refusal.cause.code().as_str() == "wrong_snapshot")
        .collect();
    assert!(!reads.is_empty(), "{found:?}");
    let text = format!("{HEADER}{wrong_owner}\n");
    for (refusal, at) in reads {
        assert_eq!(refusal.cause.cause(), Some("wrong-observation"));
        assert_eq!(at, "accepted");
        let CheckCause::ProtocolAnchor(anchor) = refusal.cause.as_ref() else {
            panic!("{refusal:?}");
        };
        let span = anchor.span();
        assert_eq!(&text[span.start..span.end], "accepted");
        assert!(span.start > text.find("choice Route").expect("choice"));
        assert!(format!("{anchor:?}").contains("Route"));
    }
    let overlap = source.replace("not accepted", "accepted");
    assert_locus(
        &overlap,
        "undefined_expression",
        "unproved-exhaustiveness",
        "choice Route by Inspector",
        "Route",
    );
}

#[trace("FR-218-AC-35", "FR-884-AC-5")]
#[test]
fn awaited_receive_records_its_clock_and_rejects_an_inverted_window() {
    let members = format!(
        "{ROLES} channel Notices from Courier to Inspector carries Boolean ordering unordered delivery at-most-once capacity 2 overflow reject;"
    );
    let run = format!(
        "sequence Main {{ {ATTEMPT} send Offer via Notices as (offered: Boolean) {{ true }}; await Collect after Dispatch using e clock \"ticks\" within [1,4] match receive Delivered via Notices of Offer as (delivered: Boolean) {{ true }}; then sequence Arrived {{ }} timeout sequence Late {{ }} }}"
    );
    let source = protocol("", &members, &run);
    let graph = super::checks(&source);
    let checked = graph.protocol("Flow").expect("protocol");
    assert_eq!(checked.awaits[0].name, "Collect");
    assert_eq!(checked.awaits[0].clock, "ticks");
    assert!(checked.awaits[0].profile.is_some());
    assert_locus(
        &source.replace("[1,4]", "[4,1]"),
        "ill_typed",
        "type-mismatch",
        "[4,1]",
        "Collect",
    );
}

#[trace("FR-113-AC-3", "FR-884-AC-5")]
#[test]
fn compensation_template_resolves_the_named_commit_not_a_neighbouring_event() {
    let source = super::recovery_flow("Main::Committed", "")
        .replace("RecoveryFlow", "RepairFlow")
        .replace("Undo", "Restore");
    super::checks(&source);
    let invalid = source.replace("commit Main::Committed;", "commit Main::MissingCommit;");
    assert_locus(
        &invalid,
        "missing_declaration",
        "missing-name",
        "Main::MissingCommit",
        "MissingCommit",
    );
}

#[trace("FR-884-AC-2", "FR-884-AC-5")]
#[test]
fn nonadjacent_duplicate_roles_report_both_declaration_names() {
    let source = protocol("", ROLES, &format!("sequence Main {{ {ATTEMPT} }}"));
    super::checks(&source);
    let invalid = source.replace("run sequence", "role Courier on M::Actor;\nrun sequence");
    let text = format!("{HEADER}{invalid}\n");
    let expected: Vec<_> = text
        .match_indices("role Courier")
        .map(|(start, _)| start + "role ".len())
        .collect();
    let found = located_refusals(&invalid);
    let actual: Vec<_> = found
        .iter()
        .filter(|(refusal, _)| refusal.cause.code().as_str() == "ambiguous_declaration")
        .map(|(refusal, at)| {
            assert_eq!(refusal.cause.cause(), Some("ambiguous-name"));
            assert_eq!(at, "Courier");
            let CheckCause::ProtocolAnchor(anchor) = refusal.cause.as_ref() else {
                panic!("{refusal:?}");
            };
            let span = anchor.span();
            assert_eq!(span.end - span.start, "Courier".len());
            span.start
        })
        .collect();
    assert_eq!(actual, expected, "{found:?}");
}

mod population_fixture {
    use crate as semantics;
    include!("../../../../tests/fixtures/protocol_obligations.rs");
}

#[trace("FR-218-AC-28", "FR-884-AC-5")]
#[test]
fn replicated_role_selects_the_declared_population_and_rejects_an_absent_one() {
    use crate::model::domain_package::{DomainPackageRecord, Extent, PopulationRecord};
    use filament_checked::key::DeclarationKey;
    use filament_semantic_value::checking::{CheckMode, CheckingLimits};

    let key = |name: &str| DeclarationKey {
        package: "example/protocol-obligations".to_owned(),
        node: format!("ix://example/protocol-obligations/{name}"),
    };
    let source = format!(
        "{}protocol Crew using p over (input: M::Node) on origin {{ role courier each M::Node from couriers max 2 lifetime workflow; run sequence Main {{ event Delivery by courier as (delivered: M::Flag) {{ true }}; }} finish End as (outcome: Boolean) {{ true }}; }}",
        population_fixture::HEADER
    );
    for (text, missing) in [
        (&source, false),
        (
            &source.replace("from couriers", "from missingPopulation"),
            true,
        ),
    ] {
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("a", "u"),
            "unit.native",
            text.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("source admitted");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let model =
            population_fixture::obligation_model_with(vec![DomainPackageRecord::Population(
                PopulationRecord {
                    key: key("couriers"),
                    member_types: vec![key("Node")],
                    extent: Extent::Closed,
                    symmetric: false,
                },
            )]);
        let declarations = crate::check::PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "fixture".to_owned(),
            fsl_forms::build_unit(&parsed).expect("forms"),
            vec![model],
            Vec::new(),
        )
        .expect("assembled");
        let regions = declarations.regions();
        let result = declarations.check_with_protocol_results(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::check::test_support::ZeroRewardEvaluator,
        );
        if missing {
            let found = match result {
                Ok(graph) => graph
                    .protocol_refusals()
                    .iter()
                    .flat_map(|result| result.refusals.iter().cloned())
                    .collect(),
                Err(found) => found,
            };
            let matches: Vec<_> = found
                .iter()
                .filter(|refusal| {
                    refusal.cause.code().as_str() == "missing_declaration"
                        && refusal.cause.cause() == Some("missing-name")
                })
                .collect();
            assert_eq!(matches.len(), 1, "{found:?}");
            let refusal = matches[0];
            let CheckCause::ProtocolAnchor(anchor) = refusal.cause.as_ref() else {
                panic!("{refusal:?}");
            };
            let expected = text
                .find("missingPopulation")
                .expect("offending population");
            assert_eq!(anchor.span().start, expected);
            assert_eq!(anchor.span().end, expected + "missingPopulation".len());
            let region = regions.refusal_region(refusal).expect("source locus");
            assert_eq!(
                &text[usize::try_from(region.start()).expect("start")
                    ..usize::try_from(region.end()).expect("end")],
                "missingPopulation"
            );
            assert!(format!("{anchor:?}").contains("courier"));
        } else {
            let graph = result.expect("replicated source checks");
            let protocol = graph.protocol("Crew").expect("checked protocol");
            assert!(protocol.roles[0].replicated);
            assert_eq!(protocol.slots.roles[0].population, key("couriers"));
            assert_eq!(protocol.slots.roles[0].max, 2);
        }
    }
}

#[trace("FR-113-AC-3", "FR-884-AC-5")]
#[test]
fn await_after_an_undeclared_anchor_refuses_at_the_written_name() {
    let members = format!(
        "{ROLES} channel Notices from Courier to Inspector carries Boolean ordering unordered delivery at-most-once capacity 2 overflow reject;"
    );
    let run = format!(
        "sequence Main {{ {ATTEMPT} send Offer via Notices as (offered: Boolean) {{ true }}; await Collect after MissingDispatch using e clock \"ticks\" within [1,4] match receive Delivered via Notices of Offer as (delivered: Boolean) {{ true }}; then sequence Arrived {{ }} timeout sequence Late {{ }} }}"
    );
    let source = protocol("", &members, &run);
    assert_locus(
        &source,
        "missing_declaration",
        "missing-name",
        "MissingDispatch",
        "MissingDispatch",
    );
}
