//! Lane T8 (report request pages, report views, query OnBeforeOpen): one test per corpus pin.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const QUERY_SELF: &str = r#"
table 60950 "RP Row"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
query 60951 "RP Self"
{
    elements { dataitem(Row; "RP Row") { column(Id; Id) { } } }

    trigger OnBeforeOpen()
    var
        Tmp: Record "RP Row" temporary;
        OutS: OutStream;
    begin
        Depth += 1;
        if Depth > 1 then
            exit;
        case Mode of
            1:
                CurrQuery.Open();
            2:
                begin
                    Tmp.Data.CreateOutStream(OutS);
                    CurrQuery.SaveAsXml(OutS);
                end;
            3:
                CurrQuery.Close();
            4:
                if CurrQuery.Read() then
                    Error('read a row');
        end;
    end;

    procedure SetMode(NewMode: Integer)
    begin
        Mode := NewMode;
    end;

    var
        Mode: Integer;
        Depth: Integer;
}
codeunit 60952 C
{
    procedure Seed()
    var R: Record "RP Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.Insert();
        R.Id := 2; R.Insert();
    end;

    procedure Run(NewMode: Integer): Text
    var
        Q: Query "RP Self";
        Rows: Text;
    begin
        Seed();
        Q.SetMode(NewMode);
        Q.Open();
        while Q.Read() do
            Rows += Format(Q.Id) + ',';
        exit(Rows);
    end;

    procedure ModeOpen(): Text begin exit(Run(1)); end;
    procedure ModeSaveAsXml(): Text begin exit(Run(2)); end;
    procedure ModeClose(): Text begin exit(Run(3)); end;
    procedure ModeRead(): Text begin exit(Run(4)); end;
}
"#;

fn error_and_code(r: Result<Value, Signal>) -> (String, String) {
    match r {
        Err(Signal::Error(e)) => (e.message.clone(), e.code.clone()),
        other => panic!("expected an error, got {other:?}"),
    }
}

/// CurrQuery.Open and CurrQuery.Close inside OnBeforeOpen raise QueryIllegalCallInTrigger
/// (streams/ChQueryLeads.al:259-290).
#[test]
fn open_and_close_inside_on_before_open_raise() {
    for routine in ["ModeOpen", "ModeClose"] {
        let (message, code) = error_and_code(al_testkit::run_in(QUERY_SELF, "C", routine));
        assert_eq!(
            message, "It is not possible to execute the current query from within the 'OnBeforeOpen' trigger.",
            "{routine}"
        );
        assert_eq!(code, "QueryIllegalCallInTrigger", "{routine}");
    }
}

/// CurrQuery.Read inside OnBeforeOpen returns false and the outer Open reads every row
/// (streams/ChQueryLeads.al:294-301).
#[test]
fn read_inside_on_before_open_returns_false() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(QUERY_SELF, "C", "ModeRead")),
        "1,2,"
    );
}

/// CurrQuery.SaveAsXml inside OnBeforeOpen returns false but writes bytes the corpus does not
/// show (only their count, streams/ChQueryLeads.al:271-278): refused.
#[test]
fn save_as_xml_inside_on_before_open_is_refused() {
    match al_testkit::run_in(QUERY_SELF, "C", "ModeSaveAsXml") {
        Err(Signal::Unsupported(s)) => assert!(s.contains("OnBeforeOpen"), "{s}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

const ZERO_DATE_VIEW: &str = r#"
table 60960 "RP Dated" { fields { field(1; Id; Integer) { } field(2; D; Date) { } field(3; Data; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 60961 "RP Log" { SingleInstance = true; var Lines: Text; procedure Add(L: Text) begin Lines += L + ';'; end; procedure Take(): Text begin exit(Lines); end; }
report 60962 "RP Zero Date"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset
    {
        dataitem(Rows; "RP Dated")
        {
            DataItemTableView = where(D = const(0D));
            trigger OnPreDataItem() var L: Codeunit "RP Log"; begin L.Add('pre'); end;
            trigger OnAfterGetRecord() var L: Codeunit "RP Log"; begin L.Add('row'); end;
        }
    }
    trigger OnPreReport() var L: Codeunit "RP Log"; begin L.Add('prereport'); end;
}
report 60966 "RP Zero Date Save"
{
    UseRequestPage = false;
    dataset { dataitem(Rows; "RP Dated") { DataItemTableView = where(D = const(0D)); column(RowId; Id) { } } }
    trigger OnPreReport() var L: Codeunit "RP Log"; begin L.Add('prereport'); end;
}
report 60963 "RP Zero Date Init"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset { dataitem(Rows; "RP Dated") { DataItemTableView = where(D = const(0D)); } }
    trigger OnInitReport() begin end;
}
report 60964 "RP Zero Date Filter"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset { dataitem(Rows; "RP Dated") { DataItemTableView = where(D = filter(0D)); } }
}
codeunit 60965 C
{
    procedure Seed() var R: Record "RP Dated"; begin R.Id := 1; R.Insert(); end;
    procedure RunIt() begin Seed(); Report.Run(Report::"RP Zero Date", false); end;
    procedure Log(): Text var L: Codeunit "RP Log"; begin exit(L.Take()); end;
    procedure RunInit() begin Seed(); Report.Run(Report::"RP Zero Date Init", false); end;
    procedure RunFilter() begin Seed(); Report.Run(Report::"RP Zero Date Filter", false); end;
    procedure SaveIt(): Text
    var Tmp: Record "RP Dated" temporary; OutS: OutStream; Saved: Boolean;
    begin
        Seed();
        Tmp.Id := 1; Tmp.Insert();
        Tmp.Data.CreateOutStream(OutS);
        ClearLastError();
        Saved := Report.SaveAs(Report::"RP Zero Date Save", '', ReportFormat::Xml, OutS);
        exit(Format(Saved) + '|' + GetLastErrorCode() + '|' + CopyStr(GetLastErrorText(), 1, 27));
    end;
}
"#;

/// A report data item whose DataItemTableView filters a Date field with const(0D) fails before OnPreReport with
/// Evaluate of the date '' (streams/CjDataset.al:190-211; record/P136Report.al:331-351).
#[test]
fn report_view_const_zero_date_fails_before_on_pre_report() {
    let mut b = al_testkit::Built::from_source(ZERO_DATE_VIEW);
    let (message, code) = error_and_code(b.run_in("C", "RunIt"));
    assert_eq!(
        message,
        "The format of the Date '''' does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'."
    );
    assert_eq!(code, "Evaluate");
    assert_eq!(al_testkit::text(b.run_in("C", "Log")), "");
}

/// With the answer used, SaveAs answers false and the last error is that Evaluate error
/// (streams/CjDataset.al:190-211, ZeroDateView_ConstZeroDate_SaveAsXml).
#[test]
fn report_view_const_zero_date_save_as_answers_false_with_the_error() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(ZERO_DATE_VIEW, "C", "SaveIt")),
        "No|Evaluate|The format of the Date ''''"
    );
}

/// With OnInitReport, or with filter(0D), nothing pins where the run fails: refused.
#[test]
fn report_view_zero_date_unpinned_shapes_are_refused() {
    for routine in ["RunInit", "RunFilter"] {
        match al_testkit::run_in(ZERO_DATE_VIEW, "C", routine) {
            Err(Signal::Unsupported(s)) => assert!(s.contains("0D"), "{routine}: {s}"),
            other => panic!("{routine}: expected a refusal, got {other:?}"),
        }
    }
}

const SAME_NAME: &str = r#"
table 60970 "RP Named"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { Clustered = true; } }
    procedure Foo(): Text begin exit('table ' + Format(Id)); end;
}
codeunit 60971 "RP Log2" { SingleInstance = true; var Lines: Text; procedure Add(L: Text) begin Lines += L + ';'; end; procedure Take(): Text begin exit(Lines); end; }
report 60972 "RP Same Name"
{
    UseRequestPage = false;
    dataset
    {
        dataitem(Row; "RP Named")
        {
            column(FooCol; Foo()) { }
            trigger OnAfterGetRecord() var L: Codeunit "RP Log2"; begin L.Add('row foo=' + Foo()); end;
        }
    }
    trigger OnPreReport() var L: Codeunit "RP Log2"; begin L.Add('prereport foo=' + Foo()); end;
    procedure Foo(): Text begin exit('report'); end;
}
codeunit 60973 C
{
    procedure Run(): Text
    var
        R: Record "RP Named";
        Tmp: Record "RP Named" temporary;
        L: Codeunit "RP Log2";
        OutS: OutStream;
        InS: InStream;
        Xml: Text;
        Line: Text;
    begin
        R.Id := 1; R.Insert();
        R.Id := 2; R.Insert();
        Tmp.Id := 1; Tmp.Insert();
        Tmp.Data.CreateOutStream(OutS);
        Report.SaveAs(Report::"RP Same Name", '', ReportFormat::Xml, OutS);
        Tmp.Data.CreateInStream(InS);
        while not InS.EOS() do begin InS.ReadText(Line); Xml += Line.Trim(); end;
        exit(L.Take() + '|' + Format(Xml.Contains('"FooCol">table 1<')) + Format(Xml.Contains('"FooCol">table 2<')) + Format(Xml.Contains('"FooCol">report<')));
    end;
}
"#;

/// An unqualified call in a data item's trigger and in its column's source runs the procedure of the data item's table,
/// not the report's of the same name; in a report trigger it runs the report's
/// (streams/CjDataset.al:215-241, SameNameProcedure_ExtensionField_SaveAsXml).
#[test]
fn same_name_procedure_in_a_data_item_runs_the_tables() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SAME_NAME, "C", "Run")),
        "prereport foo=report;row foo=table 1;row foo=table 2;|YesYesNo"
    );
}

/// The same report with the table's `Foo` taking other parameters than the report's, or declared by a table extension:
/// which one an unqualified call in the data item runs is not pinned (CjDataset.al:215-241 pins only the same
/// parameters on the table itself), so the call is refused.
#[test]
fn same_name_procedure_with_other_parameters_or_on_a_table_extension_is_refused() {
    let other_params = SAME_NAME.replace(
        "procedure Foo(): Text begin exit('table ' + Format(Id)); end;",
        "procedure Foo(X: Integer): Text begin exit('table x'); end;",
    );
    let extra_overload = SAME_NAME.replace(
        "procedure Foo(): Text begin exit('table ' + Format(Id)); end;",
        "procedure Foo(): Text begin exit('table ' + Format(Id)); end; procedure Foo(X: Integer): Text begin exit('table x'); end;",
    );
    let on_extension = SAME_NAME
        .replace("procedure Foo(): Text begin exit('table ' + Format(Id)); end;", "")
        .replace(
            "codeunit 60971",
            "tableextension 60974 \"RP Named Ext\" extends \"RP Named\" { procedure Foo(): Text begin exit('ext'); end; }\ncodeunit 60971",
        );
    for (what, src) in [
        ("other parameters", other_params),
        ("an extra overload on the table", extra_overload),
        ("table extension", on_extension),
    ] {
        match al_testkit::run_in(&src, "C", "Run") {
            Err(Signal::Unsupported(s)) => assert!(s.contains("not pinned") && s.contains("Foo"), "{what}: {s}"),
            other => panic!("{what}: expected a refusal, got {other:?}"),
        }
    }
}
