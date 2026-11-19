---
id: SR-2026111612172400000003
title: "code-review of QSL-1968 README profile correction"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-1968; README.md; examples/config-version/spine_unit.rs; docs/images/first-rule-*.svg; fsl-semantics/tests/it/; tests/it/doc_snippets.rs"
review_set: subset
---

## Summary

Reviewed QSL-1968 PR #2754 at 30157f74efd99d41a03db9999eea4e1cb5310be6. The profile correction and both existing SVG profile glyph sequences agree with the canonical unit, but the ticket's floating-point correction is missing.

## Verdict

**PASS (disposition pass 1)** — original FND-001 is fixed at 7069d2d730845a06fb0b0d76e2133570db17413d; no new code-review findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The README still states that there is no floating point, contrary to the implemented Float32/Float64 support and QSL-1968's explicit acceptance criterion. Replace that sentence with the supported float types and rounding modes. | README.md:120-121 |

## Examined scope

```yaml
[
  {
    "id": "README.md",
    "path": "README.md:34",
    "role": "examined",
    "excerpt": "profile v = \"filament.model.complete/v1\";"
  },
  {
    "id": "README.md",
    "path": "README.md:52",
    "role": "examined",
    "excerpt": "`filament.model.complete/v1` covers values, object graphs and model operations."
  },
  {
    "id": "README.md",
    "path": "README.md:120",
    "role": "examined",
    "excerpt": "**Values and functions.** Bounded integers, exact rationals, records, unions\nand pure functions. There is no floating point."
  },
  {
    "id": "QSL-1968",
    "path": "https://linear.app/agent-ix/issue/QSL-1968/language-readme-first-rule-is-refused-valuecomplete-profile#comment-af2a079e",
    "role": "examined",
    "excerpt": "The README's floating-point sentence is replaced by a statement of the float types and rounding modes supported."
  },
  {
    "id": "FR-361",
    "path": "spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md",
    "role": "context_only",
    "excerpt": "Selecting the model layer admits the forms of the layers its\n   `requires` closure names (FR-110 \"Layer selection\"). A unit that selects\n   the model profile therefore needs no second header profile to use\n   value-complete forms."
  },
  {
    "id": "examples/config-version/spine_unit.rs",
    "path": "examples/config-version/spine_unit.rs:32",
    "role": "examined",
    "excerpt": "profile v = \\\"filament.model.complete/v1\\\";"
  },
  {
    "id": "docs/images/first-rule-light.svg",
    "path": "docs/images/first-rule-light.svg:21",
    "role": "examined",
    "excerpt": "<use href=\"#mono_m\" x=\"6000\"/><use href=\"#mono_o\" x=\"6600\"/><use href=\"#mono_d\" x=\"7200\"/><use href=\"#mono_e\" x=\"7800\"/><use href=\"#mono_l\" x=\"8400\"/>"
  },
  {
    "id": "docs/images/first-rule-dark.svg",
    "path": "docs/images/first-rule-dark.svg:21",
    "role": "examined",
    "excerpt": "<use href=\"#mono_m\" x=\"6000\"/><use href=\"#mono_o\" x=\"6600\"/><use href=\"#mono_d\" x=\"7200\"/><use href=\"#mono_e\" x=\"7800\"/><use href=\"#mono_l\" x=\"8400\"/>"
  }
]
```

## Coverage

README-only diff. Language-independent code-review and docs-style checks applied. No applicable AssuranceProfile found. Rust review skipped because no Rust/Cargo file changes; spec review skipped because no spec/plan changes. No builds, tests, or aggregate gate run, per the brief. Existing Python figure tooling and docs lint were not run under the repository language policy. No new code, duplication, or vendoring introduced.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | README.md:121-122 now names Float32 and Float64 and the six supported modes. |

## Disposition pass 1 evidence

Reviewed 7069d2d730845a06fb0b0d76e2133570db17413d against the original findings. The original review artifacts were copied verbatim into reviews/ before this pass (cmp passed). The added Rust tests now also received the rust-review lane with the rust-style testing checklist: public S1/S2/model-intake/assembly/checking paths, active integration-module registration, exact checked clause names/kinds, nonempty typed negative refusal assertions, and deterministic embedded fixture/model input. The fixed literal copy is explicitly required by the ticket and compared with the extracted README text. No production, CI, unsafe, concurrency, or wire-contract changes. No builds/tests/gate run per the brief; this is source review, not fresh execution evidence. The trace mismatch is recorded once in the gap-analysis artifact.

```yaml
{
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "after_excerpt": "and pure functions. `Float32` and `Float64` support `exact`, `nearest-even`,\n`nearest-away`, `toward-zero`, `toward-positive` and `toward-negative` rounding modes."
    }
  ]
}
```

