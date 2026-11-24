---
id: SR-967175026302197768900474181941935084813
title: "Code review of QSL-2972 host allocation propagation"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; fsl-package, fsl-replay, fsl-analyze, fsl-command; FR-096-AC-24 and FSpec FR-330-AC-17"
review_set: subset
---

## Summary

Reviewed PR #3687 at `0e56a371e7ca83b7ff366347e41017a22d9cb484`, including Rust style, behavior, and test trace coverage. The new host allocation outcome is still converted to a package refusal by the model-check replay consumer, and command-path injection coverage is missing; the review is conditional pending fixes.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Model-check replay classifies an allocation failure without a limit as malformed-package refusal | fsl-replay/src/model_check/replay.rs:905-909 | implementation-bug-despite-evidence |
| FND-002 | medium | Cross-repository FSpec test tags resolve to an unrelated local FR-330 criterion | fsl-replay/src/hybrid/process_request.rs:619; fsl-command/src/command.rs:5665 | correct-requirement-no-evidence |
| FND-003 | medium | Command coverage tests only the error-category helper, not an injected reader failure through a command consumer | fsl-command/src/command.rs:5667 | correct-requirement-no-evidence |
| FND-004 | medium | Analyze terminal record uses an unregistered runtime-invariant cause | fsl-analyze/src/analyze/document/composition_item.rs:933 | implementation-bug-despite-evidence |

## Verdict

CONDITIONAL. Do not merge until the high-severity classification defect is fixed and the command consumer's acceptance evidence is exercised. Fix the trace identifiers and resolve the unregistered diagnostic code/cause as part of the same round.

## Scope examined

- FSL FR-096-AC-24: injected failures at each listed package-read consumer must be host/internal failures with stage and requested size, produce no package refusal/record or limit advice, and preserve `body_incomplete`.
- FSpec FR-330-AC-17: admitted allocation failure returns `HostAllocationFailed` with stage/locus/requested/consumed, creates no package or feature-accounting record, and is reported as a host error.
- Changed production paths in fsl-package, fsl-replay (including hybrid and model-check), fsl-analyze, and fsl-command.
- Focused changed tests and their Quire tags.

## Rust review notes

Coder evidence: `cargo check -j 8 -p fsl-analyze --tests -p fsl-command` passed; focused `cargo test -j 8 -p fsl-package -p fsl-replay -p fsl-analyze -p fsl-command --lib host_allocation` passed (5 tests); rustfmt and `git diff --check` passed. These runs do not cover the missed model-check branch or substitute for the absent command injection test. No dependency or Cargo manifest changes were present.

The existing `body_incomplete` handling was not changed in the reviewed hunks. The targeted hybrid mapper test injects the new enum variant and verifies mapping, but other consumer paths need the findings below addressed.

## Findings detail

### FND-001 — model-check replay converts the new host failure to package refusal

`fsl-replay/src/model_check/replay.rs:905-909` maps `admission_refusal` by inspecting `refusal.limit`; when it is `None`, it constructs `ReplayRefusal::PackageRead(PackageReadRefusal::Malformed)`. The calls at lines 923 and 956 take this path for `HostAllocationFailed`, whose limit is absent. `ReplayRefusal::PackageRead(PackageReadRefusal::Malformed)` is emitted with `Code::InvalidPackage` in `fsl-replay/src/execute.rs:660`. This violates the explicit prohibition on turning a host allocation failure into a package refusal. Add an explicit host-allocation classification and a test through the model-check replay consumer.

### FND-002 — test trace IDs bind to the wrong FR

The comments at `fsl-replay/src/hybrid/process_request.rs:619` and `fsl-command/src/command.rs:5665` describe FSpec FR-330-AC-17 but use bare `FR-330-AC-17`. In this repository bare IDs bind to the local FSL FR-330-AC-17, an unrelated temporal-clause criterion. Use the required `FSpec-FR-330-AC-17` tag; add the local `FR-096-AC-24` tag where the test actually verifies that FSL obligation.

### FND-003 — fsl-command does not test reader failure through a consumer

The only new fsl-command test at `fsl-command/src/command.rs:5667` calls `inspect_handoff_category(&Error)` directly. It does not inject `CheckedPackageDispatchResult::HostAllocationFailed` through `checked_handoff::admit` or execute a command consumer, and thus does not show that the changed package-run/export/handoff mappings preserve the host failure classification and message. The matrix has no fsl-command test tagged to FR-096-AC-24. Add an injected-failure test through an established command/environment seam and tag the owning local criterion plus the correctly prefixed FSpec criterion if applicable.

### FND-004 — unregistered diagnostic cause

`fsl-analyze/src/analyze/document/composition_item.rs:933` emits `CatalogCode::new(Code::RuntimeInvariant.as_str(), "host-allocation-failed")`. The registered catalog entries in `fsl-foundation/src/lib.rs:51-55` and `fsl-analyze/src/lib.rs:47-50` include `runtime_invariant/established-invariant-broken`, but not `runtime_invariant/host-allocation-failed`. This introduces an unregistered code/cause pair. Preserve the host/internal failure semantics using a registered diagnostic pair, or add the pair only with its proper catalog/spec ownership and evidence.
