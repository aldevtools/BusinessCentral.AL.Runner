//! AL0107 for a section named, unquoted, by a word of [`PARAM_NAME_KEYWORDS`], the words the AL compiler rejects as a parameter
//! name: a table field `field(2; Area; Integer)` (al-runner issue 8630), and a page control, action, group, part or view, an enum
//! value, a report or query column or data item, a query filter, an xmlport element or attribute, a request page field, a table
//! key or field group, in the object or its extension where [`NAMED`] lists it (issue 8634). The AL compilers of the BC artifacts
//! of every supported major and a BC container give AL0107 (identifier expected) at the name first, for each of the 24 words in
//! each listed section; alc's recovery after it is not reproduced. A quoted name compiles, and so do the other words (`Local`,
//! `If`, `Exit`, `Keys`). Sections [`NAMED`] does not list (a page extension's repeater, cue group, label or view, a request page
//! group, a user control, an anchor such as `addafter(Key)`) are not judged.

use super::{IDENTIFIER_EXPECTED, PARAM_NAME_KEYWORDS, push};
use crate::workspace::{Diagnostic, ObjKind, Workspace};
use al_builtins::assumptions::Assumption;
use al_syntax::{SyntaxKind, SyntaxNode, SyntaxToken, ast};

/// The sections whose name the probes measured: the object kind, the section keyword, the place of the name among the
/// `;`-separated arguments, and the registry entry the rule rests on. A compile refusal ends before any test runs, so the entry
/// carries no flagged-pass mark; the table names it so that deleting it at the pin names this check.
const NAMED: &[(ObjKind, &str, usize, Assumption)] = {
    const F: Assumption = Assumption::KeywordFieldNames;
    const S: Assumption = Assumption::KeywordSectionNames;
    use ObjKind::*;
    &[
        (Table, "field", 1, F),
        (TableExtension, "field", 1, F),
        (Table, "key", 0, S),
        (Table, "fieldgroup", 0, S),
        (TableExtension, "key", 0, S),
        (Page, "field", 0, S),
        (Page, "group", 0, S),
        (Page, "repeater", 0, S),
        (Page, "part", 0, S),
        (Page, "systempart", 0, S),
        (Page, "cuegroup", 0, S),
        (Page, "grid", 0, S),
        (Page, "fixed", 0, S),
        (Page, "label", 0, S),
        (Page, "action", 0, S),
        (Page, "actionref", 0, S),
        (Page, "separator", 0, S),
        (Page, "view", 0, S),
        (PageExtension, "field", 0, S),
        (PageExtension, "group", 0, S),
        (PageExtension, "part", 0, S),
        (PageExtension, "systempart", 0, S),
        (PageExtension, "grid", 0, S),
        (PageExtension, "fixed", 0, S),
        (PageExtension, "action", 0, S),
        (PageExtension, "actionref", 0, S),
        (PageExtension, "separator", 0, S),
        (Enum, "value", 1, S),
        (EnumExtension, "value", 1, S),
        (Report, "column", 0, S),
        (Report, "dataitem", 0, S),
        (Report, "field", 0, S),
        (ReportExtension, "column", 0, S),
        (ReportExtension, "dataitem", 0, S),
        (ReportExtension, "field", 0, S),
        (Query, "column", 0, S),
        (Query, "dataitem", 0, S),
        (Query, "filter", 0, S),
        (XmlPort, "fieldelement", 0, S),
        (XmlPort, "tableelement", 0, S),
        (XmlPort, "textelement", 0, S),
        (XmlPort, "fieldattribute", 0, S),
        (XmlPort, "textattribute", 0, S),
        (XmlPort, "field", 0, S),
    ]
};

/// AL0107 for each section of object `oi` that [`NAMED`] lists and that is named by such a word, in source order.
pub(super) fn check(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let o = &ws.objects[oi];
    for section in obj.syntax().descendants().filter_map(ast::Section::cast) {
        let keyword = section.name();
        let Some(&(.., at, _rests_on)) = NAMED
            .iter()
            .find(|(kind, kw, ..)| *kind == o.kind && keyword.eq_ignore_ascii_case(kw))
        else {
            continue;
        };
        if let Some(name) = keyword_name(section.syntax(), at) {
            push(
                out,
                ws,
                o.file,
                name.text_range().start().into(),
                "AL0107",
                IDENTIFIER_EXPECTED.to_string(),
            );
        }
    }
}

/// The name token of `section`, argument `at` of `(...; ...)`, when the argument is one unquoted word of
/// [`PARAM_NAME_KEYWORDS`].
fn keyword_name(section: &SyntaxNode, at: usize) -> Option<SyntaxToken> {
    let args = section.children().find(|n| n.kind() == SyntaxKind::SectionArgs)?;
    let mut toks = args
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia())
        .skip(1);
    for _ in 0..at {
        toks.find(|t| t.kind() == SyntaxKind::Semicolon)?;
    }
    let name = toks.next()?;
    let ends = toks
        .next()
        .is_some_and(|t| matches!(t.kind(), SyntaxKind::Semicolon | SyntaxKind::RParen));
    let keyword =
        name.kind() == SyntaxKind::Ident && PARAM_NAME_KEYWORDS.contains(&name.text().to_ascii_lowercase().as_str());
    (ends && keyword).then_some(name)
}
