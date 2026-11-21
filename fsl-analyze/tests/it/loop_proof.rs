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
    CounterexampleKind, IncompleteCause, LoopProofFailure, ModelCheckItem, ModelCheckLimit,
    ModelCheckLimits, ProtocolCertificateRequest, TerminalValue,
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

/// Loop items follow the untimed SC subject's deadlock item, grouped by
/// sibling repeat in tree order rather than sorted by repeat name.
#[trace("TC-2022", "FR-730-AC-4")]
#[test]
fn sibling_loop_items_follow_tree_order_after_freedom_items() {
    let body = multiple_repeat_unit()
        .replace("First", "Zulu")
        .replace("Second", "Alpha")
        .replace(
            "max 1 while",
            "max 1 invariant { true } variant { 1 } while",
        );
    let checked = crate::fill::package_with(crate::fill::PROTOCOL_PROFILE, &body);
    let protocol = checked.graph().protocol("P").expect("P");
    let repeat = |name| {
        protocol
            .controls
            .controls
            .iter()
            .find(|control| control.name == name)
            .expect("named repeat")
            .node
    };
    let zulu = u64::try_from(repeat("Zulu").0).expect("repeat ordinal");
    let alpha = u64::try_from(repeat("Alpha").0).expect("repeat ordinal");
    let bounded = u64::try_from(repeat("Bounded").0).expect("repeat ordinal");
    let occurrence = crate::model_settle::protocol_anchor(&checked, "P").1;
    let item = ProtocolTemporalItem {
        occurrence: &occurrence,
        subject: &"P",
        protocol,
        models: &[],
        non_atomic_access: false,
    };
    for count in [1, 2] {
        let written = crate::derived_request::protocol_request(&vec![item; count]);
        let actual: Vec<_> = written
            .items()
            .iter()
            .filter(|item| item.derived_limits().is_some())
            .map(|item| {
                (
                    item.item_kind().expect("derived kind"),
                    item.occurrence().origin().ordinal(),
                )
            })
            .collect();
        assert_eq!(
            actual,
            [
                (ItemKind::DeadlockFreedom, 0),
                (ItemKind::LoopInvariant, zulu),
                (ItemKind::LoopTermination, zulu),
                (ItemKind::LoopInvariant, alpha),
                (ItemKind::LoopTermination, alpha),
                (ItemKind::LoopInvariant, bounded),
            ]
        );
    }
}

/// FR-730 "Obligations": a repeat without a loop proof derives neither
/// obligation.
#[trace("TC-2022", "FR-730")]
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
    loop_unit_with_maximum(invariant, variant, guard, operation, None)
}

fn loop_unit_with_maximum(
    invariant: &str,
    variant: &str,
    guard: &str,
    operation: &str,
    maximum: Option<u64>,
) -> String {
    let contracts = match operation {
        "dec" => "DecPre, DecPost",
        _ => "TouchPre",
    };
    let (maximum, exhausted) = match maximum {
        Some(maximum) => (
            format!(" max {maximum}"),
            " exhausted sequence Exhausted { }",
        ),
        None => (String::new(), ""),
    };
    let run = format!(
        "sequence Main {{ repeat Retry by w visible (got.left){maximum} invariant {{ {invariant} }} \
         variant {{ {variant} }} while {{ {guard} }} \
         sequence Body {{ {} event Seen by w as (got: Left) {{ got.left = k.v }}; }}{exhausted} }}",
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
    analyze_case(&case, limits, None)
}

fn analyze_case(
    case: &Case,
    limits: &DerivedLimits,
    expected_failure: Option<(ItemKind, LoopProofFailure)>,
) -> Vec<(ItemKind, TerminalValue, bool)> {
    analyze_case_items(
        case,
        limits,
        expected_failure,
        &[ItemKind::LoopInvariant, ItemKind::LoopTermination],
    )
}

fn analyze_case_items(
    case: &Case,
    limits: &DerivedLimits,
    expected_failure: Option<(ItemKind, LoopProofFailure)>,
    kinds: &[ItemKind],
) -> Vec<(ItemKind, TerminalValue, bool)> {
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
    for &kind in kinds {
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
        let case_ref = case;
        let variant = &protocol
            .slots
            .repeats
            .iter()
            .find(|candidate| candidate.node == repeat)
            .expect("repeat")
            .loop_proof
            .as_ref()
            .expect("loop proof")
            .variant;
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
                let CounterexampleKind::LoopProof(proof) = &payload.kind else {
                    panic!(
                        "a loop-proof counterexample, not a guard failure: {:?}",
                        payload.kind
                    );
                };
                if let Some((expected_kind, failure)) = expected_failure {
                    if kind == expected_kind {
                        assert_eq!(proof.failure, failure);
                    }
                }
                case_ref.replay_at_occurrence("P", payload.clone(), &[], occurrence).inspect_err(|refusal| {
                    eprintln!("{kind:?} replay refused for variant {variant:?}: {refusal}; debug: {refusal:?}");
                })
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
    assert_eq!(
        terminals.len(),
        kinds.len(),
        "each requested loop item settles"
    );
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

fn assert_refuted(items: &[(ItemKind, TerminalValue, bool)], wanted: ItemKind) {
    let (_, value, replayed) = items
        .iter()
        .find(|(kind, _, _)| *kind == wanted)
        .expect("the loop item");
    assert!(
        matches!(value, TerminalValue::Refuted),
        "{wanted:?}: {value:?}"
    );
    assert!(
        *replayed,
        "{wanted:?}: the loop-proof counterexample replays"
    );
}

/// A maximum discharges termination independently of a constant variant;
/// the authored invariant still gets checked and replayed when false.
#[trace("TC-2022", "FR-730-AC-5")]
#[test]
fn a_bounded_repeat_with_a_proof_derives_only_its_invariant() {
    for invariant in ["true", "false"] {
        let body = loop_unit_with_maximum(invariant, "0", "got.left > 0", "dec", Some(3));
        assert_eq!(loop_items(&body), ["loop-invariant"]);
        let case = Case::new(&body, &[2]);
        let items = analyze_case_items(
            &case,
            &DerivedLimits::default(),
            Some((ItemKind::LoopInvariant, LoopProofFailure::InvariantFalse)),
            &[ItemKind::LoopInvariant],
        );
        if invariant == "false" {
            assert_refuted(&items, ItemKind::LoopInvariant);
        } else {
            let [(ItemKind::LoopInvariant, value, replayed)] = items.as_slice() else {
                panic!("only the invariant item: {items:?}");
            };
            assert!(matches!(value, TerminalValue::Proved { .. }), "{value:?}");
            assert!(!replayed);
        }
    }
}

/// Tuple order uses the first unequal component, and consults later
/// components when the earlier ones are equal.
#[trace("TC-2022", "FR-730", "FR-218-AC-43")]
#[test]
fn tuple_variants_use_lexicographic_order_in_checker_and_replay() {
    for (variant, proved) in [
        ("Pair(got.left, 3 - got.left)", true),
        ("Pair(0, got.left)", true),
        ("Pair(3 - got.left, got.left)", false),
        ("Pair(0, 3 - got.left)", false),
    ] {
        let body = format!(
            "tuple Pair(Integer, Integer);\n{}",
            loop_unit("true", variant, "got.left > 0", "dec")
        );
        let case = Case::new(&body, &[2]);
        let items = analyze_case(
            &case,
            &DerivedLimits::default(),
            Some((
                ItemKind::LoopTermination,
                LoopProofFailure::VariantDidNotDecrease,
            )),
        );
        if proved {
            for (kind, value, replayed) in items {
                assert!(
                    matches!(value, TerminalValue::Proved { .. }),
                    "{variant}, {kind:?}: {value:?}"
                );
                assert!(!replayed);
            }
        } else {
            assert_refuted(&items, ItemKind::LoopTermination);
        }
    }
}

/// A strictly decreasing negative first value still violates the scalar or
/// tuple-element floor on a completed back edge.
#[trace("TC-2022", "FR-730")]
#[test]
fn negative_first_decision_variants_refute_the_floor() {
    for variant in ["got.left - 3", "Pair(got.left, got.left - 3)"] {
        let body = format!(
            "tuple Pair(Integer, Integer);\n{}",
            loop_unit("true", variant, "got.left > 0", "dec")
        );
        let case = Case::new(&body, &[2]);
        let items = analyze_case(
            &case,
            &DerivedLimits::default(),
            Some((
                ItemKind::LoopTermination,
                LoopProofFailure::VariantBelowFloor,
            )),
        );
        assert_refuted(&items, ItemKind::LoopTermination);
    }
}

/// Undefined proof expressions refute their own obligations at a decision,
/// independently of a true or false while guard.
#[trace("TC-2022", "FR-730")]
#[test]
fn undefined_invariant_and_variant_refute_with_loop_proof_replay() {
    for (invariant, variant, wanted) in [
        ("q(0) = 0", "got.left", ItemKind::LoopInvariant),
        ("true", "q(0)", ItemKind::LoopTermination),
    ] {
        let body = loop_unit(invariant, variant, "got.left > 0", "dec");
        let case = Case::with_kernel_remainder(&body, &[2]);
        let items = analyze_case(
            &case,
            &DerivedLimits::default(),
            Some((wanted, LoopProofFailure::Undefined)),
        );
        assert_refuted(&items, wanted);
    }
}

/// Entry and post-body decisions remain reachable proof failures when the
/// same decision's guard is undefined. The post-body invariant becomes
/// false only on the second completed body, when the guard becomes undefined.
#[trace("TC-2022", "FR-730")]
#[test]
fn undecided_guards_preserve_initial_and_post_body_loop_proof_failures() {
    for (entry_invariant, post_invariant, variant, wanted, failure) in [
        (
            "false",
            "got.left > 0",
            "1",
            ItemKind::LoopInvariant,
            LoopProofFailure::InvariantFalse,
        ),
        (
            "q(0) = 0",
            "if got.left > 0 then true else q(0) = 0",
            "1",
            ItemKind::LoopInvariant,
            LoopProofFailure::Undefined,
        ),
        (
            "true",
            "true",
            "q(0)",
            ItemKind::LoopTermination,
            LoopProofFailure::Undefined,
        ),
    ] {
        let entry_run = format!(
            "sequence Main {{ repeat Retry by w visible () invariant {{ {entry_invariant} }} variant {{ {variant} }} while {{ q(0) = 0 }} sequence Body {{ {} }} }}",
            crate::fill::attempt("T", "touch", "TouchPre", "t", "true")
        );
        let entry = loop_pieces(&entry_run);
        let post_variant = if wanted == ItemKind::LoopTermination {
            "if got.left > 0 then got.left else q(0)"
        } else {
            variant
        };
        let post_body = loop_unit(
            post_invariant,
            post_variant,
            "if got.left > 0 then true else q(0) = 0",
            "dec",
        );
        for body in [entry, post_body] {
            let case = Case::with_kernel_remainder(&body, &[2]);
            let items = analyze_case_items(
                &case,
                &DerivedLimits::default(),
                Some((wanted, failure)),
                &[wanted],
            );
            assert_refuted(&items, wanted);
        }
    }
}

/// FR-730's source-backed request document resolves both loop kinds to
/// the repeat and evaluates them through the source checker.
#[trace("TC-2022", "FR-730-AC-5", "FR-730-AC-6", "FR-620")]
#[test]
fn source_loop_proof_documents_settle_both_item_kinds() {
    use crate::analyze_cli_support::{analyze, label, request_over, Run, Workspace};
    use serde_json::json;

    let item_with_record = |run: &Run, index: usize, phase: &str| {
        let item = run.item(index);
        assert!(
            item["record"]["result"].is_string(),
            "{phase} item {index}: {}",
            run.document()
        );
        item
    };

    let ws = Workspace::new("source-loop-proof-wire");
    let body = loop_unit("true", "got.left", "got.left > 0", "dec");
    let unit = crate::fill::unit_text_with(crate::fill::PROTOCOL_PROFILE, &body);
    let mut documents = crate::fill::Documents::new();
    documents.start("s0", &[("c", 2)]);
    let snapshots: Vec<Vec<u8>> = documents.snapshots.values().cloned().collect();
    let path = request_over(
        &ws,
        &unit,
        &crate::fill::document(),
        &snapshots,
        json!({"subject": {"protocol": {"protocol": "P",
        "over": {"reference": {"population": "ix://test/fill/cells", "key": "c"}},
        "roles": {"w": {"type": "model-binding", "selection": "test/fill"}}}},
        "items": [
                {"claim": "P::Main::Retry", "temporal": {"item_kind": "loop-invariant"}},
                {"claim": "P::Main::Retry", "temporal": {"item_kind": "loop-termination"}}
        ]}),
    );
    let run = analyze(&path);
    assert_eq!(run.exit, 0, "{}", run.document());
    assert_eq!(run.document()["items"].as_array().expect("items").len(), 2);
    for (index, kind) in ["loop-invariant", "loop-termination"]
        .into_iter()
        .enumerate()
    {
        let item = item_with_record(&run, index, "unbounded");
        assert_eq!(label(&item), "proved", "{item}");
        assert_eq!(item["request"]["temporal"]["item_kind"], kind, "{item}");
        assert!(
            item["request"]["temporal"].get("max_depth").is_none(),
            "{item}"
        );
    }

    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).expect("source request"))
            .expect("request JSON");
    let mut limited = original.clone();
    for item in limited["request"]["items"].as_array_mut().expect("items") {
        item["temporal"]["limits"] = json!({"max_states": 1});
    }
    let stopped = analyze(&ws.write("limited.json", limited.to_string().as_bytes()));
    assert_eq!(stopped.exit, 22, "{}", stopped.document());
    for index in 0..2 {
        let item = item_with_record(&stopped, index, "state-limited");
        assert_eq!(label(&item), "incomplete", "{item}");
        assert_eq!(item["record"]["cause"]["limit"], "max_states", "{item}");
        assert_eq!(item["record"]["cause"]["value"], 1, "{item}");
        assert!(
            item["request"]["temporal"].get("max_depth").is_none(),
            "{item}"
        );
    }

    let mut horizon = original.clone();
    for item in horizon["request"]["items"].as_array_mut().expect("items") {
        item["temporal"]["max_depth"] = json!(1);
    }
    let refused = analyze(&ws.write("horizon.json", horizon.to_string().as_bytes()));
    assert_eq!(refused.exit, 20, "{}", refused.document());
    for index in 0..2 {
        let item = item_with_record(&refused, index, "item-horizon-refused");
        assert_eq!(
            item["record"]["diagnostic"]["code"], "invalid_runtime_input",
            "{item}"
        );
        assert_eq!(
            item["record"]["diagnostic"]["detail"]["member"], "temporal.max_depth",
            "{item}"
        );
    }

    let bounded_body = loop_unit_with_maximum("true", "got.left", "got.left > 0", "dec", Some(3));
    let bounded = crate::fill::unit_text_with(crate::fill::PROTOCOL_PROFILE, &bounded_body);
    ws.write("program.fsl", bounded.as_bytes());
    let bounded = analyze(&path);
    assert_eq!(bounded.exit, 20, "{}", bounded.document());
    let invariant = item_with_record(&bounded, 0, "bounded");
    let termination = bounded.item(1);
    assert_eq!(label(&invariant), "proved", "{}", bounded.document());
    assert_eq!(termination["request_index"], 1, "{}", bounded.document());
    assert_eq!(termination["category"], "refusal", "{}", bounded.document());
    assert_eq!(
        termination["record"]["settled_by"],
        "disposition",
        "{}",
        bounded.document()
    );
    assert_eq!(
        termination["record"]["diagnostic"]["code"],
        "invalid_capability",
        "{}",
        bounded.document()
    );
    assert_eq!(
        termination["record"]["diagnostic"]["cause"],
        "inapplicable-kind",
        "{}",
        bounded.document()
    );
}
