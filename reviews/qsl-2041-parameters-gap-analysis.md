---
id: SR-2026111919340478491681402
title: "gap-analysis of QSL-2041 operation parameter corpus"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2041 PR 3078; tests/it/source_corpus.rs; tests/corpus/operations/parameters/**"
review_set: subset
---

## Summary

Frozen head 0e5fa6a91aeede0303f179220f8c88c23e3350e1. Diff-scoped gap analysis inspects requirement ownership, meaningful evidence and honest exclusions; the supplied matrix is reused. No defects found in this delivered slice.

## Verdict

**PASS for the reviewed diff.** Ready for the lead's final applicable gate. This is neither full-ticket completion nor a repository-wide assurance PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

The reviewed diff adds tests/it/source_corpus.rs staging and tests/corpus/operations/parameters/** only. It does not change production behavior, dependencies, CI, schemas, public APIs, unsafe code, or concurrency. The scoped Rust review considered ownership, error propagation, visibility, deterministic staging, test-only failure paths, and fixture provenance. No new framework or compatibility layer is introduced.

The staging helper reads existing same-repository models and applies original population declarations plus small original Workflow operations. It does not copy a complete model or alter shared fixtures. Model admission runs before the runnable cases. Switch.set is returnless with Boolean flag; Workflow.apply uses Amount, a Quantity<U> alias bounded 1..20, and a Boolean result. Total is also Quantity<U>. The original probe uses Tally, a true scalar Integer bounded 0..5; inspectPeer declares a Workflow reference. Optional metadata is explicitly optional, non-nullable, no default, multiplicity 1..1 and is preserved as versioned pending evidence.

The cases execute the public command route through the existing strict native-run-result reader and mismatch assertions. Boolean true/false pairs, Integer threshold/boundary and result-difference pairs, Quantity rational 3/2 and bounds, and reference self/other/inconsistent pairs make wrong implementation results observable. Missing/extra/wrong-kind parameters, invalid results, dangling/wrong-type references, absent pre state, allowed/forbidden field changes, and created/deleted delta disagreements have concrete lifecycle/category/code/cause oracles. No malformed wire is accepted as success.

Accepted focused peer evidence: 85 runnable programs passed, comprising 72 run and 13 compile, with four model admissions; formatting passed. Log: /home/peter/dev/.agent/qsl-2041-parameters-corrected.log. No test, build, gate, or matrix was repeated. The lead owns the final applicable gate on the reviewed head.

This is a partial QSL-2041 delivery. Pending Decimal cases do not count as coverage. QSL-2583 owns the additional native Decimal checked-type and scalar Decimal reader facets grounded in FR-056 AC-66/68; completed QSL-2451 is related context. Legacy Meter bounds and reserved-word parameter spelling are distinct pending evidence. QSL-1983's Done state is not relabelled as a current-main regression by the versioned optionalN reproducer. Global wire defects remain owned by QSL-2575/QSL-2184.

The supplied existing matrix baseline (5808 total, 4323 tagged, 308 ignored-only, 1074 untagged, 103 method-without) is repository context only, not a fresh computation or proof that these programs cover every criterion. Bare FR trace tags follow the existing area-harness convention and establish requirement ownership, not exhaustive AC completion. No unrelated repository gaps are claimed closed. No applicable AssuranceProfile was selected in the lead's existing context.

Plan completion: not assessed

## Examined Scope

```json
{
  "scope": [
    {
      "id": "stage_parameter_models",
      "path": "tests/it/source_corpus.rs",
      "role": "examined",
      "excerpt": "fn stage_parameter_models(directory: &Path, fixtures: &Path) -> Result<(), String> {"
    },
    {
      "id": "corpus_operation_parameters",
      "path": "tests/it/source_corpus.rs",
      "role": "examined",
      "excerpt": "#[trace(\"FR-104\", \"FR-106\", \"FR-107\")]\n#[test]\nfn corpus_operation_parameters() {"
    },
    {
      "id": "boolean-parameters",
      "path": "tests/corpus/operations/parameters/boolean-post-true.native",
      "role": "examined",
      "excerpt": "pre Before using v on M::Switch::set { flag }\npost After using v on M::Switch::set { self.on = flag }"
    },
    {
      "id": "integer-parameters",
      "path": "tests/corpus/operations/parameters/integer-pre-threshold.native",
      "role": "examined",
      "excerpt": "pre Before using v on M::Workflow::probe { n <= 3 }\npost After using v on M::Workflow::probe { result = n }"
    },
    {
      "id": "quantity-parameters",
      "path": "tests/corpus/operations/parameters/quantity-pre-fraction.native",
      "role": "examined",
      "excerpt": "pre Before using v on M::Workflow::apply { amount <= 10 U }\npost After using v on M::Workflow::apply { result and amount <= 10 U }"
    },
    {
      "id": "reference-parameters",
      "path": "tests/corpus/operations/parameters/reference-post-self.native",
      "role": "examined",
      "excerpt": "pre Before using v on M::Workflow::inspectPeer { peerRef = self }\npost After using v on M::Workflow::inspectPeer { result = (peerRef = self) }"
    },
    {
      "id": "pending-exclusions",
      "path": "tests/corpus/operations/parameters/pending/README.md",
      "role": "examined",
      "excerpt": "These desired outcomes are excluded from the runnable parameter directory. They do not contribute passing coverage. Evidence was produced from FSL base `f2b6c21a3` with the strict native-run-result reader."
    },
    {
      "id": "FR-104-AC-2",
      "path": "spec/functional/FR-104-check-state-clauses.md",
      "role": "context_only",
      "excerpt": "`post R using v on Config::ConfigVersion::attemptUpdate { result }` checks with `result: Boolean`. `result` in an invariant, and in a `pre` clause of `attemptUpdate`, each refuse `wrong_snapshot`/`wrong-anchor` at `result`."
    }
  ]
}
```

