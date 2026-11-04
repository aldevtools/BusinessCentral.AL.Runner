//! `Blob.Length` of a stored blob before CalcFields, written as an operand of a comparison with the literal 0
//! (al-runner issue 6896). The stored size is pinned only for some contents, but a comparison with 0 observes only its
//! sign, and the stored size of non-empty content is positive. Such a call lowers to `Blob.LengthSign#1`, which answers
//! an unpinned stored size of non-empty content with a positive number and marks its assumption. Any other use of
//! the length keeps `Blob.Length#1`, which refuses an unpinned size.
use super::*;
use al_syntax::ast::NodeText;

impl Lowerer<'_> {
    /// The builtin id to emit for the call `node` of catalog id `id`: `Blob.LengthSign#1` for a `Blob.Length` that is compared
    /// with 0, else `id`.
    pub(crate) fn blob_length_emit_id(id: &str, node: &SyntaxNode) -> String {
        if id == "Blob.Length#1" && Self::compared_with_zero(node) {
            "Blob.LengthSign#1".to_string()
        } else {
            id.to_string()
        }
    }

    /// `call` (the `Length` call or member expression) is an operand of `=`, `<>`, `<`, `>`, `<=` or `>=` whose other operand
    /// is the literal 0, in either order. Parentheses around the call do not matter.
    pub(crate) fn compared_with_zero(call: &SyntaxNode) -> bool {
        let mut child = call.clone();
        let mut parent = child.parent();
        while let Some(p) = parent.clone().filter(|p| p.kind() == SyntaxKind::ParenExpr) {
            child = p;
            parent = child.parent();
        }
        let Some(p) = parent.filter(|p| p.kind() == SyntaxKind::BinExpr) else {
            return false;
        };
        let comparison = matches!(
            sig_tokens(&p).first().map(|t| t.text().to_string()).as_deref(),
            Some("=" | "<>" | "<" | ">" | "<=" | ">=")
        );
        let mut operands = p.children();
        let (Some(a), Some(b), None) = (operands.next(), operands.next(), operands.next()) else {
            return false;
        };
        let other = if a == child { b } else { a };
        comparison && other.kind() == SyntaxKind::Literal && other.text_string().trim() == "0"
    }
}
