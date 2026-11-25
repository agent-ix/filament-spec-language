---
id: SR-967175026302197768900474181941935084833
title: "QSL-3130 Rust code review"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/filament-spec-language; QSL-3130; f5d303a0e7a7f7d61bf473aeaccb0258c0caaae1..51512a82ac24f2426019dc0922bfa54021d76c0b; bounded timed formula monitor changes"
---

## Summary

Ticket QSL-3130, frozen head `51512a82ac24f2426019dc0922bfa54021d76c0b`; the QSL-3203 base commit is excluded. Rust code-review and Rust-review covered the analyzer/replay monitor, key validation, tests and touched call sites. The bounded-window examples are present and trace-tagged; review found duplicated monitor logic and resource limits enforced after large representations have already been built.

## Verdict

**FAIL.** Resolve the high duplication finding and the resource-bound finding before merge.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The same timed formula monitor is implemented independently in analyzer and replay: AST specs, residual progression, temporal semantics, past candidates and canonical key serialization. Replay key parsing then maintains a third copy of the canonical residual grammar. These owners can drift; provide one shared source for monitor semantics/key representation while retaining independent replay validation where needed. | fsl-analyze/src/exact_probabilistic/formula_monitor.rs:1; fsl-replay/src/probability_certificate/document/formula_monitor.rs:1; fsl-replay/src/probability_certificate/document/key.rs:637 | correct-requirement-no-evidence |
| FND-002 | medium | State/storage limits are checked after potentially large intermediate representations are allocated. Replay evaluates/progresses before `check_limits`, then builds `self.key()` and serializes it fully before comparing bytes. Analyzer checks monitor cells in `key_of` after observation and constructs `certificate_key` before checking encoded bytes. Enforce budgets incrementally before adding residual/candidate cells or encoding key bytes; add tiny-budget tests proving refusal at the crossing boundary. | fsl-replay/src/probability_certificate/document/formula_monitor.rs:141; fsl-replay/src/probability_certificate/document/formula_monitor.rs:205; fsl-analyze/src/exact_probabilistic/formula_monitor.rs:155; fsl-analyze/src/exact_probabilistic/digital.rs:1339 | correct-requirement-no-evidence |

## Coverage

- Examined FR-204-AC-21: “Over the timed `Retx` subject of AC-1, under the timed profile EN-5 runs every bounded-window formula that FR-186 admits and retains a monitor for every bounded operator occurrence, including nested occurrences and their nonzero lower bounds.”
- Examined FR-204-AC-1 as the fixture/law context; examined FR-186-AC-7 for timed-profile bounded-formula admission and refusal behavior; examined FR-123-AC-3 for interval-operator profile context.
- Tests: `fsl-analyze/tests/it/timed_product_certificate/bounded_monitor_profile.rs:8` and `:36` carry `FR-204-AC-21`; matrix also binds `fsl-eval/src/probabilistic.rs::shared_nested_deadline_uses_both_windows_and_latches_acceptance`.
- Checks: `git diff --check` passed. Focused tests/checks from the coder were reported green (bounded monitor 9/9; nested punctual Always 2/2; replay timed filter 5/5); not rerun by reviewer. GitHub API returned 401 and Linear timed out, so findings could not be posted.
- Applicable AssuranceProfile: none found under `spec/`.
