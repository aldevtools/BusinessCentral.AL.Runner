//! The array arguments of `CopyArray`, which BC accepts as the array a procedure returns (al-runner issue 6658).
use super::*;

impl<'a> Lowerer<'a> {
    /// A by-var argument `a` (position `i`) of builtin `id`. CopyArray writes into its destination in place, so its destination
    /// and source are places too. Its source may also be the array a procedure returns, which compiles (alc 30, BC 27 and 29) and
    /// is only read: the copy then takes that array's elements as any source (misc/lt-system-functions/Arrays.al:CopyArray_Forms).
    /// A procedure result as the destination compiles too; what the copy into it does is not pinned, so it is refused.
    pub(crate) fn ref_arg(&mut self, id: &str, i: usize, a: &SyntaxNode) -> L<ir::Arg> {
        if id != "System.CopyArray#1" || a.kind() != SyntaxKind::CallExpr {
            return Ok(ir::Arg::Ref(self.place(a)?));
        }
        if i == 0 {
            return Err(format!(
                "CopyArray into `{}`, the array a procedure returns: what the copy does there is not pinned",
                a.text()
            ));
        }
        // A source of more than one dimension raises when it is a variable (Arrays.al:Array_MultiDimensional_ClearAndCopy
        // `af172`); a procedure that returns one is not pinned.
        if matches!(self.ty_ref(a), Ty::Array(d, _) if d.len() > 1) {
            return Err(format!(
                "CopyArray from `{}`, a multi-dimensional array a procedure returns (not pinned)",
                a.text()
            ));
        }
        Ok(ir::Arg::Value(self.expr(a)?))
    }
}
