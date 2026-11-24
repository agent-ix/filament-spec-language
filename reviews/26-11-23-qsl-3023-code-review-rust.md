<!-- reviewer repo=agent-ix/filament-spec-language visibility=public quoin=0.36.10 module=spec-artifacts-process@v0.29.2 id=SR-96722 method=code-review lang=rust pr=filament-spec-language#3648 model=claude-opus-5-5 run=7B400E65-DEC6-4752-B4E6-8811FC739BD1 -->

Head 63d4dc7a72988d7e1961aa49947bed35b5c3ae07. code-review + rust-review lane. Emitter writes `Some(Taken::Failing)` on the edge into the locked entry. `refine_edge` accepts only `Failing` there and rejects `None` or any other value with `PostDiffers`. `Failing` on a passing edge mismatches `yielded` and is rejected with `PostDiffers`. In the locked region any `taken` is rejected with `PostDiffers`. The untimed model checker faults on `Failing`. `Taken` is not serde, so the change is in-memory only.

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | low | other | Test doc still says the failing undo edge "takes nothing"; it now takes Taken::Failing |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-244-AC-8, path: "spec/functional/FR-244-*.md", role: examined, excerpt: "The certificate of a timed refinement whose check_step fails at every point of a source zone, the post-state being a time-locked entry node, writes that edge with taken Taken::Failing ({type: \"failing\"}) and every edge inside the locked region with None; the byte serialization of the two differs, and an emitter that writes None for the failing edge into the entry is a defect."}
  - {id: FR-245-AC-12, path: "spec/functional/FR-245-*.md", role: examined, excerpt: "The refinement certificate of FR-430-AC-6 with the taken of the edge into the locked entry node changed from Taken::Failing to None is Rejected PostDiffers at that ZoneEdge; the same edge restored and an edge inside the locked region given Taken::Failing is Rejected PostDiffers at that edge; a Taken::Failing on a passing edge is Rejected PostDiffers; the unmodified certificate is Accepted."}
findings:
  - fnd: FND-001
    severity: low
    confidence: high
    method: code-review
    check: other
    artifact_id: fsl-analyze/tests/it/timed_refinement_certificate.rs
    related_ids: [FR-244-AC-8]
    path: fsl-analyze/tests/it/timed_refinement_certificate.rs
    lines: "242-246"
    excerpt: |-
      /// certificate lists the post-state of `undo` as a `locked` node with the
      /// reserved index, entered by the failing `undo` edge, which takes nothing;
    finding: "Stale doc: the undo edge now carries Taken::Failing per FR-244-AC-8."
bindings: []
```

+++

