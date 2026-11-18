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

    /// The target is a `var` parameter declared `Option` without members, or an element of a `var` array parameter of such
    /// Options (`var_option_arg::memberless_var_option_param`): it reads the caller's members, the Evaluate natives of these ids
    /// read the text against them (al-builtins `evaluate::option_text`, assumption var-option-caller-members).
    fn memberless_var_option_parameter(&self, a: &SyntaxNode) -> Option<&'static str> {
        let id = match a.kind() {
            SyntaxKind::NameRef => "System.EvaluateMemberlessVarOption#1",
            SyntaxKind::IndexExpr => "System.EvaluateMemberlessVarOptionElement#1",
            _ => return None,
        };
        self.memberless_var_option_param(a).map(|_| id)
    }
}
