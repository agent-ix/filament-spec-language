---
id: SR-967175026302197768900474181941935084832
title: "spec-review/ears-conformance review of NFR-141"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; spec/non-functional/NFR-141-measure-performance-program-paths.md; QSL-1919"
review_set: subset
---

## Summary

The changed NFR Statement combines multiple independently verifiable SHALL obligations in one statement, weakening atomic traceability. No applicable AssuranceProfile was found under spec/. Quire validates the changed NFR; no production code changed.

- NFR-141 Statement (examined): For QSL-1919, baseline A SHALL use `origin/main` production sources and candidate B SHALL use candidate production sources. The wall-time axes in AC-1 through AC-3 SHALL be built without a replay-counter seam on both sides. Only the isolated parse-counter probe in AC-5 SHALL use the benchmark-only replay-counter seam, applying the identical harness files unchanged to both sides.
- NFR-141 Measurement and Evaluation (examined): A measurement session is immutable: every raw result file from the session, including failed, loaded, or invalid results, SHALL be preserved and reported. A rerun requires explicit lead authorization recorded in its report. No axis result may be selectively replaced within a session.
- NFR-141-AC-1 (context_only): For QSL-1919, `package/emit` is selectable by name and listed in `fsl-bench/BENCHMARKING.md`; it emits checked fixtures with M timed models at M = 1, 10 and 100 (one object type, one clock, one operation, one timed transition and one claim per model), and reports wall time in milliseconds per emitted package and the exponent across M. Candidate B's median wall time is no greater than A's median multiplied by `1 + noise` at each M.
- NFR-141-AC-2 (context_only): For QSL-1919, `package/read` is separately selectable and listed in the guide; it reads the same checked package fixtures at M = 1, 10 and 100, reports wall time in milliseconds per package and the exponent across M, and B's median at each M is no greater than A's median multiplied by `1 + noise`.
- NFR-141-AC-3 (context_only): For QSL-1919, `replay/request` is separately selectable and listed in the guide; it executes one end-to-end replay request over a source package containing 1, 10 or 100 timed models, reports wall time in milliseconds per request, and B's median at each size is no greater than A's median multiplied by `1 + noise`.
- NFR-141-AC-4 (context_only): For QSL-1919, the same `replay/request` input sizes (1, 10 and 100 timed models) each have an isolated peak-RSS probe; B's peak RSS is no greater than A's peak RSS multiplied by `1 + noise` at each size.
- NFR-141-AC-5 (context_only): For QSL-1919, `replay/request` reports parse calls per source document using the parse-call counter feature shared with the model-check counters; the counter is selectable in the benchmark output and B's calls per document do not exceed A's for any of the three package sizes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The changed NFR Statement combines multiple independently verifiable SHALL obligations in one statement, weakening atomic traceability. | spec/non-functional/NFR-141-measure-performance-program-paths.md:16 |

## Verdict

CONDITIONAL: separate independently verifiable obligations to restore atomic traceability.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The combined SHALL sentence was removed and the two provenance obligations now have separate AC-20 and AC-21. |
