//! The error returned when a contract cannot be constructed or updated.

use std::fmt;

use crate::types::InvalidIdentifier;

/// Error produced when a JSON document cannot be read as a contract.
///
/// An opaque wrapper over [`serde_json::Error`], the failure itself is
/// reachable through [`std::error::Error::source`].
#[derive(Debug)]
pub struct JsonError(serde_json::Error);

impl From<serde_json::Error> for JsonError {
    fn from(source: serde_json::Error) -> Self {
        Self(source)
    }
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for JsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

/// Error produced when a [`Contract`](crate::Contract) cannot be
/// constructed or updated.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The JSON document cannot be deserialized as a contract.
    Deserialization {
        /// Why the document could not be deserialized.
        source: JsonError,
    },

    /// A field that must hold an identifier does not, once its
    /// `{{this.*}}` templates are interpolated.
    InvalidIdentifier {
        /// The field that failed, as it is named in error messages —
        /// `contract type`, `slug`, `canonical slug` or `alias`.
        field: &'static str,
        /// Why the interpolated value is not an identifier.
        source: InvalidIdentifier,
    },

    /// Two children have types where one is a proper prefix of the other,
    /// e.g. `sw.os` and `sw.os.kernel`.
    OverlappingChildTypes {
        /// The shorter type, which a subtree would have to replace.
        outer: String,
        /// The longer type, nested under `outer`.
        inner: String,
    },

    /// A child has no slug.
    MissingChildSlug {
        /// The type of the slugless child.
        kind: String,
    },

    /// A child's type cannot be split into tree segments.
    InvalidChildType {
        /// The type that could not be parsed.
        kind: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Deserialization { source } => write!(f, "invalid contract: {source}"),
            Error::InvalidIdentifier { field, source } => {
                write!(f, "invalid contract: invalid {field}: {source}")
            }
            Error::OverlappingChildTypes { outer, inner } => write!(
                f,
                "invalid children: overlapping child types: '{outer}' is a prefix of '{inner}'"
            ),
            Error::MissingChildSlug { kind } => {
                write!(
                    f,
                    "invalid children: slug missing for child of type '{kind}'"
                )
            }
            Error::InvalidChildType { kind } => {
                write!(f, "invalid children: '{kind}' is not a valid tree path")
            }
        }
    }
}

impl From<JsonError> for Error {
    fn from(source: JsonError) -> Self {
        Error::Deserialization { source }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Deserialization { source } => Some(source),
            Error::InvalidIdentifier { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_contract_renders_the_serde_message() {
        let source = serde_json::from_value::<crate::RawContract>(serde_json::json!({}))
            .expect_err("a contract without a type cannot be deserialized");
        let message = source.to_string();
        let err = Error::from(JsonError::from(source));

        assert_eq!(err.to_string(), format!("invalid contract: {message}"));
    }

    #[test]
    fn display_names_the_offending_values() {
        let err = Error::OverlappingChildTypes {
            outer: "sw.os".to_string(),
            inner: "sw.os.kernel".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("sw.os.kernel"), "{msg}");
        assert!(msg.contains("overlapping child types"), "{msg}");
    }
}
