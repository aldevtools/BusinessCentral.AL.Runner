//! Whether a page's DataCaptionExpression can raise an error or leave something behind when it runs (al-runner issue 6924).
//!
//! BC runs the expression of a Worksheet at the open (corpus pages/properties/DataCaptionExprWorksheet.al:412, :439, :572,
//! :586); main's path runs it only at a caption read. An expression that can neither raise nor write anything but its own
//! locals changes nothing AL code or a test can see except the caption, so the page can keep main's open and refuse at the
//! caption read (`pages/data_caption_open.rs`). Everything else refuses at the open.
//!
//! The walk is [`Walk`] with `strict` set: on top of what [`Engine::lasting_effect`] counts, every write but one to a local of a
//! type without a length counts, and every construct that can raise. Only what is listed here is known not to raise; anything
//! else, and any construct the walk does not know, counts.

use super::{Found, Inst, Walk};
use crate::Engine;
use al_builtins::builtin_id::BuiltinId;
use al_builtins::value::RtType;
use al_ir::{Arg, BinOp, Expr, Place, Stmt, UnOp};
use std::collections::HashSet;

/// Builtins without a receiver, and methods of Text, that read their arguments and cannot raise an error.
const SAFE_VALUE: &[&str] = &[
    "System.Format",
    "System.TextConstText",
    "System.TranslatedText",
    "Text.Contains",
    "Text.DelChr",
    "Text.EndsWith",
    "Text.LowerCase",
    "Text.MaxStrLen",
    "Text.StartsWith",
    "Text.StrLen",
    "Text.StrSubstNo",
    "Text.ToLower",
    "Text.ToUpper",
    "Text.Trim",
    "Text.TrimEnd",
    "Text.TrimStart",
    "Text.UpperCase",
];

/// Record methods that only read the record variable and the database and cannot raise an error.
const SAFE_RECORD_READ: &[&str] = &[
    "Count",
    "CountApprox",
    "CurrentKey",
    "FieldCaption",
    "FieldName",
    "FieldNo",
    "GetFilter",
    "GetFilters",
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

/// Record methods that load a row into the record variable and, when their result is used, give false instead of raising.
const SAFE_RECORD_FIND: &[&str] = &["FindFirst", "FindLast", "FindSet", "Next"];

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

impl Engine {
    /// What a run of routine `routine` of object `object` (a page's DataCaptionExpression) can do that a page open has to
    /// answer for: raise an error or leave anything behind but its own locals. `None`: neither.
    pub(crate) fn caption_open_risk(&self, object: usize, routine: usize) -> Option<String> {
        let mut w = Walk {
            engine: self,
            seen: HashSet::new(),
            kept: None,
            strict: true,
        };
        // The page's own instance outlives the run.
        w.routine(object, routine, Inst::Lasting).err()
    }
}

impl Walk<'_> {
    /// A routine an event publisher is: it raises the event, and a subscriber may do anything.
    pub(super) fn strict_routine(&self, at: (usize, usize)) -> Found {
        if self.engine.events.publishers.contains_key(&at) {
            return Err(format!(
                "raises the event {}",
                self.engine.program.objects[at.0].routines[at.1].name
            ));
        }
        Ok(())
    }

    fn local_ty(&self, at: (usize, usize), slot: usize) -> Option<&RtType> {
        self.engine.program.objects[at.0].routines[at.1]
            .locals
            .get(slot)
            .map(|s| &s.ty)
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
            Expr::Binary(op, ..) => match op {
                BinOp::Add
                | BinOp::And
                | BinOp::Or
                | BinOp::Xor
                | BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge => Ok(false),
                BinOp::Div | BinOp::IntDiv | BinOp::Mod => Err("divides".into()),
                _ => Err(format!("uses the operator {op:?}")),
            },
            Expr::Convert(ty, _) => match ty {
                RtType::Text(None) | RtType::Code(None) | RtType::Boolean | RtType::Decimal | RtType::Variant => {
                    Ok(false)
                }
                _ => Err(format!("converts a value to {ty:?}")),
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
                self.strict_builtin(*id, receiver.as_deref(), *ret_used, at, inst)?;
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

    /// Builtin `id` on `receiver`: a listed one, its receiver read (or, for a find whose result is used, a local written).
    fn strict_builtin(
        &mut self,
        id: BuiltinId,
        receiver: Option<&Arg>,
        ret_used: bool,
        at: (usize, usize),
        inst: Inst,
    ) -> Found {
        let (class, method) = class_method(id);
        let full = id.name().split('#').next().unwrap_or(id.name());
        let writes = if SAFE_VALUE.contains(&full) || (class == "Record" && SAFE_RECORD_READ.contains(&method)) {
            false
        } else if class == "Record" && ret_used && SAFE_RECORD_FIND.contains(&method) {
            true
        } else {
            return Err(format!("calls {full}"));
        };
        match receiver {
            None => Ok(()),
            Some(Arg::Ref(p)) if writes => self.strict_place(p, at),
            Some(Arg::Ref(p)) => Self::strict_read(p),
            Some(Arg::Value(_)) if writes => Err(format!("calls {full} on a value, not a variable")),
            Some(Arg::Value(v)) => self.expr(v, at, inst),
        }
    }
}
