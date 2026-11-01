//! Tenant Report Layout Override (2000000248, BC 29): an ordinary stored table, empty on a fresh tenant, with no validation, and
//! Report Layout List shows the Description, Layout Status and IsObsolete of an override row whose flag is set; the current
//! company's row wins over the global row ('' company) for all three, and a flag off on the company row shows the layout's own
//! value (corpus master ac3499d6, platform/lt-platform-tables/PtCloudTables.al:794, :824, :901, BC 29.0).
//! The platform tables are not in the test workspace: tables of their number and name stand in.

use al_testkit::{Built, text};

const SRC: &str = r#"
enum 2000000015 "Report Layout Status" { value(0; Approved) { } value(1; Draft) { } value(2; "Pending Approval") { } value(3; Retired) { } }
enum 2000000014 "Excel Sheet Configuration" { value(0; Default) { } value(1; Single) { } value(2; Multiple) { } }
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
table 2000000248 "Tenant Report Layout Override" { DataPerCompany = false; fields {
    field(1; "Report ID"; Integer) { } field(2; Name; Text[250]) { } field(3; "Runtime Package ID"; Guid) { }
    field(4; "Company Name"; Text[30]) { } field(5; IsObsolete; Boolean) { } field(6; "Override IsObsolete"; Boolean) { }
    field(7; "Layout Status"; Enum "Report Layout Status") { } field(8; "Override Layout Status"; Boolean) { }
    field(9; Description; Text[250]) { } field(10; "Override Description"; Boolean) { } }
  keys { key(PK; "Report ID", Name, "Runtime Package ID", "Company Name") { Clustered = true; } } }
report 50210 "RLO Probe" { DefaultRenderingLayout = Rdlc;
  dataset { dataitem(I; Integer) { column(N; Number) { } } }
  rendering {
    layout(Rdlc) { Type = RDLC; LayoutFile = 'a.rdl'; Summary = 'rdlc summary'; }
    layout(Word) { Type = Word; LayoutFile = 'a.docx'; } } }
codeunit 50211 C {
    local procedure Row(Name: Text): Text
    var L: Record "Report Layout List";
    begin
        L.SetRange("Report ID", 50210);
        L.SetRange(Name, Name);
        if not L.FindFirst() then
            exit('none');
        exit('desc=' + L.Description + ' status=' + Format(L."Layout Status") + ' obsolete=' + Format(L.IsObsolete) + ';');
    end;

    procedure Plain(): Text
    var O: Record "Tenant Report Layout Override"; Out: Text;
    begin
        Out := Format(O.Count()) + ';';
        O."Report ID" := 99999; O.Name := 'X'; O."Company Name" := 'NoSuchCompany';
        Out += Format(O.Insert(true)) + Format(O.Insert()) + ';';
        O.Description := 'D';
        Out += Format(O.Modify(true)) + Format(O.Get(99999, 'X', O."Runtime Package ID", 'NoSuchCompany'))
            + Format(O.Get(99999, 'X', O."Runtime Package ID", '')) + ';';
        O.DeleteAll();
        exit(Out + Format(O.Count()));
    end;

    procedure Seen(): Text
    var L: Record "Report Layout List"; O: Record "Tenant Report Layout Override"; Out: Text;
    begin
        L.SetRange("Report ID", 50210);
        L.SetRange(Name, 'Rdlc');
        L.FindFirst();
        Out := Row('Rdlc');
        O."Report ID" := 50210; O.Name := L.Name; O."Runtime Package ID" := L."Runtime Package ID";
        O.Description := 'Global'; O."Override Description" := true;
        O."Layout Status" := O."Layout Status"::Retired; O."Override Layout Status" := true;
        O.IsObsolete := true; O."Override IsObsolete" := true;
        O.Insert();
        Out += Row('Rdlc') + Row('Word');
        O."Company Name" := CopyStr(CompanyName(), 1, 30);
        O.Description := 'Company';
        O."Layout Status" := O."Layout Status"::"Pending Approval";
        O."Override IsObsolete" := false;
        O.Insert();
        Out += Row('Rdlc');
        O.Get(50210, L.Name, L."Runtime Package ID", '');
        O.Delete();
        Out += Row('Rdlc');
        O.Get(50210, L.Name, L."Runtime Package ID", CompanyName());
        O."Override Description" := false;
        O."Override Layout Status" := false;
        O.Modify();
        Out += Row('Rdlc');
        O.Delete();
        exit(Out + Row('Rdlc'));
    end;
}
"#;

#[test]
fn the_override_table_is_a_plain_stored_table() {
    let mut b = Built::new(al_testkit::compile(SRC));
    assert_eq!(text(b.run_in("C", "Plain")), "0;YesNo;YesYesNo;0");
}

#[test]
fn report_layout_list_shows_the_override_and_the_company_row_wins() {
    let mut b = Built::new(al_testkit::compile(SRC));
    let base = "desc=rdlc summary status=Approved obsolete=No;";
    assert_eq!(
        text(b.run_in("C", "Seen")),
        [
            base,
            "desc=Global status=Retired obsolete=Yes;",
            "desc=Word status=Approved obsolete=No;",
            "desc=Company status=Pending Approval obsolete=No;",
            "desc=Company status=Pending Approval obsolete=No;",
            base,
            base,
        ]
        .concat()
    );
}
