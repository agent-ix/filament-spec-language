---
id: SR-1509
title: "Code and Rust review of the shared Evaluation gate"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2096; xtask/src/canonical_types.rs; c39c4c756..246f1bc591daf951f62aeb7dc128326b8ca5133b"
review_set: subset
---

## Summary

Independent diff review of the fourth shared-leaf registration and the explicitly required Evaluation identifier rule. No defects found in this confined behavior; no PR existed at review time.

## Verdict

**PASS** for the frozen diff. Adding `filament-eval` to SHARED_LEAVES admits its real metadata package into scanning independently of graph classification. The canonical registry still requires a public tagged definition. The new branch reserves only a tagged `Evaluation` owned by package `filament-eval`, and only against workspace definitions. Ecosystem copies retain kind/member comparison; direct owner re-exports remain admitted.

The three real scanner fixture pairs exercise private struct, nested restricted enum, and type alias declarations. Each checks the exact Identifier finding and both file/line sites, the real summarize refusal's typed code and exit 1, then absence of findings and gate acceptance after renaming while retaining the direct canonical import. Removing either the shared-leaf registration or the identifier branch breaks these assertions. No mocked scanner, static source assertion, production test bypass, copied implementation, new dependency, configuration change, or unsafe/concurrent path was introduced.

## Examined scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-272 behavior | examined | A canonical type tagged in a shared leaf is canonical in every run that scans that leaf: `filament_eval::Evaluation`'s tag makes an `Evaluation` namesake in an FSL workspace member an `identifier` finding. |
| FR-272-AC-5 | examined | `filament-eval` tagging `pub struct Evaluation` puts `Evaluation` in the canonical set, a `struct Evaluation` in a workspace member gives an `identifier` finding naming both locations |
| FR-272-AC-2 | context_only | with one variant renamed (`LessEqual` for `LessOrEqual`), with two variants swapped, or as a `struct` with those field names, it gives none. |
| FR-273-AC-1 | examined | Until M-6d lands, `cargo xtask canonical-types` runs on demand over the FSL workspace and reports every remaining namesake, and no merge gate waits on it. |
| TC-747 step 5 | context_only | `filament-eval` tags `pub struct Evaluation`; a workspace member holds `struct Evaluation` |
| TC-748 step 1 | context_only | Before M-6d lands, run `cargo xtask canonical-types` on demand over the FSL workspace. |

## Assurance Context

Repository AGENTS.md, CLAUDE.md, Cargo lint policy, installed code-review, rust-review, rust-style implementation/testing references, and detailed Rust review checks governed the review. No AssuranceProfile was identified by the scoped filename lookup; no profile conformance claim is made.

## Evidence and limits

Trusted author evidence: original red produced empty findings where Identifier was required; final focused `canonical_types::tests::shared_leaf_evaluation_namesakes_fail_until_renamed` passed (1 passed, 242 filtered); changed-file rustfmt passed. Original/intermediate/fixed receipts remain with the author. No tests, builds, lint, gates, or matrix computations were rerun. The lead owns the single final affected-component gate after this review.

The general shared-leaf wording of FR-272 is broader than this implementation; the explicit Evaluation statement and FR-272-AC-5 require this narrow behavior, while existing Copy distinctions remain. This review does not reconcile that wider wording, claim all FR-272-AC-5 fixtures were added, or certify generalized shared-leaf identifier reservation. FR-273-AC-1 is examined only for the gate-reporting facet exercised here. No workspace deletion/rename, full FR-273 completion, all fifty rows, or QSL-2455 facade behavior is claimed. QSL-2721 owns the wider actual defect and blocks QSL-2096.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
