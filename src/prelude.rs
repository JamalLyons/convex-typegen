pub use crate::{
    convex::{ConvexClientExt, ConvexValueExt, IntoConvexValue},
    errors::ConvexTypeGeneratorError,
    generate, resolved_function_paths, Configuration,
};
// Re-export serde types for convenience
pub use serde::{Deserialize, Serialize};
pub use serde_json::Error as ConvexJsonError;
pub use serde_json::Value as ConvexJsonValue;
