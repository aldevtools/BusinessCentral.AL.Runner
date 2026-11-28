//! A compound assignment whose right side is the `Value` of a FieldRef: the value is typed Joker, and no compound
//! assignment takes a Joker (al-runner issue 9283).
//!
//! The AL compilers of BC 27, 28 and 29 report AL0175 (`Operator '<op>' cannot be applied to operands of type '<left>'
//! and 'Joker'`) at the statement; `I := FR.Value()` compiles (tests/al-language/record/PZFindDirections.al:76). No
//! pinned corpus case has it yet, so it rests on the assumption `compound-assign-fieldref-value` (must-not-compile
//! 3834, 3835, 3850-3861 and 3869-3875 on corpus pull request 1645 are its probe).
//!
//! Judged in the measured shapes only, each fact measured on its own:
//! - the left side a variable in scope (a local, a parameter, a `var` parameter, a global or a named return value) of
//!   type Integer with `+=`, `-=`, `*=` or `/=`, or of type Decimal, BigInteger or Text (no length) with `+=`;
//! - the right side `<FieldRef>.Value()` without arguments, the FieldRef a variable in scope (a local, a parameter, a
//!   `var` parameter, a global or a named return value), an element of an array of FieldRef,
//!   `RecRef.Field(<number>)`, `RecRef.FieldIndex(<n>)`, `KeyRef.FieldIndex(<n>)` or a call of a procedure that
//!   returns a FieldRef; or `<FieldRef>.Value` without parentheses on a FieldRef variable in scope.
//!
//! Any other form (another left type or operator, a field on the left, another receiver) compiles as before.
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

    /// Whether `rhs` reads a FieldRef's value in a measured form: `X.Value()` without arguments or `X.Value`, X one of
    /// the measured receivers.
    fn is_fieldref_value_read(&self, sc: &Scope, rhs: &SyntaxNode) -> bool {
        if !self.is_fieldref_value(sc, rhs) {
            return false;
        }
        let (member, bare) = match rhs.kind() {
            SyntaxKind::CallExpr => {
                let no_args = rhs
                    .children()
                    .nth(1)
                    .is_some_and(|list| list.children().next().is_none());
                match rhs.children().next() {
                    Some(m) if no_args => (m, false),
                    _ => return false,
                }
            }
            SyntaxKind::MemberExpr => (rhs.clone(), true),
            _ => return false,
        };
        let Some(receiver) = member
            .children()
            .next()
            .filter(|_| member.kind() == SyntaxKind::MemberExpr)
        else {
            return false;
        };
        let in_scope = |n: &SyntaxNode| n.kind() == SyntaxKind::NameRef && sc_has_var(sc, &name_of(n).to_lowercase());
        if bare {
            // 3859: `I += FR.Value`, FR a local.
            return in_scope(&receiver);
        }
        match receiver.kind() {
            // 3834, 3855, 3875, 3872, 3874: a local, a parameter, a `var` parameter, a global or a named return value.
            SyntaxKind::NameRef => in_scope(&receiver),
            // 3854: an element of an array of FieldRef.
            SyntaxKind::IndexExpr => receiver.children().next().is_some_and(|base| in_scope(&base)),
            // 3853, 3869, 3870: `RecRef.Field(<number>)`, `RecRef.FieldIndex(<n>)`, `KeyRef.FieldIndex(<n>)`; 3871: a
            // procedure that returns a FieldRef.
            SyntaxKind::CallExpr => {
                let r = receiver.text_range();
                matches!(
                    self.resolutions.get(&(sc.file, r.start().into(), r.end().into())).map(Res::inner),
                    Some(Res::Builtin(id))
                        if id == "RecordRef.Field#2" || id == "RecordRef.FieldIndex#1" || id == "KeyRef.FieldIndex#1"
                ) || matches!(
                    self.resolutions
                        .get(&(sc.file, r.start().into(), r.end().into()))
                        .map(Res::inner),
                    Some(Res::Proc { .. })
                )
            }
            _ => false,
        }
    }
}
