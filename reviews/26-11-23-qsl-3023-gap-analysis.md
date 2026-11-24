<!-- reviewer repo=agent-ix/filament-spec-language visibility=public quoin=0.36.10 module=spec-artifacts-process@v0.29.2 id=SR-96723 method=gap-analysis lang=rust pr=filament-spec-language#3648 model=claude-opus-5-5 run=7B400E65-DEC6-4752-B4E6-8811FC739BD1 -->

Head 63d4dc7a72988d7e1961aa49947bed35b5c3ae07. Manual AC-to-test check over the diff. The test fails without the change: the emitter assertion fails on `None`, and the old checker accepted `None` on the failing edge. Replacing the old `None` assertion is justified by the FR-244-AC-8 change, and no assertion was weakened.

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | coverage | Locked-region clause can pass vacuously: `inside` (edges out of the entry) is never asserted non-empty |
| FND-002 | low | test-intent | `assert_ne!(Some(Taken::Failing), None, "the two serialize differently")` is tautological and checks no serialization; AC-8's byte clause and candidate_blocks clause are untested under the AC-8 tag |
| FND-003 | low | coverage | Failing edge into the entry is tampered only to `None`; another value (e.g. `Stutter`) on that edge is untested |
| FND-004 | low | test-intent | `passing` keeps edges out of other reserved-state (locked-region) nodes, so they are tested as "passing" |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-244-AC-8, path: "spec/functional/FR-244-*.md", role: examined, excerpt: "writes that edge with taken Taken::Failing ({type: \"failing\"}) and every edge inside the locked region with None; the byte serialization of the two differs, and an emitter that writes None for the failing edge into the entry is a defect. ... a position lists both with their blocks in candidate_blocks ..."}
  - {id: FR-245-AC-12, path: "spec/functional/FR-245-*.md", role: examined, excerpt: "taken of the edge into the locked entry node changed from Taken::Failing to None is Rejected PostDiffers at that ZoneEdge; the same edge restored and an edge inside the locked region given Taken::Failing is Rejected PostDiffers at that edge; a Taken::Failing on a passing edge is Rejected PostDiffers; the unmodified certificate is Accepted."}
  - {id: TC-700, path: "spec/test-cases/TC-700-*.md", role: context_only, excerpt: "9. Tamper the taken of the edge into the locked entry node (None), of an edge inside the locked region and of a passing edge (Taken::Failing) ..."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: medium
    method: gap-analysis
    check: coverage
    artifact_id: FR-245-AC-12
    related_ids: [TC-700]
    path: fsl-analyze/tests/it/timed_refinement_certificate.rs
    lines: "305-317"
    excerpt: |-
      the same edge restored and an edge inside the locked region given Taken::Failing is Rejected PostDiffers at that edge
    finding: "inside = edges with from == entry. Only passing is asserted non-empty. A time-locked entry may have no outgoing edges, and then the locked-region clause is never exercised and the test still passes. Assert !inside.is_empty() (or pick an edge whose source automaton_state == reserved)."
  - fnd: FND-002
    severity: low
    confidence: high
    method: gap-analysis
    check: test-intent
    artifact_id: FR-244-AC-8
    related_ids: []
    path: fsl-analyze/tests/it/timed_refinement_certificate.rs
    lines: "305"
    excerpt: |-
      the byte serialization of the two differs, and an emitter that writes None for the failing edge into the entry is a defect.
    finding: "assert_ne! of two literals always holds and serializes nothing. The byte clause (wire form, elsewhere) and the candidate_blocks clause are not covered, yet the test carries #[trace(TC-699, FR-244-AC-8)]. Drop the assert or reword its message, and note that the tag is partial until half 2."
  - fnd: FND-003
    severity: low
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-245-AC-12
    related_ids: []
    path: fsl-analyze/tests/it/timed_refinement_certificate.rs
    lines: "301-304"
    excerpt: |-
      the taken of the edge into the locked entry node changed from Taken::Failing to None is Rejected PostDiffers
    finding: "Only None is tampered on the failing edge. The code's else branch also rejects other values, but no test pins it."
  - fnd: FND-004
    severity: low
    confidence: medium
    method: gap-analysis
    check: test-intent
    artifact_id: FR-245-AC-12
    related_ids: []
    path: fsl-analyze/tests/it/timed_refinement_certificate.rs
    lines: "309-311"
    excerpt: |-
      a Taken::Failing on a passing edge is Rejected PostDiffers
    finding: "passing filters only at != undo and from != entry. Edges out of other locked-region nodes (automaton_state == reserved) would be labelled passing; filter on the source automaton_state < reserved."
bindings:
  - {test_id: the_locked_undo_post_state_is_a_locked_entry_node, ac_id: FR-245-AC-12, trace: correct}
  - {test_id: the_locked_undo_post_state_is_a_locked_entry_node, ac_id: FR-244-AC-8, trace: correct}
```

+++



