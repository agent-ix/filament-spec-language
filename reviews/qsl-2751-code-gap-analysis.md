---
id: SR-20261121235237001
title: "Gap analysis of PR filament-spec-language#3414 (QSL-2751 slice 1)"
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: "agent-ix/filament-spec-language; QSL-2751; PR3414 head 3ad91680d07d88c5a79205e532fa6906ae1ab76d; criteria FR-338-AC-29..31, FR-127-AC-1, FR-1030-AC-8..11, FR-1031-AC-5; source fsl-analyze/src/model_check/check.rs, fsl-replay/src/certificate/closure.rs, fsl-replay/src/model_check/item.rs, fsl-command/src/command/transfer.rs; tests in the PR diff"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-338
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1030
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1031
    type: references
---

## Summary

Ticket: QSL-2751. This is a repository-driven gap audit, scoped to the criteria and code that PR filament-spec-language#3414 touches, at head 3ad91680d07d88c5a79205e532fa6906ae1ab76d. Plan completion was not assessed. The static matrix shows every scoped criterion as tagged. However, the tag for FR-1030-AC-11 rests on a test that asserts none of that criterion's transfer or root-missing obligations, so it is coverage inflation. No runtime evidence exists because the tests are held. Model claude-opus-5-5, run b6d48eeb-9843-473a-b193-e46c54e9b8f7.

## Verdict

**FAIL**: there is one high finding, a criterion whose computed `tagged` status is backed by a test that does not assert it.

No reverse gap was found. Each changed behaviour traces to an owning FR:

- the monitored `on each` and protocol TP-3 routing traces to FR-338;
- the checker's zero-progress search traces to FR-338 "Protocol non-progress acceptance";
- the protocol abstract-subject refusal traces to FR-1031-AC-5.

There are no source stubs, no placeholder returns and no test stubs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-1030-AC-11 is computed `tagged` only through `an_on_each_tp3_claim_settles_from_the_package_as_from_source`, which runs `analyze` claim parity for `PrevSafeEach` and `PrevJumpEach`. That test never settles a transfer through `planner::settle` over the package, never offers a package without the clause root (`failed`), and never reads the emitted `temporal_clause` node's `on each` activation. The tag claims FR-1030-AC-11, but what the test actually asserts is FR-338-AC-30 package parity, so the matrix reports a criterion as covered that no test backs. | FR-1030-AC-11, fsl-analyze/tests/it/analyze_tp3_package.rs::an_on_each_tp3_claim_settles_from_the_package_as_from_source |

## Coverage

- Reconciliation: quire matrix (quire 0.52.0, engine 0.62.2). No run evidence was read: `quoin matrix --repo . --json` (quoin 0.36.8) exited 1 with `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`, so the documented `quire matrix --scope . --format json` fallback ran.
- Plan completion: not assessed
- Criteria in scope: tagged 9, untagged 0, tagged-by-ignored-test 0, method-without-symbol 0. The tagged criteria are FR-338-AC-29, FR-338-AC-30, FR-338-AC-31, FR-127-AC-1, FR-1030-AC-8, FR-1030-AC-9, FR-1030-AC-10, FR-1030-AC-11 and FR-1031-AC-5.
- Stale tags: `quire coverage --scope . --json` shows no `untracked_symbols` among the tags this diff adds. The pre-existing `FSpec-` cross-repository ids it lists in the same files are outside this diff.
- Evidence: there is no run evidence. All runtime tests are unrun under the architecture hold. This is a verification limitation and is not recorded as a finding.
- Untraced behaviours and stubs: 0 untraced among 6 changed behaviours; 0 source stubs; 0 test stubs.
- Semantic review: this was not the opt-in Quoin semantic pass. The companion code review, SR-20261121235237000, judged intent against test and code statically.
