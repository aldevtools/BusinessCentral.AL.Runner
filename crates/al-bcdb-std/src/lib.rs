//! The operating-system side of `al-bcdb` (ADR-006, ADR-020): a file as a byte source, and the
//! table provider that gives a run the rows of a BC database backup.
//!
//! The OS side of `al-bcdb`: file byte source and the backup table provider for `--test-data`
//! (ADR-020).

mod file;
mod provider;

pub use file::FileSource;
pub use provider::{BackupProvider, DEFAULT_COMPANY, default_backup, default_backup_in, default_backup_in_country};
