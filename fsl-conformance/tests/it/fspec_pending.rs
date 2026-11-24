// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cases of the standard's source fixtures that the compiler does not yet
//! answer as the fixture says (QSL-2048). Each entry keys one case to the
//! open Linear ticket of its own gap group; keys only, no hashes. An unkeyed
//! mismatch fails the run, and so does a keyed case that starts to match or
//! a key outside [`ALLOWED_TICKETS`].

/// One keyed case.
pub(crate) struct Pending {
    /// The runner: `finite-window` or `state-core`.
    pub runner: &'static str,
    /// The case id in the fixture (state-core ids are `file/id`).
    pub case: &'static str,
    /// The open ticket that owns the gap.
    pub ticket: &'static str,
}

/// The tickets the registry may name: one per gap group (QSL-3027 to
/// QSL-3041). A key outside this set fails the run.
pub(crate) const ALLOWED_TICKETS: &[&str] = &[
    "QSL-3027", "QSL-3028", "QSL-3029", "QSL-3030", "QSL-3031", "QSL-3032", "QSL-3033", "QSL-3034",
    "QSL-3035", "QSL-3036", "QSL-3037", "QSL-3038", "QSL-3039", "QSL-3040", "QSL-3041",
];

/// The checked-in registry.
pub(crate) const PENDING: &[Pending] = &[
    // G1: evaluator internal fault
    Pending {
        runner: "state-core",
        case: "typing/absent-parent-implication",
        ticket: "QSL-3027",
    },
    Pending {
        runner: "state-core",
        case: "typing/present-parent-violation",
        ticket: "QSL-3027",
    },
    // G2
    Pending {
        runner: "state-core",
        case: "typing/nested-shadowing",
        ticket: "QSL-3028",
    },
    // G3
    Pending {
        runner: "state-core",
        case: "typing/unreachable-unknown-name",
        ticket: "QSL-3029",
    },
    // G4
    Pending {
        runner: "state-core",
        case: "typing/unproved-overflow",
        ticket: "QSL-3030",
    },
    // G5
    Pending {
        runner: "state-core",
        case: "typing/ambiguous-literals",
        ticket: "QSL-3031",
    },
    // G6
    Pending {
        runner: "state-core",
        case: "state/duplicate-object",
        ticket: "QSL-3032",
    },
    // G7
    Pending {
        runner: "state-core",
        case: "invocation/deleted-self-post-context",
        ticket: "QSL-3033",
    },
    // G8
    Pending {
        runner: "state-core",
        case: "boundary/unsupported-collect",
        ticket: "QSL-3034",
    },
    // G9
    Pending {
        runner: "state-core",
        case: "boundary/foreign-import",
        ticket: "QSL-3035",
    },
    Pending {
        runner: "state-core",
        case: "boundary/ambiguous-export",
        ticket: "QSL-3035",
    },
    // G10
    Pending {
        runner: "state-core",
        case: "boundary/unbounded-integer",
        ticket: "QSL-3036",
    },
    // G11: published 0-draft sources
    Pending {
        runner: "state-core",
        case: "source/parent-order.native",
        ticket: "QSL-3037",
    },
    Pending {
        runner: "state-core",
        case: "source/version-update.native",
        ticket: "QSL-3037",
    },
    // G12: no semantic-reference reader
    Pending {
        runner: "state-core",
        case: "reference/same-exact-reference",
        ticket: "QSL-3038",
    },
    Pending {
        runner: "state-core",
        case: "reference/feature-set-reordered",
        ticket: "QSL-3038",
    },
    Pending {
        runner: "state-core",
        case: "reference/unknown-reference-version",
        ticket: "QSL-3038",
    },
    Pending {
        runner: "state-core",
        case: "reference/unknown-wire-version",
        ticket: "QSL-3038",
    },
    Pending {
        runner: "state-core",
        case: "reference/unknown-semantic-profile",
        ticket: "QSL-3038",
    },
    Pending {
        runner: "state-core",
        case: "reference/unknown-required-feature",
        ticket: "QSL-3038",
    },
    Pending {
        runner: "state-core",
        case: "reference/foreign-authority",
        ticket: "QSL-3038",
    },
    Pending {
        runner: "state-core",
        case: "reference/wrong-artifact-kind",
        ticket: "QSL-3038",
    },
    Pending {
        runner: "state-core",
        case: "reference/duplicate-required-feature",
        ticket: "QSL-3038",
    },
    Pending {
        runner: "state-core",
        case: "reference/changed-language-edition",
        ticket: "QSL-3038",
    },
    // G13: boundary wire and subject cases
    Pending {
        runner: "state-core",
        case: "boundary/unknown-wire",
        ticket: "QSL-3039",
    },
    Pending {
        runner: "state-core",
        case: "boundary/unknown-feature",
        ticket: "QSL-3039",
    },
    Pending {
        runner: "state-core",
        case: "boundary/bad-byte-span",
        ticket: "QSL-3039",
    },
    Pending {
        runner: "state-core",
        case: "boundary/split-unicode",
        ticket: "QSL-3039",
    },
    Pending {
        runner: "state-core",
        case: "boundary/line-byte-mismatch",
        ticket: "QSL-3039",
    },
    Pending {
        runner: "state-core",
        case: "boundary/wrong-invocation",
        ticket: "QSL-3039",
    },
    Pending {
        runner: "state-core",
        case: "boundary/unavailable-observation",
        ticket: "QSL-3039",
    },
    Pending {
        runner: "state-core",
        case: "invocation/wrong-invocation",
        ticket: "QSL-3039",
    },
    // G14: typing model adapter
    Pending {
        runner: "state-core",
        case: "typing/mismatched-units",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/unit-division-not-admitted",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/negative-division",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/negative-remainder",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/possible-zero-divisor",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/nonzero-guard",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/minimum-divided-by-minus-one",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/size-preserves-duplicates",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/size-needs-context",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/empty-forall",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/empty-exists",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/exists-ordered-short-circuit",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/missing-sequence",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/collect-duplicate-output-refused",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/equal-fields-distinct-objects",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/one-edge-self-cycle",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/two-edge-cycle",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/zero-edge-is-not-reachability",
        ticket: "QSL-3040",
    },
    Pending {
        runner: "state-core",
        case: "typing/graph-budget-is-not-depth-bound",
        ticket: "QSL-3040",
    },
    // G15: typing request adapters
    Pending {
        runner: "state-core",
        case: "typing/pre-guard-cannot-protect-post",
        ticket: "QSL-3041",
    },
    Pending {
        runner: "state-core",
        case: "typing/same-pre-value-guard",
        ticket: "QSL-3041",
    },
    Pending {
        runner: "state-core",
        case: "typing/captured-post-value-not-retagged",
        ticket: "QSL-3041",
    },
    Pending {
        runner: "state-core",
        case: "typing/pre-preserves-captured-alias",
        ticket: "QSL-3041",
    },
    Pending {
        runner: "state-core",
        case: "typing/boolean-operation-result",
        ticket: "QSL-3041",
    },
    Pending {
        runner: "state-core",
        case: "typing/wrong-invocation",
        ticket: "QSL-3041",
    },
    Pending {
        runner: "state-core",
        case: "typing/missing-pre-observation",
        ticket: "QSL-3041",
    },
    Pending {
        runner: "state-core",
        case: "typing/deleted-post-self",
        ticket: "QSL-3041",
    },
    Pending {
        runner: "state-core",
        case: "typing/zero-expression-budget",
        ticket: "QSL-3041",
    },
    Pending {
        runner: "state-core",
        case: "typing/let-initializer-once",
        ticket: "QSL-3041",
    },
    Pending {
        runner: "state-core",
        case: "typing/let-exhausted-before-second-read",
        ticket: "QSL-3041",
    },
    Pending {
        runner: "state-core",
        case: "typing/skipped-dangling-reference",
        ticket: "QSL-3041",
    },
    Pending {
        runner: "state-core",
        case: "typing/incomplete-universe",
        ticket: "QSL-3041",
    },
    Pending {
        runner: "state-core",
        case: "typing/nullable-is-not-absence",
        ticket: "QSL-3041",
    },
    Pending {
        runner: "state-core",
        case: "typing/cancelled-expression",
        ticket: "QSL-3041",
    },
];
