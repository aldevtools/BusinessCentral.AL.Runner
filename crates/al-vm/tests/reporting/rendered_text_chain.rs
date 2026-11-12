//! The shape of an outside reporter's e-mail body chain (al-runner issue 6924), with minimal stand-in AL, not Microsoft's code:
//! a Word layout report saved as Html into a temporary Blob, copied into the e-mail item's body, read whole as text, joined
//! with a signature in a BigText by a subscriber and formatted, stored in a message Blob, and read back from a page field. ADR-040
//! amendment 2026-11-12, A12: the chain passes (flagged), its `Contains`, comparison and `StrLen` twins are not run. The same
//! chain with the save behind a temporary selection of a user-defined Word layout (A7, the reporter's `Report Selections` path)
//! runs with stand-ins of Base Application's "Design-time Report Selection" and ReportManagement in an app with Base
//! Application's id.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

const CHAIN: &str = r#"
table 50995 "RC Message"
{
    fields { field(1; Id; Integer) { } field(2; Body; Blob) { } }
    keys { key(PK; Id) { } }
}

// Stand-in for Email Item: SetBody copies the saved Html, GetBodyText reads it whole.
codeunit 50996 "RC Item"
{
    var Body: Record "RC Message" temporary;
    procedure SetBody(var Source: Record "RT Row" temporary)
    var I: InStream; O: OutStream;
    begin
        Body.Id := 1;
        Source.Data.CreateInStream(I);
        Body.Body.CreateOutStream(O);
        CopyStream(O, I);
        Body.Insert();
    end;
    procedure GetBodyText(): Text
    var I: InStream; T: Text;
    begin
        Body.Get(1);
        Body.Body.CreateInStream(I);
        I.Read(T);
        exit(T);
    end;
}

// Stand-in for Email Message: Create raises the subscriber's event, stores the body in a Blob, GetBody reads it back.
codeunit 50997 "RC Message"
{
    SingleInstance = true;
    var Stored: Record "RC Message" temporary;
    procedure Create(Body: Text)
    var O: OutStream; Handled: Boolean;
    begin
        OnBeforeCreate(Body, Handled);
        Stored.DeleteAll();
        Stored.Id := 1;
        Stored.Body.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText(Body);
        Stored.Insert();
    end;
    procedure GetBody(): Text
    var I: InStream; T: Text;
    begin
        Stored.Get(1);
        Stored.Body.CreateInStream(I, TextEncoding::UTF8);
        I.Read(T);
        exit(T);
    end;
    [IntegrationEvent(false, false)]
    procedure OnBeforeCreate(var Body: Text; var Handled: Boolean) begin end;
}

// The partner subscriber: BodyText.AddText(Body) + a signature, then Format(BodyText).
codeunit 50998 "RC Signature"
{
    EventSubscriberInstance = StaticAutomatic;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RC Message", 'OnBeforeCreate', '', false, false)]
    local procedure AddSignature(var Body: Text; var Handled: Boolean)
    var BodyText: BigText;
    begin
        BodyText.AddText(Body);
        BodyText.AddText('<p>Signature</p>');
        Body := Format(BodyText);
    end;
}

// Stand-in for the e-mail editor: a field over a Text variable.
page 50999 "RC Editor"
{
    PageType = Card;
    layout { area(Content) { field(BodyField; BodyText) { ApplicationArea = All; } } }
    var BodyText: Text;
    trigger OnOpenPage() var Msg: Codeunit "RC Message"; begin BodyText := Msg.GetBody(); end;
}

page 51001 "RC Rich Editor"
{
    PageType = Card;
    layout { area(Content) { field(BodyField; BodyText) { ApplicationArea = All; MultiLine = true; } } }
    var BodyText: Text;
    trigger OnOpenPage() var Msg: Codeunit "RC Message"; begin BodyText := Msg.GetBody(); end;
}

codeunit 51000 "RC Tests"
{
    Subtype = Test;
    var Kept: Text;

    local procedure SendBody(): Text
    var Tmp: Record "RT Row" temporary; R: Record "RT Row"; Item: Codeunit "RC Item"; Msg: Codeunit "RC Message"; Out: OutStream;
    begin
        R.DeleteAll();
        R.Id := 1; R.Insert();
        Tmp.Id := 1;
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RT Word", '', ReportFormat::Html, Out);
        Tmp.Insert();
        Item.SetBody(Tmp);
        Msg.Create(Item.GetBodyText());
        exit(Msg.GetBody());
    end;

    procedure PassThrough(): Text var Body: Text; begin Body := SendBody(); exit(Format(Body <> '')); end;
    procedure AssertNotEmpty() var Body: Text; begin Body := SendBody(); if Body = '' then Error('empty body'); end;
    procedure BodyContains(): Boolean begin exit(SendBody().Contains('NLB')); end;
    procedure BodyCompared(): Boolean begin exit(SendBody() = '<html>'); end;
    procedure BodyLength(): Integer begin exit(StrLen(SendBody())); end;

    // The editor shows the body the message holds; the test reads the field (the reporter's handler reads
    // EmailEditor.BodyField.Value()).
    procedure ThroughTheEditor(): Text
    var Editor: TestPage "RC Editor";
    begin
        SendBody();
        Editor.OpenView();
        Kept := Editor.BodyField.Value();
        exit(Format(Kept <> ''));
    end;
    procedure ThroughTheRichEditor(): Text
    var Editor: TestPage "RC Rich Editor";
    begin
        SendBody();
        Editor.OpenView();
        Kept := Editor.BodyField.Value();
        exit(Format(Kept <> ''));
    end;
}
"#;

fn src() -> String {
    format!("{}{CHAIN}", crate::rendered_text::FIXTURE)
}

fn run(proc: &str) -> Result<Value, Signal> {
    Built::from_source(&src()).run_in("RC Tests", proc)
}

#[test]
fn rendered_text_chain_the_body_passes_through_and_is_not_empty() {
    assumptions::clear_hits();
    assert_eq!(al_testkit::text(run("PassThrough")), "Yes");
    assert!(assumptions::take_hits().contains(&Assumption::ReportHtmlRenderedText));
    run("AssertNotEmpty").expect("AreNotEqual('', Body) passes");
}

#[test]
fn rendered_text_chain_contains_compare_and_strlen_of_the_body_refuse() {
    for p in ["BodyContains", "BodyCompared", "BodyLength"] {
        match run(p) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("rendered content of report RT Word"), "{p}: {why}"),
            other => panic!("{p}: expected a refusal, got {other:?}"),
        }
    }
}

#[test]
fn rendered_text_chain_a_plain_page_field_over_the_body_shows_it() {
    // P11: a field over a Text variable with no property that changes what it shows.
    assumptions::clear_hits();
    assert_eq!(al_testkit::text(run("ThroughTheEditor")), "Yes");
    assert!(assumptions::take_hits().contains(&Assumption::ReportHtmlRenderedText));
}

#[test]
fn rendered_text_chain_a_multiline_page_field_over_the_body_refuses_until_measured() {
    match run("ThroughTheRichEditor") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("rendered content"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Stand-ins, in an app with Base Application's id, for the parts of Base Application the selection of a user-defined layout
/// takes: "Design-time Report Selection" holds the name, ReportManagement's SelectReportLayoutCode subscriber hands it to the
/// platform when no Custom Report Layout row has it (none exists here) and clears it, and its SelectedBuiltinLayoutType
/// subscriber answers from "Report Layout Selection".
const BASE: &str = r#"
table 9651 "Report Layout Selection"
{
    fields
    {
        field(1; "Report ID"; Integer) { }
        field(3; "Company Name"; Text[30]) { }
        field(4; Type; Option) { OptionMembers = "RDLC (built-in)","Word (built-in)","Custom Layout","Excel Layout","External Layout"; }
    }
    keys { key(PK; "Report ID", "Company Name") { } }
}

codeunit 9654 "Design-time Report Selection"
{
    SingleInstance = true;
    var SelectedCustomLayoutCode: Code[20]; SelectedPlatformLayoutName: Text[250]; SelectedPlatformLayoutAppID: Guid; EmptyGuid: Guid;
    procedure SetSelectedLayout(LayoutName: Text[250]; AppID: Guid) begin SelectedPlatformLayoutName := LayoutName; SelectedPlatformLayoutAppID := AppID; end;
    procedure GetSelectedLayout(): Text[250] begin if SelectedPlatformLayoutName = '' then exit(SelectedCustomLayoutCode); exit(SelectedPlatformLayoutName); end;
    procedure GetSelectedAppID(): Guid begin exit(SelectedPlatformLayoutAppID); end;
    procedure ClearLayoutSelection() begin SelectedPlatformLayoutName := ''; SelectedCustomLayoutCode := ''; SelectedPlatformLayoutAppID := EmptyGuid; end;
}

codeunit 44 "ReportManagement"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SelectReportLayoutCode', '', false, false)]
    local procedure SelectReportLayoutCode(ObjectId: Integer; var LayoutCode: Text; var LayoutType: Option "None",RDLC,Word,Excel,Custom; var Success: Boolean)
    var D: Codeunit "Design-time Report Selection"; Name: Text[250]; AppId: Guid;
    begin
        LayoutType := LayoutType::None;
        Name := D.GetSelectedLayout();
        AppId := D.GetSelectedAppID();
        if Name <> '' then begin
            if IsNullGuid(AppId) then
                LayoutCode := Name
            else
                LayoutCode := Name + '::' + Format(AppId);
            Success := true;
        end;
        D.ClearLayoutSelection();
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SelectedBuiltinLayoutType', '', false, false)]
    local procedure SelectedBuiltinLayoutType(ObjectID: Integer; var LayoutType: Option "None",RDLC,Word,Excel,Custom)
    var S: Record "Report Layout Selection";
    begin
        LayoutType := LayoutType::None;
        if S.Get(ObjectID, CompanyName()) then
            case S.Type of
                S.Type::"RDLC (built-in)": LayoutType := LayoutType::RDLC;
                S.Type::"Word (built-in)": LayoutType := LayoutType::Word;
            end;
    end;
}
"#;

/// The test app: the platform table Tenant Report Layout (a table of its number and name stands in), a stand-in for
/// `Report Selections`.`SaveReportAsHTMLImpl` (select the layout by name, save as Html, clear), and the chain over it.
const SELECTED: &str = r#"
table 2000000232 "Tenant Report Layout"
{
    fields
    {
        field(1; "Report ID"; Integer) { } field(2; Name; Text[250]) { } field(5; "App ID"; Guid) { }
        field(6; "Company Name"; Text[30]) { } field(7; Layout; Media) { }
        field(8; "Layout Format"; Option) { OptionMembers = RDLC,Word,Excel,Custom; }
    }
    keys { key(PK; "Report ID", Name, "App ID") { } }
}

table 2000000233 "Tenant Report Layout Selection"
{
    fields { field(1; "Report ID"; Integer) { } field(2; "Company Name"; Text[30]) { } field(3; "Layout Name"; Text[250]) { } }
    keys { key(PK; "Report ID", "Company Name") { } }
}

codeunit 51002 "RC Selections"
{
    procedure SaveReportAsHTML(LayoutName: Text[250]; AppId: Guid; var Tmp: Record "RT Row" temporary): Boolean
    var D: Codeunit "Design-time Report Selection"; Out: OutStream; Ok: Boolean;
    begin
        D.SetSelectedLayout(LayoutName, AppId);
        Tmp.Id := 1;
        Tmp.Data.CreateOutStream(Out, TextEncoding::UTF8);
        Ok := Report.SaveAs(Report::"RT Word", '', ReportFormat::Html, Out);
        Tmp.Insert();
        D.ClearLayoutSelection();
        exit(Ok);
    end;
}

codeunit 51003 "RC Selection Tests"
{
    var LayoutName: Text[250];

    local procedure AddLayout(Kind: Option RDLC,Word,Excel,Custom; Company: Text[30])
    var L: Record "Tenant Report Layout"; H: Record "RT Row" temporary; O: OutStream; I: InStream;
    begin
        LayoutName := '{6F1D2C3B-0A4E-4B7C-9D8E-1F2A3B4C5D6E}';
        H.Data.CreateOutStream(O);
        if Kind = Kind::RDLC then O.WriteText('<Report/>') else O.WriteText('PK-docx');
        H.Data.CreateInStream(I);
        L.Init();
        L."Report ID" := Report::"RT Word";
        L.Name := LayoutName;
        L."Layout Format" := Kind;
        L."Company Name" := Company;
        L.Layout.ImportStream(I, 'body.docx');
        L.Insert();
    end;

    local procedure Seed()
    var R: Record "RT Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.Insert();
    end;

    local procedure SendSelected(Name: Text[250]; AppId: Guid): Text
    var Tmp: Record "RT Row" temporary; Sel: Codeunit "RC Selections"; Item: Codeunit "RC Item"; Msg: Codeunit "RC Message";
    begin
        Seed();
        if not Sel.SaveReportAsHTML(Name, AppId, Tmp) then
            Error('not saved');
        Item.SetBody(Tmp);
        Msg.Create(Item.GetBodyText());
        exit(Msg.GetBody());
    end;

    local procedure SendWord(): Text
    var NullId: Guid;
    begin
        AddLayout(1, '');
        exit(SendSelected(LayoutName, NullId));
    end;

    procedure PassThrough(): Text
    var D: Codeunit "Design-time Report Selection"; Body: Text;
    begin
        Body := SendWord();
        exit(Format(Body <> '') + Format(D.GetSelectedLayout() = ''));
    end;
    procedure AssertNotEmpty() var Body: Text; begin Body := SendWord(); if Body = '' then Error('empty body'); end;
    procedure BodyContains(): Boolean begin exit(SendWord().Contains('NLB')); end;
    procedure BodyCompared(): Boolean begin exit(SendWord() = '<html>'); end;
    procedure BodyLength(): Integer begin exit(StrLen(SendWord())); end;

    // Neighbours that stay refused.
    procedure RdlcLayout(): Text var NullId: Guid; begin AddLayout(0, ''); exit(SendSelected(LayoutName, NullId)); end;
    procedure NameMatchesNothing(): Text var NullId: Guid; begin AddLayout(1, ''); exit(SendSelected('Other', NullId)); end;
    procedure LayoutOfOneCompany(): Text var NullId: Guid; begin AddLayout(1, CompanyName()); exit(SendSelected(LayoutName, NullId)); end;
    procedure WithAppId(): Text
    var Id: Guid;
    begin
        AddLayout(1, '');
        Evaluate(Id, '{437DBF0E-84FF-417A-965D-ED2BB9650972}');
        exit(SendSelected(LayoutName, Id));
    end;
    procedure WithBuiltinSelection(): Text
    var S: Record "Report Layout Selection"; NullId: Guid;
    begin
        AddLayout(1, '');
        S."Report ID" := Report::"RT Word"; S."Company Name" := CompanyName(); S.Type := S.Type::"Word (built-in)"; S.Insert();
        exit(SendSelected(LayoutName, NullId));
    end;
    procedure WithTenantSelection(): Text
    var S: Record "Tenant Report Layout Selection"; NullId: Guid;
    begin
        AddLayout(1, '');
        S."Report ID" := Report::"RT Word"; S."Layout Name" := 'WordOne'; S.Insert();
        exit(SendSelected(LayoutName, NullId));
    end;
    procedure AsPdf(): Text
    var Tmp: Record "RT Row" temporary; D: Codeunit "Design-time Report Selection"; NullId: Guid; Out: OutStream;
    begin
        AddLayout(1, '');
        Seed();
        D.SetSelectedLayout(LayoutName, NullId);
        Tmp.Data.CreateOutStream(Out);
        exit(Format(Report.SaveAs(Report::"RT Word", '', ReportFormat::Pdf, Out)));
    end;
}
"#;

fn selected_run(proc: &str) -> Result<Value, Signal> {
    use al_testkit::AppSource;
    const BASE_ID: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";
    let base = AppSource {
        id: BASE_ID.into(),
        name: "Base Application".into(),
        dependencies: vec![],
        files: vec![("base.al".into(), BASE.into())],
    };
    let test = AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![BASE_ID.into()],
        files: vec![("f.al".into(), format!("{}{SELECTED}", src()))],
    };
    Built::new(al_testkit::compile_apps(vec![base, test])).run_in("RC Selection Tests", proc)
}

#[test]
fn rendered_text_chain_a_temporary_selection_of_a_user_defined_word_layout_passes_the_body_through() {
    assumptions::clear_hits();
    assert_eq!(al_testkit::text(selected_run("PassThrough")), "YesYes");
    let hits = assumptions::take_hits();
    assert!(hits.contains(&Assumption::ReportLayoutTempSelection), "{hits:?}");
    assert!(hits.contains(&Assumption::ReportHtmlRenderedText), "{hits:?}");
    selected_run("AssertNotEmpty").expect("AreNotEqual('', Body) passes");
}

#[test]
fn rendered_text_chain_contains_compare_and_strlen_of_a_selected_layouts_body_refuse() {
    for p in ["BodyContains", "BodyCompared", "BodyLength"] {
        match selected_run(p) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("rendered content of report RT Word"), "{p}: {why}"),
            other => panic!("{p}: expected a refusal, got {other:?}"),
        }
    }
}

#[test]
fn rendered_text_chain_other_temporary_selections_stay_refused() {
    for (p, want) in [
        ("RdlcLayout", "is not one Word layout"),
        (
            "NameMatchesNothing",
            "no Tenant Report Layout row of the report is named \"Other\"",
        ),
        ("LayoutOfOneCompany", "is not one Word layout"),
        ("WithAppId", "the selection names an app id"),
        ("WithBuiltinSelection", "SelectedBuiltinLayoutType answered"),
        (
            "WithTenantSelection",
            "a Tenant Report Layout Selection row names the report",
        ),
        // Main's answer: a save as another format while a selection is held refuses (here for ReportManagement's subscriber,
        // which this test can see through the codeunit it names).
        ("AsPdf", "Report.SaveAs as Pdf"),
    ] {
        match selected_run(p) {
            Err(Signal::Unsupported(why)) => assert!(why.contains(want), "{p}: {why}"),
            other => panic!("{p}: expected a refusal naming {want}, got {other:?}"),
        }
    }
}
