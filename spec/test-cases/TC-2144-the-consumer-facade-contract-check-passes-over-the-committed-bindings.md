---
id: TC-2144
title: "The consumer-facade contract check passes over the committed bindings"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-761
    type: verifies
---
# TC-2144: The consumer-facade contract check passes over the committed bindings

## Description

The consumer-facade contract check passes over the committed bindings.

Scope: FR-761-AC-1.

## Test Procedure

Run `cargo xtask consumer-facades` in the workspace and read its report.

Tag the tests `#[trace("TC-2144", "FR-761-AC-1", "FSpec-TC-1646", "FSpec-FR-661-AC-3")]`.

## Expected Results

- Exit 0.
- The report lists every inventory fact id, including `CF-PR-04` with `fsl_package::protocol_view::role_internality`, `CF-IR-04` with `fsl_package::SchedulabilityRoot::obligation_identity`, `CF-CG-04` with `fsl_replay::check_closed_form`, `CF-RT-02` with `fsl_analyze::monitor_root::admit_progress_view` and `CF-OB-01` with `fsl_package::checked_views::subject_kinds`.
