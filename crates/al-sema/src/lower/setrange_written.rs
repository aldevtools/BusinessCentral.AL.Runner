//! The id of a `FieldRef.SetRange`, `FieldRef.SetFilter` or `Record.SetRange` call by how its values are written. BC
//! converts a value of another type than the field's by it (al-vm `setrange_convert`): a Text into a Guid field raises
//! the Guid format error when it is written as a Text, and the Evaluate error when it is written as a Variant. The
//! catalog id stays for every shape that is not listed here; the VM keeps its conversion for it.

use super::*;

/// How the values of a call are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Written {
    /// Every value has a plain value type (a number, Boolean, text, date, time, Guid, Duration, Option or Enum).
    Typed,
    /// Every value is a Variant variable or expression (not a `FieldRef.Value()` call).
    Variant,
    /// Plain values and Variants, at least one of each.
    Mixed,
    /// Anything else: a `FieldRef.Value()` call, a FieldRef, a Label, a type not known, or no value.
    Other,
}

impl Lowerer<'_> {
    /// How the values `args` are written.
    fn values_written(&self, args: &[SyntaxNode]) -> Written {
        let fieldref_value = |a: &SyntaxNode| matches!(self.res(a), Some(Res::Builtin(i)) if i == "FieldRef.Value#1");
        if args.is_empty() || args.iter().any(fieldref_value) {
            return Written::Other;
        }
        let plain = |t: &Ty| {
            matches!(
                t,
                Ty::Integer
                    | Ty::BigInteger
                    | Ty::Decimal
                    | Ty::Boolean
                    | Ty::Text(_)
                    | Ty::Code(_)
                    | Ty::Date
                    | Ty::Time
                    | Ty::DateTime
                    | Ty::Duration
                    | Ty::Guid
                    | Ty::Option(_)
                    | Ty::Enum(_)
            )
        };
        let tys: Vec<Ty> = args.iter().map(|a| self.ty_of(a)).collect();
        let variants = tys.iter().filter(|t| matches!(t, Ty::Variant)).count();
        let plains = tys.iter().filter(|t| plain(t)).count();
        match (variants, plains) {
            (v, 0) if v == tys.len() => Written::Variant,
            (0, p) if p == tys.len() => Written::Typed,
            (v, p) if v + p == tys.len() => Written::Mixed,
            _ => Written::Other,
        }
    }

    /// The id lowering gives the call `id` with arguments `args`, `None` for the catalog id. Only the shapes the corpus
    /// probe of al-runner issue 8857 measures take another id: `FieldRef.SetRange` with typed, Variant or mixed values,
    /// `FieldRef.SetFilter` with one typed or Variant value, `Record.SetRange` with one Variant value.
    pub(super) fn setrange_written_id(&self, id: &str, args: &[SyntaxNode]) -> Option<&'static str> {
        match (id, args.len()) {
            ("FieldRef.SetRange#1", _) => match self.values_written(args) {
                Written::Typed => Some("FieldRef.SetRangeTyped#1"),
                Written::Variant => Some("FieldRef.SetRangeVariant#1"),
                Written::Mixed => Some("FieldRef.SetRangeMixed#1"),
                Written::Other => None,
            },
            ("FieldRef.SetFilter#1", 2) => match self.values_written(&args[1..]) {
                Written::Typed => Some("FieldRef.SetFilterTyped#1"),
                Written::Variant => Some("FieldRef.SetFilterVariant#1"),
                Written::Mixed | Written::Other => None,
            },
            ("Record.SetRange#1", 2) => match self.values_written(&args[1..]) {
                Written::Variant => Some("Record.SetRangeVariant#1"),
                _ => None,
            },
            _ => None,
        }
    }
}
