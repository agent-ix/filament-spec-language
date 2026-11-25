---
id: SR-967175026302197768900474181941935084857
title: QSL-3309 code review
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language PR #3924; Rust source and tests for FR-284-AC-4, NFR-141-AC-10/11, FR-752-AC-9/10"
review_set: subset
---

## Summary

Reviewed PR #3924 at febcf66a90c451d38f9447204e3a2276e498bcfc against origin/main, including changed Rust source, tagged tests, owning criteria, and relevant FSpec FR-190 resource rules.

## Verdict

FAIL

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The package refusal cause strings duplicate the upstream enum vocabulary and can panic on an otherwise constructible public refusal. | FR-284-AC-4 |
| FND-002 | high | The ambient-input scanner exempts AtomicU64 globals even though changed code adds process-global mutable atomics outside cfg(test). | FR-284-AC-4, TC-768 |
| FND-003 | medium | ParserContext is not reused by process entry points; legacy parse functions reconstruct its grammar tables on every call. | NFR-141-AC-10 |
| FND-004 | medium | TextCache is recreated for every domain query, repeating expensive Unicode table construction without charging that work. | FR-752-AC-10, FR-190-AC-18 |
| FND-005 | medium | Process-global test-support counters can race across concurrently running tests that assert exact counter deltas. | FR-126-AC-26 |

## Verification

No reviewer Cargo commands were run: the target directory was intentionally deleted and QSL-3340 holds the active Terra build slot. Coder-reported focused tests passed for ParserContext, caller forms, FR-752 text limits, normalization, replay tracking, TC-768 alias fixture, and package cause labels. Coder-reported final-head arch-lint qualified-core command passed. Workspace rustfmt check failed with broad formatter churn; changed-file rustfmt also reported formatting differences. NFR-141-AC-11's required live A/B remains queued behind arch-b and was not represented as completed.

### Finding detail

- FND-001: `fsl-package/src/package_admission.rs:321-381` re-states all checked-refusal cause spellings in local `CAUSES`, while the source vocabulary is `CheckedPackageRefusalCause`. The public `PackageAdmissionRefusal.cause: Option<String>` permits arbitrary values, but `catalog_cause` calls `expect` after lookup. This creates a second owner for upstream enum values and makes a public value panic instead of returning a refusal representation.
- FND-002: `tools/arch-lint/qualified_core.rs:709-755` explicitly treats AtomicU64 as not a mutable global; the PR adds `GENERATED_INITIAL` and `CONSTRUCTED_INITIAL` static AtomicU64 values at `fsl-eval/src/simulation/model.rs:4366` and `fsl-eval/src/simulation/protocol.rs:1278`. They are under test-support cfg, not cfg(test), so the scan can pass while mutable process globals remain.
- FND-003: `fsl-cst/src/lib.rs:372-409` keeps free parse entry points and each constructs `ParserContext::new()`. `fsl-bench/src/parse.rs:58-66` continues to call `fsl_cst::parse` per input. The added test at `fsl-cst/src/grammar.rs:3782-3818` compares the address of a field within the same context before and after parses; it does not establish process-entry-point reuse or that grammar construction occurs only once per entry point.
- FND-004: `fsl-eval/src/simulation/model/domain.rs:801,957,967,976` constructs `TextCache::default()` anew for len/nth/exact_len/nth_exact. Normalized/composed alphabet builders scan the Unicode scalar space and construct tables, so repeated candidate queries rebuild tables. This work is not charged by the FR-190 resource meter before allocation. Focused tests verify individual limit outcomes, not amortized reuse or accounting of cache-building work.
- FND-005: `fsl-eval/src/simulation/model.rs:4366` and `fsl-eval/src/simulation/protocol.rs:1278` replace thread-local counters with process-global AtomicU64 counters. Tagged tests such as `fsl-analyze/tests/it/state_graph.rs:1479-1493` and `fsl-analyze/tests/it/protocol_system.rs:705-750` assert exact before/after deltas. Parallel test cases can increment the same counter inside another test's measurement window.

## Coverage

Focused code/test inspection covered changed parser, scanner, package admission, evaluation domain, and tracing paths. The live NFR-141-AC-11 A/B and independent reviewer Cargo verification were unavailable for this pass.
