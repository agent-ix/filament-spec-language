<!-- reviewer repo=agent-ix/filament-spec-language visibility=public quoin=0.36.10 module=spec-artifacts-process@v0.29.2 id=SR-96720 method=code-review lang=rust pr=filament-spec-language#3643 model=claude-opus-5-5 run=5D2C7471-BBB6-467C-9119-31933453A71A -->

Head 5cd78f803302021434588d539b67e8604125c09c. code-review + rust-review lane.

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | soundness | Package path passes target_type None: FastQueue/SlowQueue rows lift against declaring Queue, both conform, none most specific -> Fault for an S3-admitted row (FR-141-AC-16) |
| FND-002 | low | other | lift_value doc still says receiver uses the declaring type and node rows have no declared type / several rows fault |
| FND-003 | low | soundness | map_state falls back to source order when admitted rows have no most-specific one; row order still decides silently |
| FND-004 | low | code-bug | node_binder_types picks the first control by leaf name; duplicate leaf names across blocks read another node's binder types |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-141-AC-15, path: "spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md", role: examined, excerpt: "a node row for put whose argument self.ring evaluates to Reference<Ring> passes with the argument lifted to the Queue reference when the node record's field is declared Reference<Queue> (and to the Cell when declared Reference<Cell>), by the node's declared field type and never by row order; it is a StepVerdict::Fault only for a declared type that no row, or no most-specific row, conforms to."}
  - {id: FR-141-AC-16, path: "spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md", role: examined, excerpt: "rows put -> enq(self.ring, x) and put2 -> enq(self.ring, x) ... first row's target type FastQueue and the second's SlowQueue, are each admitted by S3 and each decided with its receiver lifted against its own row's target type; check_step never returns Fault for a row S3 admitted. A protocol node row ... declared Reference<Queue> passes with the Queue lift under twin and does not depend on the object rows' source order."}
  - {id: FR-140-AC-18, path: "spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md", role: examined, excerpt: "With the abstract queues population admitting Queue and its subtype SubQueue, both object rows naming the concrete Call, a reference to a Call lifts to the SubQueue reference whichever row is written first, and swapping the two rows changes no mapped state key and no verdict; rows Queue and Cell (unrelated types) are never ranked against each other ..."}
  - {id: FR-141-AC-14, path: "spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md", role: examined, excerpt: "the row put -> enq(self.ring, x) passes put(1) as the one abstract enq applied to the Queue r, not the Cell, the receiver lifting against the row's target type"}
findings:
  - fnd: FND-001
    severity: medium
    confidence: medium
    method: code-review
    check: soundness
    artifact_id: FR-141-AC-16
    related_ids: [FR-141]
    path: fsl-replay/src/refinement/package.rs
    lines: "137-141"
    excerpt: |-
      each decided with its receiver lifted against its own row's target type; check_step never returns Fault for a row S3 admitted.
    finding: "Package rows set target_type: None, so step.rs:1913 falls back to operation_declaring_type. For FastQueue/SlowQueue rows from Ring, choose_row sees two rows conforming to Queue with neither more specific and returns Ambiguous -> Fault. The package format (RefinementStepTarget::Operation) carries no target type; needs carrying it or a deferral ticket."
  - fnd: FND-002
    severity: low
    confidence: high
    method: code-review
    check: other
    artifact_id: fsl-replay/src/refinement/map.rs
    related_ids: [FR-141-AC-15, FR-141-AC-16]
    path: fsl-replay/src/refinement/map.rs
    lines: "703-711"
    excerpt: |-
      or for the receiver the type that declares the operation ... a protocol node row has none here, so the abstract type is the value's own type ... and a type several rows map is a fault.
    finding: "Doc on lift_value is stale: the receiver now lifts against the row's target type and node rows now pass the binder's declared type."
  - fnd: FND-003
    severity: low
    confidence: low
    method: code-review
    check: soundness
    artifact_id: FR-140-AC-18
    related_ids: [FR-140]
    path: fsl-replay/src/refinement/map.rs
    lines: "260-277"
    excerpt: |-
      swapping the two rows changes no mapped state key and no verdict; rows Queue and Cell (unrelated types) are never ranked against each other
    finding: "When one population admits two incomparable rows for one concrete type, .or_else(|| admitted.first()) picks by source order, so swapping rows changes the mapped state. Spec gives no rule; either fault or get a ruling."
  - fnd: FND-004
    severity: low
    confidence: low
    method: code-review
    check: code-bug
    artifact_id: fsl-replay/src/refinement/protocol_system.rs
    related_ids: [FR-141-AC-15]
    path: fsl-replay/src/refinement/protocol_system.rs
    lines: "198-208"
    excerpt: |-
      .find(|control| control.path.last().is_some_and(|last| last == node))
    finding: "First control by leaf name; two nodes with the same leaf in different blocks read the first one's binder types (binder_names above matches the full path). Would mis-lift, then fail matching rather than pass."
bindings: []
```

+++
