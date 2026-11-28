//! `I += FR.Value()` and `I -= FR.Value()`: an Integer and the Joker a `FieldRef.Value()` call is typed have no `+=` or
//! `-=` (al-runner issue 9283).
//!
//! The AL compiler reports AL0175 at the statement, at runtime 16.0 and 18.0 alike; `I := FR.Value()` compiles. No
//! corpus case pins it yet, so it rests on the assumption `integer-compound-assign-fieldref-value` (must-not-compile
//! 3834 and 3835 are its probe). Judged only in the measured shape: an Integer variable to the left, a `Value()` call
//! without arguments on a FieldRef to the right.
use super::*;
use al_builtins::assumptions::Assumption;

impl Checker<'_> {
    /// AL0175 for the assignment statement `stmt`, `lhs += rhs` or `lhs -= rhs`, where `lhs` is an Integer variable
    /// (`target` its type) and `rhs` a `FieldRef.Value()` call.
    pub(crate) fn al_integer_compound_joker(
        &mut self,
        sc: &Scope,
        stmt: &SyntaxNode,
        lhs: &SyntaxNode,
        target: &Ty,
        rhs: &SyntaxNode,
    ) {
        let _rests_on = Assumption::IntegerCompoundAssignFieldrefValue;
        let op = al_syntax::ast::significant_green_tokens(stmt).find_map(|t| match t.kind {
            SyntaxKind::PlusAssign => Some("+="),
            SyntaxKind::MinusAssign => Some("-="),
            _ => None,
        });
        let Some(op) = op else { return };
        let no_args = rhs
            .children()
            .nth(1)
            .is_some_and(|list| list.children().next().is_none());
        if matches!(target, Ty::Integer)
            && lhs.kind() == SyntaxKind::NameRef
            && sc_has_var(sc, &name_of(lhs).to_lowercase())
            && rhs.kind() == SyntaxKind::CallExpr
            && no_args
            && self.is_fieldref_value(sc, rhs)
            && !self.in_dependency(sc.file)
        {
            self.diag(
                sc,
                stmt,
                "AL0175",
                format!("Operator '{op}' cannot be applied to operands of type 'Integer' and 'Joker'"),
            );
        }
    }
}
