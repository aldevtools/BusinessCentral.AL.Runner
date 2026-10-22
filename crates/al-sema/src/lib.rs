//! Semantic model: a workspace of apps, their objects, and resolution.

pub mod alcodes;
pub mod base_access;
pub mod calcformula;
pub mod check;
pub mod features;
pub mod incremental;
pub mod lower;
pub mod members;
pub mod pages;
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
