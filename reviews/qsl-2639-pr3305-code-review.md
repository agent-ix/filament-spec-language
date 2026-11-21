---
id: SR-202611210022442313144108523
title: "code-review of QSL-2639 PR3305 exact coupled reset CAD"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2639; six Rust diff files and FR-450/FR-447"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-450
---

## Summary

Dedicated independent code-review of the frozen CAD implementation for PR3305. 2 concrete finding(s); no mathematical wrong-result counterexample established.

## Verdict

**FAIL** — unpaid quadratic projection construction violates caller WorkUnits containment.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | PSC matrix construction charges only O(s * sparse operand size) before allocating s squared Polynomial cells; the matrix-weighted determinant charge is too late to bound that allocation. | fsl-replay/src/hybrid_replay/exact/cad.rs:694 |
| FND-002 | medium | Root separator arithmetic omits rational endpoint operand weights: high.sub(low) executes before its local charge, candidate construction charges only polynomial and trial denominator size, and sector midpoint/end arithmetic has no endpoint-weighted charge. | fsl-replay/src/hybrid_replay/exact/cad.rs:462 |

## Evidence

FND-001 scenario/oracle: sparse a=x^D and b=D*x^(D-1), produced after projecting coupled x^D+y=0, have constant sparse term counts. At order 0, s=2D-1. D=4096 yields 67,092,481 cells; the precharge grows only linearly in D. A finite budget can therefore admit a large allocation (or allocator/process failure) before determinant observes exhaustion. Expected: charge enough for each unpaid row/cell before materializing it, with saturating operand-weighted accounting, and return typed ResourceExhausted before denied construction; do not add a degree cutoff. Static loop cardinalities establish the defect; no large-allocation probe was run.

FND-002 scenario/oracle: narrow rational isolating endpoints (e.g. low=1/2^N, high=2/2^N, for large N) require N-bit rational subtraction/division/addition, but the first candidate prepays the same p.size()+one-small-word-square as small endpoints. This also occurs during refinements separating close roots. With the meter exhausted at separator entry, width subtraction still happens before denial. Sector samples likewise compute endpoint +/-1 or midpoint without an endpoint-weighted charge. Expected: precharge those rational operands before every separator/refinement/sample operation; denied work performs no arithmetic.

Projection inspection matched the full positive-degree reducta, coefficient carry, derivative PSC and pairwise PSC families; constants are discarded only from root-boundary sets while signed time coefficient decisions are retained. All reachable decision branches are collected before existential witness search. Numeric conditionals, negative denominators, original division holes, short-circuit Boolean evaluation and Undefined leaves were traced against eval.rs. Actual trace undefinedness uses the existing replay path and preserves cause/locus.

Triangular lifting was inspected as an induction on prefix length: rational substitution or a one-root-interval Tarski query reduces signs to a shorter prefix. Every PRS remainder is specialized/trimmed before subsequent degree and parity decisions; primitive scaling stays positive rational. Root equality uses a zero Tarski query plus interval containment; sections and rational sectors include repeated-root/degree-drop/nullified fibers. No quotient-field assumption or degree/variable cutoff was found. This is concrete code review, not a formal verification claim.

Primary mathematics checked: [Collins, Theorem 5](https://link.springer.com/content/pdf/10.1007/3-540-07407-4_17.pdf) supports projection delineability; [Li/Passmore/Paulson, Theorem 1](https://arxiv.org/pdf/1506.08238) supports real-coefficient Sturm-Tarski queries; [Li/Paulson section 3.2](https://www.cl.cam.ac.uk/~lp15/papers/Formath/LiRealAlgebraicNumbers.pdf) motivates specialization and signed pseudo-remainder handling. Extending the reduction through an arbitrary triangular prefix is an implementation induction assessed here, not a theorem these papers already prove about this Rust code.

## Coverage

Plan completion: not assessed

Frozen head: 3a32d34ca326cf279855dcc1930cc20b4c464461; diff origin/main...HEAD, six Rust files. No source/test/branch changes, pushes, merges or new tickets. Spec #3301 is already merged; no specification diff.

Existing final-head logs were inspected: tmp/exact-kernel-tests-rebased.log: 12 passed, 0 failed (exit 0); tmp/quantifier-tests-rebased.log: 24 passed, 0 failed (exit 0), measured W=772835. The fresh W/W-1 and both settlement paths are genuine behavioral assertions. tmp/clippy-lib-final.log: exit 101, four diagnostics in unchanged certificate/product.rs and model_check/refinement_replay.rs; git diff confirms neither file changed. Changed-file formatting is author-reported passing; reviewer git diff --check passed. No Cargo execution or full gates repeated, per review brief.

Reverse inventory for this PR: 6 behavior families (projection/PSC arithmetic; triangular signs/PRS; root isolation/comparison/lifting; decision-DAG rational/Boolean evaluation; joint enabledness/time partition; typed exhaustion/undefined settlement), all owned by FR-450 with FR-447 expression context. No new external API/dependency/configuration surface. No changed source/test stubs, ignored tests, mocks of the unit under test, warning suppressions, CI weakening or synthetic exhaustion. Four binder wildcard additions preserve existing assertions. A whole-repository reverse-behavior audit is not claimed; the scoped PR audit does not turn unrelated pre-existing code into new findings.

Optional semantic gap-analysis mode was not separately invoked; code-review independently examined the requested mathematical and evaluator contracts.

## Examined Units

```yaml
scope:
  - id: FR-450-AC-14
    path: spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md
    role: examined
    excerpt: For the `Coupled` source and point lasso `L` below, `Run::Gone` is
      enabled exactly for local dwell instants `[0, 1/sqrt(2)]`. `Weak` replays
      `Reproduced` with kind `Formula`; `Strong` replays `NotReproduced` at
      check `Claim`. Neither returns `NotExact`.
  - id: FR-450-AC-15
    path: spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md
    role: examined
    excerpt: Change only AC-14's `Run::Gone` guard to `2 * v * v = 1`. With target
      invariant `v * w <= 1/2`, its enabled set is the single algebraic instant
      `1/sqrt(2)` each period, so `Strong` gives `NotReproduced` at check
      `Claim`. Changing the target comparison to `v * w < 1/2` makes the enabled
      set empty, so `Strong` gives `Reproduced` with kind `Formula`. Neither
      returns `NotExact`; a point-only enabled set is not discarded and a strict
      comparison does not include its boundary.
  - id: FR-450-AC-16
    path: spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md
    role: examined
    excerpt: Extend AC-14's model by variable `z`, initially 0, with rate 0 in both
      modes and `z = 0` added to `Run`'s invariant; add `z >= v` to
      `Run::Gone`'s reset and replace `Gone`'s invariant by `v * w * z <= 1/2`.
      Extend each point of `L` with `z = 0`. The three free post-jump variables
      have a joint solution exactly for local instants `[0, cubert(1/2)]`;
      `Weak` gives `Reproduced` with kind `Formula`, and `Strong` gives
      `NotReproduced` at check `Claim`, never `NotExact`.
  - id: FR-450-AC-17
    path: spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md
    role: examined
    excerpt: Replace only AC-14's `Run::Gone` reset by `v >= 1 and v < 0 and w >=
      0`. No post-jump state satisfies the reset at any instant. The enabled set
      is empty; both `Weak` and `Strong` replay `Reproduced` with kind
      `Formula`, never `NotExact`, rather than treating the contradictory bounds
      as independent witnesses.
  - id: FR-450-AC-18
    path: spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md
    role: examined
    excerpt: For AC-14's reset and lasso, the target invariant `(if v * w <= 1/2
      then true else false)` has the same enabled set and both replay outcomes
      as AC-14. Each target invariant `v * w != 1/2`, `v * w < 1/2 or v * w >
      1/2`, and `v * w / (1 + w * w) > 1` is feasible at every local instant, so
      both fairness claims give `NotReproduced` at check `Claim`. The
      denominator `1 + w * w` is strictly positive over the reals. None of these
      admitted Boolean, conditional, disequality or rational expressions returns
      `NotExact`.
  - id: FR-450-AC-19
    path: spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md
    role: examined
    excerpt: Replace only AC-14's `Weak` body with `always holds(1 / (h.v - h.w) >
      0)`. On `L` its divisor is 0 at local time 0 and positive afterward. Exact
      replay returns `Reproduced` with kind `UndefinedEvaluation`, cause
      `division-by-zero`, `where.position` 0, `where.time` the point interval
      `[0, 0]`, and the division's source locus, never kind `Formula` or
      `NotExact`. From `solver-inconclusive`, or a provider's `refuted` entry
      claiming that same undefined kind, position, time, locus and cause,
      settlement is `refuted` with cause `UndefinedEvaluation{where, cause}` and
      one matching counterexample, never
  - id: FR-450-AC-20
    path: spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md
    role: examined
    excerpt: "Run AC-14's exact replay of `Weak` twice with fresh meters and a bound
      sufficient to complete; the result and consumed work `W > 0` are equal. A
      fresh bound `W` completes with that result; a fresh bound `W - 1` returns
      `HybridReplayRefusal::ResourceExhausted { counter: WorkUnits }`, never a
      replay verdict or `NotExact`. Settlement from either a provider's
      `refuted` entry or `solver-inconclusive` is `incomplete`,
      `Incomplete(LimitReached{limit: work_units, value: W - 1, setting:
      accounting.work_units})`, with no verdict or counterexample. A denied
      projection or elimination charge is propagated "
  - id: FR-450
    path: spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md
    role: examined
    excerpt: |-
      Before each projection, elimination, root-isolation or algebraic-sign
        step, exact replay SHALL charge the caller's `HybridReplayMeter` for the
        work of that step. When a charge is denied, replay SHALL return
        `HybridReplayRefusal::ResourceExhausted { counter: WorkUnits }` before
        taking the step, with the incomplete settlement below; no approximation,
        verdict or unchecked continuation substitutes for the denied step.
  - id: FR-447
    path: spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md
    role: context_only
    excerpt: >-
      S3 SHALL admit in these expressions the forms FSpec FR-193 "Continuous
        values and expressions" admits: continuous variables, numeric literals,
        `exact-value`s and `rational(n, d)` read exactly, `true` and `false`,
        unary `-`, `+`, `-`, `*` and `/` (products and quotients of variables
        included), the comparisons `=`, `!=`, `<`, `<=`, `>=` and `>`, `not`,
        `and`, `or` and `implies`, `let`, `if ... then ... else`, and a call of a
        pure function whose parameters and result are `Boolean` or numeric
  - id: fsl-replay/src/hybrid_replay.rs
    path: fsl-replay/src/hybrid_replay.rs
    role: examined
    excerpt: |-
      // SPDX-License-Identifier: AGPL-3.0-or-later
      //! FR-450 (ADR-026 HY-2): a `HybridTrace` replayed against the claim's
      //! checked hybrid model (FR-447), by FSpec FR-193 "Trace replay".
      //!
  - id: fsl-replay/src/hybrid_replay/exact.rs
    path: fsl-replay/src/hybrid_replay/exact.rs
    role: examined
    excerpt: >-
      // SPDX-License-Identifier: AGPL-3.0-or-later

      //! FR-450 "Exact replay": FSpec FR-193 "Trace replay" exact checks 1 to 5

      //! over a point trace through modes with polynomial flows, and the
      decisive

      //! instant that makes a reproduced trace a `formula` or an
  - id: fsl-replay/src/hybrid_replay/exact/cad.rs
    path: fsl-replay/src/hybrid_replay/exact/cad.rs
    role: examined
    excerpt: >-
      // SPDX-License-Identifier: AGPL-3.0-or-later

      //! Sign-invariant cylindrical decomposition with rational projection and

      //! triangular algebraic samples. Coordinates are ordered time, then reset

      //! variables. Coefficients are never divided by algebraic values:
      specialized
  - id: fsl-replay/src/hybrid_replay/exact/joint.rs
    path: fsl-replay/src/hybrid_replay/exact/joint.rs
    role: examined
    excerpt: >-
      // SPDX-License-Identifier: AGPL-3.0-or-later

      //! Exact existential reset constraints. Symbolic execution preserves
      division

      //! holes and left-to-right Boolean evaluation in a finite sign decision
      DAG.

      //! Every branch's polynomials are collected before CAD projection or
      witness
  - id: fsl-replay/src/hybrid_replay/tests/quantifier_tests.rs
    path: fsl-replay/src/hybrid_replay/tests/quantifier_tests.rs
    role: examined
    excerpt: >-
      // SPDX-License-Identifier: AGPL-3.0-or-later

      //! FR-450 "Exact replay": real quantifier elimination over the post-jump

      //! variable a reset leaves free, at rational and at irrational instants,
      and

      //! the enabledness it decides in a replayed lasso. Every expectation is
      an
  - id: fsl-replay/src/spine/clause/tests.rs
    path: fsl-replay/src/spine/clause/tests.rs
    role: examined
    excerpt: |-
      // SPDX-License-Identifier: AGPL-3.0-or-later
      //! TC-464 to TC-468 (FR-106, FR-107, FR-109).
      //!
      //! The domain package below (`test/nodes`) is a single self-referencing
```

## Matrix Availability

Matrix limitation: quoin matrix --repo <frozen-worktree> --json and quire coverage --scope <frozen-worktree> --json remained CPU-active with zero JSON output after more than six minutes each; reviewer terminated those two owned processes (child exit 143). This is an unavailable bounded engine result, not a verified CLI bug. A static quire matrix fallback was also attempted with a 45-second timeout; its result is recorded separately. The provisional tracked-file index (provisional-index.json) found 6047 criterion rows, 4927 direct Rust tags and 1120 rows without a direct Rust tag across 2150 Rust files; these are NOT engine matrix statuses and do not account for qualified records, inherited tags, ignored tests or non-Rust source. All FR-450-AC-1 through AC-20 have direct tags; AC-14 through AC-20 were individually inspected against their actual behavioral assertions. No unrelated provisional absence is filed as a PR defect. Engine evidence freshness, stale-tag diagnostics and whole-repository matrix completeness remain unverified; no engine-backed gap PASS is claimed.

Static fallback result: `timeout 45s quire matrix --scope <frozen-worktree> --format json` exited 124 with no JSON output. Coverage diagnostics emitted duplicate-archetype/inverse-edge warnings, retained in the tool record; there is no computed matrix result to interpret.
