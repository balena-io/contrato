//! Deterministic hashing of contract data.
//!
//! Produces a stable SHA-256 hex digest for any `serde_json::Value`. Keys are
//! sorted recursively so that property order does not affect the hash. This
//! replaces the TypeScript `object-hash` (SHA-1) — hashes from this module are
//! NOT compatible with the old TS hashes.

use sha2::{Digest, Sha256};

/// Computes a deterministic SHA-256 hex digest for a JSON value.
///
/// Objects are canonicalized by sorting keys recursively before hashing, so
/// `{"a":1,"b":2}` and `{"b":2,"a":1}` produce the same hash. Arrays preserve
/// their element order.
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use contrato::hash::hash_object;
///
/// let h1 = hash_object(&json!({"foo": "bar", "baz": 1}));
/// let h2 = hash_object(&json!({"baz": 1, "foo": "bar"}));
/// assert_eq!(h1, h2);
/// ```
pub fn hash_object(value: &serde_json::Value) -> String {
    let mut buf = String::new();
    write_canonical(value, &mut buf);
    let hash = Sha256::digest(buf.as_bytes());
    format!("{hash:x}")
}

/// Writes a canonical string representation of a JSON value.
///
/// The canonical form sorts object keys lexicographically and uses a
/// type-tagged encoding so that structurally different values (e.g., the
/// string `"1"` vs the number `1`) produce different hashes.
fn write_canonical(value: &serde_json::Value, buf: &mut String) {
    match value {
        serde_json::Value::Null => {
            buf.push_str("null:");
        }
        serde_json::Value::Bool(b) => {
            buf.push_str("bool:");
            buf.push_str(if *b { "true" } else { "false" });
        }
        serde_json::Value::Number(n) => {
            buf.push_str("number:");
            buf.push_str(&n.to_string());
        }
        serde_json::Value::String(s) => {
            buf.push_str("string:");
            buf.push_str(s);
        }
        serde_json::Value::Array(arr) => {
            buf.push_str("array:");
            buf.push_str(&arr.len().to_string());
            buf.push(':');
            for item in arr {
                write_canonical(item, buf);
            }
        }
        serde_json::Value::Object(obj) => {
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            buf.push_str("object:");
            buf.push_str(&keys.len().to_string());
            buf.push(':');
            for key in keys {
                buf.push_str("key:");
                buf.push_str(key);
                buf.push(':');
                write_canonical(&obj[key], buf);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn returns_a_string() {
        let hash = hash_object(&json!({"foo": "bar"}));
        assert!(!hash.is_empty());
        // SHA-256 hex is 64 characters
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn does_not_care_about_property_order() {
        let h1 = hash_object(&json!({"foo": "bar", "bar": "baz"}));
        let h2 = hash_object(&json!({"bar": "baz", "foo": "bar"}));
        assert_eq!(h1, h2);
    }

    #[test]
    fn identical_values_produce_identical_hashes() {
        let obj = json!({"foo": "bar"});
        let h1 = hash_object(&obj);
        let h2 = hash_object(&obj);
        let h3 = hash_object(&json!({"foo": "bar"}));
        assert_eq!(h1, h2);
        assert_eq!(h2, h3);
    }

    #[test]
    fn different_objects_produce_different_hashes() {
        let h1 = hash_object(&json!({"foo": "bar"}));
        let h2 = hash_object(&json!({"foo": "baz"}));
        let h3 = hash_object(&json!({"foo": "qux"}));
        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_ne!(h3, h1);
    }

    #[test]
    fn nested_object_order_independence() {
        let h1 = hash_object(&json!({
            "a": {"z": 1, "y": 2},
            "b": "hello"
        }));
        let h2 = hash_object(&json!({
            "b": "hello",
            "a": {"y": 2, "z": 1}
        }));
        assert_eq!(h1, h2);
    }

    #[test]
    fn array_order_matters() {
        let h1 = hash_object(&json!([1, 2, 3]));
        let h2 = hash_object(&json!([3, 2, 1]));
        assert_ne!(h1, h2);
    }

    #[test]
    fn distinguishes_types() {
        let h1 = hash_object(&json!("1"));
        let h2 = hash_object(&json!(1));
        assert_ne!(h1, h2);
    }

    #[test]
    fn distinguishes_bool_and_string() {
        let h1 = hash_object(&json!(true));
        let h2 = hash_object(&json!("true"));
        assert_ne!(h1, h2);
    }

    #[test]
    fn null_hashes_consistently() {
        let h1 = hash_object(&json!(null));
        let h2 = hash_object(&json!(null));
        assert_eq!(h1, h2);
    }

    #[test]
    fn hashes_contract_like_objects() {
        let contract = json!({
            "type": "arch.sw",
            "name": "armv7hf",
            "slug": "armv7hf"
        });
        let hash = hash_object(&contract);
        assert_eq!(hash.len(), 64);

        // Mutating a field changes the hash
        let mutated = json!({
            "type": "arch.sw",
            "name": "ARM v7",
            "slug": "armv7hf"
        });
        let hash2 = hash_object(&mutated);
        assert_ne!(hash, hash2);
    }

    #[test]
    fn hash_stability() {
        let value = json!({"type": "arch.sw", "name": "armv7hf", "slug": "armv7hf"});
        let hash = hash_object(&value);
        assert_eq!(hash, hash_object(&value));
    }

    #[test]
    fn empty_object_and_empty_array_differ() {
        let h1 = hash_object(&json!({}));
        let h2 = hash_object(&json!([]));
        assert_ne!(h1, h2);
    }

    #[test]
    fn deeply_nested_order_independence() {
        let h1 = hash_object(&json!({
            "data": {
                "arch": "armv7hf",
                "hdpiSupport": true,
                "configuration": {
                    "config": {"z": 3, "a": 1},
                    "supported": true
                }
            },
            "type": "hw.device-type",
            "slug": "raspberrypi3"
        }));
        let h2 = hash_object(&json!({
            "slug": "raspberrypi3",
            "type": "hw.device-type",
            "data": {
                "configuration": {
                    "supported": true,
                    "config": {"a": 1, "z": 3}
                },
                "hdpiSupport": true,
                "arch": "armv7hf"
            }
        }));
        assert_eq!(h1, h2);
    }
}
