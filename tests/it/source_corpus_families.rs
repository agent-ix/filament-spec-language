// SPDX-License-Identifier: AGPL-3.0-or-later
//! Per-family accept and reject coverage, and the pinned `flips_with` keys,
//! of the collections, operations and protocol corpora (QSL-2040,
//! QSL-2041, QSL-2043).

use super::{corpus_root, Expectation};

/// A language family and the corpus programs that accept and refuse it.
pub(super) struct Family {
    pub(super) name: &'static str,
    pub(super) accepting: &'static [&'static str],
    pub(super) rejecting: &'static [&'static str],
}

fn expectation(area: &str, stem: &str) -> Expectation {
    let path = corpus_root()
        .join(area)
        .join(format!("{stem}.expected.json"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("{}: invalid expectation: {error}", path.display()))
}

/// Each family has at least one accepting program (exit 0) and one
/// refused or violated program (exit not 0) in `area`.
pub(super) fn assert_families(area: &str, families: &[Family]) {
    for family in families {
        assert!(
            !family.accepting.is_empty() && !family.rejecting.is_empty(),
            "{area}: family {} needs an accepting and a rejecting program",
            family.name
        );
        for stem in family.accepting {
            assert_eq!(
                expectation(area, stem).exit_code,
                0,
                "{area}/{stem}: listed as accepting {}",
                family.name
            );
        }
        for stem in family.rejecting {
            assert_ne!(
                expectation(area, stem).exit_code,
                0,
                "{area}/{stem}: listed as rejecting {}",
                family.name
            );
        }
    }
}

/// Each listed program is pinned at today's outcome with this flip key.
pub(super) fn assert_flips(area: &str, flips: &[(&str, &str)]) {
    for (stem, key) in flips {
        assert_eq!(
            expectation(area, stem).flips_with.as_deref(),
            Some(*key),
            "{area}/{stem}: pinned flip key changed"
        );
    }
}

pub(super) const OPERATION_FAMILIES: &[Family] = &[
    Family {
        name: "invariant",
        accepting: &["inv-ok", "inv-enum-ok", "inv-deref-ok"],
        rejecting: &[
            "inv-nonbool-bad",
            "inv-pre-bad",
            "inv-unknown-enum-member-bad",
        ],
    },
    Family {
        name: "precondition",
        accepting: &["pre-ok", "pre-ready"],
        rejecting: &["pre-unknown-op-bad", "pre-unknown-param-bad", "pre-low"],
    },
    Family {
        name: "postcondition",
        accepting: &["post-ok", "post-result"],
        rejecting: &["post-result-noresult-bad", "post-result-kind-bad"],
    },
    Family {
        name: "frame",
        accepting: &["frame-same", "frame-allowed-change"],
        rejecting: &["frame-forbidden-parent", "frame-created-mismatch"],
    },
    Family {
        name: "snapshot run",
        accepting: &["p-healthy"],
        rejecting: &["p-wrong-model", "request-selection-model", "p-dangling"],
    },
];

pub(super) const PROTOCOL_FAMILIES: &[Family] = &[
    Family {
        name: "role",
        accepting: &["declaration-minimal", "roles-replicated"],
        rejecting: &["duplicate-role", "protocol-wrong-role"],
    },
    Family {
        name: "channel",
        accepting: &["channels-loss-head", "channels-fifo-by-part"],
        rejecting: &["channels-object-payload", "object-message"],
    },
    Family {
        name: "step",
        accepting: &["steps-all-kinds", "exchange"],
        rejecting: &[
            "steps-effect-of-send",
            "steps-receive-other-channel",
            "steps-unknown-contract",
        ],
    },
    Family {
        name: "choice",
        accepting: &["choice", "inspection-compiles"],
        rejecting: &[
            "choice-overlap",
            "choice-visibility",
            "p-nonexhaustive",
            "protocol-overlapping-verdict",
        ],
    },
    Family {
        name: "parallel and join",
        accepting: &["control-parallel-all", "parallel-all"],
        rejecting: &["control-join-omits-branch", "parallel-missing-branch"],
    },
    Family {
        name: "repeat",
        accepting: &["control-repeat-progress", "repeat-zero"],
        rejecting: &["control-repeat-no-progress", "repeat-no-progress"],
    },
    Family {
        name: "await",
        accepting: &["await", "timeouts-await"],
        rejecting: &["timeouts-await-without-profile", "await-unknown-profile"],
    },
    Family {
        name: "compensation",
        accepting: &["staged-saga-compensation"],
        rejecting: &["compensation-zero-attempts"],
    },
];

pub(super) const OPERATION_FLIPS: &[(&str, &str)] =
    &[("prof-inv-slash", "QSL-2041:state-core-slash-form")];

pub(super) const PROTOCOL_FLIPS: &[(&str, &str)] = &[
    ("p-two-protocols-one-name", "G-114"),
    ("p-channel-bounded-scalar", "QSL-1137"),
    ("p-fifo-value-key", "QSL-1997 / G-129"),
    ("p-role-on-model", "QSL-1136"),
    ("p-protocol-reference", "G-119"),
    ("p-over-scalar", "G-120"),
    ("p-terminal-reads-over-on-each", "G-121"),
    ("p-repeat-post-test-body-binder", "G-122"),
    (
        "staged-saga-deadlock-free",
        "QSL-2043:deadlock-unbounded-extent",
    ),
];
