//! The compile cache format (ADR-022).
//!
//! A cache entry holds the lowered form of a set of dependency apps (`al_sema::provided`): per
//! dependency object a block of IR, compressed on its own with zstd and a dictionary shared by the
//! entry, and an index with what the lowering of other objects reads about each object. The file
//! is named and checked by a [`Key`] that covers every input of the compile.
//!
//! This crate turns values into bytes and back. It reads no files and runs no processes
//! (ADR-006); the caller reads, writes and renames the file.

mod format;
mod index;
mod key;
mod workspace;

pub use format::{CacheError, Entry, build, build_layer, build_layer_with_dictionary};
pub use index::{build_index, index_file_name, open_index};
pub use key::{AppKey, FORMAT, Key, build_tag, tag_of_file_name};
pub use workspace::{WORKSPACE_FORMAT, build_workspace, carrier_key, open_workspace};
