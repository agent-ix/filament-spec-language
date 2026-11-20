---
id: SR-1467
title: "Code and Rust review of QSL-2076 grammar near-misses"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language PR3146 at 406d58317fed97f48e981e95e728866cc4ca189f; fsl-cst/tests/it/{time_declaration,hybrid_declaration,probabilistic,synthesis_verify,main}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---

## Summary

Reviewed only the seven new behavioral tests, their parser assertion helpers and three integration registrations. The real parser rejects the authored wrong keywords, ordering, missing clauses, separators and EOF cases; valid counterparts assert whole production spans and byte-exact rendering. The formatting cleanup was resolved using the author’s in-lane receipt.

## Verdict

**PASS** — accepted the author’s in-lane receipt at 818adf75bda50dbd049177ad524869196be0dbc8: both generic mutation loops and mutation-only bookkeeping are removed; active positive/exact-boundary coverage and strict explicit near-miss oracles remain. The one changed qualified function passed 1/0. This is a record-only disposition, with no extra code read, review sweep or runtime.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | New hybrid and verification tables use unformatted tuple bindings and loop bodies; apply rustfmt to the changed Rust files. | fsl-cst/tests/it/hybrid_declaration.rs:12; fsl-cst/tests/it/synthesis_verify.rs:45 |

## Coverage

- Rust review loaded rust-style and its implementation/testing rules; the formatting rule requires the repository formatter. README names formatting as part of the local gate.
- Examined all new time, invariant/clock-bound, urgent, hybrid, delay, synthesis and verification tests, including the UTF-8-prefixed span helper and valid production/render controls. The FSpec-prefixed grammar trace tags use the repository's declared namespace.
- No copied external fixtures, production changes, lifecycle code, stub tests, ignored cases or weakened gates occur in this diff.
- Current FSL FR-002, FR-393 and FR-762 were context for parse-only ownership and diagnostic behavior. No S2/solver completion claim is made.
- Coder's seven focused passing functions and scoped CST Clippy are accepted without rerunning them. No build, product probe, matrix or existing-suite check ran in review. One final applicable component gate remains the coder's post-review step.
- Spec review is inapplicable: no specification or plan file changed. Gap analysis is recorded separately as SR-1468.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Author's in-lane receipt reports scoped rustfmt at 861fde0009d65000cb71bd5b65378b151cdb3ae4; tuple-binding and loop-body spacing only, with all inputs, assertions and semantics unchanged. Accepted without another code read, test, lint or review sweep. |
| FND-002 | still-open | Replace the whole original final significant token with equal-width spaces and a final standalone `#`, retaining the strict diagnostic-covering-boundary assertion. A suffix-byte mutation of a word can instead invalidate an earlier token prefix. |
| FND-002 | still-open | Round 3 supersedes the proposed whole-token marker repair: `#` can create a valid ReferenceLiteral prefix after a CaseArm expression. Remove the generic negative loops and their false coverage claim; retain positive production/boundary checks and strict grammar-aware near-miss fixtures. QSL-2686 blocks QSL-2076 until this harness correction lands in PR3146. |
| FND-002 | fixed | Accepted the author's in-lane implementation receipt at 818adf75bda50dbd049177ad524869196be0dbc8: both generic mutation loops and mutation-only bookkeeping removed; active function renamed for positive/exact-boundary assertions; negative-production criterion tag removed; explicit located negative fixtures unchanged. The only changed qualified function passed 1/0. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | Right-byte mutation of an exact word terminal can fail on its preceding invalid prefix, so it is not a valid oracle for a diagnostic covering the final mutated byte. Construct a standalone invalid final token while retaining the exact boundary assertion. | fsl-cst/tests/it/complete_grammar.rs:587; fsl-cst/src/grammar.rs:439; fsl-cst/src/parser.rs:799 |

## Confined gate-failure correction review

- Scope: only the correction diff from 861fde0009d65000cb71bd5b65378b151cdb3ae4 to 6540b9901112b0050ad51bf8b0914753e6cf1260. No additional review sweep, test, build, baseline probe or owner audit ran.
- Accepted four grounded corrections: explicit caller-selected work budget on admissible input; whole-declaration recovery region distinguished from the exact diagnostic token span; README native-fence recognition allowing stage metadata; and a genuine expression-choice trigger for the bounded expected-set message. Only these four changed-name passes count; the reverted history-oracle pass does not.
- Seven unchanged required integration oracles remain strict pending under QSL-2680 through QSL-2685; the formatter oracle remains pending under QSL-2679. Added ignore annotations identify their owners without changing assertions or claiming coverage. The public receipt preserves the original failed component gate and segmented results.
- The added ReferenceLiteral inventory entry has a real authored reference-literal vector. It does not justify weakening another production's location oracle.
- Source-grounded remaining issue: `#` is a recognized token (token.rs:216). `RoundingMode` and `TextProfile` match exact words (grammar.rs:439-458). `nearest-even` mutated to `nearest-eve#`, or `nfc` to `nf#`, can fail at the earlier nonmatching prefix. The terminal matcher compares whole token spelling (parser.rs:2673), and `failure_span` reports that failing token's span (parser.rs:799-809). The contract does not require that span to reach the later `#`.
- Narrow correction: capture the original final significant CST token span when collecting each positive production occurrence, require that it ends at the production boundary, replace its entire lexeme with equal-width spaces and a final standalone `#`, and keep the existing strict covering-boundary assertion. No per-production exception, prefix-span alternative, product grammar change or blanket weakened assertion is warranted.
- The failure logs omit actual diagnostic/span values; none is inferred here. The corrected vector's single changed-name execution belongs to the original coder after implementation.

## Final source/contract disposition (pass 3)

The coder's single authorized changed-function run at 28a4517602b0d8a01260d443a79b77a94c5d12da failed for `CaseArm`: the mutated source contains `Circle(r): r# Shape::Rect(w,h)...`, and the observed diagnostic is `InvalidSyntax`/`UnexpectedToken` at [231,236), expecting quoted text at the following `Shape`. This supplied evidence is accepted without rerunning or reading logs for confidence.

The exact grammar explains why no universal marker-location oracle follows: `ReferenceLiteral` is `QualifiedName`, `#`, text (grammar.rs:1038), and `CaseArm` ends in Expression followed by `;` (grammar.rs:1165). Replacing that semicolon by `#` can extend the preceding expression into a different valid grammar prefix. The earlier generic whole-token recommendation was unsound and is withdrawn; this is a harness defect, not a demonstrated parser defect.

QSL-2686 records the actual defect in the owning examples/corpus project, blocks QSL-2076 and relates to the existing Done QSL-2107 without reopening it. The concrete same-PR fix is to remove both generic negative mutation loops, their occurrence-only mutation bookkeeping and claimed universal negative coverage. Keep the independent accepted-production inventory, all real-source positive parses, node byte spans/rendering, formatter token/tree preservation and idempotence, and the real ReferenceLiteral vector. Rename the test for those actual positive/boundary assertions and keep only trace bindings those assertions discharge.

Do not ignore the entire production inventory or retain a strict-pending parser regression asserting a location the grammar does not require. No dedicated product regression is justified by this evidence. Preserve existing explicit grammar-combinator and near-miss fixtures with their strict typed diagnostic and byte-span assertions; additional grammar-aware negatives belong to the already-owned QSL-2075..2078/2107 program, with unimplemented coverage stated truthfully. No arbitrary replacement marker, per-production exception or runtime trial loop is recommended.

This final disposition ran no tests, builds, full review, baseline probes or owner audits. The seven new feature functions, four grounded stale corrections, strict product-owner annotations and original failed-gate receipt remain accepted and unchanged.

## Final record-only disposition (pass 4)

The retained active function still checks the independent production inventory, actual positive parsing, node rendering and exact byte spans, formatter significant-token/tree identity and idempotence, including the real ReferenceLiteral vector. No false generic negative-coverage claim remains. The original failed full gate and all attempts remain public; eight strict functions under QSL-2679 through QSL-2685 stay excluded from coverage. The seven new feature functions and four grounded corrections remain accepted. This disposition uses the author's receipt without another code read, review sweep, test, build or runtime probe. QSL-2686 can be closed when the same-PR correction merges.
