---
id: FR-269
title: "Settle replay disagreements as typed DisagreementCause variants"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-010
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-031
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-351
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-450
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-265
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-122
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-266
    type: depends_on
---
# FR-269: Settle replay disagreements as typed DisagreementCause variants

## Description

FR-072's shared `DisagreementCause` SHALL carry typed replay
disagreements. This requirement defines its `Witness` variant; FR-450 adds
the `HybridTrace` variant and extends this same cause codec. Each
disagreement settles `inconclusive` and is never repaired, except the
comparison-record tolerance below: a payload with no record whose re-derived
record is a comparison record is not a disagreement. So that a producer outside FSL can carry the record
the replay expects, FSL SHALL expose the state-clause witness derivation as
a public entry (ADR-031 SW-8, SW-9 and SW-12).

## Inputs

- The payload's record and the re-derived record, each
  `Option<SeparatingWitnessRecord>`.
- For a separation-check failure, the failing step.
- For the public derivation entry: FR-098's request wire, the FR-122
  envelope and the replay limits.

## Outputs

`DisagreementCause::Witness` with:

- `proved` and `replayed`: the two verdicts, which are equal;
- `given`: the payload's record, or its absence;
- `derived`: the re-derived record, or its absence;
- `failure`: `Mismatch` when the records differ or one is absent (except an
  absent payload record against a re-derived comparison record, which is
  not a disagreement; see "Replay tolerates an absent comparison record"),
  or
  `Separation { step, reason }` naming the separation-check step that
  failed, `Quantifier`, `Domain`, `Element` or `Body` (FR-268 steps 1 to 4),
  and why: `Unmet` when the step's requirement does not hold,
  `UndefinedEvaluation { where, cause }` when the step's evaluation ends
  `undefined`, or `Refused(record)` when it is refused. An exhausted meter is
  not a `Witness` failure; it settles `NoValue` (FR-268).

## Behavior

- `DisagreementCause::proved` and `replayed` SHALL return the
  `Witness` variant's verdicts, matched with no `_` arm.
- A `Witness` cause SHALL settle `inconclusive`. A `Witness`-arm result
  with this cause SHALL carry no FSpec FR-351 record of its own, so no
  consumer reads a disputed record as evidence.
- The cause SHALL serialize and read back with `given` and `derived` each
  present in full or absent (FSpec FR-351 has no partial record), and a
  reader SHALL refuse a `Witness` cause whose `failure` is not one of the
  values above, and a record whose deciding element is a record or tuple
  whose slots do not fit its declaration's shape (FSpec FR-351-AC-5).
- Two results that differ only in their `Witness` cause's `given` or
  `derived` record SHALL compare unequal.

### Deriving the record for a producer

ADR-031 SW-8 makes the payload's record a producer obligation: a producer
that builds a state-clause counterexample SHALL carry the record exactly
when the clause's basis is decisive. A producer outside FSL (for example
contract-codegen's replay of a proof counterexample) SHALL obtain that record
from FSL rather than build it itself. FSL exposes the one derivation the
clause run and the replay share (FR-265, ADR-031 SW-9) as a public entry of
`fsl-replay`, defined beside that derivation in
`fsl-replay/src/witness/derivation.rs` and exported at the crate root:

```rust
pub fn derive_state_clause_witness(
    wire: ReplayRequestWire,
    envelope: &WitnessEnvelope<StateClauseCounterexample>,
    replay_limits: ReplayLimits,
) -> Result<Option<SeparatingWitnessRecord>, DerivationRefusal>;
```

The inputs are the checked subject and the refuting payload: `wire` is the
FR-098 request that names the package, the `state_clause` replay target and
the byte provision, and `envelope` is the FR-122 counterexample whose
payload names the observation. The entry decodes the request, recompiles the
package, resolves the clause, checks the envelope's identities and
observation form, admits the observation and evaluates the clause once,
exactly as FR-122's replay does before it settles, then applies FR-265's
derivation to that evaluation. It reads no `witness` member of the payload,
so a producer calls it with `witness: None` and stores the result in the
payload it sends.

`Ok(Some(record))` is the record of a clause that evaluates `false` with a
decisive basis. `Ok(None)` is a clause that evaluates `false` with basis
`closed-scope`: the correct payload carries no record, and that is a valid
outcome, not a refusal. FR-265 settles `closed-scope` whenever its walk does
not end at a stopped quantifier or at an authored comparison with a receiver
path: it ends at a call, a member read or a literal; at a connective the
checker synthesized (FR-265-AC-9); at a `forall` or `exists` with no stop
report; or at an authored comparison when the evaluation has no receiver
path for the clause's `self`. None of these needs an occurrence key.

`DerivationRefusal` is a closed enum. Each variant has a catalog code, a
cause and a pointer, through `code()`, `cause()` and `pointer()`. The entry
settles in this order, the first that applies winning, exactly as FR-122's
replay does:

1. `Replay(ReplayRefusal)`: FR-122 refuses the request or envelope before the
   clause evaluates (decode, replay target, recompile, unknown clause, stale
   clause identity, wrong observation form, admission).
2. `RangeViolation(Vec<OutOfRange>)`: an admitted post-state integer is
   outside its declared range (FR-106 check 6.5). The violation is that
   value, not a clause decision, and carries no record (FR-122), so this
   refusal wins over every outcome of the clause's own evaluation, `true`,
   `false`, refused, incomplete, undefined or a fault.
3. The clause's own outcome: `Fault` for an evaluation fault, `Refused` for
   a refused evaluation, `Exhausted` for an incomplete one, `Undefined` for
   an undefined one, `NotRefuted` for `true`, and for `false` the
   derivation's `Ok` result.

| Variant | When | Code / cause | Pointer |
|---|---|---|---|
| `Replay(ReplayRefusal)` | step 1 | that refusal's own code and cause | that refusal's own pointer, or none |
| `RangeViolation(Vec<OutOfRange>)` | step 2 | `invalid_runtime_input` / `invalid-value` | `/family_payload/observation` |
| `Fault(InternalFault)` | an executor or S6a invariant broke, in the evaluation or the derivation | `runtime_invariant` | none |
| `Refused(SeparationRefusal)` | the clause evaluation is refused | the refusal record's code and cause | `/family_payload/observation` |
| `Exhausted` | the evaluation meter built from the request's accounting limits runs out | `resource_exhausted` / `insufficient-next-charge` | `/family_payload/observation` |
| `Undefined { expression, cause }` | the clause evaluation ends `undefined` | `undefined_expression` / the undefined cause | `/family_payload/observation` |
| `NotRefuted` | the clause evaluates `true` over the observation, so the payload is no counterexample | `invalid_runtime_input` / `invalid-value` | `/family_payload/observation` |

The pointers name the envelope's ADR-013 O-25 members. A refusal returns no
partial record.

The entry SHALL be deterministic: the same `wire`, `envelope` and
`replay_limits` give the same `Ok` value or the same refusal. A record it
returns SHALL be equal under FSpec FR-351 identity to, and SHALL encode
through ADR-013 §2's RFC 8785 encoder to the same bytes as, the record the
FR-109 clause run writes on `native-run-result/2` for the same clause and
observation (FR-266, FR-267) and the record FR-268's replay derives for the
same request and envelope; `Ok(None)` matches a run with no `witness` and a
replay that derives none. Two callers holding the same inputs therefore hold
byte-equal records.

### Replay tolerates an absent comparison record

SW-8 stays a producer obligation, and the public entry lets every producer
meet it. Replay nevertheless tolerates one breach of it. When the replayed
verdict agrees, the payload carries no record and the re-derived record is a
comparison record, the replay SHALL run FR-268's comparison separation check
on the re-derived record and, when it passes, SHALL settle the agreement
holding the re-derived record (`reproduced-with-evaluated-witness` on the
`Witness` arm, `reproduced-without-witness` on the `Input` arm). It SHALL NOT
settle `inconclusive` with a `Witness` `Mismatch` for that absence.

This adds no trust. The record is re-derived from the same payload's
observation, it passes the separation check on its own terms, and the
settled result carries it, so the result is never a witness-free decisive
settlement (FSpec FR-351 "Absence"). What replay gives up for this case is
only enforcing the producer's obligation on the payload.

If the separation check fails on that path, the replay SHALL settle
`Witness` with `failure` `Separation { step, reason }`, `given` absent (the
payload's record, which is absent) and `derived` the re-derived record; the
separation settlement therefore takes the payload's record as its own
`Option`, never copying the re-derived record into `given`. An exhausted
meter settles `NoValue`.

The tolerance is limited to comparison records on purpose. It is a scope
limit, not a soundness distinction: a quantifier record is fixed by SW-2 just
as fully and would be re-derived and checked the same way. An absent payload
record against a re-derived quantifier record therefore remains a
`Mismatch` (FR-269-AC-1 is unchanged), as do a present payload record
against an absent re-derived one and two present records that differ. SW-8,
FR-268 "Inputs" and this requirement's statement that a disagreement is
never repaired each hold except for this comparison-record tolerance.

### Hybrid trace disagreement

FR-450 adds `DisagreementCause::HybridTrace(HybridTraceDisagreement)`
directly to this shared cause type and codec. Its wire shape is
`{"cause":"hybrid-trace","disagreement":{"type":<arm>,"counterexample":{"type":"hybrid-counterexample","kind":...,"trace":...}}}`.
The `disagreement.type` is exactly `not-reproduced` or `not-exact`.
`disagreement.counterexample` is the complete FSpec FR-193
`HybridCounterexample`, including both `kind` and `trace`; the trace uses
the FR-193 named wire form. It is never projected to a verdict-only or
witness cause. The enclosing terminal cause remains `replay-parity`.

The shared cause codec SHALL extend its existing methods directly:
`DisagreementCause::to_json(hybrid_model: Option<&CheckedHybridModel>)`
and `DisagreementCause::from_json(text, package, hybrid_model: Option<&CheckedHybridModel>)`.
The caller supplies the checked model bound to the routed hybrid claim. A
`HybridTrace` cause requires that model; absence is refused. For every
other cause, the existing package context continues to resolve package
declarations and the optional model is unused. No adapter or parallel
hybrid-only cause codec is introduced.

When writing a hybrid trace, the codec maps each typed mode index to that
mode's name in the supplied checked model and each enclosure position to the
name of the corresponding checked variable. A transition remains its
zero-based position in the previous mode's transition list. Reading resolves
those names in the same checked model and reconstructs the typed indices.
Unknown or out-of-range mode, variable, or transition references are refused.
A round trip with the same checked model returns an equal typed cause.
`DisagreementCause::proved` and `replayed` return `Some(verdict)` for
`Verdicts`, `NoValue` and `Witness`, and `None` for `HybridTrace`, which
has no pair of verdicts. This preserves the typed cause without inventing a
verdict for an exact-replay failure.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-269-AC-1 | FR-268-AC-2's index-2 replay settles `inconclusive` with `Witness { proved: violation, replayed: violation, given: <index-2 record>, derived: <index-1 record>, failure: Mismatch }`, and its `Witness`-arm result carries no FSpec FR-351 record; the no-record replay holds `given` absent and `derived` the index-1 record. | Test (TC-744) |
| FR-269-AC-2 | FR-268-AC-3's failing records map to `failure` `Separation { Quantifier, Unmet }`, `Separation { Element, Unmet }` (three times) and `Separation { Body, Unmet }`; its undefined-domain, undefined-body and refused-domain variants map to `Separation { Domain, UndefinedEvaluation }`, `Separation { Body, UndefinedEvaluation }` and `Separation { Domain, Refused }`, each `UndefinedEvaluation` naming the expression and its undefined cause. | Test (TC-744) |
| FR-269-AC-3 | A replay result holding FR-269-AC-1's cause round-trips through serialize and read to an equal result; the same document with `failure` set to an undefined value refuses, and one with `given` missing its `index` refuses; two results that differ only in `derived` compare unequal. | Test (TC-744) |
| FR-269-AC-4 | A deciding element whose value or type nests 100,000 levels round-trips through the cause codec on a 512 KiB stack: the writer is `quire-canonical`'s, the reader keeps the document on the heap (`quire_canonical::read`) and decodes each deciding element from it on an explicit stack, and the element reads back and encodes to the same document. No depth refuses (ADR-030 D-1). | Test (TC-744) |
| FR-269-AC-5 | A provider formula counterexample whose exact replay returns `NotReproduced` round-trips as `DisagreementCause::HybridTrace(NotReproduced{counterexample})` through the shared `DisagreementCause::to_json(Some(checked_model))` and `from_json(text, package, Some(checked_model))` methods. The cause wire has `cause: hybrid-trace`, a `disagreement.type` discriminator, and the full `disagreement.counterexample` in FR-193 wire form, including `type: hybrid-counterexample`; mode and variable names map through the supplied checked model. The `proved` and `replayed` accessors return `None` for this cause. The `not-exact` discriminator round-trips to its distinct arm. The same hybrid cause without checked-model context is refused; the existing `Verdicts`, `NoValue` and `Witness` cause round trips remain unchanged. | Test (TC-1221) |
| FR-269-AC-6 | Over FR-265's `witness` unit, `fsl_replay::derive_state_clause_witness` called with the request and `Witness`-arm envelope for `MatchesParent` over `high` whose payload carries `witness: None` returns `Ok(Some(_))` holding FR-265-AC-11's comparison record, and for `AllBelow` over `high` `Ok(Some(_))` holding FR-265-AC-1's quantifier record; over FR-108's corpus, the changed-version `VersionAdvances` envelope with no record returns `Ok(Some(_))` holding FR-266-AC-2's comparison record of the first `=`. Each returned record is equal under FSpec FR-351 identity to, and encodes through the RFC 8785 encoder to the same bytes as, the `witness` member of the FR-109 clause run's `native-run-result/2` document for that clause and observation and the record the FR-122 replay of the same request and envelope holds. The `MatchesParent` envelope carrying a different record (operands `[600, 1]`) returns the same record as with `None`, and two calls with equal inputs return byte-equal records. Over the `no-parent` unit (FR-108's `Config` domain package and `high` snapshot, and the one invariant `NoParent`, `not present(self.parent)`, on `Config::ConfigVersion` at `current`, with `self` `mid`), the clause is `false` and its walk passes the `not` and ends at the `present` call, so the entry returns `Ok(None)`, the clause run writes no `witness`, and the replay of that envelope with no record settles `reproduced-with-evaluated-witness` with no record. | Test (TC-3121) |
| FR-269-AC-7 | `derive_state_clause_witness` refuses with no record, each refusal's `code()`, `cause()` and `pointer()` as given: `AtLeastParent` over `high` (the clause holds) `NotRefuted`, `invalid_runtime_input`/`invalid-value`, `/family_payload/observation`; the `MatchesParent` envelope whose clause node names `AllBelow`'s node `Replay(ClauseIdentity)`, `stale_dependency`/`content-mismatch`; FR-268-AC-3's `AllBelow` variant whose domain evaluates `undefined` on `high` `Undefined` naming that expression, `undefined_expression`; its refused-domain variant `Refused` with the refusal record's code and cause; and `MatchesParent` with the accounting limits one unit below the clause evaluation's need `Exhausted`, `resource_exhausted`/`insufficient-next-charge`. Each refusal is the same on a second call. | Test (TC-3121) |
| FR-269-AC-8 | Range violations settle first: over FR-122-AC-7's invocations, `VersionAdvances` taking `child.versionNumber` from 0 to -1 with result `true` (the clause is `false` over the exact -1 and would otherwise yield a comparison record) and taking `root.versionNumber` from 1000 to 1001 with result `false` (the clause is `true` and would otherwise refuse `NotRefuted`) each refuse `RangeViolation` holding FR-122-AC-7's one range violation, `invalid_runtime_input`/`invalid-value`, `/family_payload/observation`. In-crate, the entry's settlement over a clause check whose disposition is an evaluation fault refuses `Fault`, `runtime_invariant`, with no pointer, and the same disposition with a range violation present refuses `RangeViolation`. | Test (TC-3121) |
| FR-269-AC-9 | An absent comparison record settles the agreement, never `inconclusive`: the `Witness`-arm `MatchesParent` over `high` envelope whose payload carries no record settles `reproduced-with-evaluated-witness` holding FR-265-AC-11's record, and on the `Input` arm `reproduced-without-witness`; FR-108's changed-version `VersionAdvances` envelope with no record settles `reproduced-with-evaluated-witness` holding FR-266-AC-2's record; the same `MatchesParent` envelope with the evaluation meter one unit below the separation check's need settles `inconclusive`, `NoValue`. The separation settlement called on its own with a failing outcome (`Separation { Element, Unmet }`), `given` absent and FR-265-AC-11's record as `derived` gives `Witness { given: absent, derived: <that record>, failure: Separation { Element, Unmet } }`, never `given` present. FR-268-AC-2's no-record `AllBelow` envelope, whose re-derived record is a quantifier record, still settles FR-269-AC-1's `Witness` `Mismatch` with `given` absent, and a `MatchesParent` envelope carrying a record with operands `[600, 1]` settles `Witness` `Mismatch` holding both records. | Test (TC-3121) |

| FR-269-AC-10 | With equal verdicts and a re-derived non-comparison record, a payload whose `given` record alone differs settles a `Witness` disagreement with both complete records present and unequal; a payload whose `derived` record alone differs does the same. A payload lacking `given` while the re-derived record exists also disagrees. The absent-payload comparison-record exception remains as AC-9 specifies. | Test (TC-3121) |
## Dependencies

- [ADR-031](../decisions/ADR-031-state-forall-separating-witness.md) SW-8,
  SW-9, SW-12 and SW-13.
- [FR-265](FR-265-derive-a-state-clause-separating-witness.md) (the
  derivation the public entry exposes),
  [FR-122](FR-122-replay-a-state-clause-counterexample.md) (the replay whose
  front half the entry runs), [FR-266](FR-266-report-a-clause-run-s-witness-and-settlement-basis.md)
  (the clause run whose record the entry's record equals).
- [FR-072](FR-072-implement-typed-replay-result.md) (the result and its
  causes), [FR-268](FR-268-check-a-state-clause-witness-on-replay.md) (the
  replay that settles it).
- FSpec FR-351 (record identity and strict reading).

The cause's own codec round-trips here. The round-trip of a whole replay
result holding the cause lands with the `native-run-result/2` wire
([FR-267](FR-267-write-run-results-as-native-run-result-2.md)).

## References

- Owning ticket: Linear QSL-389. Implementation: Linear QSL-45.
