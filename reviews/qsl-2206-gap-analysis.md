---
id: SR-202611030004
title: "Gap analysis of filament-spec-language PR #2778: recovery criteria"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-eval/tests/it/protocol_retry_bound.rs; PR #2778 head b89a5fc7bd3452a7752a1538dd66ab1003ac4f22"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-730
    type: reviews
---
# Gap analysis of PR #2778

## Summary

Ticket: QSL-2206. Rechecked the frozen head `b89a5fc7bd3452a7752a1538dd66ab1003ac4f22`; no fix commit changed the recovery tests. The new test checks cend record identity and recovered/unrecovered successor statuses, but misses three explicit FR-730 assertions: the claim/counterexample, no-read single successor, and replay/record mutation.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FSpec-FR-426-AC-1 | fsl-eval/tests/it/protocol_retry_bound.rs | examined |
| FSpec-FR-426-AC-9 | fsl-eval/tests/it/protocol_retry_bound.rs | context_only |
| FSpec-FR-427-AC-6 | fsl-eval/tests/it/protocol_retry_bound.rs | examined |
| FSpec-FR-730-AC-1 | fsl-eval/tests/it/protocol_retry_bound.rs | examined |
| FSpec-FR-730-AC-2 | fsl-eval/tests/it/protocol_retry_bound.rs | examined |
| FSpec-FR-730-AC-3 | fsl-eval/tests/it/protocol_retry_bound.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The recovery test checks record/status successors but not FR-730-AC-1's claim and counterexample. | fsl-eval/tests/it/protocol_retry_bound.rs:502-564 |
| FND-002 | medium | No-read recovery predicate single-successor behavior is not asserted. | fsl-eval/tests/it/protocol_retry_bound.rs:472-564 |
| FND-003 | medium | No test replays a recovery counterexample or varies its carried record. | fsl-eval/tests/it/protocol_retry_bound.rs:502-564 |

## Dispositions

Disposition round 1 at `b89a5fc7bd3452a7752a1538dd66ab1003ac4f22`:

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The changed integration test checks only cend record/status successors; no claim verdict or FR-730-AC-1 counterexample carrying `bal` 1 is asserted. |
| FND-002 | still-open | The changed tests still lack a no-read predicate such as `triggered`, with exactly one cend successor and unchanged claim verdict. |
| FND-003 | still-open | No changed test replays the recovery counterexample or checks that changing its carried record to `bal` 0 stops reproducing. |

## Verdict

Not mergeable under the gap findings. FND-001 through FND-003 describe unasserted acceptance behaviors, not merely test style. Targeted test execution cannot establish those missing assertions.

Disposition round 2 at `6d89a77dfa1a1fd9da8c5487e40340c7bfff8d11`:

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The test now asserts that the all-recovered claim is false and that its counterexample cend carries `bal` 1. |
| FND-002 | fixed | The new no-read test checks one cend per registration, no bound record, and verdict parity with the baseline. |
| FND-003 | fixed | The test replays the carried `bal` 1 counterexample to `unrecovered`, then changes the record to `bal` 0 and verifies `recovered`. |

After excerpt for FND-001:

```rust
let path = recovery_counterexample_path(&reachable)
    .expect("bal 1 gives a counterexample to every finish being recovered");
assert!(!recovery_claim_holds(&reachable));
let cend = path.last().expect("the counterexample ends at cend");
assert_eq!(cend["step"], "cend");
assert_eq!(
    scalar_text(named_member(&cend["record"], "bal").expect("cend carries its record")),
    Some("1"),
    "the counterexample carries the unrecovered bal 1 record"
);
```

After excerpt for FND-002:

```rust
for (transition, _) in cends {
    cend_count += 1;
    assert!(
        transition.binder_record().is_none(),
        "a no-read cend binds no recovery record"
    );
}
assert!(cend_count > 0, "the fixture reaches cend");
assert_eq!(
    recovery_claim_holds(&reachable),
    baseline_verdict,
    "adding an unread recovery binder preserves the claim verdict"
);
```

After excerpt for FND-003:

```rust
let mut changed = path;
assert!(replace_named_scalar(
    &mut changed.last_mut().expect("cend is the final transition")["record"],
    "bal",
    "1",
    "0"
));
let changed_state = replay_transition_path(&replay_system, &changed)
    .expect("the cend transition with bal 0 replays");
assert_eq!(
    registration_status(&changed_state, &registration),
    Some("recovered"),
    "changing the carried record to bal 0 no longer reproduces the counterexample"
);
```

The additionally reported ignored analyzer fixture remains outside QSL-2206's scope: this ticket's acceptance is for protocol evaluator stepping/refusal behavior, whereas that `fsl-analyze` fixture remains explicitly ignored for analyzer settlement. Its failure is tracked separately and does not block this ticket's evaluator acceptance.

## Round 2 verdict

All findings from both review methods are fixed at the exact final head. The ignored analyzer fixture is out of scope for this ticket. Mergeable on the scoped targeted evidence reported for this head.
