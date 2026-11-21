---
id: SR-2759002
title: "QSL-2759 and QSL-2756 gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2759, QSL-2756; fsl-analyze/src/analyze/document.rs, fsl-analyze/src/model_check/state_graph/sample.rs, fsl-analyze/tests/it/, fsl-eval/src/simulation/"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-166", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-626", type: references }
---

## Summary

Repository-driven audit of the changed surface; plan completion was not assessed. Reverse-gap and stub inspection found no additional changed-code defects. The complete computed matrix audit remains unavailable under the explicit inherited limitation. Trace defects are owned once by SR-2759001.

## Verdict

**CONDITIONAL** — changed-surface reverse-gap inspection is clean; full matrix reconciliation is incomplete and cannot establish an engine-backed PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Scope

```yaml
{
  "scope": [
    {
      "id": "FR-166-AC-1",
      "path": "spec/functional/FR-166-sample-witnesses-for-possible-claims.md",
      "role": "examined",
      "excerpt": "Over ADR-022 §7.1's subject with default limits and no seed, each `ReachesTwo` instance gets a sampled witness from `(0, 0)` whose last state is the first visited state where the bound config's `versionNumber` is 2, whose source is `Sampled` with seed 0 and a trace index below 64, and phase 1 searches for no explored witness for the item."
    },
    {
      "id": "FR-166-AC-2",
      "path": "spec/functional/FR-166-sample-witnesses-for-possible-claims.md",
      "role": "examined",
      "excerpt": "The same request with `witness_samples` 0 draws no walk and every witness comes from phase 1. `ReachesThree` with the default draws 64 walks per instance, finds no witness, and leaves the item to phase 1."
    },
    {
      "id": "FR-166-AC-3",
      "path": "spec/functional/FR-166-sample-witnesses-for-possible-claims.md",
      "role": "examined",
      "excerpt": "Over the §7.1 unit with two initial snapshots, `(0, 0)` and `(1, 1)`, the witness for initial state 1 has a trace index in `64..128`. A request with seed 7 records seed 7; two requests with no seed give byte-equal witnesses and record the default seed."
    },
    {
      "id": "FR-166-AC-4",
      "path": "spec/functional/FR-166-sample-witnesses-for-possible-claims.md",
      "role": "examined",
      "excerpt": "`witness_samples` `u64::MAX` over the two-snapshot subject returns `Stopped(ResourceExhausted, WitnessSamples)` before any walk, naming the limit and its value; a cancelled `Cancel` handle stops the run before the first walk."
    },
    {
      "id": "FR-166-AC-5",
      "path": "spec/functional/FR-166-sample-witnesses-for-possible-claims.md",
      "role": "examined",
      "excerpt": "Over §7.1's subject with default limits, `possible 2 div (2 - c.versionNumber) = 2` for `c = a` under the subject's integer-division law (its package selects one, as FR-168-AC-11's fixture does; `div` needs that selection) gets a sampled witness ending at a node with `va = 1`, and the item does not settle from it: exploration runs to completion and the item settles `refuted` with cause `UndefinedEvaluation` at `(2, 0)` (FR-168-AC-7). `ReachesTwo` under the same limits settles `proved` with its sampled witnesses only after exploration completes with no open node."
    },
    {
      "id": "FR-166-AC-6",
      "path": "spec/functional/FR-166-sample-witnesses-for-possible-claims.md",
      "role": "examined",
      "excerpt": "`ReachesTwo` with default `witness_samples` and `max_states` 2 gets a sampled witness for each instance, the exploration stops at `max_states`, and the item settles `inconclusive`, `WellDefinednessUnchecked`, never `proved` (FR-169-AC-8)."
    },
    {
      "id": "FR-166-AC-7",
      "path": "spec/functional/FR-166-sample-witnesses-for-possible-claims.md",
      "role": "examined",
      "excerpt": "`ReachesThree` with the default limits draws 64 walks per instance, each stopped by `max_walk_steps` at 4,096 steps, finds no witness and decides nothing; the terminal record names `max_walk_steps`, 4,096 and a count of 64 per instance. With `max_walk_steps` 1, `ReachesTwo` gets no sampled witness, since `versionNumber` 2 is two steps away, and every witness comes from phase 1. With horizon `max_depth` 1, every walk takes at most 1 step, the record states horizon 1, and no walk is counted as stopped by a limit."
    },
    {
      "id": "FR-626-AC-1",
      "path": "spec/functional/FR-626-request-deadlock-freedom-terminal-states-and-time-lock-freedom-items.md",
      "role": "examined",
      "excerpt": "Over the Counter fixture with no `terminal` clause, `{\"claim\": \"M\", \"temporal\": {\"item_kind\": \"deadlock-freedom\"}}` settles `refuted` with a counterexample whose last state is `value = 3`."
    },
    {
      "id": "FR-626-AC-2",
      "path": "spec/functional/FR-626-request-deadlock-freedom-terminal-states-and-time-lock-freedom-items.md",
      "role": "examined",
      "excerpt": "The same unit with `terminal Done using v on M when { self.value = 3 };` settles that item `proved`."
    },
    {
      "id": "FR-626-AC-3",
      "path": "spec/functional/FR-626-request-deadlock-freedom-terminal-states-and-time-lock-freedom-items.md",
      "role": "examined",
      "excerpt": "`time-lock-freedom` over the untimed Counter subject settles `invalid-request` `inapplicable-kind`; over a timed subject whose invariant forbids time passing in a reachable location it settles `refuted` with a concretised counterexample."
    },
    {
      "id": "FSpec-FR-391-AC-2",
      "path": "agent-ix/filament-specification/functional/analysis/FR-391-settle-state-graph-claims.md",
      "role": "examined",
      "excerpt": "A reachable trap where the form requires reaching the target settles `refuted`, `closed-scope`, also when the run reached its maximum depth elsewhere."
    },
    {
      "id": "FSpec-FR-366-AC-4",
      "path": "agent-ix/filament-specification/functional/temporal/FR-366-check-deadlock-freedom.md",
      "role": "examined",
      "excerpt": "A request with two temporal items over one subject receives exactly one deadlock-freedom item, and a request over two distinct subjects receives one per subject; the item's obligation identity is the subject with item kind `deadlock-freedom`."
    },
    {
      "id": "FR-628-AC-1",
      "path": "spec/functional/FR-628-request-state-graph-query-items.md",
      "role": "examined",
      "excerpt": "Over the Counter fixture, `possible (value = 3)` settles `proved` with a witness path of three steps that replays; over the Counter started at `value = 1`, `possible (value = 0)` settles `refuted`, since `inc` only increases the value."
    }
  ]
}
```

## Coverage

Reconciliation: unavailable inherited quoin/quire command hangs; no retry authorized. No run evidence read through a matrix engine.

Plan completion: not assessed

Inventoried changed behaviors: 5 (derived-anchor resolution, stable target truth reuse, keyed walk API, ordered expansion reuse, typed graph settlement axes); all owned by FR-626, FR-166/FR-101 and FR-628/FSpec-FR-391 respectively. Untraced behaviors: 0 identified; source stubs: 0; test stubs: 0. Computed criterion counts: unavailable.

Whole-repository quoin/quire matrix, coverage and advise are unavailable due to the inherited hanging commands documented in the dispatch; the lead explicitly directed no retry. No engine coverage counts or full-repository PASS are asserted. Manual changed-surface inventory is provisional and is not a grep fallback or an engine matrix. Optional semantic review skipped. No applicable AssuranceProfile found in spec/. No stubs, mocked production seams, copied algorithm, untraced changed behavior, unsafe code or new wire vocabulary identified.

Existing exact-head evidence inspected: 21 named regression logs passed, including trace parity, original 2000ms cyclic trap, storage fallback, cancellation, unreadable predicates, explicit/default selection and protocol/axes controls. Named-crate all-targets check passed. fsl-eval no-deps clippy passed; combined lint remains blocked by 12 untouched analyze diagnostics (dependency-inclusive: 15 untouched replay diagnostics). No CI, feature, dependency, default limit, warning or qualification gate changed. No new cargo run during this static review; root owns final rebased exact-head gate.
