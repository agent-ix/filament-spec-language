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
mod determinism {
    use std::collections::BTreeMap;

    use filament_exact::{Cancel, Integer};
    use fsl_bench::prop;
    use fsl_foundation::SourceIdentity;
    use fsl_replay::spine::{self, default_accounting, Call, CallOutcome, CallValue, DependencyInput, SpineLimits};
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
        let bind = |body: &str| {
            format!("let s: {collection}<Int[0, 9]>[0, 4] = ({c}) in ({body})")
        };
        let (result, body) = match fold {
            Fold::Sum => ("Total", bind("sum<Total>(x in s: x)")),
            Fold::SumNarrow => ("Narrow", bind("sum<Narrow>(x in s: x)")),
            Fold::Count(k) => ("Small", bind(&format!("count<Small>(x in s: x < {k})"))),
            Fold::Exists(k) => ("Boolean", bind(&format!("exists(x in s: x = {k})"))),
            Fold::Forall(k) => ("Boolean", bind(&format!("forall(x in s: x < {k})"))),
        };
        format!("{HEADER}type Total = Int[0, 100];\ntype Small = Int[0, 4];\ntype Narrow = Int[0, 5];\nfunction f using v(): {result} pure {{ {body} }}\n")
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
        OutcomeDocument::from_run(&result).to_bytes().expect("encodes")
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

    /// Provenance: QSL-2063 P1. Running one request twice writes byte-identical documents,
    /// completed, refused and violated outcomes alike.
    #[test]
    fn same_request_twice_gives_byte_identical_outcomes() {
        let mut refused = 0;
        let mut completed = 0;
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
                prop_assert_eq!(success, request.fold != Fold::SumNarrow, "{}", text);
                let (c, r) = counts.get();
                counts.set(if request.fold == Fold::SumNarrow { (c, r + 1) } else { (c + 1, r) });
                Ok(())
            },
        )
        .unwrap();
        (completed, refused) = counts.get();
        eprintln!("determinism: {completed} completed-shape requests, {refused} refusal-shape requests");
        assert!(completed > 0 && refused > 0);
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
    /// outcome document unchanged.
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

    /// The Sequence operations whose result depends on element order. The checker has no table
    /// marking operations positional; the only positional form it checks is the indexed read
    /// `ExprNode::Index` (`xs[i]`), so the list is that one form.
    pub const ORDER_SENSITIVE: &[&str] = &["index"];

    /// Provenance: QSL-2063 P3. A Sequence permutation changes `sum`, `count`, `exists` and
    /// `forall` not at all. The one positional form, the indexed read `xs[i]`, is refused on a
    /// literal for want of a presence proof (`size<T>` is missing, QSL-1966), so it cannot
    /// complete with a value yet; the refusal is pinned so that the day it can, this test
    /// fails and the read gets its own order property.
    #[test]
    fn sequence_order_changes_only_order_sensitive_results() {
        assert_eq!(ORDER_SENSITIVE, ["index"]);
        prop::run(
            "sequence_order_changes_only_order_sensitive_results",
            prop::config(256),
            &(requests(vec![Kind::Sequence]), 0usize..4),
            |(request, at)| {
                let (a, b) = (
                    source(Kind::Sequence, request.fold, &request.elements),
                    source(Kind::Sequence, request.fold, &request.permuted),
                );
                prop_assert_eq!(document(&a), document(&b), "{}\n{}", a, b);
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

    /// Provenance: QSL-2063 negative. A fold that reads presentation order (the first presented
    /// element) is not order-free; the property that Set and Bag folds ignore presentation order
    /// detects it on a reversed Bag and shrinks to two elements.
    #[test]
    fn order_dependence_is_caught() {
        let mut config = prop::config(256);
        config.failure_persistence = None;
        let outcome = prop::run(
            "order_dependence_is_caught",
            config,
            &requests(vec![Kind::Bag]),
            |request| {
                let first = |elements: &[u8]| elements.first().copied();
                let reversed: Vec<u8> = request.elements.iter().rev().copied().collect();
                prop_assert_eq!(first(&request.elements), first(&reversed));
                Ok(())
            },
        );
        match outcome {
            Err(proptest::test_runner::TestError::Fail(_, minimal)) => {
                assert_eq!(minimal.elements.len(), 2, "did not shrink: {minimal:?}");
            }
            other => panic!("order dependence went unnoticed: {other:?}"),
        }
    }
}
