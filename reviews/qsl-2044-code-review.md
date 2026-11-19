---
id: SR-2026111919340478491681407
title: "Code and Rust review of QSL-2044 source corpus"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2044; PR 3094; tests/corpus/refinement/**; tests/it/refinement_source_routes.rs; additive tests/it/main.rs"
review_set: subset
---

## Summary

Reviewed frozen head 320cb578875c6b25841372d97d395e72fc15ca19: original refinement/hyperproperty source programs, strict compile/verdict expectations and public command tests. No findings in this partial corpus slice.

## Verdict

**PASS for the changed slice.** QSL-2044 remains incomplete. Six hyper routes and two diagnostic assertions remain pending under QSL-2128, QSL-2594 and QSL-2595; they are not counted as passing behavior evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Includes the Rust/style lane. No separate Rust artifact. Repository AGENTS/CLAUDE/CONTRIBUTING and applicable Rust implementation/testing conventions examined.

Examined all 72 changed files. The sixteen compile programs exercise successful admission and independent declaration/type/name refusals through the existing strict corpus runner. Three active analysis cases distinguish certified identity from refuted stutter and constant mappings; the separate active diagnostic test checks missing step/object/operation names and distinct messages. The shared analysis helper requires the named claim, exact terminal result and exit, certified proof or non-null counterexample. It exercises production public compilation/analysis with the established negotiation port double; it does not replace the engine under test. Existing same-repository Counter model and initial snapshot are referenced, not copied. New inputs are original test sources. No dependencies, CI gates, production API, unsafe code, model copies, compatibility behavior or permissive readers are added.

Accepted coder-reported focused evidence: sixteen compile programs passed; four active behavior tests validated across runs (three changed tests plus the unchanged previously passed stutter test). No reviewer tests, builds, gates, discovery or matrix repeated. Lead owns one final applicable gate. Ignored tests preserve intended outcomes and explicit owner IDs rather than accepting current misses. Additional copy-swap, projection-alignment and finite-trace cases remain ticket scope, not a finding in this explicitly partial slice.

No applicable AssuranceProfile in the established repository context. Spec review is inapplicable: no spec or plan changes. This artifact makes no whole-ticket, whole-repository or independent replay/certificate-verifier coverage claim.

## Examined Scope

```json
{
  "scope": [
    {
      "id": "FR-135-AC-2",
      "path": "spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md",
      "role": "examined",
      "excerpt": "Refusals, each at its span with no `CheckedRefinement`: the abstract alias `Nope`, resolving to no model (`missing_declaration`/`missing-name`); the `population` row removed (`missing_declaration`/`missing-name` naming `Spec::counters`);"
    },
    {
      "id": "FR-136-AC-2",
      "path": "spec/functional/FR-136-check-a-refinement-s-step-rows.md",
      "role": "examined",
      "excerpt": "Removing the `peek` row refuses `missing_declaration`/`missing-name` naming `Impl::Counter::peek`; writing `peek` twice, once `-> stutter` and once `-> any`, refuses `invalid_model_binding`/`conflicting-binding` naming both rows. A row `step Impl::Counter::nope -> stutter` refuses `missing_declaration`/`missing-name`."
    },
    {
      "id": "FR-144-AC-7",
      "path": "spec/functional/FR-144-request-and-settle-a-refinement-item.md",
      "role": "examined",
      "excerpt": "`CasRefinesCounter` with its `ensure` row removed settles `proved`, `closed-scope`, `Certified`, after FR-149 accepts the certificate; with its `ensure` row it settles `proved`, `Uncertified`; with the certificate's `Taken` for the `commitA` edge from `(0, 0, t, 0, f)` changed to `Stutter`, it settles `inconclusive`, `CertificateRejected`."
    },
    {
      "id": "FR-142-AC-3",
      "path": "spec/functional/FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md",
      "role": "examined",
      "excerpt": "The compare-and-set model with concrete initial `value` 1 returns `Violated`, `InitialNotAbstract{initial: 0}`, empty prefix. With `commitA` mapped `-> stutter` it returns `Violated` with prefix `beginA`, `commitA` and `StutterChanged{position: 2}`."
    },
    {
      "id": "FR-612-AC-1",
      "path": "spec/functional/FR-612-refuse-hyper-only-expression-forms-in-a-single-trace-temporal-clause.md",
      "role": "examined",
      "excerpt": "A non-`hyper` temporal clause over `v: V::ConfigVersion` with formula `always holds(v.versionNumber @ a = 0)` refuses `unsupported_construct`/`expression-form` at `v.versionNumber @ a`, with a message containing `hyper`; with `always holds(a.step.mix)` it refuses the same way at `a.step.mix`."
    },
    {
      "id": "FR-182-AC-1",
      "path": "spec/functional/FR-182-settle-a-hyper-verdict.md",
      "role": "context_only",
      "excerpt": "Each input row maps to its `TerminalValue`, FR-360 label, FR-243 basis and O-16 category exactly as the table states, `MatchUndetermined` and `VacuousMatch` each to inconclusive."
    },
    {
      "id": "tests/it/refinement_source_routes.rs",
      "path": "tests/it/refinement_source_routes.rs",
      "role": "examined",
      "excerpt": "Identity must settle proved with certification certified; stutter and constant mappings must settle refuted with exit 10 and a non-null counterexample. Six hyper verdict tests and two diagnostic assertions retain named-owner ignore reasons."
    },
    {
      "id": "tests/corpus/refinement/**",
      "path": "tests/corpus/refinement/",
      "role": "examined",
      "excerpt": "Seventeen original native programs, sixteen active compile expectations, nine public analysis requests, strict verdict expectations and the pending indexed-read boundary case. Uses existing same-repository Counter model and v0 snapshot by reference."
    }
  ],
  "clean": true,
  "findings": [],
  "bindings": []
}
```

