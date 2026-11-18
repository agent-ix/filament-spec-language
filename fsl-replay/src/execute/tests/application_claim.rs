// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-098-AC-22 to FR-098-AC-26 (TC-444 step 10): the application claim a
//! `function` request's occurrence key selects, over real complete-V1
//! source recompiled through the spine.

use super::*;

fn unit() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n{PROFILE}\
         function p using v(x: Int[0, 9]): Boolean pure {{ x > 3 }}\n\
         function q using v(x: Int[0, 9]): Integer pure {{ x + 1 }}\n\
         function r using v(x: Int[0, 9]): Integer pure {{ 10 mod x }}\n\
         function s using v(x: Int[0, 9]): Integer pure {{ (if x > 0 then 10 mod x else 0) + 10 mod x }}\n\
         function b using v(t: Sequence<Int[0, 9]>[0, 3]): Boolean pure {{ forall(v in t: 10 mod v > 0) }}\n\
         function u using v(x: Int[0, 9], y: Int[0, 9]): Integer pure {{ 10 mod x }}\n\
         function inc using v(x: Int[0, 9]): Int[0, 10] pure {{ x + 1 }}\n\
         function ub using v(x: Int[0, 9], flag: Boolean): Integer pure {{ 10 mod x + (if flag then 0 else 10 mod (x - 1)) }}\n\
         function ur using v(x: Int[0, 9], y: Int[3, 9]): Integer pure {{ 10 mod x + 10 mod (y - 3) }}\n\
         function un using v(x: Int[0, 9], y: Int[-9, -2]): Integer pure {{ 10 mod x + 10 mod (y + 2) }}\n\
         function us using v(x: Int[0, 9], t: Sequence<Int[0, 9]>[0, 3]): Integer pure {{ 10 mod x }}\n\
         function g using v(x: Int[0, 9]): Integer pure decreases(x) {{ 10 mod x + (if x > 0 then h(x - 1) else 0) }}\n\
         function h using v(x: Int[0, 9]): Integer pure decreases(x) {{ if x > 0 then g(x - 1) else 0 }}\n\
         function f using v(x: Int[0, 9]): Integer pure decreases(x) {{ 10 mod x + (if x > 0 then f(x - 1) else 0) }}\n"
    )
}

/// The `expression` occurrence of the `nth` (in source order) application of
/// `owner` whose source text is `needle`, read from the compiled package's
/// requirement records.
fn record(
    compiled: &ComposedUnit,
    source: &str,
    owner: &str,
    needle: &str,
    nth: usize,
) -> OccurrenceKey {
    let graph = compiled.package.graph();
    let mut found: Vec<(u64, OccurrenceKey)> = graph
        .requirements()
        .iter()
        .filter(|(_, record)| record.owner() == Some(owner))
        .filter_map(|(key, _)| {
            let location = graph.occurrence(key.node().clone(), key.origin())?;
            let region = graph.region(location)?;
            let text = &source
                [usize::try_from(region.start()).ok()?..usize::try_from(region.end()).ok()?];
            (text == needle).then(|| (region.start(), key.clone()))
        })
        .collect();
    found.sort_by_key(|(start, _)| *start);
    found
        .get(nth)
        .unwrap_or_else(|| panic!("`{owner}` holds no record #{nth} at `{needle}`"))
        .1
        .clone()
}

fn declaration(compiled: &ComposedUnit, function: &str) -> OccurrenceKey {
    let identity = compiled
        .package
        .graph()
        .callable(function)
        .expect("declared")
        .identity;
    OccurrenceKey::new(identity.clone(), Origin::new(Role::new("declaration"), 0))
}

/// A request for `function` at `key` carrying `source`.
fn at(
    source_text: &str,
    compiled: &ComposedUnit,
    function: &str,
    key: &OccurrenceKey,
    source: ReplaySource,
) -> ReplayRequestWire {
    let mut wire = request(source_text.as_bytes(), compiled, name(&[function]), source);
    wire.replay_target = crate::RequestTargetWire::function(name(&[function]), key.clone());
    wire
}

/// What a replay settled, reduced to the AC's terms.
#[derive(Debug, Eq, PartialEq)]
enum Settled {
    Reproduced(Option<EvaluatedValue>),
    ReproducedWitness(Option<EvaluatedValue>),
    Verdicts(Option<EvaluatedValue>),
    NoValue,
}

fn settled(result: Result<ReplayResult, ReplayRefusal>) -> Settled {
    match result.expect("the replay settles") {
        ReplayResult::Input(arm) => match arm.settlement() {
            InputSettlement::ReproducedWithoutWitness => Settled::Reproduced(arm.value()),
            InputSettlement::Inconclusive => match arm.disagreement() {
                Some(crate::DisagreementCause::Verdicts { .. }) => Settled::Verdicts(arm.value()),
                Some(crate::DisagreementCause::NoValue { .. }) => Settled::NoValue,
                other => panic!("unexpected disagreement {other:?}"),
            },
        },
        ReplayResult::Witness(arm) => {
            assert!(
                arm.record().is_none(),
                "a function call carries no FR-351 record"
            );
            match arm.settlement() {
                WitnessSettlement::ReproducedWithEvaluatedWitness => {
                    Settled::ReproducedWitness(arm.value())
                }
                other => panic!("unexpected settlement {other:?}"),
            }
        }
        ReplayResult::ObservedTemporal(_) => panic!("function replay returned a temporal arm"),
        ReplayResult::ModelGraph(_) => panic!("function replay returned a state-graph arm"),
        ReplayResult::ProtocolAnalysis(_) => panic!("a protocol analysis arm"),
    }
}

fn boolean(value: bool) -> Option<EvaluatedValue> {
    Some(EvaluatedValue::Boolean(value))
}

fn run(
    source: &str,
    compiled: &ComposedUnit,
    function: &str,
    key: &OccurrenceKey,
    values: &[(NodeKey, WitnessValue)],
) -> Result<ReplayResult, ReplayRefusal> {
    let assignments = values
        .iter()
        .map(|(parameter, value)| CanonicalAssignment {
            parameter: parameter.clone(),
            value: value.clone(),
        })
        .collect();
    replay(
        at(
            source,
            compiled,
            function,
            key,
            ReplaySource::Input(assignments),
        ),
        ReplayLimits::default(),
    )
}

fn int(value: i128) -> WitnessValue {
    WitnessValue::Integer(value)
}

/// FR-098-AC-22: the predicate claim at `declaration`, and the application
/// claim at a record's occurrence: undefined (or out of bound) reproduces,
/// a completed application is a `Verdicts` disagreement.
#[trace("TC-444", "FR-098-AC-22")]
#[test]
fn tc_444_the_occurrence_key_selects_the_predicate_or_the_application_claim() {
    let source = unit();
    let compiled = kernel_spine(&source, &BTreeMap::new());
    let x = |function| parameter(&compiled, function, 0);

    let p_declaration = declaration(&compiled, "p");
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "p",
            &p_declaration,
            &[(x("p"), int(0))]
        )),
        Settled::Reproduced(boolean(false))
    );
    let p_application = record(&compiled, &source, "p", "x > 3", 0);
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "p",
            &p_application,
            &[(x("p"), int(0))]
        )),
        Settled::Verdicts(boolean(true))
    );

    let r_application = record(&compiled, &source, "r", "10 mod x", 0);
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "r",
            &r_application,
            &[(x("r"), int(0))]
        )),
        Settled::Reproduced(boolean(false))
    );
    let witness = |value: &str| {
        ReplaySource::Witness(
            Witness::parse(format!(
                "<<<assertion|h|c|{}={value}>>>",
                crate::pn(&x("r"))
            ))
            .unwrap(),
        )
    };
    assert_eq!(
        settled(replay(
            at(&source, &compiled, "r", &r_application, witness("0")),
            ReplayLimits::default()
        )),
        Settled::ReproducedWitness(boolean(false))
    );
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "r",
            &r_application,
            &[(x("r"), int(3))]
        )),
        Settled::Verdicts(boolean(true))
    );
    let q_application = record(&compiled, &source, "q", "x + 1", 0);
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "q",
            &q_application,
            &[(x("q"), int(9))]
        )),
        Settled::Verdicts(boolean(true))
    );

    // The obligation identity is provenance: any value is carried and not
    // checked against the claim.
    let mut identified = at(
        &source,
        &compiled,
        "p",
        &p_application,
        ReplaySource::Input(vec![CanonicalAssignment {
            parameter: x("p"),
            value: int(0),
        }]),
    );
    identified.obligation_identity =
        crate::ObligationIdentity::new(crate::fixture_node(9), None, 0);
    assert_eq!(
        settled(replay(identified, ReplayLimits::default())),
        Settled::Verdicts(boolean(true))
    );
    // The identity is carried unchanged into the request the result settles
    // from: a distinct identity per request survives decoding.
    for identity in
        [1_u8, 2].map(|n| crate::ObligationIdentity::new(crate::fixture_node(n), None, 0))
    {
        let mut wire = at(
            &source,
            &compiled,
            "p",
            &p_application,
            ReplaySource::Input(vec![CanonicalAssignment {
                parameter: x("p"),
                value: int(0),
            }]),
        );
        wire.obligation_identity = identity.clone();
        let request = ReplayRequest::decode(wire, ReplayLimits::default()).unwrap();
        assert_eq!(request.obligation_identity(), identity);
    }
}

/// FR-098-AC-23: two records of one node are two claims, each at its own
/// location.
#[trace("TC-444", "FR-098-AC-23")]
#[test]
fn tc_444_each_occurrence_of_one_node_is_a_claim_of_its_own() {
    let source = unit();
    let compiled = kernel_spine(&source, &BTreeMap::new());
    let x = parameter(&compiled, "s", 0);
    let then_branch = record(&compiled, &source, "s", "10 mod x", 0);
    let right_operand = record(&compiled, &source, "s", "10 mod x", 1);
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "s",
            &then_branch,
            &[(x.clone(), int(0))]
        )),
        Settled::NoValue
    );
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "s",
            &right_operand,
            &[(x.clone(), int(0))]
        )),
        Settled::Reproduced(boolean(false))
    );
    assert_eq!(
        settled(run(&source, &compiled, "s", &then_branch, &[(x, int(3))])),
        Settled::Verdicts(boolean(true))
    );
}

/// FR-098-AC-24: every evaluation of a binder body is observed; a binder
/// root is never bound by the source; an unread parameter is filled and an
/// extent root left unbound refuses.
#[trace("TC-444", "FR-098-AC-24")]
#[test]
fn tc_444_binder_bodies_are_observed_and_unread_parameters_are_filled() {
    let source = unit();
    let compiled = kernel_spine(&source, &BTreeMap::new());
    let t = parameter(&compiled, "b", 0);
    let sequence = |elements: &[i128]| {
        WitnessValue::Sequence(elements.iter().map(|element| int(*element)).collect())
    };
    let b_application = record(&compiled, &source, "b", "10 mod v", 0);
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "b",
            &b_application,
            &[(t.clone(), sequence(&[3, 0]))]
        )),
        Settled::Reproduced(boolean(false))
    );
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "b",
            &b_application,
            &[(t.clone(), sequence(&[3]))]
        )),
        Settled::Verdicts(boolean(true))
    );
    // `v` is the binder of the quantifier: no parameter of `b`.
    let binder = record_binder_node(&compiled, "b");
    let refused = run(
        &source,
        &compiled,
        "b",
        &b_application,
        &[(t, sequence(&[3])), (binder.clone(), int(0))],
    )
    .unwrap_err();
    assert!(matches!(refused, ReplayRefusal::UnknownParameter(node) if node == binder));

    let x = parameter(&compiled, "u", 0);
    let y = parameter(&compiled, "u", 1);
    let u_application = record(&compiled, &source, "u", "10 mod x", 0);
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "u",
            &u_application,
            &[(x.clone(), int(0))]
        )),
        Settled::Reproduced(boolean(false))
    );
    let refused = run(&source, &compiled, "u", &u_application, &[(y, int(1))]).unwrap_err();
    assert!(matches!(refused, ReplayRefusal::UnboundParameter(node) if node == x));
    // The predicate claim binds every parameter.
    let u_declaration = declaration(&compiled, "u");
    let refused = run(
        &source,
        &compiled,
        "u",
        &u_declaration,
        &[(x.clone(), int(0))],
    )
    .unwrap_err();
    assert!(matches!(refused, ReplayRefusal::NotAPredicate { .. }));
    // A witness transcript may likewise leave the unread parameter out.
    let transcript = ReplaySource::Witness(
        Witness::parse(format!("<<<assertion|h|c|{}=0>>>", crate::pn(&x))).unwrap(),
    );
    assert_eq!(
        settled(replay(
            at(&source, &compiled, "u", &u_application, transcript),
            ReplayLimits::default()
        )),
        Settled::ReproducedWitness(boolean(false))
    );
}

/// The node of `function`'s first `forall` binder: the node of a
/// quantifier's bound variable, which is no parameter.
fn record_binder_node(compiled: &ComposedUnit, function: &str) -> NodeKey {
    let graph = compiled.package.graph();
    let record = graph
        .requirements()
        .values()
        .find(|record| record.owner() == Some(function))
        .expect("a record");
    let parameters: Vec<NodeKey> = (0..1).map(|i| parameter(compiled, function, i)).collect();
    record
        .extent_roots()
        .iter()
        .find(|root| !parameters.contains(root))
        .expect("the binder is an extent root")
        .clone()
}

/// FR-098-AC-25: a key that selects no claim of the function refuses
/// `ClaimOccurrence` before any call; the predicate and value refusals are
/// unchanged.
#[trace("TC-444", "FR-098-AC-25")]
#[test]
fn tc_444_a_key_selecting_no_claim_of_the_function_refuses() {
    let source = unit();
    let compiled = kernel_spine(&source, &BTreeMap::new());
    let x = |function| parameter(&compiled, function, 0);
    let r_application = record(&compiled, &source, "r", "10 mod x", 0);
    let no_node = OccurrenceKey::new(
        crate::fixture_node(0x11),
        Origin::new(Role::new("expression"), 0),
    );
    // The narrow that wraps `inc`'s `x + 1` is located with the application
    // but holds no record.
    let graph = compiled.package.graph();
    let inc_application = record(&compiled, &source, "inc", "x + 1", 0);
    let narrows: Vec<OccurrenceKey> = graph
        .occurrences()
        .filter(|(_, origin, location)| {
            origin.role() == &Role::new("expression")
                && graph.region(location).is_some_and(|region| {
                    let (start, end) = (region.start() as usize, region.end() as usize);
                    source.get(start..end) == Some("x + 1")
                        && source[..start].ends_with("Int[0, 10] pure { ")
                })
        })
        .map(|(node, origin, _)| OccurrenceKey::new(node.clone(), origin))
        .filter(|key| *key != inc_application && graph.requirements().get(key).is_none())
        .collect();
    assert!(
        !narrows.is_empty(),
        "inc's narrow holds an occurrence and no record"
    );
    for narrow in narrows {
        let refused = run(&source, &compiled, "inc", &narrow, &[(x("inc"), int(0))]).unwrap_err();
        assert!(
            matches!(refused, ReplayRefusal::ClaimOccurrence { .. }),
            "{narrow:?}: {refused:?}"
        );
    }
    let cases = [
        ("p", r_application.clone()),
        ("p", declaration(&compiled, "r")),
        ("p", no_node),
    ];
    for (function, key) in cases {
        let mut limited = at(
            &source,
            &compiled,
            function,
            &key,
            ReplaySource::Input(vec![CanonicalAssignment {
                parameter: x(function),
                value: int(0),
            }]),
        );
        // No charge is possible before a call: even no budget refuses alike.
        limited.accounting_limits = ScalarLimits {
            work_units: 0,
            ..UNLIMITED
        };
        let refused = replay(limited, ReplayLimits::default()).unwrap_err();
        let ReplayRefusal::ClaimOccurrence {
            key: named,
            function: selected,
        } = &refused
        else {
            panic!("expected ClaimOccurrence for {function} at {key:?}, got {refused:?}");
        };
        assert_eq!(named, &key);
        assert_eq!(selected, &name(&[function]));
        assert_eq!(refused.code(), Code::MissingDeclaration);
        assert_eq!(refused.cause(), Some("missing-name"));
    }
    let r_declaration = declaration(&compiled, "r");
    let refused = run(&source, &compiled, "r", &r_declaration, &[(x("r"), int(0))]).unwrap_err();
    assert!(matches!(refused, ReplayRefusal::NotAPredicate { .. }));
    let refused = run(
        &source,
        &compiled,
        "r",
        &r_application,
        &[(x("r"), int(12))],
    )
    .unwrap_err();
    assert_eq!(
        refused.input_refusal(),
        Some(&InputRefusal::WrongValueKind { parameter: 0 }),
        "{refused:?}"
    );
}

/// FR-098-AC-26: the refusal order. An unknown
/// function precedes `ClaimOccurrence`, which precedes the argument join.
#[trace("TC-444", "FR-098-AC-26")]
#[test]
fn tc_444_claim_selection_comes_after_the_function_and_before_the_join() {
    let source = unit();
    let compiled = kernel_spine(&source, &BTreeMap::new());
    let x = parameter(&compiled, "p", 0);
    let r_application = record(&compiled, &source, "r", "10 mod x", 0);
    let build = || {
        at(
            &source,
            &compiled,
            "p",
            &r_application,
            ReplaySource::Input(vec![CanonicalAssignment {
                parameter: x.clone(),
                value: int(0),
            }]),
        )
    };
    let mut unknown = build();
    unknown.replay_target =
        crate::RequestTargetWire::function(name(&["nope"]), r_application.clone());
    assert!(matches!(
        replay(unknown, ReplayLimits::default()).unwrap_err(),
        ReplayRefusal::UnknownFunction { .. }
    ));
    let mut stray = build();
    stray.source = ReplaySource::Input(vec![
        CanonicalAssignment {
            parameter: x,
            value: int(0),
        },
        CanonicalAssignment {
            parameter: crate::fixture_node(0x22),
            value: int(0),
        },
    ]);
    assert!(matches!(
        replay(stray, ReplayLimits::default()).unwrap_err(),
        ReplayRefusal::ClaimOccurrence { .. }
    ));
}

/// FR-098-AC-24, the unread-parameter rule per type: a `Boolean` fills
/// `false`, a range without 0 fills its end nearest 0 (observably), and a parameter of any
/// other type stays unbound and refuses `UnboundParameter`.
#[trace("TC-444", "FR-098-AC-24")]
#[test]
fn tc_444_unread_parameters_fill_by_type() {
    let source = unit();
    let compiled = kernel_spine(&source, &BTreeMap::new());
    // Each body also reads the unread parameter so that only the correct
    // fill is undefined there: the claim site (`10 mod x`) is defined at
    // `x = 1`, and the later undefined step is another location, so only the
    // correct fill (`false`, 3, -2) settles `NoValue`. Any other in-range
    // fill completes and settles `Verdicts(true)`.
    for function in ["ub", "ur", "un"] {
        let x = parameter(&compiled, function, 0);
        let key = record(&compiled, &source, function, "10 mod x", 0);
        assert_eq!(
            settled(run(&source, &compiled, function, &key, &[(x, int(1))])),
            Settled::NoValue,
            "{function}"
        );
    }
    let x = parameter(&compiled, "us", 0);
    let key = record(&compiled, &source, "us", "10 mod x", 0);
    let refused = run(&source, &compiled, "us", &key, &[(x, int(0))]).unwrap_err();
    assert!(
        matches!(refused, ReplayRefusal::UnboundParameter(_)),
        "{refused:?}"
    );
}

/// FR-098 "Application claim: the observation", recursion: `Location` has no
/// frame, so a stop at the claim site inside a recursive call cannot be told
/// from the outer evaluation. At `x = 1` the outer `10 mod 1` is defined and
/// the nested `f(0)` stops at the same site: the claim holds at `x = 1`, so
/// replay must not reproduce a violation. It settles `NoValue`.
#[trace("TC-444", "FR-098-AC-22")]
#[test]
fn tc_444_a_stop_inside_a_recursive_call_is_not_the_outer_observation() {
    let source = unit();
    let compiled = kernel_spine(&source, &BTreeMap::new());
    let x = parameter(&compiled, "f", 0);
    let key = record(&compiled, &source, "f", "10 mod x", 0);
    assert_eq!(
        settled(run(&source, &compiled, "f", &key, &[(x, int(1))])),
        Settled::NoValue
    );
}

/// The same hole through mutual recursion: `g` calls `h` calls `g`, and the
/// nested `g(0)` stops at `g`'s claim site. At `x = 2` the outer `10 mod 2`
/// is defined, so the claim holds and replay settles `NoValue`.
#[trace("TC-444", "FR-098-AC-22")]
#[test]
fn tc_444_a_stop_inside_a_mutually_recursive_call_is_not_the_outer_observation() {
    let source = unit();
    let compiled = kernel_spine(&source, &BTreeMap::new());
    let x = parameter(&compiled, "g", 0);
    let key = record(&compiled, &source, "g", "10 mod x", 0);
    assert_eq!(
        settled(run(&source, &compiled, "g", &key, &[(x, int(2))])),
        Settled::NoValue
    );
}

/// FR-098-AC-34: replay keeps the check mode recorded with the package. The
/// `Linked` half is asserted here: a possibly-zero `mod` yields no `Linked`
/// package to replay, and a `Linked` claim recompiles and settles under
/// `Linked`. The recorded-`Kernel` half is covered by the TC-1784 tests below.
#[trace("TC-1784", "FR-098-AC-34")]
#[test]
fn tc_1784_replay_never_rechecks_a_claim_under_another_mode() {
    let header = format!("language \"ix:native\" edition \"1-draft\";\n{PROFILE}");
    let unguarded =
        format!("{header}function r using v(x: Int[0, 9]): Integer pure {{ 10 mod x }}\n");
    let refused = compose(
        SourceIdentity::new(AUTHORITY, IDENTITY),
        IDENTITY,
        unguarded.as_bytes(),
        &BTreeMap::new(),
        &crate::spine::DependencyInput::default(),
        SpineLimits::default(),
    );
    assert!(
        refused.is_err(),
        "a Linked compile refuses `10 mod x` unproved-nonzero and yields no claim"
    );
    let source = format!(
        "{header}function g using v(x: Int[0, 9], y: Int[1, 9]): Integer pure {{ x mod y }}\n"
    );
    let compiled = spine(&source, &BTreeMap::new());
    let key = record(&compiled, &source, "g", "x mod y", 0);
    let x = parameter(&compiled, "g", 0);
    let y = parameter(&compiled, "g", 1);
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "g",
            &key,
            &[(x, int(3)), (y, int(2))]
        )),
        Settled::Verdicts(boolean(true))
    );
}

/// FR-098-AC-27: in the recursive `d`, only the selected call's own stop at
/// its `10 mod (n - 1)` site reproduces; a nested stop at the same
/// `Location` is no observation.
#[trace("TC-444", "FR-098-AC-27")]
#[test]
#[ignore = "QSL-2002: S6a reports no frame_depth yet"]
fn tc_444_a_recursive_site_reproduces_only_at_frame_depth_zero() {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n{PROFILE}\
         function d using v(n: Int[0, 9]): Integer pure decreases(n) \
         {{ if n = 0 then 0 else d(n - 1) + 10 mod (n - 1) }}\n"
    );
    let compiled = kernel_spine(&source, &BTreeMap::new());
    let n = parameter(&compiled, "d", 0);
    let key = record(&compiled, &source, "d", "10 mod (n - 1)", 0);
    let at = |value| {
        settled(run(
            &source,
            &compiled,
            "d",
            &key,
            &[(n.clone(), int(value))],
        ))
    };
    assert_eq!(at(1), Settled::Reproduced(boolean(false)));
    assert_eq!(at(2), Settled::NoValue);
    assert_eq!(at(3), Settled::NoValue);
    assert_eq!(at(0), Settled::Verdicts(boolean(true)));
}

/// `unit()`'s `r`, composed with every function body checked under `Kernel`
/// and emitted: the package records `kernel` and the replay request carries it.
fn kernel_spine(
    source: &str,
    modes: &BTreeMap<String, filament_semantic_value::checking::CheckMode>,
) -> ComposedUnit {
    crate::spine::compose_in_mode(
        SourceIdentity::new(AUTHORITY, IDENTITY),
        IDENTITY,
        source.as_bytes(),
        &BTreeMap::new(),
        &crate::spine::DependencyInput::default(),
        SpineLimits::default(),
        crate::spine::ComposeModes {
            check_mode: Some(filament_semantic_value::checking::CheckMode::Kernel),
            function_modes: modes,
        },
    )
    .expect("the unit compiles under Kernel")
}

/// FR-098-AC-34, FR-952-AC-2: a unit checked under `Kernel` records `kernel`
/// on `r`, and a request at `r`'s `10 mod x` record with `x = 0` recompiles
/// under that recorded mode (never `Linked`, which refuses
/// `unproved-nonzero`) and settles `reproduced-without-witness`.
#[trace("TC-1784", "FR-098-AC-34")]
#[trace("TC-2662", "FR-952-AC-2")]
#[test]
fn tc_1784_a_kernel_package_replays_under_the_recorded_kernel_mode() {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n{PROFILE}\
         function r using v(x: Int[0, 9]): Integer pure {{ 10 mod x }}\n"
    );
    let compiled = kernel_spine(&source, &BTreeMap::new());
    let package = String::from_utf8(compiled.emitted.bytes().to_vec()).expect("JSON");
    assert!(package.contains(r#""check_mode":"kernel""#), "{package}");
    let x = parameter(&compiled, "r", 0);
    let key = record(&compiled, &source, "r", "10 mod x", 0);
    let result = settled(run(&source, &compiled, "r", &key, &[(x, int(0))]));
    assert!(matches!(result, Settled::Reproduced(_)), "{result:?}");
}

/// `r` needs `Kernel` (its divisor may be zero); `g` is provable under
/// `Linked`.
fn mixed_source() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n{PROFILE}\
         function r using v(x: Int[0, 9]): Integer pure {{ 10 mod x }}\n\
         function g using v(x: Int[0, 9], y: Int[1, 9]): Integer pure {{ x mod y }}\n"
    )
}

/// FR-098-AC-34: a package whose body `r` records `kernel` and whose body
/// `g` records `linked` replays each under its own recorded mode: `r` is
/// reproduced under `Kernel` at `x = 0`, and `g` under `Linked`.
#[trace("TC-1784", "FR-098-AC-34")]
#[test]
fn tc_1784_a_mixed_package_replays_each_body_under_its_own_mode() {
    use filament_semantic_value::checking::CheckMode;
    let source = mixed_source();
    let modes = BTreeMap::from([("g".to_owned(), CheckMode::Linked)]);
    let compiled = kernel_spine(&source, &modes);
    let package = String::from_utf8(compiled.emitted.bytes().to_vec()).expect("JSON");
    assert_eq!(package.matches(r#""check_mode":"kernel""#).count(), 1);
    assert_eq!(package.matches(r#""check_mode":"linked""#).count(), 1);
    let x = parameter(&compiled, "r", 0);
    let key = record(&compiled, &source, "r", "10 mod x", 0);
    let result = settled(run(&source, &compiled, "r", &key, &[(x, int(0))]));
    assert!(matches!(result, Settled::Reproduced(_)), "{result:?}");
    let x = parameter(&compiled, "g", 0);
    let y = parameter(&compiled, "g", 1);
    let key = record(&compiled, &source, "g", "x mod y", 0);
    assert_eq!(
        settled(run(
            &source,
            &compiled,
            "g",
            &key,
            &[(x, int(3)), (y, int(2))]
        )),
        Settled::Verdicts(boolean(true))
    );
}

/// FR-098-AC-34: a forged package that flips `r`'s recorded mode to
/// `linked` is refused: `r` is rechecked under `Linked` and its unproved
/// divisor refuses, whatever the other function records.
#[trace("TC-1784", "FR-098-AC-34")]
#[test]
fn tc_1784_a_forged_recorded_mode_is_refused() {
    use filament_semantic_value::checking::CheckMode;
    let source = mixed_source();
    let modes = BTreeMap::from([("g".to_owned(), CheckMode::Linked)]);
    let compiled = kernel_spine(&source, &modes);
    let forged = String::from_utf8(compiled.emitted.bytes().to_vec())
        .expect("JSON")
        .replace(r#""check_mode":"kernel""#, r#""check_mode":"linked""#);
    let x = parameter(&compiled, "r", 0);
    let key = record(&compiled, &source, "r", "10 mod x", 0);
    let mut wire = at(
        &source,
        &compiled,
        "r",
        &key,
        ReplaySource::Input(vec![CanonicalAssignment {
            parameter: x,
            value: int(0),
        }]),
    );
    wire.package = crate::ReplayPackage::read(forged.as_bytes(), 1 << 30).expect("JSON");
    let refusal = replay(wire, ReplayLimits::default()).expect_err("a forged mode is refused");
    assert!(format!("{refusal:?}").contains("Nonzero"), "{refusal:?}");
}

/// FR-098-AC-34: a function node whose recorded `check_mode` differs from its
/// recompile's is refused `PackageMember` at that node's `check_mode`, cause
/// `check-mode-mismatch`; matching records pass. The recorded bytes are a
/// real package with `g`'s recorded `linked` edited to `kernel`, compared with
/// the recompile's own bytes.
#[trace("TC-1784", "FR-098-AC-34")]
#[test]
fn tc_1784_a_recorded_mode_that_differs_from_the_recompile_is_refused() {
    use filament_semantic_value::checking::CheckMode;
    let source = mixed_source();
    let modes = BTreeMap::from([("g".to_owned(), CheckMode::Linked)]);
    let compiled = kernel_spine(&source, &modes);
    let original = compiled.emitted.bytes();
    check_recorded_modes(original, original).expect("matching modes pass");
    let edited = String::from_utf8(original.to_vec())
        .expect("JSON")
        .replacen(r#""check_mode":"linked""#, r#""check_mode":"kernel""#, 1);
    let refusal = check_recorded_modes(edited.as_bytes(), original)
        .expect_err("g records kernel but recompiles linked");
    let ReplayRefusal::Request(crate::ReplayRequestRefusal::PackageMember { pointer, cause }) =
        refusal
    else {
        panic!("a package-member refusal: {refusal:?}");
    };
    assert_eq!(cause, crate::PackageMemberCause::CheckModeMismatch);
    let document: serde_json::Value = serde_json::from_slice(original).expect("JSON");
    let linked = document["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .position(|node| node["check_mode"] == "linked")
        .expect("g records linked");
    assert_eq!(
        pointer,
        format!("/package/semantic_graph/nodes/{linked}/check_mode")
    );
}
