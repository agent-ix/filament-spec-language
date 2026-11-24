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
clause run writes and the replay re-derives, byte for byte, returns
`Ok(None)` for a `closed-scope` basis, refuses every other input with a typed
code, cause and pointer in the stated precedence (a range violation first),
and that a replay whose payload carries no record settles the agreement when
the re-derived record is a comparison record while an absent quantifier
record stays a `Mismatch`.

Scope: FR-269-AC-6 to FR-269-AC-10.

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
4. Over the `no-parent` unit (FR-108's `Config` domain package, the `high`
   snapshot and the one invariant `NoParent`, `not present(self.parent)`,
   with `self` `mid`), call the entry, run the clause, and replay the
   envelope with no record.
5. Call the entry on: `AtLeastParent` over `high`; the `MatchesParent`
   envelope whose clause node names `AllBelow`'s node; FR-268-AC-3's
   undefined-domain and refused-domain `AllBelow` variants; `MatchesParent`
   with the accounting limits one unit below the clause evaluation's need.
   Call each a second time.
6. Call the entry on FR-122-AC-7's `VersionAdvances` invocations 0 to -1
   (result `true`) and 1000 to 1001 on `root` (result `false`). In-crate,
   settle a clause check whose disposition is an evaluation fault, without
   and with a range violation.
7. Replay the no-record `MatchesParent` envelope on the `Witness` arm and on
   the `Input` arm, the no-record changed-version `VersionAdvances`
   envelope, and the no-record `MatchesParent` envelope with the evaluation
   meter one unit below the separation check's need. Call the separation
   settlement with outcome `Separation { Element, Unmet }`, `given` absent
   and FR-265-AC-11's record as `derived`.
8. Failing cases: replay FR-268-AC-2's no-record `AllBelow` envelope and the
   `MatchesParent` envelope carrying the record with operands `[600, 1]`.
9. In-crate, settle equal-verdict inputs with a non-comparison record while
   changing only `given`, then only `derived`; also settle with `given` absent
   and the derived record present. Compare the complete records and cause.

Tag step 9 with `#[trace("TC-3121", "FR-269-AC-10")]`.

## Expected Results

| Step | Case | Expected |
|------|------|----------|
| 1 | `MatchesParent`, `AllBelow`, `VersionAdvances` | `Ok(Some(_))` holding FR-265-AC-11's comparison record, FR-265-AC-1's quantifier record, FR-266-AC-2's record of the first `=`; each second call returns a byte-equal record |
| 2 | the three records | equal under FSpec FR-351 identity and the same encoded bytes across the entry, the `/2` document's `witness` and the replay's record |
| 3 | foreign record in the payload | the same record as step 1, the payload's record unread |
| 4 | `NoParent` | `Ok(None)`; the run writes no `witness`; the replay settles `reproduced-with-evaluated-witness` with no record |
| 5 | `AtLeastParent` | `NotRefuted`, `invalid_runtime_input`/`invalid-value`, `/family_payload/observation`, no record |
| 5 | stale clause node | `Replay(ClauseIdentity)`, `stale_dependency`/`content-mismatch` |
| 5 | undefined domain | `Undefined` naming the domain expression, `undefined_expression` |
| 5 | refused domain | `Refused` with the refusal record's code and cause |
| 5 | meter one unit short | `Exhausted`, `resource_exhausted`/`insufficient-next-charge` |
| 5 | second calls | the same refusal |
| 6 | 0 to -1, clause `false` | `RangeViolation` naming `child`, `versionNumber`, `[0, 1000]`, -1; `invalid_runtime_input`/`invalid-value`, `/family_payload/observation`; no record |
| 6 | 1000 to 1001, clause `true` | `RangeViolation` naming `root`, `[0, 1000]`, 1001, never `NotRefuted` |
| 6 | evaluation fault | `Fault`, `runtime_invariant`, no pointer; with a range violation present, `RangeViolation` |
| 7 | no-record `MatchesParent`, `Witness` arm | `reproduced-with-evaluated-witness` holding FR-265-AC-11's record, never `inconclusive` |
| 7 | no-record `MatchesParent`, `Input` arm | `reproduced-without-witness` |
| 7 | no-record `VersionAdvances` | `reproduced-with-evaluated-witness` holding FR-266-AC-2's record |
| 7 | separation meter one unit short | `inconclusive`, `NoValue` |
| 7 | failed separation, `given` absent | `Witness { given: absent, derived: FR-265-AC-11's record, failure: Separation { Element, Unmet } }` |
| 8 | no-record `AllBelow` (failing case) | `inconclusive`, `Witness { given: absent, derived: <index-1 record>, failure: Mismatch }` |
| 8 | operands `[600, 1]` (failing case) | `inconclusive`, `Witness { given: <operands [600, 1]>, derived: FR-265-AC-11's record, failure: Mismatch }` |
| 9 | change only given, change only derived, absent given | each `Witness` disagreement retains the complete unequal or absent records and settles `inconclusive`; the absent comparison-record exception of AC-9 remains unchanged |
