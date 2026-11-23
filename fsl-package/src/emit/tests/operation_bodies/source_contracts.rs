// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original operation bodies recovered through the strict checked-package reader.

use super::*;
use filament_contract_model::{CheckedBindingNameKind, CheckedLiteralValue, CheckedTerm};

fn admitted(
    document: &[u8],
    emission: &Emission,
) -> std::sync::Arc<filament_contract_model::CheckedPackageV2> {
    let mut evidence = read_evidence(emission);
    evidence
        .insert_domain_package_document(package_identity(document), document)
        .expect("the selected original input is evidence");
    match read_with(emission, &evidence) {
        Read::Verified { admitted, .. } => admitted,
        other => panic!("strict reader must admit the emitted operation body: {other:?}"),
    }
}

#[trace("TC-1011", "FR-576-AC-1", "FR-576-AC-2", "FR-576-AC-9")]
#[test]
fn strict_body_lookup_recovers_called_integer_and_uncalled_boolean_values() {
    for (name, type_ref, source_expression, expected) in [
        (
            "score",
            "ix://filament/native/Integer",
            "43",
            CheckedLiteralValue::Integer(filament_exact::Integer::from(43_i64)),
        ),
        (
            "ready",
            "ix://filament/native/Boolean",
            "false",
            CheckedLiteralValue::Boolean(false),
        ),
    ] {
        let mut op = query("ConfigVersion", name, None);
        op["returns"]["typeRef"] = json!(type_ref);
        let document = package(vec![op, query("ConfigVersion", "probe", None)], &[]);
        let mut source = format!(
            "body authored using v on Config::ConfigVersion::{name}() {{ {source_expression} }}\n"
        );
        if name == "score" {
            source.push_str(
                "pre invokesScore using v on Config::ConfigVersion::probe { self.score() > 0 }\n",
            );
        }
        let emission = emitted(&document, &source);
        let checked = admitted(&document, &emission);
        let row = checked
            .operation_body_rows()
            .expect("body rows retained")
            .iter()
            .find(|row| row.operation.as_ref() == name)
            .expect("operation row");
        let OperationBody::Function { node: function } = &row.body else {
            panic!("body must not disappear");
        };
        let graph = checked.resolved_graph();
        let function = graph
            .nodes
            .iter()
            .find(|node| &node.node_id == function)
            .expect("function lookup");
        assert_eq!(function.node_tag.as_ref(), "function");
        let CheckedTerm::Aggregate { members, .. } = &function.body else {
            panic!("function aggregate");
        };
        let expression = members
            .iter()
            .find_map(|member| match member {
                CheckedTerm::Binding { name, value, .. }
                    if matches!(name.kind(), CheckedBindingNameKind::Body) =>
                {
                    Some(value.as_ref())
                }
                _ => None,
            })
            .expect("body member");
        let CheckedTerm::Reference { target } = expression else {
            panic!("body expression reference");
        };
        let expression = graph
            .nodes
            .iter()
            .find(|node| &node.node_id == target)
            .expect("expression lookup");
        assert_eq!(
            expression
                .body
                .literal_read()
                .expect("authored literal")
                .value,
            expected
        );
        let text = unit(&source, &["Config"]);
        let expected_start = text.rfind(source_expression).expect("authored expression");
        assert!(
            checked
                .source_map()
                .iter()
                .filter(|entry| entry.node_id == expression.node_id)
                .flat_map(|entry| &entry.regions)
                .any(|region| region.source.authority.as_ref() == "agent-ix"
                    && region.source.identity.as_ref() == "test:spine-model"
                    && usize::try_from(region.start).expect("source start") == expected_start
                    && usize::try_from(region.end).expect("source end")
                        == expected_start + source_expression.len()),
            "the recovered literal must retain its exact authored source identity and interval"
        );
    }
}

#[trace("TC-1011", "FR-576-AC-3", "FR-576-AC-10")]
#[test]
fn a_bodyless_void_operation_has_explicit_absence_beside_a_real_body() {
    let score = query("ConfigVersion", "score", None);
    let mut touch = query("ConfigVersion", "touch", None);
    touch.as_object_mut().expect("operation").remove("returns");
    let document = package(vec![score, touch], &[]);
    let emission = emitted(
        &document,
        "body authored using v on Config::ConfigVersion::score() { 43 }\n",
    );
    let checked = admitted(&document, &emission);
    let rows = checked.operation_body_rows().expect("operation body rows");
    let touches: Vec<_> = rows
        .iter()
        .filter(|row| row.operation.as_ref() == "touch")
        .collect();
    assert_eq!(touches.len(), 1, "exactly one explicit void-operation row");
    let touch = touches[0];
    assert_eq!(touch.body, OperationBody::NoBody);
    let score = rows
        .iter()
        .find(|row| row.operation.as_ref() == "score")
        .expect("value operation");
    assert!(matches!(score.body, OperationBody::Function { .. }));
    let graph = checked.resolved_graph();
    assert!(
        !graph.nodes.iter().any(|node| match &node.owner {
            Some(filament_contract_model::CheckedNodeOwner::Model { node, .. }) =>
                node.as_ref() == owner("ConfigVersion/touch"),
            _ => false,
        }),
        "no synthetic void body node may appear"
    );
}

fn assert_invalid_body(source: &str) {
    let mut void = query("ConfigVersion", "touch", None);
    void.as_object_mut().expect("operation").remove("returns");
    let document = package(vec![query("ConfigVersion", "score", None), void], &[]);
    let text = unit(source, &["Config"]);
    let inputs = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("original model input");
    let mut declarations =
        assemble_with_models(text.as_bytes(), &inputs).expect("operation body forms assemble");
    let linked = declarations.link_dispatch(Default::default());
    let regions = declarations.regions();
    let checked = match linked {
        Ok(pending) => declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_semantics::check::test_support::ZeroRewardEvaluator,
            )
            .map_err(|refusals| pending.report(refusals)),
        Err(refusals) => Err(refusals),
    };
    let refusals = checked.expect_err("an unreachable invalid body must not reach emission");
    assert!(!refusals.is_empty());
    assert!(
        refusals
            .iter()
            .all(|refusal| refusal.cause.code().as_str() == "ill_typed"
                && refusal.cause.cause() == Some("type-mismatch")),
        "{refusals:?}"
    );
    let body_start = text.find("body ").expect("authored body declaration");
    for refusal in &refusals {
        let region = regions
            .refusal_region(refusal)
            .unwrap_or_else(|| panic!("located body refusal for {source:?}: {refusal:?}"));
        let start = usize::try_from(region.start()).expect("region start");
        let end = usize::try_from(region.end()).expect("region end");
        assert!(
            start >= body_start && end <= body_start + source.trim_end().len(),
            "refusal must name this body, not the model selection or whole unit: {refusal:?} at {}",
            &text[start..end]
        );
        assert!(start < end);
    }
}

#[trace("TC-1011", "FR-576-AC-2")]
#[test]
fn uncalled_wrong_type_body_refuses_at_its_body() {
    assert_invalid_body("body wrongValue using v on Config::ConfigVersion::score() { true }\n");
}

#[trace("TC-1011", "FR-576-AC-2")]
#[test]
fn uncalled_wrong_arity_body_refuses_at_its_body() {
    assert_invalid_body("body wrongArity using v on Config::ConfigVersion::score(extra) { 43 }\n");
}

#[trace("TC-1011", "FR-576-AC-10")]
#[test]
fn void_operation_body_refuses_at_its_body() {
    assert_invalid_body("body voidValue using v on Config::ConfigVersion::touch() { 43 }\n");
}

#[trace("TC-1011", "FR-576-AC-2", "FR-576-AC-9")]
#[test]
fn a_parameterized_conditional_body_keeps_its_branches_in_order() {
    use filament_contract_model::ApplicationOperator;
    let mut operation = query("ConfigVersion", "decideSize", None);
    operation["params"] = json!([{
        "identity": owner("ConfigVersion/decideSize/ready"), "name": "ready",
        "typeRef": "ix://filament/native/Boolean", "presence": "required",
        "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": operation["origin"].clone(),
    }]);
    let document = package(vec![operation], &[]);
    let emission = emitted(
        &document,
        "body chooser using v on Config::ConfigVersion::decideSize(ready) { if ready then 31 else 47 }\n",
    );
    let checked = admitted(&document, &emission);
    let row = checked
        .operation_body_rows()
        .expect("operation rows")
        .iter()
        .find(|row| row.operation.as_ref() == "decideSize")
        .expect("declared operation");
    let OperationBody::Function { node: id } = &row.body else {
        panic!("conditional body function");
    };
    let graph = checked.resolved_graph();
    let function = graph
        .nodes
        .iter()
        .find(|node| &node.node_id == id)
        .expect("body lookup");
    let CheckedTerm::Aggregate { members, .. } = &function.body else {
        panic!("function members");
    };
    let body = members
        .iter()
        .find_map(|member| match member {
            CheckedTerm::Binding { name, value, .. }
                if matches!(name.kind(), CheckedBindingNameKind::Body) =>
            {
                Some(value.as_ref())
            }
            _ => None,
        })
        .expect("authored function body");
    let CheckedTerm::Reference { target } = body else {
        panic!("body reference");
    };
    let body = graph
        .nodes
        .iter()
        .find(|node| &node.node_id == target)
        .expect("conditional expression");
    let CheckedTerm::Application {
        operator,
        arguments,
        ..
    } = &body.body
    else {
        panic!("a conditional cannot be replaced with a literal");
    };
    assert_eq!(*operator, ApplicationOperator::Conditional);
    assert_eq!(arguments.len(), 3, "condition, then, else");
    let parameters = members
        .iter()
        .find_map(|member| match member {
            CheckedTerm::Binding { name, value, .. }
                if matches!(name.kind(), CheckedBindingNameKind::Parameters) =>
            {
                Some(value.as_ref())
            }
            _ => None,
        })
        .expect("the body's declared parameters");
    let CheckedTerm::Aggregate {
        members: parameters,
        ..
    } = parameters
    else {
        panic!("ordered parameter references");
    };
    let ready: Vec<_> = parameters
        .iter()
        .filter_map(|parameter| {
            let CheckedTerm::Reference { target } = parameter else {
                panic!("parameter reference");
            };
            let parameter = graph
                .nodes
                .iter()
                .find(|node| &node.node_id == target)
                .expect("declared parameter");
            assert_eq!(parameter.semantic_form.as_ref(), "parameter");
            let CheckedTerm::Aggregate { members, .. } = &parameter.body else {
                panic!("parameter metadata");
            };
            let name = members
                .iter()
                .find_map(|member| match member {
                    CheckedTerm::Binding { name, value, .. }
                        if matches!(name.kind(), CheckedBindingNameKind::Other("name")) =>
                    {
                        Some(value.as_ref())
                    }
                    _ => None,
                })
                .expect("parameter name");
            match name.literal_read().expect("parameter name value").value {
                CheckedLiteralValue::Text("ready") => Some(target),
                _ => None,
            }
        })
        .collect();
    assert_eq!(ready.len(), 1, "one exact authored ready parameter");
    let CheckedTerm::Reference { target: condition } = &arguments[0] else {
        panic!("the condition reads a parameter, not an inline Boolean constant");
    };
    assert_eq!(
        condition, ready[0],
        "the condition must read this function's ready parameter"
    );
    for (argument, expected) in arguments[1..].iter().zip([31_i64, 47_i64]) {
        let CheckedTerm::Reference { target } = argument else {
            panic!("branch expression reference");
        };
        let branch = graph
            .nodes
            .iter()
            .find(|node| &node.node_id == target)
            .expect("branch expression");
        assert_eq!(
            branch.body.literal_read().expect("literal branch").value,
            CheckedLiteralValue::Integer(filament_exact::Integer::from(expected))
        );
    }
}
