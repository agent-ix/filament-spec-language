---
id: TC-3121
title: "A producer derives the state-clause witness through the public entry, and an absent comparison record settles the agreement"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-269
    type: verifies
---
# TC-3121: A producer derives the state-clause witness through the public entry, and an absent comparison record settles the agreement

## Description

Verify that `fsl_replay::derive_state_clause_witness` returns the record the
clause run writes and the replay re-derives, byte for byte, refuses every
input that has no record with a typed code, cause and pointer, and that a
replay whose payload carries no record settles the agreement when the
re-derived record is a comparison record while an absent quantifier record
stays a `Mismatch`.

Scope: FR-269-AC-6 to FR-269-AC-8.

## Test Procedure

1. Over FR-265's `witness` unit, build the request and `Witness`-arm envelope
   for `MatchesParent` over `high` (`self` `mid`) with `witness: None`, and
   for `AllBelow` over `high`; over FR-108's corpus, the changed-version
   `VersionAdvances` envelope with no record. Call
   `derive_state_clause_witness` on each, twice.
2. Run each clause by FR-109 over the same observation and write
   `native-run-result/2`; replay each step 1 envelope with its record set to
   the returned record. Encode the returned record, the document's `witness`
   member and the replay's record through the RFC 8785 encoder.
3. Call the entry on the `MatchesParent` envelope carrying a record with
   operands `[600, 1]`.
4. Call the entry on: `AtLeastParent` over `high`; the invariant `NoParent`,
   `not present(self.parent)`, added to the unit, over `high` with `self`
   `mid`; the `MatchesParent` envelope whose clause node names `AllBelow`'s
   node; FR-268-AC-3's undefined-domain and refused-domain `AllBelow`
   variants; `MatchesParent` with the accounting limits one unit below the
   clause evaluation's need. Call each a second time.
5. Replay the no-record `MatchesParent` envelope on the `Witness` arm and on
   the `Input` arm, the no-record changed-version `VersionAdvances`
   envelope, and the no-record `MatchesParent` envelope with the evaluation
   meter one unit below the separation check's need.
6. Failing cases: replay FR-268-AC-2's no-record `AllBelow` envelope and the
   `MatchesParent` envelope carrying the record with operands `[600, 1]`.

## Expected Results

| Step | Case | Expected |
|------|------|----------|
| 1 | `MatchesParent`, `AllBelow`, `VersionAdvances` | FR-265-AC-11's comparison record, FR-265-AC-1's quantifier record, FR-266-AC-2's record of the first `=`; each second call returns a byte-equal record |
| 2 | the three records | equal under FSpec FR-351 identity and the same encoded bytes across the entry, the `/2` document's `witness` and the replay's record |
| 3 | foreign record in the payload | the same record as step 1, the payload's record unread |
| 4 | `AtLeastParent` | `NotRefuted`, `invalid_runtime_input`/`invalid-value`, `/family_payload/observation`, no record |
| 4 | `NoParent` | `ClosedScope` naming the `present` call's occurrence key, `invalid_runtime_input`/`invalid-value`, `/family_payload/witness` |
| 4 | stale clause node | `Replay(ClauseIdentity)`, `stale_dependency`/`content-mismatch` |
| 4 | undefined domain | `Undefined` naming the domain expression, `undefined_expression` |
| 4 | refused domain | `Refused` with the refusal record's code and cause |
| 4 | meter one unit short | `Exhausted`, `resource_exhausted`/`insufficient-next-charge` |
| 4 | second calls | the same refusal |
| 5 | no-record `MatchesParent`, `Witness` arm | `reproduced-with-evaluated-witness` holding FR-265-AC-11's record, never `inconclusive` |
| 5 | no-record `MatchesParent`, `Input` arm | `reproduced-without-witness` |
| 5 | no-record `VersionAdvances` | `reproduced-with-evaluated-witness` holding FR-266-AC-2's record |
| 5 | separation meter one unit short | `inconclusive`, `NoValue` |
| 6 | no-record `AllBelow` (failing case) | `inconclusive`, `Witness { given: absent, derived: <index-1 record>, failure: Mismatch }` |
| 6 | operands `[600, 1]` (failing case) | `inconclusive`, `Witness { given: <operands [600, 1]>, derived: FR-265-AC-11's record, failure: Mismatch }` |
