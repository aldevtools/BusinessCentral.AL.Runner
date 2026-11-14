//! Whether a page's DataCaptionExpression can raise an error or leave something behind when it runs (al-runner issue 6924).
//!
//! BC runs the expression of a Worksheet at the open (corpus pages/properties/DataCaptionExprWorksheet.al:412, :439, :572,
//! :586); main's path runs it only at a caption read. An expression that can neither raise nor write anything but its own
//! locals changes nothing AL code or a test can see except the caption, so the page keeps main's open (assumption
//! `data-caption-pure-expression-open-skipped`) and refuses at the caption read (`pages/data_caption_open.rs`). Everything else
//! refuses at the open.
//!
//! The walk is [`Walk`] with `strict` set. It is an allowlist: each construct below is let through only in the form whose
//! argument kinds, operand types and constant values cannot raise; every other construct counts, with the reason the open
//! names. The entry text of the assumption lists the same forms; a change here changes it too.

use super::{Found, Inst, Walk};
use crate::Engine;
use al_builtins::builtin_id::BuiltinId;
use al_builtins::value::RtType;
use al_ir::{Arg, BinOp, Const, Expr, Place, Stmt, UnOp};
use std::collections::HashSet;

/// Methods of Text that read their arguments, whatever they are, and cannot raise an error.
const TEXT_READ: &[&str] = &[
    "Contains",
    "EndsWith",
    "LowerCase",
    "MaxStrLen",
    "StartsWith",
    "StrLen",
    "ToLower",
    "ToUpper",
    "Trim",
    "TrimEnd",
    "TrimStart",
    "UpperCase",
];

/// Methods of Text whose result is a Text (for [`Walk::text_valued`]).
const TEXT_RESULT: &[&str] = &[
    "DelChr",
    "LowerCase",
    "StrSubstNo",
    "ToLower",
    "ToUpper",
    "Trim",
    "TrimEnd",
    "TrimStart",
    "UpperCase",
];

/// Record methods that only read the record variable and the database, run no AL code and cannot raise an error.
const RECORD_READ: &[&str] = &[
    "Count",
    "CountApprox",
    "CurrentKey",
    "FieldName",
    "FieldNo",
    "GetPosition",
    "GetView",
    "HasFilter",
    "IsEmpty",
    "IsTemporary",
    "ReadPermission",
    "RecordId",
    "TableCaption",
    "TableName",
    "WritePermission",
];

/// Record methods that read a field's caption: a field's CaptionClass runs AL code there, once per read, and what it changes
/// stays changed (record/captions/QdCaptionClassAreas.al:323-372); a literal one runs the resolve subscribers.
const RECORD_CAPTION_READ: &[&str] = &["FieldCaption", "GetFilter", "GetFilters"];

/// Record methods whose result is a Text (for [`Walk::text_valued`]).
const RECORD_TEXT_RESULT: &[&str] = &[
    "CurrentKey",
    "FieldCaption",
    "FieldName",
    "GetFilter",
    "GetFilters",
    "GetPosition",
    "GetView",
    "TableCaption",
    "TableName",
];

/// The Where arguments of DelChr that do not raise ('Incorrect use of the where parameter.' for any other).
const DELCHR_WHERE: &[&str] = &["=", "<", ">", "<>"];

/// A Text or Code of a fixed length (also an array of one): a value written to it that is longer raises.
fn fixed_length(ty: &RtType) -> bool {
    match ty {
        RtType::Text(Some(_)) | RtType::Code(Some(_)) => true,
        RtType::Array(_, inner) => fixed_length(inner),
        _ => false,
    }
}

/// The class and method of builtin `id` (`Text.StrSubstNo#1` gives `("Text", "StrSubstNo")`).
fn class_method(id: BuiltinId) -> (&'static str, &'static str) {
    let name = id.name().split('#').next().unwrap_or(id.name());
    name.split_once('.').unwrap_or(("", name))
}

/// The text constant `e` is, through conversions.
fn const_text(e: &Expr) -> Option<&str> {
    match e {
        Expr::Const(Const::Text(t)) => Some(t),
        Expr::Convert(_, inner) => const_text(inner),
        _ => None,
    }
}

/// A text constant or a label's text (`System.TranslatedText`, `System.TextConstText` of constants).
fn constant_or_label(e: &Expr) -> bool {
    match e {
        Expr::Builtin { id, args, .. }
            if matches!(class_method(*id), ("System", "TranslatedText" | "TextConstText")) =>
        {
            args.iter()
                .all(|a| matches!(a, Arg::Value(v) if matches!(v, Expr::Const(_))))
        }
        _ => const_text(e).is_some(),
    }
}

/// An Integer constant, through conversions.
fn const_integer(e: &Expr) -> bool {
    match e {
        Expr::Const(Const::Integer(_)) => true,
        Expr::Convert(_, inner) => const_integer(inner),
        _ => false,
    }
}

fn value_arg(a: Option<&Arg>) -> Option<&Expr> {
    match a {
        Some(Arg::Value(v)) => Some(v),
        _ => None,
    }
}

impl Engine {
    /// What a run of routine `routine` of object `object` (a page's DataCaptionExpression) can do that a page open has to
    /// answer for: raise an error or leave anything behind but its own locals. `None`: neither. Worked out once per routine.
    pub(crate) fn caption_open_risk(&self, object: usize, routine: usize) -> Option<String> {
        let mut known = self.caption_open_risks.lock().expect("caption open risks lock");
        if let Some(found) = known.get(&(object, routine)) {
            return found.clone();
        }
        let mut w = Walk {
            engine: self,
            seen: HashSet::new(),
            kept: None,
            strict: true,
            path: Vec::new(),
        };
        // The page's own instance outlives the run.
        let found = w.routine(object, routine, Inst::Lasting).err();
        known.insert((object, routine), found.clone());
        found
    }
}

impl Walk<'_> {
    /// Entering routine `at`: one lowering blocked, one already on the call path (a recursion that never ends raises 'There is
    /// insufficient memory'), and an event publisher (a subscriber may do anything) count. Else `at` joins the path.
    pub(super) fn strict_enter(&mut self, at: (usize, usize)) -> Found {
        let name = || {
            self.engine
                .program
                .objects
                .get(at.0)
                .and_then(|o| o.routines.get(at.1))
                .map_or("?", |r| r.name.as_str())
        };
        if let Some(Some(why)) = self.engine.blocked.get(at.0).and_then(|o| o.get(at.1)) {
            return Err(format!("calls {}, which the runtime cannot run ({why})", name()));
        }
        if self.path.contains(&at) {
            return Err(format!("calls itself ({})", name()));
        }
        if self.engine.events.publishers.contains_key(&at) {
            return Err(format!("raises the event {}", name()));
        }
        self.path.push(at);
        Ok(())
    }

    fn local_ty(&self, at: (usize, usize), slot: usize) -> Option<&RtType> {
        self.engine.program.objects[at.0].routines[at.1]
            .locals
            .get(slot)
            .map(|s| &s.ty)
    }

    /// The declared type of `place` in routine `at`: a local, a global of the routine's object, an element of an array of
    /// those, or a field of a record held in one.
    fn place_ty(&self, place: &Place, at: (usize, usize)) -> Option<&RtType> {
        let objects = &self.engine.program.objects;
        match place {
            Place::Local(n) => self.local_ty(at, *n),
            Place::Global(n) => objects[at.0].globals.get(*n).map(|g| &g.ty),
            Place::RecGlobal(_, o, n) => objects.get(*o)?.globals.get(*n).map(|g| &g.ty),
            Place::Index(inner, _) => match self.place_ty(inner, at)? {
                RtType::Array(_, el) => Some(el),
                _ => None,
            },
            Place::Field(inner, pos) => match self.place_ty(inner, at)? {
                RtType::Record(t, _) => objects
                    .get(*t as usize)?
                    .table
                    .as_ref()?
                    .fields
                    .get(*pos)
                    .map(|f| &f.ty),
                _ => None,
            },
            Place::BaseGlobal(_) | Place::ExtGlobal(..) => None,
        }
    }

    /// Whether `e` is known to give a Text or Code: a text constant or label, a Text or Code variable or field, a listed
    /// Text or Record method or Format, a call of a procedure that returns Text or Code, or a `+` of two such.
    fn text_valued(&self, e: &Expr, at: (usize, usize)) -> bool {
        match e {
            Expr::Const(Const::Text(_)) => true,
            Expr::Load(p) => matches!(self.place_ty(p, at), Some(RtType::Text(_) | RtType::Code(_))),
            Expr::Convert(RtType::Text(_) | RtType::Code(_), inner) => self.text_valued(inner, at),
            Expr::Binary(BinOp::Add, a, b) => self.text_valued(a, at) && self.text_valued(b, at),
            Expr::Builtin { id, .. } => match class_method(*id) {
                ("System", "Format" | "TranslatedText" | "TextConstText") => true,
                ("Text", m) => TEXT_RESULT.contains(&m),
                ("Record", m) => RECORD_TEXT_RESULT.contains(&m),
                _ => false,
            },
            Expr::Call { object, routine, .. } => self
                .engine
                .program
                .objects
                .get(*object)
                .and_then(|o| o.routines.get(*routine))
                .is_some_and(|r| matches!(r.ret, Some(RtType::Text(_) | RtType::Code(_)))),
            _ => false,
        }
    }

    /// A write: only to a local of routine `at` of a type without a length.
    pub(super) fn strict_place(&self, place: &Place, at: (usize, usize)) -> Found {
        match place {
            Place::Local(n) if self.local_ty(at, *n).is_some_and(|t| !fixed_length(t)) => Ok(()),
            Place::Local(_) => Err("writes a local Text or Code of a fixed length".into()),
            _ => Err("writes a variable that outlives the call, or an element or field of one".into()),
        }
    }

    /// A read of `place`: an array element counts (an index out of range raises); any other read cannot raise.
    fn strict_read(place: &Place) -> Found {
        match place {
            Place::Index(..) => Err("reads an array element (an index out of range raises)".into()),
            Place::Field(inner, _) | Place::RecGlobal(inner, ..) => Self::strict_read(inner),
            _ => Ok(()),
        }
    }

    pub(super) fn strict_stmt(&self, s: &Stmt, at: (usize, usize)) -> Found {
        match s {
            Stmt::Exit(Some(_)) => {
                let ret = self.engine.program.objects[at.0].routines[at.1].ret.as_ref();
                if ret.is_some_and(fixed_length) {
                    return Err("returns a Text or Code of a fixed length".into());
                }
                Ok(())
            }
            // A loop may never end, and the step of a for loop can overflow.
            Stmt::While(..) | Stmt::Repeat(..) | Stmt::For { .. } | Stmt::Foreach { .. } => Err("runs a loop".into()),
            Stmt::AssertError(_) => Err("runs asserterror".into()),
            _ => Ok(()),
        }
    }

    /// The facts of node `e` that can raise. `Ok(true)`: `e` is a builtin call, checked here in full.
    pub(super) fn strict_expr(&mut self, e: &Expr, at: (usize, usize), inst: Inst) -> Result<bool, String> {
        match e {
            Expr::Load(p) => Self::strict_read(p).map(|()| false),
            Expr::Const(_) | Expr::Ternary(..) | Expr::In(..) | Expr::FieldOf(..) => Ok(false),
            Expr::Unary(UnOp::Not | UnOp::Plus, _) => Ok(false),
            Expr::Binary(op, a, b) => match op {
                // Integer and Date addition can overflow or give a date that is not valid.
                BinOp::Add if self.text_valued(a, at) && self.text_valued(b, at) => Ok(false),
                BinOp::Add => {
                    Err("adds values that are not both Text or Code (a number or date can go out of range)".into())
                }
                BinOp::And
                | BinOp::Or
                | BinOp::Xor
                | BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge => Ok(false),
                BinOp::Div | BinOp::IntDiv | BinOp::Mod => Err("divides".into()),
                BinOp::Sub | BinOp::Mul => Err("subtracts or multiplies (a number or date can go out of range)".into()),
                _ => Err("uses an operator the check does not know".into()),
            },
            Expr::Convert(ty, _) => match ty {
                RtType::Text(None) | RtType::Code(None) | RtType::Boolean | RtType::Decimal | RtType::Variant => {
                    Ok(false)
                }
                _ => Err(
                    "converts a value to a type other than Text or Code without a length, Boolean, Decimal or Variant"
                        .into(),
                ),
            },
            Expr::Call {
                object, routine, args, ..
            } => {
                let Some(r) = self
                    .engine
                    .program
                    .objects
                    .get(*object)
                    .and_then(|o| o.routines.get(*routine))
                else {
                    return Ok(false);
                };
                let fixed = args
                    .iter()
                    .zip(&r.locals)
                    .any(|(a, slot)| matches!(a, Arg::Value(_)) && fixed_length(&slot.ty));
                if fixed {
                    return Err("hands a value to a Text or Code parameter of a fixed length".into());
                }
                Ok(false)
            }
            Expr::Builtin {
                id,
                receiver,
                args,
                ret_used,
            } => {
                self.strict_builtin(*id, receiver.as_deref(), args, *ret_used, at, inst)?;
                for a in args {
                    match a {
                        Arg::Value(v) => self.expr(v, at, inst)?,
                        Arg::Ref(p) => self.strict_place(p, at)?,
                    }
                }
                Ok(true)
            }
            _ => Err("runs code that can raise an error or that the check does not know".into()),
        }
    }

    /// Builtin `id` with `args` on `receiver`: a listed one in a form that cannot raise, its receiver read (or, for a find
    /// whose result is used, a local written).
    fn strict_builtin(
        &mut self,
        id: BuiltinId,
        receiver: Option<&Arg>,
        args: &[Arg],
        ret_used: bool,
        at: (usize, usize),
        inst: Inst,
    ) -> Found {
        let full = id.name().split('#').next().unwrap_or(id.name());
        let writes = match class_method(id) {
            // A length or a format argument can raise ("An invalid field or attribute has been specified for the 'Format'
            // property.").
            ("System", "Format") if args.len() == 1 => false,
            ("System", "Format") => return Err("calls System.Format with more than one argument".into()),
            ("System", "TranslatedText" | "TextConstText") => false,
            ("Text", m) if TEXT_READ.contains(&m) => false,
            ("Text", "StrSubstNo") if value_arg(args.first()).is_some_and(constant_or_label) => false,
            ("Text", "StrSubstNo") => {
                return Err("calls Text.StrSubstNo with a format that is not a text constant or a label".into());
            }
            ("Text", "DelChr")
                if args.len() == 1
                    || value_arg(args.get(1))
                        .and_then(const_text)
                        .is_some_and(|w| DELCHR_WHERE.contains(&w)) =>
            {
                false
            }
            ("Text", "DelChr") => {
                return Err(
                    "calls Text.DelChr with a Where argument other than a constant '=', '<', '>' or '<>'".into(),
                );
            }
            ("Record", m) if RECORD_READ.contains(&m) => false,
            ("Record", m) if RECORD_CAPTION_READ.contains(&m) => {
                self.no_caption_class(receiver, at)
                    .map_err(|()| format!("calls {full} on a table whose fields can have a CaptionClass"))?;
                false
            }
            ("Record", "FindFirst" | "FindLast" | "FindSet") if ret_used && args.is_empty() => true,
            ("Record", "Next")
                if ret_used && (args.is_empty() || value_arg(args.first()).is_some_and(const_integer)) =>
            {
                true
            }
            ("Record", "FindFirst" | "FindLast" | "FindSet" | "Next") if ret_used => {
                return Err(format!(
                    "calls {full} with an argument (a lock or a step that is not a constant)"
                ));
            }
            _ => return Err(format!("calls {full}")),
        };
        match receiver {
            None => Ok(()),
            Some(Arg::Ref(p)) if writes => self.strict_place(p, at),
            Some(Arg::Ref(p)) => Self::strict_read(p),
            Some(Arg::Value(_)) if writes => Err(format!("calls {full} on a value, not a variable")),
            Some(Arg::Value(v)) => self.expr(v, at, inst),
        }
    }

    /// The record `receiver` holds is of a table with no table extension and no field with a CaptionClass (literal or
    /// expression), so a caption read of it runs no AL code.
    fn no_caption_class(&self, receiver: Option<&Arg>, at: (usize, usize)) -> Result<(), ()> {
        let place = match receiver {
            Some(Arg::Ref(p) | Arg::Value(Expr::Load(p))) => p,
            _ => return Err(()),
        };
        let Some(RtType::Record(t, _)) = self.place_ty(place, at) else {
            return Err(());
        };
        let schema = self
            .engine
            .program
            .objects
            .get(*t as usize)
            .and_then(|o| o.table.as_ref())
            .ok_or(())?;
        let plain = schema.extensions.is_empty()
            && schema.fields.iter().all(|f| {
                f.caption_class.is_none()
                    && f.caption_class_refusal.is_none()
                    && !f.properties.iter().any(|(k, _)| k.eq_ignore_ascii_case("CaptionClass"))
            });
        if plain { Ok(()) } else { Err(()) }
    }
}
