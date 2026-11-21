---
id: SR-202611210022442313144108515
title: code-review review of FR-450 coupled reset acceptance
type: SpecReview
analysis: code-review
scope: agent-ix/filament-spec-language;
  spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md;
  QSL-2639
review_set: subset
---

## Summary

Reviewed the single FR-450 diff from origin/main to frozen 837b39c4560737cf77a23553aa13d559ac7f8847 for PR 3301, ticket QSL-2639. This is a specification-readiness review; it does not certify the pending nonlinear implementation. No applicable AssuranceProfile is declared under spec (matches in prior reviews are prose). No branch changes, Cargo, Python, gates, or CI were run. A read-only full matrix attempt was stopped after about 2.5 minutes to preserve the thermal budget; no complete matrix result is claimed. Focused Rust trace-tag search found no bindings for new AC-14–20.

The owner-approved full rational semialgebraic class and existing WorkUnits boundary are taken as the required scope. Gap-analysis and Rust/React lanes are inapplicable to this spec-only diff. Dependency/object/scope/risk/evidence lenses were not selected: no relationship edge, domain object, system boundary, verification method, or ownership decision changes. Base, integrity, failure-domain, and ears-conformance address the edited obligations and failure contracts. No new hashes, pins, external specification copies, or file-tracking machinery are introduced.

## Verdict

**PASS** — no defects found in the changed requirements or acceptance oracles. Mergeable as a specification-only PR; implementation qualification remains pending.

## Method evidence

The change introduces acceptance prose and one owned fixture, with no executable code, copied upstream definitions, suppression, lowered gate, or new API. Existing nonlinear CoupledReset implementation work is explicitly still QSL-2639; this PR does not claim it completed. Static source inspection found no trace tags for new AC-14–20, consistent with the mandated specification-before-implementation sequence and retained assurance work. No test/AC bindings were changed or certified.

## Acceptance oracle

The Run dwell has v=t, w=0 on [0,1], with pre-jump reads in reset right operands. The self-jump is source-order transition 0 and preserves w; loop_to 0 returns to the identical Run valuation after positive period 1. The profile admits unbounded temporal operators and fairness; all rates are polynomial and acyclic.

For AC-14, V,W>=t>=0 gives minimum VW=t², hence enabled [0,1/sqrt(2)]. The never-taken exit is disabled during a nonempty part of each period: Weak permits this lasso, Strong rejects it. The claim already fails at t=0. AC-15 intersects this set with 2t²=1; <= retains the isolated point, < excludes it. AC-16 similarly minimizes VWZ=t³; z is preserved by the self-jump. AC-17 requires a common witness satisfying V>=1 and V<0, so no witness exists.

AC-18's conditional is equivalent to the original comparison. Disequality/disjunction witnesses can be selected with a product different from 1/2 at any t. For the rational target, choose W=1 and V=3 for every t in [0,1]; VW/(1+W²)=3/2>1 and 1+W² is globally positive. Both fairness kinds then reject the never-taken exit.

AC-19 evaluates the actual claim divisor t at time 0; the weak fairness condition still admits L, so undefined evaluation has kind/cause/locus and position 0 at [0,0], rather than becoming an existential-domain exclusion. Settlement agrees with FR-450's existing reproduction and provider-kind parity contracts.

AC-20 measures exact replay with fresh meters, so deterministic work W completes at bound W and a denied step at W-1 yields ResourceExhausted before execution. Settlement separately requires the existing incomplete record with configured bound W-1 and no verdict/counterexample from either entry route; it does not require settlement's static+exact cost to equal standalone exact W.

## Examined scope

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
    excerpt: >-
      - For that enabledness decision, exact replay SHALL decide the full
      rational
        semialgebraic class of FR-447's admitted expressions after nonrecursive
        expansion: polynomial comparisons over the free post-jump variables and
        dwell time, Boolean combinations and conditionals, and rational expressions
        where their denominators are defined. Defined-denominator valuations form
        the quantifier-elimination domain; this does not replace the existing
        `UndefinedEvaluation` result for an expression evaluated on the replayed
        trace with a zero divisor. The decision SHALL have no fixed
        polynomial
  - id: FR-450
    path: spec/functional/FR-450-replay-a-hybrid-trace-against-the-checked-hybrid-model.md
    role: examined
    excerpt: |-
      language "ix:native" edition "1-draft";
      profile p = "filament.temporal.infinite-trace/v1";
      hybrid Coupled using p {
        initial { mode Run when { v = 0 and w = 0 }; }
        mode Run invariant { v >= 0 and v <= 1 and w = 0 }
          flow { v' = 1; w' = 0; }
          transition to Run when { v = 1 } reset { v = 0 };
          transition to Gone when { true } reset { v >= v and w >= v };
        mode Gone invariant { v * w <= 1/2 }
          flow { v' = 0; w' = 0; }
      }
      temporal Weak using p over (h: Coupled) clock "model-time" on origin {
        fair weak Run::Gone;
        always holds(h.v >= 1)
      }
      temporal Strong using p over (h: Coupled) 
  - id: FR-447
    path: spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md
    role: context_only
    excerpt: >-
      - A reset SHALL be `true`, which keeps every variable, or a conjunction,
        possibly under `let` binders, of conjuncts `x ~ e` (FSpec FR-193
        "Resets"): `x` a continuous variable naming its value after the jump, `~`
        one of `=`, `!=`, `<`, `<=`, `>=` and `>`, and `e` any expression of this
        section.
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
