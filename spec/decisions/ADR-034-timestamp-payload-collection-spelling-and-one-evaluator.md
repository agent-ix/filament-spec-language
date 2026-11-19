---
id: ADR-034
title: "Timestamp as one nanosecond count, native collection spelling in FSL documents, and a retired std evaluator"
type: ADR
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-056
    type: relates_to
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: relates_to
  - target: ix://agent-ix/filament-spec-language/FR-402
    type: relates_to
  - target: ix://agent-ix/filament-spec-language/FR-405
    type: relates_to
  - target: ix://agent-ix/filament-spec-language/AD-001
    type: relates_to
  - target: ix://agent-ix/filament-specification/FR-144
    type: relates_to
  - target: ix://agent-ix/filament-specification/FR-149
    type: relates_to
  - target: ix://agent-ix/filament-specification/FR-181
    type: relates_to
---
# ADR-034: Timestamp as one nanosecond count, native collection spelling in FSL documents, and a retired std evaluator

## Context

This record settles three questions about FSL and FSpec wire forms and FSL's
evaluators.

- **Timestamp wire form.** FSL FR-056 defines the K2 Timestamp payload as one
  exact signed count of nanoseconds since `1970-01-01T00:00:00Z` on the POSIX
  scale, within the signed 128-bit domain, written
  `{"kind":"timestamp","nanoseconds":"<decimal>"}`. FSpec FR-149 keys and
  compares a Timestamp by that count. FSpec FR-181's typed canonical form
  also needs a Timestamp form, for state keys and for supplied collection
  elements. The other candidate was a pair of decimal members, whole seconds
  and a nanosecond remainder, in UTC.
- **Collection spelling.** FSL FR-106's snapshot and invocation documents
  (`filament.state.snapshot/v1`, `filament.state.invocation/v1`) write every value
  in one closed grammar: either single-tag objects (`{"integer": "1"}`,
  `{"sequence": [...]}`) or payload objects that have a `kind` member. FSpec
  FR-144 defines the wire form of a supplied collection as the FR-181 typed
  canonical form, `{"type": "set", "elements": [...]}`, whose elements are
  FR-181 typed forms "with no second grammar". The two specifications can
  disagree about how a set, bag or ordered set is spelled in an FSL
  document.
- **The std reference evaluator.** AD-001 has two implementations of FR-262's
  evaluation semantics while `filament-eval` is built: the std reference
  evaluator in `fsl-eval` and the `no_std` evaluator in `filament-eval`. FR-402
  keeps them equal until FR-405's switch. After the switch, the std evaluator
  could be deleted (retired) or kept in a lesser role (demoted), such as a
  test oracle or a fallback.

## Decision

| ID | Decision |
| --- | --- |
| TS-1 | A Timestamp is written in every wire form as the single signed 128-bit nanosecond count of FSL FR-056, as a decimal string: `{"kind":"timestamp","nanoseconds":"<decimal>"}` in FSL documents and `{"type":"timestamp","nanoseconds":"<decimal>"}` in FSpec FR-181's typed canonical form. No form splits an instant into seconds and nanoseconds, or carries a zone, a calendar text or a precision selector. |
| TS-2 | Each instant has exactly one spelling: `0`, or an optional `-` followed by a nonzero digit and more digits. A reader refuses any other spelling and never normalizes it, so equal instants have equal bytes everywhere: in a state key, in a set element and in a document. |
| CS-1 | In FSL FR-106 documents, a sequence, set, bag or ordered set is written in FR-106's native single-tag spelling: `{"sequence": [...]}`, `{"set": [...]}`, `{"bag": [...]}` and `{"ordered-set": [...]}`. Each element is a value of FR-106's grammar. An FR-106 document never contains an FSpec FR-181 typed form, an object whose `type` member is an FR-181 tag such as `{"type": "set", "elements": [...]}`; such an object is not a value of FR-106's grammar. FR-106's record payload `{"record": {"type": "<declaration identity>", "fields": ...}}` is FR-106's own form, and its `type` names a declaration, not an FR-181 tag. |
| CS-2 | FR-106's collection spelling corresponds one to one with FSpec FR-181's `sequence`, `set`, `bag` and `ordered-set` forms: the kinds are the same, each element maps to its FR-181 typed form, and a bag is written as repeated elements in both. Admission applies FSpec FR-144's "Supplied collection values" rules unchanged: the bound is checked before any element is read; in a set or ordered set a duplicate is refused and never removed, while a bag groups equal elements into one member whose multiplicity is their count; array order is ignored for a set or bag and kept for an ordered set; and an element type with no FR-181 form is refused. |
| CS-3 | FSpec FR-181's typed canonical form is the wire for FSpec's state keys and for an FSpec runtime input (FR-144). FR-144's "no second grammar" applies inside that form: an FR-181 collection's elements are FR-181 typed forms. An FSL document is a separate FSL wire with its own complete grammar, and FSL admission maps each of its values onto the FR-181 value it denotes. |
| EV-1 | FR-405's switch retires the std reference evaluator: it is deleted. It does not remain as a Cargo feature, a `cfg` branch, a runtime or test fallback, a differential oracle, a dev-dependency or a test-support crate. After the switch, `filament-eval` is FSL's only evaluator of checked terms. |
| EV-2 | After the switch, the corpus expectations recorded at the switch base check `filament-eval` (FR-405's regression harness). FR-402's differential comparison runs only before the switch, while both implementations exist. |

## Consequences

- A Timestamp's state-key bytes, set-element bytes and K2 payload are all
  built from one count, so FR-149 equality, the FR-181 encoding and the
  FR-106 document equality agree without any conversion. A reader needs no carry rule
  for a remainder of 10^9 or more, and no sign rule for an instant before the
  epoch with a nonzero remainder.
- FSL documents keep one grammar for every value. A collection element is read
  by the same rules as a field, parameter or result value. Tools that write
  FSL documents never mix `{"type": ...}` and single-tag objects.
- The FR-106 and FR-181 spellings must stay in one-to-one correspondence. A
  new collection kind or element form is added to both, and FR-106 states
  which FR-181 form each native value denotes.
- FSL, RT and CG share one implementation of the evaluation semantics, which
  is AD-001's purpose. No second evaluator needs FR-402 equality after the
  switch, and a fault in `filament-eval` is found by the recorded corpus and the
  requirement tests, not by comparison against a second implementation.

## Alternatives Considered

- **Timestamp as seconds and nanoseconds** (`{"type":"timestamp",
  "seconds":"<decimal>","nanos":"<decimal>"}`, UTC). Rejected. It gives a
  second representation of the K2 payload that FR-056 and FR-149 already
  define. It needs range and normalization rules for `nanos` and for negative
  instants, and keeping a reader canonical needs one more refusal case per
  member.
- **FSpec FR-181 typed forms inside FSL documents** (`{"type": "set",
  "elements": [...]}` in a snapshot). Rejected. Every other FR-106 value is a
  single-tag or `kind` object, so a snapshot would hold two value grammars,
  and FR-106 would need a second reader for collection values and their
  elements.
- **Both spellings in FSL documents.** Rejected. Two spellings of one value
  give two documents for one typed observation, and each would need its own
  refusal rules.
- **Demote the std evaluator to a differential test oracle.** Rejected. It
  keeps two implementations of one semantics in the workspace indefinitely,
  and FR-402 equality would have to be maintained for every later change of
  FR-262.
- **Keep the std evaluator behind a feature or as a fallback.** Rejected for
  the same reason. It also adds an evaluator selector that FR-405 rules out.

## References

- FSL FR-056 (K2 Timestamp payload), FR-106 (snapshot and invocation value
  grammar), FR-402 (differential testing), FR-405 (the switch), AD-001
  (shared evaluation core).
- FSpec FR-144 (supplied collection values), FR-149 (equality and keys),
  FR-181 (typed canonical form and canonical element encoding).
