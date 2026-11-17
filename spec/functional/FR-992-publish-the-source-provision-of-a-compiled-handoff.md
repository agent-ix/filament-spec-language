---
id: FR-992
title: "Publish the source provision of a compiled handoff"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-991
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-896
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-433
    type: depends_on
---
# FR-992: Publish the source provision of a compiled handoff

## Description

`fsl_command::checked_handoff::CompiledHandoff` SHALL publish the source
bytes the handoff compiled from, keyed by the source labels the checked
package declares. A library that holds a handoff can then supply them
wherever FSL reads sources by label: an `analyze` request's provision, and
the byte provision of a replay request that recompiles from source.

```rust
impl CompiledHandoff {
    /// The sources this handoff compiled from, by declared source label.
    pub fn sources(&self) -> &fsl_analyze::analyze::SourceProvision;
}

impl fsl_analyze::analyze::SourceProvision {
    /// Every entry as `(authority, identity, bytes)`, in label order.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &str, &[u8])> + '_;
}
```

### Contract

- **Exactly the compiled sources.** `sources()` holds one entry for each
  source label that the checked package (`checked()`) names: the unit, and
  each library that `selection.json` names. It holds no other entry.
- **Exact bytes.** Each entry's bytes are the bytes `compile()` read and
  compiled. They are the bytes that compile to `checked-package.json`,
  because `compile()` returns a handoff only when the recompiled package
  equals that file.
- **By label, never by path.** An entry is keyed by the `{authority,
  identity}` that the package declares for it. The file name in
  `selection.json` is never the key. `SourceProvision::get` with any
  `RawSourceRef` from the package's provenance returns that source's bytes.
- **Wire form.** `entries()` yields `(authority, identity, bytes)` in
  ascending label order. Collected as owned values, it is a valid
  `ReplayRequestWire::byte_provision`, the form the `analyze` request wire
  builds internally.
- **Read-only.** The provision is owned by the handoff and cannot change
  after `compile()`. `checked()`, `admitted()`, `packages()` and
  `sources()` describe one compile.

### Decision: publish the provision FSL already keys sources by

Replay paths that recompile from source need the unit's bytes under the
labels the package records. A handoff library has those bytes only as
files named by `selection.json`, and rebuilding the label map from file
names would copy FSL's selection logic. `CompiledHandoff` already held the
bytes, so publishing them as FSL's own `SourceProvision` gives callers the
exact keys and bytes FSL uses, with no second mapping. `entries()` makes
the same provision usable as a replay request's byte provision.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-992-AC-1 | `CompiledHandoff::sources()` and `SourceProvision::entries()` are public with the signatures above. | Test (TC-2804) |
| FR-992-AC-2 | For a handoff written from the order fixture with one supplied library, `sources()` holds exactly two entries. The unit's bytes equal the unit file's bytes, the library's bytes equal the library file's bytes, and `get` with the unit's `RawSourceRef` from the package's provenance returns the unit's bytes. | Test (TC-2804) |
| FR-992-AC-3 | `entries()`, collected as owned values, is accepted as the `byte_provision` of a protocol-trace replay request for the handoff's deadlock counterexample. The replay reproduces, and the same request with the unit entry removed is refused for the missing source. | Test (TC-2804) |

## Dependencies

- [FR-991](FR-991-publish-in-process-replay-of-a-global-conformance-counterexample.md):
  the in-process replay a handoff library also uses.
- [FR-896](FR-896-decide-a-global-conformance-claim-over-the-protocol-system-or-a-recording.md):
  the claim whose results a handoff library carries.
- FSpec FR-433: the protocol counterexample a replay re-executes.
