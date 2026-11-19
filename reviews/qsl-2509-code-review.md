---
id: SR-2026111911313600000003
title: "Code review — QSL-2509 verification plan emitter and reader"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language PR #3022@570680d8fc6ab45ce7ead59f20786bd654a07335; fsl-package/src/lib.rs; fsl-package/src/verification_plan.rs; fsl-package/tests/fixtures/verification-plan-model.semantic-ir.json; fsl-package/tests/verification_plan.rs; fsl-semantics/src/check/mod.rs; FR-753-AC-3, FR-753-AC-5; FSpec FR-192 Package form; TC-2103"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-753", type: reviews }
  - { target: "ix://agent-ix/filament-spec-language/TC-2103", type: reviews }
---

## Summary

Reviewed PR #3022 at `570680d8fc6ab45ce7ead59f20786bd654a07335`. The focused test exercises identity stability, three emitted nodes, the dependency edge, and package-mismatch refusal. The emitted document shape omits several bindings required by the linked FR-192 package form, so downstream consumers cannot interpret it as the specified linked plan.

## Verdict

**FAIL** — the emitted wire document omits required clause and method binding identities and does not conform to FSpec FR-192's package form.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The emitter serializes only the claim, method, and domain names (`verification_plan.rs:102-108`), while FR-192 Package form requires each node to carry the bound clause node identity and claim occurrence, the method's FR-290 kind/extent/evidence kind, and the resolved domain declaration key; the document also omits the plan source location. As a result this `filament.verification-plan/v1` document is not the specified linked plan and a consumer cannot verify which checked clause and method contract each node refers to. Emit the complete bound identities and method facts from S3's checked plan form and add assertions for these fields. | fsl-package/src/verification_plan.rs:102-108; FSpec FR-192 Package form |

## Coverage

- Rust review: public API, serialization contract, checked-plan projection, strict-reader refusals, edge validation, test assertions, and error paths inspected. No CI workflow files changed. No applicable AssuranceProfile was found for this S4 emitter/reader scope.
- Focused test: `cargo test -p fsl-package --test verification_plan` through `locked-build.sh` passed (2/2; `Finished `test` profile [unoptimized] target(s) in 18m 34s`).
- Test gate: `cargo test -p fsl-package --test verification_plan` through `locked-build.sh` passed (2/2; 0 failures).
- Matrix: FR-753-AC-3 and FR-753-AC-5 are statically tagged; Quoin reports no run evidence for either criterion.
- Optional style notes: none.

## Examined scope

- {id: FR-753-AC-3, path: "spec/functional/FR-753-build-check-emit-and-execute-a-verification-plan.md", role: examined, excerpt: "S4 emits its plan document with three nodes and one `requires` edge."}
- {id: FR-753-AC-5, path: "spec/functional/FR-753-build-check-emit-and-execute-a-verification-plan.md", role: examined, excerpt: "a plan document whose `package` is another identity refuses before any step, naming both identities, with `max_trace_length` `Some(8)` and with `None` alike, the identity mismatch being checked first."}
- {id: FSpec-FR-192, path: "filament-specification/spec/functional/analysis/FR-192-execute-linked-verification-plans.md", role: context_only, excerpt: "Each node carries the bound clause's node identity and claim occurrence, the method's FR-290 kind, extent and evidence kind, and the resolved domain declaration key."}
- {id: fsl-package/src/verification_plan.rs, path: "fsl-package/src/verification_plan.rs", role: examined, excerpt: "The node wire form holds only the step name, claim name, method name, domain name and bound."}
- {id: fsl-package/src/lib.rs, path: "fsl-package/src/lib.rs", role: examined, excerpt: "Exports the verification-plan emitter, reader, wire types, refusal and format identifier."}
- {id: fsl-package/tests/fixtures/verification-plan-model.semantic-ir.json, path: "fsl-package/tests/fixtures/verification-plan-model.semantic-ir.json", role: examined, excerpt: "Provides the selected `acme/orders` model package used to build the checked plan test."}
- {id: fsl-semantics/src/check/mod.rs, path: "fsl-semantics/src/check/mod.rs", role: examined, excerpt: "Retains the S3-checked verification plan forms in `CheckedGraph` and exposes them to S4."}
- {id: fsl-package/tests/verification_plan.rs, path: "fsl-package/tests/verification_plan.rs", role: examined, excerpt: "The focused test checks identity stability, node and edge counts, and package mismatch refusal."}
