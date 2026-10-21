//! Files, objects and object bodies.
//!
//! Object bodies are parsed generically: a member is a routine (procedure or
//! trigger), a var section, a property `Name = value;`, or a section
//! `name [(args)] [{ members }]`, which covers `fields`, `field(...)`, `keys`,
//! `layout`, `area(...)`, `dataitem(...)`, `value(...)`, `modify(...)` and the
//! rest without a rule per object type.

use super::declarations::{at_routine_start, property_value, routine, var_section};
use crate::SyntaxKind::*;
use crate::parser::Parser;

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

fn at_object_start(p: &Parser) -> bool {
    OBJECT_KINDS.iter().any(|k| p.at_kw(k)) && matches!(p.nth(1), Integer | Ident | QuotedIdent)
}

pub(crate) fn source_file(p: &mut Parser) {
    while !p.at_eof() {
        if p.at_kw("namespace") || p.at_kw("using") {
            let kind = if p.at_kw("namespace") { NamespaceDecl } else { UsingDecl };
            p.start(kind);
            p.bump_any();
            qualified_name(p);
            p.expect(Semicolon);
            p.finish();
        } else if at_object_start(p) {
            object(p);
        } else if p.at_kw("dotnet") && p.nth(1) == LBrace {
            // `dotnet { assembly(..) { type(..; Alias) { } } }`: a declaration block, not an object.
            p.start(DotNetBlock);
            p.bump_any();
            body(p);
            p.finish();
        } else {
            p.err_and_bump("expected an object");
        }
    }
}

/// `A`, `A.B."C D"`
fn qualified_name(p: &mut Parser) {
    if !matches!(p.current(), Ident | QuotedIdent) {
        p.error("expected a name");
        return;
    }
    p.bump_any();
    while p.at(Dot) && matches!(p.nth(1), Ident | QuotedIdent) {
        p.bump_any();
        p.bump_any();
    }
}

fn object(p: &mut Parser) {
    p.start(Object);
    p.bump_any();
    p.eat(Integer);
    qualified_name(p);
    loop {
        if p.eat_kw("extends") || p.eat_kw("customizes") {
            // The target by its id (`extends 50100`): the AL compiler takes it (al-runner issue 5576; alc 18.0.39.10160-beta
            // compiles a page and a table extension by id, the oracle for compile acceptance). Probe row in
            // docs/probes-needed.md "User issues (lane-user-trust)".
            if !p.eat(Integer) {
                qualified_name(p);
            }
        } else if p.eat_kw("implements") {
            loop {
                qualified_name(p);
                if !p.eat(Comma) {
                    break;
                }
            }
        } else {
            break;
        }
    }
    body(p);
    p.finish();
}

/// `{ member* }`
fn body(p: &mut Parser) {
    p.start(ObjectBody);
    p.expect(LBrace);
    while !p.at_eof() && !p.at(RBrace) {
        // A new object header means this body's `}` is missing; let the file loop take it.
        if at_object_start(p) && p.nth(1) == Integer {
            break;
        }
        member(p);
    }
    p.expect(RBrace);
    p.finish();
}

fn member(p: &mut Parser) {
    if p.at(LBrack) || at_routine_start(p) {
        routine(p);
    } else if p.at_kw("var") || (p.at_kw("protected") && p.nth_kw(1, "var")) {
        var_section(p);
    } else if matches!(p.current(), Ident | QuotedIdent) && p.nth(1) == Eq {
        p.start(Property);
        p.bump_any();
        p.bump_any();
        property_value(p, &[Semicolon]);
        p.expect(Semicolon);
        p.finish();
    } else if p.at(Ident) && matches!(p.nth(1), LParen | LBrace) {
        section(p);
    } else if p.at_kw("event") && matches!(p.nth(1), Ident | QuotedIdent) && p.nth(2) == LParen {
        // `event Name(Params);` of a control add-in.
        section(p);
    } else if p.at(Semicolon) {
        // Stray separator after a section's closing brace.
        p.bump_any();
    } else {
        p.err_and_bump("expected a property, section, variable or procedure");
    }
}

fn section(p: &mut Parser) {
    section_from(p, None);
}

/// A section; `cp` is a checkpoint before attributes that belong to it (an attributed control
/// add-in `event`).
pub(crate) fn section_from(p: &mut Parser, cp: Option<crate::builder::Checkpoint>) {
    match cp {
        Some(cp) => p.start_at(cp, Section),
        None => p.start(Section),
    }
    let is_event = p.at_kw("event") && matches!(p.nth(1), Ident | QuotedIdent);
    // `moveafter(A; B)` and its siblings in a layout or actions section.
    let is_move = ["moveafter", "movebefore", "movefirst", "movelast"]
        .iter()
        .any(|k| p.at_kw(k));
    p.bump_any();
    if is_event {
        // `event Name(...)`: the name follows the keyword.
        p.bump_any();
    }
    if p.at(LParen) {
        p.start(SectionArgs);
        p.bump_any();
        let mut depth = 1;
        while !p.at_eof() {
            match p.current() {
                LParen => depth += 1,
                RParen => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                LBrace | RBrace => break,
                _ => {}
            }
            p.bump_any();
        }
        p.expect(RParen);
        p.finish();
    }
    if p.at(LBrace) {
        body(p);
    } else if is_event || is_move {
        // Microsoft's control add-ins leave the `;` off an event now and then
        // (`BarcodeScannerProviderAddIn.ControlAddIn.al`), and the AL compiler accepts it. It
        // accepts two `moveafter(...)` in a row with no `;` as well (a real project's
        // CustomerCard.PageExt.al), so the move statements share the rule.
        p.eat(Semicolon);
    } else {
        p.expect(Semicolon);
    }
    p.finish();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::test_util::{dump, errors};

    fn f(src: &str) -> String {
        dump(src, source_file)
    }

    #[test]
    fn namespace_and_usings() {
        assert_eq!(
            f("namespace A.B;\nusing System.Text;\ncodeunit 50100 X { }"),
            "(NamespaceDecl namespace A . B ;) (UsingDecl using System . Text ;) (Object codeunit 50100 X (ObjectBody { }))"
        );
    }

    #[test]
    fn codeunit_with_properties_var_and_routines() {
        assert_eq!(
            f(
                "codeunit 60100 \"Test X\"\n{\n    Subtype = Test;\n    var\n        A: Codeunit Assert;\n\n    [Test]\n    procedure T()\n    begin\n    end;\n}\n"
            ),
            "(Object codeunit 60100 \"Test X\" (ObjectBody { (Property Subtype = (PropertyValue Test) ;) (VarSection var (VarDecl A : (TypeRef Codeunit Assert) ;)) (Procedure (Attribute [ Test ]) procedure T (ParamList ( )) (Block begin end) ;) }))"
        );
    }

    #[test]
    fn table_with_fields_keys_and_field_triggers() {
        assert_eq!(
            f(
                "table 50900 \"ALT Universal\"\n{\n  fields\n  {\n    field(1; \"No.\"; Code[20]) { Caption = 'No.'; trigger OnValidate() begin end; }\n  }\n  keys { key(PK; \"No.\") { Clustered = true; } }\n}"
            ),
            "(Object table 50900 \"ALT Universal\" (ObjectBody { (Section fields (ObjectBody { (Section field (SectionArgs ( 1 ; \"No.\" ; Code [ 20 ] )) (ObjectBody { (Property Caption = (PropertyValue 'No.') ;) (Trigger trigger OnValidate (ParamList ( )) (Block begin end) ;) })) })) (Section keys (ObjectBody { (Section key (SectionArgs ( PK ; \"No.\" )) (ObjectBody { (Property Clustered = (PropertyValue true) ;) })) })) }))"
        );
    }

    #[test]
    fn property_values_with_nested_parentheses_commas_and_pipes() {
        assert_eq!(
            f(
                "table 1 T { fields { field(2; F; Decimal) { FieldClass = FlowField; CalcFormula = sum(L.Amount where(\"No.\" = field(\"No.\"), T = filter(A | B))); } } }"
            ),
            "(Object table 1 T (ObjectBody { (Section fields (ObjectBody { (Section field (SectionArgs ( 2 ; F ; Decimal )) (ObjectBody { (Property FieldClass = (PropertyValue FlowField) ;) (Property CalcFormula = (PropertyValue sum ( L . Amount where ( \"No.\" = field ( \"No.\" ) , T = filter ( A | B ) ) )) ;) })) })) }))"
        );
    }

    /// The AL compiler takes the object id as the target of `extends` (al-runner issue 5576: `pageextension 50101 E
    /// extends 50100` compiles on BC 29 and with alc 18.0.39.10160-beta).
    #[test]
    fn an_extension_names_its_target_by_id() {
        assert_eq!(
            f("pageextension 50101 \"E\" extends 50100 { }"),
            "(Object pageextension 50101 \"E\" extends 50100 (ObjectBody { }))"
        );
        let p = crate::parse("tableextension 50102 X extends 50100 { }", &Default::default());
        assert!(p.errors.is_empty(), "{:?}", p.errors);
        let file = crate::ast::SourceFile::cast(p.syntax()).expect("file");
        let o = file.objects().next().expect("object");
        assert_eq!(o.extends().as_deref(), Some("50100"));
        assert_eq!(o.extends_parts(), Some(vec!["50100".to_string()]));
        assert_eq!(o.name(), "X");
        assert_eq!(o.id(), Some(50102));
    }

    #[test]
    fn extension_objects_and_implements() {
        assert_eq!(
            f("tableextension 50100 \"X Ext\" extends Customer { }"),
            "(Object tableextension 50100 \"X Ext\" extends Customer (ObjectBody { }))"
        );
        assert_eq!(
            f("enum 50100 E implements I1, \"I 2\" { value(0; A) { } }"),
            "(Object enum 50100 E implements I1 , \"I 2\" (ObjectBody { (Section value (SectionArgs ( 0 ; A )) (ObjectBody { })) }))"
        );
    }

    #[test]
    fn interface_and_profile_have_no_id() {
        assert_eq!(
            f("interface IX { procedure F(): Integer; }"),
            "(Object interface IX (ObjectBody { (Procedure procedure F (ParamList ( )) (ReturnSpec : (TypeRef Integer)) ;) }))"
        );
        assert_eq!(
            f("profile P { Caption = 'P'; RoleCenter = \"Role Center\"; }"),
            "(Object profile P (ObjectBody { (Property Caption = (PropertyValue 'P') ;) (Property RoleCenter = (PropertyValue \"Role Center\") ;) }))"
        );
    }

    #[test]
    fn page_extension_moves_without_body() {
        assert_eq!(
            f("pageextension 1 P extends Q { layout { moveafter(A; B); addlast(Content) { field(X; Rec.X) { } } } }"),
            "(Object pageextension 1 P extends Q (ObjectBody { (Section layout (ObjectBody { (Section moveafter (SectionArgs ( A ; B )) ;) (Section addlast (SectionArgs ( Content )) (ObjectBody { (Section field (SectionArgs ( X ; Rec . X )) (ObjectBody { })) })) })) }))"
        );
    }

    #[test]
    fn page_extension_moves_may_leave_off_the_semicolon() {
        // NaviPartner's CustomerCard.PageExt.al writes two `moveafter(...)` in a row with no `;`,
        // and the AL compiler accepts it, as it does for the other move statements.
        let src = "pageextension 1 P extends Q { layout { moveafter(A; B) moveafter(A; C) movebefore(A; D) movefirst(G; E) movelast(G; F) modify(X) { Caption = 'x'; } } actions { moveafter(A; B) } }";
        assert_eq!(errors(src, source_file), 0);
        assert_eq!(
            f(src),
            "(Object pageextension 1 P extends Q (ObjectBody { (Section layout (ObjectBody { (Section moveafter (SectionArgs ( A ; B ))) (Section moveafter (SectionArgs ( A ; C ))) (Section movebefore (SectionArgs ( A ; D ))) (Section movefirst (SectionArgs ( G ; E ))) (Section movelast (SectionArgs ( G ; F ))) (Section modify (SectionArgs ( X )) (ObjectBody { (Property Caption = (PropertyValue 'x') ;) })) })) (Section actions (ObjectBody { (Section moveafter (SectionArgs ( A ; B ))) })) }))"
        );
    }

    #[test]
    fn other_sections_still_need_their_semicolon_or_body() {
        assert!(errors("pageextension 1 P extends Q { layout { addafter(A) } }", source_file) > 0);
    }

    #[test]
    fn dotnet_block_with_assembly_and_types() {
        assert_eq!(
            f(
                "dotnet { assembly(\"mscorlib\") { Version = '4.0.0.0'; type(\"System.Convert\"; \"Convert\") { } } }\ncodeunit 1 A { }"
            ),
            "(DotNetBlock dotnet (ObjectBody { (Section assembly (SectionArgs ( \"mscorlib\" )) (ObjectBody { (Property Version = (PropertyValue '4.0.0.0') ;) (Section type (SectionArgs ( \"System.Convert\" ; \"Convert\" )) (ObjectBody { })) })) })) (Object codeunit 1 A (ObjectBody { }))"
        );
    }

    #[test]
    fn multiple_objects_in_one_file() {
        assert_eq!(
            f("codeunit 1 A { }\ncodeunit 2 B { }"),
            "(Object codeunit 1 A (ObjectBody { })) (Object codeunit 2 B (ObjectBody { }))"
        );
    }

    #[test]
    fn control_add_in_attributed_event() {
        let src = "controladdin A { [Obsolete('x', '2023-06-28')] event Go(); [Obsolete('x', '2023-06-28')] procedure P(F: Text) }";
        assert_eq!(errors(src, source_file), 0);
    }

    #[test]
    fn control_add_in_events_and_bodiless_procedures() {
        // System Application `ControlAddIns/src/BusinessChart.ControlAddin.al`.
        let src = "controladdin BusinessChart { RequestedHeight = 320; event DataPointClicked(Point: JsonObject); event AddInReady(); procedure Update(ChartData: JsonObject); event Last(A: Text) }";
        assert_eq!(errors(src, source_file), 0);
        assert_eq!(
            f(src),
            "(Object controladdin BusinessChart (ObjectBody { (Property RequestedHeight = (PropertyValue 320) ;) (Section event DataPointClicked (SectionArgs ( Point : JsonObject )) ;) (Section event AddInReady (SectionArgs ( )) ;) (Procedure procedure Update (ParamList ( (Param ChartData : (TypeRef JsonObject)) )) ;) (Section event Last (SectionArgs ( A : Text ))) }))"
        );
    }

    #[test]
    fn unknown_member_is_an_error_and_parsing_reaches_the_next_object() {
        assert!(errors("codeunit 1 A { 42 }\ncodeunit 2 B { }", source_file) > 0);
        assert!(errors("codeunit 1 A {", source_file) > 0);
    }
}
