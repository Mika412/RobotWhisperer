#![deny(missing_debug_implementations)]
#![deny(unused_must_use)]

pub mod dialect;
pub mod hash;
pub mod id;
pub mod schema;
pub mod value;
pub mod viz;

pub use dialect::Dialect;
pub use hash::canonical_schema_id;
pub use id::SchemaId;
pub use schema::{
    ArrayLength, CanonicalSchema, ConstantDef, FieldDef, FieldType, MessageDef, ParsedSchema,
    PrimitiveType, SchemaKind,
};
pub use value::{CanonicalValue, ProjectedValue};
pub use viz::{viz_role_for_schema, VisualizationRole};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CanonicalError {
    #[error("schema error: {0}")]
    Schema(String),
    #[error("value error: {0}")]
    Value(String),
}

pub type CanonicalResult<T> = Result<T, CanonicalError>;
