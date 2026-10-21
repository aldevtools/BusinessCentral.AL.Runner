//! Variables, types, parameters, procedures, triggers, attributes.

use super::expressions::{arg_list, expr, is_reserved};
use super::statements::block;
use crate::SyntaxKind::{self, *};
use crate::parser::Parser;

pub(crate) fn var_section(p: &mut Parser) {
    p.start(VarSection);
    p.eat_kw("protected");
    p.expect_kw("var");
    while at_var_decl_start(p) || at_attributed_var_decl(p) {
        // Attributes sit before the declaration, as siblings of it in the section
        // (`[SecurityFiltering(SecurityFilter::Ignored)] Rec: Record Item;`).
        attributes(p);
        var_decl(p);
    }
    p.finish();
}

/// `[Attribute(...)] [More] Name:` -- attributes on a variable, as opposed to the
/// attributes of the procedure that follows the section.
fn at_attributed_var_decl(p: &Parser) -> bool {
    let mut n = 0;
    while p.nth(n) == LBrack {
        let mut depth = 0;
        loop {
            match p.nth(n) {
                LBrack => depth += 1,
                RBrack => depth -= 1,
                SyntaxKind::__Last => return false,
                _ => {}
            }
            n += 1;
            if depth == 0 {
                break;
            }
        }
    }
    n > 0 && at_var_decl_name(p, n) && matches!(p.nth(n + 1), Colon | Comma)
}

fn at_name(p: &Parser, n: usize) -> bool {
    match p.nth(n) {
        QuotedIdent => true,
        Ident => !is_reserved(p.nth_text(n)),
        _ => false,
    }
}

/// The reserved words the latest AL compiler (30.0.42.60748, measured 2026-10-21) accepts as the name of a variable in
/// a var section: `If: Integer;` compiles, and the variable is then used quoted (`"If" := 1;`). `begin`, `end`, `of`,
/// `var`, `procedure`, `trigger` and `local` it rejects there.
const VAR_NAME_KEYWORDS: &[&str] = &[
    "if",
    "then",
    "else",
    "case",
    "while",
    "do",
    "repeat",
    "until",
    "for",
    "to",
    "downto",
    "foreach",
    "in",
    "exit",
    "and",
    "or",
    "xor",
    "not",
    "div",
    "mod",
    "with",
    "asserterror",
];

/// Whether `word` (an `Ident`) can name a variable in a var section.
fn at_var_name(word: &str) -> bool {
    !is_reserved(word) || VAR_NAME_KEYWORDS.iter().any(|k| k.eq_ignore_ascii_case(word))
}

/// [`at_name`] for the name of a variable in a var section.
fn at_var_decl_name(p: &Parser, n: usize) -> bool {
    match p.nth(n) {
        QuotedIdent => true,
        Ident => at_var_name(p.nth_text(n)),
        _ => false,
    }
}

fn at_var_decl_start(p: &Parser) -> bool {
    at_var_decl_name(p, 0) && matches!(p.nth(1), Colon | Comma)
}

fn var_decl(p: &mut Parser) {
    p.start(VarDecl);
    p.bump_any();
    while p.eat(Comma) {
        name(p);
    }
    p.expect(Colon);
    type_ref(p);
    p.expect(Semicolon);
    p.finish();
}

fn name(p: &mut Parser) {
    if matches!(p.current(), Ident | QuotedIdent) {
        p.bump_any();
    } else {
        p.error("expected a name");
    }
}

/// Type names followed by an object reference.
const OBJECT_TYPES: &[&str] = &[
    "record",
    "codeunit",
    "page",
    "testpage",
    "report",
    "testrequestpage",
    "query",
    "xmlport",
    "enum",
    "interface",
    "testfield",
    // `DotNet Alias`: the alias a `dotnet { }` block declares.
    "dotnet",
    // `ControlAddIn Name`: a parameter that receives a control add-in.
    "controladdin",
];

pub(crate) fn type_ref(p: &mut Parser) {
    p.start(TypeRef);
    if p.eat_kw("array") {
        type_args(p, expr);
        p.expect_kw("of");
        type_ref(p);
        p.finish();
        return;
    }
    let kind = p.nth_text(0).to_ascii_lowercase();
    if !p.eat(Ident) {
        p.error("expected a type");
        p.finish();
        return;
    }
    match kind.as_str() {
        "label" => {
            p.expect(StringLit);
            while p.eat(Comma) {
                inline_property(p);
            }
        }
        // `TextConst ENU = 'Hello', DEU = 'Hallo'`: one text per language, the language an identifier.
        "textconst" => loop {
            if p.at(Ident) && p.nth(1) == Eq {
                p.bump_any();
                p.bump_any();
                p.expect(StringLit);
            } else {
                break;
            }
            if !p.eat(Comma) {
                break;
            }
        },
        "option" => loop {
            // `Option` with no members ends a return type: the `var` or `begin` after it opens
            // the body (Base Application `FALedgerEntry.Table.al`, `ConvertPostingType(): Option`).
            // As a member name they are followed by a comma.
            let body_keyword = (p.at_kw("var") || p.at_kw("begin")) && p.nth(1) != Comma;
            if matches!(p.current(), Ident | QuotedIdent | Integer) && !body_keyword {
                p.bump_any();
            }
            if !p.eat(Comma) {
                break;
            }
        },
        k if OBJECT_TYPES.contains(&k) && matches!(p.current(), Ident | QuotedIdent | Integer) => {
            p.bump_any();
            while p.at(Dot) && matches!(p.nth(1), Ident | QuotedIdent) {
                p.bump_any();
                p.bump_any();
            }
        }
        _ => {
            if p.at(LBrack) {
                type_args(p, expr);
            }
        }
    }
    if p.eat_kw("of") {
        type_args(p, type_ref);
    }
    while p.eat_kw("temporary") {}
    p.finish();
}

fn type_args(p: &mut Parser, item: fn(&mut Parser)) {
    p.start(TypeArgs);
    p.expect(LBrack);
    loop {
        item(p);
        if !p.eat(Comma) {
            break;
        }
    }
    p.expect(RBrack);
    p.finish();
}

/// `Name = value` inside a comma-separated list (Label properties).
fn inline_property(p: &mut Parser) {
    p.start(Property);
    name(p);
    p.expect(Eq);
    property_value(p, &[Comma, Semicolon, RParen]);
    p.finish();
}

/// Raw tokens up to one of `stops` at bracket depth 0. Property values get a
/// structured grammar when the semantic model needs one.
pub(crate) fn property_value(p: &mut Parser, stops: &[crate::SyntaxKind]) {
    p.start(PropertyValue);
    let mut depth = 0i32;
    while !p.at_eof() {
        let k = p.current();
        if depth == 0 && (stops.contains(&k) || k == RBrace) {
            break;
        }
        match k {
            LParen | LBrack => depth += 1,
            RParen | RBrack => depth -= 1,
            _ => {}
        }
        p.bump_any();
    }
    p.finish();
}

pub(crate) fn attributes(p: &mut Parser) {
    while p.at(LBrack) {
        p.start(Attribute);
        p.bump_any();
        name(p);
        if p.at(LParen) {
            arg_list(p);
        }
        p.expect(RBrack);
        p.finish();
    }
}

pub(crate) fn at_routine_start(p: &Parser) -> bool {
    let mut n = 0;
    while ["local", "internal", "protected"].iter().any(|m| p.nth_kw(n, m)) {
        n += 1;
    }
    p.nth_kw(n, "procedure") || p.nth_kw(n, "trigger")
}

/// A procedure or trigger with its leading attributes, as found in an object body.
pub(crate) fn routine(p: &mut Parser) {
    let cp = p.checkpoint();
    attributes(p);
    if p.at_kw("event") && matches!(p.nth(1), Ident | QuotedIdent) && p.nth(2) == LParen {
        // `[Obsolete(...)] event Name(...)` of a control add-in.
        super::objects::section_from(p, Some(cp));
        return;
    }
    let mut n = 0;
    while ["local", "internal", "protected"].iter().any(|m| p.nth_kw(n, m)) {
        n += 1;
    }
    let kind = if p.nth_kw(n, "trigger") { Trigger } else { Procedure };
    p.start_at(cp, kind);
    for _ in 0..n {
        p.bump_any();
    }
    if !p.eat_kw("trigger") {
        p.expect_kw("procedure");
    }
    name(p);
    // `trigger Variable::Event(...)` handles an event of a `WithEvents` variable.
    if kind == Trigger && p.at(ColonColon) {
        p.bump_any();
        name(p);
    }
    param_list(p);
    return_spec(p);
    p.eat(Semicolon);
    if p.at_kw("var") || p.at_kw("begin") {
        if p.at_kw("var") {
            var_section(p);
        }
        block(p);
        p.eat(Semicolon);
    }
    p.finish();
}

fn param_list(p: &mut Parser) {
    p.start(ParamList);
    p.expect(LParen);
    if !p.at(RParen) {
        loop {
            p.start(Param);
            p.eat_kw("var");
            name(p);
            p.expect(Colon);
            type_ref(p);
            p.finish();
            if !p.eat(Semicolon) {
                break;
            }
        }
    }
    p.expect(RParen);
    p.finish();
}

fn return_spec(p: &mut Parser) {
    if p.at(Colon) {
        p.start(ReturnSpec);
        p.bump_any();
        type_ref(p);
        p.finish();
    } else if at_name(p, 0) && p.nth(1) == Colon {
        p.start(ReturnSpec);
        p.bump_any();
        p.bump_any();
        type_ref(p);
        p.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::test_util::{dump, errors};

    fn t(src: &str) -> String {
        dump(src, type_ref)
    }

    #[test]
    fn a_textconst_has_a_text_per_language() {
        // Corpus types/lt-variant/VaTTextConst.al.
        assert_eq!(t("TextConst ENU = 'Hello'"), "(TypeRef TextConst ENU = 'Hello')");
        assert_eq!(
            t("TextConst DEU = 'Hallo', ENU = 'It''s %1', FRA = ''"),
            "(TypeRef TextConst DEU = 'Hallo' , ENU = 'It''s %1' , FRA = '')"
        );
    }

    #[test]
    fn simple_and_sized_types() {
        assert_eq!(t("Integer"), "(TypeRef Integer)");
        assert_eq!(t("Text[30]"), "(TypeRef Text (TypeArgs [ (Literal 30) ]))");
    }

    #[test]
    fn object_types_with_quoted_numeric_and_namespaced_names() {
        assert_eq!(
            t("Record \"ALT Universal\" temporary"),
            "(TypeRef Record \"ALT Universal\" temporary)"
        );
        assert_eq!(t("Record 18"), "(TypeRef Record 18)");
        assert_eq!(
            t("Codeunit System.Utilities.\"Temp Blob\""),
            "(TypeRef Codeunit System . Utilities . \"Temp Blob\")"
        );
        assert_eq!(t("TestPage \"ALT Card\""), "(TypeRef TestPage \"ALT Card\")");
    }

    #[test]
    fn dotnet_alias_type() {
        assert_eq!(t("DotNet Convert"), "(TypeRef DotNet Convert)");
        assert_eq!(t("DotNet \"Array\""), "(TypeRef DotNet \"Array\")");
    }

    #[test]
    fn generic_collections() {
        assert_eq!(
            t("List of [Integer]"),
            "(TypeRef List of (TypeArgs [ (TypeRef Integer) ]))"
        );
        assert_eq!(
            t("Dictionary of [Text, List of [Code[20]]]"),
            "(TypeRef Dictionary of (TypeArgs [ (TypeRef Text) , (TypeRef List of (TypeArgs [ (TypeRef Code (TypeArgs [ (Literal 20) ])) ])) ]))"
        );
    }

    #[test]
    fn arrays() {
        assert_eq!(
            t("array[2, 3] of Text[10]"),
            "(TypeRef array (TypeArgs [ (Literal 2) , (Literal 3) ]) of (TypeRef Text (TypeArgs [ (Literal 10) ])))"
        );
    }

    #[test]
    fn label_with_properties() {
        assert_eq!(
            t("Label 'Hi %1', Comment = '%1 = name', Locked = true, MaxLength = 10"),
            "(TypeRef Label 'Hi %1' , (Property Comment = (PropertyValue '%1 = name')) , (Property Locked = (PropertyValue true)) , (Property MaxLength = (PropertyValue 10)))"
        );
    }

    #[test]
    fn a_bare_option_return_type_does_not_take_the_next_keyword_as_a_member() {
        // Base Application's `procedure ConvertPostingType(): Option var FAJnlLine: ...`: the `var`
        // that follows is the next section, not an option member.
        assert_eq!(errors("procedure P(): Option var X: Integer; begin end;", routine), 0);
        assert_eq!(errors("procedure P(): Option begin end;", routine), 0);
        assert_eq!(t("Option A,B"), "(TypeRef Option A , B)");
    }

    #[test]
    fn option_members_including_empty_and_quoted() {
        assert_eq!(t("Option ,A,\"B C\""), "(TypeRef Option , A , \"B C\")");
    }

    #[test]
    fn var_section_with_multiple_names_and_protected() {
        assert_eq!(
            dump("var a, b: Integer; \"Rec X\": Record Customer;", var_section),
            "(VarSection var (VarDecl a , b : (TypeRef Integer) ;) (VarDecl \"Rec X\" : (TypeRef Record Customer) ;))"
        );
        assert_eq!(
            dump("protected var x: Boolean;", var_section),
            "(VarSection protected var (VarDecl x : (TypeRef Boolean) ;))"
        );
    }

    /// The latest AL compiler (30.0.42.60748, 2026-10-21) accepts these keywords as unquoted variable names (`If:
    /// Integer;` in a var section compiles; using the variable unquoted, `If := 1;`, is AL0224 "Expression expected").
    #[test]
    fn keywords_the_compiler_accepts_as_variable_names() {
        for w in VAR_NAME_KEYWORDS {
            assert_eq!(
                dump(&format!("var {w}: Integer; x: Integer;"), var_section),
                format!("(VarSection var (VarDecl {w} : (TypeRef Integer) ;) (VarDecl x : (TypeRef Integer) ;))"),
                "{w}"
            );
        }
        // Used unquoted, the name is the keyword again: `If := 1;` is AL0224 there, a parse error here.
        assert!(errors("begin If := 1; end", block) > 0);
        // These it rejects as names; the section ends before them.
        for w in ["begin", "end", "of", "var", "procedure", "trigger", "local"] {
            assert!(!at_var_name(w), "{w}");
        }
    }

    #[test]
    fn var_section_stops_at_begin() {
        assert_eq!(
            dump("var x: Integer;", var_section),
            "(VarSection var (VarDecl x : (TypeRef Integer) ;))"
        );
    }

    #[test]
    fn procedure_with_params_return_var_and_body() {
        assert_eq!(
            dump(
                "local procedure Add(var A: Integer; B: Integer): Integer\nvar\n  C: Integer;\nbegin\n  exit(A + B);\nend;",
                routine
            ),
            "(Procedure local procedure Add (ParamList ( (Param var A : (TypeRef Integer)) ; (Param B : (TypeRef Integer)) )) (ReturnSpec : (TypeRef Integer)) (VarSection var (VarDecl C : (TypeRef Integer) ;)) (Block begin (ExitStmt exit ( (BinExpr (NameRef A) + (NameRef B)) )) ; end) ;)"
        );
    }

    #[test]
    fn named_return_value() {
        assert_eq!(
            dump("procedure F() Result: Text\nbegin\nend;", routine),
            "(Procedure procedure F (ParamList ( )) (ReturnSpec Result : (TypeRef Text)) (Block begin end) ;)"
        );
    }

    #[test]
    fn attributes_before_a_procedure() {
        assert_eq!(
            dump(
                "[Test]\n[HandlerFunctions('H1,H2')]\nprocedure T()\nbegin\nend;",
                routine
            ),
            "(Procedure (Attribute [ Test ]) (Attribute [ HandlerFunctions (ArgList ( (Literal 'H1,H2') )) ]) procedure T (ParamList ( )) (Block begin end) ;)"
        );
    }

    #[test]
    fn interface_procedure_has_no_body() {
        assert_eq!(
            dump("procedure Compute(X: Integer): Integer;", routine),
            "(Procedure procedure Compute (ParamList ( (Param X : (TypeRef Integer)) )) (ReturnSpec : (TypeRef Integer)) ;)"
        );
    }

    #[test]
    fn trigger() {
        assert_eq!(
            dump("trigger OnRun()\nbegin\nend;", routine),
            "(Trigger trigger OnRun (ParamList ( )) (Block begin end) ;)"
        );
    }

    #[test]
    fn option_return_type_without_members_ends_before_var_or_begin() {
        // Base Application's `FALedgerEntry.Table.al` has `procedure ConvertPostingType(): Option`
        // followed by a `var` section; the `var` is no member.
        let src = "procedure P(): Option\nvar\n  X: Integer;\nbegin\nend;";
        assert_eq!(errors(src, routine), 0);
        assert_eq!(
            dump(src, routine),
            "(Procedure procedure P (ParamList ( )) (ReturnSpec : (TypeRef Option)) (VarSection var (VarDecl X : (TypeRef Integer) ;)) (Block begin end) ;)"
        );
        assert_eq!(errors("procedure P(): Option\nbegin\nexit(1);\nend;", routine), 0);
        // A member can still be named `Begin` or `Var`.
        assert_eq!(t("Option Begin,End"), "(TypeRef Option Begin , End)");
        assert_eq!(t("Option Var,Other"), "(TypeRef Option Var , Other)");
    }

    #[test]
    fn event_handler_trigger_names_the_variable_and_the_event() {
        // `trigger CameraProvider::PictureAvailable(...)` handles an event of a WithEvents variable
        // (System Application, `Camera.Page.al`).
        let src = "trigger Tour::TourEnded(PageId: Integer)\nbegin\nend;";
        assert_eq!(errors(src, routine), 0);
        assert_eq!(
            dump(src, routine),
            "(Trigger trigger Tour :: TourEnded (ParamList ( (Param PageId : (TypeRef Integer)) )) (Block begin end) ;)"
        );
    }

    #[test]
    fn control_add_in_parameter_type() {
        assert_eq!(t("ControlAddIn BusinessChart"), "(TypeRef ControlAddIn BusinessChart)");
        assert_eq!(
            errors(
                "procedure Update(Chart: ControlAddIn BusinessChart)\nbegin\nend;",
                routine
            ),
            0
        );
    }

    #[test]
    fn missing_type_is_an_error() {
        assert!(errors("var x: ;", var_section) > 0);
    }
}
