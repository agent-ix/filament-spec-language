---
id: SR-1469
title: "Review value declaration near-miss tests"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2075; codex/qsl-2075-value-near-misses at 2eee6ee41514465cf35481a71dd3eef8db56b46e; fsl-cst/tests/it/value_declarations.rs and its registration in fsl-cst/tests/it/main.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-002
    type: references
  - target: ix://agent-ix/filament-specification/FR-339
    type: references
---

## Summary

Independent Rust and code review of the frozen QSL-2075 feature branch before PR opening. The 28 malformed declarations and six accepted counterparts exercise the public parser and make concrete grammar, diagnostic-location and lossless-source assertions. No findings.

Target: `codex/qsl-2075-value-near-misses` at `2eee6ee41514465cf35481a71dd3eef8db56b46e`. No PR existed at review time; opening this unchanged feature does not require a second review.

## Verdict

**PASS** for the assigned diff. This does not claim that the repository's unrelated CST failures are resolved or that a final component gate has passed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FSpec FR-339-AC-1 | examined | Every production in the base and complete-V1 facet grammar has positive, boundary and located-negative corpus vectors. |
| FSpec FR-339-AC-2 | examined | Parsing and lossless rendering reproduce accepted UTF-8 source bytes exactly, including comments and whitespace. |
| FSL FR-002-AC-6 | examined | The bounded malformed/truncation corpus returns a `ParsedSource` or a located diagnostic from `fsl_cst::parse` without panic; any admissible variant retains its exact submitted bytes. |
| FSL FR-762, One diagnostic per error | context_only | A unit with exactly one syntax error SHALL give exactly the one diagnostic, cause, locus and message it gives without recovery. |

The normative production fragments examined are the six value declarations at pinned FSpec `f32bca6c7937e0652742b2bbe27a52419bf5a0ae`, `proposals/filament-v1/shared-grammar.md:410` through the alias production at line 424. Enum trailing commas are admitted; tuple trailing commas are not. The tests respect those distinctions and do not invent name-resolution errors at S1.

Literal requirement paths used:

- `spec/functional/FR-002-parse-native-units.md` in the assigned language worktree.
- `spec/functional/FR-762-report-every-independent-syntax-error-of-a-unit.md` in the assigned language worktree, as diagnostic context only.
- `spec/functional/source-package/FR-339-define-complete-native-grammar.md` in `/home/peter/dev/quire-specification` at the pinned FSpec revision above.

## Rust and behavior checks

The new file has its required SPDX header and uses the public `fsl_cst::parse` API with caller-owned source identity and path. Helpers share only this test family's setup, use deterministic inline originals, and do not replace parser behavior. Accepted controls require admissibility, no diagnostics, the intended production and exact node span/rendering. Negative controls fail if malformed syntax is admitted, the diagnostic code/cause or byte span is wrong, or source bytes are lost. The prefix deliberately includes UTF-8 and CRLF to distinguish byte offsets from character offsets. EOF vectors require zero-width source-end loci. All test functions carry qualified FSpec and/or FSL trace tags. The registration line exposes the new tests to the existing integration target.

No production code, dependencies, safety boundaries, workflow gates, compatibility layer or copied external artifact is added by this diff.

## Validation evidence

Accepted the coder's focused results for all 34 cases and scoped `fsl-cst --test it` Clippy result. No tests, builds, product probes, formatter sweeps or previous corpus checks were run by this reviewer. Known unrelated CST gate failures remain with their original owner. The code lead owns the one applicable final component gate after review.

The filename filter that briefly included historical review filenames was stopped without opening those files. All subsequent requirement reads used exact paths.
