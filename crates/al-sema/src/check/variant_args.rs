//! Arguments that take a Variant overload over a typed one.
use super::*;

impl Checker<'_> {
    /// For the next `pick`: the arguments that are a bare field name (`field_only_args`) and those that are a
    /// `FieldRef.Value()` call (`variant_args`).
    pub(crate) fn set_pick_args(&mut self, sc: &Scope, field_only: Vec<usize>, args: &[SyntaxNode]) {
        self.field_only_args = field_only;
        self.variant_args = (0..args.len())
            .filter(|&i| self.is_fieldref_value(sc, &args[i]))
            .collect();
    }

    /// Whether `arg`, already checked, is a `FieldRef.Value()` call. Passed to procedures overloaded on Integer and Variant, or
    /// on Decimal and Variant, it takes the Variant one whatever type the field has (types/lt-y62-undefined-date/
    /// Y62UndefinedDate.al:FieldRefValue_OverloadPick_EmptyAndSet, writer Y62: Date, Time, DateTime, Duration and Decimal fields,
    /// empty and set).
    pub(crate) fn is_fieldref_value(&self, sc: &Scope, arg: &SyntaxNode) -> bool {
        let r = arg.text_range();
        matches!(
            self.resolutions.get(&(sc.file, r.start().into(), r.end().into())).map(Res::inner),
            Some(Res::Builtin(id)) if id == "FieldRef.Value#1"
        )
    }
}

impl Checker<'_> {
    /// AL0122: the result of a built-in method that returns nothing is assigned. Pinned for `HttpContent.Clear()`
    /// (must-not-compile 2615-httpcontent-clear-result-used.al: `Result := Req.Content.Clear();` into a Boolean). Other
    /// built-ins without a result are not judged here.
    pub(crate) fn al_no_result_builtin_assignment(&mut self, sc: &Scope, rhs: &SyntaxNode, target: &Ty) {
        let r = rhs.text_range();
        let cleared = matches!(
            self.resolutions.get(&(sc.file, r.start().into(), r.end().into())).map(Res::inner),
            Some(Res::Builtin(id)) if id == "HttpContent.Clear#1"
        );
        if cleared && !self.ws.is_dependency(sc.oi) && rhs.kind() == SyntaxKind::CallExpr {
            self.diag(
                sc,
                rhs,
                "AL0122",
                format!("Cannot implicitly convert type 'None' to '{target}'"),
            );
        }
    }
}
