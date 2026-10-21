//! The debugger seam (ADR-048): a session with a debugger runs every routine in its debug form, which calls the
//! session's [`DebugHook`] before each statement. The hook decides whether to stop there and, when it stops, blocks
//! until its client resumes. Nothing here touches the OS (ADR-006): blocking is the hook's business, for example a
//! channel receive in the command line's implementation.
//!
//! Without a debugger the session runs the plain form, which has no `Op::DbgHit`: a run without one pays nothing.

use crate::capture::Rendered;
use crate::{Form, Session, Value, ir};

/// What the debugged statement does once the hook returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Resume {
    /// Run the statement.
    Continue,
}

/// The debugger of a session (ADR-048 decision 2).
pub trait DebugHook: Send {
    /// Called before each statement of the debug form, with a read-only view of where the session is. Returns at once
    /// to run on, or blocks until the client resumes.
    fn statement(&mut self, view: &DebugView<'_>) -> Resume;
}

/// A frame of the AL call stack as a debugger shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct DebugFrame {
    /// The object (index into `Program::objects`) and its routine (index into `Object::routines`).
    pub object: usize,
    pub routine: usize,
    /// The `Program::positions` id of the statement the frame is at; `ir::NO_POS` before its first statement.
    pub pos: u32,
    /// The routine's parameters and locals by name (a named return value included), as the value renderer of ADR-030
    /// renders them. The innermost frame's are read when the view is asked; an outer frame's are those it had when its
    /// current statement began (the statement that made the call below it).
    pub locals: Vec<(String, Rendered)>,
}

/// The session at a statement of the debug form.
pub struct DebugView<'a> {
    session: &'a Session,
    pos: u32,
    top: &'a [Value],
    /// Per frame of the call stack below the innermost, its locals rendered at its last statement.
    outer: &'a [Option<Vec<Rendered>>],
}

impl DebugView<'_> {
    /// The `Program::positions` id of the statement about to run.
    pub fn pos(&self) -> u32 {
        self.pos
    }

    /// The depth of the AL call stack: 1 in the test method itself, one more per call.
    pub fn depth(&self) -> usize {
        self.session.call_stack.len()
    }

    pub fn program(&self) -> &ir::Program {
        &self.session.engine.program
    }

    /// The AL call stack, innermost frame first.
    pub fn frames(&self) -> Vec<DebugFrame> {
        let program = &self.session.engine.program;
        let n = self.session.call_stack.len();
        let mut out = Vec::with_capacity(n);
        for i in (0..n).rev() {
            let frame = self.session.call_stack[i];
            let object = self.session.frames.get(i).copied().unwrap_or(usize::MAX);
            let names: Vec<&str> = program
                .objects
                .get(object)
                .and_then(|o| o.routines.get(frame.routine))
                .map(|r| r.locals.iter().map(|l| l.name.as_str()).collect())
                .unwrap_or_default();
            let values: Vec<Rendered> = if i + 1 == n {
                self.top.iter().map(|v| self.session.render(v)).collect()
            } else {
                self.outer.get(i).cloned().flatten().unwrap_or_default()
            };
            let locals = names
                .iter()
                .zip(values)
                .filter(|(name, _)| !name.is_empty())
                .map(|(name, v)| (name.to_string(), v))
                .collect();
            out.push(DebugFrame {
                object,
                routine: frame.routine,
                pos: if i + 1 == n { self.pos } else { frame.pos },
                locals,
            });
        }
        out
    }
}

/// A session's debugger and what it keeps of the frames below the running one.
pub(crate) struct Debugger {
    hook: Box<dyn DebugHook>,
    /// Per frame of the call stack, its locals rendered when its last statement began: a frame below the innermost
    /// one is not running, and these are its values (ADR-048 decision 8). Rendered, not cloned, so the debugger holds
    /// no reference to a value the code under test owns.
    outer: Vec<Option<Vec<Rendered>>>,
}

impl Session {
    /// Attaches a debugger (ADR-048): from the next call on, every routine runs in its debug form and `hook` sees each
    /// statement. Value capture is not combined with it.
    pub fn attach_debugger(&mut self, hook: Box<dyn DebugHook>) {
        self.debug = Some(Box::new(Debugger {
            hook,
            outer: Vec::new(),
        }));
        if self.form == Form::Plain {
            self.form = Form::Debug;
        }
    }

    /// Detaches the debugger: the routines called from then on run in their plain form.
    pub fn detach_debugger(&mut self) {
        self.debug = None;
        if self.form == Form::Debug {
            self.form = Form::Plain;
        }
    }

    /// The form a routine runs in when no capture tracks it.
    pub(crate) fn plain_form(&self) -> Form {
        if self.debug.is_some() { Form::Debug } else { Form::Plain }
    }

    /// `Op::DbgHit`: the statement at position `p` is about to run, in a frame whose named locals are `locals`.
    /// Cold and never inlined: the op loop's other arms must not pay for this arm (no debugger: never reached).
    #[cold]
    #[inline(never)]
    pub(crate) fn debug_hit(&mut self, p: u32, locals: &[Value]) {
        let Some(mut d) = self.debug.take() else { return };
        let depth = self.call_stack.len();
        d.outer.truncate(depth);
        d.outer.resize(depth, None);
        let resume = {
            let view = DebugView {
                session: self,
                pos: p,
                top: locals,
                outer: &d.outer,
            };
            d.hook.statement(&view)
        };
        match resume {
            Resume::Continue => {}
        }
        // This frame is an outer frame for whatever its statement calls.
        if let Some(slot) = depth.checked_sub(1).and_then(|i| d.outer.get_mut(i)) {
            *slot = Some(locals.iter().map(|v| self.render(v)).collect());
        }
        self.debug = Some(d);
    }
}
