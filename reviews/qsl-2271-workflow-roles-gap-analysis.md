---
id: SR-1795272578728440662
title: "gap-analysis — admitted workflow replicated roles"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2271; fsl-analyze/src/analyze/clause_item.rs; fsl-analyze/tests/it/analyze_package_protocol.rs; fsl-analyze/tests/it/protocol_roles.rs; fsl-eval/src/simulation/protocol.rs; fsl-eval/src/simulation/protocol/package_program.rs; fsl-eval/src/simulation/protocol/program.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: references
---

## Summary

Bounded gap-analysis inspected requirement ownership, real assertions and absence-of-source evidence for the six-file workflow role slice. This is a partial repository audit; the user prohibited matrix and full source-tree scans.

## Verdict

**CONDITIONAL** — all bounded workflow-role findings are fixed and all five focused tests pass. The computed matrix and whole-repository reverse inventory remain unassessed by explicit scope restriction; no repository-wide assurance PASS or parent completion is claimed.

Ticket: QSL-2271. PR: agent-ix/filament-spec-language#3315. Frozen head: `1fccdcaa8dd8725023f53c4232cf93a8f27e65fc`; base: `d25728d74d830ccde8e0aae5c14d0b315af0db1e`. All six changed files were inspected read-only. The owning ticket remains In Progress and already carries has-review. No applicable AssuranceProfile was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The required forged-actor replay negative was absent at the initial frozen head; valid actor identities and malformed role bindings do not establish rejection of an unspawned-object or unknown-role actor. | fsl-analyze/tests/it/analyze_package_protocol.rs:287 |

## Evidence

The package normalizer reads existing admitted role, population, maximum, binder and lifetime fields; the field spellings and shapes agree with the existing lowering and native protocol view. The binder is checked as a reference to the declared role model. Scope and Until retain distinct named Unsupported detail, and no source graph is reconstructed. Send/Receive ownership comes from the existing channel endpoints. Program.replicated and Program.acting feed the existing role universe, spawn maximum, actor enumeration, canonical identity and finish teardown machinery.

The four new tests build real source fixtures and real admitted BarePackage inputs, compare complete canonical states plus outgoing transitions, assert absent source_package in the package model, and remove program.fsl before command analysis/replay. Empty-run coverage requires two actual reachable actors and two converging spawn orders, then verifies every finish removes them. Max 1 versus 2 changes real live populations; owned Attempt transitions must name Worker actors and both object choices. The empty Idle universe cannot create an actor and must report exact no-role-instance detail. Wrong caller role binding must retain exact typed code/cause parity. Scope and Until fixtures first require a substantive source result, then require package refusal before exploration.

Source forms were checked against the existing role grammar, role-lifetime scope checker, touch operation shape, and established attempt success-binder fixtures. No new gate, warning suppression, unsafe path, wire format, dependency, copied fixture or duplicated engine was added.

## Evidence and limits

The lead's original acceptance brief requires adversarial actor replay. This mandatory requirement was confirmed after the initial static pass. FND-001 records that test gap at the original head; no production defect is alleged. Follow-up freeze `887e2d81c3d47e203e4b48f9f77b34d7847994a9` adds exactly one separate test, `package_workflow_roles_replay_refuses_forged_actors`, with the existing four functions unchanged.

The follow-up test obtains a genuine Work deadlock witness and requires valid Source and package replay baselines. It replaces one transition with real reachable transitions whose canonical fields differ only in actor: an unspawned object and an unknown role. Both must refuse at the exact prefix step with ProtocolNotEnabled, Code::InvalidRuntimeInput and cause invalid-value; whole typed refusal values must be equal between Source and package. Before package replay, the source file is deleted and byte_provision is empty. The author reports focused compilation exit 0 at /tmp/protocol-workflow-roles-forged-check.log. Static inspection found the setup and entrypoints coherent. Runtime evidence is still pending.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The mandatory test is now present and focused compilation reportedly passes at 887e2d81c3d47e203e4b48f9f77b34d7847994a9; genuine Source/package adversarial replay runtime parity has not yet been proven. |
| FND-001 | fixed | Pass 1: added genuine adversarial actor replay test at 887e2d81c3d47e203e4b48f9f77b34d7847994a9, statically inspected and reported runtime PASS by the lead (1 passed, 0 failed, exit 0). Both corruptions require exact Source/package refusal equality after valid baselines. |
| FND-002 | still-open | Pass 1: the three failed tests require a bounded fixture-only handoff reuse fix and focused runtime proof. Scope/Until and forged-actor passes remain retained. |
| FND-002 | fixed | Pass 2: authentic existing handoff reuse at 16b624dceaf31cdb9524b7e6400c78767257bea0 preserves all parity/replay assertions; the lead's actual Draco receipt reports body, bounds and empty each 1 passed, 0 failed, 0 ignored, 2595 filtered, exit 0. |

## Initial validation evidence

The author reports `cargo check --locked -p fsl-analyze --test it -j4` exit 0 with assigned locked wrapper and debug/incremental disabled, log /tmp/protocol-workflow-roles-check2.log. This reviewer ran no Cargo, runtime, aggregate gate, workspace/all-targets, or matrix command. The four-function focused runtime filter is pending through the lead. No runtime PASS is asserted.

This review is bounded to workflow lifetime roles. Predicate/scope lifetimes, mixed Parallel/Send, correlation and RepeatProof remain explicit pending capabilities under QSL-2271. Existing guards remain intact. Valid actor identity and malformed role mapping are examined; corrupt-actor replay coverage is not claimed. Full FR-208-AC-3 scope retirement and all FR-208-AC-5 live channel cases are not established by this slice.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | workflow_role_graph writes the handoff through package_request, then channel_send_parity calls package_request again in the same Workspace; FR-517 correctly refuses the second write AlreadyExists, so three tests stop during setup before their full Source/package analysis and replay oracles execute. Reuse the authentic existing handoff and bytes or use a fresh Workspace; preserve the production refusal and all semantic assertions. | fsl-analyze/tests/it/analyze_package_protocol.rs:121; fsl-analyze/tests/it/analyze_package_protocol.rs:1370 |

## Runtime evidence (lead receipt)

The lead reports original four-test filter at 1fccdcaa8dd8725023f53c4232cf93a8f27e65fc: other_lifetimes passes; body, bound-1 and empty-universe cases fail in handoff setup with handoff.directory AlreadyExists at analyze_cli_support.rs:85:64. No successful command analysis/replay claim is made for those three. This is a fixture lifecycle defect, not a production defect. The initial reviewer missed the repeated writer.

At 887e2d81c3d47e203e4b48f9f77b34d7847994a9, the lead reports the exact new forged-actor test passed: 1 passed, 0 failed, 0 ignored, 2595 filtered, exit 0. Combined with direct source inspection, its legitimate Source/package baselines and both exact forged actor refusals close FND-001. The raw runtime receipt is held by the lead; this reviewer did not run a test. The two passing tests are retained, and only the three failed fixture paths need focused reruns after the fixture fix.

## FND-002 fixture fix static inspection

Frozen follow-up `16b624dceaf31cdb9524b7e6400c78767257bea0` changes only the test fixture. The three failed workflow cases now call workflow_role_parity, which requires the existing authentic analyze-package.json written during graph comparison and passes that path into the shared parity routine. The prepared-path closure performs no second handoff write. Source record/label checks, deletion of program.fsl, exact package item and exit equality, and replay.reproduced for refuted outcomes are unchanged. The ordinary channel helper still invokes its original writer lazily after Source analysis. No read error is swallowed, no handoff is deleted or overwritten, and no production behavior changes. The lifetime and forged-actor passing test bodies are unchanged. The author reports focused compilation exit 0, log /tmp/protocol-workflow-roles-handoff-check.log. FND-002 is statically corrected but remains open until the three failed runtime cases prove their remaining oracles.

## Rebase transport evidence

Final candidate `e2d9a85dbbfb5340fab6d883c962f966dda1e540` rebases the original four commits through `16b624dceaf31cdb9524b7e6400c78767257bea0` onto `3e4108c7e7`. Direct git range-diff reports all four entries equal. Independently filtering the pre/post patch files to added/removed lines, excluding file headers and preserving newline bytes, produces exactly 742 lines in each and identical SHA-256 `b19ee9d0e8939432acfc0f8a24342755f34a1449ae7cc89ce4b4fe9c0163ea33`. The files are /tmp/protocol-3315-pre-rebase.patch and /tmp/protocol-3315-post-rebase.patch. Raw patch file hashes differ due metadata/context; no raw-file equality is claimed. This establishes transport of the reviewed patch; it does not itself supply runtime evidence. No conflict adapter, API change or semantic edit appeared in the rebase.

Final actual Draco receipt supplied by the lead: at original freeze 16b624dceaf31cdb9524b7e6400c78767257bea0, body, bounds and empty each passed 1 test, with 0 failed, 0 ignored, 2595 filtered, exit 0. Raw logs are on Draco at /tmp/qsl-3315-16b624d-{body,bounds,empty}.log; these remote files are not present in this reviewer's local /tmp, so the evidence is attributed to the lead's runtime receipt rather than falsely described as locally inspected. Together with retained lifetime PASS at 1fccdcaa8dd8725023f53c4232cf93a8f27e65fc and forged-actor PASS at 887e2d81c3d47e203e4b48f9f77b34d7847994a9, all five focused cases are green. No runtime test was rerun by this reviewer. Final range-diff was rechecked and all four entries remain equal at e2d9a85dbbfb5340fab6d883c962f966dda1e540. Both findings are closed; QSL-2271 remains In Progress for the unimplemented protocol families.

## Examined scope

## Final reviewed candidate

Final head is `e405146390867d98fa1cfce72db28de6ea80fe2b`, rebased onto `b1a310b605208560a361022877997a1d80740027`. A second direct git range-diff against the reviewed e2d9 range again shows all four commits equal, with no conflicts or semantic/API adapter. The same five focused runtime receipts apply; no extra runtime execution was performed.

The three raw Draco result logs have now been supplied locally and directly inspected at /home/peter/.cache/handoff/protocol-batch-2-latest-qualification/qsl-3315-16b624d-{body,bounds,empty}.log. Each shows exactly one intended workflow test passing, 0 failed, 0 ignored, 2595 filtered. This supplements the earlier lead-only attribution without rewriting its historical record. FND-001 and FND-002 are fixed. Code/Rust PASS and bounded gap CONDITIONAL (unassessed full matrix/inventory only) stand; there are no unresolved findings.

## Examined units

```yaml
{
  "scope": [
    {
      "id": "FR-208",
      "path": "spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md",
      "role": "examined",
      "excerpt": "A `send` node SHALL be owned by the `from` role of its channel and a `receive` node by the `to` role of its channel."
    },
    {
      "id": "FR-208-AC-2",
      "path": "spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md",
      "role": "examined",
      "excerpt": "With `max 2`, spawning `w1` then `w2` and spawning `w2` then `w1` reach states with equal keys. With two live instances, `event(Work)` gives two steps whose identities name `w1` and `w2`."
    },
    {
      "id": "FR-208-AC-3",
      "path": "spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md",
      "role": "examined",
      "excerpt": "With lifetime `workflow`, every live instance retires in the `finish` step, which removes the protocol instance with its role instances, so the state after `finish` holds none. With lifetime `scope`, an instance retires in the step after which the last `by r` node of its region is unreachable from every live thread, and no `retire` step exists."
    },
    {
      "id": "FR-208-AC-5",
      "path": "spec/functional/FR-208-spawn-and-retire-replicated-role-instances.md",
      "role": "examined",
      "excerpt": "A channel `from r to c` with `r` replicated, `max 2` and two live instances gives two `send` steps whose identities name `w1` and `w2`; a channel `from c to r` gives two `receive` steps likewise; with an empty universe a thread at the `send` waits with `NoRoleInstance{r}`."
    },
    {
      "id": "FR-940-AC-6",
      "path": "spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md",
      "role": "examined",
      "excerpt": "For each item family whose evaluator FR-531's inventory lists as taking a source-checked value (the temporal clauses `Bounded` and `AtMostTwo`, the invariant `NotAboveThree`, the deadlock-freedom item and the `SB` litmus claim under `tso`), a `program` request and a `package` request over the handoff directory of the same compiled unit give equal item records, counterexamples and replays, and no package-route record is `unsupported`;"
    },
    {
      "id": "FR-531",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "context_only",
      "excerpt": "This requirement adds no checked-package member and no Contract IR code."
    },
    {
      "id": "FR-850",
      "path": "spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md",
      "role": "examined",
      "excerpt": "No source is compiled, and the items, records, counterexamples, replays and exit code SHALL equal those of the same request over the source the package was compiled from, subject only to the named route-specific encoded replay-byte-budget exception below."
    },
    {
      "id": "fsl-analyze/src/analyze/clause_item.rs",
      "path": "fsl-analyze/src/analyze/clause_item.rs",
      "role": "examined",
      "excerpt": "Frozen diff d25728d74d830ccde8e0aae5c14d0b315af0db1e..1fccdcaa8dd8725023f53c4232cf93a8f27e65fc."
    },
    {
      "id": "fsl-analyze/tests/it/analyze_package_protocol.rs",
      "path": "fsl-analyze/tests/it/analyze_package_protocol.rs",
      "role": "examined",
      "excerpt": "Frozen diff d25728d74d830ccde8e0aae5c14d0b315af0db1e..1fccdcaa8dd8725023f53c4232cf93a8f27e65fc."
    },
    {
      "id": "fsl-analyze/tests/it/protocol_roles.rs",
      "path": "fsl-analyze/tests/it/protocol_roles.rs",
      "role": "examined",
      "excerpt": "Frozen diff d25728d74d830ccde8e0aae5c14d0b315af0db1e..1fccdcaa8dd8725023f53c4232cf93a8f27e65fc."
    },
    {
      "id": "fsl-eval/src/simulation/protocol.rs",
      "path": "fsl-eval/src/simulation/protocol.rs",
      "role": "examined",
      "excerpt": "Frozen diff d25728d74d830ccde8e0aae5c14d0b315af0db1e..1fccdcaa8dd8725023f53c4232cf93a8f27e65fc."
    },
    {
      "id": "fsl-eval/src/simulation/protocol/package_program.rs",
      "path": "fsl-eval/src/simulation/protocol/package_program.rs",
      "role": "examined",
      "excerpt": "Frozen diff d25728d74d830ccde8e0aae5c14d0b315af0db1e..1fccdcaa8dd8725023f53c4232cf93a8f27e65fc."
    },
    {
      "id": "fsl-eval/src/simulation/protocol/program.rs",
      "path": "fsl-eval/src/simulation/protocol/program.rs",
      "role": "examined",
      "excerpt": "Frozen diff d25728d74d830ccde8e0aae5c14d0b315af0db1e..1fccdcaa8dd8725023f53c4232cf93a8f27e65fc."
    }
  ]
}
```

## Coverage

Plan completion: not assessed

No plan was supplied. Repository identity is ix://agent-ix/filament-spec-language; spec root is spec/, source/test roots are the named Rust crates. The normal matrix and whole-source reverse-gap steps are unassessed because the dispatch explicitly prohibits those scans; the method was not unavailable and no grep index is represented as an engine-backed matrix. Criterion totals, stale tags, binding census and repository-wide stub/coverage counts are not claimed.

Six bounded behaviors were inventoried: role projection, endpoint ownership, Program replication/acting initialization, static-role binding checks, typed lifetime refusal, and source/package analysis/replay parity. Each maps to FR-208/531/940/850; within these inspected behaviors there are zero unowned behaviors, zero source stubs and zero test stubs. No coverage percentage is claimed. New tests have real state/transition/output assertions and no mocks or skips. The optional repository semantic-review expansion was not requested and was not run. Scope/Until are explicitly out of the implemented lifetime slice; parent assurance remains unfinished.

