---
id: SR-967175026302197768900474181941935084867
title: "Code review — QSL-3269 inline none history initial (PR #3962)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3962 head 7d880035e7f090aaaec3575e5058e7e34b3ccb69 against origin/main f9db25eff98230fb0f4c3d7f68800fc602a649c6; fsl-semantics/src/check/lowering/refinement_rows.rs; fsl-package/src/emit/tests/admission_corpus.rs; fsl-analyze/tests/it/analyze_family_cli.rs; FR-901-AC-1, FR-138-AC-9, FR-630-AC-3; FSpec FR-375 HistoryInitial, FR-375-AC-9, FR-375-AC-36 at filament-specification 691ac5a"
review_set: subset
---

## Summary

Ticket: QSL-3269. Code review with the rust-review lane folded in. The diff restores the
inline `literal` `none` history initial that #3881 (306febde7f) added and #3895 (88eb6bc392)
dropped when it rewrote `history_initial` to lower present options as `option_value` nodes.
Main is red because `none` at an `Option`-typed history became a reference to a `literal`
node, which the pinned strict reader (filament-contract-ir b81baf1,
`checked_package/v2/refinement.rs:2615-2672`) refuses with
`invalid_model_binding`/`malformed-declaration` at `histories/0/initial`. The author's red
logs at f9db25ef reproduce exactly that refusal (analyze) and the inline-form assertion
failure (package).

## Verdict

**CONDITIONAL** — the fix is correct and minimal. The inline `none` applies only when the
history value itself is absent (`option_types.is_empty()`), which is exactly where the pinned
reader and FSpec FR-375 `HistoryInitial` / AC-36 ("the same row with a `literal` `none` is
admitted") require it. A present option still lowers to an `option_value` reference whose
`semantic_type` is the history's own type, which the reader checks and the new test asserts.
Removing `a_present_option_history_initial_has_no_literal_form_and_is_refused_typed` is right:
#3895 lowered present option initials and deleted the FR-138 "current implementation gap"
note that the refusal pinned. FSpec FR-375-AC-9 and AC-36 now require admission, so no
required refusal is lost. Its helper `history_refusals` is still used by
`an_ill_typed_history_update_refuses_at_s3_not_as_not_yet_implemented` (FR-901-AC-4), and the
new package case covers the present-option form in its place. The analyze test keeps its
three S3 refusal cases. Two non-blocking findings follow.

## Rust review

The `inline` match is idiomatic, and no panic, unsafe or integer-conversion surface was
added. `#[implements("FR-901-AC-1")]` uses the bare attribute with
`use ix_trace_rs::implements;`, and `quire trace --id FR-901-AC-1` lists it as `implements`.
The tag is in its own commit (7d880035). The doc comment and the inline comment match the
reader's rule.

## Verification (author logs)

- red-package and red-analyze ran at f9db25ef (main) and fail as described.
- fix-package-lib (49 passed), fix-semantics-it (40 passed) and fix-analyze-mc (5 passed,
  including `option_history_initials_prove_or_refuse`) ran on an uncommitted tree over
  f9db25ef (head= f9db25ef). They finished 5 s to 2 min before commit 332470ca. 7d880035 adds
  only the attribute and its import.
- At 7d880035: semantics-lib (45 passed), clippy fsl-semantics, clippy fsl-package,
  clippy --no-deps fsl-analyze and `cargo check --workspace --all-targets --all-features` all
  passed (exit 0).
- clippy fsl-analyze with deps at 7d880035 failed in fsl-replay
  (`hybrid_replay/class.rs` dead fields `term_visits`… and `stats`). fsl-replay is not in the
  diff, and the same warning shows in the red-analyze log at f9db25ef, so this is pre-existing
  on main and not caused by this PR.
- The package and analyze tests were not re-run on the committed heads. The delta to
  7d880035 is attribute-only, so the earlier runs still stand.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The test now asserts that non-Option (Int, Boolean) history initials are a `reference` to a `literal` node. FSpec FR-375 `HistoryInitial` and AC-36 (pinned 691ac5a) admit a reference only to a `value`/`option_value` node of the history's `Option<T>` type, and say a reference at a non-Option history refuses. The pinned reader b81baf1 still admits it, so this shape will break the way `none` did once IR implements AC-36. The behaviour is from before this PR, but the diff asserts it again. | fsl-package/src/emit/tests/admission_corpus.rs:4302-4306; fsl-semantics/src/check/lowering/refinement_rows.rs:226-229 |
| FND-002 | low | The nested-option branch, an absent payload inside a present option (`option_types` non-empty), lowers to a `literal` node referenced by the `option_value`. The `option_types.is_empty()` guard and its comment have no test, so removing the guard or inverting it would go unnoticed. | fsl-semantics/src/check/lowering/refinement_rows.rs:213-218 |

## Disposition review round 1

Checked head 79bba218c3aa1129ba8f3d07d138aced84c00abd. The new commits are 223a87e74b (fix),
31cb3f22f6 (tags) and 79bba218c3 (these review records, with one heading edit).

- **Breadth of the behaviour change.** Every history initial that is not a present option now
  becomes the row's own inline `literal` term, through `Lowering::literal_value`. That function
  is split out of `literal_at`, so the node literals and the history initials share one
  value-to-literal decision.
  - Scalar kinds keep the kind they had before: Boolean, Integer, Rational, Decimal, Float,
    Text/String, Quantity, Uuid, Timestamp, Bytes and JsonObject. Absent options become `none`.
  - Composite, collection, reference, population, union and map values fault exactly as
    `literal_at` did before.
  - Top-level enums stay inline.
  - Inside a present option, enum payloads stay inline and every other payload stays a node,
    so the absent payload in `Option<Option<T>>` is a `literal` node.
  - The node path of `literal_at` is unchanged for expression literals, because its enum
    branch moved ahead of the `literal_value` call.
- **Readers of the change.** No FSL code reads a history `initial` term back:
  - fsl-replay and analyze compute histories from `CheckedRefinement.history`;
  - `refinement_view` refuses refinements that have histories;
  - contract-codegen has no history handling.
- **Author's r2 logs at 79bba218, all exit 0:**
  - package lib (66 passed);
  - semantics lib (192 passed) and semantics it (89 passed);
  - analyze it_model_check (30 passed) and it_refinement (92 passed);
  - fsl-replay refinement_history (29 passed);
  - conformance emitted_packages_are_admitted (2 passed);
  - clippy semantics, clippy package, clippy analyze `--no-deps`, and the workspace check.
- **Red runs.** r2-mutant-old (the package test against the 7d880035 lowering) fails on the
  Int initial. r2-mutant-nested (the guard mutated) fails on the nested test. Both new tests
  therefore discriminate.
- **Changed reader-refusal test.**
  `the_reader_refuses_a_history_row_whose_concrete_type_is_not_an_object_type` now puts the
  row's `type` node (an `Int[0, 1]` bounded domain, not an object type) into `concrete_type`.
  Before, it used the `literal` node, which no longer exists. The mutation stays a non-object
  node, and the test still asserts the refusal path `/histories/0/concrete_type`, so it proves
  the same reader behaviour.
  - Pre-existing and not caused by this PR: the test's tag FR-901-AC-4 ("update is
    ill-typed... refuses at S3") does not describe a reader refusal of `concrete_type`.
- **Tags from 31cb3f22f6.** `#[implements("FR-093-AC-33", "FR-093-AC-50")]` on `literal_at` and
  `literal_value` is justified:
  - AC-33: `literal_value` chooses `value_kind` `quantity` with the reduced magnitude, and
    `literal_at` builds the `literal` node typed at the unit node.
  - AC-50: `literal_value` keeps the profile-normalized text as `value_kind` `text`, and
    `literal_at` types the node at the `Text[...]` node.
  - `quire trace` lists all four as `implements`. Keep them.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The new unit test `an_absent_payload_inside_a_present_option_is_a_node_the_option_value_names` has no `#[trace]` tag, against the rule that every new test carries one for the criteria it asserts. Nearest FSL criterion: FR-901-AC-1 (TC-2540), as on the package test. | fsl-semantics/src/check/lowering/refinement_rows.rs:381 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 223a87e74b writes every non-present-option initial as an inline `literal` term. A `reference` remains only for a present option's `option_value`, per FSpec FR-375 HistoryInitial/AC-36. The package test asserts the inline term for integer, boolean, text, rational, decimal and none, and reader admission for each. |
| FND-002 | fixed | 223a87e74b adds a unit test for `Option<Option<Int[0, 1]>>` = present(none). It asserts an `option_value` of the declared type whose member references a `none` `literal` node of the inner type. r2-mutant-nested shows the test fails when the branch is mutated. |
