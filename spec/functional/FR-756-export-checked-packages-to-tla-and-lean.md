---
id: FR-756
title: "Export checked packages to TLA+ 2 and Lean 4 through the output-mapping coordinator"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: implements
  - target: ix://agent-ix/filament-specification/FR-651
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-297
    type: depends_on
---
# FR-756: Export checked packages to TLA+ 2 and Lean 4 through the output-mapping coordinator

## Description

FSL SHALL provide the `filament.output.tla2/v1` and `filament.output.lean4/v1`
mappers of FSpec FR-651, reading the checked package through its checked
views and producing one FR-298 record per requested obligation with its
FR-269 fidelity disposition. Exported text is never read back as source or
evidence of truth.

## Inputs

A checked package, an admitted FSpec FR-120 request selecting one of the two
profiles, and the requested obligation identities.

## Outputs

One FSpec FR-299 generated package holding the TLA+ module or Lean 4 file
and the per-obligation records.

## Behavior

- Each mapper SHALL translate the declarations FSpec FR-651 "What is
  exported" lists and assign each obligation the disposition FSpec FR-651
  "Fidelity" fixes for its form.
- Each text region SHALL carry its source obligation identity and span as a
  comment.
- The mapper SHALL be deterministic: equal requests give byte-equal packages.
- An observer run (SANY, TLC, Lean) SHALL be optional and recorded only in
  the package's observer section.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-756-AC-1 | The counter model with `inc` and `value <= 5` exports under each profile with every obligation `preserved` and each region commented with its source identity and span; two exports are byte-equal. | Test (TC-2106) |
| FR-756-AC-2 | A dense-time clause exports `conditional` (non-Zeno); `Braking`'s flows export `unrepresented`; neither is `preserved`. | Test (TC-2106) |

## Dependencies

- FSpec FR-651, FR-297, FR-298, FR-299 and FR-269.
