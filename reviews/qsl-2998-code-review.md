---
id: SR-3608001
title: "code-review of PR 3608 cancelled branches and predicate joins in replay enabledness"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/src/protocol_analysis/recording.rs, fsl-analyze/tests/it/protocol_replay_mode.rs at 115a613f78ca072982125b5cb8e38c801c453d70"
review_set: subset
---

## Summary

Ticket: QSL-2998. Rust review folded in. Titan focused runs: `--test it` filter 93 passed, 0 failed, 0 ignored; `--lib a_refused_prefix`
2 passed. Probes ran as a scratch test in the coder's titan worktree and were reverted (tree clean after).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Rule 20 equates "the other branches finished causally before the step" with "after the cancel", but the cancel happens at the join step, which ST-7 enables and does not force: under `join any ... outstanding cancel` the closed recording `L`, `R`, `Placed` (Buyer order; `R` steps after `L` finished but before the join fires) gives `R` `violated`, `unexpected` (probe P1; same with `predicate(left)`). A legitimate interleaving is refuted. Only a step that needs the join (a step after the `parallel`) causally before the branch step proves the cancel came first | fsl-analyze/src/protocol_analysis/recording.rs:4168-4195 |
| FND-002 | medium | Predicate tables are bounded by silently truncating: `bits = joined.len().min(12)` while `join_reached` builds its mask over every joined label, so 13 or more labels give a mask past the table (or the wrong arity to the function) and the join is `Unevaluable` forever with no refusal; at 64 or more labels `usize::from(finished) << bit` overflows (panic in debug). Up to 4096 evaluations per predicate join, each on a fresh `recording_meter()`, are spent before any limit is charged, and the tables are rebuilt for every prefix of an incremental assessment | fsl-analyze/src/protocol_analysis/recording.rs:4146-4156 |
| FND-003 | medium | On the package route `BarePackage::evaluate_join_predicate` returns `SOURCE_FORM`, so every predicate join is `Unevaluable` and its step stays `pending` whatever the closure: a closed, conforming recording can never settle `proved` and reports `inconclusive`/`open-history`, a cause that names the recording, not the engine's missing capability. No test covers this path | fsl-analyze/src/protocol_analysis/recording.rs:4202-4240 |

## Verdict

Not mergeable (FND-001). The predicate evaluation follows FR-218: labels sorted, Boolean `true` for a completed branch, and cancelled
branches never read; the policies `all`, `any` and `quorum(n)` are counted over branches finished causally before the step, which is
stable across prefixes, and the tests assert prefix sequences with `assert_every_verdict_only_advances` and last prefix equal to the
item. Nested parallels work through the ancestry windows. The cancel rule is wrong for steps taken after a branch finished but before
the join step (FND-001), and the predicate table has no honest bound or package-route outcome (FND-002, FND-003).

## Dispositions (fix round 1)

| Finding | Disposition |
| --- | --- |
| FND-001 (cancel refutes a legitimate interleaving) | Fixed. `after_cancel` now flags a branch step only when an observation of a step after the `parallel` (a later sibling in an enclosing sequence, which needs the join), or of `finish`, in the same instance causally precedes it. `L, R, Placed` leaves `R` satisfied; `L, Placed, R` violated; an `R` of another role unordered with `Placed` is satisfied; the same holds for `predicate(left_only)`; a nested `parallel` cancels at its own join. Tests `a_branch_step_is_after_the_cancel_only_once_a_step_after_the_parallel_precedes_it` and `a_nested_parallel_cancels_at_its_own_join` (FR-895-AC-32, AC-33); the cancel boundary (policy holds, no later step observed) is pinned in AC-32. |
| FND-002 (predicate table bound) | Fixed. A predicate join names at most 10 labels (`MAX_PREDICATE_JOIN_LABELS`); more is refused before any evaluation, `unsupported_construct`/`not-yet-implemented`, naming the join and the bound. The mask is bounded, so no shift overflow. The tables are built once per recording read, shared by every prefix. Tests at 10 (assessed), 11 and 64 (refused) in a library call and on the wire, batch and incremental (AC-34). |
| FND-003 (package route) | The package route admits only sequence, choice, send, receive and check (`CheckedProtocol::package`); a protocol with a `parallel` never reaches the assessment and is refused at selection `unsupported_construct`/`not-yet-implemented`. Test `a_predicate_join_over_a_package_settles_unsupported` (AC-36) pins it; the assessment's own package arm for a predicate join is a typed refusal and is unreachable today. |
| Never-true predicate | Test with `left and not left` (a name `never` is a reserved word): `Placed` pending in open prefixes, violated closed (AC-35). |
| Failed predicate evaluation | Refused `unsupported` (rule 25); no fixture can reach it, since a function over Boolean arguments always evaluates. |
