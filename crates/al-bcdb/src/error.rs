//! Errors. Two kinds matter to a caller, and both mirror bcdb's exceptions:
//! `InvalidData` is a structure that contradicts what was derived (bcdb's
//! `InvalidDataException`), `NotSupported` is something known but not decodable
//! (bcdb's `NotSupportedException`). Both carry a message that names the column,
//! table or page, and never a default value (see PROVENANCE.md, "Loud failures").
//!
//! Entry points: `Error`, `Result`.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The bytes contradict the derived model; the reader refuses to guess.
    InvalidData(String),
    /// A type, encoding or layout that is known but not supported.
    NotSupported(String),
    /// A caller mistake: an unknown table, an ambiguous name, a bad option value.
    Argument(String),
    /// The byte source failed.
    Io(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn message(&self) -> &str {
        match self {
            Error::InvalidData(m) | Error::NotSupported(m) | Error::Argument(m) | Error::Io(m) => m,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

/// `Err(Error::InvalidData(format!(..)))`.
#[macro_export]
macro_rules! invalid {
    ($($arg:tt)*) => { $crate::error::Error::InvalidData(format!($($arg)*)) };
}

/// `Error::NotSupported(format!(..))`.
#[macro_export]
macro_rules! unsupported {
    ($($arg:tt)*) => { $crate::error::Error::NotSupported(format!($($arg)*)) };
}
