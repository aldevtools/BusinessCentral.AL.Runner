//! Pins of misc/lt-y14-triage (corpus master, writer Y14) that need test codeunits with handlers: a SubstituteReport
//! subscriber that swaps the report for the static RunRequestPage, Run, RunModal, Execute and SaveAs
//! (Y14ReportSubstitute.al), a Page-typed PageHandler for Page.Run (Y14PageTriage.al:383), SetValue of a Decimal on a text
//! control (Y14Small.al:237) and the record's filters after OK fails on Page.RunModal(0, Rec) (Y14LookupNoRow.al:148).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
report 69700 "YR A"
{
    ProcessingOnly = true;
    UseRequestPage = true;
    requestpage
    {
        layout { area(Content) { field(AField; AText) { ApplicationArea = All; } } }
        trigger OnOpenPage() begin Log.Add('A-rp-open'); end;
    }
    var AText: Text[30]; Log: Codeunit "YR Log";
    trigger OnInitReport() begin Log.Add('A-init'); end;
    trigger OnPreReport() begin Log.Add('A-pre(' + AText + ')'); end;
}
report 69701 "YR B"
{
    ProcessingOnly = true;
    UseRequestPage = true;
    requestpage
    {
        layout { area(Content) { field(BField; BText) { ApplicationArea = All; } } }
        trigger OnOpenPage() begin Log.Add('B-rp-open'); end;
    }
    var BText: Text[30]; Log: Codeunit "YR Log";
    trigger OnInitReport() begin Log.Add('B-init'); end;
    trigger OnPreReport() begin Log.Add('B-pre(' + BText + ')'); end;
}
codeunit 69702 "YR Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(E: Text) begin T += E + ';'; end;
    procedure Take() R: Text begin R := T; T := ''; end;
}
codeunit 69703 "YR Subs"
{
    EventSubscriberInstance = Manual;
    var Log: Codeunit "YR Log";
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SubstituteReport', '', false, false)]
    local procedure OnSubstitute(ReportId: Integer; RunMode: Option Normal,ParametersOnly,Execute,Print,SaveAs,RunModal; RequestPageXml: Text; RecordRef: RecordRef; var NewReportId: Integer)
    begin
        Log.Add('Sub(' + Format(RunMode) + ',' + Format(StrLen(RequestPageXml)) + ')');
        if ReportId = Report::"YR A" then NewReportId := Report::"YR B";
    end;
}
table 69704 "YP T" { fields { field(1; Code; Code[10]) { } field(2; Kind; Code[10]) { } field(3; Label; Text[30]) { } } keys { key(PK; Code) { } } }
page 69705 "YP List"
{
    PageType = List; SourceTable = "YP T"; SourceTableTemporary = true; UsageCategory = None; ApplicationArea = All;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } } }
    var Log: Codeunit "YR Log";
    trigger OnOpenPage() begin Log.Add('open'); end;
}
page 69706 "YP Card"
{
    PageType = Card; SourceTable = "YP T"; UsageCategory = None; ApplicationArea = All;
    layout { area(Content) { field(Code; Rec.Code) { } field(Label; Rec.Label) { } } }
}
codeunit 69707 T
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Log: Codeunit "YR Log";
    local procedure Check(Want: Text; Got: Text) begin if Want <> Got then Error('want [%1] got [%2]', Want, Got); end;
    [RequestPageHandler]
    procedure HandlerB(var Request: TestRequestPage "YR B") begin Log.Add('handler-B'); Request.BField.SetValue('b1'); Request.OK().Invoke(); end;
    [PageHandler]
    procedure TypedRun(var P: Page "YP List")
    var Cur: Record "YP T" temporary;
    begin
        P.GetRecord(Cur);
        Log.Add('handler cur=' + Cur.Code + ' find=' + Format(Cur.Find()));
    end;
    [TryFunction]
    local procedure TryRun() begin Report.Run(Report::"YR A"); end;
    local procedure ParamsA(): Text
    begin
        exit('<?xml version="1.0" standalone="yes"?><ReportParameters name="YR A" id="69700"><Options><Field name="AText">a1</Field></Options></ReportParameters>');
    end;

    [Test]
    [HandlerFunctions('HandlerB')]
    procedure RunRequestPageSwapped()
    var Subs: Codeunit "YR Subs"; P: Text;
    begin
        BindSubscription(Subs);
        Log.Take();
        P := Report.RunRequestPage(Report::"YR A");
        Check('Sub(ParametersOnly,0);B-init;B-rp-open;handler-B;', Log.Take());
        Check('<?xml version="1.0" standalone="yes"?><ReportParameters name="YR B" id="69701"><Options><Field name="BText">b1</Field></Options></ReportParameters>', P);
    end;

    [Test]
    [HandlerFunctions('HandlerB')]
    procedure RunSwappedInTryFunction()
    var Subs: Codeunit "YR Subs";
    begin
        BindSubscription(Subs);
        Commit();
        Log.Take();
        Check('Yes', Format(TryRun()));
        Check('Sub(Normal,0);B-init;B-rp-open;handler-B;B-pre(b1);', Log.Take());
    end;

    [Test]
    procedure ExecuteAndSaveAsSwapped()
    var Subs: Codeunit "YR Subs"; B: Codeunit "Temp Blob Y"; OutS: OutStream; Ok: Boolean;
    begin
        BindSubscription(Subs);
        Log.Take();
        Report.Execute(Report::"YR A", ParamsA());
        Check('Sub(Execute,' + Format(StrLen(ParamsA())) + ');B-init;B-rp-open;B-pre();', Log.Take());
        B.Out(OutS);
        Ok := Report.SaveAs(Report::"YR A", ParamsA(), ReportFormat::Xml, OutS);
        Check('No', Format(Ok));
        Check('Report 69701 is a processing-only report. Output cannot be rendered and saved to a file.', GetLastErrorText());
        Check('ReportProcessingOnly', GetLastErrorCode());
        Check('Sub(SaveAs,' + Format(StrLen(ParamsA())) + ');B-init;', Log.Take());
    end;

    [Test]
    [HandlerFunctions('TypedRun')]
    procedure PageTypedPageHandler()
    begin
        Log.Take();
        Page.Run(Page::"YP List");
        Check('handler cur= find=No;', Log.Take());
    end;

    [Test]
    procedure SetValueDecimalOnText()
    var R: Record "YP T"; P: TestPage "YP Card";
    begin
        R.Code := 'C1'; R.Insert(); Commit();
        P.OpenEdit();
        P.Label.SetValue(1.11);
        Check('1.11', P.Label.Value());
        P.Label.SetValue(1234567.5);
        Check('1,234,567.5', P.Label.Value());
        P.Label.SetValue(-3.50);
        Check('-3.5', P.Label.Value());
        P.Close();
    end;
}
codeunit 69708 "Temp Blob Y"
{
    var R: Record "YB" temporary;
    procedure Out(var OutS: OutStream) begin R.Data.CreateOutStream(OutS); end;
}
table 69709 "YB" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
table 69710 "YL H"
{
    LookupPageId = "YL List";
    fields { field(1; "No."; Code[20]) { } field(2; Kind; Code[10]) { } }
    keys { key(PK; "No.") { } }
}
page 69711 "YL List"
{
    PageType = List; SourceTable = "YL H"; Editable = false; UsageCategory = None; ApplicationArea = All;
    layout { area(Content) { repeater(R) { field("No."; Rec."No.") { } } } }
}
codeunit 69712 L
{
    Subtype = Test;
    TestPermissions = Disabled;
    [ModalPageHandler]
    procedure OkHandler(var P: TestPage "YL List") begin P.First(); P.OK().Invoke(); end;
    [Test]
    [HandlerFunctions('OkHandler')]
    procedure LookupKeepsFilters()
    var H: Record "YL H"; Result: Action;
    begin
        H."No." := 'A'; H.Kind := 'ORDER'; H.Insert();
        H."No." := 'B'; H.Kind := 'QUOTE'; H.Insert();
        Commit();
        H.FilterGroup(2); H.SetFilter(Kind, 'ORDER'); H.FilterGroup(0);
        H.SetView('WHERE(No.=CONST(X))');
        asserterror Result := Page.RunModal(0, H);
        if GetLastErrorCode() <> 'TestActionNotFound' then Error('code %1', GetLastErrorCode());
        if H.Count() <> 0 then Error('count %1', H.Count());
    end;
}
"#;

#[test]
fn y14_pins_with_handlers() {
    let out = outcomes_in(SRC, "T");
    let bad: Vec<_> = out.iter().filter(|(_, o)| *o != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:#?}");
    assert_eq!(out.len(), 5);
}

#[test]
fn a_lookup_over_the_lookup_page_id_keeps_the_filters_when_ok_fails() {
    let out = outcomes_in(SRC, "L");
    assert_eq!(out, vec![("LookupKeepsFilters".to_string(), Outcome::Pass)]);
}
