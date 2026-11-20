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
    procedure RawBody(): Text begin exit(SendWord()); end;
    procedure BodyContains(): Boolean begin exit(SendWord().Contains('NLB')); end;
    procedure BodyCompared(): Boolean begin exit(SendWord() = '<html>'); end;
    procedure BodyLength(): Integer begin exit(StrLen(SendWord())); end;

    // Neighbours that stay refused.
    procedure RdlcLayout(): Text var NullId: Guid; begin AddLayout(0, ''); exit(SendSelected(LayoutName, NullId)); end;
    procedure NameMatchesNothing(): Text var NullId: Guid; begin AddLayout(1, ''); exit(SendSelected('Other', NullId)); end;
    procedure UnknownCustomCode(): Text
    var Tmp: Record "RT Row" temporary; D: Codeunit "Design-time Report Selection"; Out: OutStream;
    begin
        AddLayout(1, '');
        Seed();
        D.SetSelectedCustomLayout('HTNOSUCHCODE');
        Tmp.Data.CreateOutStream(Out, TextEncoding::UTF8);
        exit(Format(Report.SaveAs(Report::"RT Word", '', ReportFormat::Html, Out)));
    end;
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
    // Base Application's objects: `report_management_stand_in`; its SelectedBuiltinLayoutType subscriber fails the test when
    // it is raised.
    super::report_management_stand_in::built_failing_on_builtin_type(&format!("{}{SELECTED}", src()))
        .run_in("RC Selection Tests", proc)
}

#[test]
fn rendered_text_chain_a_temporary_selection_of_a_user_defined_word_layout_passes_the_body_through() {
    assumptions::clear_hits();
    assert_eq!(al_testkit::text(selected_run("PassThrough")), "YesYes");
    let hits = assumptions::take_hits();
    assert!(hits.contains(&Assumption::ReportLayoutTempSelection), "{hits:?}");
    assert!(hits.contains(&Assumption::ReportHtmlRenderedText), "{hits:?}");
    selected_run("AssertNotEmpty").expect("AreNotEqual('', Body) passes");
    // Over a Report Layout Selection row of the company: BC raises no SelectedBuiltinLayoutType once a selected layout was
    // answered (HtRenderedTextTests.al:494), so the row is not read (RxdDeclLayoutTests.al:811, :1336, corpus pull request
    // 1381; the stand-in's subscriber fails the test when it is raised).
    match selected_run("WithBuiltinSelection") {
        Ok(Value::Text(t)) => assert!(t.as_str().ends_with("<p>Signature</p>"), "{t:?}"),
        other => panic!("expected the selected layout's body, got {other:?}"),
    }
}

/// The Html of the user-defined Word layout a temporary selection named keeps that as the layout of its render: the form
/// the rgba replace of `Email Message Impl.` is measured on (corpus pull request 1371,
/// reports/html-email-body/RxTenantLayoutBodyTests.al:121 TenantGuid_WordAlpha_EmailBody; the rgba column of the save's row).
#[test]
fn rendered_text_chain_a_selected_layouts_render_is_the_selected_tenant_word_layout() {
    let body = match selected_run("RawBody") {
        Ok(Value::Text(t)) => t.as_str().to_string(),
        other => panic!("expected a text, got {other:?}"),
    };
    let marks = al_builtins::rendered_text::marks(&body).expect("whole marks");
    let render = al_builtins::rendered_text::render(marks[0].0).expect("registered");
    assert_eq!(render.rgba_replace, al_builtins::rendered_text::RgbaReplace::Measured);
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
        // A held selection that does not name one user-defined Word layout of the report (ReportManagement hands the name
        // to the platform: SelectReportLayoutCode answers it with LayoutType None) is a design-time selection no row answers.
        ("RdlcLayout", "layout selection = a design-time selection"),
        ("NameMatchesNothing", "layout selection = a design-time selection"),
        // A name or a custom layout code that matches nothing: BC answers false and writes nothing (S5,
        // HtRenderedTextTests.al:543-603); not answered here yet.
        ("UnknownCustomCode", "layout selection = a design-time selection"),
        ("LayoutOfOneCompany", "layout selection = a design-time selection"),
        ("WithAppId", "layout selection = a design-time selection"),
        (
            "WithTenantSelection",
            "layout selection another Report Layout Selection or Tenant Report Layout Selection row",
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
