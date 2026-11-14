//! What the DataCaptionExpression of a page in a cell of corpus PR 1401 pages/properties/DataCaptionGetOpen.al may do, checked on
//! the compiled code before the expression runs at the open (`data_caption_get_open.rs`). The measured bodies (DataCaptionGetOpen.al
//! :83-98, :184-198, the DCG Log procedures :222-240) read a record variable of their own with a guarded `Get`, read a field of
//! Rec (:191 `Rec."Code"`, page DCG WS Own only), join texts, assign a local and a page variable (:95 `G := 'cap-ran'`) and call
//! procedures of a SingleInstance codeunit that add to a List, count and copy a record (:222-240). The check is an allowlist of
//! those kinds over the expression and every procedure it calls: anything else (an assignment to Rec or one of its fields, a
//! method of Rec, a CurrPage or UI call, Commit, Codeunit.Run, a loop, an event, code the runtime refused to lower) is outside the
//! cells.

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
    /// A field of Rec may be read (Run and Trap, DataCaptionGetOpen.al:184-198 measured with page DCG WS Own; no cell reads Rec
    /// under RunModal, the Own cells are Run and Trap only).
    rec_field_reads: bool,
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
                self.expr(c, at)?;
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

    /// A read of `place`: Rec or xRec only as a field of Rec where `rec_field_reads` allows it.
    fn read(&mut self, place: &Place, at: (usize, usize)) -> Checked {
        if self.rec_rooted(place, at) {
            let rec_field = matches!(place, Place::Field(inner, _) if matches!(**inner, Place::Global(n)
                if self.rec_slots[0] == Some(n)));
            return if rec_field && self.rec_field_reads {
                Ok(())
            } else {
                Err("reads Rec in a way no cell measured")
            };
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

    /// Builtin `id` on `receiver`: the two the measured bodies call, `Get` of a record variable other than Rec
    /// (DataCaptionGetOpen.al:90 `Other.Get('K1')`, :191 `Twin.Get(Rec."Code")`) and `Add` of a List (:224 `Lines.Add(Line)`).
    fn builtin(&mut self, id: BuiltinId, receiver: Option<&Arg>, at: (usize, usize)) -> Checked {
        let name = id.name();
        let class = name.split_once('.').map_or("", |(c, _)| c);
        let listed = matches!((class, id.method()), ("Record", "Get") | ("List", "Add"));
        if !listed {
            return Err("calls a method no cell measured (only Get of another record variable and List.Add are)");
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
            seen: HashSet::new(),
        };
        walk.routine((page, routine))
    }
}
