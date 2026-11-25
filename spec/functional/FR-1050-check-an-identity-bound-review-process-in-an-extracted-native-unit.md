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
in the command result (FR-031, FR-662, FR-267). The selected Quire clause SHALL
bind the embedded body containing both the checked FSL protocol `Review` and
the temporal clause `ReviewAccepted`; the extraction body identity therefore
names the same unit that declares both. The temporal clause is a synthetic
example of a review process. Its event records carry explicit process,
repository, case, source authority, active and reviewed source-revision,
attempt, event, author, reviewer, assignment and provider-event identities.
Provider-event identity fields are synthetic labels for GitHub, Linear, relay
and gate facts. These values are example data; the example defines no
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
| FR-1050-AC-1 | A complete closed trace has one qualifying acknowledgement at tick 3. The command evaluates `ReviewAccepted`; it exits 0 with category `success` and a true temporal result. The checked `Review` protocol is then analyzed from the extracted body bytes under that same body identity and model binding; its protocol result is reproduced. The extraction record names the specified original identity, body identity and clause binding; the verified body bytes retain the protocol declaration and the process, repository, case, source authority, active/reviewed source-revision, attempt, event, assignment and provider-event identity fields. Repeating the identical request produces an equal temporal result document. | Test |
| FR-1050-AC-2 | The complete closed trace contains a receipt event at tick 2 with `acknowledged` false and no qualifying acknowledgement by tick 5. The temporal item is a violation; receipt does not satisfy the acknowledgement predicate. | Test |
| FR-1050-AC-3 | The complete closed trace contains a qualifying acknowledgement only at tick 6. The temporal item is a violation for the 0-through-5 window. | Test |
| FR-1050-AC-4 | With a receipt at tick 3, no qualifying acknowledgement, an open scope, watermark 3 and completeness through 3, the temporal result is `Pending` with deadline 5. With a receipt at tick 3, no qualifying acknowledgement, a closed scope, watermark 5 and `Incomplete` completeness, the temporal result is `Indeterminate` with reason `IncompleteInput`. Neither result has a completed truth. | Test |
| FR-1050-AC-5 | Each complete closed trace below has no qualifying acknowledgement by tick 5 and settles violation: (a) the reviewer equals the author; (b) the reviewed source-revision identity differs from the active source-revision identity; or (c) the event is reassigned but its assignment is not accepted. Changing only the failing condition to valid makes the event qualifying and the temporal result succeeds. | Test |
| FR-1050-AC-6 | Source identity is evaluated, not merely copied: changing the source-authority identity or a provider-event identity from its bound synthetic value while leaving all other event data unchanged makes the complete closed temporal result a violation. | Test |

## Dependencies

- [FR-031](FR-031-run-extracted-native-source.md) and
  [FR-662](FR-662-check-every-native-fence-of-a-markdown-specification.md):
  embedded-source extraction and identity.
- [FR-501](FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md):
  temporal settlement under timestamped progress and completeness.
- [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md): checked
  protocol constructs exercised alongside the temporal example.
