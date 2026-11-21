---
id: SR-2026112117335574496646
title: "code-review (Markdown lane) of QSL-1978 imported-locus wire amendment"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@720f29572ab49e9bc6d05953f8c57f35f2b1037f; spec/functional/FR-267-write-run-results-as-native-run-result-2.md, spec/functional/FR-663-report-the-parsed-unit-and-the-locus-of-every-refusal.md, spec/test-cases/TC-1746-refusal-records-carry-a-locus.md, spec/test-cases/TC-3081-input-document-refusal-locus-round-trips.md"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-663
    type: reviews
---

## Summary

Ticket: QSL-1978 (PR filament-spec-language#3352). Reviewed frozen head 720f2957 against
merge base f89c2e03 (`git diff origin/main...HEAD`, four Markdown files, no source). This is
the Markdown lane of code-review: it checks the amendment's claims against the carriers that
already ship, without asserting that any carrier is implemented in the new form. Existing
carriers, read at the frozen head: the canonical outcome locus `OutcomeLocus::Located { file,
span }`, whose span endpoints are byte/line/column `Position`s; and the command-error
`details` projection for spine and native diagnostics, `Details::Spine { source, path, span }`
and `Details::Native(Diagnostic { .., source, .. })`, where `source` is a required
`SourceIdentity` pair (fsl-command/src/command/output/types.rs:49-113). Neither carries
`sourceIdentity` today, and the amendment does not claim otherwise.

## Verdict

**FAIL (one medium finding).** The `located` locus amendment matches the approved contract.
It extends the existing row rather than adding a kind. `sourceIdentity` uses the FCD
spelling and is required exactly for the imported form. Bytes are omitted at both endpoints.
No native pair is inferred from the URI, `ix://` included. The ordinary native span is
unchanged. The strict-reader refusals stay inside the existing located wire refusals. The
FR-663 projection names both the compile route and compile reached through `run`, and
operation-parameter origin priority. The one defect is that the command-error `details`
side of the same projection is stated only as an omission, over a carrier the spec never
defines.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-267 rules that "any accompanying command-error source context" for an imported origin SHALL omit a native source-label pair, but no FR defines that carrier. The command-error `details` for a spine compile refusal are "as FR-027", which lists no members, and the shipped projection is `{source, path, span}` with `source` a required pair and `span` byte-bearing. The amendment does not say whether, for an imported origin, `details` drops `source`, drops the whole source context, or carries the imported path, the URI and a byteless point. TC-1746 steps 4 and 7 inspect "any command-error source context", so an adapter that omits `details` entirely, or keeps the program's `path` and span with only `source` removed, passes. The native-run envelope projection is therefore not concrete. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:272 |

## Gates

No build, cargo, runtime or full gate was run (reviewer-only brief). `quire validate --scope .`
over the four changed artifacts at their exact paths exited 0. Not applicable: rust-review,
since the diff has no `.rs` file or `Cargo.toml`, and gap-analysis, since the diff has no
production code.
