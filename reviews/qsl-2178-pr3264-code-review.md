---
id: SR-2026112105473201
title: "Code review (Rust lane) — filament-spec-language QSL-2178 PR #3264"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3264 at bde7f42408d78813ee810b71ab1ed9408b3f7866 vs origin/main; fsl-semantics/tests/it/clock_places.rs, tests/support/ring_queue.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-1040", type: references }
---

## Summary

Ticket: QSL-2178. Static review (no cargo, by brief) of the test-only change in `origin/main...bde7f42408` with rust-review and rust-style (testing reference) applied. Head moved from df8d5d0ebd to bde7f42408 mid-review by one spec-only commit; this SR covers the Rust files, which that commit did not touch.

## Verdict

**Mergeable if fixed.** The positive test is a real oracle: the refinement typer and clause typer share one `Scope::with_clock_places` (fsl-semantics/src/check/mod.rs:2897), so without clock places `ref(c.x)` refuses and the `unwrap_or_else` panics; the owner check pins `parameter` named `c`, which the population owner (`self`) cannot satisfy. The test passing on main unchanged is expected for a test-only gap closure of a previously untested row form, not vacuous. Defects are trace tags (FR-230-AC-11 is unrelated; the negative test is not AC-3), a cause-only refusal oracle that drops the locus, and a copied check harness. Added lines are under 100 columns and contain no construct whose layout differs between rustfmt style editions 2021 and 2024 (the repo is edition 2021, no rustfmt.toml). The transient `check.rs:1874` lifetime error is not a latent defect: on main that line is `let packages = self.model_spellings.package_unit_ids();`, all owned values; rustc is deterministic, so a pass on retry means the sources differed between runs.

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| clock_places.rs `a_refinement_row_ref_lowers_with_the_binder_as_owner` | examined | only/count rows lower with binder c as owner, place clock_place |
| clock_places.rs `a_refinement_row_clock_read_is_refused` | examined | sum(c in C::calls: c.x) >= 0 refuses RefinementCause::ClockRead |
| ring_queue.rs `CLOCKS`, `clock_calls`, `clock_refinement`, `check_clock_refinement` | examined | timed concrete Call { n, x: Clock } over calls |
| refinement.rs `check_row_as` ClockRead branch; check.rs `with_clock_places`; check.rs:1874 | context_only | how the row refuses and where places come from |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Both new tests carry the trace tag FR-230-AC-11, whose criterion is the `record Timer { x: Clock; n: Int }` field `t` over three objects (`self.t.x` guard, `Timer` literal reset, `self.u = self.t` refusal, `Timer` node lists `n` only). Neither test touches a record clock. These are the only `#[trace]` tags on FR-230-AC-11 in the repo (its real tests carry only "Part of FR-230-AC-11" doc comments), so the matrix would show FR-230-AC-11 covered by two unrelated tests. Remove the tag. | fsl-semantics/tests/it/clock_places.rs:3831, fsl-semantics/tests/it/clock_places.rs:3859 |
| FND-002 | medium | `a_refinement_row_clock_read_is_refused` is tagged FR-1040-AC-3, a positive criterion with no refusal in it. The behaviour it asserts is the new refinement-row clause of FR-1040 "Refusals that remain", which no AC states (see the spec-review SR). Tag it to the AC that states it once added (AC-6 extension or a new AC), not AC-3. | fsl-semantics/tests/it/clock_places.rs:3859 |
| FND-003 | low | The negative oracle matches only `RefinementCause::ClockRead`; it does not assert the code/subcode `unsupported_construct`/`expression-form` or the locus "at the read" (`c.x`) that FR-1040 now states. `check_clock_refinement` returns bare `CheckRefusal`s and drops the region text that `check_over`'s `Refused.at` provides, so a refusal spanning the whole row, or the `sum`, passes. Assert `at == "c.x"` and the code. The doc comment also cites "FSpec FR-430"; the timed-refinement rule is this repo's FR-430. | fsl-semantics/tests/it/clock_places.rs:3856-3874, tests/support/ring_queue.rs:535-578 |
| FND-004 | low | `check_clock_refinement` copies about 40 lines of `check_over` (selection loop, header, S1/S2/I1/assemble pipeline), differing only in the concrete package and alias, the `t` profile line, `with_unwritten_timed_members`, and the dropped regions. Parametrize `check_over` over the concrete package (alias, identity, bytes) and timed flag so both share one pipeline and `Refused` (fixes FND-003). Also, `ring_queue.rs` is now compiled twice into the `it` binary (`#[path]` mod in both clock_places.rs and refinement_populations.rs); declare it once in tests/it/main.rs as `published_fspec` is. | tests/support/ring_queue.rs:535-578, fsl-semantics/tests/it/clock_places.rs:21-22 |
| FND-005 | low | The positive test's second oracle, `timed_model("C").clocks().len() == 1`, is a model-intake property that holds whatever the refinement row lowers to; it does not tie the `clock_place` node the `ref` names to the place the timed model lists (FR-1040 "the `clock_place` node ... is the one the model's timed model lists"), and the binder's type `Reference<Call>` (AC-3) is not asserted. Compare the referenced node to the emitted timed model's place node instead of counting S3 places. | fsl-semantics/tests/it/clock_places.rs:3840-3853 |
