//! Hand-written stand-in; not Microsoft source. It holds the Base Application objects a rendering save's layout selection
//! runs, for the fixtures of this folder: the runtime raises SelectReportLayoutCode and SelectedBuiltinLayoutType to the
//! subscribers of codeunit 44 "ReportManagement" and reads no "Report Layout Selection" row itself
//! (`report_save::fact_selection`). Only object ids and names, the field ids and names the fixtures write, the procedure
//! signatures the fixtures call and the two event subscriber signatures follow Base Application. The bodies answer as
//! `fact_selection` describes, with only the branches a fixture of this folder takes:
//! - SelectReportLayoutCode: the name "Design-time Report Selection" holds (else the code of the company's Custom Layout
//!   row) is answered with Success; the code of a Custom Report Layout row with its type, any other name with LayoutType
//!   None (with `::` and the app id when one is held). The held selection is cleared.
//! - SelectedBuiltinLayoutType: RDLC or Word for the company's built-in row of that kind, None for no row or a Custom Layout
//!   row. Another row type stops the test: no fixture takes it.
//!
//! The objects sit in an app with Base Application's id, the only ReportManagement the runtime raises the events to.

use al_testkit::{AppSource, Built};

const BASE_APPLICATION_ID: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";

const TABLES_AND_HOLDER: &str = r#"
table 9651 "Report Layout Selection"
{
    fields
    {
        field(1; "Report ID"; Integer) { }
        field(3; "Company Name"; Text[30]) { }
        field(4; Type; Option) { OptionMembers = "RDLC (built-in)","Word (built-in)","Custom Layout","Excel Layout","External Layout"; }
        field(6; "Custom Report Layout Code"; Code[20]) { }
    }
    keys { key(PK; "Report ID", "Company Name") { } }
}

table 9650 "Custom Report Layout"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Report ID"; Integer) { }
        field(6; Type; Option) { OptionMembers = RDLC,Word; }
    }
    keys { key(PK; "Code") { } }
}

codeunit 9654 "Design-time Report Selection"
{
    SingleInstance = true;

    var
        HeldName: Text[250];
        HeldCode: Code[20];
        HeldApp: Guid;

    procedure SetSelectedLayout(LayoutName: Text[250])
    var
        NoApp: Guid;
    begin
        Hold(LayoutName, NoApp);
    end;

    procedure SetSelectedLayout(LayoutName: Text[250]; AppID: Guid)
    begin
        Hold(LayoutName, AppID);
    end;

    procedure SetSelectedCustomLayout(NewCustomLayoutCode: Code[20])
    begin
        HeldCode := NewCustomLayoutCode;
    end;

    procedure GetSelectedLayout(): Text[250]
    begin
        if HeldName <> '' then
            exit(HeldName);
        exit(HeldCode);
    end;

    procedure GetSelectedAppID(): Guid
    begin
        exit(HeldApp);
    end;

    procedure ClearLayoutSelection()
    begin
        Clear(HeldName);
        Clear(HeldCode);
        Clear(HeldApp);
    end;

    local procedure Hold(Name: Text[250]; App: Guid)
    begin
        HeldName := Name;
        HeldApp := App;
    end;
}
"#;

/// Codeunit 44 with the SelectReportLayoutCode subscriber; `BUILTIN` is replaced by the SelectedBuiltinLayoutType body.
const REPORT_MANAGEMENT: &str = r#"
codeunit 44 "ReportManagement"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SelectReportLayoutCode', '', false, false)]
    local procedure SelectReportLayoutCode(ObjectId: Integer; var LayoutCode: Text; var LayoutType: Option "None",RDLC,Word,Excel,Custom; var Success: Boolean)
    var
        Holder: Codeunit "Design-time Report Selection";
        Company: Record "Report Layout Selection";
        Saved: Record "Custom Report Layout";
        Pick: Text;
        App: Guid;
    begin
        Pick := Holder.GetSelectedLayout();
        App := Holder.GetSelectedAppID();
        Holder.ClearLayoutSelection();
        if Pick = '' then
            if Company.Get(ObjectId, CompanyName()) then
                if Company.Type = Company.Type::"Custom Layout" then
                    Pick := Company."Custom Report Layout Code";
        LayoutType := LayoutType::None;
        if Pick = '' then
            exit;
        Success := true;
        LayoutCode := Pick;
        if StrLen(Pick) <= 20 then
            if Saved.Get(UpperCase(Pick)) then begin
                LayoutCode := Saved.Code;
                LayoutType := LayoutType::RDLC;
                if Saved.Type = Saved.Type::Word then
                    LayoutType := LayoutType::Word;
                exit;
            end;
        if not IsNullGuid(App) then
            LayoutCode := Pick + '::' + Format(App);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SelectedBuiltinLayoutType', '', false, false)]
    local procedure SelectedBuiltinLayoutType(ObjectID: Integer; var LayoutType: Option "None",RDLC,Word,Excel,Custom)
    BUILTIN
}
"#;

/// The company's built-in row answers its kind; no row or a Custom Layout row answers None.
const BUILTIN_FROM_ROW: &str = r#"var
        Company: Record "Report Layout Selection";
    begin
        LayoutType := LayoutType::None;
        if not Company.Get(ObjectID, CompanyName()) then
            exit;
        if Company.Type = Company.Type::"RDLC (built-in)" then
            LayoutType := LayoutType::RDLC
        else
            if Company.Type = Company.Type::"Word (built-in)" then
                LayoutType := LayoutType::Word
            else
                if Company.Type <> Company.Type::"Custom Layout" then
                    Error('stand-in: this row type is not modeled');
    end;"#;

/// BC does not raise SelectedBuiltinLayoutType after SelectReportLayoutCode answered a selected layout (corpus probe pull
/// request 1362, reports/layouts-html-text/HtRenderedTextTests.al:494, :530): a raise fails the test.
const BUILTIN_FAILS: &str = r#"begin
        Error('SelectedBuiltinLayoutType raised');
    end;"#;

fn compile(src: &str, builtin: &str) -> Built {
    let base = AppSource {
        id: BASE_APPLICATION_ID.into(),
        name: "Base Application".into(),
        dependencies: vec![],
        files: vec![(
            "base.al".into(),
            format!("{TABLES_AND_HOLDER}{}", REPORT_MANAGEMENT.replace("BUILTIN", builtin)),
        )],
    };
    let test = AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![BASE_APPLICATION_ID.into()],
        files: vec![("f.al".into(), src.into())],
    };
    Built::new(al_testkit::compile_apps(vec![base, test]))
}

/// `src` compiled in a test app over the stand-in; `src` does not declare the stand-in's objects.
pub fn built(src: &str) -> Built {
    compile(src, BUILTIN_FROM_ROW)
}

/// As [`built`], with a SelectedBuiltinLayoutType subscriber that fails the test when it is raised.
pub fn built_failing_on_builtin_type(src: &str) -> Built {
    compile(src, BUILTIN_FAILS)
}
