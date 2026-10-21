//! Where a breakpoint goes (ADR-048 decision 6, as v2's `DapBreakpointResolver`): the exact line of a statement, every
//! statement that starts on the line in every object of the file, a column narrowing it to the statement that starts
//! there, else to the one that holds it, else the whole line. No nearest-line fallback.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Why a breakpoint is not verified (ADR-048 decision 6).
pub const NOT_COMPILED: &str = "the app did not compile";
pub const NO_STATEMENT: &str = "no executable AL statement on this line in this file";
pub const NO_SOURCE_FILE: &str = "no source file for this app";
/// A statement of a dependency app that no selected test reaches: it is not compiled (ADR-013 decision 6), so it cannot
/// stop.
pub const NOT_REACHED: &str = "the selected tests do not reach this statement";

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
    pub message: Option<String>,
}

/// A statement of a source file.
#[derive(Debug, Clone, Copy)]
struct Stmt {
    line: u32,
    col: u32,
    pos: u32,
}

/// The text of a file of a package, by its path in the package.
pub type PackageText = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// Where the source of an app the run compiles comes from.
#[derive(Clone)]
pub enum Origin {
    /// An app folder: its files are on disk.
    Folder { dir: PathBuf, prefix: String },
    /// A source package (`.app` with source, `*.Source.zip`): its files are in the package. `label` names the folder
    /// the debugger writes them to; `text` takes the path the program has.
    Package {
        label: String,
        prefix: String,
        text: PackageText,
    },
}

impl Origin {
    /// What the program puts before the path of a file in its app (a dependency's name and `/`).
    fn prefix(&self) -> &str {
        match self {
            Origin::Folder { prefix, .. } | Origin::Package { prefix, .. } => prefix,
        }
    }
}

/// What a debug run knows of the sources of its apps, by the caller's app index (the APPs of the command line first,
/// then the dependency apps).
#[derive(Clone, Default)]
pub struct Sources {
    pub origins: Vec<Option<Origin>>,
    /// How many of `origins` are APPs of the command line (compiled whole); the rest are dependencies, compiled only
    /// where the tests reach them (ADR-013 decision 6).
    pub given: usize,
    /// The folder a file of a package is written to when the debugger stops in it.
    pub written: PathBuf,
}

/// A source file of the program that has a source the debugger can show.
struct File {
    /// Where the client sees it: the file on disk, or for a package the file the debugger writes.
    disk: PathBuf,
    /// For a file of a package: its origin index, the path the program has and its path in the package.
    packaged: Option<(usize, String, String)>,
    /// Its app is a dependency: what no test reaches is not compiled.
    dependency: bool,
}

/// The statements of the program by the source file that holds them.
pub struct Index {
    pub program: Arc<al_ir::Program>,
    /// Per source file of the program (`SrcPos::file`).
    files: Vec<Option<File>>,
    origins: Vec<Option<Origin>>,
    /// The statements by the key of the file's `disk` path.
    by_file: HashMap<String, Vec<Stmt>>,
    /// The source file of the program by the key of its `disk` path.
    file_of: HashMap<String, u32>,
    /// The files of packages: their path in the package (with `/`), and their source file.
    packaged: Vec<(String, u32)>,
    /// Files of packages the client named by another path (a checkout of the same text), and those already written.
    shown: Mutex<HashMap<u32, PathBuf>>,
}

/// How paths compare: as the file system does (case-sensitive on Linux, not on Windows and macOS), after symbolic
/// links and `..` are resolved where the file exists.
pub fn key(p: &Path) -> String {
    let p = std::fs::canonicalize(p)
        .or_else(|_| std::path::absolute(p))
        .unwrap_or_else(|_| p.to_path_buf());
    let text = p.to_string_lossy().replace('\\', "/");
    fold(text)
}

fn fold(text: String) -> String {
    if cfg!(any(windows, target_os = "macos")) {
        text.to_lowercase()
    } else {
        text
    }
}

/// `path` (in a package) below `root`, without the parts that would leave it (`..`, a root, a drive).
fn below(root: &Path, path: &str) -> PathBuf {
    let mut out = root.to_path_buf();
    for part in path.split(['/', '\\']) {
        if part.is_empty() || part == "." || part == ".." || part.contains(':') {
            continue;
        }
        out.push(part);
    }
    out
}

/// A name for a folder: letters, digits, space, `.`, `-` and `_`; anything else becomes `_`.
fn folder_name(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The text as lines compare: no byte order mark, `\n` line ends.
fn normalized(text: &str) -> String {
    text.trim_start_matches('\u{feff}').replace("\r\n", "\n")
}

/// The lines of `text` on which an AL statement starts, by its syntax (a `begin ... end` block is not one).
fn statement_lines(text: &str) -> Vec<u32> {
    use al_syntax::SyntaxKind::*;
    let parsed = al_syntax::parse(text, &Default::default());
    let starts: Vec<usize> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let mut lines = Vec::new();
    for n in parsed.syntax().descendants() {
        if !matches!(
            n.kind(),
            AssignStmt
                | ExprStmt
                | IfStmt
                | CaseStmt
                | WhileStmt
                | RepeatStmt
                | ForStmt
                | ForeachStmt
                | ExitStmt
                | BreakStmt
                | AssertErrorStmt
                | WithStmt
                | ContinueStmt
        ) {
            continue;
        }
        let mut t = n.first_token();
        while let Some(tok) = t.as_ref().filter(|t| t.kind().is_trivia()) {
            t = tok.next_token();
        }
        if let Some(tok) = t {
            let at = usize::from(tok.text_range().start());
            lines.push(starts.partition_point(|&s| s <= at) as u32);
        }
    }
    lines
}

impl Index {
    /// `order`: per app of the program, the caller's index into `sources.origins`.
    pub fn build(program: Arc<al_ir::Program>, order: &[usize], sources: &Sources) -> Index {
        let root = {
            let _ = std::fs::create_dir_all(&sources.written);
            std::fs::canonicalize(&sources.written).unwrap_or_else(|_| sources.written.clone())
        };
        let mut files = Vec::with_capacity(program.sources.len());
        let mut file_of = HashMap::new();
        let mut packaged = Vec::new();
        for (fi, f) in program.sources.iter().enumerate() {
            let caller = order.get(f.app as usize).copied().unwrap_or(usize::MAX);
            let dependency = caller >= sources.given;
            let origin = sources.origins.get(caller).and_then(Option::as_ref);
            let inner = origin
                .and_then(|o| f.path.strip_prefix(o.prefix()))
                .unwrap_or(&f.path)
                .replace('\\', "/");
            let file = match origin {
                Some(Origin::Folder { dir, .. }) => Some(File {
                    disk: below(dir, &inner),
                    packaged: None,
                    dependency,
                }),
                Some(Origin::Package { label, .. }) => {
                    packaged.push((fold(inner.clone()), fi as u32));
                    Some(File {
                        disk: below(&root.join(folder_name(label)), &inner),
                        packaged: Some((caller, f.path.clone(), inner)),
                        dependency,
                    })
                }
                None => None,
            };
            if let Some(file) = &file {
                file_of.insert(key(&file.disk), fi as u32);
            }
            files.push(file);
        }
        let mut by_file: HashMap<String, Vec<Stmt>> = HashMap::new();
        let mut keys: HashMap<u32, String> = HashMap::new();
        let has_source = |oi: usize| {
            let app = program.objects[oi].app as usize;
            order
                .get(app)
                .is_some_and(|&c| sources.origins.get(c).is_some_and(Option::is_some))
        };
        for s in al_testrun::coverage::statements(&program, has_source, |a| a) {
            let Some(p) = program.pos(s.pos) else { continue };
            let Some(Some(file)) = files.get(p.file as usize) else {
                continue;
            };
            let k = keys.entry(p.file).or_insert_with(|| key(&file.disk)).clone();
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
            files,
            origins: sources.origins.clone(),
            by_file,
            file_of,
            packaged,
            shown: Mutex::new(HashMap::new()),
        }
    }

    /// Whether the statement at `pos` has a source the client can be shown (a debugger stops and steps only there).
    pub fn has_source(&self, pos: u32) -> bool {
        self.program
            .pos(pos)
            .and_then(|p| self.files.get(p.file as usize))
            .is_some_and(Option::is_some)
    }

    /// The text of a file of a package.
    fn package_text(&self, file: &File) -> Option<String> {
        let (origin, full, _) = file.packaged.as_ref()?;
        match self.origins.get(*origin)?.as_ref()? {
            Origin::Package { text, .. } => text(full),
            Origin::Folder { .. } => None,
        }
    }

    /// The file the client is shown for the statement at `pos`, with its line and column. A file of a package is
    /// written under [`Sources::written`] the first time (unless the client named a checkout of it).
    pub fn location(&self, pos: u32) -> Option<(PathBuf, u32, u32)> {
        let p = self.program.pos(pos)?;
        let file = self.files.get(p.file as usize)?.as_ref()?;
        if file.packaged.is_none() {
            return Some((file.disk.clone(), p.line, p.col));
        }
        let mut shown = self.shown.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(path) = shown.get(&p.file) {
            return Some((path.clone(), p.line, p.col));
        }
        let text = self.package_text(file)?;
        let same = std::fs::read_to_string(&file.disk).is_ok_and(|t| t == text);
        if !same {
            let dir = file.disk.parent()?;
            std::fs::create_dir_all(dir).ok()?;
            // Written beside and renamed, so a reader never sees half a file.
            let part = file.disk.with_extension(format!("al.{}.part", std::process::id()));
            if let Err(e) = std::fs::write(&part, &text).and_then(|_| std::fs::rename(&part, &file.disk)) {
                eprintln!("al-runner debug: cannot write {}: {e}", file.disk.display());
                let _ = std::fs::remove_file(&part);
                return None;
            }
        }
        shown.insert(p.file, file.disk.clone());
        Some((file.disk.clone(), p.line, p.col))
    }

    /// The source file of the program that the client's `path` names, or why it names none. A path that is not a
    /// source file of the program but ends with the path of a file of a package is that file when its text is the
    /// text that was compiled (a checkout of the package's source).
    fn file_named(&self, path: &str) -> Result<u32, String> {
        let k = key(Path::new(path));
        if let Some(&fi) = self.file_of.get(&k) {
            return Ok(fi);
        }
        let slashed = fold(path.replace('\\', "/"));
        let candidates: Vec<u32> = self
            .packaged
            .iter()
            .filter(|(inner, _)| slashed.ends_with(&format!("/{inner}")) || slashed == *inner)
            .map(|&(_, fi)| fi)
            .collect();
        if candidates.is_empty() {
            return Err(NO_STATEMENT.into());
        }
        let on_disk = match std::fs::read_to_string(path) {
            Ok(t) => normalized(&t),
            Err(e) => return Err(format!("this file could not be read: {e}")),
        };
        let mut differs = None;
        for fi in candidates {
            let Some(file) = self.files[fi as usize].as_ref() else {
                continue;
            };
            match self.package_text(file) {
                Some(t) if normalized(&t) == on_disk => {
                    self.shown
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(fi, PathBuf::from(path));
                    return Ok(fi);
                }
                Some(_) => differs = Some(file.packaged.as_ref().map(|(_, _, p)| p.clone()).unwrap_or_default()),
                None => {}
            }
        }
        Err(match differs {
            Some(inner) => format!("this file's text is not the text that was compiled for {inner}"),
            None => NO_SOURCE_FILE.into(),
        })
    }

    /// The breakpoints `requests` of the file `path`.
    pub fn resolve(&self, path: &str, requests: &[Request]) -> Vec<Resolved> {
        let fi = match self.file_named(path) {
            Ok(fi) => fi,
            Err(why) => {
                return requests
                    .iter()
                    .map(|r| Resolved {
                        verified: false,
                        line: r.line,
                        column: None,
                        targets: Vec::new(),
                        message: Some(why.clone()),
                    })
                    .collect();
            }
        };
        let file = self.files[fi as usize].as_ref().expect("a named file has a source");
        let stmts = self.by_file.get(&key(&file.disk));
        // A dependency is compiled only where the tests reach it: a line without a compiled statement may still hold
        // one. Its syntax tells which.
        let mut syntax_lines: Option<Vec<u32>> = None;
        let mut reason = |line: u32| -> String {
            if !file.dependency {
                return NO_STATEMENT.into();
            }
            let lines = syntax_lines.get_or_insert_with(|| {
                let text = match &file.packaged {
                    Some(_) => self.package_text(file),
                    None => std::fs::read_to_string(&file.disk).ok(),
                };
                text.map(|t| statement_lines(&t)).unwrap_or_default()
            });
            if lines.contains(&line) {
                NOT_REACHED.into()
            } else {
                NO_STATEMENT.into()
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
                        message: Some(reason(r.line)),
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
            message: Some(reason.into()),
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
        let sources = Sources {
            origins: vec![Some(Origin::Folder {
                dir: dir.path().to_path_buf(),
                prefix: String::new(),
            })],
            given: 1,
            written: dir.path().join("written"),
        };
        let index = Index::build(lowered.program.clone(), &[0], &sources);
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
        assert_eq!(
            (r[1].verified, r[1].message.as_deref()),
            (false, Some(NO_STATEMENT)),
            "{r:?}"
        );
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
        assert_eq!((r[0].verified, r[0].message.as_deref()), (false, Some(NO_STATEMENT)));
    }

    /// A path in a package never leaves the folder it is written to.
    #[test]
    fn a_path_in_a_package_stays_below_its_folder() {
        let root = Path::new("/c/debug-sources/App");
        assert_eq!(below(root, "src/A.al"), root.join("src").join("A.al"));
        assert_eq!(below(root, "../../x/A.al"), root.join("x").join("A.al"));
        assert_eq!(below(root, "C:\\w\\A.al"), root.join("w").join("A.al"));
        assert_eq!(below(root, "/etc/./A.al"), root.join("etc").join("A.al"));
    }

    /// The syntax tells which lines start a statement; a `begin` or a brace does not.
    #[test]
    fn statement_lines_are_the_lines_where_a_statement_starts() {
        let mut lines = statement_lines(SRC);
        lines.sort_unstable();
        lines.dedup();
        assert_eq!(lines, vec![7, 9]);
    }
}
