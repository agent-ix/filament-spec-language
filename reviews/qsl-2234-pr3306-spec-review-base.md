---
id: SR-202611210022442313144108521
title: "PR 3306 AC15 fixture spec-review/base"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; FR-1019-AC-15; FR-447 and class ordering context"
review_set: subset
---

## Summary

Only FR-1019-AC-15 changes (line 129), against base f1c43209; reviewed frozen head 9ecf2e6e82c058af0957e1e29f1c76196135e3d0. Ticket: QSL-2234.

Early's first left sides establish [v,a], or [a,v] in the counterpart (fsl-semantics/src/check/hybrid/model.rs:193-216). Initial-condition name order does not declare variables. Each mode defines each variable exactly once; rates are assigned by variable index (model.rs:239-267). Initial condition and both invariants are admitted polynomial comparisons; Early's constant rates are admitted. Late retains the same two quotient equations and textual order. The class check visits all modes, including disconnected Late, then rates in global variable order (fsl-replay/src/hybrid_replay/class.rs:180-211); no reachability condition suppresses Late. A numeric division is rejected at its root before its operands (class.rs:62), so v-first selects the outer / of -a / (1 + v); a-first selects the / of 1 / (1 + a). Both identify Late.

ReferenceProvider returns Unsupported before invoking the solver (fsl-hybrid-reference/src/lib.rs:54-59); diagnostic assembly resolves mode, variable and locus (fsl-analyze/src/analyze/hybrid_item.rs:304-334). This is static proof of fixture/oracle consistency, not an execution or complete acceptance claim. The existing flow_equations_are_walked_in_variable_declaration_order test (fsl-analyze/tests/it/hybrid_provider.rs:539-575) is tagged AC14 and checks only v-first class selection, not AC15 settlement or the reversed counterpart. No implementation changes are required to specify the repaired fixture; remaining implementation/driver acceptance and the umbrella ticket stay open.

Focused quire validation of the changed FR and git diff --check pass. Quire emits pre-existing module catalog warnings. No applicable AssuranceProfile is declared in spec/. No Cargo/Python/gates, complete matrix audit or whole-catalog review was run, per the bounded dispatch. No changed Rust, API, pins, defaults or driver code.

## Verdict

PASS — mergeable as a specification fixture repair. Base checklist: the criterion keeps its ID, original two diagnostic assertions and TC-2869 association; only the expressible fixture and diagnostic mode change. Inputs, deterministic competing-error selection and locations are concrete and falsifiable. The unchanged legacy Verification cell format is outside this diff. No links, requirement statements, domain objects, dependencies or artifact structure change. No new tracking/hash/pin requirement. Coverage remains explicitly partial and is not represented as delivered.

## Scope

```yaml
scope:
  - id: FR-1019-AC-15
    path: spec/functional/FR-1019-ship-a-first-party-reference-hybrid-provider.md
    role: examined
    excerpt: A model with initial entry `mode Early when { v = 20 and a = 1 };` has
      mode `Early` with invariant `v >= 0` and flow `{ v' = 1; a' = 1; }`,
      establishing continuous variables `v` then `a`, followed by mode `Late`
      with invariant `v >= 0` whose flow block writes `a' = 1 / (1 + a)` before
      `v' = -a / (1 + v)`; it settles `unsupported` from the reference provider
      with `unsupported_construct`/`expression-form` located at the outer `/` of
      `-a / (1 + v)`, naming mode `Late` and variable `v`, and not at the `/` of
      `a`'s equation; the same model with `Early`'s flow written `{ a' = 1; v' =
      1; }`
  - id: FR-1019
    path: spec/functional/FR-1019-ship-a-first-party-reference-hybrid-provider.md
    role: context_only
    excerpt: Flow equations are ordered by variable, not as written, because the
      checked mode holds one rate per continuous variable in declaration order
      (FR-447)
  - id: FR-447
    path: spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md
    role: context_only
    excerpt: "pub variables: Vec<ContinuousVariable>,   // order of first appearance
      as an equation's left side"
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
