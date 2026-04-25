//! Validated dot-separated path type.
//!
//! Provides [`DottedPath`] for representing and working with dot-separated
//! paths like `"sw.os"` or `"data.arch"`. Paths are validated on construction
//! to guarantee non-empty segments.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// A validated dot-separated path (e.g., `"sw.os"`, `"data.arch"`).
///
/// Guarantees on construction:
/// - Non-empty
/// - No empty segments (no leading, trailing, or consecutive dots)
///
/// Provides segment iteration via [`segments()`](DottedPath::segments) and
/// JSON value traversal via [`resolve()`](DottedPath::resolve).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct DottedPath(String);

/// Error returned when a string cannot be parsed as a [`DottedPath`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidPath {
    /// The input string was empty.
    Empty,
    /// The input contained an empty segment (leading, trailing, or consecutive
    /// dots). Stores the original input.
    EmptySegment(String),
}

impl fmt::Display for InvalidPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidPath::Empty => write!(f, "path cannot be empty"),
            InvalidPath::EmptySegment(s) => {
                write!(f, "path '{s}' contains an empty segment")
            }
        }
    }
}

impl std::error::Error for InvalidPath {}

impl TryFrom<String> for DottedPath {
    type Error = InvalidPath;

    /// Validates and creates a `DottedPath` from an owned string.
    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.is_empty() {
            return Err(InvalidPath::Empty);
        }
        if s.starts_with('.') || s.ends_with('.') || s.contains("..") {
            return Err(InvalidPath::EmptySegment(s));
        }
        Ok(Self(s))
    }
}

impl TryFrom<&str> for DottedPath {
    type Error = InvalidPath;

    /// Validates and creates a `DottedPath` from a string slice.
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        DottedPath::try_from(s.to_owned())
    }
}

impl DottedPath {
    /// Creates a `DottedPath` by joining parts with `"."`.
    ///
    /// Intended for constructing paths from already-validated pieces (e.g.,
    /// contract type + slug). Parts must be non-empty and must not contain dots.
    ///
    /// # Panics
    ///
    /// Panics if `parts` is empty. Debug-asserts that no part is empty or
    /// contains a dot.
    pub(crate) fn from_parts(parts: &[&str]) -> Self {
        assert!(
            !parts.is_empty(),
            "DottedPath::from_parts called with empty slice"
        );
        debug_assert!(
            parts.iter().all(|p| !p.is_empty() && !p.contains('.')),
            "DottedPath::from_parts received invalid parts: {parts:?}"
        );
        Self(parts.join("."))
    }

    /// Returns an iterator over the dot-separated segments.
    pub(crate) fn segments(&self) -> impl Iterator<Item = &str> + '_ {
        self.0.split('.')
    }

    /// Resolves this path against a JSON value by traversing nested objects.
    ///
    /// Each segment indexes into the current object. Returns `None` if any
    /// segment is missing or the traversal hits a non-object value.
    pub(crate) fn resolve<'a>(&self, root: &'a Value) -> Option<&'a Value> {
        let mut current = root;
        for segment in self.segments() {
            match current {
                Value::Object(map) => {
                    current = map.get(segment)?;
                }
                _ => return None,
            }
        }
        Some(current)
    }
}

impl fmt::Display for DottedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for DottedPath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        DottedPath::try_from(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // -----------------------------------------------------------------------
    // Construction
    // -----------------------------------------------------------------------

    #[test]
    fn try_from_two_segments() {
        let p = DottedPath::try_from("sw.os").unwrap();
        assert_eq!(p.to_string(), "sw.os");
    }

    #[test]
    fn try_from_three_segments() {
        let p = DottedPath::try_from("a.b.c").unwrap();
        assert_eq!(p.to_string(), "a.b.c");
    }

    #[test]
    fn try_from_single_segment() {
        let p = DottedPath::try_from("single").unwrap();
        assert_eq!(p.to_string(), "single");
    }

    #[test]
    fn try_from_owned_string() {
        let p = DottedPath::try_from("sw.os".to_string()).unwrap();
        assert_eq!(p.to_string(), "sw.os");
    }

    #[test]
    fn try_from_empty_is_err() {
        let err = DottedPath::try_from("").unwrap_err();
        assert_eq!(err, InvalidPath::Empty);
    }

    #[test]
    fn try_from_leading_dot_is_err() {
        let err = DottedPath::try_from(".foo").unwrap_err();
        assert!(matches!(err, InvalidPath::EmptySegment(_)));
    }

    #[test]
    fn try_from_trailing_dot_is_err() {
        let err = DottedPath::try_from("foo.").unwrap_err();
        assert!(matches!(err, InvalidPath::EmptySegment(_)));
    }

    #[test]
    fn try_from_consecutive_dots_is_err() {
        let err = DottedPath::try_from("foo..bar").unwrap_err();
        assert!(matches!(err, InvalidPath::EmptySegment(_)));
    }

    #[test]
    fn try_from_single_dot_is_err() {
        let err = DottedPath::try_from(".").unwrap_err();
        assert!(matches!(err, InvalidPath::EmptySegment(_)));
    }

    // -----------------------------------------------------------------------
    // from_parts
    // -----------------------------------------------------------------------

    #[test]
    fn from_parts_two() {
        let p = DottedPath::from_parts(&["sw", "os"]);
        assert_eq!(p.to_string(), "sw.os");
    }

    #[test]
    fn from_parts_three() {
        let p = DottedPath::from_parts(&["a", "b", "c"]);
        assert_eq!(p.to_string(), "a.b.c");
    }

    #[test]
    fn from_parts_single() {
        let p = DottedPath::from_parts(&["single"]);
        assert_eq!(p.to_string(), "single");
    }

    // -----------------------------------------------------------------------
    // segments
    // -----------------------------------------------------------------------

    #[test]
    fn segments_two() {
        let p = DottedPath::try_from("sw.os").unwrap();
        let segs: Vec<&str> = p.segments().collect();
        assert_eq!(segs, vec!["sw", "os"]);
    }

    #[test]
    fn segments_three() {
        let p = DottedPath::try_from("a.b.c").unwrap();
        let segs: Vec<&str> = p.segments().collect();
        assert_eq!(segs, vec!["a", "b", "c"]);
    }

    #[test]
    fn segments_single() {
        let p = DottedPath::try_from("single").unwrap();
        let segs: Vec<&str> = p.segments().collect();
        assert_eq!(segs, vec!["single"]);
    }

    // -----------------------------------------------------------------------
    // resolve
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_nested_object() {
        let root = json!({"data": {"arch": "amd64"}});
        let p = DottedPath::try_from("data.arch").unwrap();
        assert_eq!(p.resolve(&root), Some(&json!("amd64")));
    }

    #[test]
    fn resolve_deep_path() {
        let root = json!({"a": {"b": {"c": 42}}});
        let p = DottedPath::try_from("a.b.c").unwrap();
        assert_eq!(p.resolve(&root), Some(&json!(42)));
    }

    #[test]
    fn resolve_returns_intermediate_object() {
        let root = json!({"data": {"arch": "amd64", "os": "linux"}});
        let p = DottedPath::try_from("data").unwrap();
        assert_eq!(
            p.resolve(&root),
            Some(&json!({"arch": "amd64", "os": "linux"}))
        );
    }

    #[test]
    fn resolve_missing_key_returns_none() {
        let root = json!({"data": {"arch": "amd64"}});
        let p = DottedPath::try_from("data.missing").unwrap();
        assert_eq!(p.resolve(&root), None);
    }

    #[test]
    fn resolve_non_object_returns_none() {
        let root = json!({"data": "not an object"});
        let p = DottedPath::try_from("data.arch").unwrap();
        assert_eq!(p.resolve(&root), None);
    }

    #[test]
    fn resolve_against_array_returns_none() {
        let root = json!({"data": [1, 2, 3]});
        let p = DottedPath::try_from("data.0").unwrap();
        assert_eq!(p.resolve(&root), None);
    }

    // -----------------------------------------------------------------------
    // Display
    // -----------------------------------------------------------------------

    #[test]
    fn display() {
        let p = DottedPath::try_from("sw.os").unwrap();
        assert_eq!(p.to_string(), "sw.os");
    }

    // -----------------------------------------------------------------------
    // Serde
    // -----------------------------------------------------------------------

    #[test]
    fn serde_round_trip() {
        let p = DottedPath::try_from("sw.os").unwrap();
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json, json!("sw.os"));
        let deserialized: DottedPath = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, p);
    }

    #[test]
    fn serde_deserialize_rejects_empty() {
        let result = serde_json::from_value::<DottedPath>(json!(""));
        assert!(result.is_err());
    }

    #[test]
    fn serde_deserialize_rejects_empty_segment() {
        let result = serde_json::from_value::<DottedPath>(json!("foo..bar"));
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // InvalidPath Display
    // -----------------------------------------------------------------------

    #[test]
    fn invalid_path_empty_display() {
        let err = InvalidPath::Empty;
        assert_eq!(err.to_string(), "path cannot be empty");
    }

    #[test]
    fn invalid_path_empty_segment_display() {
        let err = InvalidPath::EmptySegment("foo..bar".to_string());
        let msg = err.to_string();
        assert!(msg.contains("foo..bar"));
        assert!(msg.contains("empty segment"));
    }
}
