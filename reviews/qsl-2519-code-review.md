---
id: SR-2026111911313600000005
title: "Code review — QSL-2519 state-graph fixtures"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2519; fsl-analyze/tests/it/{graph_world,state_graph,state_graph_certificate,state_graph_settle,analyze_timed_cli}.rs"
review_set: subset
---

## Summary

Reviewed frozen 54da9109390abc4a6ffa759596b82c4d0263489d against origin/main. Ticket: QSL-2519. All five changed test files examined: graph_world.rs, state_graph.rs, state_graph_certificate.rs, state_graph_settle.rs, analyze_timed_cli.rs under fsl-analyze/tests/it/. No production, dependency, checker or CI changes. No applicable AssuranceProfile found.

Kernel admission is the existing FR-495 function-body route: check/mod.rs:3501-3523 forces declared clauses to Linked, and :5180-5206 constructs the independent state-graph predicate context. The helper preserves normal config/fork constructors' Linked default. The quotient returns 2 at version 1 and raises DivisionByZero at version 2, so the sampled witness cannot mask undefined evaluation. Canonical stems, typed cause, predicate role, exact reached site, BadState/PredicateUndefined certificate rejection and settlement assertions remain live. The modulo helper preserves the Fork reached-state failure. No mocks replace checking or execution.

TC-592 and TC-594 explicitly require an undecided postcondition; evaluating the candidate version 2 leaves predecessor version 1 open and versions [0,1] reachable. These changed expectations agree with successor semantics. Certificate BadState tests deliberately retain preconditions and the reached version-2 node. The timed CLI binds to FR-852's general default-zone rule rather than falsely claiming its two-step reply AC: its phase-1 target needs exactly send with zero delay, and assertions require en-6, zone-certified, certified, accepted zone certificate, possible form, one witness path and that exact step.

The FR-050-AC-6 identity test retains all assertions under the explicitly authorized tracked QSL-2563 ignore. This is a disclosed deferred reader defect, not passing identity evidence; its existing OperationMemberMismatch expectation is preserved for the owning follow-up. One unrelated pre-existing ignore remains.

Method choice: code-review with Rust lane and manual scoped AC-to-test semantic binding review. Production gap-analysis is not applicable because every changed path is test code; no whole-repository matrix/gap audit was run. Spec-review and its sub-analyses are not applicable because spec/ and plan/ are unchanged. FR-165, FR-166 and FR-170 are context only; TC-592/593/594/643/644 supplied fixture and certificate context. Plan completion: not assessed.

Validation evidence: prior /private/tmp/qsl2519-final-filter.log reports 89 passed, 0 failed, 2 ignored; this predates the coordinated dependency rebase and is NOT current-head test evidence. Existing focused rustfmt and diff check passed; reviewer independently ran git diff --check successfully. Current-pin ignored identity check was still queued in /private/tmp/qsl2519-identity-new-pins.log when inspected. No cargo command launched by this reviewer. Lead owns the final name-filtered current-head gate. Strict Clippy's 567 unchanged fsl-semantics diagnostics and unrelated workspace formatting drift are inherited baseline limitations, not introduced findings.

Exact model identifier is unavailable to this harness; marker model=unavailable records that honestly.

## Verdict

**PASS** — no introduced defects found in the scoped fixture review. Current-head execution remains the lead's required focused gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope and bindings

```yaml
{
  "clean": true,
  "scope": [
    {
      "id": "FR-167-AC-2",
      "path": "spec/functional/FR-167-explore-a-state-graph-subject-and-track-open-nodes.md",
      "role": "examined",
      "excerpt": "§7.1 with `max_depth` 1: `(1, 0)` and `(0, 1)` are open, `MaxDepth`, `(0, 0)` is closed, and `end` is `Completed`. With `max_states` 3 the run ends `Stopped(ResourceExhausted, MaxStates)` and each discovered unexpanded node is open, `NotExpanded`. A subject with an undecided contract conjunction at one reachable node marks that node open, `UndecidedSuccessor`, and still expands every other reachable node."
    },
    {
      "id": "FR-168-AC-5",
      "path": "spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md",
      "role": "examined",
      "excerpt": "Over §7.1's subject, `possible 2 div (2 - c.versionNumber) = 2` for `c = a` under the subject's integer-division law (its package selects one, as FR-168-AC-11's fixture does; `div` needs that selection) returns `Undefined` with the canonical path `(0, 0) -upd(a)-> (1, 0) -upd(a)-> (2, 0)`, `where` `(2, 0)` and cause `division-by-zero`, although `(1, 0)` satisfies the target."
    },
    {
      "id": "FR-168-AC-7",
      "path": "spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md",
      "role": "examined",
      "excerpt": "FR-168-AC-5's item with default `witness_samples` holds a sampled witness from `(0, 0)` ending at `(1, 0)` and still returns `Undefined` at `(2, 0)`, not `Witnessed`; `ReachesTwo` with default limits returns `Witnessed` with `Sampled` sources from a run that completed with no open node."
    },
    {
      "id": "FR-169-AC-4",
      "path": "spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md",
      "role": "examined",
      "excerpt": "FR-168-AC-4's trap settles `refuted` (V-10) and its `NoDecision` settles `inconclusive`, `BoundReached{depth: 3}`, execution `completed`, truth `pending`, and its record states horizon 3 as a method parameter, not among the limits. A run stopped by `max_states` settles `incomplete`, `LimitReached`, naming `max_states` and its value; an undecided node with no decisive evidence settles `inconclusive`, `UndecidedSuccessor`; a subject with no initial state settles `inconclusive`, `NoInitialState`."
    },
    {
      "id": "FR-169-AC-6",
      "path": "spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md",
      "role": "examined",
      "excerpt": "FR-168-AC-5's `Undefined` outcome settles `refuted`, `decisive-counterexample`, category violation, after FR-170 replay reproduces it, with cause `UndefinedEvaluation` naming the node `(2, 0)` and `division-by-zero`."
    },
    {
      "id": "FR-169-AC-7",
      "path": "spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md",
      "role": "examined",
      "excerpt": "FR-168-AC-7's `Undefined` outcome, found although a sampled witness existed, settles `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation`; `ReachesTwo` with default limits settles `proved`, `decisive-witness`, with `Sampled` sources, and its record names a completed exploration."
    },
    {
      "id": "FR-169-AC-10",
      "path": "spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md",
      "role": "examined",
      "excerpt": "Rejected certificates, each settling `inconclusive`, `CertificateRejected{rule, at}` and never `proved`: AC-9's `ReachesTwo` certificate with `(2, 2)` removed (`SuccessorMissing` at the first, in certificate order, of the two closure states that reach it, `(1, 2)` and `(2, 1)`; the engine gives `(1, 2)`); with the rank of `(1, 0)` changed to 2 (`RankBroken` at `(0, 0)`, the only ranked state left with no successor one rank lower, because `(1, 0)` keeps its successor `(1, 1)` at rank 1); with `(0, 0)` unranked (`RankMissing` at `(0, 0)`); a hand-built certificate for FR-168-AC-5's item that lis"
    },
    {
      "id": "FR-169-AC-11",
      "path": "spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md",
      "role": "examined",
      "excerpt": "A certificate holding a state whose expansion evaluates a contract conjunction undefined is rejected `BadState` at that state and settles `inconclusive`, `CertificateRejected{BadState, at}`, never `proved`: §7.1's subject with universe `{a}`, the one initial state `va = 0` and the guard `pre Guard using v on Config::ConfigVersion::attemptUpdate { 2 div (2 - self.versionNumber) >= 1 }` (the guard of TC-592's FR-167-AC-2 fixture), checked with a hand-built certificate of its 3 states ranked 2, 1 and 0 for `ReachesTwo`, is rejected `BadState` at the state where `va = 2`, whose guard has no value;"
    },
    {
      "id": "FR-495-AC-3",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "`function d1 using v(a: Int[0, 9], b: Int[0, 9]): Int[0, 9] pure { a div b }`, the same with `rem` and `mod`, `function d2 using v(a: P, b: P): P pure { a / b }` with `type P = Decimal[-1000, 1000; 0, 2; nearest-even];`, `function d3 using v(): Boolean pure { 1 cm / 0 cm = 1 cm / 0 cm }` (with TC-187's units) and `function d4 using v(): P pure { decimal(1, 0) / decimal(0, 3) }` each refuse `undefined_expression`/`unproved-nonzero` at the operator under `CheckMode::Linked`; `function d5 using v(a: Int[0, 9], b: Int[1, 9]): Int[0, 9] pure { a div b }` and `if b != 0 then a div b else 0` over `b:"
    },
    {
      "id": "FR-050-AC-6",
      "path": "spec/functional/FR-050-publish-authenticated-temporal-artifacts.md",
      "role": "examined",
      "excerpt": "The v2-specific L5 evaluation entry points compare all authenticated clock fields before positions and retain them in results; checked `/2` counterparts additionally join the exact S3 source locus, declaration, profile and clock before evaluation and retain the same admitted definition/clock evidence. The existing `/1` entry points keep their signatures and historical unauthenticated scope and cannot establish native-to-temporal correspondence."
    },
    {
      "id": "FR-852",
      "path": "spec/functional/FR-852-decide-state-graph-items-over-a-timed-subject-on-zones.md",
      "role": "examined",
      "excerpt": "Without `options.method`, or with `zones`, EN-6 decides it on its zone graph."
    },
    {
      "id": "FR-165",
      "path": "spec/functional/FR-165-check-state-graph-claims-at-s3.md",
      "role": "context_only",
      "excerpt": "Check state-graph claims at S3 and record their form"
    },
    {
      "id": "FR-166",
      "path": "spec/functional/FR-166-sample-witnesses-for-possible-claims.md",
      "role": "context_only",
      "excerpt": "Sample witnesses for possible claims before exploration (EN-1 phase 0)"
    },
    {
      "id": "FR-170",
      "path": "spec/functional/FR-170-replay-state-graph-evidence.md",
      "role": "context_only",
      "excerpt": "Replay state-graph evidence through ModelSystem"
    }
  ],
  "findings": [],
  "bindings": [
    {
      "test_id": "an_undecided_contract_conjunction_leaves_only_its_node_open",
      "ac_id": "FR-167-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "a_sampled_witness_never_hides_an_undefined_target",
      "ac_id": "FR-168-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "an_undefined_target_refutes_in_place_of_a_witness",
      "ac_id": "FR-168-AC-5",
      "trace": "correct"
    },
    {
      "test_id": "the_checker_rejects_a_certificate_over_an_undefined_target",
      "ac_id": "FR-169-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "an_undetermined_contract_at_a_reached_state_is_bad",
      "ac_id": "TC-644",
      "trace": "correct"
    },
    {
      "test_id": "an_undefined_guard_at_a_reached_state_rejects_bad_state",
      "ac_id": "FR-169-AC-11",
      "trace": "correct"
    },
    {
      "test_id": "undefined_and_altered_count_certificates_never_settle_proved",
      "ac_id": "FR-169-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "a_bad_state_certificate_settles_certificate_rejected",
      "ac_id": "FR-169-AC-11",
      "trace": "correct"
    },
    {
      "test_id": "horizon_limit_undecided_and_empty_runs_settle_their_rows",
      "ac_id": "FR-169-AC-4",
      "trace": "correct"
    },
    {
      "test_id": "an_undefined_target_settles_refuted_with_its_site",
      "ac_id": "FR-169-AC-6",
      "trace": "correct"
    },
    {
      "test_id": "a_sampled_witness_never_hides_an_undefined_refutation",
      "ac_id": "FR-169-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "a_state_graph_claim_over_a_timed_subject_proves_on_zones",
      "ac_id": "FR-852",
      "trace": "correct"
    },
    {
      "test_id": "state_graph_identity_readback_uses_the_explicit_temporal_selection",
      "ac_id": "FR-050-AC-6",
      "trace": "correct"
    }
  ]
}
```

