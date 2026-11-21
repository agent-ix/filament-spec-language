---
id: SR-202611212158130340200000002
title: "WorkScope cohort pin repository gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2145; spec/, workspace Rust sources/tests; Cargo cohort subset"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Repository-driven, planless gap analysis at 0516015174ef2cab09b2861102c047f550c97f94. Current computed matrix and coverage show substantial unchanged repository gaps. Plan completion was not assessed. No pin-induced implementation or traceability defect was found; client acceptance remains open.

## Verdict

**FAIL** for repository-wide assurance: untagged criteria and ignored-only bindings remain and the global reverse inventory is incomplete. This FAIL is not represented as a scoped pin failure. Latest dispositions below preserve existing client work as deferred and accept unchanged baseline only under root's explicit scoped engineering override for the separately architecture-assigned dependency readiness chore.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-002-AC-9 has no bound test in the current computed matrix; the 1139 untagged rows prevent a repository-wide gap PASS. | spec/functional/FR-002-parse-native-units.md:84 |
| FND-002 | high | FR-861-AC-4 is tagged only by an ignored test; the observation authority/ledger client remains unqualified. | spec/functional/FR-861-take-partial-repair-completeness-and-activation-authority-from-filament-observation.md:141 |
| FND-003 | high | FR-875-AC-1 is tagged only by an ignored test; the on-each probability client remains unqualified. | spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md:108 |
| FND-004 | high | FR-1044-AC-10 is untagged and full Source/Package composition parity remains ignored and unmet. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:420 |
| FND-005 | high | Repository-wide reverse behavior ownership and complete extraction are not established; this bounded audit cannot establish a global assurance PASS. | spec/spec.md |

## Coverage

Plan completion: not assessed.
Semantic review: skipped; optional whole-repository intent/test/code review was not requested.
Reconciliation: quire matrix (quire 0.52.0, engine 0.62.2) static fallback; no run evidence read.
quoin 0.36.7 matrix --repo /Users/peter/dev/qsl-workscope-pin-code --json was attempted once, exit 1: native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size. Fresh stdout/stderr are /private/tmp/qsl-workscope-pin-quoin-matrix.{json,stderr}. Approved current installed Quire static fallback then exited 0; no old binary, repeated refusal or grep coverage replacement.

Quire matrix returned 6257 criteria: tagged 4932; untagged 1139; tagged-by-ignored-test 65; method-without-symbol 106; retired 15; qualified-record 0. Counts are a current computed status census, not an assertion that every row has been semantically examined; notably some textually retired FR-001 rows remain classified untagged and are not silently reclassified here.

Quire coverage exited 0: 6015 untracked symbol records, 23038 diagnostics, Rust census 13949 candidates / 13373 tagged / 12833 bound. Diagnostic reasons: section-matches-nothing 6, trace-id-outside-marker 22471, uncatalogued-verification-method 22, untracked-id-has-minted-children 537, archetype-matches-nothing 1, catch-all-universal 1. Inspected section-miss diagnostics for NFR-001/NFR-003/NFR-004; they require document heading alignment, distinct from missing test tags. Current JSON and stderr retained as /private/tmp/qsl-workscope-pin-{quire-matrix,coverage}.{json,stderr}. Coverage has a different table population (totals 6186, backed 5009, criteria 6207); these are not mixed with matrix's 6257 denominator. Neither no-symbol methods nor retired rows are counted as runnable-test coverage. Evidence-store status unavailable after Quoin refusal.

Smallest examined defect units are enumerated below. FR-1044-AC-10 is untagged, not falsely described as an engine ignored-only binding: its source/package parity function is independently visibly ignored and its criterion mentions only appear as unmarked claims. FR-861-AC-4 and FR-875-AC-1 each have one engine binder, ignored=true. These clients are context for this pin; dependency availability does not satisfy them.

Reverse gap/stub inspection is bounded: inventory identified 885 files across fsl-eval/analyze/command/package/replay src, and read their crate boundaries and selected client tests. Whole-source todo!/unimplemented! scan produced 19 textual matches, inspected as compile-fail/doc examples or deliberate scanner fixture strings; none establishes a newly introduced live stub. Test scan exposes ignored client work and existing hybrid_requests todo! test, not a new pin implementation. No changed source/test can add a new stub, behavior, duplicate rule set, or coverage inflation; source and spec trees are byte-identical to base by full name-only diff. Full public-behavior-to-requirement mapping, all tiny/re-export modules, all test bodies and coverage inflation across the entire repository were not established. Known complete-mapping and extraction limitations are explicit FND-005 rather than a fabricated global PASS. No global count of untraced behaviors or source/test stubs is claimed.

Root's supplied 33 compatibility passes are checked receipts; no execution evidence was imported into the assurance store and no cumulative probability or observation-ledger acceptance is inferred. QSL-2145 and QSL-2158 remain open; QSL-2739 parity and payload/Clock follow-on work remain context. No new runtime guard, feature consumer or public JSON change is in this diff.

## Scope units

```yaml
[
  {
    "id": "FR-002-AC-9",
    "path": "spec/functional/FR-002-parse-native-units.md",
    "lines": "84",
    "role": "examined",
    "excerpt": "`fsl_cst::parse` over a unit of three protocols in which the second holds a missing closing brace inside its own span returns a recovered `ParsedSource` with the diagnostic attached to the second declaration, and the first and third declarations' CST equal their CST in the unit without the second (FR-503-AC-1)."
  },
  {
    "id": "FR-861-AC-4",
    "path": "spec/functional/FR-861-take-partial-repair-completeness-and-activation-authority-from-filament-observation.md",
    "role": "examined",
    "excerpt": "`fsl-eval`'s manifest names `filament-observation`, its source calls the owner's `partial`, `completeness` and `activation` modules, the progress ledger's source calls `completeness::check_revision`"
  },
  {
    "id": "FR-875-AC-1",
    "path": "spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md",
    "role": "examined",
    "excerpt": "Through `fsl analyze` over the `Link` model, the claim `E` of the use case with a start snapshot whose `msgs` population holds no object settles `inconclusive` with cause `no-instance`, never `proved` and never `unsupported`."
  },
  {
    "id": "FR-1044-AC-10",
    "path": "spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md",
    "lines": "420",
    "role": "examined",
    "excerpt": "Over `test/system-default`, a request with no `items` writes `Local` at index 0, `Sys` at 1, the subject's derived deadlock-freedom item at 2 (FR-620-AC-5), then the premise items `Bounded` at 3 and `Capped` at 4, and exits 21; over the unit's `package` handoff (FR-850) it writes the same items, records and exit code."
  },
  {
    "id": "repository-assurance-audit",
    "path": "spec/spec.md",
    "role": "examined",
    "excerpt": "component_type: rust-lib\nimplementation_language: rust"
  },
  {
    "id": "observation-test",
    "path": "fsl-eval/tests/it/observation_owner.rs",
    "role": "context_only",
    "excerpt": "#[ignore = \"QSL-2145: fsl-eval still carries its own partial, completeness, activation and repair rules\"]"
  },
  {
    "id": "probability-test",
    "path": "fsl-analyze/tests/it/probability_from_source.rs",
    "role": "context_only",
    "excerpt": "#[ignore = \"QSL-2158: source-route on-each probabilistic claims settle internal-failure (runtime_invariant, established-invariant-broken) before the expected per-instance verdict\"]"
  },
  {
    "id": "package-parity-test",
    "path": "fsl-analyze/tests/it/analyze_composition_cli.rs",
    "role": "context_only",
    "excerpt": "#[ignore = \"FR-1044-AC-10 stays OPEN with terra:QSL:package-consumer-api: the package route has no composition form\"]"
  }
]
```

## Bindings examined

- FR-861-AC-4 -> fsl-eval/tests/it/observation_owner.rs:25::fsl_eval_calls_the_observation_owner_for_its_four_authority_rule_sets (ignored; trace identity correct, execution absent).
- FR-875-AC-1 -> fsl-analyze/tests/it/probability_from_source.rs:682::probabilistic_on_each_empty_population (ignored; trace identity correct, execution absent).
- FR-1044-AC-10 -> no engine binder (unmarked claims at analyze_composition_cli.rs:1081 and :1578); no trace-binding PASS claimed.

## Dispositions

No semantic fix round ran. These are baseline scope dispositions explicitly authorized in the dispatch, not evidence that the client defects were fixed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | Unchanged repository traceability baseline. Root's explicit scoped engineering override accepts this existing baseline for the separately assigned dependency readiness chore; no global assurance or parser acceptance claim. |
| FND-002 | deferred | Existing QSL-2145 / architecture ledger switch3363 remains open and must implement and qualify owner calls after the actual merged dependency base. No client completion claimed. |
| FND-003 | deferred | Existing QSL-2158 remains open for cumulative probability implementation and qualification after the actual merged dependency base. This pin enables coherent APIs only. |
| FND-004 | deferred | Existing QSL-2739 tracks FR-1044-AC-10 Source/Package composition parity. It remains unmet; the pin chore neither changes the routes nor claims acceptance. |
| FND-005 | accepted-no-change | Root's explicit scoped engineering override accepts unchanged repository assurance limits for this pin. No complete reverse ownership inventory, extraction repair, evidence-store PASS or exhaustive cleanup is claimed. |

## Publication and custody

Private pending method and baseline-disposition comments prepared outside the worktree. NOT POSTED: no tracker reads/writes, comment ids, labels or read-back claimed. Root owns central publication, actual byte/YAML read-back, and latest artifact commit to reviews/qsl-2145-workscope-pin-gap-analysis.md. No source edits or new worktree. No private tracker links in artifact.

