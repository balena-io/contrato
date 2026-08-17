//! Template interpolation for contract string values.
//!
//! Supports `{{this.property.path}}` substitution in any string value within
//! a contract JSON structure. Paths are resolved against the root contract.
//! Unresolvable paths are left as-is. A blacklist mechanism allows skipping
//! interpolation for specific field paths (e.g., `children`).

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

use crate::path::DottedPath;

/// Regex matching `{{...}}` template expressions with non-greedy capture.
static TEMPLATE_REGEXP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{(.+?)\}\}").expect("template regex is valid"));

/// Interpolates template expressions in a single string value.
///
/// Replaces all `{{this.path.to.field}}` expressions with the resolved value
/// from the root contract. Non-string resolved values (objects, arrays, numbers,
/// booleans, null) are JSON-stringified into the string. Only `this`-prefixed
/// paths are processed; other prefixes are left unchanged. Unresolvable paths
/// leave the original `{{...}}` expression in place.
///
/// # Arguments
/// * `s` - The string to interpolate
/// * `root` - The root contract value to resolve paths against
fn interpolate_string(s: &str, root: &Value) -> String {
    TEMPLATE_REGEXP
        .replace_all(s, |caps: &regex::Captures| {
            let full_match = &caps[0];
            let path_str = &caps[1];

            let mut segments = path_str.split('.');
            if segments.next() != Some("this") {
                return full_match.to_string();
            }

            let remaining: Vec<&str> = segments.collect();
            if remaining.is_empty() {
                return full_match.to_string();
            }

            let path = DottedPath::from_parts(&remaining);
            match path.resolve(root) {
                Some(Value::String(s)) => s.clone(),
                Some(other) => other.to_string(),
                None => full_match.to_string(),
            }
        })
        .into_owned()
}

/// Checks whether a field path is blacklisted.
///
/// When the blacklist contains multi-level paths (containing `.`), the full
/// dotted breadcrumb path is checked with prefix matching. Otherwise, only the
/// first path segment is checked via O(1) hash lookup.
///
/// # Arguments
/// * `breadcrumb` - Current path segments in the traversal
/// * `blacklist` - Set of path prefixes to skip
/// * `is_multi_level` - Whether any blacklist entry contains a `.`
fn is_blacklisted(
    breadcrumb: &[String],
    blacklist: &HashSet<String>,
    is_multi_level: bool,
) -> bool {
    if blacklist.is_empty() {
        return false;
    }

    if is_multi_level {
        let path = breadcrumb.join(".");
        blacklist
            .iter()
            .any(|entry| path.starts_with(entry.as_str()))
    } else {
        match breadcrumb.first() {
            Some(k) => blacklist.contains(k),
            None => false,
        }
    }
}

/// Recursively compiles template expressions in a contract JSON value.
///
/// Traverses the JSON structure depth-first, interpolating `{{this.path}}`
/// expressions in string values against the root contract. Objects are
/// recursed into, arrays have each element compiled as a sub-contract
/// (with the current contract as root for resolution), and non-string
/// leaf values pass through unchanged.
///
/// # Arguments
/// * `value` - The JSON value to compile (typically a contract object)
/// * `blacklist` - Set of path prefixes to skip during interpolation
/// * `root` - The root contract for path resolution (defaults to `value` itself)
///
/// # Blacklist behavior
/// - Single-level entries (e.g., `"name"`) match against the first path segment
/// - Multi-level entries (e.g., `"data.foo.type"`) match against the full dotted path
/// - Matching uses `starts_with`, so `"data"` blocks the entire `data` subtree
pub(crate) fn compile_contract(
    value: &Value,
    blacklist: &HashSet<String>,
    root: Option<&Value>,
) -> Value {
    let root = root.unwrap_or(value);
    let is_multi_level = blacklist.iter().any(|p| p.contains('.'));
    let mut breadcrumb = Vec::new();

    compile_inner(value, blacklist, root, &mut breadcrumb, is_multi_level)
}

/// Inner recursive implementation of contract compilation.
///
/// Separated from `compile_contract` to avoid recomputing `is_multi_level`
/// on every recursive call. Uses a single `Vec<String>` breadcrumb that is
/// pushed/popped during traversal to avoid cloning the entire path per node.
fn compile_inner(
    value: &Value,
    blacklist: &HashSet<String>,
    root: &Value,
    breadcrumb: &mut Vec<String>,
    is_multi_level: bool,
) -> Value {
    match value {
        Value::Object(map) => {
            let mut result = serde_json::Map::with_capacity(map.len());
            for (key, val) in map {
                breadcrumb.push(key.clone());
                let compiled = if val.is_object() {
                    compile_inner(val, blacklist, root, breadcrumb, is_multi_level)
                } else {
                    compile_leaf(val, blacklist, root, breadcrumb, is_multi_level)
                };
                result.insert(key.clone(), compiled);
                breadcrumb.pop();
            }
            Value::Object(result)
        }
        _ => compile_leaf(value, blacklist, root, breadcrumb, is_multi_level),
    }
}

/// Compiles a leaf (non-object) JSON value.
///
/// Handles string interpolation and array recursion. Strings are interpolated
/// unless blacklisted. Arrays have each element recursively compiled as a
/// sub-contract with the current value's parent as root. Other values pass
/// through unchanged.
fn compile_leaf(
    value: &Value,
    blacklist: &HashSet<String>,
    root: &Value,
    breadcrumb: &mut Vec<String>,
    is_multi_level: bool,
) -> Value {
    match value {
        Value::String(s) => {
            if is_blacklisted(breadcrumb, blacklist, is_multi_level) {
                return value.clone();
            }
            Value::String(interpolate_string(s, root))
        }
        Value::Array(arr) => {
            let compiled: Vec<Value> = arr
                .iter()
                .enumerate()
                .map(|(i, elem)| {
                    breadcrumb.push(i.to_string());
                    let result = if elem.is_object() {
                        compile_inner(elem, blacklist, root, breadcrumb, is_multi_level)
                    } else {
                        compile_leaf(elem, blacklist, root, breadcrumb, is_multi_level)
                    };
                    breadcrumb.pop();
                    result
                })
                .collect();
            Value::Array(compiled)
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn compile_contract_without_templates() {
        let contract = json!({
            "type": "distro",
            "name": "Debian",
            "version": "wheezy",
            "slug": "debian"
        });

        let result = compile_contract(&contract, &HashSet::new(), None);

        assert_eq!(
            result,
            json!({
                "type": "distro",
                "name": "Debian",
                "version": "wheezy",
                "slug": "debian"
            })
        );
    }

    #[test]
    fn compile_single_top_level_template() {
        let contract = json!({
            "type": "distro",
            "name": "Debian {{this.version}}",
            "version": "wheezy",
            "slug": "debian"
        });

        let result = compile_contract(&contract, &HashSet::new(), None);

        assert_eq!(
            result,
            json!({
                "type": "distro",
                "name": "Debian wheezy",
                "version": "wheezy",
                "slug": "debian"
            })
        );
    }

    #[test]
    fn compile_templates_inside_arrays() {
        let contract = json!({
            "type": "distro",
            "name": "Debian",
            "slug": "debian",
            "random": ["{{this.name}}", "{{this.name}}", "{{this.name}}"],
            "requires": [
                {
                    "name": "{{this.name}} ({{this.type}})"
                }
            ]
        });

        let result = compile_contract(&contract, &HashSet::new(), None);

        assert_eq!(
            result,
            json!({
                "type": "distro",
                "name": "Debian",
                "slug": "debian",
                "random": ["Debian", "Debian", "Debian"],
                "requires": [
                    {
                        "name": "Debian (distro)"
                    }
                ]
            })
        );
    }

    #[test]
    fn compile_multiple_top_level_templates() {
        let contract = json!({
            "type": "distro",
            "name": "Debian {{this.version}}",
            "version": "wheezy",
            "slug": "debian-{{this.version}}"
        });

        let result = compile_contract(&contract, &HashSet::new(), None);

        assert_eq!(
            result,
            json!({
                "type": "distro",
                "name": "Debian wheezy",
                "version": "wheezy",
                "slug": "debian-wheezy"
            })
        );
    }

    #[test]
    fn compile_single_nested_template() {
        let contract = json!({
            "type": "distro",
            "name": "Debian",
            "version": "wheezy",
            "slug": "debian",
            "data": {
                "foo": {
                    "bar": {
                        "baz": "{{this.type}}"
                    }
                }
            }
        });

        let result = compile_contract(&contract, &HashSet::new(), None);

        assert_eq!(
            result,
            json!({
                "type": "distro",
                "name": "Debian",
                "version": "wheezy",
                "slug": "debian",
                "data": {
                    "foo": {
                        "bar": {
                            "baz": "distro"
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn leave_missing_values_as_interpolations() {
        let contract = json!({
            "type": "distro",
            "name": "Debian",
            "version": "{{this.data.distroName}}",
            "slug": "debian"
        });

        let result = compile_contract(&contract, &HashSet::new(), None);

        assert_eq!(
            result,
            json!({
                "type": "distro",
                "name": "Debian",
                "version": "{{this.data.distroName}}",
                "slug": "debian"
            })
        );
    }

    #[test]
    fn blacklist_top_level_element() {
        let blacklist: HashSet<String> = ["name"].iter().map(|s| s.to_string()).collect();

        let contract = json!({
            "type": "distro",
            "version": "7",
            "name": "Debian v{{this.version}}",
            "data": {
                "name": "debian"
            },
            "slug": "{{this.data.name}}"
        });

        let result = compile_contract(&contract, &blacklist, None);

        assert_eq!(
            result,
            json!({
                "type": "distro",
                "version": "7",
                "name": "Debian v{{this.version}}",
                "data": {
                    "name": "debian"
                },
                "slug": "debian"
            })
        );
    }

    #[test]
    fn blacklist_nested_element() {
        let blacklist: HashSet<String> = ["data.foo.type"].iter().map(|s| s.to_string()).collect();

        let contract = json!({
            "type": "distro",
            "version": "7",
            "name": "Debian v{{this.version}}",
            "data": {
                "name": "debian",
                "foo": {
                    "type": "{{this.type}}"
                }
            },
            "slug": "{{this.data.name}}"
        });

        let result = compile_contract(&contract, &blacklist, None);

        assert_eq!(
            result,
            json!({
                "type": "distro",
                "version": "7",
                "name": "Debian v7",
                "data": {
                    "name": "debian",
                    "foo": {
                        "type": "{{this.type}}"
                    }
                },
                "slug": "debian"
            })
        );
    }

    #[test]
    fn blacklist_multiple_elements() {
        let blacklist: HashSet<String> = ["data.foo.type", "name"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let contract = json!({
            "type": "distro",
            "version": "7",
            "name": "Debian v{{this.version}}",
            "data": {
                "name": "debian",
                "foo": {
                    "type": "{{this.type}}"
                }
            },
            "slug": "{{this.data.name}}"
        });

        let result = compile_contract(&contract, &blacklist, None);

        assert_eq!(
            result,
            json!({
                "type": "distro",
                "version": "7",
                "name": "Debian v{{this.version}}",
                "data": {
                    "name": "debian",
                    "foo": {
                        "type": "{{this.type}}"
                    }
                },
                "slug": "debian"
            })
        );
    }

    #[test]
    fn blacklist_elements_inside_arrays() {
        let blacklist: HashSet<String> = ["random.1"].iter().map(|s| s.to_string()).collect();

        let contract = json!({
            "slug": "debian",
            "type": "distro",
            "random": ["{{this.slug}}", "{{this.slug}}", "{{this.slug}}"]
        });

        let result = compile_contract(&contract, &blacklist, None);

        assert_eq!(
            result,
            json!({
                "slug": "debian",
                "type": "distro",
                "random": ["debian", "{{this.slug}}", "debian"]
            })
        );
    }

    #[test]
    fn blacklist_whole_subtree() {
        let blacklist: HashSet<String> = ["data"].iter().map(|s| s.to_string()).collect();

        let contract = json!({
            "type": "distro",
            "version": "7",
            "name": "Debian v{{this.version}}",
            "data": {
                "name": "debian",
                "foo": {
                    "type": "{{this.type}}"
                }
            },
            "slug": "{{this.data.name}}"
        });

        let result = compile_contract(&contract, &blacklist, None);

        assert_eq!(
            result,
            json!({
                "type": "distro",
                "version": "7",
                "name": "Debian v7",
                "data": {
                    "name": "debian",
                    "foo": {
                        "type": "{{this.type}}"
                    }
                },
                "slug": "debian"
            })
        );
    }

    #[test]
    fn non_this_prefix_left_unchanged() {
        let contract = json!({
            "type": "distro",
            "name": "{{foo.bar}}",
            "slug": "debian"
        });

        let result = compile_contract(&contract, &HashSet::new(), None);

        assert_eq!(
            result,
            json!({
                "type": "distro",
                "name": "{{foo.bar}}",
                "slug": "debian"
            })
        );
    }

    #[test]
    fn numeric_value_interpolation() {
        let contract = json!({
            "type": "distro",
            "name": "Version {{this.data.major}}",
            "slug": "debian",
            "data": {
                "major": 7
            }
        });

        let result = compile_contract(&contract, &HashSet::new(), None);

        assert_eq!(
            result,
            json!({
                "type": "distro",
                "name": "Version 7",
                "slug": "debian",
                "data": {
                    "major": 7
                }
            })
        );
    }

    #[test]
    fn stringify_object_when_referenced_value_is_object() {
        let contract = json!({
            "type": "distro",
            "name": "{{this.data}}",
            "slug": "debian",
            "data": {
                "arch": "amd64"
            }
        });

        let result = compile_contract(&contract, &HashSet::new(), None);

        assert_eq!(
            result,
            json!({
                "type": "distro",
                "name": "{\"arch\":\"amd64\"}",
                "slug": "debian",
                "data": {
                    "arch": "amd64"
                }
            })
        );
    }

    #[test]
    fn stringify_array_when_referenced_value_is_array() {
        let contract = json!({
            "type": "distro",
            "name": "{{this.data.items}}",
            "slug": "debian",
            "data": {
                "items": ["a", "b", "c"]
            }
        });

        let result = compile_contract(&contract, &HashSet::new(), None);

        assert_eq!(
            result,
            json!({
                "type": "distro",
                "name": "[\"a\",\"b\",\"c\"]",
                "slug": "debian",
                "data": {
                    "items": ["a", "b", "c"]
                }
            })
        );
    }

    #[test]
    fn null_value_interpolates() {
        let contract = json!({
            "type": "distro",
            "name": "{{this.data.value}}",
            "slug": "debian",
            "data": { "value": null }
        });

        let result = compile_contract(&contract, &HashSet::new(), None);
        assert_eq!(result["name"], "null");
    }

    #[test]
    fn false_value_interpolates() {
        let contract = json!({
            "type": "distro",
            "name": "{{this.data.value}}",
            "slug": "debian",
            "data": { "value": false }
        });

        let result = compile_contract(&contract, &HashSet::new(), None);
        assert_eq!(result["name"], "false");
    }

    #[test]
    fn zero_value_interpolates() {
        let contract = json!({
            "type": "distro",
            "name": "{{this.data.value}}",
            "slug": "debian",
            "data": { "value": 0 }
        });

        let result = compile_contract(&contract, &HashSet::new(), None);
        assert_eq!(result["name"], "0");
    }

    #[test]
    fn empty_string_value_interpolates() {
        let contract = json!({
            "type": "distro",
            "name": "{{this.data.value}}",
            "slug": "debian",
            "data": { "value": "" }
        });

        let result = compile_contract(&contract, &HashSet::new(), None);
        assert_eq!(result["name"], "");
    }

    #[test]
    fn true_value_interpolates() {
        let contract = json!({
            "type": "distro",
            "name": "{{this.data.value}}",
            "slug": "debian",
            "data": { "value": true }
        });

        let result = compile_contract(&contract, &HashSet::new(), None);
        assert_eq!(result["name"], "true");
    }

    #[test]
    fn nonzero_number_interpolates() {
        let contract = json!({
            "type": "distro",
            "name": "{{this.data.value}}",
            "slug": "debian",
            "data": { "value": 42 }
        });

        let result = compile_contract(&contract, &HashSet::new(), None);
        assert_eq!(result["name"], "42");
    }
}
