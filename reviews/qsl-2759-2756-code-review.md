---
id: SR-2759001
title: "QSL-2759 and QSL-2756 code and Rust review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2759, QSL-2756; fsl-analyze/src/analyze/document.rs, fsl-analyze/src/model_check/state_graph/sample.rs, fsl-analyze/tests/it/, fsl-eval/src/simulation/"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-166", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-626", type: references }
---

## Summary

Reviewed frozen d04cdfa0262b0811a86738caf57c4bf7a7c80f27 against 4acd578a83d838df51fc04c8a05012c848493eac for QSL-2759 and QSL-2756. The change preserves one seeded sampler loop, canonical duplicate-transition ordering and per-position provenance; caches are request-local, truth-only where applicable, result-admitted before retention and fall back on denial. No production defect found. Two incorrect criterion bindings require correction.

## Verdict

**CONDITIONAL** — fix the two trace bindings. Production changes are sound within the inspected scope; inherited matrix and untouched lint limitations remain explicit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new deadlock regression claims FR-626-AC-2 but both fixture variants assert refuted; AC-2 requires a terminal predicate covering value 3 and a proved result. Remove this binding or add the actual proved case. | fsl-analyze/tests/it/analyze_cli.rs:752 |
| FND-002 | medium | The new single explicit deadlock-item test claims FSpec-FR-366-AC-4 but exercises neither two temporal items sharing one subject nor two subjects, and asserts no obligation identity. Remove this binding or test the specified derivation and identity behavior. | fsl-analyze/tests/it/analyze_cli.rs:752 |

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

Examined files:

- fsl-analyze/src/analyze/document.rs
- fsl-analyze/src/model_check/state_graph/sample.rs
- fsl-analyze/tests/it/analyze_cli.rs
- fsl-analyze/tests/it/analyze_cli_support.rs
- fsl-analyze/tests/it/analyze_state_graph_cli.rs
- fsl-analyze/tests/it/state_graph.rs
- fsl-eval/src/simulation/model.rs
- fsl-eval/src/simulation/order.rs
- fsl-eval/src/simulation/sample.rs

## Coverage

Existing exact-head evidence inspected: 21 named regression logs passed, including trace parity, original 2000ms cyclic trap, storage fallback, cancellation, unreadable predicates, explicit/default selection and protocol/axes controls. Named-crate all-targets check passed. fsl-eval no-deps clippy passed; combined lint remains blocked by 12 untouched analyze diagnostics (dependency-inclusive: 15 untouched replay diagnostics). No CI, feature, dependency, default limit, warning or qualification gate changed. No new cargo run during this static review; root owns final rebased exact-head gate.

Whole-repository quoin/quire matrix, coverage and advise are unavailable due to the inherited hanging commands documented in the dispatch; the lead explicitly directed no retry. No engine coverage counts or full-repository PASS are asserted. Manual changed-surface inventory is provisional and is not a grep fallback or an engine matrix. Plan completion: not assessed. Optional semantic review skipped. No applicable AssuranceProfile found in spec/. No stubs, mocked production seams, copied algorithm, untraced changed behavior, unsafe code or new wire vocabulary identified.

## Dispositions

Round 1, inspected frozen head `00e0b65a96f5e4a924d1ebd105daaf945ea8fdcc` rebased on `dab8c60624b4427e2290efe7ebaa3827f5ae0842`. Original finding rows remain unchanged.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Removed the FR-626-AC-2 binding; the regression retains only TC-1586 and FR-626-AC-1. |
| FND-002 | fixed | Removed the FSpec-FR-366-AC-4 binding; assertions and production behavior are unchanged. |

### Round 1 verdict

Both findings are fixed; no new finding. Reviewed code is mergeable subject to the lead's final exact-head named-crate gate and unchanged-main policy. Inherited computed-matrix and untouched lint limitations remain; no full-repository matrix PASS is asserted. No runtime rerun required for this tag-only fix.

After excerpt for both fixed findings:

```rust
#[trace("TC-1586", "FR-626-AC-1")]
```
