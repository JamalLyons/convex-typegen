use std::fmt;

/// Errors that can occur during the type generation process.
#[derive(Debug)]
pub enum ConvexTypeGeneratorError
{
    /// The schema file could not be found at the specified path
    MissingSchemaFile,

    /// Failed to parse a source file
    ParsingFailed
    {
        /// Path to the file that failed to parse
        file: String,
        /// Details about the parsing failure
        details: String,
    },

    /// The schema file exists but is empty
    EmptySchemaFile
    {
        /// Path to the empty schema file
        file: String,
    },

    /// The provided path doesn't have a valid file name component
    InvalidPath(String),

    /// The file name contains invalid Unicode characters
    InvalidUnicode(String),
    /// Failed to parse ESTree JSON produced from the parser AST
    SerializationFailed(serde_json::Error),

    /// An IO error occurred while reading or writing files
    IOError
    {
        /// Path to the file where the error occurred
        file: String,
        /// The underlying IO error
        error: std::io::Error,
    },

    /// The schema file has invalid structure or content
    InvalidSchema
    {
        /// Context where the invalid schema was found
        context: String,
        /// Details about why the schema is invalid
        details: String,
    },

    /// A circular reference was detected in type definitions
    CircularReference
    {
        /// The path of types that form the circular reference
        path: Vec<String>,
    },

    /// An invalid type name was encountered
    InvalidType
    {
        /// The invalid type that was found
        found: String,
        /// List of valid type names
        valid_types: Vec<String>,
    },
}

impl fmt::Display for ConvexTypeGeneratorError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            Self::MissingSchemaFile => write!(f, "Schema file not found"),
            Self::ParsingFailed { file, details } => {
                write!(f, "Failed to parse file '{}': {}", file, details)
            }
            Self::EmptySchemaFile { file } => {
                write!(f, "Schema file '{}' is empty", file)
            }
            Self::InvalidPath(path) => {
                write!(f, "Invalid path: {}", path)
            }
            Self::InvalidUnicode(path) => {
                write!(f, "Path contains invalid Unicode: {}", path)
            }
            Self::SerializationFailed(err) => {
                write!(f, "Failed to parse AST as JSON: {}", err)
            }
            Self::IOError { file, error } => {
                write!(f, "IO error while reading '{}': {}", file, error)
            }
            Self::InvalidSchema { context, details } => {
                write!(f, "Invalid schema at {}: {}", context, details)
            }
            Self::CircularReference { path } => {
                write!(f, "Circular type reference detected: {}", path.join(" -> "))
            }
            Self::InvalidType { found, valid_types } => {
                write!(f, "Invalid type '{}'. Valid types are: {}", found, valid_types.join(", "))
            }
        }
    }
}

// Add this implementation to convert std::io::Error to ConvexTypeGeneratorError
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

// Implement std::error::Error for better error handling
impl std::error::Error for ConvexTypeGeneratorError {}

impl ConvexTypeGeneratorError
{
    /// Adds file context to an IO error
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
