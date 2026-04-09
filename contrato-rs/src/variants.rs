//! Variant expansion for contracts.
//!
//! Contract variants are syntax sugar that allows expressing multiple different
//! contracts sharing many properties as a single object, to avoid repetition.
//! A contract with a `variants` array is expanded into N contracts, one per
//! variant, where each variant is deep-merged with the base contract.

use serde_json::Value;

use crate::types::RawContract;

/// Expands a contract's variants into a flat list of contracts.
///
/// If the contract has no `variants` (or an empty array), returns a
/// single-element vec containing the contract with `variants` removed.
/// Otherwise, each variant is recursively expanded and deep-merged with
/// the base contract. During merge, arrays are concatenated (base first,
/// then variant) and objects are recursively merged.
///
/// # Panics
///
/// Panics if the contract cannot be serialized to JSON or if any expanded
/// variant cannot be deserialized back into a [`ContractObject`].
pub fn build(contract: &RawContract) -> Vec<RawContract> {
    let value = serde_json::to_value(contract).expect("ContractObject must serialize to JSON");
    build_value(&value)
        .into_iter()
        .map(|v| serde_json::from_value(v).expect("expanded variant must deserialize"))
        .collect()
}

/// Recursive variant expansion operating on raw JSON values.
///
/// Extracts the `variants` array from the object, removes it from the base,
/// then for each variant recursively expands it and deep-merges each result
/// with the base.
fn build_value(contract: &Value) -> Vec<Value> {
    // Non-object values pass through unchanged. This handles the recursive case
    // where a malformed variant entry is not an object.
    let obj = match contract.as_object() {
        Some(o) => o,
        None => return vec![contract.clone()],
    };

    let mut base = obj.clone();
    let variants = base.remove("variants");
    let base = Value::Object(base);

    let variants = match variants.as_ref().and_then(Value::as_array) {
        Some(v) if !v.is_empty() => v,
        _ => return vec![base],
    };

    variants
        .iter()
        .flat_map(|variation| {
            build_value(variation)
                .into_iter()
                .map(|template| deep_merge(&base, &template))
        })
        .collect()
}

/// Deep-merges two JSON values with array concatenation semantics.
///
/// - **Objects**: keys from `overlay` are merged into `base` recursively.
///   Keys present only in `base` are preserved; keys present only in
///   `overlay` are added; keys in both are recursively merged.
/// - **Arrays**: `base` elements followed by `overlay` elements (concatenation).
/// - **Scalars**: `overlay` replaces `base`.
fn deep_merge(base: &Value, overlay: &Value) -> Value {
    match (base, overlay) {
        (Value::Object(b), Value::Object(o)) => {
            let mut result = b.clone();
            for (key, val) in o {
                let merged = match result.get(key) {
                    Some(existing) => deep_merge(existing, val),
                    None => val.clone(),
                };
                result.insert(key.clone(), merged);
            }
            Value::Object(result)
        }
        (Value::Array(b), Value::Array(o)) => {
            let mut result = b.clone();
            result.extend(o.iter().cloned());
            Value::Array(result)
        }
        (_, overlay) => overlay.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_contract_with_no_variants() {
        let contract: RawContract = serde_json::from_value(json!({
            "slug": "debian",
            "type": "distro",
            "name": "Debian"
        }))
        .unwrap();

        let result = build(&contract);
        assert_eq!(result.len(), 1);

        let json = serde_json::to_value(&result[0]).unwrap();
        assert_eq!(
            json,
            json!({
                "slug": "debian",
                "type": "distro",
                "name": "Debian"
            })
        );
    }

    #[test]
    fn build_contract_with_empty_variants() {
        let contract: RawContract = serde_json::from_value(json!({
            "slug": "debian",
            "type": "distro",
            "name": "Debian",
            "variants": []
        }))
        .unwrap();

        let result = build(&contract);
        assert_eq!(result.len(), 1);

        let json = serde_json::to_value(&result[0]).unwrap();
        assert_eq!(
            json,
            json!({
                "slug": "debian",
                "type": "distro",
                "name": "Debian"
            })
        );
    }

    #[test]
    fn build_contract_with_two_variants() {
        let contract: RawContract = serde_json::from_value(json!({
            "slug": "nodejs_{{data.arch}}",
            "type": "blob",
            "name": "Node.js",
            "data": { "libc": "musl-libc" },
            "variants": [
                {
                    "data": { "arch": "amd64" },
                    "requires": [{ "type": "arch.sw", "slug": "amd64" }]
                },
                {
                    "data": { "arch": "i386" },
                    "requires": [{ "type": "arch.sw", "slug": "i386" }]
                }
            ]
        }))
        .unwrap();

        let result = build(&contract);
        assert_eq!(result.len(), 2);

        let jsons: Vec<Value> = result
            .iter()
            .map(|c| serde_json::to_value(c).unwrap())
            .collect();

        assert_eq!(
            jsons[0],
            json!({
                "slug": "nodejs_{{data.arch}}",
                "type": "blob",
                "name": "Node.js",
                "requires": [{ "type": "arch.sw", "slug": "amd64" }],
                "data": { "arch": "amd64", "libc": "musl-libc" }
            })
        );

        assert_eq!(
            jsons[1],
            json!({
                "slug": "nodejs_{{data.arch}}",
                "type": "blob",
                "name": "Node.js",
                "requires": [{ "type": "arch.sw", "slug": "i386" }],
                "data": { "arch": "i386", "libc": "musl-libc" }
            })
        );
    }

    #[test]
    fn build_nested_variants() {
        let contract: RawContract = serde_json::from_value(json!({
            "slug": "nodejs_{{data.arch}}",
            "type": "blob",
            "name": "Node.js",
            "data": { "libc": "musl-libc" },
            "variants": [
                {
                    "data": { "arch": "amd64" },
                    "requires": [{ "type": "arch.sw", "slug": "amd64" }],
                    "variants": [
                        { "version": "6.3.0" },
                        { "version": "6.4.0" }
                    ]
                },
                {
                    "data": { "arch": "i386" },
                    "requires": [{ "type": "arch.sw", "slug": "i386" }],
                    "variants": [
                        { "version": "6.3.0" }
                    ]
                }
            ]
        }))
        .unwrap();

        let result = build(&contract);
        assert_eq!(result.len(), 3);

        let jsons: Vec<Value> = result
            .iter()
            .map(|c| serde_json::to_value(c).unwrap())
            .collect();

        assert_eq!(
            jsons[0],
            json!({
                "slug": "nodejs_{{data.arch}}",
                "type": "blob",
                "version": "6.3.0",
                "name": "Node.js",
                "requires": [{ "type": "arch.sw", "slug": "amd64" }],
                "data": { "arch": "amd64", "libc": "musl-libc" }
            })
        );

        assert_eq!(
            jsons[1],
            json!({
                "slug": "nodejs_{{data.arch}}",
                "type": "blob",
                "version": "6.4.0",
                "name": "Node.js",
                "requires": [{ "type": "arch.sw", "slug": "amd64" }],
                "data": { "arch": "amd64", "libc": "musl-libc" }
            })
        );

        assert_eq!(
            jsons[2],
            json!({
                "slug": "nodejs_{{data.arch}}",
                "type": "blob",
                "version": "6.3.0",
                "name": "Node.js",
                "requires": [{ "type": "arch.sw", "slug": "i386" }],
                "data": { "arch": "i386", "libc": "musl-libc" }
            })
        );
    }

    #[test]
    fn build_merges_arrays_correctly() {
        let contract: RawContract = serde_json::from_value(json!({
            "slug": "foo",
            "type": "blob",
            "name": "Foo",
            "requires": [{ "type": "bar", "slug": "baz" }],
            "variants": [
                { "requires": [{ "type": "arch.sw", "slug": "amd64" }] },
                { "requires": [{ "type": "arch.sw", "slug": "i386" }] }
            ]
        }))
        .unwrap();

        let result = build(&contract);
        assert_eq!(result.len(), 2);

        let jsons: Vec<Value> = result
            .iter()
            .map(|c| serde_json::to_value(c).unwrap())
            .collect();

        assert_eq!(
            jsons[0],
            json!({
                "slug": "foo",
                "type": "blob",
                "name": "Foo",
                "requires": [
                    { "type": "bar", "slug": "baz" },
                    { "type": "arch.sw", "slug": "amd64" }
                ]
            })
        );

        assert_eq!(
            jsons[1],
            json!({
                "slug": "foo",
                "type": "blob",
                "name": "Foo",
                "requires": [
                    { "type": "bar", "slug": "baz" },
                    { "type": "arch.sw", "slug": "i386" }
                ]
            })
        );
    }

    #[test]
    fn build_variant_overrides_base_scalar() {
        let contract: RawContract = serde_json::from_value(json!({
            "slug": "myapp",
            "type": "sw.app",
            "name": "Original Name",
            "description": "Original description",
            "variants": [
                { "name": "Variant Name", "description": "Variant description" }
            ]
        }))
        .unwrap();

        let result = build(&contract);
        assert_eq!(result.len(), 1);

        let json = serde_json::to_value(&result[0]).unwrap();
        assert_eq!(
            json,
            json!({
                "slug": "myapp",
                "type": "sw.app",
                "name": "Variant Name",
                "description": "Variant description"
            })
        );
    }

    #[test]
    fn build_multiple_array_fields_concatenate() {
        let contract: RawContract = serde_json::from_value(json!({
            "slug": "myapp",
            "type": "sw.app",
            "aliases": ["alias-base"],
            "requires": [{ "type": "hw.device-type", "slug": "rpi3" }],
            "variants": [
                {
                    "aliases": ["alias-variant"],
                    "requires": [{ "type": "arch.sw", "slug": "amd64" }]
                }
            ]
        }))
        .unwrap();

        let result = build(&contract);
        assert_eq!(result.len(), 1);

        let json = serde_json::to_value(&result[0]).unwrap();
        assert_eq!(
            json,
            json!({
                "slug": "myapp",
                "type": "sw.app",
                "aliases": ["alias-base", "alias-variant"],
                "requires": [
                    { "type": "hw.device-type", "slug": "rpi3" },
                    { "type": "arch.sw", "slug": "amd64" }
                ]
            })
        );
    }

    #[test]
    fn build_preserves_extra_fields() {
        let contract: RawContract = serde_json::from_value(json!({
            "slug": "myapp",
            "type": "sw.app",
            "customField": "preserved",
            "anotherExtra": 42,
            "variants": [
                { "data": { "arch": "amd64" } }
            ]
        }))
        .unwrap();

        let result = build(&contract);
        assert_eq!(result.len(), 1);

        let json = serde_json::to_value(&result[0]).unwrap();
        assert_eq!(
            json,
            json!({
                "slug": "myapp",
                "type": "sw.app",
                "customField": "preserved",
                "anotherExtra": 42,
                "data": { "arch": "amd64" }
            })
        );
    }

    #[test]
    fn build_deeply_nested_data_merge() {
        let contract: RawContract = serde_json::from_value(json!({
            "slug": "myapp",
            "type": "sw.app",
            "data": {
                "config": {
                    "timeout": 30,
                    "retry": { "enabled": true, "count": 3 }
                }
            },
            "variants": [
                {
                    "data": {
                        "config": {
                            "debug": true,
                            "retry": { "count": 5, "backoff": "exponential" }
                        }
                    }
                }
            ]
        }))
        .unwrap();

        let result = build(&contract);
        assert_eq!(result.len(), 1);

        let json = serde_json::to_value(&result[0]).unwrap();
        assert_eq!(
            json,
            json!({
                "slug": "myapp",
                "type": "sw.app",
                "data": {
                    "config": {
                        "timeout": 30,
                        "debug": true,
                        "retry": { "enabled": true, "count": 5, "backoff": "exponential" }
                    }
                }
            })
        );
    }

    #[test]
    fn deep_merge_scalars_overlay_wins() {
        let base = json!({"a": 1, "b": 2});
        let overlay = json!({"a": 10, "c": 3});
        let result = deep_merge(&base, &overlay);
        assert_eq!(result, json!({"a": 10, "b": 2, "c": 3}));
    }

    #[test]
    fn deep_merge_nested_objects() {
        let base = json!({"data": {"x": 1, "y": 2}});
        let overlay = json!({"data": {"y": 20, "z": 30}});
        let result = deep_merge(&base, &overlay);
        assert_eq!(result, json!({"data": {"x": 1, "y": 20, "z": 30}}));
    }

    #[test]
    fn deep_merge_arrays_concatenate() {
        let base = json!({"items": [1, 2]});
        let overlay = json!({"items": [3, 4]});
        let result = deep_merge(&base, &overlay);
        assert_eq!(result, json!({"items": [1, 2, 3, 4]}));
    }

    #[test]
    fn deep_merge_mixed_types_overlay_wins() {
        let base = json!({"a": [1, 2]});
        let overlay = json!({"a": "replaced"});
        let result = deep_merge(&base, &overlay);
        assert_eq!(result, json!({"a": "replaced"}));
    }

    #[test]
    fn build_manually_constructed_contract() {
        use crate::types::{ContractType, PartialContract, Slug, Version};
        use serde_json::Map;

        let contract = RawContract {
            kind: ContractType::new("sw.app"),
            canonical_slug: None,
            body: PartialContract {
                slug: Some(Slug::new("myapp")),
                version: Some(Version::new("1.0.0")),
                name: Some("My App".into()),
                description: None,
                aliases: vec![Slug::new("app-alias")],
                data: Some(json!({"lang": "rust"})),
                assets: Default::default(),
                requires: vec![],
                provides: vec![],
                children: None,
                variants: vec![
                    PartialContract {
                        slug: None,
                        version: None,
                        name: None,
                        description: None,
                        aliases: vec![Slug::new("variant-alias")],
                        data: Some(json!({"arch": "amd64"})),
                        assets: Default::default(),
                        requires: vec![],
                        provides: vec![],
                        children: None,
                        variants: vec![],
                    },
                    PartialContract {
                        slug: None,
                        version: Some(Version::new("2.0.0")),
                        name: None,
                        description: None,
                        aliases: vec![],
                        data: Some(json!({"arch": "arm64"})),
                        assets: Default::default(),
                        requires: vec![],
                        provides: vec![],
                        children: None,
                        variants: vec![],
                    },
                ],
            },
            extra: Map::new(),
        };

        let result = build(&contract);
        assert_eq!(result.len(), 2);

        let jsons: Vec<Value> = result
            .iter()
            .map(|c| serde_json::to_value(c).unwrap())
            .collect();

        assert_eq!(
            jsons[0],
            json!({
                "type": "sw.app",
                "slug": "myapp",
                "version": "1.0.0",
                "name": "My App",
                "aliases": ["app-alias", "variant-alias"],
                "data": { "lang": "rust", "arch": "amd64" }
            })
        );

        assert_eq!(
            jsons[1],
            json!({
                "type": "sw.app",
                "slug": "myapp",
                "version": "2.0.0",
                "name": "My App",
                "aliases": ["app-alias"],
                "data": { "lang": "rust", "arch": "arm64" }
            })
        );
    }

    #[test]
    fn build_variant_type_field_is_dropped() {
        // Variants are deserialized as `PartialContract`, which has no `type`
        // field. A `type` in the variant JSON is silently dropped during
        // deserialization, so the base contract's type is always preserved.
        // This differs from the TS behavior where variants are plain objects.
        let contract: RawContract = serde_json::from_value(json!({
            "slug": "myapp",
            "type": "sw.app",
            "name": "My App",
            "variants": [
                { "type": "sw.service", "name": "My Service" }
            ]
        }))
        .unwrap();

        let result = build(&contract);
        assert_eq!(result.len(), 1);

        let json = serde_json::to_value(&result[0]).unwrap();
        // Base type is preserved — variant cannot override it
        assert_eq!(json["type"], "sw.app");
        assert_eq!(json["name"], "My Service");
        assert_eq!(json["slug"], "myapp");
    }
}
