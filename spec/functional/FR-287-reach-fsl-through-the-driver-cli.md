---
id: FR-287
title: "Reach FSL through the driver's thin CLI frontend"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: references
  - target: "ix://agent-ix/filament-specification/FR-301"
    type: depends_on
  - target: "ix://agent-ix/filament-specification/FR-462"
    type: depends_on
---
# FR-287: Reach FSL through the driver's thin CLI frontend

## Description

The user binary is `quire`, built from the driver repository (ADR-029 CB-1).
FSL builds no user binary. Its only executable targets are the developer
tools, the packages that run against source trees and measurements rather
than serve a user and that no FSL library's build depends on: `xtask`, the
packages whose manifest directory is under `tools/`, `fsl-string-edge` (the
standalone string-edge scanner FR-064 requires as a binary) and `fsl-bench`
(whose `fsl-bench-probe` binary MP-006's probe and the Makefile's
`BENCH_PROBE` run). The request
handling FR-027 and FR-100 specify is an FSL library operation, `command`,
which the driver reaches.

The CLI is a thin frontend: it parses arguments, reads the files the
arguments name or stdin, builds a typed request, calls one library
operation, renders the outcome and maps it to an exit code through FR-285.
It makes no semantic decision and reads no diagnostic text (ADR-011 FB-09).
FSpec FR-301 owns the CLI's lifecycle, argument refusal and version output.
FSL's part is the library surface the CLI calls: the operations (FR-275),
`command`, the outcome document (FR-286) and the exit function (FR-285).

The request-file owner is the non-core crate `fsl-command`, whose public
modules are `fsl_command::command` and `fsl_command::formal_source`.
These replace the existing root modules, with no compatibility re-export.
The driver's `fsl-cli` package calls this owner directly for bounded
request preparation and compilation, without depending on the FSL root
package. No core crate depends on `fsl-command`, and its filesystem intake
stays outside the qualified core
([FR-284](FR-284-keep-the-qualified-core-separable-by-crate.md)).

`RunError::outcome_document(operation: fsl_replay::Operation)` returns a
`fsl_replay::OutcomeDocument` through the command owner's pure projection
of its existing typed failure. It SHALL retain the original category,
catalog code, typed cause and available location. It SHALL retain the
spine's reached stage for a stage refusal and `last_stage` null for a
pre-stage intake failure, cancellation or fault as FR-286 specifies.
Source identity causes retain FR-001's typed `label` and optional region.
The projection uses the core outcome types and their one serializer;
neither it nor the driver reads back native JSON or branches on a human
diagnostic message.

The verbs are `parse`, `format`, `check`, `compile`, `run`, `analyze`,
`monitor`, `prove`, `replay`, `generate`, `inspect` and `version` (ADR-029
CB-5). Every verb takes `--format json|text`, and `inspect` also takes
`dot`.

## Inputs

For `command`, a typed request decoded from a request document (FR-027,
FR-100).

## Outputs

`command`'s typed outcome, from which the CLI renders its output (FR-286,
FR-298) and takes FR-285's exit code.

## Behavior

- The FSL workspace shall build no executable target outside the
  developer-tool packages: `xtask`, the packages whose manifest directory is
  under `tools/`, `fsl-string-edge` and `fsl-bench`.
- The FSL root package shall declare no `[[bin]]` target and no
  `default-run`.
- No FSL workspace member other than a developer-tool package shall take a
  normal or build dependency on a developer-tool package; a dev dependency,
  which reaches only that member's tests, is allowed.
- FSL `command` shall be a library operation that takes a typed request and
  returns a typed outcome whose exit code is FR-285's code of its category.
- FSL `command` shall write nothing to the process streams.
- The `quire` frontend's argument, encoding, stream and exit-code behavior for
  the `parse`, `format`, `run` and `compile` verbs shall be as FR-287-AC-3 to
  FR-287-AC-9 state; the driver implements them and TC-1262 specifies the
  tests that bind them.
- The driver shall hand `--output <file>` on `compile` to
  `fsl_command::command::compile_outcome_with_overrides` as `RequestOverrides::output`, which writes
  the checked package to that file as FR-810 states; the driver writes no
  package bytes of its own for it, and `--output` on any other verb is an
  unknown option (FSpec FR-301).
- `--provider-config <file>` on the verbs that route items to providers
  (`analyze`, `prove`, `monitor`, `replay`, `generate`) shall read FSpec
  FR-561's `filament.provider-config/v1` document: its shape (`format`,
  `providers`, each entry's `path` and `arguments`), its refusals (a missing
  or unreadable file naming the path; `invalid-request` naming the file and the
  member for a document that is not JSON, a missing `format` or `providers`,
  an unknown member, an entry with no or a blank `path`, or `arguments` that
  are not strings; `unknown_wire`/`unsupported-wire` for another `format`; each
  exit 20 with empty stdout) and its refusal as an unknown option on a verb
  that routes no items are FSpec FR-561's, and the driver implements them
  (FSpec TC-1241). The driver shall start each configured plugin and pass
  the started providers to the verb's FSL entry as its `providers`
  parameter, by FR-635's rule for every provider-routing entry ("One rule
  for every provider-routing entry"), so FSL's `command` and `analyze` take
  no provider configuration of their own. Each started provider is the
  driver's `ProcessProvider`, an `ExternalProvider` that only transports
  (FR-635 "How a process provider runs an item", FR-950): FSL's analyze
  builds the complete FSpec FR-331 request, `fsl_analyze::provider::ProviderRequest`,
  from the admitted package bytes (FR-940), the routed items and the
  negotiated manifest entry, and calls `ExternalProvider::settle` after
  routing and negotiation. `ProcessProvider` forwards that request
  unchanged through `ProcessProviderSession::invoke` and maps the FR-331
  result back to `ProviderVerdict`. The driver holds no package and no
  routed item, and it has no provider pipeline of its own: the earlier
  `run_configured_providers` path is removed, with no shim.
- `--provider-limit <name>=<value>` on the same provider-routing verbs
  (`analyze`, `prove`, `monitor`, `replay`, `generate`) shall set one of the
  caller limits that FSpec FR-462 and FR-305 say bound each process-plugin
  run. FR-561 keeps these budgets out of the provider-configuration
  document, so one document serves runs of any size. The option may be
  given more than once, each time for a different name. FR-462 gives a
  wire member name only to the deadline (`deadline_ms`, the FR-331 request
  member) and gives no defaults, so this contract fixes the names and the
  defaults below. A limit the user does not give takes its default.

  | Name | FR-462 limit | Unit | Default |
  | --- | --- | --- | --- |
  | `max_frame_bytes` | maximum frame length, in either direction | bytes | 67108864 (64 MiB) |
  | `max_total_bytes` | maximum total frame bytes per plugin run, both directions | bytes | 1073741824 (1 GiB) |
  | `deadline_ms` | the provider deadline, sent as FR-331 `request.deadline_ms` | milliseconds | 600000 (10 minutes) |
  | `grace_ms` | grace period after `cancel` before the host ends the process | milliseconds | 5000 |
  | `max_stderr_bytes` | standard-error bytes captured from the plugin | bytes | 1048576 (1 MiB) |
  | `max_memory_bytes` | process memory limit, where the platform provides one | bytes | 4294967296 (4 GiB) |
  | `max_cpu_ms` | process CPU-time limit, where the platform provides one | milliseconds | 600000 (10 minutes) |

  A value is a positive decimal integer of at most 9007199254740991, the
  FR-331 count range, with no sign, no leading zero and no unit suffix.
  The defaults keep a checked package and its dependencies within one
  request frame with room for base64url growth, and bound a stuck plugin
  to minutes rather than hours. On a platform that provides no memory or
  CPU-time limit, `max_memory_bytes` and `max_cpu_ms` are still validated
  and are then not applied, as FR-305 states. `deadline_ms` binds every
  provider: linked providers receive it in-process as the FR-331 request's
  `deadline_ms` (FSpec FR-331), process plugins in the `request` frame. The
  other six limits are transport limits of a process plugin, which linked
  providers ignore. The option is accepted with or without
  `--provider-config`. The driver shall read every
  `--provider-limit` before any stage runs and before any plugin starts,
  and pass the resolved limits with the started providers as
  `AnalyzeOptions.provider_limits`. FSL's entries parse no option. Analyze
  writes the limits into every `ProviderRequest` it builds (FR-635), and
  `ProcessProvider` enforces the transport limits of the request it carries. An argument with no `=`, an unknown name, a
  name given twice, or a value that is empty, zero, signed, has a leading
  zero, a non-digit or a unit suffix, or is above the range refuses
  `invalid-request` naming `--provider-limit` and the offending argument,
  exit 20 with empty stdout, and starts no plugin. A verb that routes no
  items refuses the option as unknown (FSpec FR-301), exit 20.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-287-AC-1 | `cargo metadata --no-deps --format-version 1` in the FSL repository lists a `bin` target only in the developer-tool packages (`xtask`, the packages whose manifest directory is under `tools/`, `fsl-string-edge` and `fsl-bench`); the root package `filament-spec-language` has no `bin` target and no `default_run`; and no workspace member outside those packages lists one of them as a normal or build dependency. | Test (TC-771) |
| FR-287-AC-2 | `command` called in process with FR-027's compile request over `tests/fixtures/spine-compile.native` returns a typed success outcome whose FR-285 exit code is 0, and with FR-100-AC-6's `call.accounting` `{"work_units": 0}` run request a typed incomplete outcome whose exit code is 22; neither call writes a byte to stdout or stderr. | Test (TC-771) |
| FR-287-AC-3 | `quire run` and `quire compile` with a missing request-file operand, and with an extra operand, exit 20 and write the selected verb's usage to stderr before any file is read: a request path that does not exist is never opened, and no stdout byte is written. `quire parse` and `quire format` with the two labels and no file, and with one extra operand, exit 20 the same way. A `parse` or `format` call with a blank label and a missing file exits 20 with `invalid_source_identity`, cause `blank-label`, the blank label's name in `label`, not a file error. | Test (TC-1262) |
| FR-287-AC-4 | `--request-bytes` and `--dependent-bytes` set `command`'s `request_bytes` and `dependent_bytes` settings for `quire run` (defaults 1 MiB and 8 MiB per FR-026), have no ceiling, and a run limit refusal names the limit, its configured value and the option that raises it. A run request one byte over each setting refuses naming it, and the same request with the option set to its size runs. `quire compile` takes the same two options with the same defaults, and its over-limit refusal names the setting, its maximum and the option (FR-869). | Test (TC-1262) |
| FR-287-AC-5 | An invalid invocation of `parse`, `format`, `run` or `compile` (an unknown option, a missing or extra operand, a malformed option value) exits 20 with no panic, and the same verb run over a valid invocation after it runs normally. | Test (TC-1262) |
| FR-287-AC-6 | A command word or `parse`/`format` label that is not valid UTF-8 (Unix bytes, Windows unpaired surrogates) refuses before the file operand is opened, exit 20, with no panic; a valid source at a file path that is not valid UTF-8 parses. | Test (TC-1262) |
| FR-287-AC-7 | When writing the machine outcome document or formatted text source to stdout fails (stdout bound to `/dev/full`), `quire parse`, `format`, `run` and `compile` exit 30, FR-301's code for tool failure; a partial stdout prefix may remain and is not a complete artifact. | Test (TC-1262) |
| FR-287-AC-8 | When the reader of stdout closes the pipe before an attempted machine-outcome or formatted-source write, `quire parse`, `format`, `run` and `compile` end quietly, write nothing to stderr and exit with the outcome's FR-285 code, not 30. A text-mode write of the rendered outcome (FR-865) follows the same rule. | Test (TC-1262) |
| FR-287-AC-9 | In machine mode, `quire compile` and `quire run` write exactly their producer-owned FR-286 `filament-outcome/1` document to stdout, for success and refusal alike, with no CLI-added or removed member; no native package bytes, native-run-result document or refusal envelope replace it on a process stream. A compile's document is built only after package construction finishes or refuses. Resolving the same valid request as a relative and an absolute operand from two working directories gives equal outcome bytes and exit code. | Test (TC-1262) |
| FR-287-AC-10 | An external Rust consumer with no dependency on `filament-spec-language` calls `fsl_command::command::compile_outcome` over AC-2's compile request and `prepare_run` over its bounded run request. The compile bytes equal the existing `compile` entry's bytes and the document equals FR-286's projection of the actual package result. Executing the prepared run through the existing spine yields the same result and incomplete category as the existing command entry; preparation retains authored labels, package name, packages, libraries, call arguments and accounting. The FSL root defines and re-exports neither `command` nor `formal_source`, and no core crate depends on `fsl-command`. | Test |
| FR-287-AC-11 | Missing request files and malformed request JSON produce the command owner's outcome document with the original catalog code and category and `last_stage` null; source, check and emit refusals retain their owning typed cause, category, reached stage and available region. A source identity refusal retains its exact typed label with no fabricated locus. The command's existing native library result and refusal behavior remain unchanged. | Test |
| FR-287-AC-12 | `quire prove --provider-config providers.json` with no `--provider-limit` sends the fixture plugin a request whose `deadline_ms` is 600000 and enforces a 67108864-byte frame limit; with `--provider-limit deadline_ms=250 --provider-limit max_frame_bytes=1048576` the request carries `deadline_ms` 250 and a 1048577-byte result frame settles the item `failed` with `{"type": "tool-failure", "defect": "frame-length", "limit": 1048576, "value": 1048577}`. `--provider-limit max_frames=4`, `deadline_ms`, `deadline_ms=0`, `deadline_ms=10s`, `deadline_ms=010`, `max_total_bytes=9007199254740992` and `deadline_ms=5` given twice each refuse `invalid-request` naming `--provider-limit` and the argument, exit 20 with empty stdout, and start no plugin; `quire parse --provider-limit deadline_ms=5` refuses the option as unknown, exit 20. | Test (TC-2626) |

## Dependencies

- ADR-029 CB-1, CB-5: the binary and its verbs.
- ADR-011 FB-09: no diagnostic text is read back.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the library operations.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md), [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): exit code and outcome document.
- [FR-027](FR-027-export-compiled-native-package.md), [FR-100](FR-100-run-a-named-function-through-the-spine.md): FSL `command`.
- FSpec FR-462, FR-305 and FR-561: the process-plugin run limits that
  `--provider-limit` sets, and the configuration document that leaves them
  to the caller.
- FSpec FR-301: the CLI lifecycle, argument refusal, and version output, as the
  FSpec half in References aligns it.

## Overlap

The driver repository specifies and tests the `quire` binary against FSpec
FR-301: its verbs and arguments, refusal of an unknown verb, option,
profile or provider before any stage (exit 20), `version`, reading no
configuration file its arguments do not name, and stream-write failures
(exit 30, a closed pipe ending quietly with the outcome's code). For the
`run` and `compile` verbs over FR-026 and FR-027 requests it also owns the
request-file operand, usage and arity errors before any file is read (exit 20),
and writing the producer-owned FR-286 outcome document to stdout unchanged
in machine mode. FR-286 explicitly replaces the earlier native result
stream convention when the driver's verbs land; native library artifacts
remain in-process command products, except the files the command writes to
disk on request: FR-665's handoff directory and FR-810's package file. `run` and `compile` accept the `request_bytes` and
`dependent_bytes` options, and `parse`, `check` and `compile` accept `--limit`
for the settings of the stages they run (FR-869). The provider-routing verbs
accept `--provider-limit` for the process-plugin run limits.
FR-287-AC-12 states its criteria and TC-2626 tests them. For `parse` and `format` it
owns the command and label operands, their OS encoding (a non-UTF-8 command
or label refuses, exit 20), the operand count, writing the outcome document
or formatted source to stdout, and a write failure (exit 30).
FR-287-AC-3 to FR-287-AC-9 state these criteria and TC-1262 tests them.

## References

- QSL-390 (ARCH-50): owner ruling 1 (the user CLI is the driver),
  recorded on the ticket.
- FSpec FR-301 (STD-141): the FSpec half.
