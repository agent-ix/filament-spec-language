// SPDX-License-Identifier: AGPL-3.0-or-later
//! A driver outside the crate builds and writes the `filament-outcome/1`
//! document through `fsl_replay`'s root alone, with no serializer of its own.

use filament_semantic_value::catalog::Category;
use fsl_foundation::BackendId;
use fsl_replay::{
    ItemLabel, Operation, OutcomeDocument, OutcomeItem, OutcomeStage, RequestIndex, TerminalRecord,
};
use ix_trace_rs::trace;

/// FR-286-AC-3: a `prove` document built and written through the facade
/// carries its items in request order under the `filament-outcome/1` format.
#[trace("TC-770", "FR-286-AC-3", "FR-270-AC-5")]
#[test]
fn a_driver_writes_a_prove_document_through_the_facade() {
    let backend = BackendId::new("outcome-backend");
    let items = vec![
        OutcomeItem::from_terminal(&TerminalRecord::proved(
            RequestIndex::new(0),
            backend.clone(),
            2,
        )),
        OutcomeItem::from_terminal(&TerminalRecord::failed(RequestIndex::new(1), backend)),
    ];
    let document = OutcomeDocument::settled(Operation::Prove, Some(OutcomeStage::S8), items);
    let written: serde_json::Value =
        serde_json::from_slice(&document.to_bytes().expect("encodes")).expect("is JSON");
    assert_eq!(written["format"], "filament-outcome/1");
    assert_eq!(written["operation"], "prove");
    assert_eq!(written["last_stage"], "S8");
    assert_eq!(written["category"], "internal-failure");
    assert_eq!(written["items"][0]["result"], "proved");
    assert_eq!(written["items"][0]["certification"], "certified");
    assert_eq!(
        written["items"][0]["basis"],
        serde_json::json!({"type": "bounded-proof", "checks": 2})
    );
    assert_eq!(written["items"][1]["result"], "failed");
    assert_eq!(document.category(), Category::InternalFailure);
    assert_eq!(document.items()[1].category(), Category::InternalFailure);
    assert_ne!(ItemLabel::Proved, ItemLabel::Failed);
}

// ---- QSL-2063: determinism of the outcome document and order-free folds -----

/// QSL-2063. A call request is a function with no parameters whose body folds a
/// collection literal; the literal's element order is the presentation order.
///
/// What is covered: completed outcomes (sum, count, exists, forall, and `fold` with a catalog step) and two kinds of refusal at check (an unproved `sum` range, FR-680-AC-6; a
/// non-catalog fold step over a Set or Bag, FR-701-AC-2). What is not covered: a violation
/// outcome (a run that completes with a refuted claim), which no generated shape here reaches;
/// `min` and `max`, which the language does not have; and `reduce`, which the checker refuses
/// over a collection of unproved size (`undefined_expression`/`unproved-range`), so no literal of
/// this generator reaches it.
mod determinism {
    use std::collections::BTreeMap;

    use filament_exact::{Cancel, Integer};
    use fsl_bench::prop;
    use fsl_foundation::SourceIdentity;
    use fsl_replay::spine::{
        self, default_accounting, Call, CallOutcome, CallValue, DependencyInput, SpineLimits,
    };
    use fsl_replay::OutcomeDocument;
    use proptest::prelude::*;

    const HEADER: &str =
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Kind {
        Sequence,
        Set,
        Bag,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Fold {
        /// `sum<Int[0, 100]>(x in C: x)`.
        Sum,
        /// `sum<Int[0, 5]>`: the range is not proved, so the checker refuses.
        SumNarrow,
        /// `fold<I>(acc, x in C: acc + x, identity: 0)`, a catalog step.
        FoldAdd,
        /// `fold<I>(acc, x in C: x * acc, identity: 1)`, a catalog step.
        FoldMul,
        /// `fold<I>(acc, x in C: acc - x, identity: 0)`: not a catalog step, so a Set or Bag
        /// source is refused (FR-701-AC-2) and a Sequence source completes.
        FoldSubtract,
        /// `fold<I>(acc, x in C: acc * 10 + x, identity: 0)`: reads the order of its source,
        /// so only a Sequence admits it. Never drawn by `requests`.
        Digits,
        /// `count<Int[0, 4]>(x in C: x < k)`.
        Count(u8),
        Exists(u8),
        Forall(u8),
    }

    #[derive(Clone, Debug)]
    pub struct Request {
        pub kind: Kind,
        pub fold: Fold,
        pub elements: Vec<u8>,
        pub permuted: Vec<u8>,
    }

    fn literal(kind: Kind, elements: &[u8]) -> String {
        let name = match kind {
            Kind::Sequence => "sequence",
            Kind::Set => "set",
            Kind::Bag => "bag",
        };
        let list: Vec<String> = elements.iter().map(u8::to_string).collect();
        format!("{name}[{}]", list.join(", "))
    }

    pub fn source(kind: Kind, fold: Fold, elements: &[u8]) -> String {
        let c = literal(kind, elements);
        let collection = match kind {
            Kind::Sequence => "Sequence",
            Kind::Set => "Set",
            Kind::Bag => "Bag",
        };
        let element = "Int[0, 9]";
        let bind = |body: &str| format!("let s: {collection}<{element}>[0, 4] = ({c}) in ({body})");
        let (result, body) = match fold {
            Fold::Sum => ("Total", bind("sum<Total>(x in s: x)")),
            Fold::SumNarrow => ("Narrow", bind("sum<Narrow>(x in s: x)")),
            Fold::Count(k) => ("Small", bind(&format!("count<Small>(x in s: x < {k})"))),
            Fold::Exists(k) => ("Boolean", bind(&format!("exists(x in s: x = {k})"))),
            Fold::Forall(k) => ("Boolean", bind(&format!("forall(x in s: x < {k})"))),
            Fold::FoldAdd => (
                "I",
                bind("fold<I>(acc, x in s: acc + convert<I>(x), identity: 0)"),
            ),
            Fold::FoldMul => (
                "I",
                bind("fold<I>(acc, x in s: convert<I>(x) * acc, identity: 1)"),
            ),
            Fold::FoldSubtract => (
                "I",
                bind("fold<I>(acc, x in s: acc - convert<I>(x), identity: 0)"),
            ),
            Fold::Digits => (
                "I",
                bind("fold<I>(acc, x in s: acc * 10 + convert<I>(x), identity: 0)"),
            ),
        };
        format!("{HEADER}type Total = Int[0, 100];\ntype Small = Int[0, 4];\ntype Narrow = Int[0, 5];\ntype I = Integer;\nfunction f using v(): {result} pure {{ {body} }}\n")
    }

    /// The outcome document bytes of running `f` in `text`.
    pub fn document(text: &str) -> Vec<u8> {
        let call = Call::new(
            "f".to_owned(),
            Vec::new(),
            default_accounting(spine::DEFAULT_WORK_UNITS),
            Default::default(),
        );
        let result = spine::run(
            SourceIdentity::new("agent-ix", "test:determinism"),
            "fixture".to_owned(),
            "program.native",
            text.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
            &call,
            &Default::default(),
            &Cancel::new(),
        );
        OutcomeDocument::from_run(&result)
            .to_bytes()
            .expect("encodes")
    }

    fn run_value(text: &str) -> Option<CallValue> {
        let call = Call::new(
            "f".to_owned(),
            Vec::new(),
            default_accounting(spine::DEFAULT_WORK_UNITS),
            Default::default(),
        );
        match spine::run(
            SourceIdentity::new("agent-ix", "test:determinism"),
            "fixture".to_owned(),
            "program.native",
            text.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
            &call,
            &Default::default(),
            &Cancel::new(),
        ) {
            Ok(CallOutcome::Completed(value)) => Some(value),
            _ => None,
        }
    }

    pub fn requests(kinds: Vec<Kind>) -> BoxedStrategy<Request> {
        (
            proptest::sample::select(kinds),
            prop_oneof![
                3 => Just(Fold::Sum),
                1 => Just(Fold::SumNarrow),
                1 => Just(Fold::FoldAdd),
                1 => Just(Fold::FoldMul),
                1 => Just(Fold::FoldSubtract),
                2 => (0u8..=10).prop_map(Fold::Count),
                2 => (0u8..=9).prop_map(Fold::Exists),
                2 => (0u8..=10).prop_map(Fold::Forall),
            ],
            proptest::collection::btree_set(0u8..=9, 1..=4),
            proptest::collection::vec(0u8..=9, 1..=4),
        )
            .prop_flat_map(|(kind, fold, set, bag)| {
                // A set has distinct elements; a bag and a sequence may repeat.
                let elements: Vec<u8> = if kind == Kind::Set {
                    set.into_iter().collect()
                } else {
                    bag
                };
                let shuffled = Just(elements.clone()).prop_shuffle();
                (Just((kind, fold, elements)), shuffled)
            })
            .prop_map(|((kind, fold, elements), permuted)| Request {
                kind,
                fold,
                elements,
                permuted,
            })
            .boxed()
    }

    /// Whether the checker admits `fold` over a `kind` literal: an unproved `sum` range is
    /// refused (FR-680-AC-6), and so is a non-catalog step over a Set or Bag (FR-701-AC-2).
    fn admitted(kind: Kind, fold: Fold) -> bool {
        match fold {
            Fold::SumNarrow => false,
            Fold::FoldSubtract | Fold::Digits => kind == Kind::Sequence,
            _ => true,
        }
    }

    /// Provenance: QSL-2063 P1. Running one request twice writes byte-identical documents,
    /// completed outcomes and refusals (two kinds, see the module documentation) alike; no
    /// violation outcome is generated.
    #[test]
    fn same_request_twice_gives_byte_identical_outcomes() {
        let counts = std::cell::Cell::new((0u32, 0u32));
        prop::run(
            "same_request_twice_gives_byte_identical_outcomes",
            prop::config(256),
            &requests(vec![Kind::Sequence, Kind::Set, Kind::Bag]),
            |request| {
                let text = source(request.kind, request.fold, &request.elements);
                let (first, second) = (document(&text), document(&text));
                prop_assert_eq!(&first, &second, "{}", text);
                let success = String::from_utf8_lossy(&first).contains("\"category\":\"success\"");
                let expected = admitted(request.kind, request.fold);
                prop_assert_eq!(success, expected, "{}", text);
                let (c, r) = counts.get();
                counts.set(if expected { (c + 1, r) } else { (c, r + 1) });
                Ok(())
            },
        )
        .unwrap();
        let (completed, refused) = counts.get();
        eprintln!("determinism: {completed} completed requests, {refused} refused requests");
        assert!(completed > 0 && refused > 0);
    }

    /// Provenance: QSL-2063. Every fold form the generator draws completes over a Sequence,
    /// so the order properties are not vacuous: a form that was always refused would pass them.
    #[test]
    fn every_generated_fold_form_completes_over_a_sequence() {
        for fold in [
            Fold::Sum,
            Fold::Count(3),
            Fold::Exists(2),
            Fold::Forall(9),
            Fold::FoldAdd,
            Fold::FoldMul,
            Fold::FoldSubtract,
            Fold::Digits,
        ] {
            let text = source(Kind::Sequence, fold, &[1, 2, 3]);
            assert!(
                run_value(&text).is_some(),
                "{fold:?} did not complete:\n{text}\n{}",
                String::from_utf8_lossy(&document(&text))
            );
        }
    }

    /// Provenance: QSL-2063 negative. A request refused at check gives the same refusal
    /// document twice, and the document is a refusal.
    #[test]
    fn a_refused_request_gives_the_same_refusal_document_twice() {
        let text = source(Kind::Bag, Fold::SumNarrow, &[3, 4, 5]);
        let (first, second) = (document(&text), document(&text));
        assert_eq!(first, second);
        let written: serde_json::Value = serde_json::from_slice(&first).unwrap();
        assert_ne!(written["category"], "success", "{written}");
    }

    /// Provenance: QSL-2063 P2. Permuting the elements of a Set or Bag literal leaves the
    /// outcome document unchanged, for `sum`, `count`, `exists`, `forall` and the catalog `fold`
    /// steps. The language has no `min` or `max`, so neither is drawn.
    #[test]
    fn set_and_bag_folds_ignore_presentation_order() {
        prop::run(
            "set_and_bag_folds_ignore_presentation_order",
            prop::config(256),
            &requests(vec![Kind::Set, Kind::Bag]),
            |request| {
                let (a, b) = (
                    source(request.kind, request.fold, &request.elements),
                    source(request.kind, request.fold, &request.permuted),
                );
                prop_assert_eq!(document(&a), document(&b), "{}\n{}", a, b);
                Ok(())
            },
        )
        .unwrap();
    }

    // P3 (ticket: "Sequence order matters only where defined") is covered in part. The ticket
    // asks for the list of order-sensitive operations to come from the checker's own table, so
    // that a new positional operation fails the test. No such table exists: the checker marks no
    // operation positional (`ExprNode` in `fsl-forms/src/syntax.rs` is a closed enum with
    // `Index`, `DropFirst`, `Flatten`, `Accumulate` and others, and `fsl-semantics` classifies
    // only collection kinds as ordered, through `is_ordered`). A list written into this test
    // would be a literal asserted against itself, so none is. What the tests below hold:
    //
    // - permuting a Sequence leaves `sum`, `count`, `exists`, `forall` and the catalog and
    //   subtraction folds unchanged (`sequence_order_changes_only_order_sensitive_results`);
    // - an order-reading fold over a Sequence does change with the order
    //   (`a_sequence_read_in_order_is_observable`), so the permutation is a real one;
    // - the one positional read the checker types today, `xs[i]`, is refused for want of a
    //   presence proof (`an_index_with_no_presence_fact_is_refused`).

    /// Provenance: QSL-2063 P3. A Sequence permutation changes `sum`, `count`, `exists`,
    /// `forall` and the order-free folds not at all.
    #[test]
    fn sequence_order_changes_only_order_sensitive_results() {
        prop::run(
            "sequence_order_changes_only_order_sensitive_results",
            prop::config(256),
            &requests(vec![Kind::Sequence]),
            |request| {
                let (a, b) = (
                    source(Kind::Sequence, request.fold, &request.elements),
                    source(Kind::Sequence, request.fold, &request.permuted),
                );
                prop_assert_eq!(document(&a), document(&b), "{}\n{}", a, b);
                Ok(())
            },
        )
        .unwrap();
    }

    /// Provenance: QSL-2063 P3. A fold whose step reads the order of a Sequence
    /// (`acc * 10 + x`) gives a different result for a different order: the two presentations
    /// differ in their outcome documents exactly when they differ as sequences.
    #[test]
    fn a_sequence_read_in_order_is_observable() {
        prop::run(
            "a_sequence_read_in_order_is_observable",
            prop::config(256),
            &requests(vec![Kind::Sequence]),
            |request| {
                let (a, b) = (
                    source(Kind::Sequence, Fold::Digits, &request.elements),
                    source(Kind::Sequence, Fold::Digits, &request.permuted),
                );
                prop_assert_eq!(
                    document(&a) == document(&b),
                    request.elements == request.permuted,
                    "{}\n{}",
                    a,
                    b
                );
                Ok(())
            },
        )
        .unwrap();
    }

    /// Provenance: QSL-2063, FR-680-AC-1. This is not an order property. An indexed read `s[i]`
    /// of a Sequence with no presence fact is refused `undefined_expression`/`unproved-presence`
    /// whatever the element order and whatever `i` is (`size<T>` is missing, QSL-1966), so a
    /// Sequence literal cannot yet be read by position at all.
    #[test]
    fn an_index_with_no_presence_fact_is_refused() {
        prop::run(
            "an_index_with_no_presence_fact_is_refused",
            prop::config(256),
            &(requests(vec![Kind::Sequence]), 0usize..4),
            |(request, at)| {
                let at = at % request.elements.len();
                for elements in [&request.elements, &request.permuted] {
                    let text = format!(
                        "{HEADER}function f using v(): Int[0, 9] pure {{ let s: Sequence<Int[0, 9]>[0, 4] = ({}) in (s[{at}]) }}\n",
                        literal(Kind::Sequence, elements)
                    );
                    let written: serde_json::Value =
                        serde_json::from_slice(&document(&text)).expect("JSON");
                    prop_assert_eq!(
                        &written["diagnostics"][0]["cause"],
                        "unproved-presence",
                        "{}",
                        text
                    );
                }
                Ok(())
            },
        )
        .unwrap();
    }

    /// Provenance: QSL-2063 negative. The property of P2 (reversing the presentation leaves the
    /// outcome document unchanged) fails for an evaluator that reads presentation order, and
    /// shrinks to the two elements that make the difference. The order-reading evaluator is the
    /// digit fold over a Sequence, run through the real spine.
    #[test]
    fn order_dependence_is_caught() {
        let mut config = prop::config(256);
        config.failure_persistence = None;
        let outcome = prop::run(
            "order_dependence_is_caught",
            config,
            &proptest::collection::vec(0u8..=9, 1..=4),
            |elements| {
                let reversed: Vec<u8> = elements.iter().rev().copied().collect();
                let (a, b) = (
                    source(Kind::Sequence, Fold::Digits, &elements),
                    source(Kind::Sequence, Fold::Digits, &reversed),
                );
                prop_assert_eq!(document(&a), document(&b));
                Ok(())
            },
        );
        match outcome {
            Err(proptest::test_runner::TestError::Fail(_, minimal)) => {
                assert_eq!(minimal.len(), 2, "did not shrink: {minimal:?}");
            }
            other => panic!("order dependence went unnoticed: {other:?}"),
        }
    }
}
