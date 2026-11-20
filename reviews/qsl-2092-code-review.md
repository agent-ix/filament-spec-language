---
id: SR-1501
title: "Code and Rust review of QSL-2092 replay boundaries"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2092; fsl-analyze/tests/it/{weak_memory,protocol_system,zone_replay}.rs and replay_boundary_cases children; .agent/receipts/qsl-2092"
review_set: subset
---

## Summary

Reviewed the new Rust replay boundary tests and their real helper paths. No code, test-oracle, Rust idiom, or declared-scope defect was found.

## Verdict

**PASS** — no findings within the reviewed diff and its declared library-core scope. Final component-gate execution is reserved to the lead.

## Review scope

Review target: `583d0de02eed477339b887c7e84db972d1a0953d`, ticket QSL-2092. Eight changed files: three existing integration-parent registrations, their new `replay_boundary_cases` children, and two delivery receipts. Helper context examined was limited to the actual replay entry, witness production, and settlement mapping needed to assess those tests.

The ten new tests use returned TSO/RA and protocol counterexamples, real state transitions, typed replay refusals, and exact rational delays. Successful original replay controls accompany the altered evidence. The timed fixture explicitly uses the existing test-support admission for unwritten timed members; this is library-core evidence, not independent timed package read-back or task-automaton lowering. No production, dependency, configuration, or gate change is included.

Author-reported focused results: ten active functions passed, with original authoring failures and confined corrections retained. These reports were accepted without rerunning tests, builds, lint, or the matrix. The final affected-component gate remains with the lead. Existing lint limitations remain disclosed; no green lint claim is made. Broader historical QSL-2092 families remain unclaimed.

## Examined units

```yaml
{
  "scope": [
    {
      "id": "FR-227-AC-1",
      "path": "spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md",
      "role": "examined",
      "excerpt": "`SB`'s `tso` counterexample carries, at each of its four positions, both buffers as in ADR-025 §10's table and names `SB::Both` resolved to `tso`; replay recomputes each buffer and settles `reproduced-with-evaluated-witness`."
    },
    {
      "id": "FR-227-AC-5",
      "path": "spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md",
      "role": "examined",
      "excerpt": "`SB`'s `tso` counterexample replays to `reproduced-with-evaluated-witness` with `max_store_buffer` unset and with 1, since no buffer holds more than one store. `SB`'s `ra` counterexample replays to `reproduced-with-evaluated-witness` with `max_messages` unset (4); with `max_messages` 1 it refuses `invalid_runtime_input`/`invalid-value` at its first store, which would add a second message to its location."
    },
    {
      "id": "FR-227",
      "path": "spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md",
      "role": "context_only",
      "excerpt": "A counterexample over a subject with a weak `parallel` SHALL be ADR-027\nPX-1's protocol counterexample whose steps include memory steps, whose `ra`\nloads and stores name their message or slot by depth, and whose every step\ncarries its post-state's memory component in canonical form; it SHALL name\nthe resolved model of each outermost `parallel` and list the memory\nconstraints with their `memory` origin (ADR-025 MK-1)."
    },
    {
      "id": "FR-217-AC-1",
      "path": "spec/functional/FR-217-replay-a-protocol-counterexample.md",
      "role": "examined",
      "excerpt": "The deadlock counterexample of ADR-027 §7 (`fork(Both)`, `attempt(A)`) replays: both post-state keys match, no step is enabled at s2, its instance is live, the recomputed blocked threads equal FR-211-AC-2's, and the result is `reproduced-with-evaluated-witness`. The envelope's steps serialize with protocol transition identities and the blocked list."
    },
    {
      "id": "FR-217-AC-3",
      "path": "spec/functional/FR-217-replay-a-protocol-counterexample.md",
      "role": "examined",
      "excerpt": "A deadlock envelope whose blocked list drops `right` settles `inconclusive`, `ReplayParity`. An envelope whose prefix runs on to s6, which holds no live instance, with `kind: Deadlock` settles `inconclusive`, `ReplayParity`."
    },
    {
      "id": "FR-217",
      "path": "spec/functional/FR-217-replay-a-protocol-counterexample.md",
      "role": "examined",
      "excerpt": "A counterexample over a protocol subject SHALL be ADR-018's\n`TemporalCounterexample` naming the protocol subject, with each step as its\nprotocol transition identity and its post-state's state key, each\n`send` step also carrying its `send_record`, and, for\na deadlock, FR-211's blocked threads (ADR-027 PX-1)."
    },
    {
      "id": "FR-237-AC-2",
      "path": "spec/functional/FR-237-replay-a-timed-counterexample.md",
      "role": "examined",
      "excerpt": "Refusals settle no result: the `timeout` delay changed to `5/2` (guard `x >= 3` fails); the delay changed to `7/2` (`x <= 3` forbids it); a lasso whose loop has total delay 0; a lasso whose loop does not return to its entry clock values."
    },
    {
      "id": "FR-237",
      "path": "spec/functional/FR-237-replay-a-timed-counterexample.md",
      "role": "examined",
      "excerpt": "`fsl_replay::replay_model_trace` (FR-128) SHALL replay a counterexample\nover a timed subject in exact rational arithmetic (ADR-026 CT-3, TD-3): it\nstarts every clock at 0, checks each delay is admissible, checks each\ntransition is enabled after its delay, selects the successor by the\npost-state key, applies resets, checks time invariants, checks the timed\nlasso's closure and fairness, and evaluates the formula over the timed\nlasso."
    }
  ]
}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

