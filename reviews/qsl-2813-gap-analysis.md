---
id: SR-967175026302197768900474181941935084711
title: "Bounded product gap analysis of the QSL-2813 package-layer fix"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2813; fsl-package/Cargo.toml; fsl-package/src/protocol_view.rs; fsl-package/src/checked_views/hyper.rs; bounded one-fix-round disposition only"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---

## Summary

The added product fix restores the existing layer-4 dependency rule by routing package discriminants through the existing layer-3 exports. This is a bounded fix-diff analysis, not a fresh whole-repository assurance audit; plan completion is not assessed.

## Verdict

**CONDITIONAL — source clear, evidence pending.** Static requirement, test and source reconciliation found no new gap in the three changed files. A current native matrix and the fix-head titan results are not established, so this artifact does not assert an engine-backed or runtime PASS.

## Scope

```yaml
{
  "scope": [
    {
      "id": "ADR-011",
      "path": "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md",
      "role": "examined",
      "excerpt": "| 4 | `package` | S4, I2 byte reader (verification calls `library`) | 3, F, C, SV, K; `filament-contract-model` for the v2 wire contract: the typed structs the emitter builds and the IR reader the I2 reader delegates to (X-7) |"
    },
    {
      "id": "FR-519-AC-5",
      "path": "spec/functional/FR-519-publish-the-protocol-read-surface-the-filament-protocol-link-layer-imports.md",
      "role": "examined",
      "excerpt": "For each record the map marks read by an FSL view function, the view function over **R** returns the node, argument or lock member the map names, equal to the value the checked clause holds in memory."
    },
    {
      "id": "FR-940-AC-16",
      "path": "spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md",
      "role": "examined",
      "excerpt": "Over AC-4's Counter unit with the hyper clauses `Equal` (`forall trace a of M forall trace b of M { always holds(c.value @ a <= 3 and c.value @ b <= 3) }`, universal, copy-swap symmetric), `Partner` (forall-exists equality), `CanReachThree` (single existential) and a projection-aligned clause with an `align skip` list and a `match` block holding one universal and one existential conjunct, `hyper_clause_form` of each clause's `claim_name` occurrence returns equal records over the `AdmittedPackageRead` and over the `BarePackage` of its handoff directory"
    }
  ]
}
```

## Bounded reconciliation

The manifest change is owned by ADR-011 section 6.1's layer-4 row. The unchanged tests/it/family_outcome_layering.rs test no_crate_below_layer_three_depends_on_the_check_core reads actual Cargo dependency kinds and retains its exact package table; the test's allowance is not widened to accept the erroneous forms edge. Normal production access now goes through the already permitted fsl-semantics edge; dev fixtures retain forms.

protocol_view.rs retains the same ControlBody Attempt/Fence fields and the same five orderings, only qualifying the underlying AccessOrdering through fsl_semantics::check. The latter is an existing pub use of fsl_forms::AccessOrdering, so nominal type identity is unchanged. FR-519-AC-5 owns the package-view equality behavior. Existing emit::tests::protocol_view::the_execution_view_reads_attempt_facts_from_package_nodes and emit::tests::protocol_controls::a_fence_emits_each_of_its_orderings exercise actual emitted/read package nodes, not a substituted decoder.

checked_views/hyper.rs retains the exact forall gate and PackageHyperQuantifier construction, replacing only the qualification of the existing QuantifierKind::Forall variant. FR-940-AC-16 owns equality of the package/read hyper forms. Existing package hyper and model consumers remain the relevant runtime evidence; the lead is running its six direct protocol/memory/hyper consumers beside the failing root layering case.

No function, public behavior, constructor, enum, serialization rule, bound or feature is added by the fix. The three edited files contain no introduced stub, placeholder result, bypass or duplicated type. Remaining direct forms uses were inspected at their cfg(test) module boundaries, including the hyper normal-form fixture and emitter test modules.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- Reconciliation: static source/spec/test trace inspection, with installed Quire 0.52.0 (engine 0.62.2) artifact validation; not a fresh native computed-matrix or run-evidence result.
- Plan completion: not assessed
- Scope: exactly three product files in the original PR's one fix round. The remainder of the repository is not re-audited by this disposition.
- Native matrix: not rerun. The lead reports the known current-version native matrix resource/size limitation; this does not become a passing assurance result. The initial test-only review's older matrix was not reused as fix-head evidence.
- Criteria: ADR-011 layer row, FR-519-AC-5 and FR-940-AC-16 inspected directly; current engine-backed counts are unavailable.
- Inventoried changed behaviors: dependency-kind ownership, protocol ordering type route, hyper quantifier type route. New behavior/API count 0; unowned additions 0; introduced source/test stubs 0 within the bounded diff.
- Runtime: lead-reported pre-fix twelve cases 11 passed/1 failed and workspace check exit 0; fix-head root layering and six direct consumers remain pending. No local builds/tests/compiled repository scanners ran.
- Semantic comparison: the parent explicitly requested requirement and product-fix consistency; it was performed over the bounded diff. No separate whole-repository semantic campaign was run.
