//! FR-218-AC-17: protocol captures are immutable decision inputs, while a
//! direct read of the model input remains ineligible at the read.

use ix_trace_rs::trace;

use super::{checks, protocol_causes, refusals};
use crate::check::protocol_obligation::{ObligationCause, ReadClass};
use crate::check::refusal::{ProtocolAnchorCause, SlotKind};
use crate::check::{CheckCause, CheckRefusal};

const RUN: &str = "sequence Main { attempt Tried by R on M::Actor::op contracts [] \
    as (tried: Boolean) { true }; \
    choice Pick by R visible () { \
      case yes when { ready } sequence Yes { } \
      case no when { not ready } sequence No { } } }";

fn protocol(capture: &str, run: &str) -> String {
    format!(
        "protocol Flow using pp over (input: M::Actor) on origin {{\n\
         {capture}\n\
         role R on M::Actor;\n\
         run {run}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}"
    )
}

/// A protocol capture is available to a choice guard without appearing in
/// the role's `visible` list.
#[trace("FR-218-AC-17")]
#[test]
fn a_guard_over_a_visible_capture_checks() {
    let graph = checks(&protocol("capture ready: Boolean = input.ready;", RUN));
    assert!(graph.protocol("Flow").is_some());
}

/// The model input itself is not a decision fact: both guard reads refuse at
/// their exact source locations.
#[trace("FR-218-AC-17")]
#[test]
fn a_guard_reading_over_refuses_at_the_read() {
    let run = RUN.replace("ready", "input.ready");
    let (text, found) = refusals(&protocol("", &run));
    let causes = protocol_causes(&found);
    assert_eq!(found.len(), 2, "{found:?}");
    for (refusal, cause) in found.iter().zip(causes) {
        assert_eq!(
            (refusal.cause.code().as_str(), refusal.cause.cause()),
            ("ill_typed", Some("operator-ineligible")),
            "{refusal:?}"
        );
        assert!(matches!(cause,
            ProtocolAnchorCause::SlotModelState { node, slot: SlotKind::Guard, .. }
                if node == "Pick"));
        let span = cause.span();
        let at = &text[span.start..span.end];
        assert_eq!(at, "input.ready", "{cause:?}");
    }
}

/// A choice owner cannot read the other role's mutable receive fact, even
/// when the protocol also declares a capture. The refusal belongs to the
/// foreign read, not to the capture declaration.
#[trace("FR-218-AC-17")]
#[test]
fn a_guard_cannot_read_foreign_mutable_state_beside_a_capture() {
    let source = "record Plain { ready: Boolean; done: Boolean; }\n\
        protocol Flow using pp over (input: M::Actor) on origin {\n\
        capture saved: Boolean = input.ready;\n\
        role R on M::Actor; role S on M::Actor;\n\
        channel ch from R to S carries Plain ordering unordered delivery unknown capacity 2 overflow block;\n\
        run sequence Main { send Tx via ch as (tx: Plain) { true };\n\
        receive Rx via ch of Main::Tx as (rx: Plain) { true };\n\
        choice Pick by R visible () { case yes when { rx.ready } sequence Yes { }\n\
        case no when { not rx.ready } sequence No { } } }\n\
        finish End as (outcome: Boolean) { true };\
        }";
    let (text, found) = refusals(source);
    assert_eq!(found.len(), 2, "the capture itself is admitted: {found:?}");
    for (refusal, cause) in found.iter().zip(protocol_causes(&found)) {
        assert_eq!(
            (refusal.cause.code().as_str(), refusal.cause.cause()),
            ("wrong_snapshot", Some("wrong-observation")),
            "{refusal:?}"
        );
        assert!(matches!(cause,
            ProtocolAnchorCause::WrongObservation { owner, binder, field: Some(field), reason, .. }
                if owner == "R" && binder == "rx" && field == "ready"
                    && *reason == crate::check::refusal::ObservationReason::OtherRole));
        let span = cause.span();
        assert_eq!(&text[span.start..span.end], "rx.ready", "{cause:?}");
    }
}

/// A capture read from a protocol-capture initializer before its write is
/// classified by FR-381 at the name, rather than treated as an observation
/// failure in a later guard.
#[trace("FR-381-AC-2")]
#[test]
fn a_protocol_capture_read_before_write_refuses_at_the_name() {
    let source = protocol(
        "capture earlier: Boolean = later;\n\
         capture later: Boolean = input.ready;\n\
         capture ready: Boolean = true;",
        RUN,
    );
    let (_, found) = refusals(&source);
    assert!(
        found.iter().any(|refusal: &CheckRefusal| {
            matches!(&refusal.cause, CheckCause::ProtocolObligation(cause)
                if matches!(cause.as_ref(), ObligationCause::StageRead {
                    read: ReadClass::Forward, name, ..
                } if name == "later"))
        }),
        "expected the forward-read classification: {found:?}"
    );
}

/// Reading the capture from itself has the same `forward` classification
/// and identifies the self-read span.
#[trace("FR-381-AC-2")]
#[test]
fn a_protocol_capture_read_from_itself_refuses_as_forward() {
    let source = protocol("capture ready: Boolean = ready;", RUN);
    let (text, found) = refusals(&source);
    let refusal = found
        .iter()
        .find(|refusal| {
            matches!(&refusal.cause, CheckCause::ProtocolObligation(cause)
            if matches!(cause.as_ref(), ObligationCause::StageRead {
                read: ReadClass::Forward, name, ..
            } if name == "ready"))
        })
        .expect("self-read is classified as forward");
    let span = match &refusal.cause {
        CheckCause::ProtocolObligation(cause) => cause.span(),
        other => panic!("protocol obligation refusal, got {other:?}"),
    };
    assert_eq!(&text[span.start..span.end], "ready");
}
