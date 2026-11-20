---
id: SR-1474
title: "QSL-2078 protocol near-miss acceptance gaps"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2078; fsl-cst/tests/it/production_protocol_forms.rs; frozen branch 0f229970fdb6a89778b8ded28d8baee1c42619b4"
review_set: subset
---

## Summary

Reviewed only the frozen QSL-2078 additions to fsl-cst/tests/it/production_protocol_forms.rs, target branch codex/qsl-2078-protocol-near-misses at 0f229970fdb6a89778b8ded28d8baee1c42619b4, before PR opening. This is the one formal feature review; no PR was opened or modified. Plan completion: not assessed.

## Verdict

**PASS** — no findings in the changed test code or its scoped requirement bindings. This is not a repository-wide assurance or final-gate claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

The six new test functions exercise 12 admitted counterparts and 24 nonlexical near-misses across compensation, repetition, await, event nodes and join policy, including three empty-roster policies. The tests call the real public CST parser, assert typed code/cause and exact byte spans separately from expected diagnostic alternatives, require one diagnostic and lossless malformed-source rendering, and identify the intended productions in admitted counterparts. The existing CRLF/Unicode source wrapper makes byte offsets distinct from character offsets. The changed helper returns the existing parsed result for these extra assertions; existing callers retain their original checks. No production code, fixture copies, registration changes, stubs, guarded success-only assertions, or ignored cases were introduced.

The base protocol grammar and complete-V1 overrides were compared at the manifest-pinned FSpec revision f32bca6c7937e0652742b2bbe27a52419bf5a0ae: compensation, repetition/loop-proof, await-control, event-node/memory-order, parallel/branch/join-policy and the nonempty roster rule. The ordinary grammar errors are not replaced by lexical `#` mutations. Existing fence and `all []` tests were accepted without rerunning or auditing them. No later protocol engine capability is claimed.

Author-reported focused checks and scoped Clippy passed and were accepted without repetition. The final applicable component gate remains the original coder's next stage. Current unrelated CST gate failures belong to PR3146 and were outside this review.

Matrix computation, repository inventory and peer-result remeasurement were expressly excluded by the user and dispatch; this is a diff-scoped acceptance/trace/reverse-gap/stub analysis, not a full repository gap audit.

Plan completion: not assessed.

## Examined scope

```json
[
  {
    "id": "FSpec-FR-339-AC-1",
    "path": "spec/functional/source-package/FR-339-define-complete-native-grammar.md",
    "role": "examined",
    "excerpt": "Every production in the base and complete-V1 facet grammar has positive, boundary and located-negative corpus vectors."
  },
  {
    "id": "FSpec-FR-339-AC-2",
    "path": "spec/functional/source-package/FR-339-define-complete-native-grammar.md",
    "role": "examined",
    "excerpt": "Parsing and lossless rendering reproduce accepted UTF-8 source bytes exactly, including comments and whitespace."
  },
  {
    "id": "FSpec-FR-339-AC-4",
    "path": "spec/functional/source-package/FR-339-define-complete-native-grammar.md",
    "role": "examined",
    "excerpt": "An undeclared keyword, operator or extension refuses at its exact source span without producing a checked declaration."
  },
  {
    "id": "FR-002-AC-6",
    "path": "spec/functional/FR-002-parse-native-units.md",
    "role": "examined",
    "excerpt": "The bounded malformed/truncation corpus returns a `ParsedSource` or a located diagnostic from `fsl_cst::parse` without panic; any admissible variant retains its exact submitted bytes."
  },
  {
    "id": "FR-762",
    "path": "spec/functional/FR-762-report-every-independent-syntax-error-of-a-unit.md#one-diagnostic-per-error",
    "role": "context_only",
    "excerpt": "A unit with exactly one syntax error SHALL give exactly the one diagnostic, cause, locus and message it gives without recovery."
  },
  {
    "id": "FSpec/shared-grammar/protocol",
    "path": "proposals/filament-v1/shared-grammar.md",
    "role": "examined",
    "excerpt": "loop-proof    = 'invariant', block, 'variant', block ;"
  },
  {
    "id": "FSpec/shared-grammar/join",
    "path": "proposals/filament-v1/shared-grammar.md",
    "role": "examined",
    "excerpt": "A `join` list names every branch exactly once and is never empty: `join all []` is `invalid_syntax`/`unexpected-token` at the `]`."
  }
]
```
