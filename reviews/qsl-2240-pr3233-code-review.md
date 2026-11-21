---
id: SR-2026112022385700000000002
title: "Code review — QSL-2240 PR #3233 FR-418 checker DBM operations (slice S2)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3233 diff at 3aaef3851: fsl-replay/src/certificate/zone.rs, fsl-replay/src/certificate/zone/region_tests.rs"
review_set: subset
evaluated_revision: "3aaef38517238cbbbda911d6f44222ec93f4ecc2"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-245
    type: references
---

## Summary

Ticket: QSL-2240 (the branch name `qsl-2338-s2-dbm` names an unrelated ticket, QSL-2338, to which Linear also attached this PR). I reviewed `git diff origin/main...HEAD` (merge base `919afee95a`) against FSpec FR-418 at the newest checkout (FSPEC-QSL2338, which carries the arch ruling U1 text and FR-418-AC-18 to AC-21; the FSL pin `5c4384df` and FSpec `origin/main` carry neither), the clock order and refined-region paragraph of FR-418, and `/tmp/fr418-design.md` sections 2 and 3. I compared `subtract_bounded` by reading `fsl-analyze/src/zone_check/dbm.rs:354-412`.

These parts are correct:

- `subtract_bounded` (zone.rs:840-874). Error order and conditions match the engine exactly (clock range, reference clock, `amount < 0 || ceiling <= 0 || amount > ceiling`), and the zone is unchanged on every error, since all checks precede mutation. The guard `amount <= x <= ceiling` is applied non-strictly by `constrain` before the shift. The shift adds `(−a, <=)` to row `x` and `(a, <=)` to column `x`; with the `plus` encoding `e + 2c + 1 − 1`, strictness is preserved exactly. The diagonal cell is untouched. A one-variable translation keeps every shortest path consistent (`cell(i,x)+cell(x,j)` gains `+a−a`), so the result is canonical without the engine's extra `close()`; the semantics equal the engine's. An empty guard gives the canonical empty form.
- `constrain_diagonal` (zone.rs:882-896) is `constrain` with both sides checked as distinct real clocks.
- `region_normalize` (zone.rs:935-1066) builds exactly the FR-418 refined classes. Stage 1 enumerates, per clock, `{k}` and `(k, k+1)` from the zone's lower bound up to the ceiling, then `{M}` and the saturated `x > M` class, stopping at the zone's upper bound. Stage 2 splits each pair of open clocks three ways by `x_a − x_b ≺ k_a − k_b`, which is `frac(x_a) ≺ frac(x_b)`; whole clocks need no split and saturated clocks are not ordered. Stage 3 splits by each declared diagonal predicate and its exact negation (`x_r − x_l ≺' −v` with flipped strictness). Each class is a conjunction of difference constraints, so its canonical DBM is unique; inconsistent orderings are dropped because the zone part is empty. Completeness holds for non-negative valuations: every point of the zone is in the class of its integer part, its fractional order and its predicate truths, and the enumeration bounds cover it. The hand counts 18 and 19 are correct.
- `region_covers` (zone.rs:1078-1092) is exact, not merely conservative: it answers whether every class meeting `self` lies in `target`, which is "closure of `self` included in `target`" (ruling U1). It cannot say covered while a class meeting `self` is outside `target`, given stage 1-3 completeness. For a target that is not a union of classes it is still exact for the U1 question (it then answers false whenever a class is cut).
- Independence: the new code calls only zone.rs's own `constrain`, `bound`, `includes` and `universe`; nothing is imported from `fsl-analyze`.
- Exactness: all arithmetic is over `Exact`/`Integer`; the `10^40` test runs through the same code path. No `#[allow]`, no digest, hash or checksum, no `unwrap` in production code.

Tests run: `cargo-slot cargo test -p fsl-replay --lib certificate::zone` (name-filtered, two skips) at 3aaef3851: 23 passed, 0 failed, all 13 new region_tests among them.

## Verdict

**FAIL — fix round needed.** One high (FND-001: every test is tagged to an AC it does not test), two mediums (FND-002 test oracle cannot see the fractional-order constant; FND-003 no bounded or short-circuit path for coverage), three lows. The operations themselves are correct as read; no wrong-result defect was found in production code.

## Rust review

| Check | Result |
| --- | --- |
| Behaviour vs spec | Matches FR-418 refined-region relation and ruling U1; `subtract_bounded` equals the engine semantics. |
| Tests | Hand vectors are real oracles. The subtraction grid property is a real pre-image oracle (`reference::zone_contains` reads raw cells, no closure). The region property tests are partition-generic (FND-002). |
| Trace tags | Wrong AC (FND-001). |
| Idioms | `ExtrapolationLength` reused for a region-ceiling length error; acceptable but its field name `max` reads oddly. No unsafe, no concurrency, no panics on input. |
| Kani | No harness for the new operations although the module doc says the module is Kani-checked (FND-005). |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | All 12 non-exactness tests are tagged `FR-418-AC-13`, which is about task-transition templates, capture-avoiding renaming, growing activation occurrence, rank/slot refusal and concrete replay; none of that is exercised. They are the only AC-13 tags in the repo, so the matrix would show AC-13 covered by tests that cannot fail on its content | fsl-replay/src/certificate/zone/region_tests.rs:87, 99, 116, 139, 162, 211, 243, 271, 301, 322, 354, 431 |
| FND-002 | medium | The region property test reads grid points at scale 2, where every non-integer fractional part is 0.5, so no sampled point has two distinct fractional parts; its disjointness, completeness, idempotence and covers-is-inclusion checks hold for any partition by any hyperplane. A wrong fractional-order constant (for example `gap = base_b - base_a`) passes every test, because the only hand vectors with two open clocks have equal bases | fsl-replay/src/certificate/zone.rs:1033; fsl-replay/src/certificate/zone/region_tests.rs:357, 349-433 |
| FND-003 | medium | `region_covers` materializes every class meeting the successor before testing any, with no early exit, no fast path for a target already known to be a union of classes (where coverage is plain `includes`), and no work meter or cancel hook; the count is exponential (ceilings^n times n! orderings), and the design's risk case is about 15 clocks with ceilings up to 10, so S4 cannot charge `max_check_work` (FR-418-AC-20) inside it | fsl-replay/src/certificate/zone.rs:1078-1092, 935-1066 |
| FND-004 | low | `region_normalize` starts each clock at `max(lower bound, 0)` and silently drops valuations with a negative clock, so a zone read by `from_matrix` without `0 - x <= 0` rows normalizes to classes that do not cover it and `region_covers` can report covered for it; the non-negativity precondition is not stated | fsl-replay/src/certificate/zone.rs:974-977, 1068-1077 |
| FND-005 | low | The module doc says the zone code is Kani-checked for overflow freedom and reference agreement, but no harness covers `subtract_bounded`, `constrain_diagonal` or the region operations | fsl-replay/src/certificate/zone.rs:9-11; fsl-replay/src/certificate/zone/proofs.rs |
| FND-006 | low | A think-aloud comment ("y > ... no, x < y + 1 <= 7") sits above an assertion that the bound is unchanged at 6, so the first `constrain_diagonal` case asserts no tightening although its comment says closure tightens | fsl-replay/src/certificate/zone/region_tests.rs:217-218 |

## Findings detail

- **FND-001.** FR-418-AC-13: "every task-scheduler edge and bounded-subtraction row uses a finite `TaskTransitionTemplate` ... the reader closes the source-plus-scheduler graph modulo capture-avoiding renaming ... refuses a template with an out-of-range or mismatched live-job rank/slot. Concrete replay instantiates the template ...". The DBM operations touch none of this. The right AC for `region_covers` tests is FR-418-AC-18 (inclusion after region closure), which exists only on the unmerged FSpec branch, not at the FSL pin `5c4384df`. At the pin, the refined-region relation and the subtraction guard (FR-418 behaviour text "proves that every reachable subtraction of `c` from `x` is guarded by `c <= x <= M_x`") have no AC. Fix: drop the AC-13 tags; tag the coverage tests AC-18 when the pin carries it, and give the remaining unit tests an AC that names the refined-region relation and the bounded subtraction (FSpec ticket) or tag them TC-only if the trace macro allows. The `FR-418-AC-4` tag on the `10^40` test is a correct, narrow binding.
- **FND-002.** Failure scenario: a coder changes zone.rs:1033 to `base_b.minus(base_a)`. For open clocks with bases 1 and 0, the "<" piece becomes `x_a − x_b < −1`, which is not a class; every test still passes (hand vectors use bases 0 and 0; the grid has all fractions equal; each piece re-normalizes to itself). Fix: read the grid at a scale with distinct fractions (for example 4 or 6) and add a point-wise class oracle: two sampled points share a returned region iff they have equal integer parts up to the ceiling, equal zero-fraction flags, equal fractional order and equal predicate truths; plus one hand vector with unequal bases (for example `x in (1,2)`, `y in (0,1)`, ceilings 2).
- **FND-003.** Failure scenario: an S4 successor zone after `up()` over 15 clocks with ceilings 10 makes `region_normalize` allocate on the order of 21^15 classes before the first `includes`, with no way to stop for `max_check_work`. Fix: return on the first class not included (an iterator or a callback), accept a work budget, and offer `includes` as the fast path when the target is already established as a union of classes.

## Findings data

- FND-001 check: trace; confidence high.
- FND-002 check: test-intent; confidence high.
- FND-003 check: other; confidence medium.
- FND-004 check: soundness; confidence medium.
- FND-005 check: coverage; confidence high.
- FND-006 check: test-intent; confidence high.

## New findings (disposition pass 1)

Re-checked at fix commit `134c0e6133`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | Twelve unit tests are now tagged `FR-418-AC-18`. That AC is not in the FSL spec pin `5c4384df` (it is on FSpec `origin/main` only), and none of the tests checks what AC-18 observes: a task-product certificate accepted, or settling `inconclusive` with `successor-missing` at the source node. Six of them do not touch coverage at all: the hand counts, the refusal of malformed arguments, the negative-clock refusal and the fraction-order vectors. So AC-18 would read covered with no certificate-level test, the same inflation FND-001 named for AC-13 | fsl-replay/src/certificate/zone/region_tests.rs:243, 271, 301, 320, 352, 424, 484, 553, 598, 625, 662, 697 |

- **FND-007 detail.** Fix: keep these unit tests TC-only (as the subtraction tests now are) until S4 adds the certificate-level AC-18 test, or tag AC-18 only on the coverage tests and only after the FSL pin carries AC-18. The certificate-level test is the one that should carry AC-18.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | No `FR-418-AC-13` tag remains. The subtraction and diagonal tests are `#[trace("TC-371")]` and the 10^40 test keeps `FR-418-AC-4`. The AC-18 retag that replaced AC-13 is FND-007 |
| FND-002 | fixed | `regions_are_the_definitions_classes_on_a_grid_of_quarters` (region_tests.rs:555) samples at scale 4. For each point it computes a key straight from the definition: integer part, zero-fraction flag, saturation, signed fraction order, predicate truth. It then asserts a bijection between keys and returned regions. That is a real oracle and does not reuse the code under test, and it fails on a flipped `gap`. `fraction_order_splits_boxes_with_different_integer_parts` (:486) adds the hand vector with unequal bases |
| FND-003 | fixed | `region_covers` walks the regions depth first and stops at the first region not included. `charge` is called before every narrowing step; `Spent` becomes `Flow::Exhausted`, then `Coverage::Exhausted`. Only `Flow::Continue` maps to `Covered`, so a spent budget is never read as covered. `TargetShape::UnionOfRegions` answers by `includes`, after one charge. Its obligation is documented at zone.rs:996-1002 |
| FND-004 | fixed | `check_regions` refuses a non-empty zone whose `0 - x_k` cell is looser than `(0, <=)` with `NegativeClockValue` (zone.rs:1072-1080). The precondition is in the doc, and a test covers it |
| FND-005 | fixed | The module doc now says the task-product operations are not Kani-harnessed and names their tests |
| FND-006 | fixed | The case now uses `y <= 4` and asserts the tightened `x < 5` |
