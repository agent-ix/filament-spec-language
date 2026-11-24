<!-- reviewer repo=agent-ix/filament-spec-language visibility=public quoin=0.36.10 module=spec-artifacts-process@v0.29.2 id=SR-96721 method=gap-analysis lang=rust pr=filament-spec-language#3643 model=claude-opus-5-5 run=5D2C7471-BBB6-467C-9119-31933453A71A -->

Head 5cd78f803302021434588d539b67e8604125c09c. Manual AC-to-test check over the diff (quoin matrix not run: read-only, no builds).

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | coverage | FR-141-AC-16 FastQueue/SlowQueue receiver clause has no test; tag on the node-row test inflates AC-16 coverage |
| FND-002 | medium | coverage | FR-140-AC-18 (SubQueue, most-specific row in map_state) has no test; the map_state change passes every test without it |
| FND-003 | medium | test-intent | FR-141-AC-15 Fault clause / TC-546 step 11 covered only by a choose_row unit test of unchanged code that passes without this PR |
| FND-004 | low | test-intent | a_protocol_node_row_lifts_by_the_declared_field_type asserts only Passes, not that r lifted to Queue vs Cell |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-141-AC-15, path: "spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md", role: examined, excerpt: "passes with the argument lifted to the Queue reference when the node record's field is declared Reference<Queue> (and to the Cell when declared Reference<Cell>), by the node's declared field type and never by row order; it is a StepVerdict::Fault only for a declared type that no row, or no most-specific row, conforms to."}
  - {id: FR-141-AC-16, path: "spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md", role: examined, excerpt: "first row's target type the subtype FastQueue and the second's the subtype SlowQueue, are each admitted by S3 and each decided with its receiver lifted against its own row's target type ... swapping the object rows changes no verdict."}
  - {id: FR-140-AC-18, path: "spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md", role: examined, excerpt: "a reference to a Call lifts to the SubQueue reference whichever row is written first, and swapping the two rows changes no mapped state key and no verdict"}
  - {id: TC-546, path: "spec/test-cases/TC-546-check-step-decides-each-concrete-step.md", role: examined, excerpt: "11. On two rows from Ring mapped to the subtypes FastQueue and SlowQueue ... run each row's step; then ... the protocol node row of step 10 with the node field declared Reference<Queue>, with Reference<Cell>, and with a declared type neither row conforms to, and swap the object rows' source order (FR-141-AC-16)."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-141-AC-16
    related_ids: [TC-546]
    path: fsl-analyze/tests/it/refinement_protocol.rs
    lines: "2291-2306"
    excerpt: |-
      first row's target type the subtype FastQueue and the second's the subtype SlowQueue, are each admitted by S3 and each decided with its receiver lifted against its own row's target type; check_step never returns Fault for a row S3 admitted.
    finding: "No test drives two FastQueue/SlowQueue rows through check_step; the target_type receiver change is unverified. #[trace(TC-546, FR-141-AC-16)] covers only the node-row half."
  - fnd: FND-002
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-140-AC-18
    related_ids: [TC-1086]
    path: fsl-replay/src/refinement/map.rs
    lines: "260-277"
    excerpt: |-
      a reference to a Call lifts to the SubQueue reference whichever row is written first, and swapping the two rows changes no mapped state key and no verdict
    finding: "No Queue/SubQueue test in either row order; reverting map_state to .find(admits) breaks no test."
  - fnd: FND-003
    severity: medium
    confidence: medium
    method: gap-analysis
    check: test-intent
    artifact_id: FR-141-AC-15
    related_ids: [TC-546, FR-140-AC-17]
    path: fsl-replay/src/refinement/map.rs
    lines: "1289-1311"
    excerpt: |-
      it is a StepVerdict::Fault only for a declared type that no row, or no most-specific row, conforms to.
    finding: "a_protocol_node_row_faults_when_no_object_row_conforms only calls the pre-existing choose_row (unchanged here) and passes without this PR; no node row is involved. TC-546 step 11's non-conforming declared type is unexercised; if unreachable from checked source, that needs the spec ruling the coder names, not an AC-15 tag on a unit test."
  - fnd: FND-004
    severity: low
    confidence: low
    method: gap-analysis
    check: test-intent
    artifact_id: FR-141-AC-15
    related_ids: [FR-141-AC-16]
    path: fsl-analyze/tests/it/refinement_protocol.rs
    lines: "2296-2305"
    excerpt: |-
      passes with the argument lifted to the Queue reference when the node record's field is declared Reference<Queue> (and to the Cell when declared Reference<Cell>)
    finding: "The test asserts matches!(verdict, Passes {..}) only; it does not check the bound r in Taken::Protocol is the Queue/Cell reference. Likely enforced by node-argument matching, but the AC's lift target is not asserted."
bindings:
  - {test_id: a_protocol_node_row_lifts_by_the_declared_field_type, ac_id: FR-141-AC-15, trace: correct}
  - {test_id: a_protocol_node_row_lifts_by_the_declared_field_type, ac_id: FR-141-AC-16, trace: correct}
  - {test_id: a_protocol_node_row_faults_when_no_object_row_conforms, ac_id: FR-141-AC-15, trace: correct}
```

+++
