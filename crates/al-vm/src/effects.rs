//! Whether running a routine can leave something behind that AL code reads later (al-runner issue 5870).
//!
//! A page open runs the CaptionClass expressions of its source table with an empty record a number of times BC does not
//! give stably (8 per Card OpenEdit on one BC 29 container in issue 5870; 240, 480 and 492 calls for one Card open in the
//! corpus, record/captions/QdCaptionClassAreas.al:657). The number can only matter when a run leaves a
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
    "AutoFormatWriterGuard",
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
    "FormatOptionColumn",
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
        // The session's user, a value (record/captions/CaptionClassStateFreeCalls.al:160 in CueCc; captions at :606, :655).
        "Database" => method == "UserSecurityId",
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
    /// costs baseline cases: the corpus counts the runs in a SingleInstance log (record/captions/
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
            // A variable of a record's table or table extension instance lives as long as the record variable.
            Place::Field(inner, _) | Place::RecGlobal(inner, ..) => self.place(inner, at, inst),
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
                Place::Index(inner, _) | Place::Field(inner, _) | Place::RecGlobal(inner, ..) => p = inner,
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
            // `[RunOnClient]` bookkeeping calls nothing (al_ir `DotNetOp::ClientNull`, `ClientAssign`).
            Expr::DotNet {
                op: al_ir::DotNetOp::ClientNull | al_ir::DotNetOp::ClientAssign,
                receiver,
                args,
                ..
            } => {
                if let Some(r) = receiver {
                    self.expr(r, at, inst)?;
                }
                self.args(args, at, inst)
            }
            Expr::DotNet { .. } => Err("calls a .NET member".into()),
            Expr::Builtin {
                id,
                receiver,
                args,
                ret_used: _,
            } => {
                if !plain_builtin(*id)
                    && !self.on_confined_collection(*id, receiver.as_deref(), at)
                    && !self.as_integer_in_table_or_codeunit(*id, at)
                    && !get_table_on_local(*id, receiver.as_deref())
                    && !dialog_checked_at_run_time(*id)
                {
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

/// `RecordRef.GetTable` on a RecordRef local of the routine: it writes only that local, which dies with the call
/// (record/captions/CaptionClassStateFreeCalls.al:203 and :332, `RecRef.GetTable(Rec)` in RefCopyCc and in a table extension's
/// expression; the Card and part captions at :658, :750, :753; al-runner issue 7467).
fn get_table_on_local(id: al_builtins::builtin_id::BuiltinId, receiver: Option<&Arg>) -> bool {
    id.name() == "RecordRef.GetTable#1" && receiver.and_then(local_slot).is_some()
}

/// Message, Confirm and Error: whether a run raises one is a fact of the run, so the walk lets them through. On a page whose
/// empty-record runs have no pinned number, a dialog raised by any of its CaptionClass runs refuses
/// (`Session::run_caption_empty_unpinned`, `Session::fill_field_captions_with_row`); an error ends the open at its first run
/// whatever the number (record/captions/CaptionClassStateFreeCalls.al:168-181 NotTakenCc, not taken with the empty record,
/// :655-701; the taken Error branch fails the open at :919-936; al-runner issue 7391).
fn dialog_checked_at_run_time(id: al_builtins::builtin_id::BuiltinId) -> bool {
    let name = id.name();
    // Error with a text (not with an ErrorInfo, which can be collected instead of raised).
    matches!(
        name,
        "Dialog.Message#1"
            | "Dialog.Confirm#1"
            | "Dialog.Error#2"
            | "Dialog.ErrorComputed#1"
            | "Dialog.ErrorConstant#1"
            | "Dialog.ErrorNotLiteral#1"
    )
}

/// The local slot `arg` names, when it is a plain local (a receiver written as a place or as a load).
fn local_slot(arg: &Arg) -> Option<usize> {
    match arg {
        Arg::Ref(Place::Local(n)) | Arg::Value(Expr::Load(Place::Local(n))) => Some(*n),
        _ => None,
    }
}

impl Walk<'_> {
    /// `Enum.AsInteger` in a procedure of the table itself reads a value and leaves nothing behind: a Card and a List over a
    /// table whose CaptionClass expression calls it open as any other (corpus record/captions/CaptionExpressionPlainText.al:296
    /// CardControlCaptions_PlainExpressions_KindResource, :344 ListControlCaptions_PlainExpressions_KindResource, :368
    /// CountCard_AsIntegerExpression_RunsAtOpen; BC 27.5, 28.5 and 29.0; al-runner issue 7379). So does the call inside a
    /// procedure of a codeunit the expression calls (record/captions/CaptionClassStateFreeCalls.al:111-114 KindNumber, called by
    /// HelperCc at :188; the Card, List and part captions at :657, :726, :752).
    fn as_integer_in_table_or_codeunit(&self, id: al_builtins::builtin_id::BuiltinId, at: (usize, usize)) -> bool {
        let name = id.name();
        name.split('#').next() == Some("Enum.AsInteger")
            && self
                .engine
                .program
                .objects
                .get(at.0)
                .is_some_and(|o| matches!(o.kind, ObjKind::Table | ObjKind::Codeunit))
    }

    /// A method of List or Dictionary called on a collection the routine `at` made itself and never shares: what it writes dies
    /// with the call (al-runner issue 6987: a CaptionClass expression that fills a local `List of [Text]` and hands it by `var`
    /// to an integration event no subscriber listens to). A List and a Dictionary are references, so a collection the routine
    /// takes from outside may be another variable's: see [`Walk::confined_local`].
    fn on_confined_collection(
        &self,
        id: al_builtins::builtin_id::BuiltinId,
        receiver: Option<&Arg>,
        at: (usize, usize),
    ) -> bool {
        let class = id.name().split_once('.').map_or("", |(c, _)| c);
        matches!(class, "List" | "Dictionary")
            && receiver
                .and_then(local_slot)
                .is_some_and(|slot| self.confined_local(at, slot))
    }

    /// Whether local `slot` of routine `at` only ever holds the collection the routine itself made: it is not a parameter, it is
    /// never assigned (nor a loop variable), and it is handed by `var` only as the receiver of a List or Dictionary method or to an
    /// event publisher no subscriber listens to (whose body is empty, so nothing can put another collection into it).
    fn confined_local(&self, at: (usize, usize), slot: usize) -> bool {
        let Some(r) = self.engine.program.objects.get(at.0).and_then(|o| o.routines.get(at.1)) else {
            return false;
        };
        if slot < r.params {
            return false;
        }
        let is_slot = |p: &Place| matches!(p, Place::Local(n) if *n == slot);
        if !stmts_keep_local(r.body.stmts(), &is_slot) {
            return false;
        }
        let silent_publisher = |object: usize, routine: usize| {
            self.engine.events.publishers.get(&(object, routine)).is_some_and(|p| {
                self.engine
                    .events
                    .subscribers
                    .get(&(p.object, p.event.clone()))
                    .is_none_or(|subs| subs.is_empty())
            })
        };
        let by_var = |args: &[Arg]| args.iter().any(|a| matches!(a, Arg::Ref(p) if is_slot(p)));
        let mut confined = true;
        al_ir::types::for_each_expr_post(r.body.stmts(), &mut |e| match e {
            Expr::Call {
                object, routine, args, ..
            } if by_var(args) && !silent_publisher(*object, *routine) => {
                confined = false;
            }
            Expr::Builtin { args, .. } | Expr::InterfaceCall { args, .. } | Expr::DotNet { args, .. }
                if by_var(args) =>
            {
                confined = false;
            }
            _ => {}
        });
        confined
    }
}

/// No statement of `body` assigns the local `is_slot` names or loops over it.
pub(crate) fn stmts_keep_local(body: &[Stmt], is_slot: &dyn Fn(&Place) -> bool) -> bool {
    body.iter().all(|s| match s {
        Stmt::Assign(p, _) => !is_slot(p),
        Stmt::For { var, body, .. } | Stmt::Foreach { var, body, .. } => {
            !is_slot(var) && stmts_keep_local(body, is_slot)
        }
        Stmt::If(_, a, b) => stmts_keep_local(a, is_slot) && stmts_keep_local(b, is_slot),
        Stmt::While(_, b) | Stmt::Repeat(b, _) | Stmt::AssertError(b) => stmts_keep_local(b, is_slot),
        Stmt::Case {
            branches, otherwise, ..
        } => branches.iter().all(|(_, b)| stmts_keep_local(b, is_slot)) && stmts_keep_local(otherwise, is_slot),
        _ => true,
    })
}
