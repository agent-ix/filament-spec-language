---
id: SR-2026112116405040524100
title: "Code review (Rust lane) — PR #3307 bounded Rational literals and rational.div dispatch on the package route"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@e1900638963fc3fa3fe06b6968b3cd346400ed8c (base d25728d74d830ccde8e0aae5c14d0b315af0db1e); fsl-package/src/reader_protocol_projection.rs, fsl-eval/tests/it/protocol_slots/acceptance.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-390
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-533
    type: reviews
---
# SR-2026112116405040524100: Code review (Rust lane) — PR #3307

## Summary

Ticket: QSL-2271 (PR filament-spec-language#3307, which does not complete the ticket). I reviewed the
whole PR diff from base `d25728d7` to the frozen head `e1900638`: two files, +182/-5. It is
`code-review` with the `rust-review` lane folded in, using the rust-style baseline and its testing
reference, plus the repo's AGENTS.md and CLAUDE.md. I ran no runtime, as the brief requires. The
headline: the product change is correct and lines up with the source checker. One low test-coverage
finding remains. The trace-tag defect is recorded once, in the separate gap-analysis artifact
SR-2026112116405040524101 (FND-001), and is not duplicated here.

## Verdict

**CONDITIONAL**: one low finding, and no wrong result found in the product diff. The focused
runtime at head `e1900638` is still pending, and it belongs to the root. The reviewer did not run
it, and this verdict does not claim it.

What was verified by reading the source:

- **Literal fix (L/R/S).** `bounded_rational_literal_type` (reader_protocol_projection.rs:4346-4373)
  accepts only `UnsupportedExactType` with form `scalar_type/rational`. It also requires the node at
  `type_id` to be `bounded_domain`/`rational_range`, `error.at` to equal that node's `semantic_type`,
  and the semantic-type node to be `scalar_type/rational`. It reads the bounds from R through the
  existing `rational_range_type`, never from S. This matches the emitter: lowering.rs `scalar("rational")`
  is wrapped by `bounded("rational_range", base, …)`, and IR `project_exact_type_inner` refuses at the
  requested node's semantic type. Both call sites use the helper: `project_literal_type`, which is
  reached from `literal_value` on the fresh and inline paths, and the second lookup in
  `project_inline_literal`. No other error is swallowed. The Text and unit/dimension fallbacks are
  unchanged. The string tag comparisons follow the file's existing idiom (line 3932), and `WireTag`
  has no `BoundedDomain` variant.
- **Division dispatch.** check.rs:4542 produces `NodeKind::Divide` only when both operands are
  `is_integer` (`Integer | Int(_)`) and the hint is Rational. check.rs:4581 produces
  `NodeKind::Rational { Divide }` only for Rational/Rational. lowering.rs:4812-4825 emits both as
  `filament.op.rational.div`. The reader arm (reader_protocol_projection.rs:2541-2577) now selects
  between them using the projected operand `value_type`, with the same two predicates. A mixed pair
  cannot be produced by the checker, so it is a typed `InternalFault`, named in the file's
  invariant-fault style. The domain still comes from the node's projected Rational result type,
  which is what the checker stores in both kinds. The reader now builds the same `NodeKind` as the
  source, so evaluation, outcome, losses and charges follow the same kernel path. Integer division
  (`filament.op.integer.*`), Rational add/sub/mul and every Decimal arm (`filament.op.decimal.*`,
  plus #3304's decoder and typed Result) have no changes in the diff, so there is no Decimal
  regression to re-measure.
- **Tests.** `assert_guard_true_on_both_routes` evaluates the source route first and asserts
  `Completed(Boolean(true))`. Only then does it build the reader view, panicking with the refusal if
  construction fails. It then compares outcome and losses through the existing `without_location`,
  and charges and usage through the existing `charges_and_usage_match`. No guard stops before the
  arithmetic: `opened.ready` is supplied `true`, so `and` evaluates the right operand. Each fixture
  can fail. Truncating or rounding 1/2 falsifies `between` and both `= rational(1, 2)` checks. The
  pre-fix Divide-only dispatch faulted the Rational case, as measured on draco at `0a9954a6`.
  `guard_source` is a new helper. The existing tests at acceptance.rs:3820/4039/4097 inline the same
  `SOURCE.replace` profile edit, and I found no existing function it duplicates.
- **Grounding of the integer-slash fixture.** `inv(x: Int[1, 9]): Rational[0, 1; 1, 9] { 1 / x }` has
  the same signature and body as fsl-replay/src/execute/tests.rs:2169. Its source check inside this
  unit is still unconfirmed until the root runs it. This test also passes against the base dispatch,
  so it is a control for the Int branch, not a reproduction. `reader_backed_activation_guard_runs_integer_division`
  exercises `filament.op.integer.div`, which this diff does not touch, so it adds parity coverage but
  is not evidence for the changed arm.
- **Duplication and vendoring, ceremony.** None found. There are no IR, API, schema, pin, Cargo or CI
  changes.
- **Gates.** The reviewer ran no `fmt`, `clippy` or tests, because the brief forbids reviewer
  runtime. The coder reports `cargo check --locked -j1 -p fsl-eval --test it` exit 0, which I have
  not verified. The root's focused runtime is pending.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | low      | No test fails if `bounded_rational_literal_type`'s structural guards are loosened: all four tests take the accepting path only, so deleting the form, tag, `error.at == semantic_type` or scalar check (accepting any `UnsupportedExactType` whose `type_id` has readable range bounds) leaves every test green, and the PR's "malformed literals are still refused as MissingLiteralProjection" claim has no evidence. | fsl-package/src/reader_protocol_projection.rs:4350-4373 |

## Dispositions

Round 1, at head `d4f92088ca14fd09ff626c7b8a761b62dd01ae95`. The Rust change since `e1900638` is only the `#[trace]` strings and doc comments of the four tests.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | The root accepted this as a bounded technical decision, and the reviewer confirmed the author's invariants in the source. On the honest route (emit, strict admission, view), `index.projections[R]` is computed by IR `project_exact_type_inner` from the same admitted graph. That function sets `at = R.semantic_type` and `form` from that node's own tag and form, so `scalar_type/rational` at `error.at` already implies that S is the Rational scalar and that `error.at == R.semantic_type`. A non-`rational_range` R whose semantic type is the Rational scalar is still refused, because `rational_range_type` finds no four bounds and returns `None`. So each guard is defense-in-depth with no honest input that tells it apart. A negative test would need a forged `ExactTypeProjectionError`, a forged IR node, or a new test API, and the brief prohibits all three. The reviewer found no reachable admitted, mutated graph that breaks this reasoning. The PR body no longer claims a malformed-literal test. |
