---
id: FR-1022
title: "Take in TLA+ and Lean results over an export as trusted evidence"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-815
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-814
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1015
    type: depends_on
---
# FR-1022: Take in TLA+ and Lean results over an export as trusted evidence

## Description

FSL SHALL read a `native-run/1` request's `export_evidence` member, re-run
the named target tool as a child process on the export package's own target
text, and settle each named obligation as FSpec FR-815 states: a proof or a
complete model check of a `preserved` or `conditional` record as `trusted`
evidence, never `certified`; a counterexample only after native replay. The
driver's `export --evidence <request>` verb runs the same intake.

## Behavior

### Admission while evidence checking is unavailable

Until original generated-package binding and target-result verification are
implemented, FSL SHALL admit the closed `export_evidence` input shape and
then refuse admitted evidence with `unimplemented_capability` /
`unimplemented-meaning`, `details.capability` `export_evidence`, category
`unsupported`, exit `21`. This boundary performs no package-key or
obligation binding, artifact read, tool probe, tool execution or native
trace replay; it writes no proof result. Malformed evidence retains its
member-specific admission refusal before this capability refusal.

### Evidence checking contract

The following checking rules and AC-1/AC-2 remain required of the future
evidence checker; admission and the pending capability refusal establish
none of their proof, replay or package-binding claims.

- The `export_evidence` member SHALL carry `package` (the `package` member
  of an assembled export's outcome, FSpec FR-817, which with that export's
  records is its FSpec FR-299 package key), `obligation`, `tool` (`tlc`,
  `apalache`, `tlaps` or `lean`) and `artifact`: for `lean` and `tlaps` the
  proof file, for `tlc` and `apalache` the model-checker configuration, and
  for a counterexample the tool's trace file.
- FSL SHALL NOT read a pasted log as a result: it runs the tool on the
  package's `target_text` and the artifact and reads the tool's own output.
  An absent tool settles `unsupported`, `unsupported_projection`/
  `tool-unavailable`, naming it, exit `21`.
- A proof the tool accepts, or a complete run with no violation, SHALL
  settle `proved` with certification `trusted`, basis `decisive-witness`,
  and a trust base naming the mapper and its version and the tool and its
  version; a run under a state constraint or depth bound SHALL carry extent
  `bounded` with the bound. A `conditional` record's result lists its
  conditions as assumptions. An `unrepresented` or `refused` record refuses
  `invalid_runtime_input`/`invalid-value` at `/export_evidence/obligation`.
- A tool trace SHALL be read back through the package's name map (FSpec
  FR-814) into FSL states and replayed natively; it settles `refuted` with a
  replayable counterexample only when replay reproduces the violation, and
  otherwise is recorded as a non-reproducing observation and settles
  nothing.
- A `package` key that is not the key of the export the obligation belongs
  to (a changed model, version, profile or mapper) refuses
  `invalid_runtime_input`/`invalid-value` at `/export_evidence/package`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1022-AC-1 | A Lean proof of `Ob_Low` accepted by the Lean child process settles `proved`, `trusted`, extent `unbounded`, never `certified`; with `lean` absent the intake settles `unsupported`, `tool-unavailable`, exit `21`. | Test (TC-2872) |
| FR-1022-AC-2 | A TLC trace of a seeded violation reads back through the name map and settles `refuted` after native replay; the same trace edited so it no longer replays settles nothing. | Test (TC-2872) |
| FR-1022-AC-3 | Until original generated-package binding and target-result verification are implemented, a well-shaped `export_evidence` input carrying an assembled export's `package`, a nonempty obligation, a declared tool and a nonempty artifact path is admitted and then refused with `unimplemented_capability` / `unimplemented-meaning`, `details.capability` `export_evidence`, category `unsupported`, exit `21`, with null `last_stage` and `result`. No artifact is read or tool probed or run; no proof, counterexample or package binding is accepted. Malformed evidence is refused at its own member before this capability refusal. | Test |

## Dependencies

- FSpec FR-815 (the meaning of a target result), FSpec FR-814 (name map),
  FR-1015 (the export outcome).
