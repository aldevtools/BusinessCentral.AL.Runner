//! The `Reporting Triggers` events of an Xml `Report.SaveAs`: `SubstituteReport` before the run and `OnDocumentReady`
//! after it (events/lt-platform-events-v/PeVTests.al:432 SaveAs_Xml_RaisedEvents, :483
//! SubstituteReport_SubscriberChangesTheReport, :513 ChangeVarArguments_XmlSaveAs_Second).

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const FIXTURE: &str = r#"
table 50970 "SE Row"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { } }
}

codeunit 50971 "SE Log"
{
    SingleInstance = true;
    var Entries: Text; Mode: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take(): Text var Result: Text; begin Result := Entries; Entries := ''; exit(Result); end;
    procedure SetMode(NewMode: Text) begin Mode := NewMode; end;
    procedure GetMode(): Text begin exit(Mode); end;
}

report 50972 "SE Report A"
{
    DefaultRenderingLayout = Rdlc;
    dataset { dataitem(T; "SE Row") { column(TId; Id) { } } }
    rendering { layout(Rdlc) { Type = RDLC; LayoutFile = './a.rdl'; } }
}

report 50973 "SE Report B"
{
    DefaultRenderingLayout = Rdlc;
    dataset { dataitem(T; "SE Row") { column(TId; Id) { } } }
    rendering { layout(Rdlc) { Type = RDLC; LayoutFile = './b.rdl'; } }
}

codeunit 50974 "SE Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SubstituteReport', '', false, false)]
    local procedure Substitute(ReportId: Integer; RunMode: Option Normal,ParametersOnly,Execute,Print,SaveAs,RunModal; RequestPageXml: Text; RecordRef: RecordRef; var NewReportId: Integer)
    var L: Codeunit "SE Log";
    begin
        L.Add('SubstituteReport(' + Format(ReportId) + ',' + Format(RunMode) + ',in=' + Format(NewReportId) + ')');
        if L.GetMode() = 'substitute' then
            NewReportId := Report::"SE Report B";
        if L.GetMode() = 'substitute unknown' then
            NewReportId := 50999;
        if L.GetMode() = 'req' then
            L.Add('req=' + RequestPageXml);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'OnDocumentReady', '', false, false)]
    local procedure Ready(ObjectId: Integer; ObjectPayload: JsonObject; DocumentStream: InStream; var TargetStream: OutStream; var Success: Boolean)
    var L: Codeunit "SE Log"; Line: Text;
    begin
        L.Add('OnDocumentReady(' + Format(ObjectId) + ',in=' + Format(Success) + ')');
        if L.GetMode() = 'ready' then begin
            TargetStream.WriteText('SE ready');
            Success := true;
        end;
        if L.GetMode() = 'ready no success' then
            TargetStream.WriteText('SE ready');
        if L.GetMode() = 'read document' then
            DocumentStream.ReadText(Line);
    end;
}

codeunit 50975 "SE Tests"
{
    procedure SaveAs(Mode: Text): Text
    var
        R: Record "SE Row";
        L: Codeunit "SE Log";
        OutS: OutStream;
        InS: InStream;
        Ok: Boolean;
        Head: Text;
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Insert();
        L.Take();
        L.SetMode(Mode);
        R.Data.CreateOutStream(OutS);
        Ok := Report.SaveAs(Report::"SE Report A", '', ReportFormat::Xml, OutS);
        R.Modify();
        R.CalcFields(Data);
        R.Data.CreateInStream(InS, TextEncoding::UTF8);
        InS.ReadText(Head, 9);
        exit(Format(Ok) + '|' + Head + '|' + L.Take());
    end;

    procedure Html(): Text
    var
        R: Record "SE Row";
        L: Codeunit "SE Log";
        OutS: OutStream;
        Ok: Boolean;
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Insert();
        L.Take();
        L.SetMode('req');
        R.Data.CreateOutStream(OutS);
        Ok := Report.SaveAs(Report::"SE Report A", 'MARK', ReportFormat::Html, OutS);
        exit(Format(Ok) + '|' + GetLastErrorText() + '|' + L.Take());
    end;

    procedure Plain(): Text begin exit(SaveAs('')); end;
    procedure Substituted(): Text begin exit(SaveAs('substitute')); end;
    procedure SubstitutedUnknown(): Text begin exit(SaveAs('substitute unknown')); end;
    procedure Ready(): Text begin exit(SaveAs('ready')); end;
    procedure ReadyNoSuccess(): Text begin exit(SaveAs('ready no success')); end;
    procedure ReadDocument(): Text begin exit(SaveAs('read document')); end;
}
"#;

fn run(routine: &str) -> Result<Value, Signal> {
    al_testkit::run_in(FIXTURE, "SE Tests", routine)
}

fn assert_refused(r: Result<Value, Signal>, needle: &str) {
    match r {
        Err(Signal::Unsupported(m)) => assert!(m.contains(needle), "{m}"),
        other => panic!("expected a refusal containing {needle:?}, got {other:?}"),
    }
}

/// PeVTests.al:432: SubstituteReport(id, SaveAs, in=-1), then OnDocumentReady(id, in=No), and the Xml dataset is saved.
#[test]
fn an_xml_saveas_raises_substitutereport_then_ondocumentready() {
    assert_eq!(
        text(run("Plain")),
        "Yes|<?xml ver|SubstituteReport(50972,SaveAs,in=-1);OnDocumentReady(50972,in=No);"
    );
}

/// PeVTests.al:483: a subscriber that sets NewReportId runs that report instead; OnDocumentReady names it.
#[test]
fn a_substitutereport_subscriber_changes_the_report() {
    assert_eq!(
        text(run("Substituted")),
        "Yes|<?xml ver|SubstituteReport(50972,SaveAs,in=-1);OnDocumentReady(50973,in=No);"
    );
}

/// report/lt-am-rdlc-output/AMRdlcOutput.al:TextRdlc_SaveAsHtml_IsRefused: SubstituteReport, then the layout error. A subscriber
/// that does not read RequestPageXml runs.
#[test]
fn an_html_saveas_of_an_rdlc_report_raises_substitutereport_then_fails() {
    let fixture = FIXTURE.replace("L.Add('req=' + RequestPageXml);", "L.Add('req');");
    assert_eq!(
        text(al_testkit::run_in(&fixture, "SE Tests", "Html")),
        "No|The Html output format is not supported by the current report layout format.|SubstituteReport(50972,SaveAs,in=-1);req;"
    );
}

/// AMRdlcOutput.al:TextRdlc_SaveAsHtml_IsRefused pins only that RequestPageXml of an Html SaveAs contains the parameters, not
/// its text: a SubstituteReport subscriber that reads it is refused.
#[test]
fn an_html_saveas_whose_substitutereport_subscriber_reads_requestpagexml_is_refused() {
    assert_refused(run("Html"), "RequestPageXml");
    for read in [
        "Format(StrPos(RequestPageXml, 'MARK') > 0)",
        "Format(StrLen(RequestPageXml))",
        "Format(RequestPageXml.Contains('MARK'))",
        "Format(RequestPageXml)",
    ] {
        let fixture = FIXTURE.replace("L.Add('req=' + RequestPageXml);", &format!("L.Add({read});"));
        assert_refused(al_testkit::run_in(&fixture, "SE Tests", "Html"), "RequestPageXml");
    }
}

/// A NewReportId no report declares: what BC does is not pinned.
#[test]
fn a_substitute_with_an_unknown_id_is_refused() {
    assert_refused(run("SubstitutedUnknown"), "50999");
}

/// PeVTests.al:513: an OnDocumentReady subscriber that writes TargetStream and sets Success saves what it wrote.
#[test]
fn an_ondocumentready_subscriber_that_succeeds_saves_its_target_stream() {
    assert_eq!(
        text(run("Ready")),
        "Yes|SE ready|SubstituteReport(50972,SaveAs,in=-1);OnDocumentReady(50972,in=No);"
    );
}

/// A write to TargetStream without Success is not pinned.
#[test]
fn a_target_stream_written_without_success_is_refused() {
    assert_refused(run("ReadyNoSuccess"), "TargetStream");
}

/// The content of DocumentStream is not pinned.
#[test]
fn a_read_of_the_document_stream_is_refused() {
    assert_refused(run("ReadDocument"), "SE Report A");
}

const HEADER_FIXTURE: &str = r#"
table 79 "Company Information"
{
    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(2; Name; Text[100]) { }
        field(4; Address; Text[100]) { }
        field(5; "Address 2"; Text[50]) { }
        field(6; City; Text[30]) { }
        field(30; "Post Code"; Code[20]) { }
        field(31; County; Text[30]) { }
        field(36; "Country/Region Code"; Code[10]) { }
    }
    keys { key(PK; "Primary Key") { } }
}

table 50976 "SE Blob"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { } }
}

report 50977 "SE Header"
{
    dataset { dataitem(T; "SE Blob") { column(TId; Id) { } } }
}

codeunit 50978 "SE Header Tests"
{
    procedure SaveAs(City: Text): Boolean
    var
        Info: Record "Company Information";
        B: Record "SE Blob";
        OutS: OutStream;
    begin
        Info.Name := 'Co';
        Info.City := City;
        Info.Insert();
        B.Id := 1;
        B.Insert();
        B.Data.CreateOutStream(OutS);
        exit(Report.SaveAs(Report::"SE Header", '', ReportFormat::Xml, OutS));
    end;

    procedure AddressOnly(): Text
    var
        Info: Record "Company Information";
        B: Record "SE Blob";
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        Result: Text;
    begin
        Info.Name := 'Co';
        Info.Address := 'Street 1';
        Info.Insert();
        B.Id := 1;
        B.Insert();
        B.Data.CreateOutStream(OutS);
        Report.SaveAs(Report::"SE Header", '', ReportFormat::Xml, OutS);
        B.Modify();
        B.CalcFields(Data);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            if Line.Contains('CompanyAddressLine') and not Line.Contains('CompanyAddressLines') then
                Result += Line.Trim();
        end;
        exit(Result);
    end;

    procedure NoAddress(): Boolean begin exit(SaveAs('')); end;
    procedure WithCity(): Boolean begin exit(SaveAs('Town')); end;
}
"#;

/// The city line of the header depends on the General Ledger Setup's local address format (report/lt-ah-dataset-header/
/// AhDatasetHeader.al:125-126); without that table a Company Information row with City set is refused.
#[test]
fn a_header_with_company_address_fields_is_refused() {
    assert_refused(
        al_testkit::run_in(HEADER_FIXTURE, "SE Header Tests", "WithCity"),
        "address",
    );
    assert!(al_testkit::boolean(al_testkit::run_in(
        HEADER_FIXTURE,
        "SE Header Tests",
        "NoAddress"
    )));
}

/// misc/lt-probes-needed-2/PnBReportSaveAs.al:392-396 (OtherFormats_NoLayout): the ordinals of ReportFormat.
#[test]
fn reportformat_has_the_platform_ordinals() {
    let src = r#"
codeunit 50979 "SE Ordinals"
{
    procedure F(): Text
    begin
        exit(Format(ReportFormat::Excel, 0, 2) + Format(ReportFormat::Pdf, 0, 2) + Format(ReportFormat::Word, 0, 2)
            + Format(ReportFormat::Xml, 0, 2) + Format(ReportFormat::Html, 0, 2));
    end;
}
"#;
    assert_eq!(text(al_testkit::run_in(src, "SE Ordinals", "F")), "12345");
}

/// AhDatasetHeader.al:120-124: Address is line 2 whenever it is set (report/W3ReportDataset.al:204 sets only Address).
#[test]
fn the_address_alone_is_the_second_address_line() {
    assert_eq!(
        text(al_testkit::run_in(HEADER_FIXTURE, "SE Header Tests", "AddressOnly")),
        "<CompanyAddressLine1>Co</CompanyAddressLine1><CompanyAddressLine2>Street 1</CompanyAddressLine2>\
         <CompanyAddressLine3></CompanyAddressLine3><CompanyAddressLine4></CompanyAddressLine4>\
         <CompanyAddressLine5></CompanyAddressLine5><CompanyAddressLine6></CompanyAddressLine6>\
         <CompanyAddressLine7></CompanyAddressLine7><CompanyAddressLine8></CompanyAddressLine8>"
    );
}

/// A SubstituteReport subscriber that passes RequestPageXml on to an event of its own (Base Application's ReportManagement
/// does, OnAfterSubstituteReport) runs while nothing reads it (AMRdlcOutput.al:TextRdlc_SaveAsHtml_IsRefused).
#[test]
fn an_html_saveas_whose_subscriber_passes_requestpagexml_on_runs() {
    let fixture = FIXTURE
        .replace("L.Add('req=' + RequestPageXml);", "L.Add('req'); OnForward(RequestPageXml); Pass(RequestPageXml);")
        .replace(
            "    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Reporting Triggers\", 'OnDocumentReady'",
            "    [IntegrationEvent(false, false)]\n    local procedure OnForward(RequestPageXml: Text) begin end;\n    local procedure Pass(Xml: Text) begin end;\n    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"SE Sub\", 'OnForward', '', false, false)]\n    local procedure Forwarded(RequestPageXml: Text) var L: Codeunit \"SE Log\"; begin L.Add('fwd'); end;\n\n    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Reporting Triggers\", 'OnDocumentReady'",
        );
    assert_eq!(
        text(al_testkit::run_in(&fixture, "SE Tests", "Html")),
        "No|The Html output format is not supported by the current report layout format.|SubstituteReport(50972,SaveAs,in=-1);req;fwd;"
    );
}
