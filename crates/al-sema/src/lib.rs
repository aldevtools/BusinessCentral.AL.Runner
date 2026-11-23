//! Semantic model: a workspace of apps, their objects, and resolution.
//!
//! Crate root of the semantic model.

pub mod alcodes;
pub mod base_access;
pub mod calcformula;
pub mod check;
pub mod features;
pub mod halt;
pub mod incremental;
pub mod lower;
pub mod members;
pub(crate) mod own_named_page_methods;
pub mod pages;
pub mod property_values;
pub mod provided;
pub mod queries;
pub mod relation;
pub mod reports;
pub(crate) mod scan;
pub mod types;
pub mod workspace;
pub mod xmlports;

pub use workspace::{
    AppKind, AppSource, Diagnostic, FileIndex, LazyDep, LazySource, ObjKind, Resolution, SourceLoader, Workspace,
};
