// SPDX-License-Identifier: AGPL-3.0-or-later
//! Per-family acceptance and rejection coverage for the QSL-2040 collection corpus.

use super::{corpus_root, Expectation};

struct Family {
    name: &'static str,
    accepting: &'static [&'static str],
    rejecting: &'static [&'static str],
}

fn expectation(stem: &str) -> Expectation {
    let path = corpus_root()
        .join("collections")
        .join(format!("{stem}.expected.json"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("{}: invalid expectation: {error}", path.display()))
}

pub(super) fn assert_families() {
    let families = [
        Family {
            name: "set",
            accepting: &["coll-set-ok", "r-set-dedupe", "r-set-eq"],
            rejecting: &["coll-set-index-bad"],
        },
        Family {
            name: "bag",
            accepting: &["coll-bag-ok", "r-bag"],
            rejecting: &["coll-bag-bad"],
        },
        Family {
            name: "ordered set",
            accepting: &["coll-oset-ok", "r-oset"],
            rejecting: &["coll-oset-bad"],
        },
        Family {
            name: "sequence",
            accepting: &["coll-seq-ok", "coll-nested-seq-ok", "r-seq"],
            rejecting: &["coll-dropfirst-bad", "coll-index-unproved-bad"],
        },
        Family {
            name: "map",
            accepting: &["coll-map-ok", "r-map-lookup"],
            rejecting: &["map-key-bad"],
        },
        Family {
            name: "aggregate",
            accepting: &[
                "agg-sum-named",
                "agg-count-ok",
                "agg-size-range-ok",
                "r-sum",
            ],
            rejecting: &[
                "agg-size-range-bad",
                "agg-size-unbounded-coll-bad",
                "agg-sum-narrow-bad",
            ],
        },
        Family {
            name: "quantifier",
            accepting: &["quant-forall-ok", "quant-nested", "quant-range-fact"],
            rejecting: &["quant-nonbool-bad", "quant-binder-shadow"],
        },
        Family {
            name: "function call",
            accepting: &["fn-call-ok", "fn-call-enum-arg", "fn-call-record-arg"],
            rejecting: &["fn-call-arity-bad", "fn-call-argrange-bad"],
        },
        Family {
            name: "recursion",
            accepting: &["fn-recursion-ok", "r-recursion"],
            rejecting: &["fn-recursion-nodecrease-bad", "fn-recursion-nomeasure-bad"],
        },
    ];

    for family in families {
        assert!(
            !family.accepting.is_empty() && !family.rejecting.is_empty(),
            "collection family {} needs accepting and rejecting programs",
            family.name
        );
        for stem in family.accepting {
            assert_eq!(
                expectation(stem).exit_code,
                0,
                "collections/{stem}: listed as accepting {}",
                family.name
            );
        }
        for stem in family.rejecting {
            assert_ne!(
                expectation(stem).exit_code,
                0,
                "collections/{stem}: listed as rejecting {}",
                family.name
            );
        }
    }
}
