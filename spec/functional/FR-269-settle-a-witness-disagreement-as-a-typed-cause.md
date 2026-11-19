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
disagreement settles `inconclusive` and is never repaired. It also fixes
what is not a disagreement: a payload with no record whose re-derived record
is a comparison record. So that a producer outside FSL can carry the record
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
  not a disagreement; see below), or
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

A producer outside FSL that builds a state-clause counterexample (for
example contract-codegen's replay of a proof counterexample) SHALL obtain
the payload's record from FSL rather than build it itself. FSL exposes the
one derivation the clause run and the replay share (FR-265, ADR-031 SW-9) as
a public entry of `fsl-replay`, defined beside that derivation in
`fsl-replay/src/witness/derivation.rs` and exported at the crate root:

```rust
pub fn derive_state_clause_witness(
    wire: ReplayRequestWire,
    envelope: &WitnessEnvelope<StateClauseCounterexample>,
    replay_limits: ReplayLimits,
) -> Result<SeparatingWitnessRecord, DerivationRefusal>;
```

The inputs are the checked subject and the refuting payload: `wire` is the
FR-098 request that names the package, the `state_clause` replay target and
the byte provision, and `envelope` is the FR-122 counterexample whose
payload names the observation. The entry decodes the request, recompiles the
package, resolves the clause, checks the envelope's identities and
observation form, admits the observation and evaluates the clause once,
exactly as FR-122's replay does before it settles, then applies FR-265's
derivation to that evaluation. It reads no `witness` member of the payload,
so a producer calls it with `witness: None` and stores the returned record
in the payload it sends. Because the entry is the replay's own front half and
FR-265's own derivation, the record it returns is the record the replay
re-derives for the same request and envelope, and FR-268's comparison
agrees.

`DerivationRefusal` is a closed enum. Each variant has a catalog code, a
cause and a pointer, through `code()`, `cause()` and `pointer()`:

| Variant | When | Code / cause | Pointer |
|---|---|---|---|
| `Replay(ReplayRefusal)` | FR-122 refuses the request or envelope before the clause evaluates: decode, replay target, recompile, unknown clause, stale clause identity, wrong observation form, admission | that refusal's own code and cause | that refusal's own pointer, or none |
| `NotRefuted` | the clause evaluates `true` over the observation, so the payload is no counterexample | `invalid_runtime_input` / `invalid-value` | `/family_payload/observation` |
| `RangeViolation(Vec<OutOfRange>)` | an admitted post-state integer is outside its declared range (FR-106 check 6.5); the violation is that value, not a clause decision, and carries no record (FR-122) | `invalid_runtime_input` / `invalid-value` | `/family_payload/observation` |
| `Undefined { expression, cause }` | the clause evaluation ends `undefined` | `undefined_expression` / the undefined cause | `/family_payload/observation` |
| `Refused(SeparationRefusal)` | the clause evaluation is refused | the refusal record's code and cause | `/family_payload/observation` |
| `Exhausted` | the evaluation meter built from the request's accounting limits runs out | `resource_exhausted` / `insufficient-next-charge` | `/family_payload/observation` |
| `ClosedScope { end: OccurrenceKey }` | the clause evaluates `false` but FR-265's walk ends at a node that is neither a stopped quantifier nor an authored comparison, so the basis is `closed-scope` and the correct payload carries no record | `invalid_runtime_input` / `invalid-value` | `/family_payload/witness` |
| `Fault(InternalFault)` | an executor or S6a invariant broke | `runtime_invariant` | none |

The pointers name the envelope's ADR-013 O-25 members. A refusal returns no
partial record.

The entry SHALL be deterministic: the same `wire`, `envelope` and
`replay_limits` give the same `Ok` record or the same refusal. The record it
returns SHALL be equal under FSpec FR-351 identity to, and SHALL encode
through ADR-013 §2's RFC 8785 encoder to the same bytes as, the record the
FR-109 clause run writes on `native-run-result/2` for the same clause and
observation (FR-266, FR-267) and the record FR-268's replay derives for the
same request and envelope. Two callers holding the same inputs therefore
hold byte-equal records.

### An absent comparison record is not a disagreement

A comparison record (FR-265 "Outputs") names nothing a producer's search
chose: its comparison, its decision path and its operand values are fixed by
the package and the closed observation. A quantifier record names the
element the producer claims stops the quantifier, and replay checks that
claim. So when the replayed verdict agrees, the payload carries no record
and the re-derived record is a comparison record, the replay SHALL run
FR-268's comparison separation check on the re-derived record and, when it
passes, SHALL settle the agreement holding the re-derived record
(`reproduced-with-evaluated-witness` on the `Witness` arm,
`reproduced-without-witness` on the `Input` arm). It SHALL NOT settle
`inconclusive` with a `Witness` `Mismatch` for that absence. A failed
separation step still settles `Witness` with `Separation`, and an exhausted
meter `NoValue`. The settled result then carries the decisive record, so it
is never a witness-free decisive settlement (FSpec FR-351 "Absence").
An absent payload record against a re-derived quantifier record, a present
payload record against an absent re-derived one, and two present records
that differ each remain a `Mismatch` (FR-269-AC-1).

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
| FR-269-AC-6 | Over FR-265's `witness` unit, `fsl_replay::derive_state_clause_witness` called with the request and `Witness`-arm envelope for `MatchesParent` over `high` whose payload carries `witness: None` returns FR-265-AC-11's comparison record, and called for `AllBelow` over `high` returns FR-265-AC-1's quantifier record; over FR-108's corpus, the changed-version `VersionAdvances` envelope with no record returns FR-266-AC-2's comparison record of the first `=`. Each returned record is equal under FSpec FR-351 identity to, and encodes through the RFC 8785 encoder to the same bytes as, the `witness` member of the FR-109 clause run's `native-run-result/2` document for that clause and observation and the record the FR-122 replay of the same request and envelope holds. The `MatchesParent` envelope carrying a different record (operands `[600, 1]`) returns the same record as with `None`, and two calls with equal inputs return byte-equal records. | Test (TC-3121) |
| FR-269-AC-7 | `derive_state_clause_witness` refuses with no record, each refusal's `code()`, `cause()` and `pointer()` as given: `AtLeastParent` over `high` (the clause holds) `NotRefuted`, `invalid_runtime_input`/`invalid-value`, `/family_payload/observation`; the invariant `NoParent`, `not present(self.parent)`, added to the `witness` unit and evaluated over `high` with `self` `mid` (`false`, its walk passing the `not` and ending at the `present` call) `ClosedScope` naming that call's occurrence key, `invalid_runtime_input`/`invalid-value`, `/family_payload/witness`; the `MatchesParent` envelope whose clause node names `AllBelow`'s node `Replay(ClauseIdentity)`, `stale_dependency`/`content-mismatch`; FR-268-AC-3's `AllBelow` variant whose domain evaluates `undefined` on `high` `Undefined` naming that expression, `undefined_expression`; its refused-domain variant `Refused` with the refusal record's code and cause; and `MatchesParent` with the accounting limits one unit below the clause evaluation's need `Exhausted`, `resource_exhausted`/`insufficient-next-charge`. Each refusal is the same on a second call. | Test (TC-3121) |
| FR-269-AC-8 | An absent comparison record settles the agreement, never `inconclusive`: the `Witness`-arm `MatchesParent` over `high` envelope whose payload carries no record settles `reproduced-with-evaluated-witness` holding FR-265-AC-11's record, and on the `Input` arm `reproduced-without-witness`; FR-108's changed-version `VersionAdvances` envelope with no record settles `reproduced-with-evaluated-witness` holding FR-266-AC-2's record; the same `MatchesParent` envelope with the evaluation meter one unit below the separation check's need settles `inconclusive`, `NoValue`. FR-268-AC-2's no-record `AllBelow` envelope, whose re-derived record is a quantifier record, still settles FR-269-AC-1's `Witness` `Mismatch` with `given` absent, and a `MatchesParent` envelope carrying a record with operands `[600, 1]` settles `Witness` `Mismatch` holding both records. | Test (TC-3121) |

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
