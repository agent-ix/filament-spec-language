---
id: FR-1021
title: "Plan over a hybrid model through the negotiated hybrid provider"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-819 filament-spec-language/FR-750 filament-spec-language/FR-1012
    type: depends_on
---
# FR-1021: Plan over a hybrid model through the negotiated hybrid provider

## Description

FSL SHALL run FSpec FR-819's hybrid plan synthesis: S3 SHALL admit a `synthesis` whose `grammar` names a `hybrid` declaration, the run request SHALL require `synthesis.dwell_grid`, and each candidate schedule SHALL be validated as a bounded-reachability item on the negotiated hybrid provider (FR-750, FR-1019).

## Behavior

- Only a `flowpipe-outward-rounded` or `inductive-enclosure` enclosure SHALL validate a candidate; `infeasible` SHALL require an FSpec FR-193 `inductive-enclosure` or `flowpipe-outward-rounded` proof of `always not goal` over the schedule space whose FSpec FR-652 certificate FR-751 accepts. `always not goal` is a claim form both certificate forms read: rule (d) of an `inductive-enclosure` admits an obligation `always φ` whose state formula `φ` negates the goal's `holds` atoms, and a `flowpipe`'s `claim` rule reads it per instant by Kleene's three-valued logic (FSpec FR-652), so `not` is applied to each reading and never to a whole-box result.
- No hybrid plan outcome SHALL be `optimal`.
- The hybrid provider SHALL be chosen by the engine set the caller runs the request with, never by a request member: `fsl_command::command::run` settles against `Engines::fsl()`, which holds no hybrid provider, and `run_with_providers` with the reference provider (FR-1019) registered beside it, as the driver's default engine set does. A request cannot name an engine; a `synthesis` member `engines` refuses `unknown-member` (FR-1010).
- `synthesis.dwell_grid` SHALL be read in FSpec FR-181's typed form of an exact rational, `{"type": "rational", ...}` or `{"type": "decimal", ...}` (FSpec FR-805), in the hybrid declaration's time unit; a JSON number refuses `invalid_runtime_input`/`wrong-value-kind` and zero or a negative value `invalid-value`, at `/request/synthesis/dwell_grid`, and its absence `missing-member` there. That typed form belongs to the run request, whose values are FSpec FR-181 typed values. Each validation item goes to the provider through FR-750, so its exact numbers, a candidate dwell, a horizon and every trace interval bound, are FSpec FR-331 `Rational` objects `{"numerator", "denominator"}` with no `type` member, and the plan's `replay` reads the provider's answer in that form (FSpec FR-193 "Solver result on the wire").
- With no hybrid candidate the outcome SHALL be `unsupported`, exit `21`; a grid exhausted with no infeasibility proof SHALL be `incomplete` with `grid-exhausted{dwell_grid}`, exit `22`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1021-AC-1 | The `Fill`/`Drain` tank with `dwell_grid` `{"type": "rational", "numerator": "1", "denominator": "1"}` plans `feasible` with dwell 8 into `Drain` under the reference provider registered through `run_with_providers`; the goal `h >= 12` is `infeasible` with a `certified` proof of `always not goal`; run through `run`, whose engine set holds no hybrid provider, the outcome is `unsupported` naming `hybrid-dynamics`, exit `21`; without `dwell_grid` the request refuses `missing-member` at `/request/synthesis/dwell_grid`. | Test (TC-2871) |
| FR-1021-AC-2 | `dwell_grid` `1` (a JSON number) refuses `invalid_runtime_input`/`wrong-value-kind` and `{"type": "rational", "numerator": "0", "denominator": "1"}` refuses `invalid-value`, each at `/request/synthesis/dwell_grid`, exit `20`; a `synthesis` member carrying `engines` refuses `unknown-member` at `/request/synthesis/engines`, exit `20`; the `unsupported` outcome carries every outcome member, with `objective.value`, `plan`, `replay`, `certificate` and `certification` `null`. | Test (TC-2871) |
| FR-1021-AC-3 | The `Fill`/`Drain` validation item for dwell 8 reaches the reference provider with its dwell and horizon as `{"numerator": "8", "denominator": "1"}`-form `Rational` objects, and a fixture provider whose answer writes a trace bound as `"8"` makes that item refuse `invalid_result`/`wrong-value-kind` at the member, so the candidate is not validated. | Test (TC-3035) |

## Dependencies

- FR-819 filament-spec-language/FR-750 filament-spec-language/FR-1012 (filament-specification).
