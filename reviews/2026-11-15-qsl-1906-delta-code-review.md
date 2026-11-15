---
id: SR-2026111414542500000005
title: "Code review of PR #2714 macOS RSS reader delta"
type: SpecReview
analysis: code-review
scope: 'agent-ix/filament-spec-language@f22fba6f9295f23872cc6262d41198392652f217; delta d524008c0976dbeb8b08b775b4cf9bb86a34cad4..f22fba6f9295f23872cc6262d41198392652f217; Cargo.lock, fsl-bench/Cargo.toml, fsl-bench/src/rss.rs, fsl-bench/BENCHMARKING.md'
review_set: subset
date: '2026-11-15'
---

## Summary

Reviewed the macOS `getrusage(RUSAGE_SELF)` implementation, nix dependency and related unit documentation at PR #2714 head `f22fba6f9295f23872cc6262d41198392652f217`. The safe nix API returns Darwin `ru_maxrss` in bytes, and the implementation converts to KiB; no applicable AssuranceProfile was found under `spec/`. One stale source comment remains after removing the `vmmap` reader.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| macOS peak RSS implementation and docs | fsl-bench/src/rss.rs | examined |
| nix target dependency and lock entries | fsl-bench/Cargo.toml, Cargo.lock | examined |
| benchmark measurement description | fsl-bench/BENCHMARKING.md | examined |
| `getrusage` and `max_rss` safe wrapper/unit contract | nix 0.31.3 source in the local Cargo registry | context_only |
| QSL-1906-AC-2 | Linear issue QSL-1906 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `#[string_edge]` explanation still says the function reads a host tool's text report by field label. The macOS path now reads nix's `getrusage` result, so this explanation no longer describes the annotated function. | fsl-bench/src/rss.rs:14-15 |

## Verdict

Conditional: the new macOS reader uses a safe API and its byte-to-KiB conversion matches nix's documented Darwin unit. Fix the stale `#[string_edge]` explanation before merge. Focused formatter, tests, Clippy and cargo-deny checks passed on `aarch64-apple-darwin`; no aggregate gate was run.

## Verification

- `nice -n 19 /Users/peter/dev/_lead-bin/qsl-slot cargo fmt --all -- --check` — exit 0 (the `nice` priority request was denied by the environment).
- `nice -n 19 /Users/peter/dev/_lead-bin/qsl-slot /Users/peter/dev/_lead-bin/qsl-timeout 1800 cargo test --locked -p fsl-bench` — exit 0; 21 passed, 0 failed on `aarch64-apple-darwin`.
- `nice -n 19 /Users/peter/dev/_lead-bin/qsl-slot cargo clippy --locked -p fsl-bench --all-targets -- -D warnings` — exit 0.
- `nice -n 19 /Users/peter/dev/_lead-bin/qsl-slot cargo deny check bans --config ../deny.toml` from `fsl-bench/` — exit 0.
- `git diff --check d524008c0976dbeb8b08b775b4cf9bb86a34cad4..f22fba6f9295f23872cc6262d41198392652f217` — exit 0.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The comment now describes the OS-provided Linux and macOS RSS sources used by the function. |
