---
id: FR-1050
title: "Check an identity-bound review process in an extracted native unit"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-031
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-501
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-662
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: references
---
# FR-1050: Check an identity-bound review process in an extracted native unit

## Description

FSL SHALL run a selected temporal clause from an `ix:native` fence embedded in
a Markdown process document and retain the original document identity, the
extracted body identity, the authored clause binding, and the temporal result
in the command result (FR-031, FR-662, FR-267). The temporal clause is a
synthetic example of a review process. Its event records carry explicit
process, repository, case, source-revision, attempt, author, reviewer and
assignment identities. These values are example data; the example defines no
production review policy or identity namespace.

For this example, receipt and acknowledgement are separate event facts. A
receipt alone does not satisfy the acknowledgement predicate. An acknowledgement
qualifies only when it is accepted, its reviewer differs from its author, its
reviewed revision equals the active revision, and a reassigned review has an
accepted assignment. A qualifying acknowledgement must occur within the
fixture's finite time window. These example rules define the test oracle; they
do not establish any external acceptance or live workflow state.

## Inputs

- A Markdown document with one owned `ix:native` fence declaring the example
  record and temporal clause.
- A `native-run/1` request selecting that clause through `program.extraction`
  and `program.clauses`, and a timestamped trace containing the event records
  and progress assertions described by an acceptance criterion.

## Outputs

- The FR-267 `native-run-result/2` document, including the unchanged extraction
  record and one temporal item with the result defined by the matching
  criterion.

## Behavior

- FSL SHALL preserve the original source identity, extracted body identity,
  clause binding and each explicit identity value through extraction and
  temporal evaluation.
- FSL SHALL evaluate the example predicate as written: receipt is not
  acknowledgement; self-review, an acknowledgement bound to a stale source
  revision, and reassignment without accepted assignment do not qualify.
- FSL SHALL distinguish a qualifying event within the closed window, an event
  after the window, and an unresolved trace whose progress or completeness
  does not settle the window, according to FR-501.
- Repeating a request with identical source and trace bytes SHALL produce an
  equal command result.

## Acceptance Criteria

The example uses only synthetic identity values. Let its owned clause be
`review_acceptance`, its original identity be `synthetic:review-process`, its
body identity be `synthetic:review-process#review_acceptance`, its clause
binding name the same clause, and its temporal window be ticks 0 through 5.
Each event carries process, repository, case, source-revision, attempt,
author, reviewer and assignment values. A qualifying event also has
`acknowledged`, `accepted`, `revision_is_current`, and
`assignment_accepted` facts set true, except that `assignment_accepted` is
required only when `reassigned` is true.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1050-AC-1 | A complete closed trace has one qualifying acknowledgement at tick 3. The command exits 0 with category `success` and a completed true temporal item. Its extraction record names the specified original identity, body identity and clause binding; the item retains the synthetic process, repository, case, source-revision, attempt and assignment values. Repeating the identical request produces an equal result document. | Test |
| FR-1050-AC-2 | The complete closed trace contains a receipt event at tick 2 with `acknowledged` false and no qualifying acknowledgement by tick 5. The temporal item is a violation; receipt does not satisfy the acknowledgement predicate. | Test |
| FR-1050-AC-3 | The complete closed trace contains a qualifying acknowledgement only at tick 6. The temporal item is a violation for the 0-through-5 window. | Test |
| FR-1050-AC-4 | The trace contains no qualifying acknowledgement and declares the relevant window incomplete or still open. The item is pending or indeterminate as specified by FR-501's progress, closure and completeness rules; it has no completed truth. | Test |
| FR-1050-AC-5 | Each complete closed trace below has no qualifying acknowledgement by tick 5 and settles violation: (a) the reviewer equals the author; (b) the reviewed source revision differs from the active source revision; or (c) the event is reassigned but its assignment is not accepted. Changing only the failing condition to valid makes the event qualifying and the item succeeds. | Test |

## Dependencies

- [FR-031](FR-031-run-extracted-native-source.md) and
  [FR-662](FR-662-check-every-native-fence-of-a-markdown-specification.md):
  embedded-source extraction and identity.
- [FR-501](FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md):
  temporal settlement under timestamped progress and completeness.
- [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md): checked
  protocol constructs exercised alongside the temporal example.
