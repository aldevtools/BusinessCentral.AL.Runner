//! `al-bcdb`: the bcdb read commands over the `al-bcdb` core. This is the one place that
//! touches the OS (ADR-006): it maps the file and hands the core a byte source.
//!
//! Output is byte-for-byte what the published bcdb prints for the same command and file
//! (the differential tests in `tests/differential.rs` hold it to that), except where this
//! header says otherwise:
//!  * `tables` sorts ordinally; bcdb sorts with the current culture.
//!  * `--prefetch` is accepted and does nothing: the file is memory-mapped.
//!  * `restore` is not ported: it needs a SQL Server as destination.
//!  * `export` is new.
//!
//! The al-bcdb binary: reads BC database backups (.bak, .bacpac). Entry points: `main`.

mod cmd;
mod opts;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match cmd::run(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    };
    ExitCode::from(code)
}
