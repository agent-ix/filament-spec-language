---
id: SR-34922
title: "QSL-3492 spec review of the FR-239 fairness change"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language@6cf78369a61fceb593f57374bd8eff9e3bf97c15; spec/functional/FR-239 as changed, read against FR-235, FR-242, FR-244, FR-430, FR-159 and FSpec FR-416"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-239
    type: references
---

## Summary

Ticket: QSL-3492. A hand integrity check of the one requirement the PR changes against the requirements that name the same results. The skill's full procedure was not run.

## Verdict

CONDITIONAL: FR-239 now moves fair TT-1 and TT-2 claims to the liveness product and the neighbouring requirements were not moved with it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A fair TT-1 or TT-2 result now carries non_zeno, which FR-235 and FSpec FR-416 forbid for every TT-1 and TT-2 record | spec/functional/FR-239-check-a-timed-claim-by-symbolic-zone-search.md:162 |
| FND-002 | medium | Fair TT-1 and TT-2 claims under symmetry settle unsupported, and no requirement says so while FR-430 says EN-6 applies symmetry | spec/functional/FR-239-check-a-timed-claim-by-symbolic-zone-search.md:162 |
| FND-003 | low | FR-242 still requires a horizon automaton for every BoundedWindow formula and FR-244 a reachability part for TT-1 and TT-2 | spec/functional/FR-239-check-a-timed-claim-by-symbolic-zone-search.md:162 |

### FND-001

search.rs sets liveness_product for the fair claim, so its proof certificate carries non_zeno (the checker requires it on a liveness certificate), a lasso refutation carries checked and no-admitted-behaviour carries refuted. FR-235 says both "an item EN-6 decides on its liveness product SHALL carry non_zeno" and "every TT-1, TT-2 ... record" carries none; FSpec FR-416 "Non-Zeno premise" says every result of a TT-1 or TT-2 claim carries none. Either the rule or the record has to change.

### FND-002

Refusing is conservative, but FR-159 preserves symmetry for every form and fairness set and FR-430 advertises it for dense-time EN-6. The refusal is stated only in a code comment and the PR body.

### FND-003

FR-242 "A BoundedWindow formula SHALL translate to a timed automaton with one added horizon clock" and FR-244 "Reachability part. For a TT-1, TT-2 or TT-3 item" now disagree with FR-239 for fair claims.
