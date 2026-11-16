---
id: FR-931
title: "Select the TL correspondence of a lowering through the tl_lowering request member"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-757
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-527
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-045
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-095
    type: depends_on
---
# FR-931: Select the TL correspondence of a lowering through the tl_lowering request member

## Description

A `native-run/1` request asks for FR-757's lowering through its `tl_lowering`
member. The member names the clause, may name the versioned TL correspondence
the lowering is to use, and may ask for cross-evaluation. The correspondence is
part of the claim: `mltl.closed-trace/v1` and `mltl.online-prefix/v1` read the
same formula under a complete and an open surrounding execution, and
`mltl.origin-complete-history/v1` reads past operators back to an
authoritative origin. Naming it lets a caller require one reading and get a
typed answer when the clause does not have it, instead of a formula under a
reading the caller did not ask for.

## Inputs

A `native-run/1` request (FR-026, FR-312) carrying a program unit and the
`tl_lowering` member; for cross-evaluation, a trace or a model snapshot.

## Outputs

The run result document's `tl_lowering` record, or a request refusal.

## Behavior

### The member

- `tl_lowering` SHALL be an object decoded under FR-026's closed-envelope
  rules with exactly the members `clause` (required), `correspondence`
  (optional) and `cross_evaluate` (optional). A missing `clause`, a member that
  is `null` or of the wrong JSON kind, or an unknown member SHALL refuse
  `invalid-request` at stage `request`, exit 20, naming the member path
  (`tl_lowering.clause`, `tl_lowering.correspondence`, ...), before any source
  file is read.
- `clause` SHALL name a temporal clause of the program unit by its declared
  name. A name that names no temporal clause SHALL refuse
  `missing_declaration`/`missing-name` naming `tl_lowering.clause` and the
  name, exit 20.

### The correspondence

- `correspondence`, when present, SHALL be exactly one of
  `mltl.closed-trace/v1`, `mltl.online-prefix/v1` and
  `mltl.origin-complete-history/v1`, compared byte for byte; these are the TL
  targets of FR-045's support table. Any other string SHALL refuse
  `unknown_profile`/`unsupported-selection`, exit 20, naming
  `tl_lowering.correspondence` and the value, before the unit is compiled.
- The member fixes the surrounding-execution closure under which FR-527
  classifies the clause: `mltl.online-prefix/v1` classifies it under an open
  surrounding execution, and `mltl.closed-trace/v1` and
  `mltl.origin-complete-history/v1` under a complete one.
- When `correspondence` is absent, the clause SHALL be classified under a
  complete surrounding execution and lowered under the target that
  classification names; the default is therefore `mltl.closed-trace/v1` for a
  future-only clause and `mltl.origin-complete-history/v1` for a past-only
  clause.
- When the classification is supported and its target equals the named
  correspondence (or none is named), the lowering SHALL proceed as FR-757
  states, and the record SHALL carry `disposition: "supported"`,
  `correspondence` the target, `formula` the lowered `tl-syntax` formula in its
  display form and `atoms` the atom table (each TL atom name mapped to the
  identity text of its `holds` leaf). The run exits 0 unless cross-evaluation
  reports a fault.
- When the classification is unsupported, or is supported under a target other
  than the named correspondence, the record SHALL carry
  `disposition: "unsupported"`, `correspondence` the named or default target
  (absent when the classification names none), `classified` the target the
  classification names (absent when it names none) and `dimensions` the
  classification's unmatched dimensions in FR-045 order (`profile`,
  `infinite-trace`, `direction`, `closure`, ...), and no `formula` or `atoms`;
  the run SHALL exit 21 (unsupported, FR-285). A named correspondence that
  differs from a supported target only by closure SHALL report `closure`, and
  one that differs by operator direction SHALL report `direction`.

### Cross-evaluation

- `cross_evaluate`, when present, SHALL be an object with exactly one of two
  forms: `trace` (with an optional `over`), read exactly as FR-312 reads the
  `trace` and `over` members of a `temporal` selection, of which only a
  `finite` trace is admitted (a `lasso` or `timestamped` trace refuses
  `invalid-request` naming `tl_lowering.cross_evaluate.trace`); or `initial`
  (a snapshot file of the model subject) with `max_steps` (an integer of at
  least 1), which evaluates every finite behaviour of the subject's model from
  that snapshot of 1 to `max_steps` steps, in FR-120's enumeration order. Both
  forms, neither form, or `max_steps` 0 SHALL refuse `invalid-request` at stage
  `request`, exit 20, naming the member.
- Cross-evaluation SHALL run only on a supported lowering, and SHALL compare
  the native and `tl-mltl` verdicts at every activation position of every
  trace (FR-757). The record's `cross_evaluation` SHALL hold `traces` (the
  number evaluated), `positions` (the number of activation positions
  compared), `agree` (true exactly when `faults` is empty) and `faults`, each a
  `correspondence-loss` entry naming the clause, the trace index, the position
  and both verdicts. A fault SHALL make the run exit 30 (internal-failure,
  FR-285), because two conforming evaluators of one correspondence never
  disagree. A work-budget stop during enumeration SHALL report the counts
  reached and exit 22.
- On an unsupported lowering `cross_evaluate` SHALL be ignored and the record
  SHALL carry no `cross_evaluation`.

### Placement

- The lowering and the cross-evaluation SHALL be served by the command adapter
  crate `fsl-command`, which holds ADR-011's only edges to `tl-syntax` and
  `tl-mltl`; no other FSL crate names those crates.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-931-AC-1 | For the event-position clause `Low` (`on origin { always [0, 3] holds(c.versionNumber <= 2) }`), `tl_lowering` `{"clause": "Low"}` and `{"clause": "Low", "correspondence": "mltl.closed-trace/v1"}` each return `disposition` `supported`, `correspondence` `mltl.closed-trace/v1`, `formula` `G[0,3] p0` and an `atoms` entry for `p0`, exit 0; the two records are byte-equal. | Test (TC-2601) |
| FR-931-AC-2 | For `Low`, `"correspondence": "mltl.online-prefix/v1"` returns `supported` under `mltl.online-prefix/v1`; `"correspondence": "mltl.origin-complete-history/v1"` returns `unsupported`, `classified` `mltl.closed-trace/v1`, `dimensions` `["direction"]`, no `formula`, exit 21. For a past-only clause `on origin { historically [0, 2] holds(c.versionNumber <= 2) }` the default is `mltl.origin-complete-history/v1`. The same `Low` under `filament.temporal.timed/v1` returns `unsupported` with no `classified` and no `formula`, exit 21. | Test (TC-2601) |
| FR-931-AC-3 | `"correspondence": "mltl.closed-trace/v2"` refuses `unknown_profile`/`unsupported-selection` naming `tl_lowering.correspondence` and the value, exit 20; `"correspondence": 1`, `"correspondence": null`, a missing `clause`, an unknown member `formula`, `cross_evaluate` with both `trace` and `initial`, with neither, with `max_steps` 0 and with a `lasso` trace each refuse `invalid-request` at stage `request`, exit 20, naming the member; `"clause": "Nothing"` refuses `missing_declaration`/`missing-name` naming `tl_lowering.clause`. No refusal writes a result document. | Test (TC-2602) |
| FR-931-AC-4 | `Low` with `cross_evaluate` `{"initial": "spine-snapshot.json", "max_steps": 4}` returns `cross_evaluation` with `traces` at least 1, `positions` at least `traces`, `agree` true and `faults` `[]`, exit 0; FR-757-AC-2's deliberately mis-mapped atom table (`p0` mapped to the negated leaf, supplied at the `fsl-command` library entry, never through the request) renders through the same record writer as one `correspondence-loss` fault naming `Low`, the trace index, the position and both verdicts, `agree` false, exit 30. | Test (TC-2603) |
| FR-931-AC-5 | Only `fsl-command`'s manifest among the FSL workspace crates names `tl-syntax` or `tl-mltl` (ADR-011). | Inspection (TC-2603) |

## Dependencies

- FR-757 (lowering and cross-evaluation), FR-527 and FR-045 (classification and
  targets), FR-026 and FR-312 (request decoding, trace members), FR-120
  (behaviour enumeration), FR-285 (exit codes), ADR-011 (TL edges).
- FSpec FR-095 (correspondence contract).
