//! What the DataCaptionExpression of a page in a cell of pages/properties/DataCaptionGetOpen.al may do, checked on the compiled
//! code before the expression runs at the open (`data_caption_get_open.rs`). The measured bodies (DataCaptionGetOpen.al :83-98,
//! :184-198, the DCG Log procedures :222-240) read a local record with a Get whose result is the condition of an If, keyed by a
//! constant (:90 `Other.Get('K1')`) or by a field of Rec on a record of the page's table (:191 `Twin.Get(Rec."Code")`, page DCG
//! WS Own only), join texts, assign a local and a page variable (:95 `G := 'cap-ran'`) and call procedures of a SingleInstance
//! codeunit that add to a List, count and copy a record (:222-240). The check is an allowlist of those forms over the expression
//! and every procedure it calls: anything else (another Get, a read of Rec outside that key, an assignment to Rec or one of its
//! fields, a method of Rec, a CurrPage or UI call, Commit, Codeunit.Run, a loop, an event, code the runtime refused to lower) is
//! outside the cells.

use super::*;
use al_builtins::builtin_id::BuiltinId;
use al_ir::{Arg, Expr, Place, SetElem, Stmt};
use std::collections::HashSet;

/// The checked facts of one walk.
struct BodyWalk<'a> {
    program: &'a ir::Program,
    blocked: &'a [Vec<Option<String>>],
    publishers: &'a dyn Fn((usize, usize)) -> bool,
    /// The page object and the slots of its Rec and xRec.
    page: usize,
    rec_slots: [Option<usize>; 2],
    /// A field of Rec may be the key of a guarded Get (Run and Trap, DataCaptionGetOpen.al:184-198 measured with page DCG WS Own;
    /// no cell reads Rec under RunModal, the Own cells are Run and Trap only).
    rec_field_reads: bool,
    /// The table of the page's Rec.
    rec_table: Option<u32>,
    seen: HashSet<(usize, usize)>,
}

type Checked = Result<(), &'static str>;

impl BodyWalk<'_> {
    /// Whether `place`, in routine `at`, is Rec or xRec of the page or a part of one.
    fn rec_rooted(&self, place: &Place, at: (usize, usize)) -> bool {
        match place {
            Place::Global(n) => at.0 == self.page && self.rec_slots.contains(&Some(*n)),
            Place::Field(inner, _) | Place::RecGlobal(inner, ..) | Place::Index(inner, _) => self.rec_rooted(inner, at),
            _ => false,
        }
    }

    fn routine(&mut self, at: (usize, usize)) -> Checked {
        if !self.seen.insert(at) {
            return Ok(());
        }
        if self
            .blocked
            .get(at.0)
            .and_then(|o| o.get(at.1))
            .is_some_and(Option::is_some)
        {
            return Err("calls a procedure the runtime cannot run");
        }
        if (self.publishers)(at) {
            return Err("raises an event");
        }
        let Some(r) = self.program.objects.get(at.0).and_then(|o| o.routines.get(at.1)) else {
            return Err("calls a procedure the runtime has no code for");
        };
        self.stmts(r.body.stmts(), at)
    }

    fn stmts(&mut self, body: &[Stmt], at: (usize, usize)) -> Checked {
        body.iter().try_for_each(|s| self.stmt(s, at))
    }

    fn stmt(&mut self, s: &Stmt, at: (usize, usize)) -> Checked {
        match s {
            Stmt::Assign(p, e) => {
                self.write(p, at)?;
                self.expr(e, at)
            }
            Stmt::Expr(e) => self.expr(e, at),
            Stmt::If(c, a, b) => {
                match Self::record_get(c) {
                    Some((receiver, args)) => self.guarded_get(receiver, args, at)?,
                    None => self.expr(c, at)?,
                }
                self.stmts(a, at)?;
                self.stmts(b, at)
            }
            Stmt::Case {
                scrutinee,
                branches,
                otherwise,
            } => {
                self.expr(scrutinee, at)?;
                for (set, body) in branches {
                    self.set(set, at)?;
                    self.stmts(body, at)?;
                }
                self.stmts(otherwise, at)
            }
            Stmt::Exit(e) => e.as_ref().map_or(Ok(()), |e| self.expr(e, at)),
            Stmt::Pos(_) | Stmt::Assume(_) | Stmt::TddMark(_) => Ok(()),
            _ => Err("runs a loop, asserterror, break or continue"),
        }
    }

    /// A write to `place`: a local, a variable of the routine's object, or a field or element of one; never Rec or xRec of the
    /// page, nor a variable of another object (a base page, another extension).
    fn write(&mut self, place: &Place, at: (usize, usize)) -> Checked {
        if self.rec_rooted(place, at) {
            return Err("assigns Rec or a field of it");
        }
        match place {
            Place::Local(_) | Place::Global(_) => Ok(()),
            Place::Field(inner, _) => self.write(inner, at),
            Place::Index(inner, ix) => {
                ix.iter().try_for_each(|e| self.expr(e, at))?;
                self.write(inner, at)
            }
            _ => Err("writes a variable of another object"),
        }
    }

    /// A read of `place`: never Rec or xRec (a field of Rec is read only as the key of a guarded Get, [`Self::guarded_get`]).
    fn read(&mut self, place: &Place, at: (usize, usize)) -> Checked {
        if self.rec_rooted(place, at) {
            return Err("reads Rec in a way no cell measured");
        }
        match place {
            Place::Local(_) | Place::Global(_) => Ok(()),
            Place::Field(inner, _) => self.read(inner, at),
            Place::Index(inner, ix) => {
                ix.iter().try_for_each(|e| self.expr(e, at))?;
                self.read(inner, at)
            }
            _ => Err("reads a variable of another object"),
        }
    }

    fn set(&mut self, set: &[SetElem], at: (usize, usize)) -> Checked {
        set.iter().try_for_each(|e| match e {
            SetElem::One(e) => self.expr(e, at),
            SetElem::Range(a, b) => a.iter().chain(b.iter()).try_for_each(|e| self.expr(e, at)),
        })
    }

    fn args(&mut self, args: &[Arg], at: (usize, usize)) -> Checked {
        args.iter().try_for_each(|a| match a {
            Arg::Value(e) => self.expr(e, at),
            Arg::Ref(p) => self.write(p, at),
        })
    }

    fn expr(&mut self, e: &Expr, at: (usize, usize)) -> Checked {
        match e {
            Expr::Const(_) => Ok(()),
            Expr::Load(p) => self.read(p, at),
            Expr::Unary(_, a) | Expr::Convert(_, a) | Expr::FieldOf(a, _) => self.expr(a, at),
            Expr::Binary(_, a, b) | Expr::CharAt(a, b) => {
                self.expr(a, at)?;
                self.expr(b, at)
            }
            Expr::Ternary(c, a, b) => {
                self.expr(c, at)?;
                self.expr(a, at)?;
                self.expr(b, at)
            }
            Expr::In(a, set) => {
                self.expr(a, at)?;
                self.set(set, at)
            }
            Expr::Call {
                object,
                routine,
                instance,
                args,
                ..
            } => {
                if let Some(i) = instance {
                    self.expr(i, at)?;
                }
                self.args(args, at)?;
                self.routine((*object, *routine))
            }
            Expr::Builtin { id, receiver, args, .. } => {
                self.builtin(*id, receiver.as_deref(), at)?;
                self.args(args, at)
            }
            _ => Err("runs code of a kind no cell measured (an interface, .NET, a control, CurrPage, refused code)"),
        }
    }

    /// The receiver and arguments of `c` when it is a call of `Record.Get`.
    fn record_get(c: &Expr) -> Option<(Option<&Arg>, &[Arg])> {
        match c {
            Expr::Builtin { id, receiver, args, .. } if Self::is_record_get(*id) => Some((receiver.as_deref(), args)),
            _ => None,
        }
    }

    fn is_record_get(id: BuiltinId) -> bool {
        id.name().split_once('.').is_some_and(|(c, _)| c == "Record") && id.method() == "Get"
    }

    /// A Get that is the condition of an If, as the measured bodies have it: on a local record of the routine, keyed by one
    /// constant (DataCaptionGetOpen.al:90 `if Other.Get('K1') then`) or, where `rec_field_reads` allows it, by one field of Rec on
    /// a record of the page's table (:191 `if Twin.Get(Rec."Code") then`).
    fn guarded_get(&mut self, receiver: Option<&Arg>, args: &[Arg], at: (usize, usize)) -> Checked {
        let Some(Arg::Ref(Place::Local(n)) | Arg::Value(Expr::Load(Place::Local(n)))) = receiver else {
            return Err("calls Get on a record that is not a local of the procedure");
        };
        let table = self
            .program
            .objects
            .get(at.0)
            .and_then(|o| o.routines.get(at.1))
            .and_then(|r| r.locals.get(*n))
            .and_then(|slot| match slot.ty {
                al_builtins::value::RtType::Record(t, _) => Some(t),
                _ => None,
            });
        let rec_field = |p: &Place| {
            matches!(p, Place::Field(inner, _) if at.0 == self.page
                && matches!(**inner, Place::Global(g) if self.rec_slots[0] == Some(g)))
        };
        match args {
            [Arg::Value(Expr::Const(_))] => Ok(()),
            [Arg::Value(Expr::Load(p))]
                if self.rec_field_reads && rec_field(p) && table.is_some() && table == self.rec_table =>
            {
                Ok(())
            }
            _ => {
                Err("calls Get with a key other than one constant or one field of Rec on a record of the page's table")
            }
        }
    }

    /// Builtin `id` on `receiver`: the one the measured bodies call outside a guarded Get, `Add` of a List (:224
    /// `Lines.Add(Line)`).
    fn builtin(&mut self, id: BuiltinId, receiver: Option<&Arg>, at: (usize, usize)) -> Checked {
        if Self::is_record_get(id) {
            return Err("calls Get other than as the condition of an If");
        }
        let name = id.name();
        let class = name.split_once('.').map_or("", |(c, _)| c);
        if !matches!((class, id.method()), ("List", "Add")) {
            return Err("calls a method no cell measured (only a guarded Get of a local record and List.Add are)");
        }
        match receiver {
            None => Ok(()),
            Some(Arg::Ref(p) | Arg::Value(Expr::Load(p))) if self.rec_rooted(p, at) => Err("calls a method of Rec"),
            Some(Arg::Ref(p)) => self.write(p, at),
            Some(Arg::Value(v)) => self.expr(v, at),
        }
    }
}

impl Session {
    /// Whether the DataCaptionExpression of page `st` (routine `routine` of the page object) and every procedure it calls do
    /// only what the bodies of DataCaptionGetOpen.al do (see the module comment); with `modal` (the ModalPageHandler run) without
    /// reading Rec at all, as the measured RunModal cells (Get_*_RunModal, page DCG WS Get, :83-98) have it. `Err`: what the
    /// code does outside the cells.
    pub(super) fn data_caption_body_in_cells(&self, st: &St, routine: usize, modal: bool) -> Result<(), &'static str> {
        let engine = self.engine();
        let page = st.borrow().object as usize;
        let object = &engine.program.objects[page];
        let publishers = |at: (usize, usize)| engine.events.publishers.contains_key(&at);
        let mut walk = BodyWalk {
            program: &engine.program,
            blocked: &engine.blocked,
            publishers: &publishers,
            page,
            rec_slots: [object.rec_slot, object.xrec_slot],
            rec_field_reads: !modal,
            rec_table: object.rec_slot.and_then(|n| match object.globals.get(n)?.ty {
                al_builtins::value::RtType::Record(t, _) => Some(t),
                _ => None,
            }),
            seen: HashSet::new(),
        };
        walk.routine((page, routine))
    }
}
