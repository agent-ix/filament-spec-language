---
id: TC-1396
title: "qualify covers the corpus, the lifecycle matrix and parity"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-567
    type: verifies
---
# TC-1396: qualify covers the corpus, the lifecycle matrix and parity

## Description

Verify that the `qualify` verdict requires all three parts of the final
path.

Scope: FR-567-AC-1 to FR-567-AC-5, FR-567-AC-7 and FR-567-AC-8.

## Test Procedure

The runs through `xtask::qualify::run` may set a replacement generated-unit
count.

1. Run `qualify` over FR-353-AC-1's passing fixture corpus with the matrix
   cases and a linked test backend that delegates to the interpreter.
2. Through `xtask::qualify::run` under `test-support`, repeat with the matrix case set replaced by one whose `check`/`refusal`
   case expects exit 21.
3. The same way, repeat step 1 with FR-295-AC-1's rounding backend supplied
   instead.
4. Repeat step 1 with no non-interpreter backend, and with a substituted
   parity reference that returns `Fault`.

5. Read step 1's report's FSL version; build a scratch copy whose
   `filament-spec-language` package version is `9.9.9` and run step 1 there.
6. Through `xtask::qualify::run`, repeat step 1 with a replacement
   generated-unit count of 3 and a backend counting its `prepare` calls; run
   `cargo xtask qualify` with `--count 3`; inspect the values the `parity`
   part takes with no replacement.
7. Repeat step 1 with an added `runtime`/`run` vector that expects a value and
   names an absent source file, then over a corpus with two vectors of one id
   and over one with a parent-traversing input path.

Tag the tests `#[trace("TC-1396", "<AC id>")]`.

## Expected Results

- Step 1: `complete-v1: qualified`, three parts passed, exit 0.
- Step 2: `not-qualified` naming `lifecycle-matrix/check/refusal`, exit 1,
  with the `corpus` part passed.
- Step 3: `not-qualified` naming `parity/jit` and its finding, exit 1.
- Step 4: parity `no-backend` and `complete-v1: qualified`, exit 0; then
  `not-qualified` naming `parity/jit` with a `reference-fault` finding,
  exit 1.
- Step 5: the version equals the crate's `CARGO_PKG_VERSION`, then
  `9.9.9`.
- Step 6: a pass, with one `prepare` call per corpus parity case and three
  more; `--count` refused, exit 2; the defaults are FR-295's seed and count
  10,000 and `ParityLimits::default()`.
- Step 7: `not-qualified` naming `parity/jit` with an `unbuildable-case`
  finding for that vector, the other vectors executed, exit 1; then for each of
  the two corpora a preflight refusal with no `prepare` call and no verdict,
  exit 2.
