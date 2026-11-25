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
    /// The failure the key is for, exactly as the run reports it: the
    /// actual outcome of a mismatch or the reason a case is unsupported. A
    /// case that fails any other way fails the run.
    pub pin: &'static str,
}

/// The runner names a registry entry may carry.
pub(crate) const RUNNERS: &[&str] = &["finite-window", "state-core"];

/// The tickets the registry may name: one per gap group (QSL-3027 to
/// QSL-3041). A key outside this set fails the run.
pub(crate) const ALLOWED_TICKETS: &[&str] = &[
    "QSL-3027", "QSL-3028", "QSL-3029", "QSL-3030", "QSL-3031", "QSL-3032", "QSL-3033", "QSL-3034",
    "QSL-3035", "QSL-3036", "QSL-3037", "QSL-3038", "QSL-3039", "QSL-3040", "QSL-3041",
];

/// The checked-in registry.
pub(crate) const PENDING: &[Pending] = &[
    // G2
    Pending {
        runner: "state-core",
        case: "typing/nested-shadowing",
        ticket: "QSL-3028",
        pin: r#"expected Refused("ill_typed"), actual Refused("ambiguous_declaration")"#,
    },
    // G3
    Pending {
        runner: "state-core",
        case: "typing/unreachable-unknown-name",
        ticket: "QSL-3029",
        pin: r#"expected Refused("ill_typed"), actual Refused("missing_declaration")"#,
    },
    // G4
    Pending {
        runner: "state-core",
        case: "typing/unproved-overflow",
        ticket: "QSL-3030",
        pin: r#"expected Refused("undefined_expression"), actual Completed(true)"#,
    },
    // G5
    Pending {
        runner: "state-core",
        case: "typing/ambiguous-literals",
        ticket: "QSL-3031",
        pin: r#"expected Refused("ill_typed"), actual Completed(true)"#,
    },
    // G6
    Pending {
        runner: "state-core",
        case: "state/duplicate-object",
        ticket: "QSL-3032",
        pin: r#"expected Refused("duplicate_object"), actual Refused("invalid_runtime_input")"#,
    },
    // G8
    Pending {
        runner: "state-core",
        case: "boundary/unsupported-collect",
        ticket: "QSL-3034",
        pin: r#"expected Refused("unsupported_construct"), actual Refused("invalid_syntax")"#,
    },
    // G9
    Pending {
        runner: "state-core",
        case: "boundary/foreign-import",
        ticket: "QSL-3035",
        pin: r#"expected Refused("foreign_reference"), actual Refused("invalid_model_binding")"#,
    },
    Pending {
        runner: "state-core",
        case: "boundary/ambiguous-export",
        ticket: "QSL-3035",
        pin: r#"expected Refused("ambiguous_declaration"), actual Refused("invalid_model_binding")"#,
    },
    // G10
    Pending {
        runner: "state-core",
        case: "boundary/unbounded-integer",
        ticket: "QSL-3036",
        pin: r#"expected Refused("unbounded_domain"), actual Completed(true)"#,
    },
    // G11: published 0-draft sources
    Pending {
        runner: "state-core",
        case: "source/parent-order.native",
        ticket: "QSL-3037",
        pin: r#"source/parent-order.native: the published source should parse and check, actual Refused("unknown_edition")"#,
    },
    Pending {
        runner: "state-core",
        case: "source/version-update.native",
        ticket: "QSL-3037",
        pin: r#"source/version-update.native: the published source should parse and check, actual Refused("unknown_edition")"#,
    },
    // G12: no semantic-reference reader
    Pending {
        runner: "state-core",
        case: "reference/same-exact-reference",
        ticket: "QSL-3038",
        pin: r#"same-exact-reference: FSL has no reader that computes a semantic reference from an artifact reference"#,
    },
    Pending {
        runner: "state-core",
        case: "reference/feature-set-reordered",
        ticket: "QSL-3038",
        pin: r#"feature-set-reordered: FSL has no reader that computes a semantic reference from an artifact reference"#,
    },
    Pending {
        runner: "state-core",
        case: "reference/unknown-reference-version",
        ticket: "QSL-3038",
        pin: r#"unknown-reference-version: FSL has no reader that computes a semantic reference from an artifact reference"#,
    },
    Pending {
        runner: "state-core",
        case: "reference/unknown-wire-version",
        ticket: "QSL-3038",
        pin: r#"unknown-wire-version: FSL has no reader that computes a semantic reference from an artifact reference"#,
    },
    Pending {
        runner: "state-core",
        case: "reference/unknown-semantic-profile",
        ticket: "QSL-3038",
        pin: r#"unknown-semantic-profile: FSL has no reader that computes a semantic reference from an artifact reference"#,
    },
    Pending {
        runner: "state-core",
        case: "reference/unknown-required-feature",
        ticket: "QSL-3038",
        pin: r#"unknown-required-feature: FSL has no reader that computes a semantic reference from an artifact reference"#,
    },
    Pending {
        runner: "state-core",
        case: "reference/foreign-authority",
        ticket: "QSL-3038",
        pin: r#"foreign-authority: FSL has no reader that computes a semantic reference from an artifact reference"#,
    },
    Pending {
        runner: "state-core",
        case: "reference/wrong-artifact-kind",
        ticket: "QSL-3038",
        pin: r#"wrong-artifact-kind: FSL has no reader that computes a semantic reference from an artifact reference"#,
    },
    Pending {
        runner: "state-core",
        case: "reference/duplicate-required-feature",
        ticket: "QSL-3038",
        pin: r#"duplicate-required-feature: FSL has no reader that computes a semantic reference from an artifact reference"#,
    },
    Pending {
        runner: "state-core",
        case: "reference/changed-language-edition",
        ticket: "QSL-3038",
        pin: r#"changed-language-edition: FSL has no reader that computes a semantic reference from an artifact reference"#,
    },
    // G13: boundary wire and subject cases
    Pending {
        runner: "state-core",
        case: "boundary/unknown-language-profile",
        ticket: "QSL-3039",
        pin: r#"unknown-language-profile: the fixture sets profile.version of subject.json to 999 and no FSL reader accepts the subject wire; renaming the source profile is a different input"#,
    },
    Pending {
        runner: "state-core",
        case: "boundary/deleted-object",
        ticket: "QSL-3039",
        pin: r#"deleted-object: the fixture dereferences a removed object in the post snapshot of a future operation; a dangling parent in a current snapshot is a different input"#,
    },
    Pending {
        runner: "state-core",
        case: "boundary/unknown-wire",
        ticket: "QSL-3039",
        pin: r#"unknown-wire: no source adapter for the mutation of "subject.json" (unknown-wire)"#,
    },
    Pending {
        runner: "state-core",
        case: "boundary/unknown-feature",
        ticket: "QSL-3039",
        pin: r#"unknown-feature: no source adapter for the mutation of "subject.json" (unknown-feature)"#,
    },
    Pending {
        runner: "state-core",
        case: "boundary/bad-byte-span",
        ticket: "QSL-3039",
        pin: r#"bad-byte-span: no source adapter for the mutation of "subject.json" (bad-byte-span)"#,
    },
    Pending {
        runner: "state-core",
        case: "boundary/split-unicode",
        ticket: "QSL-3039",
        pin: r#"split-unicode: no source adapter for the mutation of "subject.json" (split-unicode)"#,
    },
    Pending {
        runner: "state-core",
        case: "boundary/line-byte-mismatch",
        ticket: "QSL-3039",
        pin: r#"line-byte-mismatch: no source adapter for the mutation of "subject.json" (line-byte-mismatch)"#,
    },
    Pending {
        runner: "state-core",
        case: "boundary/wrong-invocation",
        ticket: "QSL-3039",
        pin: r#"wrong-invocation: the snapshot wire carries no invocation member, so pre and post cannot name different invocations"#,
    },
    Pending {
        runner: "state-core",
        case: "boundary/unavailable-observation",
        ticket: "QSL-3039",
        pin: r#"unavailable-observation: the snapshot wire has no unavailable field value; only a whole missing document is unavailable"#,
    },
    Pending {
        runner: "state-core",
        case: "invocation/wrong-invocation",
        ticket: "QSL-3039",
        pin: r#"wrong-invocation: the snapshot wire carries no invocation member, so the post snapshot cannot name another invocation"#,
    },
    // G14: typing model adapter
    Pending {
        runner: "state-core",
        case: "typing/mismatched-units",
        ticket: "QSL-3040",
        pin: r#"mismatched-units: the model has no `distance`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/unit-division-not-admitted",
        ticket: "QSL-3040",
        pin: r#"unit-division-not-admitted: the model has no `distance`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/negative-division",
        ticket: "QSL-3040",
        pin: r#"negative-division: the model has no `signed`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/negative-remainder",
        ticket: "QSL-3040",
        pin: r#"negative-remainder: the model has no `signed`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/possible-zero-divisor",
        ticket: "QSL-3040",
        pin: r#"possible-zero-divisor: the model has no `signed`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/nonzero-guard",
        ticket: "QSL-3040",
        pin: r#"nonzero-guard: the model has no `signed`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/minimum-divided-by-minus-one",
        ticket: "QSL-3040",
        pin: r#"minimum-divided-by-minus-one: the model has no `wide`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/size-preserves-duplicates",
        ticket: "QSL-3040",
        pin: r#"size-preserves-duplicates: the model has no `count`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/size-needs-context",
        ticket: "QSL-3040",
        pin: r#"size-needs-context: the model has no `items`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/empty-forall",
        ticket: "QSL-3040",
        pin: r#"empty-forall: the model has no `items`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/empty-exists",
        ticket: "QSL-3040",
        pin: r#"empty-exists: the model has no `items`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/exists-ordered-short-circuit",
        ticket: "QSL-3040",
        pin: r#"exists-ordered-short-circuit: the model has no `items`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/missing-sequence",
        ticket: "QSL-3040",
        pin: r#"missing-sequence: the model has no `items`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/collect-duplicate-output-refused",
        ticket: "QSL-3040",
        pin: r#"collect-duplicate-output-refused: the model has no `items`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/equal-fields-distinct-objects",
        ticket: "QSL-3040",
        pin: r#"equal-fields-distinct-objects: the model has no `peer`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/one-edge-self-cycle",
        ticket: "QSL-3040",
        pin: r#"one-edge-self-cycle: the model has no `reaches`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/two-edge-cycle",
        ticket: "QSL-3040",
        pin: r#"two-edge-cycle: the model has no `reaches`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/zero-edge-is-not-reachability",
        ticket: "QSL-3040",
        pin: r#"zero-edge-is-not-reachability: the model has no `reaches`; the typing model's Node is not written as a domain package"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/graph-budget-is-not-depth-bound",
        ticket: "QSL-3040",
        pin: r#"graph-budget-is-not-depth-bound: the model has no `reaches`; the typing model's Node is not written as a domain package"#,
    },
    // G15: typing request adapters
    Pending {
        runner: "state-core",
        case: "typing/pre-guard-cannot-protect-post",
        ticket: "QSL-3041",
        pin: r#"pre-guard-cannot-protect-post: a post anchor needs an operation invocation adapter"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/same-pre-value-guard",
        ticket: "QSL-3041",
        pin: r#"same-pre-value-guard: a post anchor needs an operation invocation adapter"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/captured-post-value-not-retagged",
        ticket: "QSL-3041",
        pin: r#"captured-post-value-not-retagged: a post anchor needs an operation invocation adapter"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/pre-preserves-captured-alias",
        ticket: "QSL-3041",
        pin: r#"pre-preserves-captured-alias: a post anchor needs an operation invocation adapter"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/boolean-operation-result",
        ticket: "QSL-3041",
        pin: r#"boolean-operation-result: a post anchor needs an operation invocation adapter"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/wrong-invocation",
        ticket: "QSL-3041",
        pin: r#"wrong-invocation: a post anchor needs an operation invocation adapter"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/missing-pre-observation",
        ticket: "QSL-3041",
        pin: r#"missing-pre-observation: a post anchor needs an operation invocation adapter"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/deleted-post-self",
        ticket: "QSL-3041",
        pin: r#"deleted-post-self: a post anchor needs an operation invocation adapter"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/zero-expression-budget",
        ticket: "QSL-3041",
        pin: r#"zero-expression-budget: an expression or graph budget has no request adapter"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/let-initializer-once",
        ticket: "QSL-3041",
        pin: r#"let-initializer-once: an expression or graph budget has no request adapter"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/let-exhausted-before-second-read",
        ticket: "QSL-3041",
        pin: r#"let-exhausted-before-second-read: an expression or graph budget has no request adapter"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/skipped-dangling-reference",
        ticket: "QSL-3041",
        pin: r#"skipped-dangling-reference: the hypotheses are not a plain snapshot of self and other"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/incomplete-universe",
        ticket: "QSL-3041",
        pin: r#"incomplete-universe: the hypotheses are not a plain snapshot of self and other"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/nullable-is-not-absence",
        ticket: "QSL-3041",
        pin: r#"nullable-is-not-absence: the hypotheses are not a plain snapshot of self and other"#,
    },
    Pending {
        runner: "state-core",
        case: "typing/cancelled-expression",
        ticket: "QSL-3041",
        pin: r#"cancelled-expression: the hypotheses are not a plain snapshot of self and other"#,
    },
];
