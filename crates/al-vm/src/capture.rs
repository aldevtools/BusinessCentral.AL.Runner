//! Value capture and loop tracking (ADR-030): what a scratch run or a test did, seen from the statements.
//!
//! Off, a session pays nothing for it: the statements of a block run in the loop of `exec_block` as ever, and every hook
//! here starts with a test of `Session::capture`. On (`Session::enable_capture`), `exec_block` runs a block through
//! `exec_block_captured`, which tells this module when a statement started and when it ended.
//!
//! * **Values** (`CaptureConfig::values`). After each statement of a routine that ends without an error, every local of
//!   the routine is rendered and compared with what was last seen of it. A local that changed gets a record with the
//!   value and the id of the statement (the position id of its `Stmt::Pos`, the id `coverage` uses). A local that the
//!   statement writes by its shape (the target of `:=`, the receiver of a method called as a statement, a `var`
//!   argument) gets one when its value did not change too: a record per execution, not per change. A parameter's value
//!   at entry is the baseline and is no record; a local never written is none.
//! * **Loops** (`CaptureConfig::loops`). The interpreter knows when a loop starts, when a pass starts and how it ends, so
//!   the passes are counted, not inferred. A loop instance has an id (unique within the capture), the statement of the
//!   loop, the loop and pass that were active when it started (across calls), the statements of its own routine that
//!   each pass ran (nested loops' included), and how it ended. A record of a value or a message produced inside a loop
//!   names the innermost active loop (across calls) and its pass.

use crate::{Frame, Session};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_ir as ir;

/// What to capture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CaptureConfig {
    /// The values of the locals, statement by statement.
    pub values: bool,
    /// The loops and their passes.
    pub loops: bool,
}

impl CaptureConfig {
    pub fn any(self) -> bool {
        self.values || self.loops
    }
}

/// A value as JSON holds it.
#[derive(Debug, Clone, PartialEq)]
pub enum Rendered {
    Null,
    Bool(bool),
    Int(i64),
    /// A decimal number, as digits.
    Num(String),
    Text(String),
    /// The value is of a kind that is not rendered; the text says which.
    Unrendered(String),
}

/// A local that was written, with the value it then had.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueRecord {
    /// The routine (procedure or trigger) the local belongs to.
    pub scope: String,
    pub variable: String,
    pub value: Rendered,
    /// The statement: its position id.
    pub statement: u32,
    /// The innermost loop and pass that were active, if any.
    pub loop_id: Option<u32>,
    pub iteration: Option<u32>,
}

/// How a loop instance ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedBy {
    /// The statement after the loop ran (the loop ended, also by `break`).
    Exit,
    /// The routine returned or `exit`ed from inside the loop.
    ScopeExit,
    /// An error went through the loop, or the run ended.
    Unfinished,
}

/// One execution of a loop statement.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopRecord {
    pub id: u32,
    pub scope: String,
    /// The loop statement: its position id.
    pub statement: u32,
    /// The loop instance and pass active when this one started.
    pub parent: Option<u32>,
    pub parent_iteration: Option<u32>,
    /// The statements each pass ran in this loop's own routine, nested loops' included: position ids in order of first
    /// run, one list per pass.
    pub iterations: Vec<Vec<u32>>,
    pub closed_by: ClosedBy,
}

/// A `Message` of a scratch run.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageRecord {
    pub text: String,
    pub scope: String,
    pub statement: Option<u32>,
    pub loop_id: Option<u32>,
    pub iteration: Option<u32>,
}

/// The most value records and messages a capture keeps; a loop of a million passes must not fill the memory. Past it the
/// records are not kept and `Captured::truncated` says so.
pub const MAX_RECORDS: usize = 200_000;

/// What a capture collected.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Captured {
    /// Records were dropped because `MAX_RECORDS` was reached.
    pub truncated: bool,
    pub values: Vec<ValueRecord>,
    pub loops: Vec<LoopRecord>,
    pub messages: Vec<MessageRecord>,
}

struct CapFrame {
    /// The routine belongs to an app of the run, not to a dependency: only those are captured.
    tracked: bool,
    scope: String,
    /// What was last seen of each local.
    last: Vec<Rendered>,
    /// The loop instances of this routine's execution that are open, outermost first.
    loops: Vec<u32>,
    /// `active.len()` when the frame started.
    active_at_start: usize,
    /// `stmt.len()` when the frame started: what an `exit` or an error leaves open above it is dropped with the frame.
    stmt_base: usize,
}

pub(crate) struct Capture {
    cfg: CaptureConfig,
    out: Captured,
    frames: Vec<CapFrame>,
    /// The loop instances open now, across calls, outermost first.
    active: Vec<u32>,
    /// The statements being executed, innermost last.
    stmt: Vec<u32>,
    open: Vec<bool>,
}

impl Capture {
    /// Whether the routine running now is captured.
    pub(crate) fn tracking(&self) -> bool {
        self.frames.last().is_some_and(|f| f.tracked)
    }

    fn innermost(&self) -> (Option<u32>, Option<u32>) {
        match self.active.last() {
            Some(&id) => (Some(id), Some(self.out.loops[id as usize].iterations.len() as u32)),
            None => (None, None),
        }
    }

    fn close(&mut self, id: u32, by: ClosedBy) {
        if std::mem::replace(&mut self.open[id as usize], false) {
            self.out.loops[id as usize].closed_by = by;
        }
    }
}

/// A local the compiler made up (a temporary), which no author wrote.
fn synthetic(name: &str) -> bool {
    name.is_empty() || name.starts_with("__") || name.starts_with('$')
}

fn root_local(p: &ir::Place) -> Option<usize> {
    match p {
        ir::Place::Local(i) => Some(*i),
        ir::Place::Index(b, _) | ir::Place::Field(b, _) => root_local(b),
        ir::Place::Global(_) | ir::Place::BaseGlobal(_) => None,
    }
}

fn arg_writes(args: &[ir::Arg], out: &mut Vec<usize>) {
    for a in args {
        if let ir::Arg::Ref(p) = a {
            out.extend(root_local(p));
        }
    }
}

/// The locals a statement writes by its shape: the target of an assignment, the receiver of a method called as a
/// statement (`Rec.Insert()`, `List.Add(1)`), and the `var` arguments of a call (`Clear(x)`, `Evaluate(x, s)`, `Fill(x)`).
pub(crate) fn written_locals(s: &ir::Stmt) -> Vec<usize> {
    let mut out = Vec::new();
    match s {
        ir::Stmt::Assign(p, _) => out.extend(root_local(p)),
        ir::Stmt::Expr(e) => match e {
            ir::Expr::Builtin { receiver, args, .. } => {
                match receiver.as_deref() {
                    Some(ir::Arg::Ref(p)) => out.extend(root_local(p)),
                    Some(ir::Arg::Value(ir::Expr::Load(p))) => out.extend(root_local(p)),
                    _ => {}
                }
                arg_writes(args, &mut out);
            }
            ir::Expr::Call { args, .. } | ir::Expr::InterfaceCall { args, .. } => arg_writes(args, &mut out),
            ir::Expr::DotNet { args, receiver, .. } => {
                if let Some(ir::Expr::Load(p)) = receiver.as_deref() {
                    out.extend(root_local(p));
                }
                arg_writes(args, &mut out);
            }
            _ => {}
        },
        _ => {}
    }
    out
}

impl Session {
    /// Turns capture on. `Session::take_capture` hands over what was collected.
    pub fn enable_capture(&mut self, cfg: CaptureConfig) {
        self.form = self.plain_form();
        self.capture = cfg.any().then(|| {
            Box::new(Capture {
                cfg,
                out: Captured::default(),
                frames: Vec::new(),
                active: Vec::new(),
                stmt: Vec::new(),
                open: Vec::new(),
            })
        });
    }

    /// What was captured since the last call; the capture goes on empty. Loops that are still open are unfinished.
    pub fn take_capture(&mut self) -> Captured {
        let Some(c) = self.capture.as_mut() else {
            return Captured::default();
        };
        for id in 0..c.open.len() {
            c.close(id as u32, ClosedBy::Unfinished);
        }
        c.frames.clear();
        c.active.clear();
        c.stmt.clear();
        c.open.clear();
        self.form = self.plain_form();
        let c = self.capture.as_mut().expect("checked above");
        std::mem::take(&mut c.out)
    }

    pub(crate) fn render(&self, v: &Value) -> Rendered {
        use al_builtins::format::format_value;
        let text = |v: &Value, standard: i32| match format_value(v, 0, standard, self) {
            Ok(t) => Rendered::Text(t),
            Err(_) => Rendered::Unrendered(format!("{} has no format here yet", v.describe_kind())),
        };
        match v {
            Value::Null => Rendered::Null,
            Value::Boolean(b) => Rendered::Bool(*b),
            Value::Integer(i) => Rendered::Int(i64::from(*i)),
            Value::BigInteger(i) => Rendered::Int(*i),
            Value::Byte(b) => Rendered::Int(i64::from(*b)),
            Value::Char(c) => Rendered::Int(i64::from(*c)),
            Value::Decimal(d) => Rendered::Num(d.normalize().to_string()),
            Value::Text(t) | Value::Code(t) => Rendered::Text(t.to_string()),
            Value::Date(..) | Value::Time(_) | Value::DateTime(_) | Value::Duration(_) | Value::Guid(_) => text(v, 9),
            Value::DateFormula(s) => Rendered::Text(s.clone()),
            Value::Option(..) | Value::Enum(..) => text(v, 0),
            Value::Record(rc) => {
                let Ok(st) = rc.try_borrow() else {
                    return Rendered::Unrendered("a record in use".into());
                };
                let program = &self.engine.program;
                let key: Option<Vec<usize>> = program
                    .objects
                    .get(st.table as usize)
                    .and_then(|o| o.table.as_ref())
                    .and_then(|t| t.keys.first().cloned());
                match key {
                    Some(fields) => {
                        let parts: Vec<String> = fields
                            .iter()
                            .map(|&f| match st.fields.get(f) {
                                Some(v) => format_value(v, 0, 9, self).unwrap_or_default(),
                                None => String::new(),
                            })
                            .collect();
                        Rendered::Text(parts.join(", "))
                    }
                    None => Rendered::Unrendered("a record without a primary key".into()),
                }
            }
            other => Rendered::Unrendered(format!("{} is not rendered yet", other.describe_kind())),
        }
    }

    /// A routine starts: the locals it starts with are the baseline of what its statements write.
    #[inline(never)]
    pub(crate) fn cap_enter_frame(&mut self, object: usize, r: &ir::Routine, locals: &[Value]) {
        if self.capture.is_none() {
            return;
        }
        let tracked = self
            .engine
            .program
            .objects
            .get(object)
            .and_then(|o| self.engine.program.apps.get(o.app as usize))
            .is_some_and(|a| !a.dependency);
        let last: Vec<Rendered> = if tracked && self.capture.as_ref().is_some_and(|c| c.cfg.values) {
            locals.iter().map(|v| self.render(v)).collect()
        } else {
            Vec::new()
        };
        let Some(c) = self.capture.as_mut() else { return };
        let active_at_start = c.active.len();
        let stmt_base = c.stmt.len();
        self.form = if tracked {
            crate::Form::Captured
        } else {
            self.plain_form()
        };
        let Some(c) = self.capture.as_mut() else { return };
        c.frames.push(CapFrame {
            tracked,
            scope: r.name.clone(),
            last,
            loops: Vec::new(),
            active_at_start,
            stmt_base,
        });
    }

    /// A routine ends. The loops of it that are open end with it.
    #[inline(never)]
    pub(crate) fn cap_leave_frame(&mut self, outcome: &Result<(), Signal>) {
        let Some(c) = self.capture.as_mut() else { return };
        let Some(fr) = c.frames.pop() else { return };
        let by = match outcome {
            Err(Signal::Exit(_)) => ClosedBy::ScopeExit,
            Ok(()) | Err(Signal::Break | Signal::Continue) => ClosedBy::ScopeExit,
            Err(_) => ClosedBy::Unfinished,
        };
        for id in fr.loops {
            c.close(id, by);
        }
        c.active.truncate(fr.active_at_start);
        c.stmt.truncate(fr.stmt_base);
        let tracked = c.frames.last().is_some_and(|f| f.tracked);
        self.form = if tracked {
            crate::Form::Captured
        } else {
            self.plain_form()
        };
    }

    /// A statement marker ran: the statement is one of the open loops' pass.
    pub(crate) fn cap_stmt_hit(&mut self, p: u32) {
        let Some(c) = self.capture.as_mut() else { return };
        if !c.cfg.loops {
            return;
        }
        let Some(fr) = c.frames.last().filter(|f| f.tracked) else {
            return;
        };
        for &id in &fr.loops {
            if let Some(pass) = c.out.loops[id as usize].iterations.last_mut() {
                if !pass.contains(&p) {
                    pass.push(p);
                }
            }
        }
    }

    pub(crate) fn cap_push_stmt(&mut self, p: u32) {
        if let Some(c) = self.capture.as_mut() {
            c.stmt.push(p);
        }
    }

    pub(crate) fn cap_pop_stmt(&mut self) {
        if let Some(c) = self.capture.as_mut() {
            c.stmt.pop();
        }
    }

    /// Statement `s` (marked `pos`) ended without an error: the locals of the routine it changed or wrote are recorded.
    /// The depth of the open statements (an `asserterror` that catches an error drops what the error left open).
    pub(crate) fn cap_stmt_depth(&self) -> usize {
        self.capture.as_ref().map_or(0, |c| c.stmt.len())
    }

    pub(crate) fn cap_stmt_truncate(&mut self, n: usize) {
        if let Some(c) = self.capture.as_mut() {
            c.stmt.truncate(n);
        }
    }

    /// [`Session::cap_after`] with the locals the statement writes by its shape already worked out (the bytecode does it
    /// once, at compile time).
    pub(crate) fn cap_after_writes(&mut self, pos: u32, writes: &[usize], f: &Frame) {
        if !self.capture.as_ref().is_some_and(|c| c.cfg.values && c.tracking()) {
            return;
        }
        let engine = self.engine.clone();
        let r = &engine.program.objects[f.object].routines[f.routine];
        let seen: Vec<(usize, Rendered)> = f
            .locals
            .iter()
            .enumerate()
            .filter(|(i, _)| r.locals.get(*i).is_some_and(|slot| !synthetic(&slot.name)))
            .map(|(i, v)| (i, self.render(v)))
            .collect();
        let Some(c) = self.capture.as_mut() else { return };
        let (loop_id, iteration) = c.innermost();
        let Some(fr) = c.frames.last_mut() else { return };
        let mut records = Vec::new();
        for (i, now) in seen {
            if c.out.values.len() + records.len() >= MAX_RECORDS {
                c.out.truncated = true;
                break;
            }
            if fr.last.get(i) != Some(&now) || writes.contains(&i) {
                if let Some(slot) = fr.last.get_mut(i) {
                    *slot = now.clone();
                }
                records.push(ValueRecord {
                    scope: fr.scope.clone(),
                    variable: r.locals[i].name.clone(),
                    value: now,
                    statement: pos,
                    loop_id,
                    iteration,
                });
            }
        }
        c.out.values.extend(records);
    }

    /// A loop statement starts. `None`: nothing is tracked.
    #[inline(never)]
    pub(crate) fn cap_loop_enter(&mut self) -> Option<u32> {
        let c = self.capture.as_mut()?;
        if !c.cfg.loops {
            return None;
        }
        let statement = *c.stmt.last()?;
        let fr = c.frames.last_mut().filter(|f| f.tracked)?;
        let id = c.out.loops.len() as u32;
        let (parent, parent_iteration) = match c.active.last() {
            Some(&p) => (Some(p), {
                let n = c.out.loops[p as usize].iterations.len() as u32;
                (n > 0).then_some(n)
            }),
            None => (None, None),
        };
        c.out.loops.push(LoopRecord {
            id,
            scope: fr.scope.clone(),
            statement,
            parent,
            parent_iteration,
            iterations: Vec::new(),
            closed_by: ClosedBy::Unfinished,
        });
        c.open.push(true);
        fr.loops.push(id);
        c.active.push(id);
        Some(id)
    }

    /// A pass of the loop starts.
    #[inline(never)]
    pub(crate) fn cap_loop_pass(&mut self, id: Option<u32>) {
        let (Some(id), Some(c)) = (id, self.capture.as_mut()) else {
            return;
        };
        c.out.loops[id as usize].iterations.push(Vec::new());
    }

    /// The loop ends (its statement completes).
    #[inline(never)]
    pub(crate) fn cap_loop_exit(&mut self, id: Option<u32>) {
        let (Some(id), Some(c)) = (id, self.capture.as_mut()) else {
            return;
        };
        c.close(id, ClosedBy::Exit);
        if let Some(fr) = c.frames.last_mut() {
            fr.loops.retain(|&l| l != id);
        }
        c.active.retain(|&l| l != id);
    }

    /// The variable of a `for` or `foreach` was given the value of the pass that starts: a record for the loop statement.
    #[inline(never)]
    pub(crate) fn cap_loop_var(&mut self, var: &ir::Place, f: &Frame) {
        if !self.capture.as_ref().is_some_and(|c| c.cfg.values && c.tracking()) {
            return;
        }
        let Some(i) = root_local(var) else { return };
        let engine = self.engine.clone();
        let r = &engine.program.objects[f.object].routines[f.routine];
        let Some(slot) = r.locals.get(i).filter(|s| !synthetic(&s.name)) else {
            return;
        };
        let Some(v) = f.locals.get(i) else { return };
        let now = self.render(v);
        let Some(c) = self.capture.as_mut() else { return };
        let (loop_id, iteration) = c.innermost();
        let Some(statement) = c.stmt.last().copied() else {
            return;
        };
        let Some(fr) = c.frames.last_mut() else { return };
        if let Some(last) = fr.last.get_mut(i) {
            *last = now.clone();
        }
        if c.out.values.len() >= MAX_RECORDS {
            c.out.truncated = true;
            return;
        }
        c.out.values.push(ValueRecord {
            scope: fr.scope.clone(),
            variable: slot.name.clone(),
            value: now,
            statement,
            loop_id,
            iteration,
        });
    }

    /// A `Message` of a scratch run.
    pub(crate) fn cap_message(&mut self, text: &str) {
        let Some(c) = self.capture.as_mut() else { return };
        let (loop_id, iteration) = c.innermost();
        let scope = c.frames.last().map(|f| f.scope.clone()).unwrap_or_default();
        let statement = c.stmt.last().copied();
        if c.out.messages.len() >= MAX_RECORDS {
            c.out.truncated = true;
            return;
        }
        c.out.messages.push(MessageRecord {
            text: text.to_string(),
            scope,
            statement,
            loop_id,
            iteration,
        });
    }
}

/// A short name of the kind of a value, for the text of a capture error.
trait Kind {
    fn describe_kind(&self) -> &'static str;
}

impl Kind for Value {
    fn describe_kind(&self) -> &'static str {
        match self {
            Value::List(_) => "a List",
            Value::Dictionary(_) => "a Dictionary",
            Value::Array(_) => "an array",
            Value::Codeunit(_) => "a codeunit variable",
            Value::Library(..) => "a library value",
            Value::Blob(_) => "a Blob",
            Value::TestPage(_) | Value::TestControl(_) => "a test page",
            Value::Query(_) => "a Query",
            Value::DotNet(_) => "a DotNet value",
            _ => "a value",
        }
    }
}
