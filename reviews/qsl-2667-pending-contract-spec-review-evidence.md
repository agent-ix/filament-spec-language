---
id: SR-543
title: "spec-review/evidence of pending export evidence contract"
type: SpecReview
analysis: evidence
scope: "agent-ix/filament-spec-language; spec/functional/FR-1022-take-in-tla-and-lean-results-as-trusted-evidence.md; QSL-2667"
review_set: subset
---

## Summary

Frozen candidate a06df09c6e7a4dd726790deed16c385beee08c58, PR filament-spec-language#3318, ticket QSL-2667. Only FR-1022 changes. No applicable AssuranceProfile exists (exact type search under spec/ found none). Manual bounded review selected by the dispatch; no enforced workflow or full-repository assurance is claimed.

## Verdict

**NOT ASSESSED (advisor unavailable); manual inspection found no defect** — The declared Test method fits the finite admission/refusal assertions by manual judgement. Existing unit and integration assertions inspect the exact pending diagnostic and assembled-package lifecycle, but are tagged FR-1022 rather than amended AC-3 and are not counted as a computed criterion binding. Deterministic quoin advise and matrix extraction are unavailable from the known hanging runs and were deliberately not retried. This is an incomplete evidence-method analysis, not a catalog-backed PASS.

## Examined scope

```yaml
scope:
  - id: FR-1022
    path: spec/functional/FR-1022-take-in-tla-and-lean-results-as-trusted-evidence.md
    role: examined
    excerpt: Until original generated-package binding and target-result verification
      are implemented, FSL SHALL admit the closed `export_evidence` input shape
      and then refuse admitted evidence with `unimplemented_capability` /
      `unimplemented-meaning`, `details.capability` `export_evidence`, category
      `unsupported`, exit `21`. This boundary performs no package-key or
      obligation binding, artifact read, tool probe, tool execution or native
      trace replay; it writes no proof result. Malformed evidence retains its
      member-specific admission refusal before this capability refusal.
  - id: FR-1022
    path: spec/functional/FR-1022-take-in-tla-and-lean-results-as-trusted-evidence.md
    role: examined
    excerpt: The following checking rules and AC-1/AC-2 remain required of the
      future evidence checker; admission and the pending capability refusal
      establish none of their proof, replay or package-binding claims.
  - id: FR-1022-AC-1
    path: spec/functional/FR-1022-take-in-tla-and-lean-results-as-trusted-evidence.md
    role: context_only
    excerpt: A Lean proof of `Ob_Low` accepted by the Lean child process settles
      `proved`, `trusted`, extent `unbounded`, never `certified`; with `lean`
      absent the intake settles `unsupported`, `tool-unavailable`, exit `21`.
  - id: FR-1022-AC-2
    path: spec/functional/FR-1022-take-in-tla-and-lean-results-as-trusted-evidence.md
    role: context_only
    excerpt: A TLC trace of a seeded violation reads back through the name map and
      settles `refuted` after native replay; the same trace edited so it no
      longer replays settles nothing.
  - id: FR-1022-AC-3
    path: spec/functional/FR-1022-take-in-tla-and-lean-results-as-trusted-evidence.md
    role: examined
    excerpt: capability` `export_evidence`, category `unsupported`, exit `21`, with
      null `last_stage` and `result`.
  - id: FR-1022-AC-3
    path: spec/functional/FR-1022-take-in-tla-and-lean-results-as-trusted-evidence.md
    role: examined
    excerpt: No artifact is read or tool probed or run; no proof, counterexample or
      package binding is accepted.
  - id: FR-1022-AC-3
    path: spec/functional/FR-1022-take-in-tla-and-lean-results-as-trusted-evidence.md
    role: examined
    excerpt: Malformed evidence is refused at its own member before this capability
      refusal.
```

## Evidence and limits

No Cargo, proof toolchain, Python test, aggregate gate, or full matrix was run. Known hanging advisor/matrix commands were not repeated by explicit dispatch instruction; catalog-backed evidence recommendations and computed criterion coverage remain unavailable. The original genuine proof/record tests remain ignored and unchanged. Their historical AC-3 trace names the superseded future record-binding behavior and is not counted as evidence for amended AC-3. No new runtime code or trace tag was requested by this spec-only amendment. Genuine AC-1/AC-2 remain unmet future checker obligations.

Inspected existing behavior at fsl-command/src/command/export/evidence.rs:23 and existing oracle at fsl-command/tests/it/cli_contract/export.rs:86. No execution evidence was minted. Fresh origin/main 4a4e5c69b41d2a43cde58f28d45309c45f36728f contains no SR-539 through SR-543; reserved SR-528 through SR-538 were not used.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
