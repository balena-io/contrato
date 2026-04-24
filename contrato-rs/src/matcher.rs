//! Matcher primitives used by the child-search path.
//!
//! This module is the crate-private home for two predicates:
//!
//! - [`partial_match`], a deep partial-match predicate over
//!   [`serde_json::Value`]: an object pattern matches a target when
//!   every key in the pattern is present in the target and the values
//!   match (recursively for objects, strict equality for everything
//!   else).
//! - [`version_match`], a semver-aware version predicate: when both
//!   target and requirement parse as semver, the target version must
//!   satisfy the requirement range; otherwise matching falls back to
//!   string equality so that identifier-shaped versions such as
//!   `"wheezy"` still work.

use serde_json::Value;

use crate::types::{Version, VersionReq};

/// Deep partial match over [`serde_json::Value`].
///
/// Returns `true` when `target` is "compatible" with `pattern`:
///
/// - **Objects**: every key in `pattern` must also appear in `target`,
///   and the corresponding values must themselves satisfy
///   [`partial_match`]. Extra keys in `target` are ignored.
/// - **Everything else** (numbers, strings, booleans, null, arrays):
///   strict equality. Arrays are compared element-by-element including
///   length — partial array matching is deliberately not supported.
///
/// This predicate is the deep-match primitive used by
/// [`Contract::find_children`](crate::Contract::find_children):
/// the pattern is the matcher's `data` field and the target is
/// the child contract's own `data` field.
pub(crate) fn partial_match(pattern: &Value, target: &Value) -> bool {
    match pattern {
        Value::Object(pattern_obj) => {
            let Some(target_obj) = target.as_object() else {
                return false;
            };
            for (key, pattern_value) in pattern_obj {
                match target_obj.get(key) {
                    Some(target_value) => {
                        if !partial_match(pattern_value, target_value) {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            true
        }
        _ => pattern == target,
    }
}

/// Semver-aware version predicate.
///
/// - If `required` is `None` the requirement is vacuous and the
///   predicate returns `true` regardless of `target`.
/// - If `required` is present but `target` is `None`, the predicate
///   returns `false` — there is nothing for the requirement to apply
///   to.
/// - Otherwise the match is delegated to [`VersionReq::matches`],
///   which takes the allocation-free fast path for the two common
///   cases (semver range × semver version, identifier × identifier)
///   and falls back to string equality only for the mismatched
///   identifier-vs-semver combinations.
///
/// Taking typed [`Version`] / [`VersionReq`] references rather than
/// `Option<&str>` lets callers on the validation hot path skip both
/// the per-call `.to_string()` allocation **and** the re-parse from
/// string to semver — the parsed inner values are already owned by
/// the caller's `Version` / `VersionReq`.
pub(crate) fn version_match(target: Option<&Version>, required: Option<&VersionReq>) -> bool {
    let Some(required) = required else {
        return true;
    };
    let Some(target) = target else {
        return false;
    };
    required.matches(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── partial_match ────────────────────────────────────────────────────

    #[test]
    fn partial_match_empty_pattern_matches_any_object() {
        assert!(partial_match(&json!({}), &json!({"foo": 1})));
        assert!(partial_match(&json!({}), &json!({})));
    }

    #[test]
    fn partial_match_single_key_subset() {
        assert!(partial_match(
            &json!({"type": "sw.os"}),
            &json!({"type": "sw.os", "slug": "debian"})
        ));
    }

    #[test]
    fn partial_match_missing_key_rejects() {
        assert!(!partial_match(
            &json!({"type": "sw.os"}),
            &json!({"slug": "debian"})
        ));
    }

    #[test]
    fn partial_match_key_value_mismatch_rejects() {
        assert!(!partial_match(
            &json!({"type": "sw.os"}),
            &json!({"type": "hw.device-type"})
        ));
    }

    #[test]
    fn partial_match_nested_object() {
        assert!(partial_match(
            &json!({"data": {"arch": "armv7hf"}}),
            &json!({"type": "hw.device-type", "data": {"arch": "armv7hf", "soc": "bcm2837"}})
        ));
    }

    #[test]
    fn partial_match_nested_object_mismatch() {
        assert!(!partial_match(
            &json!({"data": {"arch": "armv7hf"}}),
            &json!({"data": {"arch": "aarch64"}})
        ));
    }

    #[test]
    fn partial_match_array_strict_equality() {
        assert!(partial_match(&json!([1, 2, 3]), &json!([1, 2, 3])));
        assert!(!partial_match(&json!([1, 2]), &json!([1, 2, 3])));
    }

    #[test]
    fn partial_match_primitive_equality() {
        assert!(partial_match(&json!(42), &json!(42)));
        assert!(!partial_match(&json!(42), &json!(43)));
        assert!(partial_match(&json!("foo"), &json!("foo")));
        assert!(!partial_match(&json!("foo"), &json!("bar")));
        assert!(partial_match(&Value::Null, &Value::Null));
    }

    #[test]
    fn partial_match_object_pattern_rejects_non_object_target() {
        assert!(!partial_match(&json!({"foo": 1}), &json!([1, 2])));
        assert!(!partial_match(&json!({"foo": 1}), &json!("string")));
        assert!(!partial_match(&json!({"foo": 1}), &Value::Null));
    }

    // ── version_match ────────────────────────────────────────────────────

    /// Builds a [`Version`] from a string literal for use in the
    /// `version_match` tests. Mirrors the production construction
    /// path (`Version::new`) so semver-vs-identifier classification
    /// matches the runtime behavior.
    fn v(s: &str) -> Version {
        Version::new(s)
    }

    /// Builds a [`VersionReq`] from a string literal. Same rationale
    /// as [`v`] — goes through the standard constructor so `is_semver_range`
    /// classification matches what the search path sees.
    fn vr(s: &str) -> VersionReq {
        VersionReq::new(s)
    }

    #[test]
    fn version_match_none_requirement_is_satisfied() {
        assert!(version_match(Some(&v("1.0.0")), None));
        assert!(version_match(None, None));
    }

    #[test]
    fn version_match_required_but_target_missing_fails() {
        assert!(!version_match(None, Some(&vr("1.0.0"))));
    }

    #[test]
    fn version_match_semver_range_satisfies() {
        assert!(version_match(Some(&v("4.8.1")), Some(&vr(">=4.8.0"))));
        assert!(version_match(Some(&v("4.8.0")), Some(&vr("^4.8.0"))));
    }

    #[test]
    fn version_match_semver_range_does_not_satisfy() {
        assert!(!version_match(Some(&v("3.0.0")), Some(&vr(">=4.8.0"))));
    }

    #[test]
    fn version_match_identifier_equality() {
        assert!(version_match(Some(&v("wheezy")), Some(&vr("wheezy"))));
        assert!(!version_match(Some(&v("wheezy")), Some(&vr("jessie"))));
    }

    #[test]
    fn version_match_identifier_target_with_semver_requirement_falls_back_to_equality() {
        // "wheezy" parses as an identifier Version, ">=1.0.0" parses
        // as a SemverRange VersionReq. The mismatched case falls
        // back to Display-string equality — "wheezy" != ">=1.0.0".
        assert!(!version_match(Some(&v("wheezy")), Some(&vr(">=1.0.0"))));
    }

    #[test]
    fn version_match_semver_target_with_identifier_requirement_falls_back_to_equality() {
        // "1.0.0" parses as a semver Version, "wheezy" parses as an
        // identifier VersionReq. The mismatched case falls back to
        // Display-string equality — they differ.
        assert!(!version_match(Some(&v("1.0.0")), Some(&vr("wheezy"))));
    }
}
