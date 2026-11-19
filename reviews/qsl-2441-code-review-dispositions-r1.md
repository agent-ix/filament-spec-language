<!-- reviewer-dispositions repo=agent-ix/filament-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-2026111911313600000001 round=1 model=claude-opus-5-5 run=92e745d0-947b-40c7-b975-b187fca814d0 -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | deferred | QSL-2546 (arch ruling on QSL-2441: claim body + path carry the scheduler after spec W77); ClaimWorkload and probabilistic::claim_workloads marked TEMPORARY, removal recorded on QSL-2546. |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | still-open | fsl-package/src/lib.rs re-export block still not rustfmt-clean: rustfmt sorts ClaimWorkload before ClockSelectionCause and repacks `SelectedSourceCollectionRow, ...` onto full lines. Run rustfmt on the block. |
| FND-007 | fixed | - |

New findings (disposition pass 1):

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-008 | low | other | Panic message is wrong for UnresolvedClaimWorkload, an emission invariant (no emitted workload node named W), not a lowering one. |
| FND-009 | low | other | The public accessor is not marked TEMPORARY (only the struct and the private fn are), so callers see a stable API that QSL-2546 removes. |

+++ [reviewer data]

```yaml
head: 9588a1a78f391de7c609d8e5b0cca221398b30b9
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: "assert!(\n    resolved.contains(&(claim_node(\"A\"), workload_named(\"Steady\"))),\n\
    \    \"A resolves to Steady: {resolved:?}\"\n);\nassert!(\n    resolved.contains(&(claim_node(\"B\"\
    ), workload_named(\"Bursty\"))),\n    \"B resolves to Bursty: {resolved:?}\"\n);"
- fnd: FND-002
  outcome: deferred
  reason: 'QSL-2546 (arch ruling on QSL-2441: claim body + path carry the scheduler after spec W77); ClaimWorkload
    and probabilistic::claim_workloads marked TEMPORARY, removal recorded on QSL-2546.'
- fnd: FND-003
  outcome: fixed
  after_excerpt: '#[ignore = "QSL-2546: FSL FR-498 fixes a claim node''s identity as name + content ordinal
    with an empty body, so no workload or reward identity reaches it; FSpec FR-640-AC-3 says it does"]'
- fnd: FND-004
  outcome: fixed
  after_excerpt: 'let by_name = workload_nodes_by_name(nodes);

    ...

    let workload = by_name.get(name.as_str()).ok_or_else(|| EmitRefusal::UnresolvedClaimWorkload { claim:
    claim.name.clone(), workload: name.clone() })?;'
- fnd: FND-005
  outcome: fixed
  after_excerpt: '/// TC-2062 (FR-185-AC-12): the S4 refusal of a reward, a delay, a random

    /// parameter and a workload is located at the declaration (its label span in

    /// the unit''s text) and names the declaration in words, ...

    assert_eq!((*feature, declaration.as_str()), (expected, name));

    ...

    assert!(label.contains(word), "{name}: the region is `{label}`");

    assert!(message.contains(word), "{message}");'
- fnd: FND-006
  outcome: still-open
  reason: 'fsl-package/src/lib.rs re-export block still not rustfmt-clean: rustfmt sorts ClaimWorkload
    before ClockSelectionCause and repacks `SelectedSourceCollectionRow, ...` onto full lines. Run rustfmt
    on the block.'
- fnd: FND-007
  outcome: fixed
  after_excerpt: "Some(\"float\") => {\n    filament_semantic_value::declaration::EqualitySchedule::Float\n\
    }"
new_findings:
- fnd: FND-008
  severity: low
  confidence: high
  method: code-review
  check: other
  artifact_id: emit_checked_never_returns_incomplete_across_the_fixture_set
  related_ids: []
  path: fsl-package/src/emit/tests.rs
  lines: 2025-2030
  excerpt: "Err(\n    EmitRefusal::UnkeyedProbabilisticClaim { .. }\n    | EmitRefusal::UnresolvedClaimWorkload\
    \ { .. },\n) => {\n    panic!(\"lowering keys every probabilistic claim\")\n}"
  finding: Panic message is wrong for UnresolvedClaimWorkload, an emission invariant (no emitted workload
    node named W), not a lowering one.
- fnd: FND-009
  severity: low
  confidence: high
  method: code-review
  check: other
  artifact_id: fsl-package/src/emit.rs#Emission::claim_workloads
  related_ids:
  - QSL-2546
  path: fsl-package/src/emit.rs
  lines: 1376-1378
  excerpt: '/// The `workload` node each claim `under W` resolves to, ascending by

    /// claim node (FSL FR-740).

    pub fn claim_workloads(&self) -> &[ClaimWorkload] {'
  finding: The public accessor is not marked TEMPORARY (only the struct and the private fn are), so callers
    see a stable API that QSL-2546 removes.
```

+++
