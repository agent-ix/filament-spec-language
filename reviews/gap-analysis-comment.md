<!-- reviewer repo=agent-ix/filament-spec-language visibility=public quoin=0.36.7 module=spec-artifacts-process@v0.36.0 id=SR-2026111911313600000004 method=gap-analysis lang=rust pr=filament-spec-language#3022 model=gpt-6.2-astra run=0dc1051f-f5a5-4865-8d1c-c4d34c2e75cd -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | high | soundness | The emitted plan node lacks the checked clause, claim-occurrence, method and domain binding identities required by FSpec FR-192. |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-753-AC-3, path: "spec/functional/FR-753-build-check-emit-and-execute-a-verification-plan.md", role: examined, excerpt: "The unit's checked package identity is equal with and without Release; S4 emits its plan document with three nodes and one `requires` edge."}
  - {id: FR-753-AC-5, path: "spec/functional/FR-753-build-check-emit-and-execute-a-verification-plan.md", role: examined, excerpt: "a plan document whose `package` is another identity refuses before any step, naming both identities, with `max_trace_length` `Some(8)` and with `None` alike, the identity mismatch being checked first."}
  - {id: FSpec-FR-192, path: "filament-specification/spec/functional/analysis/FR-192-execute-linked-verification-plans.md", role: examined, excerpt: "nodes: one entry per step, in source order, with its step name, the bound clause's node identity and claim occurrence in that package, the method, its FR-290 kind, extent and evidence kind, the resolved domain's declaration key, and the bound or its absence;"}
  - {id: TC-2103, path: "spec/test-cases/TC-2103-a-verify-plan-builds-checks-emits-and-executes.md", role: examined, excerpt: "Step 3: equal package identities; a document with three nodes and one `requires` edge."}
  - {id: fsl-package/src/lib.rs, path: "fsl-package/src/lib.rs", role: examined, excerpt: "Exports the verification-plan emitter, reader, wire types, refusal and format identifier."}
  - {id: fsl-package/src/verification_plan.rs, path: "fsl-package/src/verification_plan.rs", role: examined, excerpt: "The node wire form holds only the step name, claim name, method name, domain name and bound."}
  - {id: fsl-package/tests/verification_plan.rs, path: "fsl-package/tests/verification_plan.rs", role: examined, excerpt: "Tests package identity stability, the emitted plan graph, and strict package-identity mismatch refusal."}
  - {id: fsl-package/tests/fixtures/verification-plan-model.semantic-ir.json, path: "fsl-package/tests/fixtures/verification-plan-model.semantic-ir.json", role: examined, excerpt: "Provides the selected `acme/orders` model package used to build the checked plan test."}
  - {id: fsl-semantics/src/check/mod.rs, path: "fsl-semantics/src/check/mod.rs", role: examined, excerpt: "Retains the S3-checked verification plan forms in `CheckedGraph` and exposes them to S4."}
findings:
  - fnd: FND-001
    severity: high
    confidence: high
    method: gap-analysis
    check: soundness
    artifact_id: FSpec-FR-192
    related_ids: [FR-753-AC-3]
    path: fsl-package/src/verification_plan.rs
    lines: "33-45, 94-108"
    excerpt: |-
      nodes: one entry per step, in source order, with its step name, the bound clause's node identity and claim occurrence in that package, the method, its FR-290 kind, extent and evidence kind, the resolved domain's declaration key, and the bound or its absence;
    finding: "The emitted node contains claim/method/domain names only; it omits the checked clause and claim-occurrence identities, method semantics, and domain declaration key required by FSpec FR-192. The document therefore cannot establish the specified checked bindings for a consumer."
bindings:
  - {test_id: emission_keeps_package_identity_and_writes_the_plan_graph, ac_id: FR-753-AC-3, trace: correct}
  - {test_id: strict_reader_refuses_a_plan_bound_to_another_package, ac_id: FR-753-AC-5, trace: correct}
```

+++
