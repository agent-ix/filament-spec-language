---
id: SR-20261125200207
title: "gap-analysis of PR 3966 written binding with the admitted member type"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3966, review pass at head 0ac1e4e84eb4bac1ccaecc97b5c6c643588f7003, disposition pass 1 at head 80764c452acd233ecadd192eeb3c0237d6caa725; git diff origin/main...HEAD"
review_set: subset
---

## Summary

Ticket: QSL-3327. Gap analysis limited to the PR diff. `quire trace --scope . --id FR-125-AC-9` was run on the worktree, which uses the repository's `traceability:` model; it warned of symbol-id collisions in unrelated files only. Criteria in scope: FR-125-AC-9 (its written-binding clause), FR-125-AC-15 (hyper parameter reads), FR-181 (HP-5 evidence) and FR-172 (hyper instances). FSpec FR-361 "Instancing" and FR-395 "Object parameters" were read as context.

Plan completion: not assessed

## Verdict

Review pass: FAIL under this skill's rule, because a criterion in scope was untagged. It was fixable within the PR by tagging. FR-125-AC-9 had one claim (the `implements` on `ModelSystem::written_reference`) and no test tag. The three new tests asserted its refuted-binding clause but were tagged `TC-520` only (FND-001). There was no stub. The new code had an owning requirement, except the fallback for an unadmitted key (FND-002).

The author's open question was about that fallback. No requirement states what is written for the binding of an HP-5 key that no initial record admits. FSL FR-125's hyper bullet and FR-172 make such a key an instance. FSpec FR-395 makes only admitted keys instances, and FSpec FR-361 states the written type only for an admitted record. Writing the declared type is the better of the two unspecified choices. The previous least member type could name a type in another universe, while the declared type stays in the parameter's universe. The recommendation was to keep the declared type, pin it with a test, and get a spec ruling, not to restore the prior behaviour.

Disposition pass 1, at 80764c452a: CONDITIONAL.

- **FND-001 fixed.** Three refuted-binding tests now carry FR-125-AC-9 and assert what it states.
- **FND-002 deferred.** The fallback is pinned by a test, and the spec ruling is filed as QSL-3408.
- **New FND-003 (low).** Four tests tagged FR-125-AC-9 assert the witness-evidence binding of a proved item, which the criterion does not state.
- **New FND-004 (low).** The changed functions that write the refuted binding carry no `#[implements]`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-125-AC-9 has no tagged test: the three new tests assert its written-binding clause (refuted binding is (`UniverseId`, admitted type, key)) but carry only `TC-520`, so the matrix still reports AC-9 as untested while `written_reference` claims `implements` | fsl-analyze/tests/it/model_instances.rs:2458,2478,2498; fsl-eval/src/simulation/model.rs:2808 |
| FND-002 | medium | `written_reference`'s fallback (a key no initial record admits is written with the parameter's declared type, reachable only through HP-5 `hyper_instances`) has no owning requirement and no test, and changes the wire output for that key from the population's least member type; FSL FR-125/FR-172 (every universe key is an HP-5 instance) conflict with FSpec FR-395 (admitted keys only) | fsl-eval/src/simulation/model.rs:2814-2818, spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:240-253 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | Four tests are tagged FR-125-AC-9 but assert the witness-evidence binding of a `proved` item: `an_hp5_witness_binds_the_admitted_member_type`, `state_graph_evidence_binds_the_admitted_member_type`, `zone_state_graph_evidence_binds_the_admitted_member_type` and `digital_state_graph_evidence_binds_the_admitted_member_type`. AC-9 states only a refuted binding. The evidence binding is FR-125 "Instancing" body text, and no criterion states it, so these tags overclaim AC-9 coverage. Keep `TC-520` only, or add a criterion for written evidence bindings | fsl-analyze/tests/it/model_instances.rs:2459,2479; fsl-analyze/tests/it/timed_state_graph.rs:569,586 |
| FND-004 | low | `ClauseLetters::instance_reference`, `items::bound_over` and `evaluate_model_trace` (replay) were changed to write the refuted `over_binding`, which is FR-125-AC-9's written-binding clause. None of them carries `#[implements("FR-125-AC-9")]`; only `ModelSystem::written_reference` does | fsl-eval/src/simulation/model/letters.rs:583-590, fsl-analyze/src/analyze/items.rs:203-214, fsl-replay/src/model_check/replay.rs:3100-3108 |

## Coverage

- **FR-125-AC-9.** It has one `implements` (`ModelSystem::written_reference`) and seven tagged tests at 80764c452a. Three of them assert the refuted binding AC-9 states: `a_refuted_over_binding_names_the_admitted_member_type`, `a_zone_temporal_over_binding_names_the_admitted_member_type` and `a_model_counterexample_keeps_a_non_conforming_admitted_binding`. The other four assert evidence bindings (FND-003). Its remainder, the key-alone obligation identity and the probabilistic and protocol bindings, is QSL-3126.
- **FR-181 / HP-5 evidence wire.** `an_hp5_witness_binds_the_admitted_member_type` now separates the admitted type from the declared type.
- **Unadmitted HP-5 key.** Pinned by `an_hp5_witness_of_an_unadmitted_key_binds_the_declared_type` (`TC-520` only, correctly untagged for any AC). The spec conflict is QSL-3408.
- **Reverse gap:** 1, deferred (FND-002).
- **Stubs:** none.
- **Semantic review:** skipped (not requested).
- **Plan completion:** not assessed

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The refuted-binding tests now carry `#[trace("TC-520", "FR-125-AC-9")]`, and `quire trace` would list them as AC-9 tests. The over-tagging of the evidence tests is new finding FND-003 |
| FND-002 | deferred | QSL-3408 records the spec conflict and the missing written type. The fallback is now pinned by `an_hp5_witness_of_an_unadmitted_key_binds_the_declared_type`, which asserts (Thing universe, `Thing`, `m`) |
