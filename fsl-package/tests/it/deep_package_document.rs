// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-729 step 2 (FR-259-AC-3): the shared package-document reader returns
//! the same 100,000-deep JSON tree on small and ordinary thread stacks.

use fsl_foundation::IntakeLimits;
use fsl_semantics::model::intake::PackageDocument;
use ix_trace_rs::trace;
use serde_json::Value;

const DEPTH: usize = 100_000;

/// The package JSON document with `payload` nested under 100,000 arrays.
fn package_document() -> Vec<u8> {
    let mut text = String::with_capacity(2 * DEPTH + 64);
    text.push_str(r#"{"package":{"identity":"acme/orders","version":"1"},"payload":"#);
    text.push_str(&"[".repeat(DEPTH));
    text.push('0');
    text.push_str(&"]".repeat(DEPTH));
    text.push('}');
    text.into_bytes()
}

fn on_stack<T: Send + 'static>(stack_size: usize, run: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(stack_size)
        .spawn(run)
        .expect("the reader thread spawns")
        .join()
        .expect("the package document read does not overflow its stack")
}

/// Read the same bytes on a thread with a caller-selected stack.
fn read_package_document(stack_size: usize, bytes: Vec<u8>) -> PackageDocument {
    on_stack(stack_size, move || {
        PackageDocument::parse(&bytes, IntakeLimits::default().with_input_bytes(u64::MAX))
            .expect("the nested package JSON is read")
    })
}

/// Compare JSON trees without recursively descending their arrays or objects.
fn assert_json_trees_equal(left: &Value, right: &Value) {
    let mut pending = vec![(left, right)];
    while let Some((left, right)) = pending.pop() {
        match (left, right) {
            (Value::Array(left), Value::Array(right)) => {
                assert_eq!(left.len(), right.len());
                pending.extend(left.iter().zip(right));
            }
            (Value::Object(left), Value::Object(right)) => {
                assert_eq!(left.len(), right.len());
                for (key, left_value) in left {
                    let right_value = right.get(key).expect("both trees contain each member");
                    pending.push((left_value, right_value));
                }
            }
            (left, right) => assert_eq!(left, right),
        }
    }
}

/// Confirm the package identity and every level of the nested payload.
fn assert_document_shape(document: &PackageDocument) {
    let tree = document.tree();
    assert_eq!(tree["package"]["identity"], "acme/orders");
    assert_eq!(tree["package"]["version"], "1");
    let mut value = &tree["payload"];
    for _ in 0..DEPTH {
        let array = value.as_array().expect("each payload level is an array");
        assert_eq!(array.len(), 1);
        value = &array[0];
    }
    assert_eq!(value.as_i64(), Some(0));
}

/// FR-259-AC-3: the shared reader admits the 100,000-deep package JSON on a
/// 512 KiB stack, with the same parsed tree as an 8 MiB stack.
#[trace("TC-729", "FR-259-AC-3")]
#[test]
fn a_100_000_deep_package_document_reads_equally_on_small_and_large_stacks() {
    let bytes = package_document();
    let small_stack = read_package_document(512 * 1024, bytes.clone());
    let large_stack = read_package_document(8 * 1024 * 1024, bytes);

    assert_document_shape(&small_stack);
    assert_document_shape(&large_stack);
    assert_json_trees_equal(small_stack.tree(), large_stack.tree());
}

/// TC-729 step 5 candidate for FR-259-AC-8: malformed package JSON preserves
/// the shared reader's byte offset and carries no limit payload. This is not
/// tagged with AC-8 because the public read API has no way to inject its
/// separate 4096-byte allocation failure.
#[trace("TC-729")]
#[test]
fn malformed_package_json_reports_its_offset_without_a_limit_payload() {
    let bytes = b"{\"package\":";
    let refusal = PackageDocument::parse(bytes, IntakeLimits::default())
        .expect_err("an incomplete JSON object is malformed");

    assert_eq!(
        refusal.code,
        fsl_foundation::diagnostic::Code::InvalidModelBinding
    );
    assert_eq!(refusal.cause.as_str(), "malformed-declaration");
    assert!(refusal.detail.contains("byte 11:"), "{}", refusal.detail);
    assert_eq!(refusal.cause.catalog_fields(), None);
}
