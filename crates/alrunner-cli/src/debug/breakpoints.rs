//! Where a breakpoint goes (ADR-048 decision 6, as v2's `DapBreakpointResolver`): the exact line of a statement, every
//! statement that starts on the line in every object of the file, a column narrowing it to the statement that starts
//! there, else to the one that holds it, else the whole line. No nearest-line fallback.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Why a breakpoint is not verified (ADR-048 decision 6).
pub const NOT_COMPILED: &str = "the app did not compile";
pub const NO_STATEMENT: &str = "no executable AL statement on this line in this file";
pub const NO_SOURCE_FILE: &str = "no source file for this app";

/// A breakpoint the client asked for: line and column, 1-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub line: u32,
    pub column: Option<u32>,
}

/// The answer to one [`Request`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub verified: bool,
    /// The line it stands on (the requested one: there is no fallback).
    pub line: u32,
    /// The first column of the statements it stops at.
    pub column: Option<u32>,
    /// The `Program::positions` ids it stops at.
    pub targets: Vec<u32>,
    pub message: Option<&'static str>,
}

/// A statement of a file on disk.
#[derive(Debug, Clone, Copy)]
struct Stmt {
    line: u32,
    col: u32,
    pos: u32,
}

/// The statements of the program by the file on disk that holds them.
pub struct Index {
    pub program: Arc<al_ir::Program>,
    /// Per source file of the program (`SrcPos::file`): its file on disk, when its app was given as a folder.
    pub disk: Vec<Option<PathBuf>>,
    by_file: HashMap<String, Vec<Stmt>>,
    /// The paths, inside their package, of the files of apps given as a package (no file on disk).
    packaged: Vec<String>,
}

/// How paths compare: as the file system does (case-sensitive on Linux, not on Windows and macOS), after symbolic
/// links and `..` are resolved where the file exists.
pub fn key(p: &Path) -> String {
    let p = std::fs::canonicalize(p)
        .or_else(|_| std::path::absolute(p))
        .unwrap_or_else(|_| p.to_path_buf());
    let text = p.to_string_lossy().replace('\\', "/");
    if cfg!(any(windows, target_os = "macos")) {
        text.to_lowercase()
    } else {
        text
    }
}

impl Index {
    /// `apps`: the APPs of the command line, by the caller's index; `order`: per app of the program, the caller's index.
    pub fn build(program: Arc<al_ir::Program>, order: &[usize], apps: &[PathBuf]) -> Index {
        let folder = |caller: usize| apps.get(caller).filter(|p| p.is_dir());
        let mut disk = Vec::with_capacity(program.sources.len());
        let mut packaged = Vec::new();
        for f in &program.sources {
            let caller = order.get(f.app as usize).copied().unwrap_or(usize::MAX);
            match folder(caller) {
                Some(dir) => disk.push(Some(dir.join(&f.path))),
                None => {
                    if caller < apps.len() {
                        packaged.push(f.path.replace('\\', "/"));
                    }
                    disk.push(None);
                }
            }
        }
        let mut by_file: HashMap<String, Vec<Stmt>> = HashMap::new();
        let mut keys: HashMap<u32, String> = HashMap::new();
        let in_folder = |oi: usize| {
            let app = program.objects[oi].app as usize;
            order.get(app).is_some_and(|&c| folder(c).is_some())
        };
        for s in al_testrun::coverage::statements(&program, in_folder, |a| a) {
            let Some(p) = program.pos(s.pos) else { continue };
            let Some(Some(path)) = disk.get(p.file as usize).map(Option::as_ref) else {
                continue;
            };
            let k = keys.entry(p.file).or_insert_with(|| key(path)).clone();
            by_file.entry(k).or_default().push(Stmt {
                line: s.line,
                col: s.column,
                pos: s.pos,
            });
        }
        for v in by_file.values_mut() {
            v.sort_by_key(|s| (s.line, s.col, s.pos));
        }
        Index {
            program,
            disk,
            by_file,
            packaged,
        }
    }

    /// Whether the statement at `pos` is in a file on disk (a debugger stops and steps only there).
    pub fn on_disk(&self, pos: u32) -> bool {
        self.program
            .pos(pos)
            .and_then(|p| self.disk.get(p.file as usize))
            .is_some_and(Option::is_some)
    }

    /// The file on disk of the statement at `pos`, with its line and column.
    pub fn location(&self, pos: u32) -> Option<(&Path, u32, u32)> {
        let p = self.program.pos(pos)?;
        let path = self.disk.get(p.file as usize)?.as_deref()?;
        Some((path, p.line, p.col))
    }

    /// The breakpoints `requests` of the file `path`.
    pub fn resolve(&self, path: &str, requests: &[Request]) -> Vec<Resolved> {
        let stmts = self.by_file.get(&key(Path::new(path)));
        let reason = if stmts.is_some() {
            NO_STATEMENT
        } else {
            let slashed = path.replace('\\', "/");
            if self
                .packaged
                .iter()
                .any(|p| slashed.ends_with(&format!("/{p}")) || slashed == *p)
            {
                NO_SOURCE_FILE
            } else {
                NO_STATEMENT
            }
        };
        requests
            .iter()
            .map(|r| {
                let on_line: Vec<Stmt> = stmts
                    .map(|v| v.iter().filter(|s| s.line == r.line).copied().collect())
                    .unwrap_or_default();
                if on_line.is_empty() {
                    return Resolved {
                        verified: false,
                        line: r.line,
                        column: None,
                        targets: Vec::new(),
                        message: Some(reason),
                    };
                }
                let chosen: Vec<Stmt> = match r.column {
                    None => on_line,
                    Some(c) => {
                        let starts: Vec<Stmt> = on_line.iter().filter(|s| s.col == c).copied().collect();
                        if !starts.is_empty() {
                            starts
                        } else {
                            // The statement that holds the column: the last one that starts before it on the line.
                            match on_line.iter().rev().find(|s| s.col < c) {
                                Some(s) => on_line.iter().filter(|o| o.col == s.col).copied().collect(),
                                None => on_line,
                            }
                        }
                    }
                };
                Resolved {
                    verified: true,
                    line: r.line,
                    column: chosen.iter().map(|s| s.col).min(),
                    targets: chosen.iter().map(|s| s.pos).collect(),
                    message: None,
                }
            })
            .collect()
    }
}

/// Every breakpoint unverified for `reason` (the app did not compile).
pub fn unverified(requests: &[Request], reason: &'static str) -> Vec<Resolved> {
    requests
        .iter()
        .map(|r| Resolved {
            verified: false,
            line: r.line,
            column: None,
            targets: Vec::new(),
            message: Some(reason),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "codeunit 50100 A\n{\n    procedure P()\n    var\n        X: Integer;\n    begin\n        X := 1; X := 2;\n\n        X := 3;\n    end;\n}\n";

    fn index() -> (tempfile::TempDir, Index) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("A.Codeunit.al"), SRC).unwrap();
        let mut lowered = al_testkit::compile(SRC);
        let program = Arc::make_mut(&mut lowered.program);
        for f in &mut program.sources {
            f.path = "A.Codeunit.al".into();
        }
        let index = Index::build(lowered.program.clone(), &[0], &[dir.path().to_path_buf()]);
        (dir, index)
    }

    fn file(dir: &tempfile::TempDir) -> String {
        dir.path().join("A.Codeunit.al").to_string_lossy().into_owned()
    }

    #[test]
    fn a_line_with_two_statements_arms_both_and_a_blank_line_is_not_verified() {
        let (dir, ix) = index();
        let r = ix.resolve(
            &file(&dir),
            &[
                Request { line: 7, column: None },
                Request { line: 8, column: None },
                Request { line: 9, column: None },
            ],
        );
        assert!(
            r[0].verified && r[0].targets.len() == 2 && r[0].column == Some(9),
            "{r:?}"
        );
        assert_eq!((r[1].verified, r[1].message), (false, Some(NO_STATEMENT)), "{r:?}");
        assert!(r[2].verified && r[2].targets.len() == 1, "{r:?}");
    }

    #[test]
    fn a_column_narrows_to_the_statement_that_starts_or_holds_it() {
        let (dir, ix) = index();
        let both = ix.resolve(&file(&dir), &[Request { line: 7, column: None }])[0]
            .targets
            .clone();
        let at = |c| {
            ix.resolve(
                &file(&dir),
                &[Request {
                    line: 7,
                    column: Some(c),
                }],
            )
            .remove(0)
        };
        // "        X := 1; X := 2;": the statements start at columns 9 and 17.
        assert_eq!(at(17).targets, vec![both[1]]);
        assert_eq!(at(17).column, Some(17));
        assert_eq!(at(20).targets, vec![both[1]]);
        assert_eq!(at(12).targets, vec![both[0]]);
        // A column before the first statement: the whole line.
        assert_eq!(at(2).targets, both);
    }

    #[test]
    fn a_file_that_is_not_part_of_the_app_is_not_verified() {
        let (dir, ix) = index();
        let other = dir.path().join("Other.al").to_string_lossy().into_owned();
        let r = ix.resolve(&other, &[Request { line: 7, column: None }]);
        assert_eq!((r[0].verified, r[0].message), (false, Some(NO_STATEMENT)));
    }
}
