//! `Report.SaveAs(.., ReportFormat::Xml, ..)`, end to end: the dataset a report run writes. Each
//! test names the corpus test that pins it. The fixtures copy the probe in
//! handlers/TestReportSaveAsDatasetContracts.al (tables 68300/68301, report 68302).

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_version::{BcVersion, Target};
use al_vm::{Engine, Session};

const FIXTURES: &str = r#"
table 68300 "Probe Rep Parent"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Amt; Decimal) { }
        field(4; D; Date) { }
        field(5; B; Boolean) { }
        field(6; Kids; Integer) { FieldClass = FlowField; CalcFormula = count("Probe Rep Child" where(ParentId = field(Id))); }
        field(7; Kind; Option) { OptionMembers = Alpha,Beta,Gamma; }
        field(8; At; Time) { }
        field(10; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 68301 "Probe Rep Child"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; ParentId; Integer) { }
        field(3; Code; Code[10]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

report 68302 "Probe SaveAs Report"
{
    UsageCategory = None;
    DefaultRenderingLayout = Rdlc;

    dataset
    {
        dataitem(P; "Probe Rep Parent")
        {
            column(PNo; Id) { }
            column(PName; Name) { }
            column(PAmt; Amt) { }
            column(PDate; D) { }
            column(PBool; B) { }

            dataitem(C; "Probe Rep Child")
            {
                DataItemLink = ParentId = field(Id);

                column(CNo; Id) { }
                column(CCode; Code) { }
            }
        }
    }

    requestpage
    {
        layout
        {
            area(Content)
            {
                group(Options)
                {
                    field(Prefix; Prefix) { ApplicationArea = All; }
                }
            }
        }
    }

    rendering
    {
        layout(Rdlc)
        {
            Type = RDLC;
            LayoutFile = './ProbeSaveAsDatasetLayout.rdl';
            Caption = 'Probe layout';
        }
    }

    var
        Prefix: Text;
}

// corpus reportextensiontrigger/TestReportExtensionDataItemPlacementContracts.al (68810..68815)
table 68810 "Probe Place Parent" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
table 68811 "Probe Place Child" { fields { field(1; Id; Integer) { } field(2; ParentId; Integer) { } } keys { key(PK; Id) { } } }
codeunit 68814 "Probe Place Log"
{
    SingleInstance = true;
    var Log: Text;
    procedure Add(Entry: Text) begin Log += Entry + ';'; end;
    procedure Take(): Text var Result: Text; begin Result := Log; Log := ''; exit(Result); end;
}
report 68812 "Probe Place Base"
{
    UsageCategory = None;
    DefaultRenderingLayout = Rdlc;
    dataset
    {
        dataitem(Parent; "Probe Place Parent")
        {
            dataitem(ChildA; "Probe Place Child")
            {
                DataItemLink = ParentId = field(Id);
                column(AId; Id) { }
                trigger OnAfterGetRecord() begin PlaceLog.Add('ChildA' + Format(Id)); end;
            }
            dataitem(ChildB; "Probe Place Child")
            {
                DataItemLink = ParentId = field(Id);
                column(BId; Id) { }
                trigger OnAfterGetRecord() begin PlaceLog.Add('ChildB' + Format(Id)); end;
            }
            column(PId; Id) { }
            trigger OnAfterGetRecord() begin PlaceLog.Add('Parent' + Format(Id)); end;
        }
    }
    rendering { layout(Rdlc) { Type = RDLC; LayoutFile = './ProbePlaceLayout.rdl'; Caption = 'Probe layout'; } }
    var PlaceLog: Codeunit "Probe Place Log";
}
reportextension 68813 "Probe Place Ext" extends "Probe Place Base"
{
    dataset
    {
        addfirst(Parent) { dataitem(ExtFirst; "Probe Place Child") { DataItemLink = ParentId = field(Id); column(EFId; Id) { }
            trigger OnAfterGetRecord() begin PlaceLog.Add('ExtFirst' + Format(Id)); end; } }
        addafter(ChildA) { dataitem(ExtAfterA; "Probe Place Child") { DataItemLink = ParentId = field(Id); column(EAId; Id) { }
            trigger OnAfterGetRecord() begin PlaceLog.Add('ExtAfterA' + Format(Id)); end; } }
        addbefore(ChildB) { dataitem(ExtBeforeB; "Probe Place Child") { DataItemLink = ParentId = field(Id); column(EBId; Id) { }
            trigger OnAfterGetRecord() begin PlaceLog.Add('ExtBeforeB' + Format(Id)); end; } }
        addlast(Parent) { dataitem(ExtLast; "Probe Place Child") { DataItemLink = ParentId = field(Id); column(ELId; Id) { }
            trigger OnAfterGetRecord() begin PlaceLog.Add('ExtLast' + Format(Id)); end; } }
        addbefore(ChildA) { dataitem(ExtBeforeA; "Probe Place Child") { DataItemLink = ParentId = field(Id); column(EBAId; Id) { }
            trigger OnAfterGetRecord() begin PlaceLog.Add('ExtBeforeA' + Format(Id)); end; } }
    }
    var PlaceLog: Codeunit "Probe Place Log";
}

report 68399 "Probe Processing Only"
{
    UsageCategory = None;
    ProcessingOnly = true;
    dataset { dataitem(P; "Probe Rep Parent") { column(PNo; Id) { } } }
}

// A report without a layout, whose columns are expressions and whose triggers log.
report 68310 "Probe Plain Report"
{
    ProcessingOnly = false;

    dataset
    {
        dataitem(P; "Probe Rep Parent")
        {
            column(Tag; 'T-' + Format(Id) + '-' + Format(Seen)) { }
            column(Tens; Seen * 10) { }

            trigger OnAfterGetRecord()
            begin
                Seen += 1;
            end;
        }
    }

    trigger OnPreReport()
    begin
        if FailPre then
            Error('plain pre failed');
    end;

    var
        Seen: Integer;
        FailPre: Boolean;

    procedure RowsSeen(): Integer
    begin
        exit(Seen);
    end;

    procedure FailInPre()
    begin
        FailPre := true;
    end;
}

// A report with an RDLC layout whose OnPreReport fails (corpus handlers/TestReportSaveAsRecordRefFilterReport.al).
report 68311 "Probe Layout Guard Report"
{
    DefaultRenderingLayout = Rdlc;

    dataset
    {
        dataitem(P; "Probe Rep Parent")
        {
            column(PNo; Id) { }
        }
    }

    rendering
    {
        layout(Rdlc)
        {
            Type = RDLC;
            LayoutFile = './x.rdl';
        }
    }

    trigger OnPreReport()
    begin
        if P.GetFilters() = '' then
            Error('You must specify one or more filters.');
    end;
}

// A column bound to a FlowField, and one the trigger fills from the same FlowField without CalcFields.
report 68312 "Probe Flow Report"
{
    ProcessingOnly = false;

    dataset
    {
        dataitem(P; "Probe Rep Parent")
        {
            column(Kids; Kids) { }
            column(SeenKids; SeenKids) { }

            trigger OnAfterGetRecord()
            begin
                SeenKids := Kids;
            end;
        }
    }

    var
        SeenKids: Integer;
}

// Columns of the types the probe does not pin, and a property no test pins.
report 68313 "Probe Types Report"
{
    ProcessingOnly = false;

    dataset
    {
        dataitem(P; "Probe Rep Parent")
        {
            column(KindCol; Kind) { }
            column(DecExpr; Amt * 2) { AutoFormatType = 1; }
        }
    }
}

// report/lt-y-report-props/YReports.al:111 (a report with the FormatRegion property).
report 68391 "Probe Region Fr"
{
    ProcessingOnly = false;
    FormatRegion = 'fr-FR';

    dataset
    {
        dataitem(Row; "Probe Rep Parent")
        {
            DataItemTableView = where(Id = const(1));
            column(AmtCol; Money) { }
            column(DayCol; DayVal) { }
            column(StampCol; Stamp) { }
            column(AmtText; AmtText) { }
            column(StampText; StampText) { }
            column(LongDayText; LongDayText) { }

            trigger OnAfterGetRecord()
            begin
                AmtText := Format(Money);
                StampText := Format(Stamp);
                LongDayText := Format(DayVal, 0, 4);
            end;
        }
    }

    trigger OnInitReport()
    begin
        Money := 1234567.891;
        DayVal := DMY2Date(31, 1, 2025);
        Stamp := CreateDateTime(DayVal, 143015T);
    end;

    var
        Money: Decimal;
        DayVal: Date;
        Stamp: DateTime;
        AmtText: Text;
        StampText: Text;
        LongDayText: Text;
}

// report/lt-y-report-props/YReports.al:111 (a report with the FormatRegion property).
report 68392 "Probe Region Jp"
{
    ProcessingOnly = false;
    FormatRegion = 'ja-JP';

    dataset
    {
        dataitem(Row; "Probe Rep Parent")
        {
            DataItemTableView = where(Id = const(1));
            column(AmtCol; Money) { }
            column(DayCol; DayVal) { }
            column(StampCol; Stamp) { }
            column(AmtText; AmtText) { }
            column(StampText; StampText) { }
            column(LongDayText; LongDayText) { }

            trigger OnAfterGetRecord()
            begin
                AmtText := Format(Money);
                StampText := Format(Stamp);
                LongDayText := Format(DayVal, 0, 4);
            end;
        }
    }

    trigger OnInitReport()
    begin
        Money := 1234567.891;
        DayVal := DMY2Date(31, 1, 2025);
        Stamp := CreateDateTime(DayVal, 143015T);
    end;

    var
        Money: Decimal;
        DayVal: Date;
        Stamp: DateTime;
        AmtText: Text;
        StampText: Text;
        LongDayText: Text;
}

// report/lt-y-report-props/YReports.al:111 (a report with the FormatRegion property).
report 68393 "Probe Region Nl"
{
    ProcessingOnly = false;
    FormatRegion = 'nl-NL';

    dataset
    {
        dataitem(Row; "Probe Rep Parent")
        {
            DataItemTableView = where(Id = const(1));
            column(AmtCol; Money) { }
            column(DayCol; DayVal) { }
            column(StampCol; Stamp) { }
            column(AmtText; AmtText) { }
            column(StampText; StampText) { }
            column(LongDayText; LongDayText) { }

            trigger OnAfterGetRecord()
            begin
                AmtText := Format(Money);
                StampText := Format(Stamp);
                LongDayText := Format(DayVal, 0, 4);
            end;
        }
    }

    trigger OnInitReport()
    begin
        Money := 1234567.891;
        DayVal := DMY2Date(31, 1, 2025);
        Stamp := CreateDateTime(DayVal, 143015T);
    end;

    var
        Money: Decimal;
        DayVal: Date;
        Stamp: DateTime;
        AmtText: Text;
        StampText: Text;
        LongDayText: Text;
}

// A report with the FormatRegion property whose OnInitReport formats a Decimal: which culture Format uses there is not pinned
// (report/lt-y-report-props/YReports.al sets values in OnInitReport and formats them only in OnAfterGetRecord).
report 68394 "Probe Region Init"
{
    ProcessingOnly = false;
    FormatRegion = 'de-DE';

    dataset
    {
        dataitem(Row; "Probe Rep Parent")
        {
            DataItemTableView = where(Id = const(1));
            column(InitText; InitText) { }
        }
    }

    trigger OnInitReport()
    begin
        InitText := Format(1234.5);
    end;

    var
        InitText: Text;
}

report 68314 "Probe Time Report"
{
    ProcessingOnly = false;

    dataset
    {
        dataitem(P; "Probe Rep Parent")
        {
            column(AtCol; At) { }
        }
    }
}

report 68315 "Probe Caption Report"
{
    ProcessingOnly = false;

    dataset
    {
        dataitem(P; "Probe Rep Parent")
        {
            column(WithCaption; Id) { IncludeCaption = true; }
        }
    }
}

// IncludeCaption on a field written with a data item name: the own item named like its table, a child's field in the parent, the
// parent's in the child, an earlier sibling's (errors/lt-ms-refusals-138/P138ColCaptionQualified.al:113, report 87352).
report 68398 "Probe Caption Sources"
{
    ProcessingOnly = false;

    dataset
    {
        dataitem("Probe Rep Parent"; "Probe Rep Parent")
        {
            column(IdOwn; "Probe Rep Parent".Id) { IncludeCaption = true; }
            column(FromChild; Child.Code) { IncludeCaption = true; }
            dataitem(Child; "Probe Rep Child")
            {
                DataItemLink = ParentId = field(Id);
                column(FromParent; "Probe Rep Parent".Name) { IncludeCaption = true; }
            }
        }
        dataitem(Sibling; "Probe Rep Child")
        {
            column(FromSibling; "Probe Rep Parent".Amt) { IncludeCaption = true; }
        }
    }
}

// Skip leaves a row out of the dataset.
report 68316 "Probe Skip Report"
{
    ProcessingOnly = false;

    dataset
    {
        dataitem(P; "Probe Rep Parent")
        {
            column(PNo; Id) { }

            trigger OnAfterGetRecord()
            begin
                if Id = 1 then
                    CurrReport.Skip();
            end;
        }
    }
}

// A data item whose name has spaces and parentheses.
report 68317 "Probe Quoted Report"
{
    ProcessingOnly = false;

    dataset
    {
        dataitem("Probe Item (Parent)"; "Probe Rep Parent")
        {
            column("Probe Col"; Id) { }
        }
    }
}

// A reportextension that adds a column to a base item and fails on request.
reportextension 68318 "Probe Plain Ext" extends "Probe Plain Report"
{
    dataset
    {
        add(P)
        {
            column(ExtCol; 'E-' + Format(Id)) { }
        }
    }

    trigger OnPreReport()
    begin
    end;
}

// A processing-only report: SaveAs of it is not pinned.
report 68320 "Probe Processing Report"
{
    ProcessingOnly = true;

    dataset
    {
        dataitem(P; "Probe Rep Parent")
        {
            column(PNo; Id) { }
        }
    }
}

// A data item over a temporary record: it holds only what code inserts, not the table's rows.
report 68319 "Probe Temporary Report"
{
    ProcessingOnly = false;

    dataset
    {
        dataitem(T; "Probe Rep Child")
        {
            UseTemporary = true;
            column(TNo; Id) { }
        }
    }
}

codeunit 68304 Helpers
{
    procedure Seed()
    var
        Parent: Record "Probe Rep Parent";
        Child: Record "Probe Rep Child";
    begin
        Child.DeleteAll();
        Parent.DeleteAll();
        Parent.Init();
        Parent.Id := 1;
        Parent.Name := 'one <&>';
        Parent.Amt := 12.5;
        Parent.D := DMY2Date(5, 3, 2024);
        Parent.B := true;
        Parent.Insert();
        Parent.Init();
        Parent.Id := 2;
        Parent.Name := 'two';
        Parent.Insert();
        Child.Init();
        Child.Id := 10;
        Child.ParentId := 1;
        Child.Code := 'a';
        Child.Insert();
        Child.Id := 11;
        Child.Insert();
        Child.Id := 12;
        Child.ParentId := 2;
        Child.Code := 'c';
        Child.Insert();
        Commit();
    end;

    // The lines of the stream joined with '|'.
    procedure Lines(var Buf: Record "Probe Rep Parent"): Text
    var
        InStr: InStream;
        Line: Text;
        Result: Text;
    begin
        Buf.CalcFields(Data);
        Buf.Data.CreateInStream(InStr, TextEncoding::UTF8);
        while not InStr.EOS() do begin
            InStr.ReadText(Line);
            Result += Line + '|';
        end;
        exit(Result);
    end;

    procedure BomBytes(var Buf: Record "Probe Rep Parent"): Text
    var
        InStr: InStream;
        A: Byte;
        B: Byte;
        C: Byte;
        N: Integer;
        Out: Text;
    begin
        Buf.CalcFields(Data);
        Buf.Data.CreateInStream(InStr);
        InStr.Read(A);
        InStr.Read(B);
        InStr.Read(C);
        // A Byte formats as the character of its code (streams/TestProbeStreamContracts.al:Read_IntoTypes).
        N := A;
        Out := Format(N);
        N := B;
        Out += '.' + Format(N);
        N := C;
        exit(Out + '.' + Format(N));
    end;

    // The dataset the probe report writes, from its DataItems element on, with the tableId/systemId pairs kept.
    procedure Dataset(ReportId: Integer; Parameters: Text): Text
    var
        Buf: Record "Probe Rep Parent";
        OutStr: OutStream;
        Saved: Boolean;
        All: Text;
        At: Integer;
    begin
        Buf.Data.CreateOutStream(OutStr);
        Saved := Report.SaveAs(ReportId, Parameters, ReportFormat::Xml, OutStr);
        if not Saved then
            exit('FALSE:' + GetLastErrorText());
        All := Lines(Buf);
        exit(All);
    end;
}
"#;

fn run_on(bc: Option<&str>, body: &str) -> Result<Value, Signal> {
    let src = format!("{FIXTURES}\ncodeunit 50100 C {{ procedure F(): Text {body} }}");
    let target = bc.map(|v| Target::for_bc(BcVersion::parse(v).unwrap()));
    let apps = vec![AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), src)],
    }];
    let (ws, l) = match &target {
        Some(t) => al_sema::lower::compile_for(apps, &Default::default(), t),
        None => al_sema::lower::compile(apps, &Default::default()),
    };
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "F")
        .unwrap();
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.get(ri)) {
        return Err(Signal::Unsupported(r.clone()));
    }
    let engine = Engine::with_target(l.program, blocked, target);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn run(body: &str) -> Result<Value, Signal> {
    run_on(None, body)
}

fn f(body: &str) -> String {
    match run(body) {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

fn refusal(body: &str) -> String {
    match run(body) {
        Err(Signal::Unsupported(r)) => r,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Lines given as (indent in spaces, text), joined the way the AL helper joins a stream.
fn lines(ls: &[(usize, &str)]) -> String {
    ls.iter().map(|(n, t)| format!("{}{t}|", " ".repeat(*n))).collect()
}

// handlers/TestReportSaveAsDatasetContracts.al:SaveAs_Xml_DataItemsBlock_ExactText. The text is the one of BC 27.5,
// which writes no identity pair on a DataItem.
#[test]
fn the_data_items_block_is_written_exactly() {
    let text = match run_on(Some("27.5"), DATASET) {
        Ok(Value::Text(t)) => t,
        other => panic!("{other:?}"),
    };
    let at = text.find("  <DataItems>|").expect("a DataItems block");
    let top = "<DataItem language=\"en-US\" formatRegion=\"en-US\" name=\"P\">";
    let expected = lines(&[
        (2, "<DataItems>"),
        (4, top),
        (6, "<Columns>"),
        (8, "<Column name=\"PNo\">1</Column>"),
        (8, "<Column name=\"PName\">one &lt;&amp;&gt;</Column>"),
        (8, "<Column name=\"PAmt\" decimalformatter=\"#,##0.00\">12.5</Column>"),
        (8, "<Column name=\"PDate\">3/5/2024</Column>"),
        (8, "<Column name=\"PBool\">True</Column>"),
        (6, "</Columns>"),
        (6, "<DataItems>"),
        (8, "<DataItem name=\"C\">"),
        (10, "<Columns>"),
        (12, "<Column name=\"CNo\">10</Column>"),
        (12, "<Column name=\"CCode\">A</Column>"),
        (10, "</Columns>"),
        (8, "</DataItem>"),
        (8, "<DataItem name=\"C\">"),
        (10, "<Columns>"),
        (12, "<Column name=\"CNo\">11</Column>"),
        (12, "<Column name=\"CCode\">A</Column>"),
        (10, "</Columns>"),
        (8, "</DataItem>"),
        (6, "</DataItems>"),
        (4, "</DataItem>"),
        (4, top),
        (6, "<Columns>"),
        (8, "<Column name=\"PNo\">2</Column>"),
        (8, "<Column name=\"PName\">two</Column>"),
        (8, "<Column name=\"PAmt\" decimalformatter=\"#,##0.00\">0</Column>"),
        (8, "<Column name=\"PDate\"></Column>"),
        (8, "<Column name=\"PBool\">False</Column>"),
        (6, "</Columns>"),
        (6, "<DataItems>"),
        (8, "<DataItem name=\"C\">"),
        (10, "<Columns>"),
        (12, "<Column name=\"CNo\">12</Column>"),
        (12, "<Column name=\"CCode\">C</Column>"),
        (10, "</Columns>"),
        (8, "</DataItem>"),
        (6, "</DataItems>"),
        (4, "</DataItem>"),
        (2, "</DataItems>"),
        (0, "</ReportDataSet>"),
    ]);
    assert_eq!(&text[at..], expected);
}

const DATASET: &str =
    "var H: Codeunit Helpers; begin H.Seed(); exit(H.Dataset(Report::\"Probe SaveAs Report\", '')); end;";

// handlers/TestReportSaveAsDatasetContracts.al:SaveAs_Xml_WritesAByteOrderMarkDeclarationAndHeader (:206-212)
#[test]
fn the_stream_starts_with_a_byte_order_mark_a_declaration_and_the_root_element() {
    assert_eq!(
        f(
            "var H: Codeunit Helpers; Buf: Record \"Probe Rep Parent\"; Out: OutStream; begin
            H.Seed();
            Buf.Data.CreateOutStream(Out);
            Report.SaveAs(Report::\"Probe SaveAs Report\", '', ReportFormat::Xml, Out);
            exit(H.BomBytes(Buf));
        end;"
        ),
        "239.187.191"
    );
    let text = f(DATASET);
    let mut lines = text.split('|');
    assert_eq!(
        lines.next().unwrap().trim_start_matches('\u{feff}'),
        "<?xml version=\"1.0\" standalone=\"yes\"?>"
    );
    assert_eq!(
        lines.next().unwrap(),
        "<ReportDataSet name=\"Probe SaveAs Report\" id=\"68302\" language=\"en-US\" formatRegion=\"en-US\">"
    );
}

/// The names of the header elements that carry a value on one line, in order.
fn header_tags(text: &str) -> String {
    let mut tags = String::new();
    for line in text.split('|') {
        let t = line.trim();
        if t == "<DataItems>" {
            break;
        }
        if t.starts_with('<') && !t.starts_with("</") && t.contains("</") {
            tags.push_str(&t[1..t.find('>').unwrap()]);
            tags.push(' ');
        }
    }
    tags
}

// handlers/TestReportSaveAsDatasetContracts.al:SaveAs_Xml_WritesAByteOrderMarkDeclarationAndHeader (:213-220):
// BC 27 and 28 write one list, BC 29 adds ReportCaption and the company information block.
#[test]
fn the_header_lists_its_elements_in_the_order_the_probe_pins() {
    let before = "ExtensionId ExtensionName ExtensionPublisher ExtensionVersion ReportId ReportName AboutThisReportTitle AboutThisReportText ReportHelpLink TenantEntraId EnvironmentName EnvironmentType CompanyName CompanyDisplayName CompanyId UserName DateAndTime Language FormatRegion Year MonthNumber DayNumber Hour Minute ";
    let after = "ExtensionId ExtensionName ExtensionPublisher ExtensionVersion ReportId ReportName ReportCaption AboutThisReportTitle AboutThisReportText ReportHelpLink TenantEntraId EnvironmentName EnvironmentType CompanyName CompanyDisplayName CompanyId UserName DateAndTime Language FormatRegion Year MonthNumber DayNumber Hour Minute CompanyName CompanyDisplayName CompanyAddressLine1 CompanyAddressLine2 CompanyAddressLine3 CompanyAddressLine4 CompanyAddressLine5 CompanyAddressLine6 CompanyAddressLine7 CompanyAddressLine8 CompanyPhone CompanyPhoneCaption CompanyFaxNo CompanyFaxNoCaption CompanyEmail CompanyEmailCaption CompanyHomePage CompanyHomePageCaption CompanyLogo CompanyVATRegistrationNo CompanyVATRegistrationNoCaption CompanyRegistrationNo CompanyRegistrationNoCaption CompanyBankName CompanyBankNameCaption CompanyBankAccountNo CompanyBankAccountNoCaption CompanyBankBranchNo CompanyBankBranchNoCaption CompanyIBAN CompanyIBANCaption CompanyBankSWIFT CompanyBankSWIFTCaption CompanyGiroNo CompanyGiroNoCaption ";
    for bc in ["27.5", "28.5"] {
        let text = match run_on(Some(bc), DATASET) {
            Ok(Value::Text(t)) => t,
            other => panic!("{other:?}"),
        };
        assert_eq!(header_tags(&text), before, "BC {bc}");
    }
    for bc in [Some("29.0"), None] {
        let text = match run_on(bc, DATASET) {
            Ok(Value::Text(t)) => t,
            other => panic!("{other:?}"),
        };
        assert_eq!(header_tags(&text), after, "BC {bc:?}");
    }
}

// The parents of the header elements: misc/lt-probes-needed-2/PnBReportSaveAs.al:526-575 and report/W3ReportDataset.al:510-590
// (BC 29). `DateTimeValues` holds the clock parts on BC 27 and 28 too: report/W3ReportDataset.al:256 counts 46 line ends
// there, two more than a flat list gives.
#[test]
fn the_header_groups_the_clock_parts_and_the_company_metadata() {
    for bc in ["27.5", "28.5", "29.0"] {
        let text = match run_on(Some(bc), DATASET) {
            Ok(Value::Text(t)) => t,
            other => panic!("{other:?}"),
        };
        let at = text.find("      <DateTimeValues>|").expect("a DateTimeValues element");
        assert!(
            text[at..].starts_with("      <DateTimeValues>|        <Year>"),
            "BC {bc}: {text}"
        );
        assert!(
            text.contains("        <Minute>")
                && text.contains("</Minute>|      </DateTimeValues>|    </ReportRequest>|"),
            "BC {bc}: {text}"
        );
        if bc == "29.0" {
            assert!(
                text.contains("    </ReportRequest>|    <CompanyMetadata>|      <CompanyName>"),
                "{text}"
            );
            assert!(
                text.contains("</CompanyDisplayName>|      <CompanyAddressLines>|        <CompanyAddressLine1>"),
                "{text}"
            );
            assert!(
                text.contains(
                    "        <CompanyAddressLine8></CompanyAddressLine8>|      </CompanyAddressLines>|      <CompanyPhone></CompanyPhone>|      <CompanyPhoneCaption>Phone No.</CompanyPhoneCaption>|"
                ),
                "{text}"
            );
            assert!(
                text.contains("      <CompanyGiroNoCaption>Giro No.</CompanyGiroNoCaption>|    </CompanyMetadata>|  </BCReportInformation>|"),
                "{text}"
            );
        } else {
            assert!(
                text.contains("    </ReportRequest>|  </BCReportInformation>|"),
                "BC {bc}: {text}"
            );
        }
    }
}

// handlers/TestReportSaveAsDatasetContracts.al:SaveAs_Xml_WritesAByteOrderMarkDeclarationAndHeader (:221-222)
#[test]
fn the_header_names_the_report_and_the_company() {
    let text = f(DATASET);
    assert!(
        text.contains("      <ReportName>Probe SaveAs Report</ReportName>|"),
        "{text}"
    );
    assert!(text.contains("<ReportId>68302</ReportId>|"), "{text}");
    let company = f("begin exit(CompanyName()); end;");
    assert!(
        text.contains(&format!("<CompanyName>{company}</CompanyName>|")),
        "{text}"
    );
}

const FILTERED: &str = "<?xml version=\"1.0\" standalone=\"yes\"?><ReportParameters name=\"Probe SaveAs Report\" id=\"68302\"><Options><Field name=\"Prefix\">PFX</Field></Options><DataItems><DataItem name=\"P\">VERSION(1) SORTING(Field1) WHERE(Field1=1(2))</DataItem><DataItem name=\"C\">VERSION(1) SORTING(Field1)</DataItem></DataItems></ReportParameters>";

// handlers/TestReportSaveAsDatasetContracts.al:RunRequestPage_ReturnsTheParametersString_AndSaveAsHonoursItsFilter (:370-378)
#[test]
fn the_parameters_string_filters_the_dataset() {
    let text = f(&format!(
        "var H: Codeunit Helpers; begin H.Seed(); exit(H.Dataset(Report::\"Probe SaveAs Report\", '{FILTERED}')); end;"
    ));
    assert!(text.contains("<Column name=\"PNo\">2</Column>"), "{text}");
    assert!(!text.contains("<Column name=\"PNo\">1</Column>"), "{text}");
    assert!(text.contains("<Column name=\"CNo\">12</Column>"), "{text}");
    assert!(!text.contains("<Column name=\"CNo\">10</Column>"), "{text}");
}

// handlers/TestReportSaveAsDatasetContracts.al:SaveAs_Html_WithAnRdlcLayout_IsRefused
#[test]
fn html_of_a_report_with_an_rdlc_layout_answers_false_with_the_layout_error_and_writes_nothing() {
    assert_eq!(
        f(
            "var H: Codeunit Helpers; Buf: Record \"Probe Rep Parent\"; Out: OutStream; Ok: Boolean; begin
            H.Seed();
            Buf.Data.CreateOutStream(Out);
            ClearLastError();
            Ok := Report.SaveAs(Report::\"Probe SaveAs Report\", '', ReportFormat::Html, Out);
            exit(Format(Ok) + '|' + GetLastErrorText() + '|' + H.Lines(Buf));
        end;"
        ),
        "No|The Html output format is not supported by the current report layout format.|"
    );
}

#[test]
fn a_rendered_format_is_refused_not_guessed() {
    for format in ["Pdf", "Word", "Excel"] {
        let why = refusal(&format!(
            "var H: Codeunit Helpers; Buf: Record \"Probe Rep Parent\"; Out: OutStream; begin
                H.Seed();
                Buf.Data.CreateOutStream(Out);
                Report.SaveAs(Report::\"Probe SaveAs Report\", '', ReportFormat::{format}, Out);
            end;"
        ));
        assert!(why.contains(&format!("as {format}")), "{why}");
    }
}

// handlers/TestReportRunExecution.al:InstanceSaveAsXml_ExecutesTriggersAndBody; the columns read what
// OnAfterGetRecord left (handlers/TestReportFlowFieldColumnAutoCalc.al:FlowFieldColumn_IsCalculatedBeforeOnAfterGetRecord
// reads a column bound to a variable the trigger sets).
#[test]
fn an_instance_saveas_runs_the_triggers_and_the_columns_see_their_result() {
    assert_eq!(
        f("var H: Codeunit Helpers; R: Report \"Probe Plain Report\"; Buf: Record \"Probe Rep Parent\"; Out: OutStream; begin
            H.Seed();
            Clear(R);
            Buf.Data.CreateOutStream(Out);
            R.SaveAs('', ReportFormat::Xml, Out);
            exit(Format(R.RowsSeen()) + '|' + H.Lines(Buf));
        end;")
        .split('|')
        .filter(|l| l.contains("<Column ") || l.len() == 1)
        .map(|l| l.trim().to_string())
        .collect::<Vec<_>>()
        .join(","),
        "2,<Column name=\"Tag\">T-1-1</Column>,<Column name=\"Tens\">10</Column>,<Column name=\"ExtCol\">E-1</Column>,<Column name=\"Tag\">T-2-2</Column>,<Column name=\"Tens\">20</Column>,<Column name=\"ExtCol\">E-2</Column>"
    );
}

// reportextension/TestReportExtensionTriggers.al:SaveAsXml_ExtPreReportError_FailsTheSave: a report with no
// layout raises the error of its trigger.
#[test]
fn an_error_in_a_report_without_a_layout_reaches_the_caller() {
    match run("var H: Codeunit Helpers; R: Report \"Probe Plain Report\"; Buf: Record \"Probe Rep Parent\"; Out: OutStream; begin
            H.Seed();
            Buf.Data.CreateOutStream(Out);
            R.FailInPre();
            R.SaveAs('', ReportFormat::Xml, Out);
        end;")
    {
        Err(Signal::Error(e)) => assert_eq!(e.message, "plain pre failed"),
        other => panic!("{other:?}"),
    }
}

// handlers/TestReportSaveAsRecordRefFilter.al:TestReport_SaveAs_NoRecordRef_LeavesDataItemUnfiltered: the
// guard in OnPreReport of a report with an RDLC layout fails the SaveAs, which answers false.
#[test]
fn an_error_in_a_report_with_an_rdlc_layout_answers_false() {
    assert_eq!(
        f(
            "var H: Codeunit Helpers; Buf: Record \"Probe Rep Parent\"; Out: OutStream; Ok: Boolean; begin
            H.Seed();
            Buf.Data.CreateOutStream(Out);
            Ok := Report.SaveAs(Report::\"Probe Layout Guard Report\", '', ReportFormat::Xml, Out);
            exit(Format(Ok) + '|' + H.Lines(Buf));
        end;"
        ),
        "No|"
    );
}

// handlers/TestReportSaveAsRecordRefFilter.al:TestReport_SaveAs_FilteredRecordRef_LimitsDatasetToThatRecord
#[test]
fn a_filtered_recordref_limits_the_data_item_over_its_table() {
    let text = f("var H: Codeunit Helpers; Buf: Record \"Probe Rep Parent\"; Parent: Record \"Probe Rep Parent\"; RR: RecordRef; Out: OutStream; Ok: Boolean; begin
            H.Seed();
            Parent.Get(2);
            Parent.SetRecFilter();
            RR.GetTable(Parent);
            Buf.Data.CreateOutStream(Out);
            Ok := Report.SaveAs(Report::\"Probe Layout Guard Report\", '', ReportFormat::Xml, Out, RR);
            exit(Format(Ok) + '|' + H.Lines(Buf));
        end;");
    assert!(text.starts_with("Yes|"), "{text}");
    assert!(text.contains("<Column name=\"PNo\">2</Column>"), "{text}");
    assert!(!text.contains("<Column name=\"PNo\">1</Column>"), "{text}");
}

fn dataset_of(report: &str, parameters: &str) -> String {
    f(&format!(
        "var H: Codeunit Helpers; begin H.Seed(); exit(H.Dataset(Report::\"{report}\", '{parameters}')); end;"
    ))
}

// handlers/TestReportFlowFieldColumnAutoCalc.al:FlowFieldColumn_IsCalculatedWithoutCalcFields,
// FlowFieldColumn_IsCalculatedBeforeOnAfterGetRecord (the trigger reads the calculated FlowField)
#[test]
fn a_flowfield_column_is_calculated_for_every_row_before_the_trigger() {
    let text = dataset_of("Probe Flow Report", "");
    // Parent 1 has two children, parent 2 one.
    let cells: Vec<&str> = text
        .split('|')
        .map(str::trim)
        .filter(|l| l.starts_with("<Column "))
        .collect();
    assert_eq!(
        cells,
        [
            "<Column name=\"Kids\">2</Column>",
            "<Column name=\"SeenKids\">2</Column>",
            "<Column name=\"Kids\">1</Column>",
            "<Column name=\"SeenKids\">1</Column>",
        ]
    );
}

// reportextension/TestReportExtensionTriggers.al:SaveAsXml_ExtensionRowsAndColumns_ReachTheDataset
#[test]
fn a_column_an_extension_adds_is_written_after_the_columns_of_the_item() {
    let text = dataset_of("Probe Plain Report", "");
    let cells: Vec<&str> = text
        .split('|')
        .map(str::trim)
        .filter(|l| l.starts_with("<Column "))
        .collect();
    assert_eq!(
        cells,
        [
            "<Column name=\"Tag\">T-1-1</Column>",
            "<Column name=\"Tens\">10</Column>",
            "<Column name=\"ExtCol\">E-1</Column>",
            "<Column name=\"Tag\">T-2-2</Column>",
            "<Column name=\"Tens\">20</Column>",
            "<Column name=\"ExtCol\">E-2</Column>",
        ]
    );
}

// handlers/TestReportSaveAsDatasetContracts.al:SaveAs_Xml_DataItemsBlock_ExactText: Skip in
// OnAfterGetRecord leaves the row and its nested items out (handlers/TestReportRuntimeContracts.al:322-368).
#[test]
fn a_skipped_row_is_not_written() {
    let text = dataset_of("Probe Skip Report", "");
    assert!(!text.contains("<Column name=\"PNo\">1</Column>"), "{text}");
    assert!(text.contains("<Column name=\"PNo\">2</Column>"), "{text}");
}

// handlers/TestReportSaveAsColumnTypesContracts.al:174-215: an Option is written as its caption (its
// name when the field declares none), a Decimal expression keeps its scale (`12.5 * 2` is `25.0`, as
// `12.345678 * 1.5` is `18.5185170`), and with no DecimalPlaces the formatter is the default.
#[test]
fn an_option_is_written_as_its_caption_and_a_decimal_expression_keeps_its_scale() {
    let text = dataset_of("Probe Types Report", "");
    assert!(text.contains("<Column name=\"KindCol\">Alpha</Column>"), "{text}");
    assert!(
        text.contains("<Column name=\"DecExpr\" decimalformatter=\"#,##0.00\">25.0</Column>"),
        "{text}"
    );
}

// handlers/TestReportSaveAsColumnTypesContracts.al:178-182: a Time is `h:mm:ss`, a plain space, AM or PM; an
// undefined one is blank (not pinned, docs/probes-needed.md).
#[test]
fn a_time_column_is_written_in_the_en_us_form_with_a_plain_space() {
    let text = f(
        "var H: Codeunit Helpers; Buf: Record \"Probe Rep Parent\"; P: Record \"Probe Rep Parent\"; Out: OutStream; begin H.Seed(); P.Get(1); P.At := 123456T; P.Modify(); Buf.Data.CreateOutStream(Out); Report.SaveAs(Report::\"Probe Time Report\", '', ReportFormat::Xml, Out); exit(H.Lines(Buf)); end;",
    );
    assert!(text.contains("<Column name=\"AtCol\">12:34:56 PM</Column>"), "{text}");
    assert!(text.contains("<Column name=\"AtCol\"></Column>"), "{text}");
}

// handlers/TestReportSaveAsColumnTypesContracts.al:222-229: IncludeCaption adds a Labels block, between
// the information block and the data items, with the field's caption.
#[test]
fn include_caption_writes_a_label_for_the_column() {
    let text = dataset_of("Probe Caption Report", "");
    assert!(
        text.contains("</BCReportInformation>|  <Labels>|    <Label name=\"WithCaptionCaption\">Id</Label>|  </Labels>|  <DataItems>|"),
        "{text}"
    );
}

// handlers/TestReportNestedLinkImplicitParent.al: `Change Log Setup (Table)` is `Change_Log_Setup_Table`.
#[test]
fn a_data_item_name_with_spaces_and_parentheses_is_written_the_way_the_corpus_shows() {
    let text = dataset_of("Probe Quoted Report", "");
    assert!(
        text.contains(
            "<DataItem language=\"en-US\" formatRegion=\"en-US\" name=\"Probe_Item_Parent\" tableId=\"68300\" "
        ),
        "{text}"
    );
}

// handlers/PWReportSaveAsColumns.al:Labels_NoRowAtAll sees a line that starts with `<DataItems` in a dataset with no row at
// all. Assumption (docs/probes-needed.md): it is the short form `<DataItems />`.
#[test]
fn a_dataset_without_rows_has_an_empty_data_items_element() {
    let text = f(
        "var H: Codeunit Helpers; Buf: Record \"Probe Rep Parent\"; Out: OutStream; begin
            Buf.DeleteAll();
            Buf.Data.CreateOutStream(Out);
            Report.SaveAs(Report::\"Probe SaveAs Report\", '', ReportFormat::Xml, Out);
            exit(H.Lines(Buf));
        end;",
    );
    assert!(
        text.ends_with("  </BCReportInformation>|  <DataItems />|</ReportDataSet>|"),
        "{text}"
    );
}

#[test]
fn a_second_saveas_on_one_report_variable_is_refused() {
    let why = refusal(
        "var H: Codeunit Helpers; R: Report \"Probe Plain Report\"; Buf: Record \"Probe Rep Parent\"; Out: OutStream; begin
            H.Seed();
            Buf.Data.CreateOutStream(Out);
            R.SaveAs('', ReportFormat::Xml, Out);
            R.SaveAs('', ReportFormat::Xml, Out);
        end;",
    );
    assert!(why.contains("second SaveAs"), "{why}");
}

// report/W3ReportDataset.al:Rows_ParentsWithoutChildRows (staging probe, BC 27 to 29): a parent row whose item
// declares nested items but no child row reaches them is written with an empty `DataItems` element.
#[test]
fn a_parent_row_without_child_rows_has_an_empty_nested_data_items_element() {
    let text = f("var H: Codeunit Helpers; Buf: Record \"Probe Rep Parent\"; Parent: Record \"Probe Rep Parent\"; Out: OutStream; begin
            H.Seed();
            Parent.Init(); Parent.Id := 3; Parent.Insert();
            Buf.Data.CreateOutStream(Out);
            Report.SaveAs(Report::\"Probe SaveAs Report\", '', ReportFormat::Xml, Out);
            exit(H.Lines(Buf));
        end;");
    let third = text.rsplit("<Column name=\"PNo\">3</Column>|").next().unwrap();
    assert!(third.starts_with("        <Column name=\"PName\"></Column>|"), "{text}");
    assert!(
        third.contains("      </Columns>|      <DataItems />|    </DataItem>|  </DataItems>|</ReportDataSet>|"),
        "{third}"
    );
}

// handlers/TestReportDatasetHelpLink.al:RenderDataset reads the stream back with XmlDocument.ReadFrom, and
// handlers/TestReportFlowFieldColumnAutoCalc.al walks the rows with SelectNodes and foreach.
#[test]
fn the_dataset_reads_back_as_xml() {
    assert_eq!(
        f("var H: Codeunit Helpers; Buf: Record \"Probe Rep Parent\"; Out: OutStream; Inp: InStream; Doc: XmlDocument; Rows: XmlNodeList; Row: XmlNode; Cell: XmlNode; Result: Text; begin
            H.Seed();
            Buf.Data.CreateOutStream(Out);
            Report.SaveAs(Report::\"Probe SaveAs Report\", '', ReportFormat::Xml, Out);
            Buf.CalcFields(Data);
            Buf.Data.CreateInStream(Inp);
            if not XmlDocument.ReadFrom(Inp, Doc) then exit('unreadable');
            Doc.SelectNodes('//DataItem[@name=''P'']', Rows);
            foreach Row in Rows do begin
                Row.SelectSingleNode('Columns/Column[@name=''PName'']', Cell);
                Result += Cell.AsXmlElement().InnerText() + ';';
            end;
            Doc.SelectSingleNode('/ReportDataSet/BCReportInformation/ReportMetadata/ReportId', Cell);
            exit(Result + Cell.AsXmlElement().InnerText());
        end;"),
        "one <&>;two;68302"
    );
}

// docs/probes-needed.md, "Report.SaveAs dataset": UseTemporary makes the item's record temporary.
#[test]
fn a_data_item_over_a_temporary_record_has_no_rows_of_the_table() {
    let text = dataset_of("Probe Temporary Report", "");
    assert!(!text.contains("<DataItem "), "{text}");
}

// session/TestBCPlatformContracts.al:CompanyProperty_DisplayName_NonEmpty
#[test]
fn the_company_display_name_is_not_empty() {
    assert_ne!(f("begin exit(CompanyProperty.DisplayName()); end;"), "");
}

#[test]
fn a_processing_only_report_raises_in_the_statement_form() {
    // handlers/TestReportSaveAsColumnDetailsContracts.al:ProcessingOnlyReport_StatementFormRaises
    match run(
        "var H: Codeunit Helpers; Buf: Record \"Probe Rep Parent\"; Out: OutStream; begin H.Seed(); Buf.Data.CreateOutStream(Out); Report.SaveAs(Report::\"Probe Processing Report\", '', ReportFormat::Xml, Out); end;",
    ) {
        Err(Signal::Error(e)) => {
            assert!(
                e.message
                    .contains("is a processing-only report. Output cannot be rendered and saved to a file.")
            );
            assert_eq!(e.code, "ReportProcessingOnly");
        }
        other => panic!("{other:?}"),
    }
}

// handlers/TestReportSaveAsDatasetContracts.al:253-300 (BC28PLUS): from BC 28 every DataItem element carries
// ` tableId="<table id>" systemId="<guid>"` after its name, the table id of the data item's table and a different
// lower case 36 character GUID per element; before BC 28 it carries neither.
#[test]
fn from_bc_28_every_data_item_carries_its_table_id_and_a_distinct_guid() {
    for (bc, with) in [("27.5", false), ("28.5", true), ("29.0", true)] {
        let text = match run_on(Some(bc), DATASET) {
            Ok(Value::Text(t)) => t,
            other => panic!("{other:?}"),
        };
        let items: Vec<&str> = text
            .split('|')
            .filter(|l| l.trim_start().starts_with("<DataItem "))
            .collect();
        assert_eq!(items.len(), 5, "BC {bc}");
        if !with {
            assert!(items.iter().all(|l| !l.contains("tableId")), "BC {bc}: {items:?}");
            continue;
        }
        let mut seen = Vec::new();
        for (l, table) in items.iter().zip(["68300", "68301", "68301", "68300", "68301"]) {
            let at = l
                .find(" tableId=\"")
                .unwrap_or_else(|| panic!("BC {bc}: no tableId in {l}"));
            assert!(
                l[..at].ends_with(&format!("name=\"{}\"", if table == "68300" { "P" } else { "C" })),
                "{l}"
            );
            let rest = &l[at + " tableId=\"".len()..];
            assert!(rest.starts_with(&format!("{table}\" systemId=\"")), "{l}");
            let guid = rest[table.len() + "\" systemId=\"".len()..]
                .trim_end_matches(">")
                .trim_end_matches('"');
            assert_eq!(guid.len(), 36, "{l}");
            assert_eq!(guid, guid.to_lowercase(), "{l}");
            assert!(!seen.contains(&guid), "different per item: {l}");
            seen.push(guid);
        }
    }
}

// handlers/TestReportSaveAsColumnTypesContracts.al:372-388: a ProcessingOnly report answers false with the
// text and the code ReportProcessingOnly, and writes nothing; with the answer dropped it is refused.
#[test]
fn a_processing_only_report_answers_false_with_its_error_and_writes_nothing() {
    let out = f(
        "var H: Codeunit Helpers; Buf: Record \"Probe Rep Parent\"; Out: OutStream; Saved: Boolean; begin H.Seed(); Buf.Data.CreateOutStream(Out); Saved := Report.SaveAs(Report::\"Probe Processing Only\", '', ReportFormat::Xml, Out); exit(Format(Saved) + '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + H.Lines(Buf)); end;",
    );
    assert_eq!(
        out,
        "No|Report 68399 is a processing-only report. Output cannot be rendered and saved to a file.|ReportProcessingOnly|"
    );
}

// reportextensiontrigger/TestReportExtensionDataItemPlacementContracts.al:150-198: the triggers run, and the
// data items and their columns are written, in position order.
#[test]
fn added_data_items_run_and_are_written_in_position_order() {
    let out = f(
        "var P: Record \"Probe Place Parent\"; C: Record \"Probe Place Child\"; L: Codeunit \"Probe Place Log\"; Buf: Record \"Probe Rep Parent\"; H: Codeunit Helpers; Out: OutStream; s: Text; begin
            P.Id := 1; P.Insert(); C.Id := 10; C.ParentId := 1; C.Insert(); L.Take();
            Buf.Data.CreateOutStream(Out);
            Report.SaveAs(Report::\"Probe Place Base\", '', ReportFormat::Xml, Out);
            s := L.Take() + '#' + H.Lines(Buf);
            exit(s);
        end;",
    );
    let (log, xml) = out.split_once('#').unwrap();
    assert_eq!(
        log,
        "Parent1;ExtFirst10;ExtBeforeA10;ChildA10;ExtAfterA10;ExtBeforeB10;ChildB10;ExtLast10;"
    );
    let names: Vec<&str> = xml
        .split('|')
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("<DataItem "))
        .filter_map(|l| l.split("name=\"").nth(1))
        .filter_map(|l| l.split('"').next())
        .collect();
    assert_eq!(
        names,
        [
            "Parent",
            "ExtFirst",
            "ExtBeforeA",
            "ChildA",
            "ExtAfterA",
            "ExtBeforeB",
            "ChildB",
            "ExtLast"
        ]
    );
    let cols: Vec<&str> = xml
        .split('|')
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("<Column name=\""))
        .filter_map(|l| l.split('"').next())
        .collect();
    assert_eq!(cols, ["PId", "EFId", "EBAId", "AId", "EAId", "EBId", "BId", "ELId"]);
}

// A report with the FormatRegion property: report/lt-y-report-props/YReportPropsTests.al:34-148 pins the DataItems block of its
// SaveAs Xml dataset (the helper cuts the text at `<DataItems>`), not the header before it (`ReportDataSet formatRegion=` and the
// `FormatRegion` element). What BC writes there is not pinned, so the SaveAs is refused (docs/probes-needed.md, corpus #657).
#[test]
fn the_save_as_of_a_report_with_the_format_region_property_is_refused_for_its_header() {
    let body = |id: u32| {
        format!(
            "var H: Codeunit Helpers; All: Text; begin H.Seed(); All := H.Dataset({id}, ''); exit(CopyStr(All, StrPos(All, '<DataItems>'))); end;"
        )
    };
    for (id, region) in [(68391, "fr-FR"), (68392, "ja-JP")] {
        for bc in ["27.5", "29.0"] {
            match run_on(Some(bc), &body(id)) {
                Err(Signal::Unsupported(r)) => {
                    assert!(r.contains("header") && r.contains(region), "{id} {bc}: {r}")
                }
                other => panic!("{id} {bc}: {other:?}"),
            }
        }
    }
    // A region no test measured is refused before the run.
    match run_on(Some("27.5"), &body(68393)) {
        Err(Signal::Unsupported(r)) => assert!(r.contains("nl-NL"), "{r}"),
        other => panic!("{other:?}"),
    }
}

// `Format` in OnInitReport of a report with the FormatRegion property: whether it already follows the report's region is not
// pinned (the corpus reports format only in OnAfterGetRecord, report/lt-y-report-props/YReports.al:32-48), so it is refused
// rather than written in the session's en-US.
#[test]
fn format_in_oninitreport_of_a_report_with_the_format_region_property_is_refused() {
    let r = refusal("var H: Codeunit Helpers; All: Text; begin H.Seed(); All := H.Dataset(68394, ''); exit(All); end;");
    assert!(r.contains("OnInitReport") && r.contains("de-DE"), "{r}");
}

// P138ColCaptionQualified.al:QS_Labels_WhenNoRows: the label of a field written with the name of the own, the parent, a child or an
// earlier top-level data item is the field's caption, in dataset order, also when no row is written. With rows the value of a
// parent column that reads the child is refused (QS_Columns_DataItemNamedLikeTable: BC writes the child's first row).
#[test]
fn include_caption_of_a_field_written_with_a_data_item_name() {
    assert!(
        refusal(
            "var H: Codeunit Helpers; begin H.Seed(); exit(H.Dataset(Report::\"Probe Caption Sources\", '')); end;"
        )
        .contains("column FromChild reads a field of the data item Child nested below its own")
    );
    let text = f(
        "var H: Codeunit Helpers; P: Record \"Probe Rep Parent\"; C: Record \"Probe Rep Child\"; begin
            P.DeleteAll(); C.DeleteAll(); exit(H.Dataset(Report::\"Probe Caption Sources\", '')); end;",
    );
    assert!(
        text.contains("<Labels>|    <Label name=\"IdOwnCaption\">Id</Label>|    <Label name=\"FromChildCaption\">Code</Label>|    <Label name=\"FromParentCaption\">Name</Label>|    <Label name=\"FromSiblingCaption\">Amt</Label>|  </Labels>"),
        "{text}"
    );
}
