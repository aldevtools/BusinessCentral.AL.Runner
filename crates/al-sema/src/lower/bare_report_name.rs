//! The reason for a bare report method named without parentheses in a report's code (`if not UseRequestPage then`).

use super::Lowerer;
use crate::workspace::ObjKind;
use al_syntax::{SyntaxKind, SyntaxNode, ast};

impl Lowerer<'_> {
    /// A report method of the bare form (`crate::check::BARE_REPORT_MEMBERS`) named without a call in a report's or report
    /// extension's code, where it is no read the checker resolved. The forms with parentheses are pinned
    /// (reports/bare-report-member/Y36BareReportMember.al:83-93), and the read `UseRequestPage` without them in a report
    /// (`check::calls::lookup_name`, al-runner issue 7414); the others, and the name as an assignment target, are not pinned,
    /// so they are refused with this reason, not as "not assignable".
    pub(crate) fn bare_report_method_without_call(&self, n: &SyntaxNode) -> Option<String> {
        if n.kind() != SyntaxKind::NameRef
            || !matches!(
                self.ws.objects[self.oi].kind,
                ObjKind::Report | ObjKind::ReportExtension
            )
        {
            return None;
        }
        let token = n
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| matches!(t.kind(), SyntaxKind::Ident | SyntaxKind::QuotedIdent))?;
        let name = ast::ident_text(&token);
        let method = crate::check::BARE_REPORT_MEMBERS
            .iter()
            .find(|m| m.eq_ignore_ascii_case(&name))?;
        Some(format!(
            "the report method `{method}` named without parentheses in a report's code (not pinned)"
        ))
    }
}
