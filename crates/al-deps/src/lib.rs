//! Dependency apps for the runtime (ADR-013): version selection, reading `.app`
//! packages, and loading them into the workspace.
//!
//! This crate may touch the OS (ADR-006). The selection rules and the container
//! readers are pure functions over values and byte slices, so they are tested
//! without files or a network.

pub mod artifact;
pub mod build;
pub mod cache;
pub mod demo;
pub mod lazy;
pub mod load;
pub mod package;
pub mod remote;
pub mod select;
pub mod test_apps;
pub mod version;
pub mod zip;
