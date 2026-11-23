---
id: SR-3613001
title: "Spec review of the FR-875 undefined-read gap and FR-186-AC-10 (filament-spec-language PR 3613)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@fcdd6028cb15c8d3ce2e80f481b24ab870512b14; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md (FR-186-AC-10); spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md (Out-of-profile forms (current)); spec/test-cases/TC-2167-an-on-each-claim-settles-from-source.md"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-186
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-875
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-741
    type: references
---

# Spec review of the FR-875 undefined-read gap and FR-186-AC-10 (PR 3613)

## Summary

Ticket: QSL-2979. Scope `git diff origin/main...HEAD` at fcdd6028.

FR-875 undefined-read sentence. FR-741 says "An undefined evaluation refutes
with FR-202's `Undefined` evidence", and FR-875 says "The item SHALL NOT be
settled `unsupported` for its activation". The new sentence extends the
already-merged "(current)" bullet:

- It names a successor ticket (QSL-3003) that builds FR-741's undefined-path
  refutation.
- It settles `unsupported` with the FR-272 diagnostic. That outcome is
  non-committal: it never gives a false proof or a false refutation.

So it is consistent as a declared implementation gap, in the same form as the
merged out-of-profile forms sentence and FR-741's own "(current)" bullet. Its
framing is a finding only for cross-reference (FND-003).

FR-186-AC-10. FR-741 "Binding": every binding in `entries` has the `over`
binding and `instance`, and "`instance` is null for an `on origin` claim".

- The on-each `E` has no `over`, so its entry has a null `binding` and
  `instance` `m`. This is correct and agrees with FR-741-AC-5 (`instance` is
  `m`'s reference and `binding` is null).
- The on-origin `Eventually` entry names `m` as its `binding` only if that
  claim is `over (m: M::Msg)`. That holds for the fixture in
  fsl-analyze/tests/it/probability_from_source.rs:51, but the spec does not
  state it (FND-001).
- 391/400 is the ADR-028 section 15.4 workload value, which I derived by hand
  in SR-3603001.
- TC-2167's Expected Results restate the AC faithfully.
- No other spec text still says the basis "equals" the on-origin claim's.
  FR-196-AC-15 says only that the value equals the on-origin value, which
  stays true.

quire validate on the three changed files exits 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new clause "where the `on origin` claim's entry names `m` as its `binding`" depends on that claim being written `over (m: M::Msg)`. The spec never gives its source: AC-10 says only "the `on origin` claim", and TC-2167 names `Eventually` without defining it. The on-origin claims of ADR-028 section 15.4 have no `over`, so their entries have a null `binding`, and a reader following the spec would expect null there. The fix is to spell the claim in AC-10 and TC-2167: `probabilistic Eventually using inf on M over (m: M::Msg) under Even on origin { probability >= rational(9, 10) [ eventually holds(m.delivered) ] }`. | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:373 |
| FND-002 | low | In "its proof basis carries ... (391/400); its entry names `m` as its `instance`", "its entry" reads as part of the proof basis. The `ExactValue` basis has no binding members. The binding is in FR-741's `entries` result member (FR-203 `ExactEntry{binding, instance}`). Write "its FR-741 `entries` row names `m` as its `instance` ...". | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:373 |
| FND-003 | low | The bullet is titled "Out-of-profile forms (current)", but it now also holds an in-profile gap: an undefined guard or target read. FR-741, which owns the rule "An undefined evaluation refutes with FR-202's `Undefined` evidence", has no pointer to this exception, so a reader of FR-741 expects a refutation. Retitle the bullet (for example "Unbuilt on-each cases (current)") and add a one-line "(current)" cross-reference to FR-875 next to FR-741's undefined-evaluation sentence. | spec/functional/FR-875-settle-each-instance-of-a-probabilistic-on-each-claim.md:103-113 |

## Verdict

The `binding` and `instance` statement is right and agrees with FR-741
"Binding" and FR-741-AC-5, and the "(current)" undefined-read gap is
consistent as an implementation gap. FND-001 should be fixed before merge so
that the AC can be checked from the spec alone. FND-002 and FND-003 are
wording.
