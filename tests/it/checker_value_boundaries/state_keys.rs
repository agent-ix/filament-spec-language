// SPDX-License-Identifier: AGPL-3.0-or-later
//! The deep expected fragment is built from the published typed-form grammar.
use super::{
    observations::{carrier, fields},
    HEADER,
};
use fsl_eval::simulation::{
    model::{
        declaration_identity_text, ExpansionLimits, ModelCheckInput, ModelSimulationRequest,
        ModelSystem, PopulationUniverse,
    },
    TransitionSystem,
};
use fsl_foundation::SourceIdentity;
use fsl_replay::spine::DependencyInput;
use fsl_semantics::model::{
    accounting::ModelNormalizationLimits,
    intake::package_input,
    observation::{DocumentRef, ObservationLimits},
};
use ix_trace_rs::trace;
use serde_json::json;
use std::collections::BTreeMap;

#[test]
#[trace("FR-120", "FR-262")]
#[ignore = "QSL-2704: public recursive-state journey stack overflow; preserve strict independent key oracle"]
fn deep_record_state_keys_preserve_each_link_and_the_absent_terminal() {
    std::thread::Builder::new().stack_size(512 * 1024).spawn(|| {
        let integer = "ix://filament/native/Integer";
        let link = carrier::declaration("Link","money",json!({"fields":[
            fields::field("Link","head",integer),fields::optional("Link","tail",&carrier::id("Link"))]}));
        let cargo = carrier::declaration("Cargo","thing",json!({"identityFields":[carrier::id("Cargo/id")],
            "fields":[fields::field("Cargo","id",integer),fields::optional("Cargo","chain",&carrier::id("Link"))],"operations":[]}));
        let mut document = fields::package(vec![link,cargo],true);
        document["populations"] = json!([{"identity":carrier::id("cargo"),"displayName":"cargo",
            "kind":{"module":carrier::BUSINESS,"name":"population"},"members":[carrier::id("Cargo")],
            "extent":"closed","origin":carrier::origin("cargo")}]);
        let bytes = serde_json::to_vec(&document).unwrap();
        let packages = package_input([bytes.as_slice()]).unwrap();
        let source = format!("{HEADER}model M = {:?}; function marker using v(): Boolean pure {{ true }}",carrier::PACKAGE);
        let checked = crate::support::front_end::checked(SourceIdentity::new("boundary-tests","original:deep-state-key"),"key.native",source.as_bytes(),&packages,&DependencyInput::default()).unwrap();
        // This key case stays inside the published default byte budgets.
        // The separate evaluator case constructs 100,000 links.
        const DEPTH: usize = 1_000;
        let wire_prefix = format!(r#"{{"present":{{"record":{{"type":{},"fields":{{"head":{{"integer":"7"}},"tail":"#,json!(carrier::id("Link")));
        let chain = format!("{}{{\"absent\":{{}}}}{}",wire_prefix.repeat(DEPTH),"}}}}".repeat(DEPTH));
        let snapshot = |label:&str,chain:&str| {
            let document = json!({"format":"filament.state.snapshot/v1","identity":{"authority":"boundary-tests","identity":label},
                "model":{"identity":carrier::PACKAGE},"observation":"current","anchor":{"kind":"initialization","name":"seed"},
                "populations":[{"population":carrier::id("cargo"),"complete":true,"objects":[{"key":"box","type":carrier::id("Cargo"),
                    "fields":{"id":{"integer":"17"},"chain":"CHAIN"}}]}]});
            document.to_string().replace("\"CHAIN\"",chain).into_bytes()
        };
        let reference = |name:&str| DocumentRef {authority:"boundary-tests".into(),identity:name.into()};
        let shallow = reference("original:empty-chain");
        let deep = reference("original:deep-chain");
        let snapshots = BTreeMap::from([(shallow.clone(),snapshot(&shallow.identity,r#"{"absent":{}}"#)),(deep.clone(),snapshot(&deep.identity,&chain))]);
        let system = ModelSystem::new(ModelSimulationRequest {input:ModelCheckInput::source(checked.package()),packages:&packages,
            normalization:ModelNormalizationLimits::default(),snapshots:&snapshots,initial:vec![shallow,deep],
            universes:vec![PopulationUniverse {population:filament_checked::key::DeclarationKey {package:carrier::PACKAGE.into(),node:carrier::id("cargo")},keys:vec!["box".into()]}],
            observation:ObservationLimits::default(),expansion:ExpansionLimits {max_candidates:1},
            meter:fsl_replay::spine::default_accounting(1_000_000),emit:fsl_package::EmitLimits::new(1_048_576)}).expect("both real initial states admit");
        let states = system.initial();
        assert_eq!(states.len(),2,"a nonempty deep chain must not coalesce with absence");
        let encode = |state| quire_canonical::to_vec(&system.key(state),quire_canonical::Limits::new(16_777_216)).unwrap();
        let shallow_key = String::from_utf8(encode(&states[0])).unwrap();
        let deep_key = String::from_utf8(encode(&states[1])).unwrap();
        let name = declaration_identity_text(&filament_checked::key::DeclarationKey {package:carrier::PACKAGE.into(),node:carrier::id("Link")}).unwrap();
        let prefix = r#"{"fields":[{"name":"head","value":{"type":"integer","value":"7"}},{"name":"tail","value":"#;
        let suffix = format!(r#"}}],"name":{},"type":"record"}}"#,json!(name));
        let expected_chain = format!("{}null{}",prefix.repeat(DEPTH),suffix.repeat(DEPTH));
        let slot = r#"{"name":"chain","value":null}"#;
        assert_eq!(shallow_key.matches(slot).count(),1,"one original field is the metamorphic boundary");
        let expected = shallow_key.replace(slot,&format!(r#"{{"name":"chain","value":{expected_chain}}}"#));
        assert_eq!(deep_key,expected,"every nested head/tail and the final absence must be encoded in declaration order");
    }).unwrap().join().unwrap();
}
