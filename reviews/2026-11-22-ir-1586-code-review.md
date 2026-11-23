---
id: SR-202611230621510
title: "code-review of PR3553 finite hyper body temporal_profile (IR-1586 emitter half)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3553 3c7b2d59b069e2f6b4fe7c72ba7e128df9071bff; Cargo.toml; Cargo.lock; fsl-semantics/src/check/node_key/mod.rs; fsl-semantics/src/check/lowering/hyper.rs; fsl-package/src/emit.rs; fsl-package/src/emit/tests/hyper_clause.rs; 40 pattern/constructor adaptation sites"
review_set: subset
---

## Summary

Ticket: IR-1586. Code review with the rust-review lane folded in, scoped to `git diff origin/main...HEAD` (one commit, 44 files). Governing spec: filament-specification PR 815 at acc7fd01 (FR-322, FR-322-AC-464, TC-2028, TCB-14..24). Reader: filament-contract-ir PR 1136 at c772d912.

The finite hyperproperty body aggregate now carries `temporal_profile` built from `CheckedFiniteHyperClause::profile()`, the clause's own `using` profile, through `BodyTerm::aggregate_with_profile` and `checked_term`. Every other aggregate gets `None`, and behaviours-domain hypers are lowered through a different path, so they omit the member. The removed `!is_bounded()` skip in the lock loop was dead code: `check_finite` refuses any profile other than the three bounded ones (`FiniteProfileRequired`), so the lock contents are unchanged for every package. The other 40 sites are mechanical `..` / `temporal_profile: None` adaptations with no behaviour change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new two-selection test never runs the I2 reader. It asserts the emitted lock order and body member but not that contract-ir 1136's exactly-one lock join admits the package. If the reader refuses the multi-selection shape, this test still passes. Only the single-selection test runs admission. | fsl-package/src/emit/tests/hyper_clause.rs:182-212 |
| FND-002 | low | `filament-contract-model` is pinned to the unmerged contract-ir 1136 branch head. Merging FSL first would put main on unreviewed reader code. Re-pin to the 1136 squash commit before merge and rerun the focused tests: 1136's base is 14c06b04, but contract-ir main is now bced32fb, so the re-pin also brings in the main commits in between. | Cargo.toml:30; Cargo.lock:611 |

## Scope and evidence

- Lock loop soundness: `fsl-semantics/src/check/hyper_clause.rs` `check_finite` admits only EventPositionFalseExtension, FixedSampleFalseExtension and TimestampedEventFiniteWindow, and all three are `is_bounded()`. The removed skip never fired, so no profile that was skipped before is selected now, and non-hyper lock contents are unchanged. The `contains` dedup is kept.
- Profile source: `lowering/hyper.rs` passes `clause.profile()` (the finite clause), not a lock row, and `emit.rs:2271` maps it with `profile_definition`. The new test puts INFINITE first in the lock and EVENT on the hyper and asserts EVENT on the body. Reverting the emitter (member absent) or reading the first lock row (INFINITE) both fail it.
- Behaviours absence: `the_wrapper_carries_the_profile_law_activation_and_formula` asserts `body.get("temporal_profile").is_none()`.
- Identity: `AggregateTerm` derives Hash/Eq, so the profile takes part in equality, but only hyper claim nodes carry it and those are keyed by declaration. No manual encoder drops the field. `map_refs` paths carry it.
- Call-site adaptations: every non-test `temporal_profile: None` builds a fresh aggregate (group, tuple, member, claim, scalar type, model root, time nominal). None rebuilds an existing hyper aggregate in a lossy way. In-place mutators (`rename_bindings`, `reachable_ids`, `rewrite_source_keys_in_clock_place`, `order_protocol_sets`) keep the field.
- Lockfile: the only change is the `filament-contract-model` source line. The lockfile matches Cargo.toml, and `cargo test --locked` resolved it. contract-ir 14c06b04..c772d912 holds only the three IR-1586 commits, so there is no unrelated dependency drift.
- Diff hygiene: only intended files. rustfmt differences in touched files are pre-existing on main and are not introduced by this diff. There are no stray directories.
- Focused run: `cargo test --locked -p fsl-package --lib -- emit::tests::hyper_clause` on titan, 38 passed, 0 failed.

## Verdict

Correct and minimal. The lock-loop change is sound and changes nothing observable. Both findings are low. Mergeable once contract-ir 1136 has merged and the pin moves to its squash commit (FND-002).
