---
id: SR-20261119213624
title: "spec review of PR3105 FR-269 public state-clause witness derivation entry and absent comparison record rule"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; spec/functional/FR-269-settle-a-witness-disagreement-as-a-typed-cause.md, spec/functional/FR-268-check-a-state-clause-witness-on-replay.md, spec/decisions/ADR-031-state-forall-separating-witness.md, spec/test-cases/TC-3121-derive-a-state-clause-witness-through-the-public-entry.md; PR3105 f202c34237222d0c35e90f08e155a88a736bb46f"
review_set: subset
---

## Summary

Ticket: QSL-2549 (ruling comment 92015bb8; code tickets QSL-2611, QSL-2612). The PR adds the public `fsl_replay::derive_state_clause_witness` entry with a closed `DerivationRefusal`, the rule that an absent payload record against a re-derived comparison record settles the agreement, amendments to FR-268, ADR-031 SW-9 and SW-12, FR-269-AC-6 to AC-8 and TC-3121. The front-half claim matches `replay_state_clause` (decode, target, recompile, clause, identities, form, admission, `check_clause_admitted`, whose report already carries `derive_separating_witness`'s record). `compare_witness`, `separate` and `settle_separation` behave as the amended rule assumes, apart from FND-005. Catalog codes and causes exist for every variant. The `NoParent` fixture evaluates `false` over `high` with `self` `mid` and closes scope at the `present` call, which matches the walk in `decision_end` and FR-265-AC-12's precedent. FR-269-AC-6 to AC-8 and TC-3121 are free on main (highest TC there is TC-3084) and in every open PR. `quire validate` is clean on all four files.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The absent-comparison-record rule gives a false reason for treating quantifier and comparison records differently, and leaves SW-8 and FR-268 Inputs ("present exactly when decisive") contradicting the new tolerance | spec/functional/FR-269-settle-a-witness-disagreement-as-a-typed-cause.md:133-151 |
| FND-002 | medium | RangeViolation's precedence over NotRefuted, Undefined, Refused, Exhausted, ClosedScope and evaluation Fault is not stated, and no AC exercises RangeViolation or Fault | spec/functional/FR-269-settle-a-witness-disagreement-as-a-typed-cause.md:195 |
| FND-003 | medium | ClosedScope's "When" text misses the walk that ends at an authored comparison with no receiver path (the code closes scope there), and `end: OccurrenceKey` is not optional although a synthesized end node may have no expression occurrence | spec/functional/FR-269-settle-a-witness-disagreement-as-a-typed-cause.md:118 |
| FND-004 | low | ClosedScope is a valid outcome but is coded `invalid_runtime_input`/`invalid-value`, and its pointer `/family_payload/witness` names a member the entry never reads | spec/functional/FR-269-settle-a-witness-disagreement-as-a-typed-cause.md:118 |
| FND-005 | low | For a failed separation step on an absent payload record, `given` must be absent per Outputs, but the shared `settle_separation` hardcodes `given: Some(record)`; the spec does not say which, and no AC pins it | spec/functional/FR-269-settle-a-witness-disagreement-as-a-typed-cause.md:145-147 |
| FND-006 | low | "NoParent added to the witness unit" does not say whether the shared FR-265 fixture changes; another failing invariant in the shared unit can change other clause-run ACs over that unit | spec/functional/FR-269-settle-a-witness-disagreement-as-a-typed-cause.md:195 |

## Scope and evidence

Diff only (origin/main...f202c34237). Code read at that head: fsl-replay/src/witness/derivation.rs (`derive_separating_witness`, `decision_end`, `occurrence_at`), fsl-replay/src/execute/state_clause.rs (`replay_state_clause`, `compare_witness`, `settle_separation`, `separate`), fsl-replay/src/execute.rs (`ReplayRefusal::code`, `cause`), fsl-replay/src/result.rs (`SeparationRefusal`), fsl-replay/src/spine/call.rs (`CallOutcome`). FSpec FR-351 read on filament-specification origin/main.

- FR-269 "Deriving the record for a producer" (examined): signature, front-half claim, refusal table, determinism and byte-equality.
- FR-269 "An absent comparison record is not a disagreement" (examined).
- FR-269-AC-6, FR-269-AC-7, FR-269-AC-8 (examined).
- FR-268 Inputs bullet and the amended agreement bullets (examined).
- ADR-031 SW-9 and SW-12 amendments (examined); SW-8 and SW-13 (context only).
- TC-3121 (examined).
- FR-265-AC-11, FR-265-AC-12, FR-266-AC-2, FR-268-AC-2, FR-268-AC-3, FSpec FR-351 "Absence" (context only).

### FND-001 detail

The new section says a comparison record "names nothing a producer's search chose" while a quantifier record "names the element the producer claims stops the quantifier". Under SW-2 the quantifier record is equally fixed by the package and the observation: it is the first stop in domain order, and replay already re-derives it and requires equality. The reason given does not separate the two cases, so the asymmetry FR-269-AC-8 tests has no stated basis. Soundness is not weakened: the agreed record is re-derived from the same payload's observation and passes the separation check, and the settled result carries it, so FSpec FR-351 holds at the settlement. What the rule gives up is enforcing SW-8's producer obligation on the payload. SW-8 ("present exactly when the clause's basis is decisive") and FR-268 Inputs still state that obligation unchanged, and FR-269's description still says a disagreement is "never repaired". The ruling rejected relaxing absent-given as weakening the contract, yet also asked for this rule. With the public entry, CG carries the record anyway. Fix: either state SW-8 as a producer obligation that replay tolerates for comparison records, with the real reason, or drop the asymmetry.

### FND-002 detail

`replay_state_clause` settles a range violation ahead of the clause's own outcome: refused, incomplete, undefined, false, true, and even an `EvaluateFault`. The refusal table does not order the variants. So a clause that evaluates `true`, or `false` with a comparison record, over an observation with an out-of-range post-state integer could be read as `NotRefuted`, as `Ok(record)`, or as `RangeViolation`. AC-7 has no RangeViolation or Fault row.

### FND-003 detail

`derive_separating_witness` returns `closed_scope()` when `decision_end` finds an authored comparison but `evaluation.receiver_path()` is `None`. The table's ClosedScope row says the walk ends at a node that is "neither a stopped quantifier nor an authored comparison", so this case falls outside every variant. The walk can also end at a synthesized connective (FR-265-AC-9), or at a `forall` or `exists` with no stop report. `occurrence_at` only finds `expression`-role occurrences, so whether every such end has an `OccurrenceKey` for `end` is unstated.

### FND-005 detail

`settle_separation(Failed)` builds `Disagrees { given: Some(record), derived: Some(record), .. }`. On the new absent-payload path, FR-269 Outputs requires `given` to be "the payload's record, or its absence", so reusing the helper as-is would misreport the payload.

## Verdict

The design follows ruling 92015bb8. The entry's front-half and derivation claims hold against the code, the catalog codes exist, the ACs can fail (the foreign `[600, 1]` record, the no-record `AllBelow` Mismatch, the meter boundaries), and the ids are free. Not mergeable as is: fix FND-001 and FND-002 (and preferably FND-003) in one round. FND-004 to FND-006 are wording.
