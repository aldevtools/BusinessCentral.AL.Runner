//! The columns of a Report Layout List row of a layout declared in AL: CaptionML (4), Runtime Package ID (5), Report Name
//! (12), Layout Publisher (13), User Defined (14), ReportIsInstalled (15), Layout Status (18) and, for a layout without a
//! Subtype, Layout Subtype (19), HeaderFooterPart (20) and ThemePart (21); Custom Type (8) and MIME Type (9). Pins: platform/lt-ug-report-layout-list/
//! UgReportLayoutList.al:94-150 (an RDLC and a Word layout) and platform/lt-aw-fresh-tenant/AwTests.al:220-300 (Excel
//! layouts). The platform table is not in the test workspace: a table of its number and name stands in.

use al_builtins::native::Signal;
use al_testkit::{Built, text};

const TABLE: &str = r#"
enum 2000000014 "Excel Sheet Configuration" { value(0; Default) { } value(1; Single) { } value(2; Multiple) { } }
enum 2000000015 "Report Layout Status" { value(0; Approved) { } value(1; Draft) { } value(2; "Pending Approval") { } value(3; Retired) { } }
enum 2000000018 "Report Layout Subtype" { value(0; Default) { } value(1; HeaderFooter) { } value(2; Theme) { } value(3; Body) { } }
table 2000000234 "Report Layout List" { fields {
    field(1; "Report ID"; Integer) { } field(2; Name; Text[250]) { } field(3; Caption; Text[250]) { } field(4; CaptionML; Text[250]) { }
    field(5; "Runtime Package ID"; Guid) { } field(6; Layout; Media) { }
    field(7; "Layout Format"; Option) { OptionMembers = RDLC,Word,Excel,Custom; }
    field(8; "Custom Type"; Guid) { } field(9; "MIME Type"; Text[255]) { } field(10; Description; Text[250]) { }
    field(11; "Application ID"; Guid) { } field(12; "Report Name"; Text[250]) { } field(13; "Layout Publisher"; Text[250]) { }
    field(14; "User Defined"; Boolean) { } field(15; ReportIsInstalled; Boolean) { } field(16; IsObsolete; Boolean) { }
    field(17; ExcelLayoutMultipleDataSheets; Enum "Excel Sheet Configuration") { }
    field(18; "Layout Status"; Enum "Report Layout Status") { }
    field(19; "Layout Subtype"; Enum "Report Layout Subtype") { }
    field(20; HeaderFooterPart; Text[300]) { } field(21; ThemePart; Text[300]) { } }
  keys { key(key1; "Report ID", Name, "Runtime Package ID") { Clustered = true; } } }
report 50200 "RL Probe" { DefaultRenderingLayout = Rdlc;
  dataset { dataitem(I; Integer) { column(N; Number) { } } }
  rendering {
    layout(Rdlc) { Type = RDLC; LayoutFile = 'a.rdl'; Caption = 'RL rdlc'; }
    layout(Word) { Type = Word; LayoutFile = 'a.docx'; }
    layout(Xl) { Type = Excel; LayoutFile = 'a.xlsx'; }
    layout(Ml) { Type = Word; LayoutFile = 'b.docx'; CaptionML = ENU = 'ml'; }
    layout(Part) { Type = Word; LayoutFile = 'c.docx'; Subtype = HeaderFooter; } } }
report 50201 "RL Captioned" { Caption = 'Another caption';
  dataset { dataitem(I; Integer) { column(N; Number) { } } }
  rendering { layout(Word) { Type = Word; LayoutFile = 'a.docx'; } } }
codeunit 50202 C {
    procedure Col(ReportId: Integer; LayoutName: Text; FieldNo: Integer): Text
    var RR: RecordRef; G: Guid; Info: ModuleInfo;
    begin
        RR.Open(2000000234);
        RR.Field(1).SetRange(ReportId);
        RR.Field(2).SetRange(LayoutName);
        RR.FindFirst();
        if FieldNo = 5 then begin
            G := RR.Field(5).Value;
            NavApp.GetCurrentModuleInfo(Info);
            exit(Format(IsNullGuid(G)) + ',' + Format(G = Info.Id()));
        end;
        exit(Format(RR.Field(FieldNo).Value));
    end;
    procedure Rdlc4(): Text begin exit(Col(50200, 'Rdlc', 4)); end;
    procedure Rdlc5(): Text begin exit(Col(50200, 'Rdlc', 5)); end;
    procedure Rdlc8(): Text begin exit(Col(50200, 'Rdlc', 8) + '|' + Col(50200, 'Xl', 9)); end;
    procedure Rdlc12(): Text begin exit(Col(50200, 'Rdlc', 12)); end;
    procedure Rdlc13(): Text begin exit(Col(50200, 'Rdlc', 13)); end;
    procedure Rdlc15(): Text begin exit(Col(50200, 'Rdlc', 15)); end;
    procedure Rdlc18(): Text begin exit(Col(50200, 'Rdlc', 18)); end;
    procedure Rdlc19(): Text begin exit(Col(50200, 'Rdlc', 19) + '|' + Col(50200, 'Rdlc', 20) + '|' + Col(50200, 'Rdlc', 21)); end;
    procedure Word14(): Text begin exit(Col(50200, 'Word', 14)); end;
    procedure Xl14(): Text begin exit(Col(50200, 'Xl', 14) + '|' + Col(50200, 'Xl', 18) + '|' + Col(50200, 'Xl', 12)); end;
    procedure Ml4(): Text begin exit(Col(50200, 'Ml', 4)); end;
    procedure Part18(): Text begin exit(Col(50200, 'Part', 18)); end;
    procedure Captioned12(): Text begin exit(Col(50201, 'Word', 12)); end;
}
"#;

fn built() -> Built {
    let mut l = al_testkit::compile(TABLE);
    let mut program = (*l.program).clone();
    for a in program.apps.iter_mut() {
        a.id = "5d86850b-0d76-4eca-bd7b-951ad998e997".into();
        a.name = "RL App".into();
        a.publisher = "RL Publisher".into();
    }
    l.program = std::sync::Arc::new(program);
    Built::new(l)
}

fn call(b: &mut Built, routine: &str) -> Result<al_builtins::value::Value, Signal> {
    b.run_in("C", routine)
}

fn refused(r: Result<al_builtins::value::Value, Signal>, what: &str) {
    assert!(matches!(&r, Err(Signal::Unsupported(m)) if m.contains(what)), "{r:?}");
}

// UgReportLayoutList.al:101-118, :135-152 and AwTests.al:228 (Excel): CaptionML blank, Runtime Package ID a non-null guid
// other than the app id, Report Name the report's name (it declares no Caption), Layout Publisher "<app> by <publisher>",
// User Defined No, ReportIsInstalled Yes, Layout Status Approved, and Default with blank parts for a layout without a Subtype.
#[test]
fn the_pinned_columns_of_a_layout_declared_in_al() {
    let mut b = built();
    assert_eq!(text(call(&mut b, "Rdlc4")), "");
    assert_eq!(text(call(&mut b, "Rdlc5")), "No,No");
    assert_eq!(text(call(&mut b, "Rdlc8")), "{00000000-0000-0000-0000-000000000000}|");
    assert_eq!(text(call(&mut b, "Rdlc12")), "RL Probe");
    assert_eq!(text(call(&mut b, "Rdlc13")), "RL App by RL Publisher");
    assert_eq!(text(call(&mut b, "Rdlc15")), "Yes");
    assert_eq!(text(call(&mut b, "Rdlc18")), "Approved");
    assert_eq!(text(call(&mut b, "Rdlc19")), "Default||");
    assert_eq!(text(call(&mut b, "Word14")), "No");
    assert_eq!(text(call(&mut b, "Xl14")), "No|Approved|RL Probe");
}

// Not pinned: CaptionML of a layout that declares one, Layout Status of a layout part, and Report Name of a report whose
// Caption differs from its name (UgReportLayoutList.al and AwTests.al read only reports without a Caption).
#[test]
fn the_columns_no_pin_covers_stay_refused() {
    let mut b = built();
    refused(call(&mut b, "Ml4"), "CaptionML");
    refused(call(&mut b, "Part18"), "Layout Status");
    refused(call(&mut b, "Captioned12"), "Report Name");
}
