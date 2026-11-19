---
id: SR-2026111911313600000001
title: "Code review of QSL-2441 claim-under-workload resolution and probabilistic refusal locus tests (filament-spec-language PR 3015)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@d581681798b0839ff1689ff2fd212a0b0d686187; git diff origin/main...HEAD (PR 3015): fsl-package/src/emit.rs, fsl-package/src/emit/probabilistic.rs, fsl-package/src/emit/monitor_plan.rs, fsl-package/src/emit/tests.rs, fsl-package/src/emit/tests/probabilistic.rs, fsl-package/src/lib.rs, fsl-command/tests/it/probabilistic_package_locus.rs"
review_set: subset
date: 2026-11-19
---

## Summary

Ticket: QSL-2441 (QSL-2000 slice D, partial delivery). The review covers `code-review` with
the `rust-review` lane folded in. It looked at the new `Emission::claim_workloads()` /
`ClaimWorkload` / `EmitRefusal::UnresolvedClaimWorkload` emit-side mapping, the two
un-ignored tests, the two new tests, the reworded ignore reason, and the formatting hunks.
The focused tests were run (see Verdict). Clippy ran only on `-p fsl-package -p fsl-command
--cap-lints warn`, and only diff findings are reported.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `each_claim_resolves_to_the_workload_it_is_stated_under` never checks the pairing. It collects only the resolved *workload* ids and asserts that this set contains Steady's and Bursty's nodes. A resolver that swapped them (A to Bursty, B to Steady) still passes, so the test does not test what its name says. Key each resolution by its claim node, for example by mapping claim name to claim node id through `node_where(&wire, "claim", "probabilistic_claim")`, and assert A→Steady and B→Bursty | fsl-package/src/emit/tests/probabilistic.rs:1051-1086 |
| FND-002 | medium | The resolution exists only as an in-memory side table: `Emission::claim_workloads()` has no production consumer, and nothing about it reaches the package bytes. A strict reader or IR consumer of the package cannot recover which workload a claim is checked under. The read-back `Read::Verified` in the FR-740-AC-4 test therefore exercises none of the resolution. This is an architecture question, not a coding error: FR-498 keeps the claim body empty. It should be stated as the open arch need (together with the FR-640-AC-3 conflict), not read as FR-740 "resolve to W's workload node" being fully met | fsl-package/src/emit.rs:572-582,1374-1378; fsl-package/src/emit/probabilistic.rs:594-641 |
| FND-003 | medium | The ignore reason on `a_claim_that_reads_a_reward_changes_identity_with_the_reward_expression` names QSL-2441, the ticket this PR delivers. On merge, the branch link will most likely close QSL-2441, and the ignore will then point at a closed ticket and track nothing. The FR-498 vs FSpec FR-640-AC-3/FR-407 spec conflict needs its own open ticket (the arch item), and that ticket's id belongs in the ignore reason | fsl-package/src/emit/tests/probabilistic.rs:1900 |
| FND-004 | low | `claim_workloads` rescans every package node for each claim (O(claims x nodes), with a literal read per probabilistic node). It also finds the name by position, through `member(&node.body, 1)`, which silently depends on `workload_node` writing `name` as the second binding. Build a `name -> node_id` map once from the workload nodes (or record it in `workload_node` / `append_probabilistic_nodes` when the node is written) so the lookup is linear and owns no layout knowledge | fsl-package/src/emit/probabilistic.rs:622-633,380-384 |
| FND-005 | low | `probabilistic_refusals_have_locus`'s doc says it covers "every S4 refusal of a random parameter, a workload, a reward or a delay" and that each "names the declaration in words". The test has only a reward case and a delay case. It asserts neither the `declaration` field nor that the message contains the declaration's name (`cost` / `O::step`), and the fsl-command twin asserts only `reward`, a word that comes from `feature.display_name()` and not from the declaration. Narrow the doc, or add the name assertions | fsl-package/src/emit/tests/probabilistic.rs:1979-2024; fsl-command/tests/it/probabilistic_package_locus.rs:59 |
| FND-006 | low | The `pub use emit::{...}` re-export hunk is not rustfmt-clean. `ProtocolEmission,` is left alone on its own line, and rustfmt repacks it with `ProtocolEmissionAggregate, ProtocolEmissionResult` (checked with rustfmt on the file with its `mod` lines stubbed) | fsl-package/src/lib.rs:124-126 |
| FND-007 | low | Unrelated scope noise: the coder's rustfmt reflowed the `Some("float") =>` arm in `assert_equality_reader_outcomes`. The reflow is correct rustfmt output with no behaviour change, but it touches an equality-reader helper outside QSL-2441's scope | fsl-package/src/emit/tests.rs:689-692 |

## Scope and evidence

Examined:
- `probabilistic::claim_workloads`. Soundness of resolution by name: FR-185-AC-5 refuses a
  second `workload Steady` package-wide (`duplicate-declaration`), so a workload name is
  unique and `find` cannot pick between two candidates. Resolution is deterministic: it
  iterates over checked graph claims and sorts the output by `CheckedNodeId`, and claim ids
  are distinct. It is bounded by the package node count (FND-004). Claim ids come from
  `emitted_node_ids` and workload ids from the final `nodes`. Both tests compare them with the
  wire `node_id`, which shows they are in the same id space. `under every scheduler` claims
  are skipped as the doc says.
- `EmitRefusal::UnresolvedClaimWorkload` maps to `RuntimeInvariant` in both match arms.
  `emit_checked_never_returns_incomplete_across_the_fixture_set` panics on it, as for
  `UnkeyedProbabilisticClaim`. That is the right treatment for an invariant S3 guarantees.
- `emit_timed_models` and `monitor_plan::finish` set `claim_workloads: Vec::new()`. Neither
  path emits probabilistic claims, so the empty table is accurate.
- `a_claim_under_a_workload_is_emitted_with_its_workload_node` (un-ignored). It now checks
  that the single resolution's claim and workload ids equal the wire nodes. Not a tautology.
- `fsl-command/tests/it/probabilistic_package_locus.rs` (un-ignored). It previously returned
  on `Ok` and tested nothing. It now `expect`s a refusal, and the package-level twin shows the
  same unit text passes S3, so the refusal is S4's `ProbabilisticModelNotEmitted`. It cannot
  pass vacuously: success fails the `expect`, and a different refusal would fail the
  `reward` / locus assertions. It is coupled to QSL-2463, which will turn the refusal into
  success, and the test's doc says so.
- No `#[allow]` was added. No `unwrap` was added on a production path: the new code returns
  `EmitRefusal`.

## Verdict

The emit-side mapping is sound for what it is: names are unique, the output is
deterministic, and every claim is resolved or refused. The partial framing in the PR body is
accurate. The INDEX.md absence was verified (no INDEX.md anywhere in the repo), and the
remaining items are listed. The two medium test/trace findings (FND-001, FND-003) should be
fixed in this PR. FND-002 is an architecture need to raise, not a code change.

Focused runs at d5816817 (each with `--test-threads=1` and the two house skips). These pass:
`a_claim_under_a_workload_is_emitted_with_its_workload_node`,
`each_claim_resolves_to_the_workload_it_is_stated_under`, `probabilistic_refusals_have_locus`,
and fsl-command `a_package_stage_refusal_of_a_reward_carries_its_locus`.
`a_claim_that_reads_a_reward_changes_identity_with_the_reward_expression` is ignored as
stated. `emit_checked_never_returns_incomplete_across_the_fixture_set` fails with a
fixture `EmptyPackage` check refusal at tests.rs:234. It fails the same way at merge-base
601096bee, so the PR did not cause it, but it means the new `UnresolvedClaimWorkload` arm in
that test is never reached today. Clippy (`-p fsl-package -p fsl-command --all-targets --
--cap-lints warn`): no findings in diff lines.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | The combined `UnkeyedProbabilisticClaim \| UnresolvedClaimWorkload` arm in `emit_checked_never_returns_incomplete_across_the_fixture_set` now panics with `"lowering keys every probabilistic claim"`. That message is wrong for `UnresolvedClaimWorkload`, which is an emission invariant (no emitted workload node named `W`), not a lowering one | fsl-package/src/emit/tests.rs:2025-2030 |
| FND-009 | low | The public accessor `Emission::claim_workloads()` (re-exported with `ClaimWorkload` from the crate root) is not marked TEMPORARY. Only the struct and the private `probabilistic::claim_workloads` say that QSL-2546 removes it, so a caller reading the accessor's doc sees a stable API | fsl-package/src/emit.rs:1376-1378 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | The round-2 hunks in `fsl-package/src/emit/tests/probabilistic.rs` are not rustfmt-clean. In-repo `cargo fmt --check -p fsl-package` (toolchain 1.98) splits the `refusal_of` return tuple `-> (EmitRefusal, fsl_foundation::source::provenance::RawSourceRef)` and the `assert_eq!(region.source(), &unit, ...)` call onto several lines | fsl-package/src/emit/tests/probabilistic.rs:1998-2001,2087 |
| FND-011 | low | `reviews/qsl-2441-code-review-dispositions-r1.md` and `reviews/qsl-2441-gap-analysis-dispositions-r1.md` were committed. They are Linear comment bodies, not SpecReview artifacts: they have no frontmatter, and `quire validate` fails on both ("required 'type' is missing from frontmatter"). The dispositions already live in the SR files' `## Dispositions` sections and on Linear, so delete these two files | reviews/qsl-2441-code-review-dispositions-r1.md:1; reviews/qsl-2441-gap-analysis-dispositions-r1.md:1 |

## Dispositions

Round 1, head 9588a1a78f391de7c609d8e5b0cca221398b30b9. Focused runs, each with `--test-threads=1` and the two house skips, all pass: `a_claim_under_a_workload_is_emitted_with_its_workload_node`, `each_claim_resolves_to_the_workload_it_is_stated_under`, `probabilistic_refusals_have_locus`, and fsl-command `a_package_stage_refusal_of_a_reward_carries_its_locus`. The FR-640-AC-3 test is ignored, citing QSL-2546. Clippy (`-p fsl-package -p fsl-command --all-targets -- --cap-lints warn`) reports nothing in the diff files.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The test now collects (claim, workload) pairs and asserts `(claim_node("A"), workload_named("Steady"))` and `(claim_node("B"), workload_named("Bursty"))` |
| FND-002 | deferred | QSL-2546 (arch ruling on QSL-2441: the claim body and path carry the scheduler after spec W77). `ClaimWorkload` and `probabilistic::claim_workloads` are marked TEMPORARY, and QSL-2546 records that they are to be removed |
| FND-003 | fixed | The ignore reason now cites QSL-2546, an open ticket |
| FND-004 | fixed | `workload_nodes_by_name` builds a name map once, keyed on the `name` binding rather than on position |
| FND-005 | fixed | The doc now matches four cases: reward, delay, random parameter and workload. Each asserts the `declaration`, that the region slice of the unit text contains the name, and that the message contains the name |
| FND-006 | still-open | `fsl-package/src/lib.rs` is still not rustfmt-clean. rustfmt sorts `ClaimWorkload` before `ClockSelectionCause` and repacks `SelectedSourceCollectionRow, ...` onto full lines (`rustfmt --check` on the file with its `mod` lines stubbed). Fix: run rustfmt on that block |
| FND-007 | fixed | The float-arm reflow is reverted. `git diff origin/main...HEAD -- fsl-package/src/emit/tests.rs` touches only the fixture-set refusal arm |

Round 2, head 357352b6f08618d43f9cfdfdeccd25d057d90820. The full `emit::tests::probabilistic::` group passes (34 passed, 2 ignored: QSL-2546 and QSL-2463), and so does fsl-command `a_package_stage_refusal_of_a_reward_carries_its_locus`. Clippy reports nothing in the diff files. In-repo `cargo fmt --check`: `fsl-package/src/lib.rs` is clean. `emit/tests.rs:689` and `emit.rs:3770,7652` are pre-existing on main.

| FND | outcome | reason |
| --- | --- | --- |
| FND-006 | fixed | The re-export block is now rustfmt-clean (in-repo `cargo fmt --check` reports nothing for lib.rs) |
| FND-008 | fixed | The arm is split. `UnresolvedClaimWorkload` panics with "emission emits every recorded workload a claim is stated under" |
| FND-009 | fixed | The accessor doc now carries "TEMPORARY (QSL-2441, QSL-2546, spec W77): in memory only, like [`ClaimWorkload`]; the claim body carries it later." |
