---
id: SR-202611210022442313144108514
title: "spec-review/dependency review of HP-1 unchecked settlement"
type: SpecReview
analysis: dependency
scope: "agent-ix/filament-spec-language; spec/functional/FR-182-settle-a-hyper-verdict.md; FR-163; FR-174; QSL-2128; PR 3298"
review_set: subset
---

## Summary

Review of filament-spec-language#3298 at ac3a2e1fc01e179574f5c39e84475d5e2cb796a8. Ticket: QSL-2128.

## Verdict

**FAIL** — The added FR-182 depends_on FR-163 edge creates a direct prerequisite cycle with FR-163's existing depends_on FR-182 edge. Neither requirement can be scheduled first by the declared dependency graph. Keep the checker citation as references rather than introducing the reverse prerequisite.

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

No applicable AssuranceProfile exists in the reviewed spec tree; direct review applies. Selected code-review and spec base, EARS for edited requirements, integrity for added criterion/table row, and dependency for new relationship edges. No production change, Rust lane or implementation gap review. No Cargo, Python, full gate, schema, vectors or dependency pin edits. Scoped Quire validation of FR-182 succeeds; requirement grammar is 1/1 clean with zero findings. No Rust code embeds FR-182. All new IDs and literal references were checked; new AC17 is implementation work, not claimed covered. FR-179-AC-1 owns the 64 tuple fixture. The new table's FSpec label failed/resource-incomplete is distinct from O-16 incomplete category and is consistent with FR-127's terminal axes. FR-163 is checker enablement and FR-182 settlement is a feature. Intended logical order is checker contract before certified settlement; the newly reciprocal depends_on edge prevents that order. FSpec FR-331 references links the cause owner and does not itself create a prerequisite cycle.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The added FR-182 depends_on FR-163 edge creates a direct prerequisite cycle with FR-163's existing depends_on FR-182 edge. Neither requirement can be scheduled first by the declared dependency graph. Keep the checker citation as references rather than introducing the reverse prerequisite. | spec/functional/FR-182-settle-a-hyper-verdict.md:30-31 |
