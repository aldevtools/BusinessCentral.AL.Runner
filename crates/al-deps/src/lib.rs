//! Dependency apps for the runtime (ADR-013): version selection, reading `.app`
//! packages, and loading them into the workspace.
//!
//! This crate may touch the OS (ADR-006). The selection rules and the container
//! readers are pure functions over values and byte slices, so they are tested
//! without files or a network.

pub mod artifact;
pub mod assembly_refs;
pub mod build;
pub mod cache;
pub mod demo;
pub mod empty;
pub mod lazy;
pub mod license;
pub mod load;
pub mod package;
pub mod remote;
pub mod render_set;
pub mod select;
pub mod service_tier;
pub mod test_apps;
pub mod version;
pub mod xliff;
pub mod zip;
