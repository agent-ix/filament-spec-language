---
id: SR-20261121013119000000001
title: "Code review — QSL-2338 PR #3238 clause source_unit emission and interval spelling"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3238 diff: Cargo.toml, Cargo.lock, fsl-package/src/emit.rs, fsl-package/src/emit/monitor_plan.rs, fsl-package/src/emit/tests/{plan_subject_selection,retained_rows}.rs, fsl-semantics/src/check/lowering.rs, fsl-analyze/tests/it/monitor_root.rs, tests/it/clock_reach.rs"
review_set: subset
evaluated_revision: "ef3b3b31689aadc49e33d4fa9824cb560a440c8e"
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---

## Summary

Ticket: QSL-2338. This review covers PR #3238 at head `ef3b3b31`. It was read from the diff, with context from `git show` of the head, of FSpec at `6ad873d5d8f2` (FR-322, FR-778), of contract-ir at `7b76c8be2a24` (`temporal_binding.rs`, `temporal.rs`) and of filament-exact at `b26a8e4c5925` (`Rational` Display). Nothing was built or run. The diff has no Python and no logic-bearing shell.

## Verdict

**FAIL — one fix round needed.** Three mediums. No high: every identity spelling matches FSpec FR-322 "Formula intervals", every clause row names its declaring unit, the pin moves are exactly the two requested, and no test was bent.

- **Pins.** `Cargo.toml` moves only `filament-specification(-qualification)` to `6ad873d5` and `filament-contract-model` to `7b76c8be`. `Cargo.lock` changes only those two `source` lines. Both revs are on their repos' `origin/main` and are descendants of the old pins.
- **`source_unit` on clause rows.** The value is `source_artifact(region.source())`, where `region` is the region of the clause's own claim occurrence (`graph.occurrence(temporal.identity(), temporal.claim())`). That is the unit that writes the clause, not the unit of its subject. So a clause over a `Reference` to another unit's selected model names its own unit, as FR-322 and FR-778-AC-8 require. `source_artifact` is the same `(authority, identity)` conversion the emitter uses to build `lock.sources` (`emit.rs:7022`), so the identity form matches the lock. The trace-subject timed rows also get the value, which is right because they too are keyed by a `temporal_clause` node.
- **Await, compensation and hyper rows carry `None`.** FR-322 "Clause source unit" says a `source_unit` on a compensation, await or `hyperproperty` row refuses `invalid_package`/`timed-node-reference`. The reader at `7b76c8be` does the same: `temporal_binding.rs:687-707` admits `(None, false)` and refuses every other pairing for a row that is not a clause. On where a hyper clause's timestamp units resolve: the spec at this pin does not say. FR-322 and FR-778 resolve unit names only from a `temporal_clause` row's `source_unit`. The reader calls `validate_source_clock_unit` only for clause nodes that have a duration window (`temporal.rs:1609-1616`). A hyper body wrapper has no rows of its own, and the hyper rows' clocks are derived from the model or bound by the caller. So nothing resolves a hyper row's `timestamp_unit` in any unit. That is an open question for FSpec, not a defect in this PR.
- **Interval spelling (`lowering.rs`).** I checked each form character by character against FR-322 "Formula intervals" and FR-322-AC-509:
  - `steps:<lo>..<hi>` and `steps:<lo>..inf` are correct.
  - An unbounded operator (`interval: None`) now writes no operand. The spec says such an operator "contribute[s] no interval operand".
  - The timed form is `time:` + `[`/`(` + `<lo>..<hi>` + `]`/`)`. For a missing upper bound, `TimedInterval::unbounded` sets `upper_closed = false`, so the text ends `inf)`.
  - Bounds use the exact decimal when it terminates. `21/2` gives `10.5`, `1/4` gives `0.25` (zero padded), `0` gives `0` and `3` gives `3`. The reduced denominator and `places = max(twos, fives)` mean there is never a trailing zero.
  - A non-terminating bound falls back to `Rational` Display, `n/d`, which is already reduced. So `(1/3, 1]` gives `time:(1/3..1]`.
  - This is the only place in FSL that spells an interval into an identity. Claims, probabilistic claims and hybrid nodes take the formula node's id, so they follow the new text. The FR-498-AC-7 tests in `emit/tests/probabilistic.rs` only compare identities with each other and are unaffected. `braking2_restored_nodes_match_fspec_vectors` compares against FSpec's `hybrid-node-vectors.json`, which holds `steps:0..10`, `time:[0..10.5]` and `steps:0..2`, so the terminating forms are pinned by the standard. The contract-ir reader at this pin does not recompute the interval text.
- **`monitor_root.rs` and the `monitor_plan` rekey.** The new `member_type: None` and `collections: Vec::new()`, with `type_node`/`value_type` wrapped in `Some`, match `fsl-package/src/emit/identity_rows.rs:311-314` exactly. The rekey patterns ignore `source_unit: _`. That is correct, because it is a source reference and not a node id, so there is nothing to rekey.
- **`clock_reach.rs` re-ignores.** The refusal is real product behaviour on main. `fsl-replay/src/spine/clause/temporal.rs:1933-1947` refuses every sampled trace that is not a closed finite event-position trace with `site.window_aggregate(trace.kind())`, which gives `unsupported-window-aggregate` at the `Admit` stage (`trace_refusal.rs:158-169`). The ignore text describes that accurately. But the ticket it cites is closed (FND-003).

## Scope and skill provenance

The code-review lane and the rust-review lane ran by reading the diff. `quoin gap-analysis` was not run, because the brief allows no worktree and no build. A manual check of the ACs against tests was done instead:
- FR-322-AC-503 and FR-322-AC-509: examined.
- FR-778-AC-8 and AC-10: examined.
- FR-194-AC-1, AC-2 and AC-13, through the two ignored tests: examined.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No running test pins the value of the emitted clause `source_unit`. The reader checks only that it is a lock source, so in a one-unit package any unit passes. The case that matters, a clause over a selected model from another unit (FR-778-AC-8, FR-322-AC-503), was exercised only by the two `clock_reach` tests, which stay ignored. A regression to the subject's unit would ship green. | fsl-package/src/emit.rs:9119 |
| FND-002 | medium | The new spelling branches that FSpec's vectors do not cover have no test: `steps:<lo>..inf`, an open lower `(`, the `inf)` upper, and the non-terminating `n/d` fallback in `exact_decimal_or_ratio` (FR-322's own example `(1/3, 1]` → `time:(1/3..1]`). Each of these changes node ids that the standard fixes. | fsl-semantics/src/check/lowering.rs:7749-7820 |
| FND-003 | medium | Both re-ignores cite QSL-2138, which is Done, so the `unsupported-window-aggregate` gap they name has no open ticket. QSL-2338's own Done-when ("the QSL-2138 p95 run test passes") and QSL-2138's corrected acceptance ("un-ignore the p95 test") are not met, yet both tickets are Done. | tests/it/clock_reach.rs:132,197 |

## Findings detail

- **FND-001.** **Fix:** add an emit test that compiles a two-unit package, with a clause in `example` over a `Reference` to a model declared in another unit and selected from it (the shape of `positive-all-families`), and assert that the clause row's `source_unit` is `example`'s lock source and not the model's unit.
- **FND-002.** **Fix:** add a `lowering.rs` unit test that asserts `steps:0..inf`, `time:[0..10.5]`, `time:(1/3..1]`, `time:[1/4..inf)` → `time:[0.25..inf)`, and no operand for an unbounded `always`.
- **FND-003.** **Fix:** have the lead file (or reopen) a ticket for window aggregates over a timestamped sample trace in `fsl-replay`, cite it in both `#[ignore]` reasons, and leave QSL-2338 open until that ticket is linked as the blocker of its Done-when.
