// SPDX-License-Identifier: AGPL-3.0-or-later
//! Protocol productions preserve policy children, scoped binders and source locations.

use fsl_cst::{
    CompleteCause, CompleteCode, CstElement, CstNode, Limits, ParsedSource, Production, TokenKind,
};
use fsl_foundation::{SourceIdentity, Span};
use ix_trace_rs::trace;

fn unit(members: &str, control: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\r\n\
         // λ makes character positions differ from byte positions\r\n\
         profile p = \"filament.protocol.finite-global/v1\";\r\n\
         profile t = \"filament.temporal.event-position.false-extension/v1\";\r\n\
         model Ledger = \"example/parser-ledger\";\r\n\
         protocol Dispatch using p over (ticket: Ledger::Ticket) on origin {{\n\
         role Agent on Ledger::Actor;\n\
         {members}\nrun {control}\n\
         finish Closed as (outcome: Boolean) {{ outcome }};\n}}"
    )
}

fn parsed(source: &str) -> ParsedSource {
    fsl_cst::parse(
        SourceIdentity::new("test", "test:production-protocol-forms"),
        "production-protocol-forms.native",
        source.as_bytes(),
        Limits::default(),
    )
    .expect("the authored UTF-8 source is readable")
}

fn admitted(source: &str) -> ParsedSource {
    let result = parsed(source);
    assert!(
        result.is_admissible(),
        "{source}: {:?}",
        result.diagnostics()
    );
    assert_eq!(result.cst().render(), source.as_bytes());
    result
}

fn text<'a>(source: &'a str, node: &CstNode) -> &'a str {
    let span = node.span();
    &source[span.start..span.end]
}

fn node<'a>(
    parsed: &'a ParsedSource,
    source: &str,
    production: Production,
    expected: &str,
) -> &'a CstNode {
    let candidates: Vec<&CstNode> = parsed
        .cst()
        .nodes()
        .iter()
        .filter(|item| item.production() == production && text(source, item) == expected)
        .collect();
    let [found] = candidates.as_slice() else {
        panic!("one {production:?} with spelling {expected:?}: {candidates:?}");
    };
    assert_eq!(
        parsed.cst().render_node(found).unwrap(),
        expected.as_bytes()
    );
    found
}

fn children<'a>(parsed: &'a ParsedSource, parent: &CstNode) -> Vec<&'a CstNode> {
    parent
        .children()
        .iter()
        .filter_map(|element| match element {
            CstElement::Node(index) => Some(&parsed.cst().nodes()[*index]),
            CstElement::Token(_) => None,
        })
        .collect()
}

fn child_texts<'a>(
    parsed: &ParsedSource,
    source: &'a str,
    parent: &CstNode,
    production: Production,
) -> Vec<&'a str> {
    children(parsed, parent)
        .into_iter()
        .filter(|item| item.production() == production)
        .map(|item| text(source, item))
        .collect()
}

fn direct_identifiers<'a>(
    parsed: &ParsedSource,
    source: &'a str,
    parent: &CstNode,
) -> Vec<&'a str> {
    parent
        .children()
        .iter()
        .filter_map(|element| match element {
            CstElement::Token(index) => Some(&parsed.cst().tokens()[*index]),
            CstElement::Node(_) => None,
        })
        .filter(|token| token.kind() == TokenKind::Identifier)
        .map(|token| {
            let span = token.span();
            &source[span.start..span.end]
        })
        .collect()
}

fn rejected(source: &str, marker: &str, width: usize) -> ParsedSource {
    let result = parsed(source);
    assert!(!result.is_admissible(), "{source}");
    let diagnostic = result
        .diagnostics()
        .first()
        .expect("a located syntax refusal");
    assert_eq!(
        diagnostic.code,
        CompleteCode::InvalidSyntax,
        "{diagnostic:?}"
    );
    assert_eq!(
        diagnostic.cause,
        CompleteCause::UnexpectedToken,
        "{diagnostic:?}"
    );
    let start = source.rfind(marker).expect("the authored offending token");
    assert_eq!(
        diagnostic.byte_span(),
        Some(Span {
            start,
            end: start + width
        }),
        "{source}: {diagnostic:?}"
    );
    result
}

fn rejected_with_expected(source: &str, marker: &str, width: usize, expected: &str) {
    let result = rejected(source, marker, width);
    assert_eq!(result.cst().render(), source.as_bytes());
    let [diagnostic] = result.diagnostics() else {
        panic!("one independent syntax error: {:?}", result.diagnostics());
    };
    // The public diagnostic renders its deterministic expected alternatives.
    // Code, cause and location remain separate typed assertions in rejected.
    assert_eq!(diagnostic.message, expected, "{source}: {diagnostic:?}");
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn channel_policies_attach_to_their_own_channel_in_source_order() {
    for (ordering, delivery, capacity, overflow) in [
        ("unordered", "unknown", "0", "block"),
        (
            "fifo by (packet: Envelope) { packet.slot }",
            "at-most-once",
            "1",
            "reject",
        ),
        ("unordered", "at-least-once", "symbolic(pool)", "loss head"),
        (
            "unordered",
            "exactly-once-premise",
            "18446744073709551615",
            "loss tail",
        ),
        ("unordered", "[1, 3]", "2", "loss new"),
    ] {
        let channel = format!(
            "channel Inbox from Agent to Peer via Links carries Envelope ordering {ordering} delivery {delivery} capacity {capacity} overflow {overflow};"
        );
        let source = unit(
            &format!("role Peer on Ledger::Actor;\nrelationship Links = Ledger::links;\n{channel}"),
            "sequence Empty {}",
        );
        let result = admitted(&source);
        let channel = node(&result, &source, Production::Channel, &channel);
        let policies: Vec<(Production, &str)> = children(&result, channel)
            .into_iter()
            .filter(|item| {
                matches!(
                    item.production(),
                    Production::Ordering
                        | Production::DeliveryPolicy
                        | Production::Capacity
                        | Production::OverflowPolicy
                )
            })
            .map(|item| (item.production(), text(&source, item)))
            .collect();
        assert_eq!(
            policies,
            [
                (Production::Ordering, ordering),
                (Production::DeliveryPolicy, delivery),
                (Production::Capacity, capacity),
                (Production::OverflowPolicy, overflow),
            ]
        );
        let relation = node(
            &result,
            &source,
            Production::Relationship,
            "relationship Links = Ledger::links;",
        );
        assert_eq!(
            child_texts(&result, &source, relation, Production::ModelName),
            ["Ledger::links"]
        );
        if ordering.starts_with("fifo") {
            let order = node(&result, &source, Production::Ordering, ordering);
            assert_eq!(
                child_texts(&result, &source, order, Production::Parameter),
                ["packet: Envelope"]
            );
            assert_eq!(
                child_texts(&result, &source, order, Production::Block),
                ["{ packet.slot }"]
            );
        }
        if delivery.starts_with('[') {
            let policy = node(&result, &source, Production::DeliveryPolicy, delivery);
            assert_eq!(
                child_texts(&result, &source, policy, Production::DeliveryInterval),
                [delivery]
            );
        }
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn dynamic_role_lifetimes_and_observation_keep_the_role_parent() {
    for lifetime in ["workflow", "scope", "until (ticket.closed)"] {
        let role = format!(
            "observe role Helpers each Ledger::Actor from candidates max 2 lifetime {lifetime};"
        );
        let source = unit(&role, "sequence Empty {}");
        let result = admitted(&source);
        let role = node(&result, &source, Production::Role, &role);
        assert_eq!(
            child_texts(&result, &source, role, Production::RoleLifetime),
            [lifetime]
        );
        assert_eq!(
            child_texts(&result, &source, role, Production::QualifiedName),
            ["Ledger::Actor"]
        );
        if lifetime.starts_with("until") {
            let lifetime = node(&result, &source, Production::RoleLifetime, lifetime);
            assert_eq!(
                child_texts(&result, &source, lifetime, Production::Expression),
                ["ticket.closed"]
            );
        }
    }
}

const COMPENSATION: &str = "compensate Restore for Work::Offered as (offer: Offer) by Agent on Ledger::Ticket::restore using t clock \"ticks\" {\n  capture original: Integer = offer.amount;\n  activate first (signal: Signal) when { signal.failed } {\n    capture checkpoint: Integer = original;\n  } within [1, 4];\n  attempts 3 of RetryOffer;\n  retry (beforeState: Before, afterState: After) { beforeState.amount = afterState.amount };\n  commit Work::Posted;\n  recover (restored: State) { restored.clean };\n}";

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn compensation_preserves_capture_phases_retry_parameters_and_commit_reference() {
    let source = unit(COMPENSATION, "sequence Work {}");
    let result = admitted(&source);
    let compensation = node(&result, &source, Production::Compensation, COMPENSATION);
    assert_eq!(
        child_texts(&result, &source, compensation, Production::NodeReference),
        ["Work::Offered", "Work::Posted"]
    );
    assert_eq!(
        child_texts(&result, &source, compensation, Production::Capture),
        [
            "capture original: Integer = offer.amount;",
            "capture checkpoint: Integer = original;"
        ]
    );
    assert_eq!(
        child_texts(&result, &source, compensation, Production::Parameter),
        [
            "offer: Offer",
            "signal: Signal",
            "beforeState: Before",
            "afterState: After",
            "restored: State"
        ]
    );
    assert_eq!(
        child_texts(&result, &source, compensation, Production::Block),
        [
            "{ signal.failed }",
            "{ beforeState.amount = afterState.amount }",
            "{ restored.clean }"
        ]
    );
    assert_eq!(
        child_texts(&result, &source, compensation, Production::Interval),
        ["[1, 4]"]
    );
    let captures = children(&result, compensation)
        .into_iter()
        .filter(|item| item.production() == Production::Capture)
        .collect::<Vec<_>>();
    let activate = source.find("activate first").unwrap();
    assert!(captures[0].span().end < activate);
    assert!(captures[1].span().start > activate);
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn compensation_never_commit_and_zero_attempts_remain_syntax_for_the_checker() {
    let compensation = "compensate RetryLater for Offered as (offer: Offer) by Agent on Ledger::Ticket::restore using t clock \"ticks\" { activate first (signal: Signal) when { true } {} within [0, 0]; attempts 0 of RetryOffer; retry (beforeState: Before, afterState: After) { true }; commit never; recover (restored: State) { true }; }";
    let source = unit(compensation, "sequence Empty {}");
    let result = admitted(&source);
    let item = node(&result, &source, Production::Compensation, compensation);
    assert_eq!(
        child_texts(&result, &source, item, Production::NodeReference),
        ["Offered"]
    );
    assert!(child_texts(&result, &source, item, Production::Capture).is_empty());
    assert_eq!(
        child_texts(&result, &source, item, Production::TypeName),
        ["RetryOffer"]
    );
    assert!(text(&source, item).contains("attempts 0 of RetryOffer;"));
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn protocol_requirement_terminal_scheduling_and_run_keep_declaration_order() {
    let members = format!("requires temporal Ledger::Settled;\n{COMPENSATION}\nterminal when {{ ticket.closed }};\nscheduling adversarial;");
    let source = unit(&members, "sequence Empty {}");
    let result = admitted(&source);
    let protocol = result
        .cst()
        .nodes()
        .iter()
        .find(|item| item.production() == Production::ProtocolClause)
        .unwrap();
    let selected: Vec<Production> = children(&result, protocol)
        .into_iter()
        .map(|item| item.production())
        .filter(|kind| {
            matches!(
                kind,
                Production::ProtocolRequirement
                    | Production::Compensation
                    | Production::ProtocolTerminal
                    | Production::ProtocolScheduling
                    | Production::Control
                    | Production::Finish
            )
        })
        .collect();
    assert_eq!(
        selected,
        [
            Production::ProtocolRequirement,
            Production::Compensation,
            Production::ProtocolTerminal,
            Production::ProtocolScheduling,
            Production::Control,
            Production::Finish
        ]
    );
    let terminal = node(
        &result,
        &source,
        Production::ProtocolTerminal,
        "terminal when { ticket.closed };",
    );
    assert_eq!(
        child_texts(&result, &source, terminal, Production::Block),
        ["{ ticket.closed }"]
    );
    let source = unit("terminal any;", "sequence Empty {}");
    let result = admitted(&source);
    let terminal = node(
        &result,
        &source,
        Production::ProtocolTerminal,
        "terminal any;",
    );
    assert!(child_texts(&result, &source, terminal, Production::Block).is_empty());
}

const AWAIT: &str = "await Arrival after Work::Offered using t clock \"ticks\" within [1, 4] match receive Got via Inbox of Work::Offered as (message: Envelope) { message.ok }; then sequence Accepted {} timeout sequence Expired {}";

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn observed_await_keeps_match_and_both_arms_inside_the_await() {
    let control = format!("observe {AWAIT}");
    let source = unit("", &control);
    let result = admitted(&source);
    let outer = node(&result, &source, Production::Control, &control);
    assert_eq!(
        child_texts(&result, &source, outer, Production::AwaitControl),
        [AWAIT]
    );
    let wait = node(&result, &source, Production::AwaitControl, AWAIT);
    assert_eq!(
        child_texts(&result, &source, wait, Production::NodeReference),
        ["Work::Offered"]
    );
    assert_eq!(
        child_texts(&result, &source, wait, Production::Interval),
        ["[1, 4]"]
    );
    assert_eq!(
        child_texts(&result, &source, wait, Production::EventNode),
        ["receive Got via Inbox of Work::Offered as (message: Envelope) { message.ok };"]
    );
    assert_eq!(
        child_texts(&result, &source, wait, Production::Control),
        ["sequence Accepted {}", "sequence Expired {}"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn joins_preserve_branch_order_policy_memory_and_roster_attachment() {
    for (memory, policy, outstanding) in [
        (" memory sc", "all", ""),
        (" memory tso", "any", " outstanding continue"),
        (" memory ra", "quorum(2)", " outstanding cancel"),
        ("", "predicate(ready)", ""),
    ] {
        let control = format!("parallel Fanout{memory} {{ branch accepted sequence FirstPath {{}} branch delivered sequence SecondPath {{}} }} join {policy} [accepted, delivered]{outstanding};");
        let source = unit("", &control);
        let result = admitted(&source);
        let parallel = node(&result, &source, Production::Parallel, &control);
        assert_eq!(
            child_texts(&result, &source, parallel, Production::Branch),
            [
                "branch accepted sequence FirstPath {}",
                "branch delivered sequence SecondPath {}"
            ]
        );
        assert_eq!(
            child_texts(&result, &source, parallel, Production::JoinPolicy),
            [policy]
        );
        assert_eq!(
            direct_identifiers(&result, &source, parallel),
            ["Fanout", "accepted", "delivered"]
        );
        let expected_memory: Vec<&str> = if memory.is_empty() {
            vec![]
        } else {
            vec![memory.trim()]
        };
        assert_eq!(
            child_texts(&result, &source, parallel, Production::MemoryClause),
            expected_memory
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn repeats_keep_proof_guard_body_and_exhaustion_in_their_own_children() {
    let bounded = "repeat Drain by Agent visible (ticket.ready) max 3 invariant { ticket.count >= 0 } variant { ticket.count } while { ticket.ready } sequence More {} exhausted sequence Spent {}";
    let source = unit("", bounded);
    let result = admitted(&source);
    let repeat = node(&result, &source, Production::Repetition, bounded);
    assert_eq!(
        child_texts(&result, &source, repeat, Production::Block),
        [
            "{ ticket.count >= 0 }",
            "{ ticket.count }",
            "{ ticket.ready }"
        ]
    );
    assert_eq!(
        child_texts(&result, &source, repeat, Production::Control),
        ["sequence More {}", "sequence Spent {}"]
    );
    let unbounded = "repeat Drain by Agent visible () while { ticket.ready } sequence More {}";
    let source = unit("", unbounded);
    let result = admitted(&source);
    let repeat = node(&result, &source, Production::Repetition, unbounded);
    assert_eq!(
        child_texts(&result, &source, repeat, Production::Block),
        ["{ ticket.ready }"]
    );
    assert_eq!(
        child_texts(&result, &source, repeat, Production::Control),
        ["sequence More {}"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn choice_guards_and_visibility_attach_to_cases_in_source_order() {
    let control = "choice Routing by Agent visible (ticket.ready, ticket.count) { case Ready when { ticket.ready } sequence Start {} case Waiting when { not ticket.ready } sequence Pause {} case Retryable when { ticket.count > 0 } sequence Again {} }";
    let source = unit("", control);
    let result = admitted(&source);
    let choice = node(&result, &source, Production::Choice, control);
    assert_eq!(
        child_texts(&result, &source, choice, Production::Case),
        [
            "case Ready when { ticket.ready } sequence Start {}",
            "case Waiting when { not ticket.ready } sequence Pause {}",
            "case Retryable when { ticket.count > 0 } sequence Again {}"
        ]
    );
    let visibility = node(
        &result,
        &source,
        Production::Visibility,
        "visible (ticket.ready, ticket.count)",
    );
    assert_eq!(
        child_texts(&result, &source, visibility, Production::Expression),
        ["ticket.ready", "ticket.count"]
    );
    let ready = node(
        &result,
        &source,
        Production::Case,
        "case Ready when { ticket.ready } sequence Start {}",
    );
    assert_eq!(
        child_texts(&result, &source, ready, Production::Block),
        ["{ ticket.ready }"]
    );
    assert_eq!(
        child_texts(&result, &source, ready, Production::Control),
        ["sequence Start {}"]
    );
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn event_sources_relations_and_control_wrappers_keep_their_binders() {
    for event in [
        "send Offered via Inbox as (offer: Envelope) { offer.ok };",
        "receive Got via Inbox of Work::Offered as (offer: Envelope) { offer.ok };",
        "effect Changed of Work::Attempted as (offer: Envelope) { offer.ok };",
        "event Seen by Agent for Work::Offered as (offer: Envelope) related by Links (offer, ticket) { offer.ok };",
        "recommend Plan by Agent on Ledger::Ticket::apply contracts [Allowed, Safe] as (offer: Envelope) { offer.ok };",
    ] {
        let control = format!("observe {event}");
        let source = unit("", &control);
        let result = admitted(&source);
        let outer = node(&result, &source, Production::Control, &control);
        assert_eq!(child_texts(&result, &source, outer, Production::EventNode), [event]);
        let event_node = node(&result, &source, Production::EventNode, event);
        assert_eq!(child_texts(&result, &source, event_node, Production::Parameter), ["offer: Envelope"]);
        assert_eq!(child_texts(&result, &source, event_node, Production::Block), ["{ offer.ok }"]);
        if event.contains("related by") {
            let related = node(&result, &source, Production::Related, "related by Links (offer, ticket)");
            assert_eq!(child_texts(&result, &source, related, Production::Expression), ["offer", "ticket"]);
        }
    }
    for (control, production) in [
        (
            "observe check Verified using v { true };",
            Production::Check,
        ),
        (
            "observe commit Published by Agent as (receipt: Receipt) { receipt.ok };",
            Production::Commit,
        ),
    ] {
        let source = unit("", control);
        let result = admitted(&source);
        let outer = node(&result, &source, Production::Control, control);
        assert_eq!(
            child_texts(&result, &source, outer, production),
            [control.trim_start_matches("observe ")]
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-1", "FSpec-FR-339-AC-2")]
fn attempt_and_fence_orderings_remain_children_of_their_own_controls() {
    for order in ["relaxed", "acquire", "release", "acq_rel", "seq_cst"] {
        let control = format!("attempt Apply by Agent on Ledger::Ticket::apply ordering {order} contracts [] as (receipt: Receipt) {{ receipt.ok }};");
        let source = unit("", &control);
        let result = admitted(&source);
        let event = node(&result, &source, Production::EventNode, &control);
        assert_eq!(
            child_texts(&result, &source, event, Production::MemoryOrder),
            [order]
        );
    }
    for order in ["acquire", "release", "acq_rel", "seq_cst"] {
        let control = format!("observe fence Publish ordering {order};");
        let source = unit("", &control);
        let result = admitted(&source);
        let fence = node(
            &result,
            &source,
            Production::Fence,
            control.trim_start_matches("observe "),
        );
        assert_eq!(
            child_texts(&result, &source, fence, Production::FenceOrder),
            [order]
        );
    }
}

#[test]
#[trace("FSpec-FR-339-AC-4")]
fn malformed_channel_policies_refuse_at_the_bad_policy_token() {
    for (channel, marker, width) in [
        ("channel Inbox from Agent to Peer carries Envelope ordering unordered delivery unknown capacity symbolic(pool overflow block;", "overflow", 8),
        ("channel Inbox from Agent to Peer carries Envelope ordering unordered delivery unknown capacity 18446744073709551616 overflow block;", "18446744073709551616", 20),
        ("channel Inbox from Agent to Peer carries Envelope ordering unordered delivery unknown capacity 2 overflow loss middle;", "middle", 6),
    ] {
        rejected(&unit(channel, "sequence Empty {}"), marker, width);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-4")]
fn malformed_compensation_retry_and_commit_refuse_the_missing_member() {
    for (bad, marker, width) in [
        (
            COMPENSATION.replace(
                "beforeState: Before, afterState",
                "beforeState: Before; afterState",
            ),
            "; afterState",
            1,
        ),
        (
            COMPENSATION.replace("attempts 3 of RetryOffer;", "attempts 3 of ;"),
            ";\n  retry",
            1,
        ),
        (
            COMPENSATION.replace("commit Work::Posted;", "commit ;"),
            ";\n  recover",
            1,
        ),
    ] {
        rejected(&unit(&bad, "sequence Empty {}"), marker, width);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-4")]
fn malformed_awaits_refuse_at_the_interval_or_timeout_boundary() {
    let bad = AWAIT.replace("within [1, 4] match", "within [1, 4 match");
    rejected(&unit("", &bad), "match", 5);
    let bad = AWAIT.replace("timeout sequence", "otherwise sequence");
    rejected(&unit("", &bad), "otherwise", 9);
}

#[test]
#[trace("FSpec-FR-339-AC-4")]
fn malformed_parallel_joins_refuse_at_the_branch_count_or_roster_boundary() {
    for (control, marker, width) in [
        ("parallel Fanout { branch A sequence Idle {} } join all [A];", "} join", 1),
        ("parallel Fanout { branch A sequence Idle {} branch B sequence Idle {} } join all [];", "];", 1),
        ("parallel Fanout { branch A sequence Idle {} branch B sequence Idle {} } join quorum(2 [A, B];", "[A", 1),
    ] {
        rejected(&unit("", control), marker, width);
    }
}

#[test]
#[trace("FSpec-FR-339-AC-4")]
fn malformed_loops_and_fences_refuse_their_own_illegal_keyword() {
    rejected(&unit("", "repeat Drain by Agent visible () max 2 until { true } sequence Again {} exhausted sequence Done {}"), "until", 5);
    rejected(
        &unit(
            "",
            "repeat Drain by Agent visible () max 2 while { true } sequence Again {}",
        ),
        "finish",
        6,
    );
    rejected(&unit("", "fence Publish ordering relaxed;"), "relaxed", 7);
}

const NEAR_COMPENSATION: &str = "compensate UndoUpdate for Work::Applied as (saved: Offer) by Agent on Ledger::Ticket::restore using t clock \"ticks\" { activate first (signal: Signal) when { true } {} within [0, 2]; attempts 2 of RetryOffer; retry (leftState: Before, rightState: After) { true }; commit never; recover (fixed: State) { true }; }";

#[test]
#[trace(
    "FSpec-FR-339-AC-1",
    "FSpec-FR-339-AC-2",
    "FSpec-FR-339-AC-4",
    "FR-002-AC-6"
)]
fn near_miss_compensation_members_refuse_order_omission_unknown_word_and_missing_separator() {
    let good = unit(NEAR_COMPENSATION, "sequence Work {}");
    let result = admitted(&good);
    node(&result, &good, Production::Compensation, NEAR_COMPENSATION);
    for (bad, marker, width, expected) in [
        (
            NEAR_COMPENSATION.replace(
                "retry (leftState: Before, rightState: After) { true }; commit never;",
                "commit never; retry (leftState: Before, rightState: After) { true };",
            ),
            "commit never",
            6,
            "expected `retry`",
        ),
        (
            NEAR_COMPENSATION.replace("attempts 2 of RetryOffer; ", ""),
            "retry (",
            5,
            "expected `attempts`",
        ),
        (
            NEAR_COMPENSATION.replace("attempts 2", "attempt_budget 2"),
            "attempt_budget",
            14,
            "expected `attempts`",
        ),
        (
            NEAR_COMPENSATION.replace(
                "attempts 2 of RetryOffer; retry",
                "attempts 2 of RetryOffer retry",
            ),
            "retry (",
            5,
            "expected `;`",
        ),
    ] {
        rejected_with_expected(&unit(&bad, "sequence Work {}"), marker, width, expected);
    }
}

const NEAR_REPEAT: &str = "repeat Drain by Agent visible () max 2 invariant { true } variant { 2 } while { true } sequence Step {} exhausted sequence Stop {}";

#[test]
#[trace(
    "FSpec-FR-339-AC-1",
    "FSpec-FR-339-AC-2",
    "FSpec-FR-339-AC-4",
    "FR-002-AC-6"
)]
fn near_miss_repetitions_refuse_missing_bounds_arms_proof_order_and_body_delimiters() {
    let good = unit("", NEAR_REPEAT);
    let result = admitted(&good);
    let repetition = node(&result, &good, Production::Repetition, NEAR_REPEAT);
    assert_eq!(
        child_texts(&result, &good, repetition, Production::Control),
        ["sequence Step {}", "sequence Stop {}"]
    );
    for (bad, marker, width, expected) in [
        (
            NEAR_REPEAT.replace("max 2", "max ;"),
            "; invariant",
            1,
            "expected unsigned 64-bit integer",
        ),
        (
            NEAR_REPEAT.replace("exhausted sequence Stop {}", "exhausted ;"),
            ";\nfinish",
            1,
            "expected a declaration",
        ),
        (
            NEAR_REPEAT.replace(
                "invariant { true } variant { 2 }",
                "variant { 2 } invariant { true }",
            ),
            "variant { 2 }",
            7,
            "expected `while`",
        ),
        (
            NEAR_REPEAT.replace("sequence Step {}", "sequence Step"),
            "exhausted sequence",
            9,
            "expected `{`",
        ),
    ] {
        rejected_with_expected(&unit("", &bad), marker, width, expected);
    }
}

const NEAR_AWAIT: &str = "await Receipt after Work::Sent using t clock \"ticks\" within [0, 2] match receive Arrived via Inbox of Work::Sent as (packet: Envelope) { true }; then sequence Continue {} timeout sequence Expire {}";

#[test]
#[trace(
    "FSpec-FR-339-AC-1",
    "FSpec-FR-339-AC-2",
    "FSpec-FR-339-AC-4",
    "FR-002-AC-6"
)]
fn near_miss_awaits_refuse_missing_match_bounds_then_keyword_and_timeout_body() {
    let good = unit("", NEAR_AWAIT);
    let result = admitted(&good);
    let awaiting = node(&result, &good, Production::AwaitControl, NEAR_AWAIT);
    assert_eq!(
        child_texts(&result, &good, awaiting, Production::Control),
        ["sequence Continue {}", "sequence Expire {}"]
    );
    for (bad, marker, width, expected) in [
        (
            NEAR_AWAIT.replace(
                "match receive Arrived via Inbox of Work::Sent as (packet: Envelope) { true };",
                "match ;",
            ),
            "; then",
            1,
            "expected `attempt` or `effect` or `event` or `receive` or `recommend` or `send`",
        ),
        (
            NEAR_AWAIT.replace("within [0, 2]", "within [0, ]"),
            "] match",
            1,
            "expected `*` or `decimal` or `rational` or decimal literal or unsigned integer",
        ),
        (
            NEAR_AWAIT.replace("then sequence Continue", "otherwise sequence Continue"),
            "otherwise",
            9,
            "expected `then`",
        ),
        (
            NEAR_AWAIT.replace("timeout sequence Expire {}", "timeout sequence Expire"),
            "finish Closed",
            6,
            "expected `{`",
        ),
    ] {
        rejected_with_expected(&unit("", &bad), marker, width, expected);
    }
}

#[test]
#[trace(
    "FSpec-FR-339-AC-1",
    "FSpec-FR-339-AC-2",
    "FSpec-FR-339-AC-4",
    "FR-002-AC-6"
)]
fn near_miss_event_nodes_refuse_missing_channels_binders_names_and_invalid_memory_order() {
    for (good, bad, marker, width, expected) in [
        ("send Offer via Inbox as (saved: Envelope) { true };", "send Offer via ; as (saved: Envelope) { true };", "; as", 1, "expected identifier"),
        ("receive Seen via Inbox of Work::Offer as (saved: Envelope) { true };", "receive Seen via Inbox of Work::Offer as () { true };", ") { true };", 1, "expected identifier"),
        ("attempt Apply by Agent on Ledger::Ticket::apply ordering acquire contracts [] as (receipt: Receipt) { true };", "attempt Apply by Agent on Ledger::Ticket::apply ordering consume contracts [] as (receipt: Receipt) { true };", "consume", 7, "expected `acq_rel` or `acquire` or `relaxed` or `release` or `seq_cst`"),
        ("effect Recorded of Work::Applied as (receipt: Receipt) { true };", "effect ; of Work::Applied as (receipt: Receipt) { true };", "; of", 1, "expected identifier"),
        ("event Retired by Agent as (receipt: Receipt) { true };", "event Retired by ; as (receipt: Receipt) { true };", "; as", 1, "expected identifier"),
    ] {
        let source = unit("", good);
        let result = admitted(&source);
        node(&result, &source, Production::EventNode, good);
        rejected_with_expected(&unit("", bad), marker, width, expected);
    }
}

const NEAR_PARALLEL: &str = "parallel Split { branch Alpha sequence First {} branch Beta sequence Second {} } join quorum(1) [Alpha, Beta];";

#[test]
#[trace(
    "FSpec-FR-339-AC-1",
    "FSpec-FR-339-AC-2",
    "FSpec-FR-339-AC-4",
    "FR-002-AC-6"
)]
fn near_miss_joins_refuse_unknown_policy_trailing_name_separator_missing_branch_and_predicate() {
    let source = unit("", NEAR_PARALLEL);
    let result = admitted(&source);
    let parallel = node(&result, &source, Production::Parallel, NEAR_PARALLEL);
    assert_eq!(
        child_texts(&result, &source, parallel, Production::JoinPolicy),
        ["quorum(1)"]
    );
    for (bad, marker, width, expected) in [
        (
            NEAR_PARALLEL.replace("join quorum(1)", "join consensus"),
            "consensus",
            9,
            "expected `all` or `any` or `predicate` or `quorum`",
        ),
        (
            NEAR_PARALLEL.replace("[Alpha, Beta]", "[Alpha, Beta,]"),
            "];",
            1,
            "expected identifier",
        ),
        (
            NEAR_PARALLEL.replace("branch Beta sequence Second {}", "branch Beta"),
            "} join",
            1,
            "expected a declaration",
        ),
        (
            NEAR_PARALLEL.replace("quorum(1)", "predicate()"),
            ") [Alpha",
            1,
            "expected identifier",
        ),
    ] {
        rejected_with_expected(&unit("", &bad), marker, width, expected);
    }
}

#[test]
#[trace(
    "FSpec-FR-339-AC-1",
    "FSpec-FR-339-AC-2",
    "FSpec-FR-339-AC-4",
    "FR-002-AC-6"
)]
fn near_miss_empty_join_rosters_match_the_current_nonempty_standard() {
    // The current shared grammar requires ident before any optional comma tail.
    // These policies complement the already-qualified `all []` refusal.
    for policy in ["any", "quorum(1)", "predicate(ready)"] {
        let good = NEAR_PARALLEL.replace("quorum(1)", policy);
        let source = unit("", &good);
        let result = admitted(&source);
        node(&result, &source, Production::JoinPolicy, policy);
        let bad = good.replace("[Alpha, Beta]", "[]");
        rejected_with_expected(&unit("", &bad), "];", 1, "expected identifier");
    }
}
