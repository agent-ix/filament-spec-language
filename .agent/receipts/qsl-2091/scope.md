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

QSL-2713 owns state-constraint liveness admission. The original combined
initial-boundary test executed its deadlock and safety controls, then failed
when the eventuality selection returned `ReducedLiveness` before exploration.
Current FR-158-AC-2 requires boundary handling without a stutter lasso. Its
desired initial-boundary oracle is retained as one ignored test, excluded
from coverage; no new liveness run or proof is claimed. The issue blocks
QSL-2091. Existing Done QSL-913 and QSL-541 were not reopened.

This slice covers concrete FR-149, FR-158 and FR-195 behavior. The ticket's
broader historical refinement, reduction, hyper and statistical families
remain unclaimed by this change. Already delivered QSL-2082 overlaps and
QSL-2086/2087 replay/certificate cases are not duplicated for trace tags.
No matrix or historical criterion recount is completion evidence.

No production, dependency, configuration, binary or external fixture is changed.
Only three existing integration parents register their new child modules.
