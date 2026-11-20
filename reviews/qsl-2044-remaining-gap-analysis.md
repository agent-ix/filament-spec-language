---
id: SR-2026111919340478491681416
title: "gap-analysis of QSL-2044 remaining source corpus"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2044; PR3103; tests/corpus/refinement/remaining/**; tests/it/refinement_remaining_source_routes.rs; additive child registration; source_corpus runner visibility"
review_set: subset
---

## Summary

Reviewed frozen head f4c1adf7cf4ab5abdc64d57262e72dcf1e789da7 for QSL-2044 / filament-spec-language#3103. Scope is the changed source corpus and test helpers only; no whole-ticket or whole-repository conclusion.

## Verdict

CONDITIONAL: one incorrect trace binding. Active behavior evidence is sound for this explicitly partial slice; pending tests are not passing coverage.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The HP-1 execution-relation test claims FR-488, which explicitly excludes HP-1 relations | tests/it/refinement_remaining_source_routes.rs:125 |

## Coverage

All 34 changed files examined, including each source, request and expectation, the small reference README, additive child-module registration, and restricted test-helper visibility. Original source programs reference the existing same-repository Counter input; no model, schema, grammar or sibling-repository files are copied. The shared runner invokes public source compilation and checks exit/category/stage/code/cause. The active protocol test checks class order, operation/stutter targets and that the finish source node remains an emitted finish obligation. Pending reference-resolution oracles assert actual referenced node identities and preserve QSL-2609 rather than blessing missing targets. Empty align-skip is outside the runnable directory under QSL-2599. QSL-2239, QSL-2248 and QSL-2128 remain other pending families, not passing coverage.

Accepted coder-reported execution: nine source expectations and protocol emission passed; the original four-test run exited101 with two intended-reference failures, now staged under QSL-2609. Staging-only compilation exited0. No tests, builds, matrix, gates or old baseline results were repeated by this reviewer. Lead owns the one final applicable gate. Rust/style, direct borrowing, local helper reuse, test panic invariants, bounded original fixtures and module conventions were reviewed. No production API, CI, dependencies, unsafe code, task lifecycle or wire-reader behavior changed. The pub(super) change is confined to the integration-test crate and enables reuse of the existing runner.

No applicable AssuranceProfile in the established repository context. Spec review is inapplicable: no specification or plan changes. Docs-style was considered manually for the small corpus reference note; no additional documentation build or Python checks ran.

Plan completion: not assessed

This is a diff-scoped manual trace, reverse-ownership and hollow-evidence check. A whole-repository matrix computation was explicitly excluded by the brief and has not been implied by this review.

## Examined Scope

```json
{
  "scope": [
    {
      "id": "FR-172-AC-3",
      "path": "spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md",
      "role": "examined",
      "excerpt": "§13's clause with `align skip { V::Vault::mix }` checks with `align_skip` holding `mix`; `align skip { }`, `align skip { V::Vault::mix, V::Vault::mix }`, `align skip { N::Device::tick }` in a clause over `V` alone, and `align skip` with `fair weak V::Vault::step` each refuse `unsupported_construct`/`expression-form`."
    },
    {
      "id": "FR-171-AC-3",
      "path": "spec/functional/FR-171-build-forms-for-hyper-clauses-over-behaviours.md",
      "role": "examined",
      "excerpt": "`relation Det using v over (x: V::Vault::step, y: V::Vault::step) { x.i = y.i and pre(x.self.l) = pre(y.self.l) implies x.self.l = y.self.l }` builds a `ModelRelationForm` with executions `x` and `y` in order and the body's `pre` reads as written. None of AC-1 to AC-3's clauses refuses `UnrepresentedConstruct`."
    },
    {
      "id": "FR-136-AC-6",
      "path": "spec/functional/FR-136-check-a-refinement-s-step-rows.md",
      "role": "examined",
      "excerpt": "With FR-136-AC-4's protocol, named `Run`, the concrete alias `M` names the model and the rows naming `M::Run::…` select the protocol, recorded as `concrete_protocol`; a refinement whose rows name no protocol clause has a model subject and `concrete_protocol` absent."
    },
    {
      "id": "FR-901",
      "path": "spec/functional/FR-901-write-every-checked-refinement-at-s4.md",
      "role": "examined",
      "excerpt": "The S4 emitter SHALL write every refinement that S3 checks (FR-135 to FR-139, FR-147) to `filament.checked-semantic-graph/v2` as its `relation`/`refinement` node"
    },
    {
      "id": "FR-488",
      "path": "spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md",
      "role": "examined",
      "excerpt": "A hyper clause over a finite trace domain (FR-191) and a `relation` over model executions (HP-1) are outside this requirement and are emitted as they are today."
    },
    {
      "id": "tests/it/refinement_remaining_source_routes.rs",
      "path": "tests/it/refinement_remaining_source_routes.rs",
      "role": "examined",
      "excerpt": "Two active tests cover nine source cases and finish/stutter emission. Two QSL-2609 ignored tests preserve intended internal-reference resolution assertions."
    },
    {
      "id": "tests/it/source_corpus.rs",
      "path": "tests/it/source_corpus.rs",
      "role": "examined",
      "excerpt": "pub(super) fn run_directory(directory: &Path) -> Result<String, String>"
    },
    {
      "id": "tests/it/refinement_source_routes.rs",
      "path": "tests/it/refinement_source_routes.rs",
      "role": "examined",
      "excerpt": "#[path = \"refinement_remaining_source_routes.rs\"] mod remaining;"
    },
    {
      "id": "tests/corpus/refinement/remaining",
      "path": "tests/corpus/refinement/remaining",
      "role": "examined",
      "excerpt": "Nine original native/request/expectation triples reuse the same-repository counter-step fixture; an excluded empty-skip triple carries QSL-2599. README records active/pending scope."
    }
  ],
  "clean": false,
  "findings": [
    {
      "fnd": "FND-001",
      "severity": "medium",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "trace",
      "artifact_id": "FR-488",
      "related_ids": [
        "FR-171-AC-3",
        "FR-172-AC-4"
      ],
      "path": "tests/it/refinement_remaining_source_routes.rs",
      "lines": "125-127",
      "excerpt": "A hyper clause over a finite trace domain (FR-191) and a `relation` over model executions (HP-1) are outside this requirement and are emitted as they are today.",
      "finding": "The HP-1 execution-relation test claims FR-488 even though FR-488 explicitly excludes relations over model executions. Remove that trace and retain the actual relation/form ownership.",
      "test_id": "execution_relations_keep_distinct_receivers_of_the_same_operation",
      "claimed_ac": "FR-488",
      "correct_ac": "FR-171-AC-3"
    }
  ],
  "bindings": [
    {
      "test_id": "remaining_source_forms_compile_and_refuse_as_specified",
      "ac_id": "FR-171",
      "trace": "correct"
    },
    {
      "test_id": "remaining_source_forms_compile_and_refuse_as_specified",
      "ac_id": "FR-172",
      "trace": "correct"
    },
    {
      "test_id": "remaining_source_forms_compile_and_refuse_as_specified",
      "ac_id": "FR-136",
      "trace": "correct"
    },
    {
      "test_id": "projection_emits_its_bound_skip_operation_and_match",
      "ac_id": "FR-171",
      "trace": "correct"
    },
    {
      "test_id": "projection_emits_its_bound_skip_operation_and_match",
      "ac_id": "FR-488",
      "trace": "correct"
    },
    {
      "test_id": "protocol_finish_is_a_concrete_step_mapped_to_stutter",
      "ac_id": "FR-136",
      "trace": "correct"
    },
    {
      "test_id": "protocol_finish_is_a_concrete_step_mapped_to_stutter",
      "ac_id": "FR-901",
      "trace": "correct"
    },
    {
      "test_id": "execution_relations_keep_distinct_receivers_of_the_same_operation",
      "ac_id": "FR-171",
      "trace": "correct"
    },
    {
      "test_id": "execution_relations_keep_distinct_receivers_of_the_same_operation",
      "ac_id": "FR-488",
      "trace": "wrong"
    }
  ]
}
```

## Dispositions

Round 1, fix commit `29793cb22ae794489f8d453d7cac2f1f5ceaaa25`: **PASS**. The sole metadata finding is fixed. Accepted the lead's exact in-lane report that only the invalid FR-488 trace was removed and FR-171 retained. No runtime change, broad review, tests, builds or matrix repeated.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The HP-1 test retains FR-171 and no longer claims the explicitly excluded FR-488 requirement. |

```json
{
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "after_excerpt": "#[trace(\"FR-171\")]",
      "fix_commit": "29793cb22ae794489f8d453d7cac2f1f5ceaaa25"
    }
  ]
}
```


