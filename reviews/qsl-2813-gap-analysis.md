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

## Confined expectation-delta disposition

The later one-file test delta adds precisely the three existing evaluator dependencies authorized by arch: filament-observation, sha2 and unicode-normalization. The complete sorted normal-dependency equality remains exact, build dependencies remain prohibited, and all later workspace-layer edge assertions are unchanged. There is no new product behavior or production dependency change in this delta.

The static ownership reconciliation is ADR-011 section 6.1 for the two third-party crates and X-8's explicit FR-861 observation-authority dependency sentence for filament-observation. The PR body quotes the governing full sentences. This is an exact expectation correction, not an open-ended exemption. No new requirement, API, type copy, stub, unowned behavior or semantic value conversion is introduced.

The already-reviewed layer-4 product correction remains untouched. No additional whole-repository audit or native matrix sweep is performed; the existing native-matrix evidence limitation is unchanged. The lead reports sixteen of seventeen named cases and workspace compilation passing on the preceding tree, with compilation and the single formerly failing exact case queued for the current delta.

**Latest disposition: source clear; runtime/evidence remains CONDITIONAL.** No new finding is added and the original Findings table is preserved. Plan completion: not assessed. This disposition supplies no current-head runtime PASS and no repository-wide assurance claim.

### Additional examined scope

```yaml
{
  "scope": [
    {
      "id": "ADR-011",
      "path": "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md",
      "role": "examined",
      "excerpt": "The column is the exhaustive allow-list of first-party layer edges; an external third-party crate is not a layer edge and this column does not govern it (for example, `filament-driver` uses `serde_json` to read FR-331 manifest bytes as data, ADR-013 T-7), except that a cell that restricts its external crates with \"only\" or \"without `std`\" (SV, WG) is exhaustive for that crate; a per-crate exact dependency table in a layering test is a stricter gate that governs where it exists."
    },
    {
      "id": "ADR-011/X-8",
      "path": "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md",
      "role": "examined",
      "excerpt": "FR-861 adds `filament-observation`, the observation-authority owner, as a git dependency on the repository `agent-ix/filament-observation` at `branch = \"main\"`, resolved through the workspace patch that keeps one `fsl-foundation` in the graph; its own dependencies (`fsl-foundation`, `filament-exact`, `quire-canonical`, serialization crates) are all below layer 5."
    }
  ]
}
```
