// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-730 (TC-2022): a repeat's loop proof is discharged over the explored
//! protocol subject as two derived items of the repeat, `loop-invariant` and
//! `loop-termination`, each settling `proved`, `refuted` or `incomplete`.

use crate::derived_request::{items_of_kind, protocol_request_under};
use crate::protocol_system::Case;
use filament_exact::Cancel;
use fsl_analyze::analyze::{
    analyze_items, derived_protocol_item, split_records, Engines, ProtocolItem,
};
use fsl_analyze::model_check::ProtocolCheckRequest;
use fsl_replay::{
    IncompleteCause, ModelCheckItem, ModelCheckLimit, ModelCheckLimits, ProtocolCertificateRequest,
    TerminalValue,
};
use fsl_route::deadlock::{ObligationKind, ProtocolTemporalItem};
use fsl_route::request::DerivedLimits;
use fsl_route::request::ItemKind;
use ix_trace_rs::trace;

/// FR-730-AC-4: the request writer has the two derived item kinds a loop
/// proof contributes, beside deadlock freedom, so a repeat's invariant and
/// termination obligations are routed and settled like every other derived
/// item instead of being recorded and left undischarged.
#[trace("TC-2022", "FR-730-AC-4", "FSpec-TC-1520", "FSpec-FR-171-AC-6")]
#[test]
fn a_loop_proof_contributes_invariant_and_termination_item_kinds() {
    let kinds: Vec<&str> = ItemKind::ALL.iter().map(|kind| kind.to_wire()).collect();
    for expected in ["loop-invariant", "loop-termination"] {
        assert!(
            kinds.contains(&expected),
            "{expected} is a derived item kind: {kinds:?}"
        );
    }
}

/// FR-730-AC-4: two proof-carrying repeats contribute two items each; a
/// bounded repeat contributes none, and duplicate requests over one subject
/// do not duplicate the four derived items.
#[trace("TC-2022", "FR-730-AC-4")]
#[test]
fn two_loop_proofs_derive_four_items_once_and_bounded_repeat_derives_none() {
    let body = multiple_repeat_unit();
    let four = [
        "loop-invariant",
        "loop-termination",
        "loop-invariant",
        "loop-termination",
    ];
    assert_eq!(loop_items_for_request_count(&body, 1), four);
    assert_eq!(loop_items_for_request_count(&body, 2), four);
}

/// FR-730 "Obligations": a repeat without a loop proof derives neither
/// obligation.
#[trace("TC-2022", "FR-730-AC-4")]
#[test]
fn a_repeat_without_a_variant_derives_no_loop_proof_items() {
    let run = format!(
        "sequence Main {{ repeat Retry by w visible () while {{ false }} sequence Body {{ {} }} }}",
        crate::fill::attempt("T", "touch", "TouchPre", "t", "true")
    );
    let body = crate::fill::contracts("self.v = 0") + &crate::fill::protocol("P", "", &run, "d");
    assert!(loop_items(&body).is_empty());
}

/// A unit whose protocol `P` holds `repeat Retry` with no maximum, the loop
/// proof `invariant { invariant } variant { variant }` and guard `guard`.
/// Its body steps the cell `k` with `operation` and then observes the cell's
/// counter as `got.left`, so the decision slots read a body binder, as
/// FR-730's use case does.
fn loop_unit(invariant: &str, variant: &str, guard: &str, operation: &str) -> String {
    let contracts = match operation {
        "dec" => "DecPre, DecPost",
        _ => "TouchPre",
    };
    let run = format!(
        "sequence Main {{ repeat Retry by w visible (got.left) invariant {{ {invariant} }} \
         variant {{ {variant} }} while {{ {guard} }} \
         sequence Body {{ {} event Seen by w as (got: Left) {{ got.left = k.v }}; }} }}",
        crate::fill::attempt("T", operation, contracts, "t", "true")
    );
    format!("record Left {{ left: Int[0, 3]; }}\n{}", loop_pieces(&run))
}

/// The contracts and the protocol `P` over `run`.
fn loop_pieces(run: &str) -> String {
    crate::fill::contracts("self.v = 0")
        + "pre DecPre using v on M::Cell::dec { self.v >= 1 }\n\
           post DecPost using v on M::Cell::dec { pre(self.v) - self.v = 1 }\n"
        + &crate::fill::protocol("P", "", run, "d")
}

/// The derived items `loop-invariant` and `loop-termination` of `P` in
/// `body`, through the request writer that derives every protocol item.
fn loop_items(body: &str) -> Vec<&'static str> {
    loop_items_for_request_count(body, 1)
}

/// The loop-proof item kinds written for `request_count` caller requests of
/// the same protocol subject.
fn loop_items_for_request_count(body: &str, request_count: usize) -> Vec<&'static str> {
    use crate::derived_request::protocol_request;
    use crate::model_settle::protocol_anchor;
    use fsl_route::deadlock::ProtocolTemporalItem;

    let checked = crate::fill::package_with(crate::fill::PROTOCOL_PROFILE, body);
    let protocol = checked.graph().protocol("P").expect("P");
    let occurrence = protocol_anchor(&checked, "P").1;
    let item = ProtocolTemporalItem {
        occurrence: &occurrence,
        subject: &"P",
        protocol,
        models: &[],
        non_atomic_access: false,
    };
    let items = vec![item; request_count];
    let written = protocol_request(&items);
    written
        .items()
        .iter()
        .filter_map(|item| item.item_kind())
        .map(ItemKind::to_wire)
        .filter(|kind| kind.starts_with("loop-"))
        .collect()
}

/// Two unbounded proof-carrying repeats and one bounded repeat share a
/// protocol subject.
fn multiple_repeat_unit() -> String {
    let attempt = |node: &str, binder: &str| {
        crate::fill::attempt(node, "dec", "DecPre, DecPost", binder, "true")
    };
    let run = format!(
        "sequence Main {{ \
         repeat First by w visible () invariant {{ true }} variant {{ 1 }} while {{ false }} \
           sequence FirstBody {{ {} }} \
         repeat Second by w visible () invariant {{ true }} variant {{ 1 }} while {{ false }} \
           sequence SecondBody {{ {} }} \
         repeat Bounded by w visible () max 1 while {{ false }} \
           sequence BoundedBody {{ {} }} \
           exhausted check Out using v {{ true }}; }}",
        attempt("FirstStep", "one"),
        attempt("SecondStep", "two"),
        attempt("BoundedStep", "three"),
    );
    loop_pieces(&run)
}

/// The EN-1 protocol obligations over `body`, through the public request
/// writer, derived-item adapter, checker, certificate and replay paths.
fn analyze_loop(
    body: &str,
    initial: i64,
    limits: &DerivedLimits,
) -> Vec<(ItemKind, TerminalValue, bool)> {
    let case = Case::new(body, &[initial]);
    let protocol = case.checked.graph().protocol("P").expect("P");
    let occurrence = crate::model_settle::protocol_anchor(&case.checked, "P").1;
    let subject = case.subject("P");
    let models = subject
        .memory
        .parallels()
        .iter()
        .map(|resolved| (resolved.parallel.clone(), resolved.model))
        .collect::<Vec<_>>();
    let written = protocol_request_under(
        &[ProtocolTemporalItem {
            occurrence: &occurrence,
            subject: &subject.protocol,
            protocol,
            models: &models,
            non_atomic_access: false,
        }],
        limits,
    );
    let mut items = Vec::new();
    let mut indexes = Vec::new();
    for kind in [ItemKind::LoopInvariant, ItemKind::LoopTermination] {
        let [(derived, obligation)] = &items_of_kind(&written, kind)[..] else {
            panic!("one {kind:?} item: {:?}", written.items());
        };
        let repeat = match &obligation.kind {
            ObligationKind::ProtocolLoopInvariant { repeat, .. }
            | ObligationKind::ProtocolLoopTermination { repeat, .. } => *repeat,
            other => panic!("wrong obligation: {other:?}"),
        };
        let check_item = match kind {
            ItemKind::LoopInvariant => ModelCheckItem::LoopInvariant { repeat },
            ItemKind::LoopTermination => ModelCheckItem::LoopTermination { repeat },
            _ => unreachable!(),
        };
        let occurrence = derived.occurrence().clone();
        let case_ref = &case;
        let item = ProtocolItem {
            request: ProtocolCheckRequest {
                temporal_clock_selections: &[],
                subject: case.subject("P"),
                item: check_item,
                max_depth: None,
                max_store_buffer: None,
                max_messages: None,
                max_live_instances: None,
                limits: ModelCheckLimits::default(),
                ignore_memory_constraints: false,
            },
            certificate: ProtocolCertificateRequest {
                temporal_clock_selections: &[],
                subject: case.subject("P"),
                item: check_item,
                max_store_buffer: None,
                max_messages: None,
                max_live_instances: None,
            },
            replay: Box::new(move |payload| {
                case_ref.replay_at_occurrence("P", payload.clone(), &[], occurrence)
            }),
        };
        indexes.push((kind, derived.index()));
        items.push(
            derived_protocol_item(derived, obligation, item).expect("the derived loop proof"),
        );
    }
    let records = analyze_items(
        &Engines::fsl().expect("EN-1 registers"),
        items,
        &Cancel::new(),
    );
    let (terminals, counterexamples) = split_records(records, &case.checked);
    terminals
        .iter()
        .map(|terminal| {
            let item_kind = indexes
                .iter()
                .find_map(|(kind, index)| (*index == terminal.request_index()).then_some(*kind))
                .unwrap_or_else(|| {
                    panic!("unexpected item index {}", terminal.request_index().get())
                });
            (
                item_kind,
                terminal.value().clone(),
                counterexamples
                    .iter()
                    .any(|found| found.request_index == terminal.request_index()),
            )
        })
        .collect()
}

/// FR-730-AC-1: a repeat with no maximum, the loop proof
/// `invariant { got.left >= 0 } variant { got.left }` and a body that
/// decreases `got.left` by one from `2` settles both loop items `proved`
/// over a complete exploration.
#[trace("TC-2022", "FR-730-AC-1")]
#[test]
fn a_decreasing_variant_proves_both_loop_items() {
    let body = loop_unit("got.left >= 0", "got.left", "got.left > 0", "dec");
    assert_eq!(loop_items(&body), ["loop-invariant", "loop-termination"]);
    for (kind, value, replayed) in analyze_loop(&body, 2, &DerivedLimits::default()) {
        assert!(
            matches!(value, TerminalValue::Proved { .. }),
            "{kind:?}: {value:?}"
        );
        assert!(!replayed, "{kind:?}: unexpected counterexample");
    }
}

/// FR-730-AC-2: a body that leaves `got.left` unchanged refutes
/// `loop-termination` with a counterexample that replays and still proves
/// `loop-invariant`; the invariant `got.left > 0` over the decreasing body
/// refutes `loop-invariant` at the decision where `got.left` is `0`, after
/// two body steps, with a counterexample that replays.
#[trace("TC-2022", "FR-730-AC-2")]
#[test]
fn an_unchanged_variant_and_a_false_invariant_refute_with_replayed_counterexamples() {
    for (invariant, operation, refuted, other) in [
        (
            "got.left >= 0",
            "touch",
            ItemKind::LoopTermination,
            Some(ItemKind::LoopInvariant),
        ),
        ("got.left > 0", "dec", ItemKind::LoopInvariant, None),
    ] {
        let body = loop_unit(invariant, "got.left", "got.left > 0", operation);
        assert_eq!(loop_items(&body), ["loop-invariant", "loop-termination"]);
        let items = analyze_loop(&body, 2, &DerivedLimits::default());
        let (kind, value, replayed) = items
            .iter()
            .find(|(kind, _, _)| *kind == refuted)
            .expect("requested item");
        assert!(
            matches!(value, TerminalValue::Refuted),
            "{kind:?}: {value:?}"
        );
        assert!(*replayed, "{kind:?}: counterexample replayed");
        if let Some(other) = other {
            let (_, value, _) = items
                .iter()
                .find(|(kind, _, _)| *kind == other)
                .expect("the other item");
            assert!(
                matches!(value, TerminalValue::Proved { .. }),
                "{other:?}: {value:?}"
            );
        }
    }
}

/// FR-730-AC-3: the AC-1 protocol checked with a state limit below its
/// reachable state count settles both loop items `incomplete`, naming the
/// state limit and claiming neither result.
#[trace("TC-2022", "FR-730-AC-3")]
#[test]
fn a_state_limit_leaves_both_loop_items_incomplete_naming_it() {
    let body = loop_unit("got.left >= 0", "got.left", "got.left > 0", "dec");
    assert_eq!(loop_items(&body), ["loop-invariant", "loop-termination"]);
    let limits = DerivedLimits::from_settings([("max_states", 1)]).expect("a member");
    for (kind, value, replayed) in analyze_loop(&body, 2, &limits) {
        assert!(
            matches!(
                value,
                TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::MaxStates,
                    value: 1,
                })
            ),
            "{kind:?}: {value:?}"
        );
        assert!(!replayed, "{kind:?}: unexpected counterexample");
    }
}
