---
id: SR-20261125200207
title: "gap-analysis of PR 3966 written binding with the admitted member type"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3966 head 0ac1e4e84eb4bac1ccaecc97b5c6c643588f7003; git diff origin/main...HEAD"
review_set: subset
---

## Summary

Ticket: QSL-3327. Gap analysis limited to the PR diff. `quire trace --scope . --id FR-125-AC-9` was run on the worktree (repo `traceability:` model; symbol-id collision warnings on unrelated files only). Criteria in scope: FR-125-AC-9 (written binding clause), FR-125-AC-15 (hyper parameter reads), FR-181 (HP-5 evidence), FR-172 (hyper instances); FSpec FR-361 "Instancing" and FR-395 "Object parameters" as context.

Plan completion: not assessed

## Verdict

FAIL under this skill's rule (an untagged criterion in scope), fixable within this PR by tagging. `quire trace` gives FR-125-AC-9 one claim, the `implements` on `ModelSystem::written_reference`, and no test tag; the only other hit is a file-scope mention in the `model_instances.rs` module doc. The three new tests assert AC-9's refuted-binding clause and are tagged `TC-520` only (FND-001). No stub; the new code has an owning requirement except the unadmitted-key fallback (FND-002). Semantic review not run (not requested).

The author's open question (FND-002): no requirement specifies what a binding of an HP-5 key that no initial record admits writes. FSL FR-125's hyper bullet and FR-172 (`hyper_instances`: "whether or not an initial state holds the key") make such a key an instance. FSpec FR-395 "Object parameters" gives one instance per admitted key (FR-361), so under the standard the case does not arise. FSpec FR-361 says only that the wire reference carries the admitted record's type, and here there is none. Writing the declared type is the better of the two unspecified choices. The previous least member type of the population could name a type in another universe (`Dog` in `Animal`'s for a `Lamp` parameter, the defect itself). The declared type stays in the parameter's universe and conforms to `T`. Accept it in this PR, add a test that pins it, and get a spec ruling on the FR-125/FR-172 against FSpec FR-395 instance-set conflict. Do not restore the prior behaviour.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-125-AC-9 has no tagged test: the three new tests assert its written-binding clause (refuted binding is (`UniverseId`, admitted type, key)) but carry only `TC-520`, so the matrix still reports AC-9 as untested while `written_reference` claims `implements` | fsl-analyze/tests/it/model_instances.rs:2458,2478,2498; fsl-eval/src/simulation/model.rs:2808 |
| FND-002 | medium | `written_reference`'s fallback (a key no initial record admits is written with the parameter's declared type, reachable only through HP-5 `hyper_instances`) has no owning requirement and no test, and changes the wire output for that key from the population's least member type; FSL FR-125/FR-172 (every universe key is an HP-5 instance) conflict with FSpec FR-395 (admitted keys only) | fsl-eval/src/simulation/model.rs:2814-2818, spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:240-253 |

## Coverage

- FR-125-AC-9: `implements` 1 (`ModelSystem::written_reference`), test tags 0; asserted in part by 3 untagged-for-AC tests. Its remainder (key-alone obligation identity, probabilistic and protocol bindings) is QSL-3126.
- FR-181 / HP-5 evidence wire: asserted by `an_hp5_witness_binds_the_admitted_member_type`, which does not separate the admitted type from the declared type (code-review SR-20261125200206 FND-001).
- Reverse gap: 1 (FND-002). Stubs: none.
- Semantic review: skipped (not requested).
- Plan completion: not assessed
