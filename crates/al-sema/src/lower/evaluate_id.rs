//! Which implementation `Evaluate` (`System.Evaluate#1`) lowers to, chosen by its target.
//!
//! Which builtin `Evaluate` lowers to, by its target (an Option field with captions, a `var` Option
//! parameter declared without members or an element of a `var` array of them, an Option variable, an Enum variable or field, a Date variable, a
//! bounded Text or Code). Entry points: `evaluate_id`.
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
        // A Date or DateTime local variable or parameter: the readers of corpus PR 1503 (al-builtins `evaluate::short_us_date`)
        // were measured on these targets only; a field, a global and an array element keep the plain id.
        if matches!(self.ty_of(a), Ty::Date | Ty::DateTime) && matches!(self.res(a), Some(Res::Local(_))) {
            return "System.EvaluateDateVariable#1";
        }
        // An Enum variable of the routine's `var` section, and an Enum record field named directly: the blank-text readers of
        // al-builtins `evaluate::enum_text` were measured on these targets only (corpus PR 1306, EvalNamedBlankCaption.al:81-107;
        // corpus PR 1577, EvalOptionBlankCaptions.al OBC_Enum_*; al-runner issue 8356). A global, a parameter, a return value and
        // an array element keep the plain id.
        if matches!(self.ty_of(a), Ty::Enum(_)) {
            if self.declared_local(a) {
                return "System.EvaluateEnumVariable#1";
            }
            if matches!(self.res(a), Some(Res::Field { .. })) {
                return "System.EvaluateEnumField#1";
            }
        }
        if matches!(self.ty_of(a), Ty::Text(Some(_)) | Ty::Code(Some(_))) {
            return "System.EvaluateBounded#1";
        }
        "System.Evaluate#1"
    }

    /// `a` names a variable of the routine's `var` section: a slot past the parameters that is not the named return value.
    fn declared_local(&self, a: &SyntaxNode) -> bool {
        let Some(Res::Local(n)) = (a.kind() == SyntaxKind::NameRef).then(|| self.res(a)).flatten() else {
            return false;
        };
        self.locals
            .get(n)
            .is_some_and(|&slot| slot >= self.var_params.len() && Some(slot) != self.named_ret)
    }

    /// The target is a `var` parameter declared `Option` without members, or an element of a `var` array parameter of such
    /// Options (`var_option_arg::memberless_var_option_param`): it reads the caller's members, the Evaluate natives of these ids
    /// read the text against them (al-builtins `evaluate::option_text`).
    fn memberless_var_option_parameter(&self, a: &SyntaxNode) -> Option<&'static str> {
        let id = match a.kind() {
            SyntaxKind::NameRef => "System.EvaluateMemberlessVarOption#1",
            SyntaxKind::IndexExpr => "System.EvaluateMemberlessVarOptionElement#1",
            _ => return None,
        };
        self.memberless_var_option_param(a).map(|_| id)
    }
}
