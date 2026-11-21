---
id: SR-202611210022442313144108504
title: "Spec integrity — FR-1030/FR-1020/FR-1031/FR-127 consistency after the QSL-2247 ruling"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; FR-1030 (edited), TC-2920 (edited); context FR-1020, FR-1031, FR-127, ADR-018 RU-6/V-1/PC-1; FSpec FR-380, FR-820 (read-only, other repository)"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1030
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1020
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1031
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-127
    type: references
---

## Summary

Ticket: QSL-2247. Integrity (consistency, completeness) of the edited FR-1030
against the requirements it shares fixtures and outcomes with. The edit is
internally consistent, but a sibling AC still states the formula the ruling
found unproved, and the settlement the implementation relies on for FR-1030-AC-1
has no row in FR-127.

## Verdict

**CONDITIONAL** — medium and low findings only; FND-002 needs an arch ruling
before code-review FND-001 can be dispositioned. Clean: FR-1030 and TC-2920 agree
on AC-5's clause, refinement, slot and exit; FR-1031's refusal-stage table agrees
with FR-109-AC-2 and ruling ef6f2f49 item 1 (stage `compile`); FR-1030's
"outcomes, never refusals" list agrees with FSpec FR-820.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | FR-1020-AC-1 still names `always (holds(c.value = 2) implies once[1,1] holds(c.value = 1))` as the proved clause that transfers over a step-for-step refinement, the formula ruling ef6f2f49 found not proved under terminal stutter and FR-1030-AC-5 now pins as `premise-unproved`. Its binding test (`previous_transfers_only_through_a_step_for_step_refinement`) uses the new PrevSafe formula, so FR-1020-AC-1's stated example is never shown. Amend FR-1020-AC-1 citing the same ruling (FSpec FR-380-AC-9 carries the same example; raise it with FSpec, do not edit here). | spec/functional/FR-1020-transfer-previous-through-a-step-for-step-refinement.md:27 |
| FND-002 | medium | Missing requirement: FR-127's settlement table and ADR-018 RU-6 have no row for an EN-1 `Holds{Exhaustive}` whose certificate the core cannot translate (PC-3 checker absent for TP-3 past/interval forms); RU-6 says such a proof "settles only `certified`". FR-1030-AC-1 is silent on certification, so whether PrevSafe/SoonTwo settle `uncertified` (the PR's choice) or something else is unspecified. Needs an arch ruling and an FR-127 row before the code change is accepted. | spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:174-175; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:588 |
| FND-003 | low | FR-1030 Inputs and FR-1031 row 1 list the exclusive selection members as `call`, `clause`, `items`, `synthesis`, `export`, `export_evidence`; the implementation also refuses `tl_lowering`, `model_based_test` and `verification_plan` with `refinement_transfer`. The spec should name the full exclusive set so code and spec agree. | spec/functional/FR-1031-refuse-a-refinement-transfer-request-by-stage.md:31; fsl-command/src/command.rs:3919-3927 |

## Dispositions

Round 1 — head c410fe3190700371f7a5dc2c5da261e90c7ea938. Arch ruling A (draco:QSL:arch, quoted in /tmp/qsl-2247-lead-steering.md) is authoritative.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-1020-AC-1 now names `always (holds(c.value = 1) implies once[1,1] holds(c.value = 0))` and cites the 2026-11-20 ruling and FR-1030-AC-5. The external FSpec FR-380-AC-9 and TC-1042 correction is reported merged in FSpec PR 782 (lead steering; not measured by this reviewer, and no pin bump). |
| FND-002 | rejected | Arch ruling A holds that the FR-127 / ADR-018 V-1 and RU-6 certificate requirement stands. A refused check settles under FR-127's existing `failed` row and there is no checker-absent `Proved`, so no new row is required. The unmet FR-1030-AC-1 is tracked as gap FND-007. |
| FND-003 | fixed | FR-1030 Inputs and FR-1031 row 1 now list `tl_lowering`, `model_based_test`, `verification_plan` and `package` beside the original members, matching `refuse_transfer_selection_conflict`. |

## Dispositions (round 2)

Round 2: head d4aeb3439e7b741b0ba8ce1828f385e667d32ae2. No finding is open. The FR-1030 Behavior bullet added in fix 2 agrees with FR-127 and ADR-018 V-1/RU-6 (no checker-absent `proved`) and with the CR011 ruling. FR-1030-AC-1 stays the unmet target rather than being rewritten to fit the code. No new findings.

## Dispositions (round 3)

Round 3: head 9ae17da354abebc355818a6fd9ffd55c2d422da6. The mechanical commit touches no spec file. Nothing open; no new findings.

## Dispositions (round 4)

Round 4: head e859e681ce37b0b9192771411909073b0055aa9e. The commit touches only fsl-command/src/command.rs and export.rs (CR018). Nothing open in this artifact; no new findings.
