# Pending operation parameter programs

`decimal-model/` and `original-decimal-model/` are not pending: `corpus_decimal_model_and_native_parameter_cases` runs both. It runs `decimal-model/` and its `admission/` directory as they stand, and stages `original-decimal-model/` twice, merging each of `workflow.decimal.json` (a Decimal result type) and `workflow.native-decimal.json` (a native Decimal parameter carrying its own policy) into `examples/checked-handoff/model.semantic-ir.json`, then runs `decimal.native` against each complete model.

The cases below are the remaining desired outcomes, excluded from the runnable parameter directory. Evidence was produced from FSL base `f2b6c21a3` with the strict native-run-result reader.

`QSL-1983/optional-parameter-present.native` asks whether the original `probeOptional` operation's `optionalN` parameter is present. The declaration is in `../models/workflow.population.json`: `typeRef` Tally (Integer bounds 0..5), `presence` optional, `nullable` false, `defaultKind` none, multiplicity 1..1. Its source currently refuses S3 `ill_typed/type-mismatch`. The ticket's current Done state is not changed by this versioned evidence. Materialize its workflow model using the same repository base and extension as `stage_parameter_models`; no complete model is duplicated here.

`decimal-keyword-parameter.native` separately preserves access to Meter.adjust's parameter `by`: S1 currently refuses `invalid_syntax/unexpected-token`. Its model request also directly references the repository fixture. The parser refusal precedes model intake.

