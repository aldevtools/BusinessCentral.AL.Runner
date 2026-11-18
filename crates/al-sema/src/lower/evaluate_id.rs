//! Which implementation `Evaluate` (`System.Evaluate#1`) lowers to, chosen by its target.
use super::*;

impl Lowerer<'_> {
    /// The builtin id `Evaluate(target, ...)` runs; `field_captions`: the target is an Option field whose captions
    /// `option_field_captions` found.
    pub(super) fn evaluate_id(&self, target: Option<&SyntaxNode>, field_captions: bool) -> &'static str {
        let Some(a) = target else {
            return "System.Evaluate#1";
        };
        if field_captions {
            return "System.EvaluateOptionField#1";
        }
        if let Some(id) = self.memberless_var_option_parameter(a) {
            return id;
        }
        // Evaluate into an Option variable: a variable has no OptionCaption, so a text that is neither an ordinal nor a
        // member name is a plain rejection (P86Eval.al:Eval Option, `Bet`, `-1`, `1,0`). A field may name a member by its
        // caption, which the value does not carry, so a field keeps the id that refuses it.
        // An element of an Option array variable reads as the variable does: an unknown name answers false and keeps the
        // element (types/arrays/PnC14OptionArray.al:51-78, OptionArray_Evaluate_And_Out_Of_Range).
        let var = if a.kind() == SyntaxKind::IndexExpr {
            a.children().next()
        } else {
            Some(a.clone())
        };
        if matches!(self.ty_of(a), Ty::Option(_))
            && var.is_some_and(|v| matches!(self.res(&v), Some(Res::Local(_) | Res::Global(_))))
        {
            return "System.EvaluateVariableOption#1";
        }
        if matches!(self.ty_of(a), Ty::Text(Some(_)) | Ty::Code(Some(_))) {
            return "System.EvaluateBounded#1";
        }
        "System.Evaluate#1"
    }

    /// The target is a `var` parameter declared `Option` without members (al-runner issue 7994; the type reads as one
    /// nameless member, as `Option ""` does, `ast::TypeRef::option_members`). Named directly: BC reads the caller's members
    /// there (Microsoft's System Application test `Json Test`.TestGetEnumPropertyValueFromJObjectByName passes on BC 28.4,
    /// measured in the issue); the runner hands the parameter none, so `evaluate_memberless_var_option` refuses what would
    /// need them. An element of a `var` array parameter of such Options: the element has the caller's members for the
    /// Evaluate (measured on main by the round-2 review), but the store back drops them and the caller would show the
    /// number (issue 8291), so `evaluate_memberless_var_option_element` refuses with that reason.
    fn memberless_var_option_parameter(&self, a: &SyntaxNode) -> Option<&'static str> {
        let (name, id) = match a.kind() {
            SyntaxKind::NameRef => (a.clone(), "System.EvaluateMemberlessVarOption#1"),
            SyntaxKind::IndexExpr => match a.children().next() {
                Some(n) if n.kind() == SyntaxKind::NameRef => (n, "System.EvaluateMemberlessVarOptionElement#1"),
                _ => return None,
            },
            _ => return None,
        };
        (matches!(self.ty_of(a), Ty::Option(m) if m.len() == 1 && m[0].is_empty())
            && matches!(self.res(&name), Some(Res::Local(n))
                if self.locals.get(n).is_some_and(|i| self.var_params.get(*i).copied() == Some(true))))
        .then_some(id)
    }
}
