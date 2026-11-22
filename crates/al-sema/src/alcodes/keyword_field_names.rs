//! AL0107 for a table field named, unquoted, by a word of [`PARAM_NAME_KEYWORDS`], the words the AL compiler rejects as a
//! parameter name (al-runner issue 8630): `field(2; Area; Integer)`. The AL compilers of the BC artifacts of every supported
//! major and a BC container give AL0107 (identifier expected) at the name first, in a table and in a table extension;
//! alc's recovery after it is not reproduced. A quoted name compiles, and so do the other words (`Local`, `If`, `Exit`,
//! `Keys`, `Fields`, `Layout`). Assumption `keyword-field-names` until the corpus pins it. Other sections that name
//! something in their arguments (page controls, enum values, report columns) are not judged.

use super::{IDENTIFIER_EXPECTED, PARAM_NAME_KEYWORDS, push};
use crate::workspace::{Diagnostic, Workspace};
use al_builtins::assumptions::Assumption;
use al_syntax::{SyntaxKind, SyntaxToken, ast};

/// AL0107 for each field of table or table extension `oi` named by such a word, in field order.
pub(super) fn check(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    for fields in obj.sections().filter(|s| s.name().eq_ignore_ascii_case("fields")) {
        for field in fields.sections().filter(|s| s.name().eq_ignore_ascii_case("field")) {
            if let Some((name, _rests_on)) = keyword_name(field.syntax()) {
                push(
                    out,
                    ws,
                    file,
                    name.text_range().start().into(),
                    "AL0107",
                    IDENTIFIER_EXPECTED.to_string(),
                );
            }
        }
    }
}

/// The name token of a field section, `field(No; Name; Type)`, when it is one unquoted word of [`PARAM_NAME_KEYWORDS`],
/// with the registry entry the rule rests on (a compile refusal ends before any test runs, so it carries no flagged-pass
/// mark; the gate names the entry so that deleting it at the pin names this check).
fn keyword_name(field: &al_syntax::SyntaxNode) -> Option<(SyntaxToken, Assumption)> {
    let args = field.children().find(|n| n.kind() == SyntaxKind::SectionArgs)?;
    let mut toks = args
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia())
        .skip_while(|t| t.kind() != SyntaxKind::Semicolon)
        .skip(1);
    let name = toks.next()?;
    let ends = toks.next().is_some_and(|t| t.kind() == SyntaxKind::Semicolon);
    let keyword =
        name.kind() == SyntaxKind::Ident && PARAM_NAME_KEYWORDS.contains(&name.text().to_ascii_lowercase().as_str());
    (ends && keyword).then_some((name, Assumption::KeywordFieldNames))
}
