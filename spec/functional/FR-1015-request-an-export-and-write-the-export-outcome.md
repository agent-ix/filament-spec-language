---
id: FR-1015
title: "Request a TLA+ or Lean export from a run request and write the export outcome"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-817 filament-specification/FR-811 filament-spec-language/FR-756
    type: depends_on
---
# FR-1015: Request a TLA+ or Lean export from a run request and write the export outcome

## Description

FSL SHALL read a `native-run/1` request's `export` member as FSpec FR-817's export request and write FSpec FR-817's `filament.export-outcome/v1` document as the run outcome's `export` member, applying FSpec FR-811's fidelity floor and the `export.*` limits, with FSpec FR-817's exits. The driver's `export --out <file>` verb writes the outcome's `target_text`.

## Behavior

- The `export` member SHALL carry `profile`, `target_family`, `obligations`, `minimum_fidelity` (default `conditional`), `observer` and `limits` (`output_bytes`, `nodes`, `work_units`).
- A request below its floor SHALL write `status` `refused` with `below-fidelity-floor`, no `target_text`, exit `21`.
- A denied export charge SHALL write `status` `incomplete` naming `export.<counter>`, exit `22`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1015-AC-1 | Exporting `SlowEnough` over `Braking` under the default floor writes `status` `refused` with `below-fidelity-floor`, no `target_text`, exit `21`; under `minimum_fidelity` `any` it writes `assembled` with the `unrepresented` record claiming no region. | Test (TC-2865) |
| FR-1015-AC-2 | `limits.output_bytes` 1 writes `incomplete` naming `export.output_bytes`, exit `22`. | Test (TC-2865) |

## Dependencies

- FR-817 filament-specification/FR-811 filament-spec-language/FR-756 (filament-specification).
