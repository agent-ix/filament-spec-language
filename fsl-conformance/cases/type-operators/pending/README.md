# Pending required admissions

These original positive units are excluded from the active matrix and do not count as successful conformance.

[QSL-2610](https://linear.app/agent-ix/issue/QSL-2610/resolve-bytes-and-jsonobject-names-in-source-function-signatures), related to completed QSL-2242 and blocking QSL-2046, owns source signature resolution of Bytes and JsonObject. The original combined native unit is preserved unchanged; its UUID/Timestamp functions remain in the active cells. At tested FSL base `2075e4889`, the preserved source refuses assembly with UnresolvedTypeName for Bytes and JsonObject. This is versioned source-surface evidence, not a regression or latest-main claim.

[QSL-2371](https://linear.app/agent-ix/issue/QSL-2371) also blocks QSL-2046. Dividing `a: Rational[1,9;1,9]` by `rational(1,1)` must preserve those intervals under FSpec FR-146 literal point rules, but the original source refuses `undefined_expression/unproved-range`. This evidence was added once to the existing owner without assuming the internal cause is its FR-700 square case. A separate wider-result division unit remains active. Both preserved units require admission through S4; no current refusal is substituted as success.
