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

- The `export_evidence` member SHALL carry `package` (the FSpec FR-299
  package key of an assembled export), `obligation`, `tool` (`tlc`,
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
| FR-1022-AC-3 | Evidence naming an `unrepresented` record, and evidence whose `package` key names an export of a changed model, each refuse `invalid_runtime_input`/`invalid-value` at the member. | Test (TC-2872) |

## Dependencies

- FSpec FR-815 (the meaning of a target result), FSpec FR-814 (name map),
  FR-1015 (the export outcome).
