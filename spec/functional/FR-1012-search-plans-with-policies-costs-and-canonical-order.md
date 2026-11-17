---
id: FR-1012
title: "Search plans with exact costs, policy monitors and the canonical plan order"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-800 filament-specification/FR-802 filament-specification/FR-803 filament-specification/FR-804 filament-specification/FR-808 filament-specification/FR-809 filament-spec-language/FR-755
    type: depends_on
---
# FR-1012: Search plans with exact costs, policy monitors and the canonical plan order

## Description

FSL's plan search in `fsl-analyze` SHALL implement FSpec FR-800 to FR-804, FR-808 and FR-809: search nodes keyed by the canonical state key, monitor states and accumulators; exact objectives classified as per-step sums or general; safety monitors that prune and non-safety policies evaluated on each completed plan's behaviour; the canonical plan order; delays over the region abstraction; and exact probabilistic outcome distributions.

## Behavior

- The search SHALL be a separate module from `fsl-replay`'s proof-composition planner and from the verification planner.
- The reported plan and every count SHALL be those of the canonical sequential exploration, whatever the worker count.
- Every reported plan SHALL replay natively before it is written (FR-755).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1012-AC-1 | The spine plan under the policy `eventually holds(c.versionNumber = 3)` is evaluated, not refused, and every reported plan visits version 3. | Test (TC-2862) |
| FR-1012-AC-2 | Two runs of one plan request give byte-identical outcome documents. | Test (TC-2862) |
| FR-1012-AC-3 | A decimal per-step cost objective is written exactly in the run value form and certified with a `potential` certificate. | Test (TC-2862) |

## Dependencies

- FR-800 filament-specification/FR-802 filament-specification/FR-803 filament-specification/FR-804 filament-specification/FR-808 filament-specification/FR-809 filament-spec-language/FR-755 (filament-specification).
