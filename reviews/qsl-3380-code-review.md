---
id: SR-967175026302197768900474181941935084873
title: "code-review review of PR4026 contract-ir, filament-specification and contract-codegen pin move"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; Cargo.toml, Cargo.lock, fsl-analyze/Cargo.toml, fsl-conformance/tests/it/fspec_pending.rs; PR4026 head 7dc7eb4786e29e24197a7083a5f535756fd058f0; tickets QSL-3380, QSL-3310"
review_set: subset
---

## Summary

Ticket: QSL-3380 (also closes QSL-3310). The PR moves three dependency pins forward and removes one pending conformance entry that the new standard revision now enforces. The manifests and the lockfile name one revision per moved dependency, every move is forward, and every pinned revision is on its repository's default branch. The contract-codegen pin names filament-spec-language commit c7d8ea8cb9, which is an ancestor of the head and reaches main only through a merge commit. One merge condition the architecture owner set for the single newly red test is not met: no filament-spec-language ticket tracks that red test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The newly red test `checked_v2::tests::protocol_mutation_manifest::conformance_protocol_mutation_manifest_replays_through_i2` has no filament-spec-language ticket; the approval for this one red requires one, filed, assigned and named in the PR, before merge. The PR names only STD-731 and STD-738, and QSL-3380 closes on merge | fsl-package/src/checked_v2/tests/protocol_mutation_manifest.rs:573 |
| FND-002 | low | The PR body says STD-738 is marked Done in filament-specification; the tracker shows STD-738 In Progress | PR4026 body, "Newly red because of this pin" |

## Scope and evidence

Rust review lane folded in: the diff has no Rust source change apart from the removal of one `Pending` entry in a test registry. Gap analysis was not run because the diff adds no production code and no acceptance criterion.

Pins, measured on the head:

- filament-contract-ir (crate `filament-contract-model`, and `filament-contract-ir` in the lockfile through contract-codegen): `fcefa8460e243a0c0446f960e4a6036e12b3f896` -> `67ba7b9e2a0d55b54e2a070633a4a28e30a560e5`. GitHub compare old...new: ahead 6, behind 0; new...main: ahead 6, behind 0. Changed source files: `certificate_member.rs` (new `CertificateMember::ZeroWeight`), `checked_package/v2/protocol_body.rs`, `checked_package/v2/monitor_plan/reached_model_read.rs`, `checked_package/v2/typed_terms.rs`; no new `pub` item other than the variant.
- filament-specification: `691ac5a6748edc324f014e26ca8370eb15b127d5` -> `a6968fb8a457feb07208c33e8545c8880a33d095`. Ahead 7, behind 0; on main (main ahead 9). Changed non-spec files: backend-provider-v1 `vectors.json` and `loop-proof-vectors.json`, eight checked-package-v2 fixtures, the protocol-node mutation manifest, `shared-grammar.md`, state-core `invocation-cases.json`, `tests/backend_provider_v1.rs`. No `schema.json` changed.
- filament-contract-codegen (dev-dependency of fsl-analyze): `521796ccb0702f5acf83c704c7f1ffdd7b40756b` -> `34ae50759b04bb27f612aa3af522bfc92435c54d`. Ahead 1, behind 0; identical to codegen main. Only `Cargo.toml`, `Cargo.lock` and a review file change. Its manifest pins contract-ir and contract-model at `67ba7b9e2a`, and fsl-replay, fsl-package, fsl-eval and fsl-semantics at `c7d8ea8cb9`; the workspace `[patch]` maps those to the local crates.
- Lockfile: each moved package has exactly one source line. The only packages with two git sources (quire-canonical, quire-canonical-derive, quire-rs, agent-ix-semantic-schema) have the same two sources on main and are untouched here.
- `git merge-base --is-ancestor c7d8ea8cb9 HEAD` holds; c7d8ea8cb9 is not on main yet. The PR body asks for a merge commit.
- `git diff origin/main...HEAD` touches only the four files above; the six merges of main into the branch add nothing else.

Dependency version moves rule: the PR body gives both revisions of each move, the changed files and public items, the references here (`CertificateMember` in fsl-analyze certificate tests, fsl-replay certificate modules and fsl-probability-check; `CheckedTerm` in fsl-package) and the tests run. This matches what was measured.

Reds: every red listed in the PR body fails with the same test names on main `0cc199a2f0` with the four files reverted (fsl-package emit 4, fsl-analyze it_monitor_protocol 39 of 190, fsl-conformance 1, fsl-cst 2, fsl-replay 1, xtask grammar_drift 1), and the failure messages match. The only measured difference: `deep_chain_emission_heap_and_allocation_per_step_are_bounded` reports an allocation total of 3,562,219 bytes on the head against 3,561,248 on main, against a ceiling of 375,000, so it is the same failure. The four scan reds (fmt, arch-lint, string-edge, canonical-types) are red on main `5019ec766f` too.

Newly red test (protocol mutation manifest): the run on the head reports "executed 25 admit, 90 refuse, 4 property rows (85 manifest rows + 34 layout rows)" and "2 rows differ", V11 (locus node 62, expected node 82) and C10 (code `invalid_package`, expected `invalid_model_binding`); the test collects every mismatch before failing. A diagnostic replay with each stale part corrected passes the unchanged field checks. On main the test passes, so the pin causes it. The requirements cited are present at `a6968fb8`: FR-464 "Reader order" (V11) and FR-322-AC-538, which requires the clause path to end in the owner operand and refuses `invalid_package`/`stale-node-key` at `owner_model` otherwise (C10). The reader is right on both rows. STD-731 and STD-738 exist, are assigned and are named in the PR. No row is skipped or edited.

Tests on the final head versus `21e86d2491`: the recipe test filters ran on `21e86d2491` (main `0cc199a2f0`). The main commits merged since then (#3979, #4001, #4004, #4012, #4015, #4018, #4019, #4020, plus spec-only #4006 and #4007) change no line that names `CertificateMember`, `CheckedTerm`, a protocol clause path or the codegen API; none touches fsl-package; #4019 and #4020 add only trace tags. The one file they share with a standard-fixture reader, `fsl-analyze/tests/it/zone_certificate.rs`, reads only the backend-provider `schema.json`, which this move does not change. The final head passes `cargo check --workspace --all-targets --all-features`, clippy on the three touched crates and the `fspec_` conformance filter (19 passed). No rerun is needed.

Main has moved one commit since the head (`f805ade0f9`, #4022), which changes only `spec/` and `reviews/` files.

- QSL-3380 acceptance (zone_certificate::the_certificate_members_are_the_reconciled_schema_set): 1 passed on `21e86d2491`.
- QSL-3310 acceptance (a_fifty_thousand_deep_oracle_returns_failed_and_releases_its_report): 1 passed on `21e86d2491`.
- fsl-conformance/tests/it/fspec_pending.rs (G7 entry for invocation/deleted-self-post-context): the registry fails a keyed case that starts to match, and the case now matches, so removing the entry is required; `fspec_` passes on the head.

## Verdict

Not mergeable yet, at head 7dc7eb4786e29e24197a7083a5f535756fd058f0, until FND-001 is resolved. The fix needs no code change: file the filament-spec-language ticket for the red test (defect, priority, project, assignee) and name it in the PR body. FND-002 is a wording fix in the same body edit. The pins, the lockfile, the reachability of c7d8ea8cb9, the dependency comparison, the attribution of the existing reds to main and the evidence for the one new red are all sound.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | QSL-3474 filed (defect, regression, priority High, project set, assigned), with the failing command and head; the PR body and QSL-3380 name it. Head unchanged at 7dc7eb4786e29e24197a7083a5f535756fd058f0 |
| FND-002 | fixed | PR body now reads "STD-738 is fixed in filament-specification #854, which this pin does not include; both tickets stay open until FSL moves past them"; #854 is merged (1fbc44ee66) and STD-738 is In Progress |
