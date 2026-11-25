---
id: FR-1049
title: "Classify alpha tutorial probability outcomes and project them in the journey gate"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-127
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-192
    type: references
---
# FR-1049: Classify alpha tutorial probability outcomes and project them in the journey gate

## Description

When the alpha tutorial journey runs a documented probability example, the
journey gate SHALL preserve the candidate's documented settled category and
exit status in its outcome projection, including the distinction between an
unsupported form and a work-limited incomplete result.

This requirement covers the explicit unsupported and incomplete cases in the
candidate-1 probability tutorial, plus the common cancellation and internal
fault outcome classifications. It does not assert that an ordinary tutorial
command deterministically triggers cancellation or an internal fault.

## Inputs

- The literal FSL source and native request files identified in the acceptance
  criteria, run by the alpha tutorial journey gate against candidate-1.
- A caller cancellation or internal runtime fault, only when that condition
  actually occurs during a tutorial journey.

## Outputs

- The command exit status and `filament-outcome/1` document produced by FSL.
- The journey gate's recorded outcome projection, containing `category`,
  `last_stage`, `result`, `first_diagnostic`, and `items`.

## Behavior

- The journey gate SHALL compare and retain the documented outcome category,
  exit status, and projected code/cause fields for each covered example.
- For an unsupported example, the gate SHALL retain an unsupported outcome and
  its documented diagnostic code and cause.
- For a work-limited example, the gate SHALL retain an incomplete outcome and
  its typed limit cause, including limit name, setting, and configured value.
- The gate projection SHALL omit diagnostic message text. In particular,
  `diagnostics[].message` is not part of the journey gate contract because the
  gate does not project that field.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-1049-AC-1 | Given the exact source `language "ix:native" edition "1-draft"; profile v = "filament.model.complete/v1"; profile ev = "filament.temporal.event-position.false-extension/v1"; profile inf = "filament.temporal.infinite-trace/v1"; model Arch = "example/architecture"; random Arch::Service::call(ok) ~ { true: 9999, false: 1 }; post UpFollowsCall using v on Arch::Service::call { self.up = ok }; probabilistic Availability using ev on Arch over (s: Arch::Service) under every scheduler on origin { long_run_fraction holds(s.up) >= 0.99 }`, with the `native-analyze/1` request selecting claim `Availability` and exact evidence from `examples/architecture/requests/probability/claims/claims-long-run-fraction.json`, the command exits 21 and the journey projection has top-level `category: "unsupported"`, `last_stage: "S6b"`, `result: null`, `first_diagnostic: null`, and one item with `category: "unsupported"`, `record.result: "unsupported"`, `record.settled_by: "disposition"`, and `record.diagnostic: {"code":"unsupported_construct","cause":"not-yet-implemented","detail":{}}`. | Test |
| FR-1049-AC-2 | Given the exact source `language "ix:native" edition "1-draft"; profile v = "filament.model.complete/v1"; profile ev = "filament.temporal.event-position.false-extension/v1"; profile inf = "filament.temporal.infinite-trace/v1"; model Arch = "example/architecture"; random Arch::Service::call(ok) ~ { true: 9999, false: 1 }; post UpFollowsCall using v on Arch::Service::call { self.up = ok }; probabilistic UpMostly using ev on Arch over (s: Arch::Service) under every scheduler on origin { mean of (fraction holds(s.up) over [0, 10]) >= 0.9 }`, with the `native-analyze/1` request selecting `UpMostly` and exact evidence from `examples/architecture/requests/probability/claims/claims-mean-of-fraction.json`, the command exits 22 and the journey projection has top-level `category: "incomplete"`, `last_stage: "S6c"`, `result: null`, `first_diagnostic: null`, and one item with `category: "incomplete"`, `record.result: "incomplete"`, `record.settled_by: "result"`, `record.backend: "en-5"`, and `record.cause: {"kind":"limit-reached","limit":"work_units","setting":"accounting.work_units","value":1000000}`. | Test |
| FR-1049-AC-3 | Given the tutorial's bounded-profile source form `probabilistic StaysUp using ev on Arch over (s: Arch::Service) under every scheduler on origin { probability >= 0.9 [ always holds(s.up) ] }` from `claims-unbounded-event.fsl`, compiled by the `native-compile/1` request in `claims-unbounded-event.json`, the command exits 21 and the journey projection has `category: "unsupported"`, `last_stage: "S3"`, `result: null`, `first_diagnostic: {"code":"unsupported_construct","cause":"expression-form"}`, and `items: []`. | Test |
| FR-1049-AC-4 | If a caller cancellation actually interrupts a tutorial command, the outcome category is `incomplete`, the exit status is 22, and its diagnostic identifies code `cancelled` and cause `caller-cancelled`. If an internal runtime invariant fault actually occurs, the outcome category is `internal-failure`, the exit status is 30, and its diagnostic identifies code `runtime_invariant` and cause `established-invariant-broken`. In either conditional case, the gate projection contains the `category`, `last_stage`, `result`, `first_diagnostic` code/cause pair, and `items` fields; this criterion specifies no tutorial input that causes either interruption. | Inspection |
| FR-1049-AC-5 | For the inputs in AC-1 through AC-4, the journey gate's projected outcome contains no diagnostic `message` field and makes no comparison of native `diagnostics[].message` text. The category, exit status, code/cause, and output shape in the other criteria remain the assertions. | Test |

## Dependencies

- Candidate behavior is pinned to FSL commit
  `dccaf11c3d00f51738761f26cb72ef3124415e02`.
- Tutorial prose and outcome category/exit definitions are pinned to
  `filament-docs` commit
  `9d02b2cd325ed0450a2edac895afd0bd70855fd8`, particularly
  `src/content/docs/fsl/tutorials/tutorial-probability.md`,
  `src/content/docs/fsl/checks/outcomes.md`, and
  `src/content/docs/fsl/model-checking/verdicts-and-budgets.md`.
- Literal sources and projected expected outcomes are pinned to
  `filament-examples` commit
  `2529505b79a8913c398b0a657b6332e9b2666e41`, particularly the three
  `claims-*.fsl`, request, and expected JSON files named in AC-1 through AC-3.
- Journey projection behavior is pinned to filament-driver PR #231 merge
  `557245e9d6ff92398ca73aa97a14ba0fae992bd5`.
