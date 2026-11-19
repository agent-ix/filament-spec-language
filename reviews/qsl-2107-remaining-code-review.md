---
id: SR-1421
title: "Code and Rust review of remaining executable parser forms"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2107; PR3118; fsl-cst/tests/it/production_{value,protocol,analysis}_forms.rs and additive main.rs registration"
review_set: subset
---

## Summary

Reviewed frozen head 75e918d1b104fbd772e032b60f9e30c3e156078c for QSL-2107 / filament-spec-language#3118. This is a partial parser-production test slice, not an exhaustive grammar or repository assurance claim.

## Verdict

**CONDITIONAL:** one medium finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Eight protocol tests falsely bind the header-only-unit criterion FSpec-FR-339-AC-6. | fsl-cst/tests/it/production_protocol_forms.rs:145,257,385,602,614,641,650,662 |

The eight attributes at the listed lines attach AC-6 to channel, compensation, await and malformed protocol tests. AC-6 is specifically the header-only/profile-only admission and empty-package check/compile criterion. Every affected test supplies a complete protocol declaration, so all eight can pass while AC-6 is broken. Remove AC-6 from these attributes and retain the applicable AC-1/AC-2/AC-4 tags. Their behavior assertions otherwise exercise real parser paths; no runtime assertion needs to change.

## Coverage

Reviewed all three new modules and their registration diff. Every active case invokes the public parser and checks concrete CST production/member attachment, child order, source slices, precedence or located refusal data. The protocol and analysis admission helpers also assert lossless whole-source rendering; value tests assert rendered selected nodes. Malformed cases require a refusal and its exact first diagnostic. Authored UTF-8, CRLF and non-ASCII trivia exercise byte loci. Test-helper indexing and panics are assertion failures on broken parser output, not new production panic paths. No product API, dependency, unsafe code, task lifecycle, wire format, CI lane or source/schema/grammar copy is added. Existing harness/module conventions are used. No applicable AssuranceProfile is supplied in the established repository context; spec review is inapplicable because the diff changes tests only.

Accepted the coder-reported 65 distinct runnable new passes across focused runs (27 value/graph/binding/aggregate/verification, 16 protocol, 22 analysis), scoped fsl-cst Clippy pass, and four explicitly pending tests owned by QSL-2620, QSL-2621, QSL-2626 and QSL-2235. These are reported execution facts, not reviewer reruns or an all-production/all-alternative claim. The disputed empty-alignment row below must not be treated as settled intended-behavior coverage. No tests, builds, matrices, prior 16/21/200-case groups or root gates were repeated. The lead owns one final applicable gate.

Plan completion: not assessed.

## Examined Scope

```json
{
  "clean": false,
  "scope": [
    {
      "id": "FSpec-FR-339-AC-1",
      "path": "filament-specification/spec/functional/source-package/FR-339-define-complete-native-grammar.md",
      "role": "examined",
      "excerpt": "Every production in the base and complete-V1 facet grammar has positive, boundary and located-negative corpus vectors."
    },
    {
      "id": "FSpec-FR-339-AC-2",
      "path": "filament-specification/spec/functional/source-package/FR-339-define-complete-native-grammar.md",
      "role": "examined",
      "excerpt": "Parsing and lossless rendering reproduce accepted UTF-8 source bytes exactly, including comments and whitespace."
    },
    {
      "id": "FSpec-FR-339-AC-4",
      "path": "filament-specification/spec/functional/source-package/FR-339-define-complete-native-grammar.md",
      "role": "examined",
      "excerpt": "An undeclared keyword, operator or extension refuses at its exact source span without producing a checked declaration."
    },
    {
      "id": "FSpec-FR-339-AC-6",
      "path": "filament-specification/spec/functional/source-package/FR-339-define-complete-native-grammar.md",
      "role": "examined",
      "excerpt": "`language \"ix:native\" edition \"1-draft\";` alone, and the same header followed by `profile v = \"filament.value.complete/v1\";`, each parse with no `invalid_syntax` diagnostic and format unchanged; checking or compiling either refuses `missing_declaration`/`empty-package` at the `language` header, exit 20, with no checked package."
    },
    {
      "id": "FR-172-AC-3",
      "path": "spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md",
      "role": "examined",
      "excerpt": "§13's clause with `align skip { V::Vault::mix }` checks with `align_skip` holding `mix`; `align skip { }`, `align skip { V::Vault::mix, V::Vault::mix }`, `align skip { N::Device::tick }` in a clause over `V` alone, and `align skip` with `fair weak V::Vault::step` each refuse `unsupported_construct`/`expression-form`."
    },
    {
      "id": "align-skip",
      "path": "filament-specification/proposals/filament-v1/shared-grammar.md",
      "role": "context_only",
      "excerpt": "An `align skip` list\n  names at least one operation; `align skip {}` is\n  `invalid_syntax`/`unexpected-token` at the `}`."
    },
    {
      "id": "production_value_forms",
      "path": "fsl-cst/tests/it/production_value_forms.rs",
      "role": "examined",
      "excerpt": "//! Executable value, dimension, graph, hybrid and implementation-binding forms."
    },
    {
      "id": "production_protocol_forms",
      "path": "fsl-cst/tests/it/production_protocol_forms.rs",
      "role": "examined",
      "excerpt": "//! Protocol productions preserve policy children, scoped binders and source locations."
    },
    {
      "id": "production_analysis_forms",
      "path": "fsl-cst/tests/it/production_analysis_forms.rs",
      "role": "examined",
      "excerpt": "//! Analysis declarations retain their grammatical roles, source order and spans."
    },
    {
      "id": "production_analysis_forms-registration",
      "path": "fsl-cst/tests/it/main.rs",
      "role": "examined",
      "excerpt": "mod production_analysis_forms;"
    },
    {
      "id": "production_protocol_forms-registration",
      "path": "fsl-cst/tests/it/main.rs",
      "role": "examined",
      "excerpt": "mod production_protocol_forms;"
    },
    {
      "id": "production_value_forms-registration",
      "path": "fsl-cst/tests/it/main.rs",
      "role": "examined",
      "excerpt": "mod production_value_forms;"
    }
  ],
  "findings": [
    {
      "fnd": "FND-001",
      "severity": "medium",
      "confidence": "high",
      "method": "code-review",
      "check": "trace",
      "artifact_id": "FSpec-FR-339-AC-6",
      "related_ids": [
        "FSpec-FR-339-AC-1",
        "FSpec-FR-339-AC-2",
        "FSpec-FR-339-AC-4"
      ],
      "path": "fsl-cst/tests/it/production_protocol_forms.rs",
      "lines": "145,257,385,602,614,641,650,662",
      "excerpt": "`language \"ix:native\" edition \"1-draft\";` alone, and the same header followed by `profile v = \"filament.value.complete/v1\";`, each parse with no `invalid_syntax` diagnostic and format unchanged; checking or compiling either refuses `missing_declaration`/`empty-package` at the `language` header, exit 20, with no checked package.",
      "finding": "Eight protocol tests falsely bind the header-only-unit criterion FSpec-FR-339-AC-6."
    }
  ],
  "bindings": []
}
```

## Dispositions

Round 1 at fix head 1b6797e6527242cf2a87eba5f10989f7bade31b1: **PASS**. Checked only this method's finding fix; no tests, builds, matrix, metadata lookup or review sweep was repeated.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The eight incorrect FSpec-FR-339-AC-6 tags were removed; the applicable AC-1/AC-2/AC-4 tags and behavior assertions remain unchanged. |

