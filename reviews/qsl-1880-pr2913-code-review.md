---
id: SR-2026111612172400000035
title: "Code review of filament-spec-language PR #2913"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #2913 diff origin/main...c8c9216dc: fsl-package/src/{bare.rs,emit.rs,emit/identity_rows.rs,emit/monitor_plan.rs,emit/tests.rs,emit/tests/source_objects.rs,lib.rs}, fsl-package/tests/source_object_identity_rows_public.rs, fsl-replay/src/spine/lifecycle.rs, fsl-semantics/src/check/{assemble.rs,check.rs,mod.rs}, tests/it/source_object_emission.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-569", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-531", type: references}
---
# Code review of PR #2913

## Summary

Ticket: QSL-1880. Reviewed exact head `c8c9216dc4fac45c5000f8c889e2ea3333b98bd8`, branch `qsl-1880-own-operations`. Rust lane (rust-review) folded in. The change removes the dead `EmitRefusal::SourceObjectEmission` variant and the `first_source_object` plumbing (no constructor existed on main; no other crate in the workspace or sibling filament-* repos references either), and adds own-operation `{identity, query}` entries to identities/4 `source_objects` rows, carried beside the IR row as `SourceObjectOperationFacts`.

## Verdict

Mergeable with low/medium follow-ups inside this PR. The own-operation filter (`operation.owner == object.key`) gives own operations only, with a redefining operation listed under its own type; the query rule `operation.result.is_some() && operation.effect.is_empty()` is the same expression `checked_dispatch.rs:1043` uses for dispatch-eligible queries. The identities/4 compatibility break (older files without `operations` read as `Malformed`) is intentional and matches the no-shims policy. No new `#[allow]`; the existing `too_many_arguments` allow on `of_rows_with_source_rows` is pre-existing. Trace tags on new tests are correct (`FR-569-AC-16`, `TC-1408`). The three `#[ignore]` tests on QSL-2347 are accepted per the brief.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers the reader-rejection clause of FR-569-AC-16 (a `source_objects` row whose `operations` is absent, repeated, or holds an entry with another member set is `Malformed` at `source_objects`); it does not depend on QSL-2347, since the running Order-only roundtrip test already produces a readable file with `"operations": []` to mutate. | fsl-package/src/bare.rs:605-640; fsl-package/src/emit/tests/source_objects.rs:305-418 |
| FND-002 | low | `source_object_rows_carry_their_own_operations_and_query_flag` matches `operation_effects` rows by `provenance.member` only; AC-16 also requires each row's `declaring` to be the source-object row's `structural_node`, which is never asserted. | fsl-package/src/emit/tests/source_objects.rs:527-539 |
| FND-003 | low | `source_object_rows` drops an own operation silently (`filter_map` with `?`) when its key does not parse as `<owner>/<name>`, while FR-569 requires one entry per own operation; the rest of the function returns `EmitRefusal::SourceMapInvariant` on such invariant breaks. | fsl-package/src/emit.rs:7601-7621 |
| FND-004 | low | The refactor of `public_source_object_rows_constructor_serializes_nonempty_member_facts` into `serialized_identities` dropped `assert_eq!(member_facts.len(), 1)` and `assert_eq!(member_facts[0].len(), 1)`; the test no longer pins the exact member-fact count. | fsl-package/tests/source_object_identity_rows_public.rs:80-119 |

## Review scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-569-AC-16 | examined | Over fixture S, the `operations` of the source-object rows are: `Order` [...size, count, scaled, bump...]; `Sub` [...size, scaled...], with no entry for an inherited operation; `Shape` `[]`. [...] a row whose `operations` member is absent, repeated, or holds an entry with another member set is `Malformed` at `source_objects`. D's emission has none. |
| FR-569-AC-11 | examined | S4 [...] emits the unit's source objects per FR-995 and FR-996 and does not refuse the unit for declaring an `object`. |
| FR-569-AC-8 | examined | The emission over S carries one source-object row per type [...] The rows are written to `identities.json` `source_objects` and read back equal. |
| FR-531-AC-8 | context_only | The identity file equals, byte for byte, the encoding of the in-process unit's rows. |

- Removed plumbing: `git grep` on origin/main shows `SourceObjectEmission` only matched, never constructed; `first_source_object` fed only that match arm in `fsl-replay` `emit_refusal`. No sibling repo references either.
- Test weakening check: the old AC-11 refusal test asserted a refusal the spec no longer requires (AC-11 now says S4 emits); its replacement asserts successful emission, and other AC-11 tests (`protocol_graph.rs:1064`, `fsl-eval` `source_node.rs:157`) cover the package content. The roundtrip test's `[0]` index changed to `.any(..)` with `len() == 1` still asserted: equivalent.

## Test evidence

Confined runs in a fresh worktree at the exact head, cargo under `flock /tmp/cargo-slot.lock`, `FSPEC_DIR=/home/peter/dev/filament-specification`:

- `cargo fmt --all -- --check`: only the known pre-existing diff in `fsl-eval/src/temporal/tick_monitor.rs`.
- `cargo clippy -p fsl-package -p fsl-semantics -p fsl-replay --all-targets`: clean. `cargo clippy -p filament-spec-language --features checked-handoff --tests`: clean.
- `cargo test -p fsl-package --lib source_objects`: 15 passed, 8 ignored (all QSL-2347).
- `cargo test -p fsl-package --test source_object_identity_rows_public`: 1 passed, 1 ignored (QSL-2347).
- `cargo test -p filament-spec-language --features checked-handoff --test it source_object_emission`: 2 passed, 1 ignored (QSL-2347).
