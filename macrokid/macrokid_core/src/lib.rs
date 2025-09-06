pub mod ir;
pub mod attr;
pub mod function;
pub mod common;
pub mod derive;

pub use ir::{FieldKind, TypeKind, TypeSpec, VariantSpec};
pub use common::{attrs, builders, patterns, diag, type_utils, repr, attr_schema, collect, codegen};
pub use derive::impl_for_trait;
#[cfg(feature = "pattern_dsl")]
pub use common::pattern_dsl;
