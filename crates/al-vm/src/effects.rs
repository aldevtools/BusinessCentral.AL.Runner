//! Whether running a routine can leave something behind that AL code reads later (al-runner issue 5870).
//!
//! A page open runs the CaptionClass expressions of its source table with an empty record a number of times BC does not
//! give stably (8 per Card OpenEdit on one BC 29 container in issue 5870; 240, 480 and 492 calls for one Card open in the
//! corpus, record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:657). The number can only matter when a run leaves a
//! trace: a write to a variable that outlives the call, a database write, a dialog, an error. [`Engine::lasting_effect`]
//! answers that from the code alone, before anything runs.
//!
//! The answer is conservative: "no effect" only when every statement the call can reach is known to leave nothing
//! behind. Anything else (an unknown builtin, a .NET call, an interface call, a write to a SingleInstance codeunit, a
//! subscriber that does any of these) is an effect, and its description says which.
//!
//! What dies with the call: the routine's locals, and the variables of the instance it runs on when that instance is
//! *transient*: the empty record of the page open, a record or codeunit held in a local or in a variable of a
//! transient instance, and a fresh instance of a codeunit an event subscriber runs on.

use crate::Engine;
use al_ir::{Arg, Expr, ObjKind, Place, SetElem, Stmt};
use std::collections::HashSet;

/// The builtins without a receiver that read their arguments and the clock or the session settings, and write only a
/// `var` argument (which the call site checks like any other write).
const PLAIN_SYSTEM: &[&str] = &[
    "Abs",
    "ArrayLen",
    "CalcDate",
    "Clear",
    "ClosingDate",
    "CreateDateTime",
    "CurrentDateTime",
    "Date",
    "DaTi",
    "DMY",
    "DT",
    "DWY",
    "Evaluate",
    "EvaluateBounded",
    "EvaluateOptionField",
    "EvaluateVariableOption",
    "Format",
    "FormatAutoFormatField",
    "FormatDecimalField",
    "FormatVariant",
    "GlobalLanguage",
    "GuiAllowed",
    "IsNull",
    "IsNullGuid",
    "IsServiceTier",
    "NormalDate",
    "Power",
    "Round",
    "RoundDateTime",
    "TextConstText",
    "Time",
    "Today",
    "TranslatedText",
    "WindowsLanguage",
];

/// Record methods that read the database or change only the record variable they are called on.
const PLAIN_RECORD: &[&str] = &[
    "Ascending",
    "CalcFields",
    "CalcSums",
    "CopyFilter",
    "CopyFilters",
    "Count",
    "CountApprox",
    "CurrentCompany",
    "CurrentKey",
    "CurrFieldNo",
    "FieldActive",
    "FieldCaption",
    "FieldName",
    "FieldNo",
    "FilterGroup",
    "Find",
    "FindFirst",
    "FindLast",
    "FindSet",
    "Get",
    "GetAscending",
    "GetBySystemId",
    "GetFilter",
    "GetFilters",
    "GetPosition",
    "GetRangeMax",
    "GetRangeMin",
    "GetView",
    "HasFilter",
    "Init",
    "IsEmpty",
    "IsTemporary",
    "Next",
    "ReadPermission",
    "RecordId",
    "Reset",
    "SetAscending",
    "SetAutoCalcFields",
    "SetCurrentKey",
    "SetFilter",
    "SetLoadFields",
    "SetRange",
    "SetRecFilter",
    "TableCaption",
    "TableName",
    "WritePermission",
];

/// Whether `id` is a builtin from the lists above, or a method of Text, Label or TextConst (they work on values).
fn plain_builtin(id: al_builtins::builtin_id::BuiltinId) -> bool {
    let name = id.name();
    let (class, _) = name.split_once('.').unwrap_or(("", name));
    let method = id.method();
    match class {
        "System" => PLAIN_SYSTEM.contains(&method),
        "Record" => PLAIN_RECORD.contains(&method),
        "Text" | "Label" | "TextConst" => true,
        _ => false,
    }
}

struct Walk<'a> {
    engine: &'a Engine,
    /// (object, routine, instance) already entered: a call back into one of them adds nothing new.
    seen: HashSet<(usize, usize, Inst)>,
    /// The first write to a variable of a SingleInstance codeunit, which is kept (see [`Effect::Kept`]).
    kept: Option<String>,
}

/// The instance code runs on, for what its writes to the instance's variables do.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Inst {
    /// Dies with the call: the empty record of the page open, a record or codeunit held in a local or in a variable of a
    /// transient instance, a fresh instance of a subscriber codeunit.
    Transient,
    /// The session's one instance of a SingleInstance codeunit, or a record or codeunit held in its variables.
    Single,
    /// Any other instance that outlives the call (a bound subscriber, an instance an expression gave).
    Lasting,
}

/// What a run of a CaptionClass expression on the empty record can leave behind ([`Engine::lasting_effect`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Effect {
    /// Only writes to variables of SingleInstance codeunits (the description names the first). Refusing such page opens
    /// costs baseline cases: the corpus counts the runs in a SingleInstance log (record/lt-qd-captionclass-areas/
    /// QdCaptionClassAreas.al:1227 SF_Count_CardOpenViewFiveTimes and 32 more cases of codeunit 86504), and Base
    /// Application's Sales Line CaptionClass Mgmt keeps the document header in one. Kept, not pinned (al-runner issue 5870).
    Kept(String),
    /// Anything else that outlives the call: refused.
    Refused(String),
}

impl Engine {
    /// What a run of routine `routine` of object `object` on a transient instance (the empty record of a page open) can
    /// leave behind that AL code may read later; `None` when it leaves nothing. Worked out once per routine.
    pub(crate) fn lasting_effect(&self, object: usize, routine: usize) -> Option<Effect> {
        if let Some(known) = self
            .lasting_effects
            .lock()
            .expect("lasting effects lock")
            .get(&(object, routine))
        {
            return known.clone();
        }
        let mut w = Walk {
            engine: self,
            seen: HashSet::new(),
            kept: None,
        };
        let found = match w.routine(object, routine, Inst::Transient) {
            Err(why) => Some(Effect::Refused(why)),
            Ok(()) => w.kept.map(Effect::Kept),
        };
        self.lasting_effects
            .lock()
            .expect("lasting effects lock")
            .insert((object, routine), found.clone());
        found
    }
}

type Found = Result<(), String>;

impl Walk<'_> {
    fn name(&self, object: usize) -> &str {
        &self.engine.program.objects[object].name
    }

    fn single_instance(&self, object: usize) -> bool {
        let o = &self.engine.program.objects[object];
        o.kind == ObjKind::Codeunit
            && o.properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("SingleInstance") && v.trim().eq_ignore_ascii_case("true"))
    }

    fn routine(&mut self, object: usize, routine: usize, inst: Inst) -> Found {
        if !self.seen.insert((object, routine, inst)) {
            return Ok(());
        }
        let Some(r) = self
            .engine
            .program
            .objects
            .get(object)
            .and_then(|o| o.routines.get(routine))
        else {
            return Err(format!(
                "calls a routine of {} the runtime has no code for",
                self.name(object)
            ));
        };
        let at = (object, routine);
        self.stmts(r.body.stmts(), at, inst)?;
        // An event publisher also runs its subscribers, each on its own instance: the session's one instance of a
        // SingleInstance codeunit, the bound one of a manual subscriber, else a fresh one.
        if let Some(p) = self.engine.events.publishers.get(&at) {
            let key = (p.object, p.event.clone());
            if let Some(subs) = self.engine.events.subscribers.get(&key) {
                for s in subs.clone() {
                    let on = if self.single_instance(s.object) {
                        Inst::Single
                    } else if s.manual {
                        Inst::Lasting
                    } else {
                        Inst::Transient
                    };
                    self.routine(s.object, s.routine, on)?;
                }
            }
        }
        Ok(())
    }

    /// A write to `place` dies with the call: a local, or a variable of a transient instance; a variable of a SingleInstance
    /// codeunit is noted as kept.
    fn place(&mut self, place: &Place, at: (usize, usize), inst: Inst) -> Found {
        match place {
            Place::Local(_) => Ok(()),
            Place::Global(slot) => {
                let o = &self.engine.program.objects[at.0];
                let var = o.globals.get(*slot).map_or("?", |s| s.name.as_str());
                let what = format!("writes the variable {var} of {}, which outlives the call", o.name);
                match inst {
                    Inst::Transient => Ok(()),
                    Inst::Single => {
                        self.kept.get_or_insert(what);
                        Ok(())
                    }
                    Inst::Lasting => Err(what),
                }
            }
            Place::BaseGlobal(_) => Err(format!(
                "writes a variable of the page or report {} extends",
                self.name(at.0)
            )),
            Place::ExtGlobal(..) => Err(format!(
                "writes a variable of another extension of the page {} extends",
                self.name(at.0)
            )),
            Place::Index(inner, ix) => {
                for e in ix {
                    self.expr(e, at, inst)?;
                }
                self.place(inner, at, inst)
            }
            Place::Field(inner, _) => self.place(inner, at, inst),
        }
    }

    /// The instance `e` evaluates to, for a call on it, in code that runs on `inst`.
    fn instance_of(&self, e: &Expr, inst: Inst) -> Inst {
        let mut p = match e {
            Expr::Load(p) => p,
            _ => return Inst::Lasting,
        };
        loop {
            match p {
                Place::Local(_) => return Inst::Transient,
                Place::Global(_) => return inst,
                Place::BaseGlobal(_) | Place::ExtGlobal(..) => return Inst::Lasting,
                Place::Index(inner, _) | Place::Field(inner, _) => p = inner,
            }
        }
    }

    fn args(&mut self, args: &[Arg], at: (usize, usize), inst: Inst) -> Found {
        for a in args {
            match a {
                Arg::Value(e) => self.expr(e, at, inst)?,
                Arg::Ref(p) => self.place(p, at, inst)?,
            }
        }
        Ok(())
    }

    fn stmts(&mut self, body: &[Stmt], at: (usize, usize), inst: Inst) -> Found {
        for s in body {
            self.stmt(s, at, inst)?;
        }
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt, at: (usize, usize), inst: Inst) -> Found {
        match s {
            Stmt::Assign(p, e) => {
                self.place(p, at, inst)?;
                self.expr(e, at, inst)
            }
            Stmt::Expr(e) => self.expr(e, at, inst),
            Stmt::If(c, a, b) => {
                self.expr(c, at, inst)?;
                self.stmts(a, at, inst)?;
                self.stmts(b, at, inst)
            }
            Stmt::While(c, b) | Stmt::Repeat(b, c) => {
                self.expr(c, at, inst)?;
                self.stmts(b, at, inst)
            }
            Stmt::For {
                var,
                from,
                to,
                down: _,
                body,
            } => {
                self.place(var, at, inst)?;
                self.expr(from, at, inst)?;
                self.expr(to, at, inst)?;
                self.stmts(body, at, inst)
            }
            Stmt::Foreach { var, collection, body } => {
                self.place(var, at, inst)?;
                self.expr(collection, at, inst)?;
                self.stmts(body, at, inst)
            }
            Stmt::Case {
                scrutinee,
                branches,
                otherwise,
            } => {
                self.expr(scrutinee, at, inst)?;
                for (set, body) in branches {
                    self.set(set, at, inst)?;
                    self.stmts(body, at, inst)?;
                }
                self.stmts(otherwise, at, inst)
            }
            Stmt::Exit(e) => e.as_ref().map_or(Ok(()), |e| self.expr(e, at, inst)),
            Stmt::AssertError(b) => self.stmts(b, at, inst),
            Stmt::Break | Stmt::Continue | Stmt::Pos(_) | Stmt::Assume(_) | Stmt::TddMark(_) => Ok(()),
        }
    }

    fn set(&mut self, set: &[SetElem], at: (usize, usize), inst: Inst) -> Found {
        for e in set {
            match e {
                SetElem::One(e) => self.expr(e, at, inst)?,
                SetElem::Range(a, b) => {
                    for e in a.iter().chain(b.iter()) {
                        self.expr(e, at, inst)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn expr(&mut self, e: &Expr, at: (usize, usize), inst: Inst) -> Found {
        match e {
            // Code the runtime refuses or fails when it runs: the run ends there, with every number of runs.
            Expr::Const(_)
            | Expr::Load(_)
            | Expr::BaseInstance
            | Expr::ThisInstance
            | Expr::Refused(_)
            | Expr::Fail { .. }
            | Expr::ControlAddInCall { .. } => Ok(()),
            Expr::Unary(_, a)
            | Expr::Convert(_, a)
            | Expr::FieldOf(a, _)
            | Expr::DotNetCoerce(_, a)
            | Expr::DotNetOperand(a, _)
            | Expr::TddFit { inner: a, .. }
            | Expr::FitOrRefuse { inner: a, .. } => self.expr(a, at, inst),
            Expr::Binary(_, a, b) | Expr::CharAt(a, b) => {
                self.expr(a, at, inst)?;
                self.expr(b, at, inst)
            }
            Expr::In(a, set) => {
                self.expr(a, at, inst)?;
                self.set(set, at, inst)
            }
            Expr::Ternary(c, a, b) => {
                self.expr(c, at, inst)?;
                self.expr(a, at, inst)?;
                self.expr(b, at, inst)
            }
            Expr::Call {
                object,
                routine,
                instance,
                args,
                try_call: _,
            } => {
                self.args(args, at, inst)?;
                let callee = match instance {
                    None => inst,
                    Some(i) => {
                        self.expr(i, at, inst)?;
                        if self.single_instance(*object) {
                            Inst::Single
                        } else {
                            self.instance_of(i, inst)
                        }
                    }
                };
                self.routine(*object, *routine, callee)
            }
            Expr::InterfaceCall {
                receiver,
                targets,
                args,
            } => {
                self.expr(receiver, at, inst)?;
                self.args(args, at, inst)?;
                let _ = targets;
                Err("calls a procedure through an interface".into())
            }
            Expr::InterfaceTest { value, .. } | Expr::EnumImpl { value, .. } => self.expr(value, at, inst),
            Expr::Control { page, .. } => self.expr(page, at, inst),
            Expr::QueryColumn { query, .. } => self.expr(query, at, inst),
            Expr::DotNet { .. } => Err("calls a .NET member".into()),
            Expr::Builtin {
                id,
                receiver,
                args,
                ret_used: _,
            } => {
                if !plain_builtin(*id) {
                    return Err(format!("calls {}", id.name().split('#').next().unwrap_or(id.name())));
                }
                if let Some(r) = receiver {
                    self.args(std::slice::from_ref(r), at, inst)?;
                }
                self.args(args, at, inst)
            }
            Expr::TddStub { args, .. } => {
                for a in args {
                    self.expr(a, at, inst)?;
                }
                Ok(())
            }
        }
    }
}
