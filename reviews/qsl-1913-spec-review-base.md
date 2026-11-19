---
id: SR-2026111414035501
title: "base review of QSL-1913 and QSL-1914 model-checker scaling spec"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md; spec/decisions/ADR-016-state-model-finite-execution-mapping.md; spec/test-cases/TC-1417-worker-count-hash-indexed-seen-set-and-packed-state-leave-results-unchanged.md; spec/test-cases/TC-1418-a-directly-solved-assignment-keeps-the-successors-and-counts-its-candidates.md; PR quire-spec-language#2705 at a6d625440"
review_set: subset
---

## Summary

Tickets: QSL-1913 and QSL-1914. Base checklist over `git diff origin/main...HEAD`
(head a6d625440, 0 commits behind main). The diff adds the FR-101 worker
count, the seen-set and packed-state rules and AC-16 to AC-22, the FR-126
`workers` member and AC-20, the FR-191 Workers section and AC-6 and AC-7,
the FR-120 "Directly solved assignments" rule and AC-29 to AC-33, the ADR-016
G-4 amendment, and TC-1417 and TC-1418.

The base review checked whether each new AC can fail and whether QSL-1833,
QSL-1835, QSL-1837 and the parallel sampling ticket can tag it. Quoin's
criterion-strength method (`spec-criterion-strength-analysis`) was skipped:
it needs a Jev client, and none exists (`jev` is not installed and the skill
says no client exists yet). The base review did that check by hand instead.

IDs: TC-1417 and TC-1418 are not on main and no open PR adds them. PR #2698
adds only TC-1416. No other open PR touches FR-101, FR-120, FR-126, FR-191 or
ADR-016. No hash, digest, pin or tracking-record antipattern was found.

Workers semantics: FR-101, FR-126 and FR-191 agree that budgets are charged in
merge order, that a cancelled run equals a one-worker prefix, and that
`workers` 0 is refused with `invalid_runtime_input`/`invalid-value`. None of
the three contradicts another. The gaps are recorded under integrity,
failure-domain and scope-boundary.

Counting rule: it is consistent with FR-120-AC-13. That AC's `spawn` has no
`post` clause, so it is ineligible and still enumerated. ADR-016 G-4 and its
summary bullet are amended. Findings, successors and order are kept below the
limit, and the count depends only on the state, so it is the same for every
worker count. The rule states openly that a `max_candidates` stop can move
later ("MAY complete"), so this is not a silent weakening. Its strategy
independence is broken by the optional `MAY`, recorded as FND-001 of the
failure-domain review (SR-2026111414035504).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-126-AC-20 cannot catch a merge-order defect in its `max_candidates` and cancel cases. The `max_candidates` case stops at the first state, where every worker count agrees anyway. The cancel case checks only `Stopped{Cancelled, None}` and never the "one-worker result of a prefix" rule. Those two FR-126 SHALL clauses have no criterion that can fail. | FR-126:328-340, FR-126-AC-20 | correct-requirement-no-evidence |
| FND-002 | medium | FR-120-AC-30 says the 100 successors are "ordered by ascending `b`" but does not say where that order is observed. FR-101 exploration orders by state-key bytes, and integers are decimal strings in the key (`"value":"10"` sorts before `"2"`), so exploration order is not ascending `b`. Only FR-120's candidate order is. The Invariant bullet's "same canonical order" has the same two readings. | FR-120-AC-30, FR-120:724-733, FR-101:241-252 | wrong-requirement |
| FND-003 | low | FR-120-AC-32's oracle is "as a run with the solver absent does", but the spec defines no way to turn the solver off. It should compare against the ineligible `... and true` reference form, as AC-30 does. | FR-120-AC-32 | wrong-requirement |
| FND-004 | low | FR-101-AC-16 requires "the same trace order when the run is replayed", but exploration returns no trace, so it is unclear what is compared. | FR-101-AC-16 | wrong-requirement |
| FND-005 | low | FR-191-AC-6 says the `max_samples` 100 run "draws equal samples at trace indices 0 to 99" but does not say what they equal (presumably the one-worker run's samples). | FR-191-AC-6 | wrong-requirement |
| FND-006 | low | FR-120-AC-29 says the charge is exact but does not give the number. It is 3: one argument vector, one candidate, one pair. The AC can still fail through the 1,000 case, but pinning 3 would let it catch an over-count. | FR-120-AC-29 | wrong-requirement |

## Verdict

Not mergeable as is. The blocking defects are recorded in the integrity
review (two tables leave FR-120-AC-29 and FR-191-AC-6/7 unparsed) and the
failure-domain review (`MAY` against the strategy-independence claim). Apart
from FND-001, the other new ACs can fail and are testable through
`explore_request`, `check_model`, `explore_model` and the statistical run:
FR-101-AC-16 to AC-19, FR-120-AC-30 and AC-31, and FR-191-AC-6 and AC-7 once
FR-191's table is repaired. The hash and pack ACs need a test seam (see
scope-boundary).

## New findings (disposition pass 1)

Checked at 6d3ffe68e.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | The charges in FR-120-AC-29, AC-30, AC-31 and AC-33 and in TC-1418 do not match qsl-eval's counting. A (candidate, result) pair is charged only when the operation declares a result (`if self.application.declaration.result().is_some() { budget.charge()?; }`, qsl-eval/src/simulation/model/frames.rs:1521-1523). So `swap` charges 2 (argument vector and candidate), not 3. The solved `step` charges 101, not 201, and its enumerated reference form 10,001, not 20,001. The review pass's FND-006 itself said 3, which was also wrong. Second, AC-30 to AC-32 are "over AC-29's model", which already contains `swap`, and the budget covers the whole expansion (one `Budget` per `expand_state`, walk.rs:389). The observed expansion charge would therefore add swap's application (101 + 2 = 103 from `a` 3, `b` 5). Fix: give each AC a model with just the one operation, correct the numbers to 2, 101 and 10,001 (AC-31's out-of-domain case stays 1), and state in FR-120 "Post-states" that an operation with no result charges no pair. Changing the code to charge a (candidate, none) pair would change existing stop points and is outside QSL-1914. | FR-120-AC-29, FR-120-AC-30, FR-120-AC-31, FR-120-AC-33, TC-1418:62-79 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-126-AC-20 now sets `max_candidates` to stop at the one-worker run's seventh expanded state, makes a later state finish first, and requires equal counts and frontier. The cancel case now requires the counts and frontier of a one-worker prefix of length `n`. Both can fail on a merge-order defect. |
| FND-002 | fixed | FR-120-AC-30 compares successors as keyed sets and says exploration orders by state-key bytes (`b` 10 precedes `b` 2). The Invariant now says "candidate order" and that exploration order is FR-101's. |
| FND-003 | fixed | FR-120-AC-32 now compares each case with its ineligible `... and true` reference form. |
| FND-004 | fixed | FR-101-AC-16 no longer mentions trace order. It compares `explore_request_parallel` at 1, 2, 4 and 64 with `explore_request`. |
| FND-005 | fixed | FR-191-AC-6 names the comparand ("equal to the one-worker run's samples at those indices") and bounds the double at 103 draws. |
| FND-006 | fixed | FR-120-AC-29 now pins the charge and defines how it is observed (the smallest completing `max_candidates`). The pinned value is wrong; see FND-007. |
| FND-007 | fixed | Round 2 (a905dc14f): every case in FR-120-AC-29 to AC-33 now uses a model with a single operation, and the charges are 2 (swap), 101 (step solved), 10,001 (step enumerated) and 1 (out of domain). TC-1418 matches. FR-120 "Post-states" now says an operation with no result charges no pair and that the budget covers the whole expansion. Checked against qsl-eval: `walk::expand_state` makes one `Budget` per expansion (walk.rs:389) and calls `frames::apply` per row and application. That charges one per argument vector (frames.rs:201) and one per generated candidate before the closure check (frames.rs:849). It charges a pair only when the operation declares a result (frames.rs:1521-1523). |
