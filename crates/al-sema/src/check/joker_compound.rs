//! A compound assignment whose right side is the `Value` of a FieldRef: the value is typed Joker, and no compound
//! assignment takes a Joker (al-runner issue 9283).
//!
//! The AL compilers of BC 27, 28 and 29 report AL0175 (`Operator '<op>' cannot be applied to operands of type '<left>'
//! and 'Joker'`) at the statement; `I := FR.Value()` compiles (tests/al-language/record/PZFindDirections.al:76). No
//! pinned corpus case has it yet, so it rests on the assumption `compound-assign-fieldref-value` (must-not-compile 3834,
//! 3835 and 3850-3861 on corpus pull request 1645 are its probe).
//!
//! Judged in the measured shapes only, each fact measured on its own:
//! - the left side a variable in scope (a local, a parameter, a `var` parameter or a global) of type Integer with `+=`,
//!   `-=`, `*=` or `/=`, or of type Decimal, BigInteger or Text (no length) with `+=`;
//! - the right side `<FieldRef>.Value()` without arguments or `<FieldRef>.Value` without parentheses, the FieldRef a
//!   variable, a parameter, `RecRef.Field(n)` or an array element.
use super::*;
use al_builtins::assumptions::Assumption;

impl Checker<'_> {
    /// AL0175 for the assignment statement `stmt`, `lhs <op>= rhs`, where `lhs` (`target` its type) and `rhs` are of
    /// the measured shapes.
    pub(crate) fn al_compound_joker(
        &mut self,
        sc: &Scope,
        stmt: &SyntaxNode,
        lhs: &SyntaxNode,
        target: &Ty,
        rhs: &SyntaxNode,
    ) {
        let _rests_on = Assumption::CompoundAssignFieldrefValue;
        let op = al_syntax::ast::significant_green_tokens(stmt).find_map(|t| match t.kind {
            SyntaxKind::PlusAssign => Some("+="),
            SyntaxKind::MinusAssign => Some("-="),
            SyntaxKind::StarAssign => Some("*="),
            SyntaxKind::SlashAssign => Some("/="),
            _ => None,
        });
        let Some(op) = op else { return };
        let left = match target {
            Ty::Integer => "Integer",
            Ty::Decimal if op == "+=" => "Decimal",
            Ty::BigInteger if op == "+=" => "BigInteger",
            Ty::Text(None) if op == "+=" => "Text",
            _ => return,
        };
        if lhs.kind() == SyntaxKind::NameRef
            && sc_has_var(sc, &name_of(lhs).to_lowercase())
            && self.is_fieldref_value_read(sc, rhs)
            && !self.in_dependency(sc.file)
        {
            self.diag(
                sc,
                stmt,
                "AL0175",
                format!("Operator '{op}' cannot be applied to operands of type '{left}' and 'Joker'"),
            );
        }
    }

    /// Whether `rhs` reads a FieldRef's value: `X.Value()` without arguments or `X.Value` without parentheses.
    fn is_fieldref_value_read(&self, sc: &Scope, rhs: &SyntaxNode) -> bool {
        match rhs.kind() {
            SyntaxKind::CallExpr => {
                let no_args = rhs
                    .children()
                    .nth(1)
                    .is_some_and(|list| list.children().next().is_none());
                no_args && self.is_fieldref_value(sc, rhs)
            }
            SyntaxKind::MemberExpr => self.is_fieldref_value(sc, rhs),
            _ => false,
        }
    }
}
