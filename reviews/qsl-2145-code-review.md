---
id: SR-2026111612172400000044
title: "Code and Rust review of QSL-2145 partial delivery, cover order and completeness exception (filament-spec-language PR 2936)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@2d7c94ff5d32b0c582758827aee1bfc0d957a74d; git diff origin/main...HEAD (PR 2936, branch qsl-2145-observation-partials, 8 files): Cargo.lock, fsl-eval/Cargo.toml, fsl-eval/src/lib.rs, fsl-eval/src/completeness.rs, fsl-eval/src/temporal/finite_window/cover.rs, fsl-eval/src/temporal/ledger.rs, fsl-eval/tests/it/observation_owner.rs, fsl-analyze/tests/it/observation_mapping.rs"
review_set: subset
date: 2026-11-18
---

## Summary

Ticket: QSL-2145 (partial delivery; the ticket returns to Backlog with the
remaining scope). The PR adds `filament-observation` v0.1.1 to `fsl-eval`,
rewrites `ClockCover::within_bound` and `ClockCover::definitely_before` to
read the owner's `partial::EventTimeInterval::relation_to` on FR-944's
half-tick intervals, moves the ledger's `Complete { through }` revision
comparison into `fsl_eval::completeness::revise_complete_through` (FR-861's
QSL-2327 exception), un-ignores TC-2624's FR-944-AC-1 test and adds a
TC-2442 test for the exception function. Rust lane (rust-review) folded in.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-2624's FR-944-AC-1 test is now a copy compared with a copy: its `cover` helper and its `Containment` arms restate `half_tick_interval` and `within_bound` exactly, and `ClockCover` now answers through the same `relation_to`, so the test compares the owner with itself. A wrong mapping or a wrong `Containment` arm made in both places still passes. The only independent oracle for the two rewritten methods is TC-1295's 15 FSpec vectors, which use Counter only | fsl-analyze/tests/it/observation_mapping.rs:14-36,53-68; fsl-eval/src/temporal/finite_window/cover.rs:181-236 |
| FND-002 | medium | The new `Bounded` arm of `half_tick_interval` (`[2(k-u), 2(k+u)]`) is not in FR-944's mapping, which states only Counter and Exact, and no test runs `within_bound` or `definitely_before` under `ClockCover::Bounded`. TC-2624 loops over Exact and Counter{1}; TC-1295 and the fsl-replay vector test use Counter only. Hand derivation shows the arm agrees with the old closed-extent arithmetic, but nothing asserts it | fsl-eval/src/temporal/finite_window/cover.rs:189-192 |
| FND-003 | medium | The two rewritten methods have no non-test caller (only fsl-eval tests/it/finite_window.rs, fsl-replay's temporal tests and TC-2624 call them). The evaluator's verdict paths still decide order with local `Extent` arithmetic for point stamps as well as interval stamps: `Extent::eligible` (run.rs:936), `Extent::overlaps` (run.rs:1017) and `order_span`/`trace.rs:275`. The PR title says fsl-eval takes cover order from the owner, and "Not delivered" item 1 blames only FR-966 open interval endpoints, so the record overstates the switch | fsl-eval/src/temporal/finite_window/run.rs:928-940,1016-1018; fsl-eval/src/temporal/finite_window/cover.rs:168-174,316-345 |
| FND-004 | low | FR-944 says the half-tick interval's domain is `EventTimeInterval::new(clock, profile, unit, ...)` with "the unit identity the clock unit with the suffix `/2`". The code uses the literals `"clock"`, `"cover"`, `"tick/2"`, and the test uses different literals again (`"clock:counter"`, `"profile:finite-window"`). The intervals never leave the function, so no answer changes, but the code departs from the SHALL text without saying so | fsl-eval/src/temporal/finite_window/cover.rs:198-205 |
| FND-005 | low | Each `within_bound` / `definitely_before` call now builds 6 heap `Identity` strings and formats every endpoint with `i128::to_string` about 8 times (`validate_limits` runs in each `new` and twice in `relation_to`), where the old code did a few integer compares. This is harmless while the only callers are tests, but it becomes per-pair cost once `eligible`/`overlaps` move onto this path (FND-003) | fsl-eval/src/temporal/finite_window/cover.rs:181-236 |

## Scope and evidence

- Half-tick mapping against FR-944 "Partial values and interval order" and the
  owner at filament-observation v0.1.1 (8399e97, `src/authority/partial.rs`:
  `before = l.earliest < r.latest`, `equal = overlap`, `after = l.latest >
  r.earliest`, `definitely_before = before && !equal && !after`). Derived by
  hand for Counter `[2(k-u), 2(k+1+u)-1]`:
  `True` iff `a-r+1+2u <= b`, `False` iff `a-r-1-2u >= b`,
  `definitely_before` iff `a+1+2u <= b`. These are the same as the old
  `Difference::at_most` on `[k-u, k+1+u)` and `Extent::before`. Exact and
  Bounded (closed) also agree with the old arithmetic. Negative bounds hold
  under the same derivation.
- The `expect`s on `EventTimeInterval::new`/`relation_to` cannot be reached:
  `u >= 0` keeps the endpoints ordered, `Limits::owner_max().max_string_bytes`
  is 1 MiB against at most about 22 bytes per endpoint, the identities are
  non-empty literals, and both intervals share one domain.
- Extent / eligible / overlaps / order_span: unchanged by the diff. `Extent`,
  `Difference::at_most/at_least`, `Extent::before` and `ClockCover::extent`
  keep their callers (run.rs, trace.rs), so their results do not change.
- Ledger refactor: `revise_complete_through` keeps the old semantics exactly.
  A retained `Incomplete` gives `Ok`; a retreat (asserted `<` through, or
  `Incomplete`) reports `ProgressValue::Completeness(asserted)`; the first
  position with `stamp <= through` that is not in `covered` reports
  `ProgressValue::Position`. The retained value and the dimension are
  unchanged, and the order of the checks is preserved.
- The doc comment of `fsl_eval::completeness::revise_complete_through` and
  the module doc cite QSL-2327, as FR-861's tracked exception requires.
- Pin: fsl-eval's `tag = "v0.1.1"` matches fsl-analyze's. Cargo.lock resolves
  one filament-observation (8399e97). v0.1.1 has no `completeness::revise`;
  origin/main has it from #61 (2207f7c), so the PR body's claim that it is
  on main but in no tag is correct.
- No `#[allow]` added. `unwrap`/`expect` appear only in tests and in the
  unreachable cases above.
- Test tags: `#[trace("TC-2624", "FR-944-AC-1")]` and `#[trace("TC-2442",
  "FR-861-AC-4")]` use the ix-trace-rs attribute and name existing TC/AC
  pairs. For how much of AC-4 the TC-2442 test covers, see the companion gap
  analysis.
- Agreement test grid: readings a, b in 0..=6 with bound in 0..=6 under Exact
  and Counter{1}, which is all 49 pairs times 7 bounds, as FR-944-AC-1
  states. The pairs are complete; what the test is worth is FND-001.
- PR body and ticket state: the body says partial delivery and that the
  ticket stays open. The ticket is In Progress with one W12 comment. Apart
  from FND-003, the body's delivered and not-delivered lists match the diff.
- Gates (fresh review worktree, CARGO_TARGET_DIR under /home, cargo-slot,
  focused, `--test-threads=1` with the house skips):
  - `cargo clippy -p fsl-eval --all-targets --no-deps --keep-going -- -D
    warnings` is clean.
  - `cargo clippy -p fsl-eval -p fsl-analyze --all-targets --no-deps -- -D
    warnings` fails only on known issues that predate this PR: the fsl-command
    `wire.rs:458` dead `document` field (a warning) and
    `fsl-analyze/tests/it/analyze_package_selection.rs:711` (for loop over a
    single element). Nothing points at a file in the diff.
  - Without `--no-deps` the run stops first on the known fsl-replay lints
    (`result/wire.rs:831`, `witness/derivation.rs:94`).
  - Tests: fsl-eval lib `temporal::ledger` (10 passed). fsl-eval it
    `observation_owner` (1 passed; 1 ignored, the source-scan test). fsl-eval
    it `finite_window` (35 passed, including TC-1295's FSpec vector test with
    FSPEC_DIR set). fsl-analyze it `observation_mapping` (1 passed).

## Verdict

The half-tick mapping is correct for Exact, Counter and Bounded, and the
ledger's behaviour is unchanged. The Extent, eligible and overlaps paths are
untouched. The weaknesses are evidence and record defects: the AC-1 test can
no longer catch a mapping error (FND-001), the Bounded arm is untested
(FND-002), and the PR body overstates what moved to the owner (FND-003). As
a partial delivery it can merge once FND-002 is covered by a Bounded case
and FND-001/FND-003 are fixed or deferred to the remaining QSL-2145 scope.
