//! A Label or TextConst variable going into a Variant (al-runner issue 7987). In the cells listed in
//! [`Lowerer::text_constant_cell`] the Variant holds a TextConstant (`Variant.FromTextConstant#1`): IsTextConstant is true and
//! every other Is... test false, while Format and a Text read its text. Outside them a Label goes in as a Text (main's
//! conversion) and a TextConst is refused ([`Lowerer::refuse_textconst_into_variant`]).
use super::*;
use al_builtins::assumptions::Assumption;

/// The kind of a variable a bare name reads, when it is declared as just `Label 'text'` or `TextConst ENU = 'text'` (the
/// declarations the pins use: types/TestTernaryContracts.al:529-530, :548-550; types/variant/VaTTextConst.al:13, :278).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Source {
    LocalLabel,
    LocalTextConst,
    GlobalLabel,
    GlobalTextConst,
}

/// Where the Label or TextConst goes into the Variant.
pub(super) enum Site<'n> {
    /// `Target := Name` (a plain assignment).
    Assign(&'n SyntaxNode),
    /// A by-value Variant parameter of `routine` of object `object`, called bare (no receiver, no `with`); the cell requires a
    /// local procedure of the object being lowered, without attributes.
    Arg { object: usize, routine: usize },
}

impl Lowerer<'_> {
    /// The Label or TextConst kind of the bare name `node`, or `None` for any other expression or declaration.
    fn label_source(&self, node: &SyntaxNode) -> Option<Source> {
        if node.kind() != SyntaxKind::NameRef {
            return None;
        }
        match self.res(node)? {
            Res::Local(n) if self.local_plain_text_decls.contains(n) => Some(match self.local_textconsts.contains(n) {
                true => Source::LocalTextConst,
                false => Source::LocalLabel,
            }),
            Res::Global(n) if self.global_plain_text_decls.contains(n) => {
                Some(match self.global_textconsts.contains(n) {
                    true => Source::GlobalTextConst,
                    false => Source::GlobalLabel,
                })
            }
            _ => None,
        }
    }

    /// Whether `target` is a bare name of a non-array Variant variable the routine's `var` section declares (not a parameter,
    /// not the return value, not a global).
    fn declared_local_variant(&self, target: &SyntaxNode) -> bool {
        target.kind() == SyntaxKind::NameRef
            && matches!(self.res(target), Some(Res::Local(n))
                if self.locals.get(n).is_some_and(|&slot| slot >= self.declared_locals_from))
            && matches!(self.ty_of(target), Ty::Variant)
    }

    /// The cells where `value` (the name `node`) goes into the Variant as a TextConstant, each with its pin. `Some(None)` is a
    /// pinned cell, `Some(Some(a))` a cell that runs on assumption `a`, `None` no cell (main's conversion or refusal).
    fn text_constant_cell(&self, node: &SyntaxNode, site: &Site<'_>) -> Option<Option<Assumption>> {
        let source = self.label_source(node)?;
        match site {
            Site::Assign(target) if self.declared_local_variant(target) => match source {
                // types/TestTernaryContracts.al:564 (`V := LblA`, local Label), types/variant/VaTTextConst.al:283 with :289
                // (`V := Lbl`, local Label).
                Source::LocalLabel => Some(None),
                // types/TestTernaryContracts.al:566 with :572 (`V := Tc`, local TextConst ENU = 'Text const').
                Source::LocalTextConst => Some(None),
                // types/variant/VaTTextConst.al:280 with :287 (`V := Simple`, global TextConst ENU = 'Hello') and :144-151
                // (TextConst_In_Variant: no Is... test of the helper, Format and a Text read 'Hello').
                Source::GlobalTextConst => Some(None),
                // A global Label: corpus PR 1382, types/variant/VariantIsMatrix.al:224 with :242, not pinned yet.
                Source::GlobalLabel => Some(Some(Assumption::LabelGlobalIntoVariant)),
            },
            // types/variant/VaTTextConst.al:282 with :288 (`VariantArg(Simple)`: a global TextConst handed to the by-value
            // Variant parameter of a local procedure of the same codeunit without attributes, and handed back as its result);
            // the procedure has that one parameter (:124).
            Site::Arg { object, routine }
                if source == Source::GlobalTextConst
                    && *object == self.oi
                    && self.routine_nodes.get(*object).get(*routine).is_some_and(|r| {
                        r.is_local() && r.attributes().next().is_none() && r.params().count() == 1
                    }) =>
            {
                Some(None)
            }
            _ => None,
        }
    }

    /// `node` lowered as a TextConstant for the Variant slot at `site` when it is one of the cells of
    /// [`text_constant_cell`](Self::text_constant_cell); `None` otherwise.
    pub(super) fn text_constant_into_variant(&mut self, node: &SyntaxNode, site: Site<'_>) -> L<Option<ir::Expr>> {
        let Some(assumed) = self.text_constant_cell(node, &site) else {
            return Ok(None);
        };
        if let Some(a) = assumed {
            self.assume(a);
        }
        let text = self.expr(node)?;
        Ok(Some(ir::Expr::Builtin {
            id: super::call::builtin_id("Variant.FromTextConstant#1")?,
            receiver: None,
            args: vec![ir::Arg::Value(text)],
            ret_used: true,
        }))
    }

    /// The right side of the plain assignment `n` (children `kids`) lowered as a TextConstant when it is one of the cells of
    /// [`text_constant_cell`](Self::text_constant_cell); `None` for a compound assignment and every other shape.
    pub(super) fn assigned_text_constant(&mut self, n: &SyntaxNode, kids: &[SyntaxNode]) -> L<Option<ir::Expr>> {
        if is_compound_assign(n) {
            return Ok(None);
        }
        self.text_constant_into_variant(&kids[1], Site::Assign(&kids[0]))
    }

    /// A variable declared `TextConst` going into a Variant outside the cells of
    /// [`text_constant_cell`](Self::text_constant_cell): which of them BC answers the same way is not pinned, so the
    /// conversion is refused. A Label there is a Text in a Variant (main's conversion; Base App hands labels to Variant
    /// parameters).
    pub(crate) fn refuse_textconst_into_variant(&self, target: &Ty, node: &SyntaxNode) -> L<()> {
        if !matches!(target, Ty::Variant) {
            return Ok(());
        }
        let textconst = match self.res(node) {
            Some(Res::Local(n)) => self.local_textconsts.contains(n),
            Some(Res::Global(n)) => self.global_textconsts.contains(n),
            _ => false,
        };
        if textconst {
            return Err("a TextConst in a Variant outside the pinned cells (not pinned)".into());
        }
        Ok(())
    }
}
