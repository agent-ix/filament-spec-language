---
id: SR-202611210022442313144108559
title: "integrity spec review of the QSL-2739 PR #3385 disposition-round clarifications"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; only the clauses changed in fix round 1 and its continuation (68957d6ce7..11c4db178296af2965c5a5e1c950c0776c956bc8): FR-530 Settlement-and-outcomes mismatch paragraph and FR-530-AC-9; TC-1330 expected result 8; FR-1044 Outputs cycle bullet, Registered provider Malformed-witness and Well-formed-witness bullets, FR-1044-AC-6 and FR-1044-AC-14. Checked against FSpec FR-243, FR-331, FR-492 at 5423fcf4 and FSL FR-621."
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-530
    type: references
---

## Summary

Ticket: QSL-2739, PR #3385, final head 11c4db17. This is a bounded integrity and consistency check of the Markdown clarifications made in the disposition round, under the binding ARCH budget and witness-shape rulings. It is not a full spec review.

The FR-530, AC-9 and TC-1330 budget wording agrees with FSpec FR-492: "this section relaxes none of admission, replay, certification, trust, extent or limits", FR-492-AC-9, and `limit-reached` naming `max_work`. The FR-1044 malformed-witness wording is the ruling's text: a closed linked type, and the plugin producer's FR-331 schema obligation, with no phantom FSL reader or API. The cycle clarification is grounded in the upstream specs. FSpec FR-331 writes `basis` on `proved` only, `unsupported` carries exactly one of `cause` and `diagnostic`, and FR-621 items carry an FR-243 settlement basis only under `axes`.

## Verdict

**CONDITIONAL**: one low consistency finding. Every other changed clause was examined and is consistent with its authority and its sibling clauses.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new FR-1044 cycle parenthetical says FSpec FR-243's settlement basis "is not a member of a terminal record". FR-1044-AC-14 (and the Outputs bullet at line 205 and the Behavior bullet at line 330) for an admitted `interface-mismatch` still require "basis `decisive-counterexample`" on the same kind of record. That record is a provider `refuted` entry, which FSpec FR-331 gives no `basis`, and an FR-621 composition item has no `axes`. That clause of AC-14 cannot be tested as written, and the AC-14 test (correctly) does not assert it. The fix is to word it as the cycle bullet now is, for example "no proof `basis` member". | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:423 |

## Dispositions

Disposition round 1 at frozen head c81747fa6d7376058eabcb4cb019e1f81ab26ede (parent 11c4db178296af2965c5a5e1c950c0776c956bc8). The diff between them changes only the three FR-1044 passages and the committed review copies; Rust and test bodies are identical to 11c4. The new wording matches FSpec FR-331 (`basis` on `proved` only; `refuted` carries `counterexample`) and FR-621 (no FR-243 settlement basis outside `axes`), and agrees with the AC-6 cycle wording.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-1044 line 205, line 330 and AC-14 (line 423) now say "with no proof `basis` member" instead of the `decisive-counterexample` basis. |

after_excerpt (FR-1044-AC-14, line 423): `the composition settles `refuted` with no proof `basis` member, with the witness as its `counterexample` entry, no `replay` member, no certification, no conclusion, category `violation` and exit 10, and its premise items keep their own records in slot order.`
