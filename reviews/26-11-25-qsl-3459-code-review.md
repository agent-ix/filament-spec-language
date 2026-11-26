---
id: SR-967175026302197768900474181941935084810
title: "Code review — QSL-3459 writer and strict-reader round trips"
type: SpecReview
analysis: code-review
scope: "fsl-analyze/tests/it/model_trace_run.rs, fsl-analyze/tests/it/probability_from_source.rs, fsl-package/src/emit/tests/admission_corpus.rs, fsl-package/src/emit/tests/source_objects.rs; FR-641-AC-4, FR-621-AC-16, FR-741-AC-2, FR-901-AC-1, FR-825-AC-11, FSpec-FR-322"
review_set: subset
---

## Summary

Reviewed the test-only QSL-3459 changes and the existing package-emission round-trip tests they rely on. The added assertions exercise real writer and strict-reader paths, compare decoded values, and retain the existing criterion tags.

## Verdict

**PASS** — the focused tests pass, the changed test files pass formatting, and no correctness or traceability issue was found in this diff.

## Review Notes

- No production code, dependency, or schema copy changed.
- `a_model_trace_whose_expansion_stops_is_incomplete_naming_the_limit` now asserts the strict reader selects a clause result and preserves `incomplete` plus `work_units`.
- `measured_and_proved_items_validate_against_the_backend_provider_schema` now validates the unmodified emitted probability certificate against the pinned schema, reads it strictly, canonically rewrites it, reads it again, and compares the typed values.
- Existing `history_initials_of_each_literal_form_are_written` and `fixture_s_regions_and_rows_follow_the_owner_walk_numbering` cover package writer/reader cases; both focused tests pass.
- Quoin's repository matrix request was refused as too large. Quire's static matrix completed; the touched run-result, certificate, history and body-location tests carry their criteria tags. No full-repository matrix verdict is claimed.
- Workspace `cargo fmt --check` reports unrelated pre-existing formatting differences. File-level `rustfmt --edition 2021 --check` and `git diff --check` pass.
- Broad Clippy and crate suites were not run because QSL-3459 forbids whole-crate suites; focused test commands and their results are recorded on the ticket.

## Tests

- `cargo test -p fsl-package --lib history_initials_of_each_literal_form_are_written -j 4` — passed.
- `cargo test -p fsl-package --lib source_objects::fixture_s_regions_and_rows_follow_the_owner_walk_numbering -j 4` — passed.
- `cargo test -p fsl-analyze --test it_model_check a_model_trace_whose_expansion_stops_is_incomplete_naming_the_limit -j 4` — passed.
- `cargo test -p fsl-analyze --test it_probabilistic measured_and_proved_items_validate_against_the_backend_provider_schema -j 4` — passed.

## Findings

| ID      | Severity | Summary                  | Refs |
| ------- | -------- | ------------------------ | ---- |
| FND-001 | low      | No findings (placeholder) | -    |
