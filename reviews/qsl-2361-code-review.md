---
id: SR-2026111612172400000024
title: "code-review of PR2834 QSL-2361 mod equality and if-hull emission"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-semantics/src/check/lowering.rs, fsl-replay/src/application/division_tests.rs; PR2834 0dc06c81bbf1b415668602fe8cf1c484eaea1e03"
review_set: subset
---

## Summary

Ticket: QSL-2361. Code-review with the rust-review lane folded in, covering PR #2834 at head 0dc06c81b (merge-base faac49e15). The PR makes three lowering changes. (1) `Lowering::application` builds the checked-interval type node only for an `IntegerDivision` whose lowered type differs from its checked type. Before this, every application ran `type_ref(node.value_type())`, so an `if`/`let`/`case` that lowers at `Integer` left behind an orphan `bounded_domain` with no occurrence (defect B). (2) `equality_identity` names a Source operand's type at the child's lowered type. (3) The text/plan type nodes use `child.lowered_value_type()`. Together, (2) and (3) fix defect A. The root cause is correct. The diff has two low-severity comment defects and nothing else.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The comment on the application interval guard cites FR-495-AC-24, but the orphan-interval rule it describes is FR-495-AC-25 (AC-24 is the equality case) | fsl-semantics/src/check/lowering.rs:4357-4362 |
| FND-002 | low | The doc comment on test helper `bounded_domains_ending` is wrong: it describes a node "of `form` and `tag` whose semantic type's key ends with `max` followed by `maximum`", but the function counts `bounded_domain` nodes whose `node_id.path` last segment equals `maximum` | fsl-replay/src/application/division_tests.rs:435-436 |

## Scope and evidence

- **Root cause, hunk 1.** `lowered_integer_types` (fsl-semantics/src/check/ir.rs:1070) marks a node as lowered-Integer only in three cases: its type is `Integer`, it is an `IntegerDivision`, or it is an Int-typed `Local`/`If`/`Let`/`Case` over such a node. Lowered type and checked type therefore differ only for divisions and for control/let/case nodes. Every caller of `application` was checked: catalog applications (5800), let (6275), binder (6303), field query (6331), case (6455) and fold (6524). In the `lowered == value` branch the result node is unchanged, because `type_ref(node.value_type())` is still built. The only node no longer built is the checked interval of a control/let/case node that lowers at Integer. No edge, occurrence or record named that node. The narrows of FR-495-AC-21/23 build their own `type_argument` node, and division requirement records keep theirs through the retained guard. So no eval, replay, emit, monitor or CG consumer loses a node it used.
- **Hunk 2 (equality operands).** Only `Source`-role facts whose child `value_type` equals the fact's type are rewritten to the child's lowered type, so non-integer and ConversionTarget operands are unchanged. `fsl-package/src/emit.rs:3230-3300` requires each text/plan key to equal the matching operand-fact node. Hunks 2 and 3 apply the same lowered type, so that cross-check still holds.
- **Hunk 3 (text/plan).** `EqualitySchedule::Plan` covers integer operands, so this hunk is load-bearing. For non-integer children the lowered type equals the checked type, so nothing changes for them.
- **Each hunk fails a test when reverted.** Each lowering hunk was reverted alone in a scratch worktree and `cargo test -p fsl-replay --lib division_tests` was run (FSPEC_DIR set). Reverting hunk 1 fails `an_if_over_integer_arithmetic_emits_and_reads_back_without_an_orphan_interval` with `UnlocatedOccurrence` on `bounded_domain ... max`. Reverting hunk 2 alone, and reverting hunk 3 alone, each fails `an_equality_over_a_mod_application_emits_and_reads_back` with `Emit(MissingEqualityIdentityContext)`. At head, 9/9 pass.
- **Extra shapes probed** with a temporary test, reverted afterwards. These emit and read back: `(if c then n mod 2 else 0) = 0`, `(let r = n mod 2 in r) = 0`, `(n mod 7) mod 2 = 0`, `n mod 2 = m mod 2`, `n mod 2 = k` with `k: Int[0,1]`, the `if`-over-mod invariant, `(if a > 1 then n div a else 0) = 0` (Euclidean), `(if c then n else n + 1000) = 5`, and a conjunction of an if-hull and a mod equality. In each case the mod side's `source_type` is `scalar_type/integer`. Shapes refused `IllTyped(TypeMismatch)` at check: `n mod 2 = m mod 3`, `n div a = n rem a`, `n mod 2 = k` with `k: Integer` or `Int[0,5]`. These are not defects of this PR: `n = k` over `Int[0,100]` and `Integer` (or `Int[0,5]`) is refused the same way, because the language rule is that both sides of `=` must have the same type.
- **Rust-review lane.** No `unwrap`/`expect`/`panic` was added to production code; the test-only `unwrap`/`panic!` in `emit_and_read_back` is fine. No `unsafe`, integer conversion, async or lock change. The new tests carry plain `#[trace("TC-2873", "FR-495-AC-24")]` and `#[trace("TC-2874", "FR-495-AC-25")]` attributes. There is no scope creep: the diff touches only the two lowering sites, the two tests, the FR-495 AC rows and the two TCs.

## Verdict

The root cause is correct and the change is minimal. Each lowering hunk is needed and is covered by a test that fails without it. The two findings are comment fixes only, and neither blocks merge.

## Dispositions

Round 1 was checked against head 2c8a731816f5f3ff59c934ce6afd72510315e209: fix commit 2c8a73181, after merging origin/main 15faf6452.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | lowering.rs:4376 now reads `// places it (FR-495-AC-25).` |
| FND-002 | fixed | division_tests.rs:435-436 now reads "How many emitted `bounded_domain` nodes have an identity path ending in `maximum`, the interval maximum an `if`'s checked hull would carry." |
