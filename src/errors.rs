use thiserror::Error;

/// Errors that can occur during the type generation process.
#[derive(Debug, Error)]
pub enum ConvexTypeGeneratorError {
    /// The schema file could not be found at the specified path
    #[error("Schema file not found")]
    MissingSchemaFile,

    /// Failed to parse a source file
    #[error("Failed to parse file '{file}': {details}")]
    ParsingFailed {
        /// Path to the file that failed to parse
        file: String,
        /// Details about the parsing failure
        details: String,
    },

    /// The schema file exists but is empty
    #[error("Schema file '{file}' is empty")]
    EmptySchemaFile {
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
    IOError {
        /// Path to the file where the error occurred
        file: String,
        /// The underlying IO error
        #[source]
        error: std::io::Error,
    },

    /// The schema file has invalid structure or content
    #[error("Invalid schema at {context}: {details}")]
    InvalidSchema {
        /// Context where the invalid schema was found
        context: String,
        /// Details about why the schema is invalid
        details: String,
    },

    /// A circular reference was detected in type definitions
    #[error("Circular type reference detected: {}", .path.join(" -> "))]
    CircularReference {
        /// The path of types that form the circular reference
        path: Vec<String>,
    },

    /// An invalid type name was encountered
    #[error("Invalid type '{found}'. Valid types are: {}", .valid_types.join(", "))]
    InvalidType {
        /// The invalid type that was found
        found: String,
        /// List of valid type names
        valid_types: Vec<String>,
    },
}

impl From<std::io::Error> for ConvexTypeGeneratorError {
    fn from(error: std::io::Error) -> Self {
        ConvexTypeGeneratorError::IOError {
            file: String::new(),
            error,
        }
    }
}

impl ConvexTypeGeneratorError {
    /// Adds file context to an IO error
    pub fn with_file_context(self, file: impl Into<String>) -> Self {
        match self {
            Self::IOError { error, .. } => Self::IOError {
                file: file.into(),
                error,
            },
            other => other,
        }
    }
}
