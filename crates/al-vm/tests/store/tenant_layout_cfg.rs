//! Tenant Report Layout Cfg (2000000249, BC 29 on): a stored table whose Insert checks the layout parts a row names.
//! Pins: platform/lt-platform-tables/PtCloudTables.al Y17_TenantReportLayoutCfg_InsertModifyDelete and
//! Y17_TenantReportLayoutCfg_Shapes (corpus master 97c29f86). The platform objects are not in the test workspace: a table
//! and a report of their numbers and names stand in.

use al_builtins::native::Signal;
use al_testkit::{Built, error_message, text};

const SRC: &str = r#"
table 2000000249 "Tenant Report Layout Cfg" { fields {
    field(1; "Report ID"; Integer) { } field(2; "Layout Name"; Text[250]) { } field(3; "Company Name"; Text[30]) { }
    field(4; "Header Part Name"; Text[300]) { } field(5; "Theme Part Name"; Text[300]) { } }
  keys { key(PK; "Report ID", "Layout Name", "Company Name") { Clustered = true; } } }
report 2000000001 "Tenant Report Defaults" { dataset { dataitem(I; Integer) { } } }
reportextension 50310 "Default Parts" extends "Tenant Report Defaults" { rendering {
    layout(DefHF) { Type = Word; Subtype = HeaderFooter; LayoutFile = 'dhf.docx'; }
    layout(DefTheme) { Type = Word; Subtype = Theme; LayoutFile = 'dth.dotx'; } } }
report 50300 "Parts" { DefaultRenderingLayout = Body; dataset { dataitem(I; Integer) { } } rendering {
    layout(Body) { Type = Word; Subtype = Body; LayoutFile = 'b.docx'; }
    layout(HF) { Type = Word; Subtype = HeaderFooter; LayoutFile = 'hf.docx'; }
    layout(Theme) { Type = Word; Subtype = Theme; LayoutFile = 't.dotx'; } } }
report 50301 "Plain" { DefaultRenderingLayout = Rdl; dataset { dataitem(I; Integer) { } } rendering {
    layout(Rdl) { Type = RDLC; LayoutFile = 'r.rdl'; } } }
codeunit 50302 C {
    local procedure P(Name: Text): Text begin exit('5d86850b-0d76-4eca-bd7b-951ad998e997::' + Name); end;
    local procedure Ins(ReportId: Integer; Body: Text; Header: Text; Theme: Text): Boolean
    var Cfg: Record "Tenant Report Layout Cfg";
    begin
        Cfg."Report ID" := ReportId; Cfg."Layout Name" := CopyStr(Body, 1, 250);
        Cfg."Header Part Name" := CopyStr(Header, 1, 300); Cfg."Theme Part Name" := CopyStr(Theme, 1, 300);
        exit(Cfg.Insert(true));
    end;
    procedure Valid(): Text
    var Cfg: Record "Tenant Report Layout Cfg"; R: Text;
    begin
        if not Cfg.IsEmpty() then exit('fresh');
        R += Format(Ins(0, '', P('DefHF'), P('DefTheme')));
        R += Format(Ins(50300, '', P('HF'), P('Theme')));
        R += Format(Ins(50301, '', P('DefHF'), P('DefTheme')));
        R += Format(Ins(50300, P('Body'), P('HF'), ''));
        R += Format(Ins(99999990, '', P('DefHF'), P('DefTheme')));
        R += Format(Ins(0, '', P('DefHF'), P('DefTheme')));
        Cfg.SetRange("Header Part Name", P('HF'));
        R += '|' + Format(Cfg.Count());
        Cfg.ModifyAll("Header Part Name", '');
        Cfg.Reset();
        Cfg.Get(50300, P('Body'), '');
        R += '|' + Cfg."Header Part Name" + '|' + Format(Cfg.Count());
        exit(R);
    end;
    procedure WrongHeader(): Boolean begin exit(Ins(0, '', 'x::A', 'y::B')); end;
    procedure ThemeAsHeader(): Boolean begin exit(Ins(0, '', P('DefTheme'), P('DefTheme'))); end;
    procedure OwnPartsOnGlobal(): Boolean begin exit(Ins(0, '', P('HF'), P('Theme'))); end;
    procedure PlainBody(): Boolean begin exit(Ins(50300, 'Body', P('HF'), '')); end;
    procedure DefaultSubtypeBody(): Boolean begin exit(Ins(50301, P('Rdl'), P('DefHF'), P('DefTheme'))); end;
    procedure UndeclaredBody(): Boolean begin exit(Ins(50300, P('NoSuch'), P('HF'), P('Theme'))); end;
    procedure BlankHeader(): Boolean begin exit(Ins(0, '', '', P('DefTheme'))); end;
    procedure WrongTheme(): Boolean begin exit(Ins(0, '', P('DefHF'), 'y::B')); end;
    procedure WrongHeaderAndBody(): Boolean begin exit(Ins(50300, 'Body', 'x::A', '')); end;
    procedure ModifyPart(): Boolean
    var Cfg: Record "Tenant Report Layout Cfg";
    begin
        Ins(50300, '', P('HF'), P('Theme')); Cfg.Get(50300, '', ''); Cfg."Theme Part Name" := P('DefTheme'); exit(Cfg.Modify());
    end;
}
"#;

fn built() -> Built {
    let mut l = al_testkit::compile(SRC);
    let mut program = (*l.program).clone();
    for a in program.apps.iter_mut() {
        a.id = "5d86850b-0d76-4eca-bd7b-951ad998e997".into();
    }
    l.program = std::sync::Arc::new(program);
    Built::new(l)
}

fn refused(r: Result<al_builtins::value::Value, Signal>) {
    assert!(matches!(&r, Err(Signal::Unsupported(m)) if m.contains("Tenant Report Layout Cfg")), "{r:?}");
}

const HF_ERR: &str = "The Header/Footer part '{}' must reference a Header/Footer-subtype layout in the tenants Report Layout List.";
const BODY_ERR: &str = "The Body part '{}' must reference a Body-subtype layout in the tenants Report Layout List.";

#[test]
fn valid_rows_insert_and_take_a_blank_part_by_modify_all() {
    let mut b = built();
    assert_eq!(text(b.run_in("C", "Valid")), "YesYesYesYesYesNo|2||5");
}

#[test]
fn a_wrong_header_or_body_part_raises_the_pinned_error() {
    let app = "5d86850b-0d76-4eca-bd7b-951ad998e997";
    let mut b = built();
    assert_eq!(error_message(b.run_in("C", "WrongHeader")), HF_ERR.replace("{}", "x::A"));
    assert_eq!(error_message(b.run_in("C", "ThemeAsHeader")), HF_ERR.replace("{}", &format!("{app}::DefTheme")));
    assert_eq!(error_message(b.run_in("C", "OwnPartsOnGlobal")), HF_ERR.replace("{}", &format!("{app}::HF")));
    assert_eq!(error_message(b.run_in("C", "PlainBody")), BODY_ERR.replace("{}", "Body"));
    assert_eq!(error_message(b.run_in("C", "DefaultSubtypeBody")), BODY_ERR.replace("{}", &format!("{app}::Rdl")));
    assert_eq!(error_message(b.run_in("C", "UndeclaredBody")), BODY_ERR.replace("{}", &format!("{app}::NoSuch")));
}

#[test]
fn shapes_no_pin_covers_are_refused() {
    let mut b = built();
    for r in ["BlankHeader", "WrongTheme", "WrongHeaderAndBody", "ModifyPart"] {
        refused(b.run_in("C", r));
    }
}
