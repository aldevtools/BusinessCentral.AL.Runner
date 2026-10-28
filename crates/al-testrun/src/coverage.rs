//! Statement coverage for `al-runner test --coverage`: the statements of the workspace's apps, and a per-thread sink in
//! which a test codeunit's run leaves what each of its tests executed.
//!
//! A statement is a `Stmt::Pos` marker of the lowered program (`ir::Program::positions`); the VM counts how often each
//! runs (`Session::enable_coverage`). The universe of statements is every marker of a routine of an object that does not
//! belong to a dependency app, so statements no test reached show with 0 runs.

use al_ir::{Program, Stmt};
use std::cell::RefCell;

/// One statement of a workspace app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageStatement {
    /// The position id: `Stmt::Pos(pos)`, an index into `Program::positions`.
    pub pos: u32,
    /// Index of the app (as given to the run) whose source file holds the statement.
    pub app: usize,
    /// The file's path inside its app.
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    /// The procedure or trigger the statement is in.
    pub scope: String,
    /// The object's name.
    pub object: String,
}

/// The statements a test ran: (position id, runs).
pub type Hits = Vec<(u32, u32)>;

thread_local! {
    static SINK: RefCell<Option<Vec<(String, Hits)>>> = const { RefCell::new(None) };
}

/// Starts collecting on this thread (a test codeunit run).
pub(crate) fn begin() {
    SINK.with(|s| *s.borrow_mut() = Some(Vec::new()));
}

pub(crate) fn enabled() -> bool {
    SINK.with(|s| s.borrow().is_some())
}

/// Records what test `name` ran.
pub(crate) fn push(name: &str, hits: Hits) {
    SINK.with(|s| {
        if let Some(v) = s.borrow_mut().as_mut() {
            v.push((name.to_string(), hits));
        }
    });
}

/// Stops collecting for a while (a second run of a codeunit that only checks its outcomes): what was collected, for
/// [`resume`].
pub(crate) fn suspend() -> Option<Vec<(String, Hits)>> {
    SINK.with(|s| s.borrow_mut().take())
}

/// Collects again what [`suspend`] stopped.
pub(crate) fn resume(kept: Option<Vec<(String, Hits)>>) {
    SINK.with(|s| *s.borrow_mut() = kept);
}

/// Stops collecting and returns what was collected.
pub(crate) fn finish() -> Vec<(String, Hits)> {
    SINK.with(|s| s.borrow_mut().take()).unwrap_or_default()
}

fn walk(stmts: &[Stmt], out: &mut Vec<u32>) {
    for s in stmts {
        match s {
            Stmt::Pos(p) => out.push(*p),
            Stmt::If(_, a, b) => {
                walk(a, out);
                walk(b, out);
            }
            Stmt::While(_, b) | Stmt::Repeat(b, _) | Stmt::AssertError(b) => walk(b, out),
            Stmt::For { body, .. } | Stmt::Foreach { body, .. } => walk(body, out),
            Stmt::Case {
                branches, otherwise, ..
            } => {
                for (_, b) in branches {
                    walk(b, out);
                }
                walk(otherwise, out);
            }
            _ => {}
        }
    }
}

/// Every statement of the objects `keep` accepts (index into `Program::objects`), in program order. `app_of` maps an app
/// index of the program to the caller's.
pub fn statements(
    program: &Program,
    keep: impl Fn(usize) -> bool,
    app_of: impl Fn(usize) -> usize,
) -> Vec<CoverageStatement> {
    let mut out = Vec::new();
    for (_, o) in program.objects.iter().enumerate().filter(|(oi, _)| keep(*oi)) {
        for r in &o.routines {
            let mut ids = Vec::new();
            walk(&r.body, &mut ids);
            for id in ids {
                let Some(p) = program.pos(id) else { continue };
                let Some(f) = program.sources.get(p.file as usize) else {
                    continue;
                };
                out.push(CoverageStatement {
                    pos: id,
                    app: app_of(f.app as usize),
                    file: f.path.clone(),
                    line: p.line,
                    column: p.col,
                    end_line: p.end_line,
                    scope: r.name.clone(),
                    object: o.name.clone(),
                });
            }
        }
    }
    out
}
