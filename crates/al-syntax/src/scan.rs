//! A cheap declaration scan: what a file declares, without building a tree.
//!
//! The scan lexes and preprocesses the file, then walks the significant tokens
//! once. It reports the namespace, the `using`s, and for each object its kind,
//! id, name, `extends` and `implements` clauses and its event subscriber
//! attributes. That is what an object index needs. The result for the object
//! headers is the same as `ast::SourceFile` reports after a full parse (a test
//! over many files compares them), at a small fraction of the cost.
//!
//! The scan does not check the grammar. A malformed file may be scanned
//! differently from how it parses; the index it feeds is only trusted for files
//! that are later parsed for real, which reports their errors.

use crate::SyntaxKind::{self, *};
use crate::lex;
use crate::preprocessor::preprocess;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileScan {
    pub namespace: Option<String>,
    pub usings: Vec<String>,
    pub objects: Vec<ScannedObject>,
    /// The file has a `dotnet { }` block. The scan does not read it; the caller parses such a file.
    pub dotnet_block: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedObject {
    /// Lower-case keyword: `codeunit`, `tableextension`, ...
    pub kind: String,
    pub id: Option<u32>,
    pub name: String,
    /// Byte offset of the kind keyword in the scanned text.
    pub offset: usize,
    /// `extends X.Y` as parts.
    pub extends: Option<Vec<String>>,
    pub implements: Vec<Vec<String>>,
    pub subscribers: Vec<ScannedSubscriber>,
    /// The properties written directly in the object body (`Caption = 'X';`), in source order:
    /// name as written, value as the parser's `Property::value_text` gives it. Properties of
    /// nested sections (fields, controls) are not scanned.
    pub properties: Vec<(String, String)>,
    /// A codeunit that declares `Subtype = Install;` in its body.
    pub install: bool,
    /// An object that declares `EventSubscriberInstance = Manual;`: its subscribers fire only while
    /// an instance is bound.
    pub manual_subscribers: bool,
    /// For a report: the layouts its `rendering` section declares, in source order.
    pub layouts: Vec<ScannedLayout>,
}

/// `layout(Name) { Type = RDLC; ... }` in the `rendering` section of a report.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScannedLayout {
    /// The name as the parser's `Section::args` gives it (a lone quoted name loses its quotes).
    pub name: String,
    /// The properties of the layout, as for an object body.
    pub properties: Vec<(String, String)>,
}

/// `[EventSubscriber(ObjectType::Codeunit, Codeunit::"Sales-Post", 'OnBeforePost', ...)]`
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScannedSubscriber {
    /// The publisher's kind as written after `ObjectType::`, lower case (`table`, `codeunit`, ...).
    pub publisher_kind: String,
    /// The publisher as written after `Database::` / `Codeunit::` / ...: name parts, or one id.
    pub publisher: Vec<String>,
    /// The event name without its quotes.
    pub event: String,
    /// The `ElementName` argument (a field or control name for field events), without its quotes;
    /// empty when absent.
    pub element: String,
    /// Byte offset of the attribute's `[`.
    pub offset: usize,
}

const OBJECT_KINDS: &[&str] = &[
    "codeunit",
    "table",
    "page",
    "report",
    "query",
    "xmlport",
    "enum",
    "interface",
    "tableextension",
    "pageextension",
    "enumextension",
    "reportextension",
    "permissionset",
    "permissionsetextension",
    "profile",
    "profileextension",
    "pagecustomization",
    "controladdin",
    "entitlement",
];

struct Tok<'a> {
    kind: SyntaxKind,
    start: usize,
    end: usize,
    text: &'a str,
}

impl Tok<'_> {
    fn is_kw(&self, kw: &str) -> bool {
        self.kind == Ident && self.text.eq_ignore_ascii_case(kw)
    }
    fn is_name(&self) -> bool {
        matches!(self.kind, Ident | QuotedIdent | Dot)
    }
    /// `"Quoted"` loses its quotes.
    fn ident_text(&self) -> String {
        crate::ast::unquote_ident(self.text)
    }
}

/// A property value as `Property::value_text` gives it: directives and comments inside it dropped
/// (a `CalcFormula` can span lines that carry `#pragma`).
fn value_text(raw: &str) -> String {
    if !raw.contains(['#', '/']) {
        return raw.to_string();
    }
    let mut out = String::with_capacity(raw.len());
    let mut pos = 0;
    for t in lex(raw) {
        let end = pos + t.len as usize;
        if !matches!(t.kind, Directive | LineComment | BlockComment | DisabledText) {
            out.push_str(&raw[pos..end]);
        }
        pos = end;
    }
    out.trim().to_string()
}

fn qualified(toks: &[Tok]) -> String {
    toks.iter()
        .map(|t| if t.kind == Dot { ".".to_string() } else { t.ident_text() })
        .collect()
}

fn is_clause(t: &Tok) -> bool {
    t.kind == Ident
        && ["extends", "implements", "customizes"]
            .iter()
            .any(|k| t.text.eq_ignore_ascii_case(k))
}

/// Scans `src` with the preprocessor symbols `symbols`.
pub fn scan(src: &str, symbols: &BTreeSet<String>) -> FileScan {
    let tokens = lex(src);
    let tokens = if tokens.iter().any(|t| t.kind == Directive) {
        preprocess(src, &tokens, symbols).tokens
    } else {
        tokens
    };
    let mut sig: Vec<Tok> = Vec::with_capacity(tokens.len() / 2);
    let mut pos = 0;
    for t in &tokens {
        let end = pos + t.len as usize;
        if !t.kind.is_trivia() {
            sig.push(Tok {
                kind: t.kind,
                start: pos,
                end,
                text: &src[pos..end],
            });
        }
        pos = end;
    }
    let mut out = FileScan::default();
    let mut i = 0;
    while i < sig.len() {
        let t = &sig[i];
        if t.is_kw("namespace") || t.is_kw("using") {
            let is_ns = t.is_kw("namespace");
            i += 1;
            // `qualified_name`: names and dots up to the `;`.
            let s = i;
            while i < sig.len() && sig[i].is_name() {
                i += 1;
            }
            let name = qualified(&sig[s..i]);
            if is_ns {
                // The parser takes the first namespace declaration.
                out.namespace.get_or_insert(name);
            } else {
                out.usings.push(name);
            }
            if i < sig.len() && sig[i].kind == Semicolon {
                i += 1;
            }
        } else if t.is_kw("dotnet") && sig.get(i + 1).is_some_and(|n| n.kind == LBrace) {
            out.dotnet_block = true;
            i += 1;
        } else if OBJECT_KINDS.iter().any(|k| t.is_kw(k))
            && sig
                .get(i + 1)
                .is_some_and(|n| matches!(n.kind, Integer | Ident | QuotedIdent))
        {
            i = scan_object(src, &sig, i, &mut out);
        } else {
            i += 1;
        }
    }
    out
}

/// Scans the object starting at `sig[at]`; returns the index after its body.
fn scan_object(src: &str, sig: &[Tok], at: usize, out: &mut FileScan) -> usize {
    // Header: everything up to the body's `{` (or the next object, if the brace is missing).
    let mut end = at + 1;
    while end < sig.len() && sig[end].kind != LBrace {
        end += 1;
    }
    let header = &sig[at..end];
    let kind = header[0].text.to_ascii_lowercase();
    let mut k = 1;
    let id = header
        .get(k)
        .filter(|t| t.kind == Integer)
        .and_then(|t| t.text.parse().ok());
    // AST `id()` is the first Integer anywhere in the header; the grammar puts it right after the kind.
    let id = id.or_else(|| {
        header
            .iter()
            .find(|t| t.kind == Integer)
            .and_then(|t| t.text.parse().ok())
    });
    if header.get(k).is_some_and(|t| t.kind == Integer) {
        k += 1;
    }
    let name_start = k;
    while k < header.len() && header[k].is_name() && !is_clause(&header[k]) {
        k += 1;
    }
    let name = qualified(&header[name_start..k]);
    let (mut extends, mut implements) = (None, Vec::new());
    // The AST reads clauses from the first clause word on.
    let mut c = header.iter().position(is_clause).unwrap_or(header.len());
    while c < header.len() && is_clause(&header[c]) {
        let kw = header[c].text.to_ascii_lowercase();
        c += 1;
        let mut names = Vec::new();
        loop {
            let mut parts = Vec::new();
            // A target by its id (`extends 50100`) is one part.
            while c < header.len() && (header[c].is_name() || header[c].kind == Integer) && !is_clause(&header[c]) {
                if header[c].kind != Dot {
                    parts.push(header[c].ident_text());
                }
                c += 1;
            }
            names.push(parts);
            if header.get(c).is_some_and(|t| t.kind == Comma) {
                c += 1;
            } else {
                break;
            }
        }
        match kw.as_str() {
            "extends" if extends.is_none() => extends = names.into_iter().next(),
            "implements" if implements.is_empty() => implements = names,
            _ => {}
        }
    }
    let mut obj = ScannedObject {
        kind,
        id,
        name,
        offset: header[0].start,
        extends,
        implements,
        subscribers: Vec::new(),
        properties: Vec::new(),
        install: false,
        layouts: Vec::new(),
        manual_subscribers: false,
    };
    // Body: braces nest; subscriber attributes sit directly in the object body.
    let mut i = end;
    let mut depth = 0usize;
    // Depth of `begin` / `case` ... `end` in the code of the object's own routines.
    let mut code = 0usize;
    while i < sig.len() {
        if depth == 1 && sig[i].kind == Ident {
            if sig[i].text.eq_ignore_ascii_case("begin") || sig[i].text.eq_ignore_ascii_case("case") {
                code += 1;
            } else if sig[i].text.eq_ignore_ascii_case("end") && code > 0 {
                code -= 1;
            }
        }
        if depth == 1
            && sig[i].kind == Ident
            && sig[i].text.eq_ignore_ascii_case("Subtype")
            && sig.get(i + 1).is_some_and(|t| t.kind == Eq)
            && sig.get(i + 2).is_some_and(|t| t.is_kw("Install"))
        {
            obj.install = true;
        }
        if depth == 1
            && sig[i].kind == Ident
            && sig[i].text.eq_ignore_ascii_case("EventSubscriberInstance")
            && sig.get(i + 1).is_some_and(|t| t.kind == Eq)
            && sig.get(i + 2).is_some_and(|t| t.is_kw("Manual"))
        {
            obj.manual_subscribers = true;
        }
        match sig[i].kind {
            LBrace => depth += 1,
            RBrace => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    i += 1;
                    break;
                }
            }
            LBrack if depth == 1 => {
                if let Some(s) = subscriber(sig, i) {
                    obj.subscribers.push(s);
                }
            }
            Ident | QuotedIdent
                if depth == 1
                    && code == 0
                    && sig.get(i + 1).is_some_and(|n| n.kind == Eq)
                    && i > 0
                    && matches!(sig[i - 1].kind, LBrace | Semicolon | RBrace | RBrack) =>
            {
                // `Name = value;` directly in the object body.
                let mut j = i + 2;
                while j < sig.len() && sig[j].kind != Semicolon && sig[j].kind != RBrace {
                    j += 1;
                }
                let value = if j > i + 2 {
                    value_text(&src[sig[i + 2].start..sig[j - 1].end])
                } else {
                    String::new()
                };
                obj.properties.push((
                    sig[i].ident_text(),
                    crate::ast::strip_text_attributes(&sig[i].ident_text(), value),
                ));
                i = j;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    if obj.kind == "report" {
        obj.layouts = scan_layouts(src, &sig[end..i.min(sig.len())]);
    }
    out.objects.push(obj);
    i.max(at + 1)
}

/// The layouts of the `rendering` section in a report body (`body` starts at the body's `{`).
fn scan_layouts(src: &str, body: &[Tok]) -> Vec<ScannedLayout> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut j = 0;
    while j < body.len() {
        match body[j].kind {
            LBrace => depth += 1,
            RBrace => depth = depth.saturating_sub(1),
            Ident if depth == 1 && body[j].is_kw("rendering") && body.get(j + 1).is_some_and(|t| t.kind == LBrace) => {
                // Inside the section: `layout(Name) { Prop = value; }` blocks.
                let mut k = j + 2;
                let mut d = 1usize;
                while k < body.len() && d > 0 {
                    match body[k].kind {
                        LBrace => d += 1,
                        RBrace => d -= 1,
                        Ident
                            if d == 1
                                && body[k].is_kw("layout")
                                && body.get(k + 1).is_some_and(|t| t.kind == LParen) =>
                        {
                            let mut m = k + 2;
                            let mut name = String::new();
                            while m < body.len() && body[m].kind != RParen {
                                name.push_str(&body[m].ident_text());
                                m += 1;
                            }
                            let mut properties = Vec::new();
                            if body.get(m + 1).is_some_and(|t| t.kind == LBrace) {
                                m += 2;
                                while m < body.len() && body[m].kind != RBrace {
                                    if matches!(body[m].kind, Ident | QuotedIdent)
                                        && body.get(m + 1).is_some_and(|n| n.kind == Eq)
                                    {
                                        let mut e = m + 2;
                                        while e < body.len() && body[e].kind != Semicolon && body[e].kind != RBrace {
                                            e += 1;
                                        }
                                        let value = if e > m + 2 {
                                            value_text(&src[body[m + 2].start..body[e - 1].end])
                                        } else {
                                            String::new()
                                        };
                                        properties.push((
                                            body[m].ident_text(),
                                            crate::ast::strip_text_attributes(&body[m].ident_text(), value),
                                        ));
                                        m = e;
                                    } else {
                                        m += 1;
                                    }
                                }
                            }
                            out.push(ScannedLayout { name, properties });
                            k = m;
                        }
                        _ => {}
                    }
                    k += 1;
                }
                j = k;
                continue;
            }
            _ => {}
        }
        j += 1;
    }
    out
}

/// `[EventSubscriber(ObjectType::K, K::Name, 'Event', ...)]` at `sig[at]`.
fn subscriber(sig: &[Tok], at: usize) -> Option<ScannedSubscriber> {
    let t = |i: usize| sig.get(at + i);
    if !t(1)?.is_kw("EventSubscriber") || t(2)?.kind != LParen {
        return None;
    }
    // ObjectType :: Kind ,
    if !t(3)?.is_kw("ObjectType") || t(4)?.kind != ColonColon {
        return None;
    }
    let publisher_kind = t(5)?.text.to_ascii_lowercase();
    if t(6)?.kind != Comma {
        return None;
    }
    // Kind :: name-or-id ,
    t(7)?;
    if t(8)?.kind != ColonColon {
        return None;
    }
    let mut i = 9;
    let mut publisher = Vec::new();
    while let Some(n) = t(i) {
        match n.kind {
            Ident | QuotedIdent | Integer => publisher.push(n.ident_text()),
            Dot => {}
            _ => break,
        }
        i += 1;
    }
    if t(i)?.kind != Comma {
        return None;
    }
    // The event is a string, or the publisher method's name written bare (`OnAfterInsertEvent`).
    let ev = t(i + 1)?;
    let event = match ev.kind {
        StringLit => ev.text.trim_matches('\'').replace("''", "'"),
        Ident | QuotedIdent => ev.ident_text(),
        _ => return None,
    };
    // , 'ElementName'
    let element = match (t(i + 2), t(i + 3)) {
        (Some(c), Some(e)) if c.kind == Comma && e.kind == StringLit => e.text.trim_matches('\'').replace("''", "'"),
        _ => String::new(),
    };
    Some(ScannedSubscriber {
        publisher_kind,
        publisher,
        event,
        element,
        offset: sig[at].start,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(src: &str) -> FileScan {
        scan(src, &["BC27PLUS".to_string()].into())
    }

    #[test]
    fn namespace_usings_and_objects_are_found() {
        let f = s(
            "namespace Microsoft.Sales.Customer;\nusing System.Utilities;\nusing Other.\"Name Space\";\n\
                   table 18 \"Customer\" { fields { field(1; \"No.\"; Code[20]) { } } }\n\
                   codeunit 80 \"Sales-Post\" { procedure P() begin end; }",
        );
        assert_eq!(f.namespace.as_deref(), Some("Microsoft.Sales.Customer"));
        assert_eq!(f.usings, ["System.Utilities", "Other.Name Space"]);
        assert_eq!(f.objects.len(), 2);
        assert_eq!(
            (f.objects[0].kind.as_str(), f.objects[0].id, f.objects[0].name.as_str()),
            ("table", Some(18), "Customer")
        );
        assert_eq!(
            (f.objects[1].kind.as_str(), f.objects[1].id, f.objects[1].name.as_str()),
            ("codeunit", Some(80), "Sales-Post")
        );
        assert!(f.objects[0].offset < f.objects[1].offset);
    }

    #[test]
    fn an_extension_target_by_id_is_one_part() {
        let f = s("pageextension 50101 \"E\" extends 50100 { }");
        assert_eq!(f.objects[0].extends, Some(vec!["50100".to_string()]));
        assert_eq!(f.objects[0].name, "E");
    }

    #[test]
    fn extends_and_implements_clauses_are_parts() {
        let f = s(
            "tableextension 50000 \"My Ext\" extends Microsoft.Sales.\"Sales Header\" { }\n\
                   codeunit 50001 Impl implements \"Interface A\", Ns.B { }\n\
                   enum 50002 E implements I { }",
        );
        assert_eq!(
            f.objects[0].extends,
            Some(vec!["Microsoft".into(), "Sales".into(), "Sales Header".into()])
        );
        assert_eq!(
            f.objects[1].implements,
            vec![vec!["Interface A".to_string()], vec!["Ns".to_string(), "B".to_string()]]
        );
        assert_eq!(f.objects[2].implements, vec![vec!["I".to_string()]]);
    }

    #[test]
    fn inactive_preprocessor_branches_declare_nothing() {
        let f = s(
            "#if CLEAN99\ntable 1 Gone { }\n#else\ntable 2 Here { }\n#endif\n#if BC27PLUS\ncodeunit 3 Yes { }\n#endif",
        );
        let names: Vec<_> = f.objects.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, ["Here", "Yes"]);
    }

    #[test]
    fn braces_inside_strings_and_comments_do_not_end_an_object() {
        let f = s("codeunit 1 A { procedure P() begin Message('}'); // }\n /* } */ end; }\ncodeunit 2 B { }");
        assert_eq!(f.objects.len(), 2);
        assert_eq!(f.objects[1].name, "B");
    }

    #[test]
    fn event_subscribers_are_collected_from_the_object_body() {
        let f = s("codeunit 50 S {\n\
                   [EventSubscriber(ObjectType::Table, Database::\"Sales Header\", 'OnAfterInsertEvent', '', false, false)]\n\
                   local procedure A() begin end;\n\
                   [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Sales-Post\", 'OnBeforePost', '', false, false)]\n\
                   local procedure B() begin end;\n\
                   [EventSubscriber(ObjectType::Codeunit, Codeunit::80, 'It''s', '', false, false)]\n\
                   local procedure C() begin end;\n\
                   [EventSubscriber(ObjectType::Table, Database::\"Sales Header\", OnAfterValidateEvent, 'Status', false, false)]\n\
                   local procedure D() begin end;\n}");
        let subs = &f.objects[0].subscribers;
        assert_eq!(subs.len(), 4);
        assert_eq!(subs[3].event, "OnAfterValidateEvent");
        assert_eq!(
            (
                subs[0].publisher_kind.as_str(),
                subs[0].publisher.clone(),
                subs[0].event.as_str()
            ),
            ("table", vec!["Sales Header".to_string()], "OnAfterInsertEvent")
        );
        assert_eq!(subs[1].publisher, ["Sales-Post"]);
        assert_eq!(subs[2].publisher, ["80"]);
        assert_eq!(subs[2].event, "It's");
        // The element name of a field event.
        assert_eq!(subs[3].element, "Status");
        assert_eq!(subs[0].element, "");
    }

    #[test]
    fn a_manual_subscriber_instance_is_noted() {
        let f = s("codeunit 50 M { EventSubscriberInstance = Manual; \
                   [EventSubscriber(ObjectType::Codeunit, Codeunit::P, 'OnX', '', false, false)] local procedure A() begin end; }\
                   codeunit 51 A { EventSubscriberInstance = StaticAutomatic; }\
                   codeunit 52 N { procedure P() begin end; }");
        let manual: Vec<bool> = f.objects.iter().map(|o| o.manual_subscribers).collect();
        assert_eq!(manual, [true, false, false]);
    }

    #[test]
    fn other_attributes_are_not_subscribers() {
        let f = s(
            "codeunit 50 S { [IntegrationEvent(false, false)] procedure P() begin end; [Test] procedure T() begin end; }",
        );
        assert!(f.objects[0].subscribers.is_empty());
    }

    #[test]
    fn a_file_without_objects_scans_empty() {
        assert_eq!(s("// nothing\n"), FileScan::default());
        assert_eq!(s("").objects.len(), 0);
    }
}
