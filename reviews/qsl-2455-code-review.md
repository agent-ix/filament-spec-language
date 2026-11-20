---
id: SR-1523
title: "code-review of QSL-2455 consumer facade cleanup"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2455; Cargo.toml, Makefile, fsl-package/src/checked_v2/tests.rs, fsl-replay/src/certificate/closed_form.rs, fsl-replay/src/lib.rs, fsl-replay/src/request.rs, fsl-replay/src/spine/lifecycle.rs, tests/fixtures/facade-contract-consumer/src/lib.rs, tests/fixtures/facade-contract-consumer/tests/it/main.rs, xtask/Cargo.toml, xtask/src/conformance/ci_cargo_double.rs, xtask/src/conformance/tests.rs, xtask/src/consumer_facades.rs, xtask/src/error.rs, xtask/src/facade_tests.rs, xtask/src/main.rs, xtask/src/published_fspec.rs, xtask/src/qualification.rs, xtask/tests/qualification_adapters.rs, xtask/tests/qualification_cli.rs, xtask/tests/qualify_parts.rs"
review_set: subset
---

## Summary

Independent confined diff review of frozen `4c39036eafa2043d396c4f66dbb58ffb35fb9f31`, including the handed cleanup and resolver repair. Ticket: QSL-2455. One private-import resolution defect remains.

## Verdict

**CONDITIONAL** — one medium finding requires a bounded resolver fix and regression.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Private imports used by inherent impls are discarded: for `pub struct Thing; mod extension { use crate::Thing; impl Thing { pub fn extra() {} } }`, the valid public accessor `sample::Thing::extra` is rejected. Record private imports for internal type resolution while preserving public reachability checks. The regression must also assert that the private import path itself stays inaccessible to consumers. | xtask/src/consumer_facades.rs:555; xtask/src/consumer_facades.rs:439 |

## Coverage

The 21 changed files were inspected as one diff. Reviewed cleanup includes authoritative pinned published FSpec reads and retained TempDir lifetimes, concrete facade assertions and corrected trace targets, the positive public-accessor compile doctest, typed facade-test errors, conformance negative summaries, parsed EmptyPackage header spans, and removal of unused dependencies, parameters and dead make declarations. Scanner checks cover exact module/type/member ownership, private source re-exports, named and glob aliases, public module aliases, qualified impls, and private-path rejection. The new tests invoke the real scanner over parsed fixture source; they do not substitute text searches. The positive doctest establishes the compilation behavior FR-911-AC-4 promises; runtime causal-relation behavior belongs to the existing consumer tests, not this compile-only criterion.

No applicable AssuranceProfile was found under spec. No new source vendoring, compatibility layer, type migration, runtime bypass or weakened gate was introduced by this diff. No code edits or public PR comments were made.

Plan completion: not assessed

This is the requested confined gap check, not a new whole-repository matrix audit. Matrix computation, builds, tests, lint and gates were deliberately not repeated. Prior six passing validations and the corrected focused scanner result (three passing tests, exit 0) are accepted as in-lane evidence. The original eight-accessor scanner failure is historical context; the final facade gate remains for the coder after review fixes. No execution PASS is claimed by this artifact.

## Examined criteria

- FR-761-AC-1 (spec/functional/FR-761-bind-every-consumer-fact-to-a-public-accessor.md): `cargo xtask consumer-facades` over the committed bindings exits 0 and its report lists every inventory fact id, `CF-IR-04` with `fsl_package::SchedulabilityRoot::obligation_identity`, `CF-CG-04` with `fsl_replay::check_closed_form`, `CF-RT-02` with `fsl_analyze::monitor_root::admit_progress_view` and `CF-OB-01` with `fsl_package::checked_views::subject_kinds` among them.
- FR-761-AC-2 (spec/functional/FR-761-bind-every-consumer-fact-to-a-public-accessor.md): With a bindings file whose `CF-PR-03` accessors are `["no_such_accessor"]`, the check exits 1 and names `CF-PR-03` and `fsl_package::protocol_view::no_such_accessor`; a crate-private item named as an accessor fails the same way.
- FR-911-AC-2 (spec/functional/FR-911-call-every-bound-consumer-fact-from-an-outside-crate.md): `cargo test -p facade-contract-consumer` runs and passes, with none ignored, one `cf_<code>_<nn>_*` test for each fact of the pinned inventory; among them the protocol tests read, over one compiled protocol clause, its view's roles and channels, the causal relation, each channel's sending role, receiving role and message type, the anchor of an `await`, the owner and visible expressions of a choice and the attempts maximum of a compensation, each equal to the declared value.
- FR-911-AC-4 (spec/functional/FR-911-call-every-bound-consumer-fact-from-an-outside-crate.md): The crate's `compile_fail` doctest naming `fsl_package::protocol_view::relation_over` fails to compile with `E0603`; the same doctest naming the public `fsl_package::protocol_view::causal_relation` compiles.
