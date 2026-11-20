# QSL-2091 delivered behavior

Eight new active tests use real checked models, engine-produced refinement
certificates, real model-check outcomes and EN-5 provider results.

- The simulation checker stops below the certificate's exact position or edge
  count and accepts at the inclusive ceiling. A terminal chain's stutter edge
  consumes a transition unit. A malformed successor-list shape is rejected at
  the missing position even when both recomputation budgets are zero.
- The constrained three-counter cube stores eight expanding 0/1 triples and
  twelve boundary triples. A 19-state budget stops; 20 and 21 complete with
  exactly 20 states, 24 edges and 12 boundaries. Initial boundaries produce no
  deadlock or safety stutter edge. A false initial property is read before the
  constraint can cut expansion and settles refuted through actual replay.
- EN-5 distinguishes `0.95999999999999999999`, `0.96` and
  `0.96000000000000000001` against the Link model's exact probability `24/25`.
  The first two claims are certified with that independently expected value;
  the last refutes. All three checked claims retain `exact_only`.

## Pending behavior

The original reduction fixture selected `invariant` declarations as constraints,
which FR-158's Resolution rule forbids. Its passing reduction receipts do not
back the required constraint behavior, and its liveness refusal is retracted
as QSL-2713 evidence. The inherited fixture and tests remain unchanged.

Only the new child now declares actual native `constraint Low` and
`constraint Started` clauses, using real parsing, checking and `World::new`
under Linked mode. All three supported reduction cases passed this fixture.
The changed liveness case was executed once with this actual declaration
source; it still refused `ReducedLiveness` before exploration. That new receipt
grounds QSL-2713 against FR-158-AC-2, replacing the invalid original evidence.
Its desired initial-boundary oracle remains ignored and excluded from coverage;
no liveness exploration or proof is claimed. The issue blocks QSL-2091 on the
new evidence. Existing Done QSL-913 and QSL-541 were not reopened.

This slice covers concrete FR-149, FR-158 and FR-195 behavior. The ticket's
broader historical refinement, reduction, hyper and statistical families
remain unclaimed by this change. Already delivered QSL-2082 overlaps and
QSL-2086/2087 replay/certificate cases are not duplicated for trace tags.
No matrix or historical criterion recount is completion evidence.

No production, dependency, configuration, binary or external fixture is changed.
Only three existing integration parents register their new child modules.
