//! The cells of the run matrix of implicit conversions (`al_builtins::conv`) seen from the lowering: which pairs of types
//! a conversion the matrix pins joins.

use crate::types::Ty;
use al_builtins::conv::{Family, Kind, verdict};

/// The kind of the matrix a type is, `None` for a type the matrix does not try.
pub(super) fn kind(ty: &Ty) -> Option<Kind> {
    Some(match ty {
        Ty::Integer => Kind::Integer,
        Ty::BigInteger => Kind::BigInteger,
        Ty::Decimal => Kind::Decimal,
        Ty::Char => Kind::Char,
        Ty::Byte => Kind::Byte,
        Ty::Option(_) => Kind::Option,
        Ty::Enum(_) => Kind::Enum,
        Ty::Text(_) => Kind::Text,
        Ty::Code(_) => Kind::Code,
        Ty::Label => Kind::Label,
        Ty::Guid => Kind::Guid,
        Ty::Duration => Kind::Duration,
        Ty::Boolean => Kind::Boolean,
        Ty::Date => Kind::Date,
        Ty::Time => Kind::Time,
        Ty::DateTime => Kind::DateTime,
        Ty::Variant => Kind::Variant,
        _ => return None,
    })
}

/// Whether the matrix holds the cell (`a`, `b`) of `family`.
pub(super) fn covers(family: Family, a: &Ty, b: &Ty) -> bool {
    matches!((kind(a), kind(b)), (Some(a), Some(b)) if verdict(family, a, b).is_some())
}

impl super::Lowerer<'_> {
    /// An element of `in [...]` or a case label `n` (of `family`, `CmpIn` or `CmpCase`) whose type differs from the selector's
    /// `sel` and whose pair the matrix holds: the element converts to the selector's type before the comparison
    /// (types/operators-expressions/Y31ConvCmp.al: `Integer_Decimal` 1 in [1.5] raises ConversionCast to Int32,
    /// `Code10_Text` 'ABC' in ['abc'] is true, `Text10_Text` a 36-character element raises StringLengthExceeded, `Text_Guid`
    /// 'a' in [Guid] is false). The matrix tries variables only: a literal or a range keeps the plain comparison (`None`).
    pub(super) fn matrix_set_elem(
        &mut self,
        family: Family,
        sel: &Ty,
        n: &al_syntax::SyntaxNode,
    ) -> super::L<Option<al_ir::SetElem>> {
        use al_syntax::SyntaxKind;
        if matches!(n.kind(), SyntaxKind::RangeExpr | SyntaxKind::Literal) {
            return Ok(None);
        }
        let ty = self.ty_of(n);
        if &ty == sel || !covers(family, sel, &ty) {
            return Ok(None);
        }
        let e = self.expr(n)?;
        Ok(Some(al_ir::SetElem::One(self.convert_to(sel, &ty, e)?)))
    }
}

impl super::Lowerer<'_> {
    /// Whether `yes ? ... : no` hands the Variant a TextConstant when the Label branch is taken: a Label before the colon against
    /// another Label or against a Text or Code that is no literal (types/Y81LabelVariant.al routes 1, 4, 6, 8, 10 and 13
    /// give IsTextConstant for the Label branch; routes 2, 3, 5, 7, 9, 11 and 12, with a literal or the Label second, give Text or
    /// Code; needs corpus bump).
    fn label_branch_is_text_constant(&self, yes: &al_syntax::SyntaxNode, no: &al_syntax::SyntaxNode) -> bool {
        let literal = no.kind() == al_syntax::SyntaxKind::Literal
            && al_syntax::ast::significant_green_tokens(no).any(|t| t.kind() == al_syntax::SyntaxKind::StringLit);
        matches!(self.ty_of(yes), Ty::Label)
            && match self.ty_of(no) {
                Ty::Label => true,
                Ty::Text(_) | Ty::Code(_) => !literal,
                _ => false,
            }
    }

    /// `node` lowered for a slot of type `target`: a conditional with a Label branch that gives a Variant a TextConstant is built
    /// branch by branch (the Label branch is a TextConstant, the other branch a Variant of its own kind), anything else as `expr`.
    pub(super) fn expr_into(&mut self, target: &Ty, node: &al_syntax::SyntaxNode) -> super::L<al_ir::Expr> {
        let mut inner = node.clone();
        while inner.kind() == al_syntax::SyntaxKind::ParenExpr {
            match inner.children().next() {
                Some(next) => inner = next,
                None => break,
            }
        }
        let kids: Vec<al_syntax::SyntaxNode> = inner.children().collect();
        if !matches!(target, Ty::Variant) || inner.kind() != al_syntax::SyntaxKind::TernaryExpr || kids.len() != 3 {
            return self.expr(node);
        }
        if !self.label_branch_is_text_constant(&kids[1], &kids[2]) {
            return self.expr(node);
        }
        let cond = self.expr(&kids[0])?;
        let yes = self.expr(&kids[1])?;
        let yes = al_ir::Expr::Builtin {
            id: super::call::builtin_id("Variant.FromTextConstant#1")?,
            receiver: None,
            args: vec![al_ir::Arg::Value(yes)],
            ret_used: true,
        };
        let no_ty = self.ty_of(&kids[2]);
        let no = self.expr(&kids[2])?;
        let no = if matches!(no_ty, Ty::Label) {
            al_ir::Expr::Builtin {
                id: super::call::builtin_id("Variant.FromTextConstant#1")?,
                receiver: None,
                args: vec![al_ir::Arg::Value(no)],
                ret_used: true,
            }
        } else {
            self.convert_to(&Ty::Variant, &no_ty, no)?
        };
        Ok(al_ir::Expr::Ternary(Box::new(cond), Box::new(yes), Box::new(no)))
    }

    /// `V := B ? x : y` into a Variant with a Label branch and one of another type: in BC the Variant holds the branch taken
    /// with its own type, a Label answering no Is... test and a Code staying a Code (types/operators-expressions/
    /// Y31ConvCd.al, LT Y31 Cd Label `Label_Text`, `Label_Code10`: `|abc` and `Code|A`). A Label is a Text in a Variant here
    /// (`refuse_textconst_into_variant`), so the shape is refused.
    pub(super) fn refuse_label_ternary_into_variant(
        &mut self,
        target: &Ty,
        rhs: &al_syntax::SyntaxNode,
    ) -> super::L<()> {
        if !matches!(target, Ty::Variant) || rhs.kind() != al_syntax::SyntaxKind::TernaryExpr {
            return Ok(());
        }
        let kids: Vec<al_syntax::SyntaxNode> = rhs.children().collect();
        let (Some(yes), Some(no)) = (kids.get(1), kids.get(2)) else {
            return Ok(());
        };
        let (t, f) = (self.ty_of(yes), self.ty_of(no));
        // A Guid against a Label is pinned into a Variant (types/guid/UfTernaryGuidLabel.al:24-104).
        let guid = matches!(t, Ty::Guid) || matches!(f, Ty::Guid) || self.label_branch_is_text_constant(yes, no)
            // A Label against a text literal is a Text whichever branch is taken (Y81LabelVariant.al routes 2 and 12).
            || (matches!(t, Ty::Label) && matches!(f, Ty::Text(_) | Ty::Code(_)));
        // A Label false branch after a Text or Code true branch takes the true branch's type: `Text|abc` and `Code|ABC`
        // (Y31ConvCd.al Text10_Label :3187-3212, Code10_Label :3660-3700).
        let label_takes_text = matches!(f, Ty::Label) && matches!(t, Ty::Text(_) | Ty::Code(_));
        // Two Labels hold a TextConstant in BC (types/text-code-char/Y66TernaryLabel.al:1029-1180, `LblMax : Lbl` and
        // `Lbl : LblMax`, real BC values not on corpus master yet), which is not built either.
        let two_labels = false;
        if (t != f || two_labels) && !guid && !label_takes_text && (matches!(t, Ty::Label) || matches!(f, Ty::Label)) {
            return Err(format!(
                "a conditional expression with a Label branch ({t} and {f}) into a Variant: the Variant holds a Label kind that is not built"
            ));
        }
        Ok(())
    }
}

impl super::Lowerer<'_> {
    /// The shapes refused for an argument handed to a Variant parameter.
    pub(super) fn refuse_variant_argument(&self, target: &Ty, node: &al_syntax::SyntaxNode) -> super::L<()> {
        self.refuse_session_ending_ternary(target, node)?;
        self.refuse_textconst_into_variant(target, node)
    }

    /// `Cond ? Label : 'literal'` (a Label before the colon, a text literal after it) as the exit value of a Variant function
    /// or as the argument of a Variant parameter: BC 29 ends the whole session when the codeunit holding it is loaded, even if
    /// the routine is never called (writer Y81, types/Y81LabelVariant.al codeunit 94259 "LT Y81 Label Variant
    /// Held", Suspect_ReturnLabelOrLiteral and Suspect_ArgumentLabelOrLiteral; needs corpus bump). The literal-first order,
    /// two Labels, an assignment and a named return value do not crash and are not refused here.
    pub(super) fn refuse_session_ending_ternary(&self, target: &Ty, node: &al_syntax::SyntaxNode) -> super::L<()> {
        let mut node = node.clone();
        while node.kind() == al_syntax::SyntaxKind::ParenExpr {
            match node.children().next() {
                Some(inner) => node = inner,
                None => break,
            }
        }
        if !matches!(target, Ty::Variant) || node.kind() != al_syntax::SyntaxKind::TernaryExpr {
            return Ok(());
        }
        let kids: Vec<al_syntax::SyntaxNode> = node.children().collect();
        let (Some(yes), Some(no)) = (kids.get(1), kids.get(2)) else {
            return Ok(());
        };
        let literal = no.kind() == al_syntax::SyntaxKind::Literal
            && al_syntax::ast::significant_green_tokens(no).any(|t| t.kind() == al_syntax::SyntaxKind::StringLit);
        if matches!(self.ty_of(yes), Ty::Label) && literal {
            return Err(
                "a conditional expression `Cond ? Label : 'literal'` as a Variant argument or the exit value of a \
                 Variant function: Business Central 29 ends the whole session when the codeunit is loaded (refused statically)"
                    .into(),
            );
        }
        Ok(())
    }
}

impl super::Lowerer<'_> {
    /// `G += T` with a Guid `G` and a Text, Code or Label `T`: the Guid stays as it was, whatever the text, a text that is no
    /// GUID included (types/operators-expressions/Y31ConvAr.al, LT Y31 Ar Text `Text_Guid`, Text10, Code10, Label:
    /// `compound_add`). The right side is still evaluated.
    pub(super) fn guid_plus_text_is_kept(&self, target: &Ty, rhs: &al_syntax::SyntaxNode) -> bool {
        matches!(target, Ty::Guid) && covers(Family::AddAssign, &self.ty_of(rhs), target)
    }
}

impl super::Lowerer<'_> {
    /// `value`, of type Variant, wrapped in `Variant.Into#1` when it goes into a slot of a kind the matrix tries from a
    /// Variant (`al_builtins::conv::variant_into`): the cells pinned for a Variant apply to it, and to no other dynamic value.
    pub(super) fn variant_into(&self, target: &Ty, value: al_ir::Expr) -> super::L<al_ir::Expr> {
        // A RecordId, RecordRef or Record slot takes the errors pinned for a Variant too (`conv::record_slot_code`).
        let code = match kind(target).and_then(al_builtins::conv::kind_code) {
            Some(code) => code,
            None => match self
                .rt(target)
                .ok()
                .as_ref()
                .and_then(al_builtins::conv::record_slot_code)
            {
                Some(code) => code,
                None => return Ok(value),
            },
        };
        Ok(al_ir::Expr::Builtin {
            id: super::call::builtin_id("Variant.Into#1")?,
            receiver: None,
            args: vec![
                al_ir::Arg::Value(value),
                al_ir::Arg::Value(al_ir::Expr::Const(al_ir::Const::Integer(code))),
            ],
            ret_used: true,
        })
    }
}

impl super::Lowerer<'_> {
    /// The elements of `x in [...]`, each converted to the selector's type where the matrix pins it (`matrix_set_elem`).
    pub(super) fn in_set_elems(
        &mut self,
        selector: &al_syntax::SyntaxNode,
        set: &al_syntax::SyntaxNode,
    ) -> super::L<Vec<al_ir::SetElem>> {
        let sel = self.ty_of(selector);
        set.children()
            .map(|e| match self.matrix_set_elem(Family::CmpIn, &sel, &e)? {
                Some(x) => Ok(x),
                None => self.set_elem(&e),
            })
            .collect()
    }

    /// The operands of a comparison: a one-character literal compared with a Char is a Char, `C > 'A'` with C = 'a' is true
    /// (types/decimal-integer/ScT70061.al:Char_Arithmetic `gt`). A Char against a Text variable compares as texts
    /// (`al_builtins::conv::compare_cell`). Other operands are as they are.
    pub(super) fn char_literal_comparison(
        &mut self,
        op: &str,
        kids: &[al_syntax::SyntaxNode],
        l: al_ir::Expr,
        r: al_ir::Expr,
    ) -> super::L<(al_ir::Expr, al_ir::Expr)> {
        let literal = crate::check::one_char_literal;
        let char_pair = matches!(self.ty_ref(&kids[0]), Ty::Char) && literal(&kids[1])
            || matches!(self.ty_ref(&kids[1]), Ty::Char) && literal(&kids[0]);
        if !matches!(op, "=" | "<>" | "<" | "<=" | ">" | ">=") || !char_pair {
            return Ok((l, r));
        }
        let text = Ty::Text(None);
        Ok(if literal(&kids[0]) {
            (self.convert_to(&Ty::Char, &text, l)?, r)
        } else {
            (l, self.convert_to(&Ty::Char, &text, r)?)
        })
    }
}
