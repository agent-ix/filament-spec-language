---
id: SR-2026111414542500000002
title: "Gap analysis of QSL-1931 CheckMode on package checking (PR #2706)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1931; PR #2706 head 8478f1759e98aaa0e60d7779f59d0dd43099377f vs origin/main; spec/functional/FR-495, FR-562, FR-093; qsl-semantics/src/check/{facts.rs,family.rs,mod.rs}; qsl-semantics/tests/it/check_mode.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-495
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-562
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---
# Gap analysis: QSL-1931 CheckMode on package checking

## Summary

Ticket: QSL-1931. Plan completion: not assessed (planless). Evidence:
`quoin matrix --json` (quoin 0.29.4, quire 0.38.3) on the PR head, the diff,
and the reviewer's run of the check_mode tests (8 passed).

Matrix, for the criteria this diff tags:

- FR-562-AC-6: tagged by `an_unproved_result_bound_checks_only_under_kernel`.
  The test asserts the checker half only (Linked refuses `unproved-range`,
  Kernel checks). The JIT half belongs to the later JIT PRs.
- FR-495-AC-3: tagged by `an_unproved_nonzero_divisor_checks_only_under_kernel`
  (decimal `d4`) and by two existing qsl-eval quantity tests. The integer
  `d1` Linked refusal is tracked as QSL-1962 and is out of scope here.
- FR-093-AC-62: tagged by the PR's `a_checked_operation_exposes_its_node_key`
  and, already on main, by `lowering/tests/node_keys.rs`. See SR-2026111414542500000001
  FND-001: the PR's accessor duplicates main's.

Code with no owning criterion: four implemented Kernel behaviours have no
FR-495 acceptance criterion, so their tests carry no trace tag. The coder
reports a spec ticket for them is being filed. They are recorded below as
spec gaps, not as defects of this PR. No stubs or coverage inflation were
found. Every new test asserts a typed cause.

## Verdict

The code is aligned with the FR-495 Inputs and Kernel bullets for the
obligations it drops. The four gaps below need FR-495 acceptance criteria
before those tests can be traced. None of them blocks this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Spec gap, not a PR defect. Kernel drops the presence obligation of `value(x)` (`Obligation::Presence`). The FR-495 Kernel bullets name only range and nonzero for division, and no FR-495 AC states presence under Kernel. `an_unproved_presence_checks_only_under_kernel` is therefore untraced. | qsl-semantics/src/check/facts.rs:1452-1455; qsl-semantics/tests/it/check_mode.rs:97-104 |
| FND-002 | medium | Spec gap, not a PR defect. Kernel reaches function bodies only; a state clause keeps refusing its unproved obligation (FR-104 "Definedness facts"). No FR-495 AC states this scope boundary, so `a_state_clause_still_refuses_its_obligation_under_kernel` is untraced. | qsl-semantics/tests/it/check_mode.rs:106-131 |
| FND-003 | medium | Spec gap, not a PR defect. A termination measure is checked as under Linked in both modes (SR-61009). No AC in FR-495 or FR-091 states it, so `kernel_mode_still_proves_a_termination_measure` is untraced. | qsl-semantics/src/check/family.rs:1614-1626; qsl-semantics/tests/it/check_mode.rs:251-266 |
| FND-004 | medium | Spec gap, not a PR defect. Under Kernel a Rational operation whose operand is not Rational-typed still refuses `Unproved(RationalRange)` (SR-61009 FND-011). No AC owns this structural refusal, and no test reaches it; the code's own doc says no checked node can have that shape. | qsl-semantics/src/check/facts.rs:1199-1226 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | Spec gap, not a PR defect. Kernel dropping a rational-range obligation, a rational `/` nonzero obligation and a nonempty-reduction obligation has no acceptance criterion. FR-495's Kernel bullets name range and nonzero for `div`/`rem`/`mod` only. AC-3 covers decimal and quantity divisors, AC-10 to AC-13 cover other behaviours, and FR-096-AC-14 and FR-100-AC-10 cover `sum` evaluation. TC-1419's description lists all five obligations, but no AC states the other three. The three new tests (`an_unproved_rational_range_checks_only_under_kernel`, `an_unproved_rational_divisor_checks_only_under_kernel`, `an_unproved_nonempty_reduction_checks_only_under_kernel`) are correctly untagged; tagging AC-1, AC-2 or AC-3 would mis-trace them. | qsl-semantics/tests/it/check_mode.rs:190-222; spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md |

## Dispositions

Round 1 verified at head 6297dca4c111619607591b408f56ef5c41a59fb5 against origin/main with spec PR #2707 (FR-495-AC-10 to AC-13, TC-1419, TC-1420). `quoin matrix` shows AC-10 to AC-13 tagged, and every new trace tag cites an existing criterion.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |

Round 2 at head 6a3caa977b91ac9784872beff784a4b2e8774479. The delta touches two test files and no spec.

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | deferred | Spec PR #2708 (QSL-1982) adds FR-495-AC-14 to AC-16 for the rational-range, rational divisor and nonempty-reduction Kernel pairs; it is under separate review, and the coder tags the three tests after it merges. |
