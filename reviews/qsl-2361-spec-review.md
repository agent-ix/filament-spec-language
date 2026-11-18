---
id: SR-2026111612172400000026
title: "spec-review of PR2834 QSL-2361 FR-495-AC-24, AC-25, TC-2873, TC-2874"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md (AC-24, AC-25), spec/test-cases/TC-2873-equality-over-a-mod-application-emits.md, spec/test-cases/TC-2874-if-over-integer-arithmetic-emits.md; PR2834 0dc06c81bbf1b415668602fe8cf1c484eaea1e03"
review_set: subset
---

## Summary

Ticket: QSL-2361. This is a base spec-review (EARS and integrity checks) of the two new FR-495 acceptance criteria and their two test cases. `quire validate --scope .` passes on all three files. Ids TC-2873 and TC-2874 and FR-495-AC-24 and AC-25 are unused on origin/main, and no other open PR claims them. Each TC verifies FR-495, and its Description names its AC. AC-25 is accurate and testable. AC-24 contains one source its own source-type clause cannot apply to, and the Expected Results of TC-2873 are weaker than AC-24.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-495-AC-24 lists `(n mod a) + 1 = 1` as "an equality over an integer division application", but that equality's operand is a `+` application, so the clause "the equality decision's source type for that operand being … the builtin `Integer` node" has no defined operand for this source | spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md:436 |
| FND-002 | low | TC-2873 Expected Results step 3 ("an operand's `source_type` is the `mod` application's own `semantic_type`") states less than FR-495-AC-24. It omits the builtin `Integer` node, the "never the checked interval" clause, and the div/rem sources | spec/test-cases/TC-2873-equality-over-a-mod-application-emits.md:25 |

## Scope and evidence

- **FR-495-AC-24** was examined. Every listed source was confirmed to compile, emit and read back at head. The mod operand's emitted `source_type` was confirmed to be `scalar_type/integer`.
- **FR-495-AC-25** was examined. The hull maxima 1999, 1001, 2000 and 1001 follow from `Int[0,999]`, from `Int[0,1000]`, and from `versionNumber`'s published `min 0` / `max 1000` constraints in examples/config-version. The parenthetical "a control node names no interval node that no edge or occurrence places" matches the lowering at fsl-semantics/src/check/lowering.rs:4357-4380.
- **TC-2874** was examined. Its procedure and expected results match the test exactly.
- **TC-2873** was examined: see FND-002.
- FR-495-AC-21 and AC-23 were read as context. They establish that a division application and an `if` over one lower at `Integer`. The new ACs are consistent with them.

## Verdict

The spec additions are valid and consistent with the implementation. Both findings are wording fixes and do not block merge.

## Dispositions

Round 1 was checked against head 2c8a731816f5f3ff59c934ce6afd72510315e209: fix commit 2c8a73181. `quire validate` was re-run on the FR and the TC.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | In FR-495-AC-24, the source `(n mod a) + 1 = 1` is replaced by `n mod a = 1` over `n: Int[0, 100]`, `a: Int[1, 3]`. Every listed source now has a division operand. |
| FND-002 | fixed | TC-2873 step 3 now reads: "one equality decision per source; the division application's `semantic_type` is the builtin `Integer` node, and the decision at that operand's position has `source_type` equal to that node, for all eight sources." |
