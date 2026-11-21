---
id: SR-202611210022442313144108512
title: "spec-review/ears-conformance review of HP-1 unchecked settlement"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; spec/functional/FR-182-settle-a-hyper-verdict.md; FR-163; FR-174; QSL-2128; PR 3298"
review_set: subset
---

## Summary

Review of filament-spec-language#3298 at ac3a2e1fc01e179574f5c39e84475d5e2cb796a8. Ticket: QSL-2128.

## Verdict

**PASS** — No additional defect found by this method.

## Scope

- FR-182 (examined): When an exhaustive HP-1 result is closed and its certificate is absent or has not been checked in the core, settlement SHALL return `incomplete` with `IncompleteCause::CertificateUnchecked`, never `proved` labelled `Uncertified`.
- FR-182 (examined): A rejected certificate still settles V-6 `CertificateRejected`,   and a check stopped by a limit still settles V-7 naming that limit.
- FR-182-AC-17 (examined): with the check stopped by `max_certificate_edges` it settles V-7 naming that limit, its configured value and setting rather than `CertificateUnchecked`.
- FR-182 (examined):   - target: ix://agent-ix/filament-spec-language/FR-163     type: depends_on
- FR-163 (context_only):   - target: ix://agent-ix/filament-spec-language/FR-182     type: depends_on
- FR-163 (context_only): - **Limits.** The checker SHALL count recomputed states and successors   against the request's `ModelCheckLimits` with checked arithmetic.
- FR-163-AC-4 (context_only): The AC-1 `NonInterference` certificate checked with `max_states` 1 returns `Stopped` with `{MaxStates, 1}` naming the limit and its value; checking it twice gives equal results.
- FR-174 (context_only):   symmetry member to `Proved{basis: Reduced{reductions}, certification:   Uncertified}`, and one with only a constraint and copy-swap or nothing to

## Review selection and evidence

No applicable AssuranceProfile exists in the reviewed spec tree; direct review applies. Selected code-review and spec base, EARS for edited requirements, integrity for added criterion/table row, and dependency for new relationship edges. No production change, Rust lane or implementation gap review. No Cargo, Python, full gate, schema, vectors or dependency pin edits. Scoped Quire validation of FR-182 succeeds; requirement grammar is 1/1 clean with zero findings. No Rust code embeds FR-182. All new IDs and literal references were checked; new AC17 is implementation work, not claimed covered. FR-179-AC-1 owns the 64 tuple fixture. The new table's FSpec label failed/resource-incomplete is distinct from O-16 incomplete category and is consistent with FR-127's terminal axes. The two new SHALL statements have explicit HP-1 conditions and concrete terminal outcomes. Scoped engine check is 1/1 grammar-clean with zero grammar findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
