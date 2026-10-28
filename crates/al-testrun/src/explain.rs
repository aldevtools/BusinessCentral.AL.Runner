//! `--explain TEST`: why one test ended as it did, with the routine and source position where a
//! refusal or a failure was raised (`Program::positions`, docs/architecture.md "Source positions").

use crate::Outcome;
use al_ir as ir;
use al_vm::FaultFrame;

/// One routine, named and located.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackLine {
    /// `codeunit 50100 "Name".Routine`
    pub who: String,
    /// `app: path:line:col`, with a note when the position is not a statement's.
    pub at: String,
}

/// An AL error as the runtime raised it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorInfo {
    pub message: String,
    pub code: String,
    pub call_stack: String,
}

/// What the runtime knows about one test beyond its outcome.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestDetail {
    pub error: Option<ErrorInfo>,
    /// The routines the failure or the refusal left, innermost first. Each shows the statement it
    /// was running.
    pub raised_in: Vec<StackLine>,
    /// A refusal decided before the run (a lowering reason): the routine that carries it. The
    /// reason has no statement position of its own, so this is the routine header.
    pub refused_in: Option<StackLine>,
}

/// `object kind id "name".routine`, located at the statement `pos` (`NO_POS`: the routine header).
pub fn describe(program: &ir::Program, object: usize, routine: usize, pos: u32) -> StackLine {
    describe_at(program, object, routine, pos, None)
}

/// The routine that carries a refusal decided before the run, located at the statement the refusal came from when the
/// lowering knows it (`Lowered::refusal_at`), else at the routine header.
pub fn describe_refused(lowered: &al_sema::lower::Lowered, object: usize, routine: usize) -> StackLine {
    let at = lowered
        .refusal_at
        .get(object)
        .and_then(|r| r.get(routine))
        .and_then(Option::as_ref);
    describe_at(&lowered.program, object, routine, ir::NO_POS, at)
}

fn describe_at(
    program: &ir::Program,
    object: usize,
    routine: usize,
    pos: u32,
    refused_at: Option<&ir::SrcPos>,
) -> StackLine {
    let obj = &program.objects[object];
    let r = obj.routines.get(routine);
    let who = format!(
        "{} {} \"{}\".{}{}",
        obj.kind,
        obj.id.map_or(String::from("?"), |i| i.to_string()),
        obj.name,
        r.map_or("?", |r| r.name.as_str()),
        if r.is_some_and(|r| r.is_trigger) {
            " (trigger)"
        } else {
            ""
        }
    );
    let place = |p: &ir::SrcPos, note: &str| {
        let src = program.sources.get(p.file as usize);
        let app = src
            .and_then(|s| program.apps.get(s.app as usize))
            .map_or("?", |a| a.name.as_str());
        format!(
            "{app}: {}:{}:{}{note}",
            src.map_or("?", |s| s.path.as_str()),
            p.line,
            p.col
        )
    };
    let at = if let Some(p) = refused_at {
        place(p, " (the statement the refusal came from)")
    } else if let Some(p) = program.pos(pos) {
        place(p, "")
    } else if let Some(p) = r.and_then(|r| program.pos(r.pos)) {
        place(p, " (routine header; no statement position)")
    } else {
        "no source position (a routine the compiler made up)".to_string()
    };
    StackLine { who, at }
}

/// The detail of a test that has just ended: `site` is the runtime's record of where the error or
/// the refusal left from (outermost first).
pub fn detail(program: &ir::Program, site: &[FaultFrame], error: Option<ErrorInfo>) -> TestDetail {
    TestDetail {
        error,
        raised_in: site
            .iter()
            .rev()
            .map(|f| describe(program, f.object, f.routine, f.pos))
            .collect(),
        refused_in: None,
    }
}

/// The text `--explain` prints.
pub fn render(head: &str, outcome: &Outcome, oracle: &str, detail: Option<&TestDetail>) -> String {
    render_with(head, Some(oracle), outcome, detail)
}

/// The same for a run that has no oracle (`al-runner test --explain`).
pub fn render_without_oracle(head: &str, outcome: &Outcome, detail: Option<&TestDetail>) -> String {
    render_with(head, None, outcome, detail)
}

fn render_with(head: &str, oracle: Option<&str>, outcome: &Outcome, detail: Option<&TestDetail>) -> String {
    let mut s = match oracle {
        Some(o) => format!("{head}\nreal BC (oracle): {o}\n"),
        None => format!("{head}\n"),
    };
    match outcome {
        Outcome::Pass => s.push_str("runtime: PASS\n"),
        Outcome::NotRun(why) => {
            s.push_str(&format!("runtime: NOT RUN (refused)\nreason: {why}\n"));
            match detail {
                Some(TestDetail {
                    refused_in: Some(l), ..
                }) => s.push_str(&format!("refused before the run, in {}\n  at {}\n", l.who, l.at)),
                Some(d) if !d.raised_in.is_empty() => {
                    s.push_str("refused while running, in:\n");
                    push_lines(&mut s, &d.raised_in);
                }
                _ => s.push_str("where: not recorded (the refusal came before any AL routine ran)\n"),
            }
        }
        Outcome::Fail(why) => {
            s.push_str(&format!("runtime: FAIL\nerror: {why}\n"));
            if let Some(e) = detail.and_then(|d| d.error.as_ref()) {
                s.push_str(&format!(
                    "code: {}\n",
                    if e.code.is_empty() { "(none)" } else { &e.code }
                ));
                if !e.call_stack.is_empty() {
                    s.push_str("AL call stack (GetLastErrorCallStack form):\n");
                    for l in e.call_stack.lines() {
                        s.push_str(&format!("  {l}\n"));
                    }
                }
            }
            match detail {
                Some(d) if !d.raised_in.is_empty() => {
                    s.push_str("raised in (innermost first):\n");
                    push_lines(&mut s, &d.raised_in);
                }
                _ => s.push_str("where: not recorded\n"),
            }
        }
    }
    s
}

fn push_lines(s: &mut String, lines: &[StackLine]) {
    for l in lines {
        s.push_str(&format!("  {}\n    at {}\n", l.who, l.at));
    }
}
