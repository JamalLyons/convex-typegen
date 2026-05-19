//! Error taxonomy for discovery, parse, schema validation, and emit.
//!
//! [`ConvexTypeGeneratorError::IOError`] may be constructed with an empty `file` (for example via
//! [`From<std::io::Error>`]); use [`ConvexTypeGeneratorError::with_file_context`] when the caller
//! knows which path failed.

use thiserror::Error;

/// Unified error type for the whole codegen pipeline.
#[derive(Debug, Error)]
pub enum ConvexTypeGeneratorError
{
    /// The schema file could not be found at the specified path
    #[error("Schema file not found")]
    MissingSchemaFile,

    /// Failed to parse a source file
    #[error("Failed to parse file '{file}': {details}")]
    ParsingFailed
    {
        /// Path to the file that failed to parse
        file: String,
        /// Details about the parsing failure
        details: String,
    },

    /// The schema file exists but is empty
    #[error("Schema file '{file}' is empty")]
    EmptySchemaFile
    {
        /// Path to the empty schema file
        file: String,
    },

    /// The provided path doesn't have a valid file name component
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// The file name contains invalid Unicode characters
    #[error("Path contains invalid Unicode: {0}")]
    InvalidUnicode(String),

    /// Failed to parse ESTree JSON produced from the parser AST
    #[error("Failed to parse AST as JSON: {0}")]
    SerializationFailed(#[from] serde_json::Error),

    /// An IO error occurred while reading or writing files
    #[error("IO error while reading '{file}': {error}")]
    IOError
    {
        /// Path to the file where the error occurred
        file: String,
        /// The underlying IO error
        #[source]
        error: std::io::Error,
    },

    /// The schema file has invalid structure or content
    #[error("Invalid schema at {context}: {details}")]
    InvalidSchema
    {
        /// Context where the invalid schema was found
        context: String,
        /// Details about why the schema is invalid
        details: String,
    },

    /// A circular reference was detected in type definitions
    #[error("Circular type reference detected: {}", .path.join(" -> "))]
    CircularReference
    {
        /// The path of types that form the circular reference
        path: Vec<String>,
    },

    /// An invalid type name was encountered
    #[error("Invalid type '{found}'. Valid types are: {}", .valid_types.join(", "))]
    InvalidType
    {
        /// The invalid type that was found
        found: String,
        /// List of valid type names
        valid_types: Vec<String>,
    },
}

impl From<std::io::Error> for ConvexTypeGeneratorError
{
    fn from(error: std::io::Error) -> Self
    {
        ConvexTypeGeneratorError::IOError {
            file: String::new(),
            error,
        }
    }
}

impl ConvexTypeGeneratorError
{
    /// Attaches `file` to [`ConvexTypeGeneratorError::IOError`]; leaves other variants unchanged.
    pub fn with_file_context(self, file: impl Into<String>) -> Self
    {
        match self {
            Self::IOError { error, .. } => Self::IOError {
                file: file.into(),
                error,
            },
            other => other,
        }
    }
}

#[cfg(test)]
mod with_file_context_tests
{
    use std::io::Error as IoError;

    use super::*;

    #[test]
    fn attaches_path_to_io_error()
    {
        let err = ConvexTypeGeneratorError::IOError {
            file: String::new(),
            error: IoError::other("boom"),
        };
        let wrapped = err.with_file_context("/tmp/x.ts");
        assert!(matches!(wrapped, ConvexTypeGeneratorError::IOError { ref file, .. } if file == "/tmp/x.ts"));
    }

    #[test]
    fn leaves_non_io_variants_unchanged()
    {
        let err = ConvexTypeGeneratorError::MissingSchemaFile;
        assert!(matches!(
            err.with_file_context("nope"),
            ConvexTypeGeneratorError::MissingSchemaFile
        ));
    }
}

#[cfg(test)]
mod from_io_error_tests
{
    use std::io::Error as IoError;

    use super::*;

    #[test]
    fn maps_std_io_to_io_error_with_empty_file()
    {
        let e: ConvexTypeGeneratorError = IoError::other("read fail").into();
        match e {
            ConvexTypeGeneratorError::IOError { file, .. } => assert!(file.is_empty()),
            other => panic!("expected IOError, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod display_tests
{
    use super::*;

    #[test]
    fn missing_schema_file_message()
    {
        let s = ConvexTypeGeneratorError::MissingSchemaFile.to_string();
        assert!(s.contains("Schema"));
    }

    #[test]
    fn invalid_type_lists_known_validators()
    {
        let e = ConvexTypeGeneratorError::InvalidType {
            found: "bogus".into(),
            valid_types: vec!["string".into()],
        };
        let s = e.to_string();
        assert!(s.contains("bogus") && s.contains("string"));
    }
}
