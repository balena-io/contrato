//! Variant expansion for contracts.
//!
//! Contract variants are syntax sugar that allows expressing multiple different
//! contracts sharing many properties as a single object, to avoid repetition.
//! A contract with a `variants` array is expanded into N contracts, one per
//! variant, where each variant is merged with the base contract.

use std::collections::HashMap;

use serde_json::Value;

use crate::children_tree::{self, ChildrenTree};
use crate::types::{Asset, PartialContract, RawContract};

/// Expands a contract's variants into a flat list of contracts.
///
/// If the contract has no `variants` (or an empty array), returns a
/// single-element vec containing the contract with `variants` removed.
/// Otherwise, each variant is recursively expanded and merged with the base
/// contract. See [`merge_partial`] for the per-field merge rules.
pub(crate) fn build(contract: RawContract) -> Vec<RawContract> {
    let RawContract {
        kind,
        canonical_slug,
        body,
        extra,
    } = contract;

    let mut bodies = expand_partial(body);
    let last = bodies.pop();

    let mut result: Vec<RawContract> = bodies
        .into_iter()
        .map(|body| RawContract {
            kind: kind.clone(),
            canonical_slug: canonical_slug.clone(),
            body,
            extra: extra.clone(),
        })
        .collect();

    if let Some(body) = last {
        result.push(RawContract {
            kind,
            canonical_slug,
            body,
            extra,
        });
    }
    result
}

/// Recursive variant expansion over [`PartialContract`].
///
/// A contract without variants expands to itself. Otherwise each variant is
/// recursively expanded and every expansion is merged onto the contract, which
/// acts as the base
fn expand_partial(mut partial: PartialContract) -> Vec<PartialContract> {
    if partial.variants.is_empty() {
        return vec![partial];
    }

    let variants = std::mem::take(&mut partial.variants);
    let base = partial;

    let mut templates: Vec<PartialContract> =
        variants.into_iter().flat_map(expand_partial).collect();
    let last = templates.pop();

    let mut expanded: Vec<PartialContract> = templates
        .into_iter()
        .map(|template| merge_partial(base.clone(), template))
        .collect();

    if let Some(template) = last {
        expanded.push(merge_partial(base, template));
    }
    expanded
}

/// Merges a variant (`overlay`) onto a base contract body.
///
/// - `slug`, `version`, `name`, `description`: the overlay wins when set.
/// - `aliases`, `requires`: concatenated, base first.
/// - `data`: recursively [`deep_merge`]d when both sides are set, otherwise the
///   overlay wins when set.
/// - `assets`: merged key-wise; colliding keys are merged field-wise by
///   [`merge_asset`].
/// - `children`: flattened and concatenated by [`merge_children`].
/// - `variants`: dropped — a merged contract is by definition already expanded.
fn merge_partial(base: PartialContract, overlay: PartialContract) -> PartialContract {
    PartialContract {
        slug: overlay.slug.or(base.slug),
        version: overlay.version.or(base.version),
        name: overlay.name.or(base.name),
        description: overlay.description.or(base.description),
        aliases: concat(base.aliases, overlay.aliases),
        data: match (base.data, overlay.data) {
            (Some(b), Some(o)) => Some(deep_merge(&b, &o)),
            (b, o) => o.or(b),
        },
        assets: merge_assets(base.assets, overlay.assets),
        requires: concat(base.requires, overlay.requires),
        variants: Vec::new(),
        children: merge_children(base.children, overlay.children),
    }
}

/// Appends the overlay's elements to the base's, reusing the base's allocation.
fn concat<T>(mut base: Vec<T>, overlay: Vec<T>) -> Vec<T> {
    base.extend(overlay);
    base
}

/// Merges two asset maps key-wise, merging colliding entries field-wise.
fn merge_assets(
    mut base: HashMap<String, Asset>,
    overlay: HashMap<String, Asset>,
) -> HashMap<String, Asset> {
    for (key, asset) in overlay {
        let merged = match base.remove(&key) {
            Some(existing) => merge_asset(existing, asset),
            None => asset,
        };
        base.insert(key, merged);
    }
    base
}

/// Merges two assets field-wise.
///
/// `url` is required on both sides, so the overlay's always wins. The optional
/// fields fall back to the base when the overlay leaves them unset, and the
/// untyped `extra` fields are deep-merged like `data`.
fn merge_asset(base: Asset, overlay: Asset) -> Asset {
    let mut extra = base.extra;
    for (key, val) in overlay.extra {
        let merged = match extra.get(&key) {
            Some(existing) => deep_merge(existing, &val),
            None => val,
        };
        extra.insert(key, merged);
    }

    Asset {
        url: overlay.url,
        name: overlay.name.or(base.name),
        checksum: overlay.checksum.or(base.checksum),
        checksum_type: overlay.checksum_type.or(base.checksum_type),
        extra,
    }
}

/// Merges two children trees by flattening both sides and concatenating them,
/// base children first.
///
/// Children declare the capabilities a contract provides to its context, so the
/// merge must be a union: a structural tree merge would collapse two different
/// capabilities sharing a type path into a single hybrid contract.
fn merge_children(
    base: Option<ChildrenTree>,
    overlay: Option<ChildrenTree>,
) -> Option<ChildrenTree> {
    match (base, overlay) {
        (None, None) => None,
        (base, overlay) => Some(ChildrenTree::Multiple(
            base.into_iter()
                .chain(overlay)
                .flat_map(children_tree::into_all)
                .collect(),
        )),
    }
}

/// Deep-merges two JSON values with array concatenation semantics.
///
/// Only used for the untyped `data` field; every other field has a typed rule
/// in [`merge_partial`].
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

        let result = build(contract);
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

        let result = build(contract);
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

        let result = build(contract);
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

        let result = build(contract);
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

        let result = build(contract);
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

        let result = build(contract);
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

        let result = build(contract);
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

        let result = build(contract);
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

        let result = build(contract);
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
    fn build_preserves_unknown_asset_fields() {
        // Contracts attach their own metadata to assets and reference it from
        // templated fields, so unknown keys must survive expansion.
        let contract: RawContract = serde_json::from_value(json!({
            "slug": "fedora",
            "type": "sw.os",
            "assets": {
                "test": {
                    "main": "test-os",
                    "commit": "a95300e",
                    "url": "https://example.com/{{this.assets.test.commit}}/script.sh"
                }
            },
            "variants": [
                { "assets": { "test": { "commit": "b12f4c1", "url": "https://example.com/{{this.assets.test.commit}}/script.sh" } } }
            ]
        }))
        .unwrap();

        let result = build(contract);
        assert_eq!(result.len(), 1);

        let json = serde_json::to_value(&result[0]).unwrap();
        assert_eq!(
            json["assets"],
            json!({
                "test": {
                    "main": "test-os",
                    "commit": "b12f4c1",
                    "url": "https://example.com/{{this.assets.test.commit}}/script.sh"
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
                        children: None,
                        variants: vec![],
                    },
                ],
            },
            extra: Map::new(),
        };

        let result = build(contract);
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
        let contract: RawContract = serde_json::from_value(json!({
            "slug": "myapp",
            "type": "sw.app",
            "name": "My App",
            "variants": [
                { "type": "sw.service", "name": "My Service" }
            ]
        }))
        .unwrap();

        let result = build(contract);
        assert_eq!(result.len(), 1);

        let json = serde_json::to_value(&result[0]).unwrap();
        // Base type is preserved — variant cannot override it
        assert_eq!(json["type"], "sw.app");
        assert_eq!(json["name"], "My Service");
        assert_eq!(json["slug"], "myapp");
    }

    /// Expands a contract expected to yield exactly one result and returns its
    /// children as `type/slug` strings, in order.
    ///
    /// The assertions are on the flattened set of children, not on tree shape:
    /// the shape emitted by the merge is an intermediate that `Contract::new`
    /// regenerates from its child index.
    fn expand_child_ids(contract: Value) -> Vec<String> {
        let contract: RawContract = serde_json::from_value(contract).unwrap();
        let result = build(contract);
        assert_eq!(result.len(), 1);

        let children = result[0].body.children.as_ref().expect("children present");
        children_tree::get_all(children)
            .iter()
            .map(|c| format!("{}/{}", c.kind, c.body.slug.as_ref().unwrap()))
            .collect()
    }

    #[test]
    fn build_concatenates_children_lists() {
        assert_eq!(
            expand_child_ids(json!({
                "slug": "debian",
                "type": "sw.os",
                "children": [{ "type": "sw.feature", "slug": "secureboot" }],
                "variants": [
                    { "children": [{ "type": "arch.sw", "slug": "amd64" }] }
                ]
            })),
            ["sw.feature/secureboot", "arch.sw/amd64"]
        );
    }

    #[test]
    fn build_merges_disjoint_children_trees() {
        assert_eq!(
            expand_child_ids(json!({
                "slug": "debian",
                "type": "sw.os",
                "children": {
                    "sw": { "feature": { "type": "sw.feature", "slug": "secureboot" } }
                },
                "variants": [
                    {
                        "children": {
                            "arch": { "sw": { "type": "arch.sw", "slug": "amd64" } }
                        }
                    }
                ]
            })),
            ["sw.feature/secureboot", "arch.sw/amd64"]
        );
    }

    #[test]
    fn build_keeps_children_of_same_type_with_different_slugs() {
        // Regression guard: a structural tree merge collapses these two into a
        // single hybrid contract, destroying the base's capability.
        assert_eq!(
            expand_child_ids(json!({
                "slug": "myapp",
                "type": "sw.application",
                "children": {
                    "sw": { "os": { "type": "sw.os", "slug": "debian", "version": "wheezy" } }
                },
                "variants": [
                    {
                        "children": {
                            "sw": { "os": { "type": "sw.os", "slug": "fedora", "version": "38" } }
                        }
                    }
                ]
            })),
            ["sw.os/debian", "sw.os/fedora"]
        );
    }

    #[test]
    fn build_merges_children_list_with_tree() {
        assert_eq!(
            expand_child_ids(json!({
                "slug": "myapp",
                "type": "sw.application",
                "children": [
                    { "type": "sw.feature", "slug": "secureboot" },
                    { "type": "sw.feature", "slug": "tpm" }
                ],
                "variants": [
                    {
                        "children": {
                            "arch": { "sw": { "type": "arch.sw", "slug": "amd64" } }
                        }
                    }
                ]
            })),
            ["sw.feature/secureboot", "sw.feature/tpm", "arch.sw/amd64"]
        );
    }

    #[test]
    fn build_keeps_base_only_children() {
        assert_eq!(
            expand_child_ids(json!({
                "slug": "debian",
                "type": "sw.os",
                "children": [{ "type": "sw.feature", "slug": "secureboot" }],
                "variants": [{ "version": "wheezy" }]
            })),
            ["sw.feature/secureboot"]
        );
    }

    #[test]
    fn build_keeps_variant_only_children() {
        assert_eq!(
            expand_child_ids(json!({
                "slug": "debian",
                "type": "sw.os",
                "variants": [
                    { "children": [{ "type": "arch.sw", "slug": "amd64" }] }
                ]
            })),
            ["arch.sw/amd64"]
        );
    }

    #[test]
    fn build_merges_colliding_assets_field_wise() {
        let contract: RawContract = serde_json::from_value(json!({
            "slug": "firmware",
            "type": "sw.blob",
            "assets": {
                "binary": {
                    "url": "https://example.com/base.bin",
                    "name": "Base binary",
                    "checksum": "abc123",
                    "checksumType": "sha256"
                },
                "docs": { "url": "https://example.com/docs.pdf" }
            },
            "variants": [
                {
                    "assets": {
                        "binary": {
                            "url": "https://example.com/variant.bin",
                            "checksum": "def456"
                        },
                        "extra": { "url": "https://example.com/extra.bin" }
                    }
                }
            ]
        }))
        .unwrap();

        let result = build(contract);
        assert_eq!(result.len(), 1);

        let json = serde_json::to_value(&result[0]).unwrap();
        assert_eq!(
            json["assets"],
            json!({
                "binary": {
                    // `url` is required, so the overlay's always wins; the
                    // optional fields the overlay leaves unset fall back.
                    "url": "https://example.com/variant.bin",
                    "name": "Base binary",
                    "checksum": "def456",
                    "checksumType": "sha256"
                },
                "docs": { "url": "https://example.com/docs.pdf" },
                "extra": { "url": "https://example.com/extra.bin" }
            })
        );
    }
}
