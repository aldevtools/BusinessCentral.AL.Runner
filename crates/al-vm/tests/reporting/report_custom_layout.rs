//! `Report.SaveAs` of a report whose rendering layout is `Type = Custom`: the platform's `CustomDocumentMergerEx` is raised
//! with the layout file, and a subscriber that reads the ObjectPayload is refused (corpus topic #176). The raise itself
//! is the assumption custom-layout-merger; handlers/TestReportLayoutByNameMergerCapture.al pins the subscriber's shape.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_vm::{Engine, Session};

fn fixtures(subscriber_body: &str, save: &str) -> String {
    format!(
        r#"
table 68400 "CL Row" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ Clustered = true; }} }} }}
report 68401 "CL Report"
{{
    UsageCategory = None;
    Caption = 'CL Caption';
    DefaultRenderingLayout = Mine;
    dataset {{ dataitem(R; "CL Row") {{ column(Id; Id) {{ }} }} }}
    rendering {{ layout(Mine) {{ Type = Custom; LayoutFile = './x/mine.cl'; MimeType = 'application/x-cl'; }} }}
}}
codeunit 68402 "CL Sub"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'CustomDocumentMergerEx', '', false, false)]
    local procedure Merge(ObjectID: Integer; ReportAction: Option SaveAsPdf,SaveAsWord,SaveAsExcel,Preview,Print,SaveAsHtml; ObjectPayload: JsonObject; XmlData: InStream; LayoutData: InStream; var DocumentStream: OutStream; var Success: Boolean)
    var Line: Text; Layout: Text; Probe: JsonToken; K: Text; Keys: Text;
    begin
        while not LayoutData.EOS() do begin LayoutData.ReadText(Line); Layout += Line; end;
        {subscriber_body}
        DocumentStream.WriteText('rendered:' + Layout);
        Success := true;
    end;
}}
table 68403 "CL Blob" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Id) {{ Clustered = true; }} }} }}
codeunit 50100 C
{{
    procedure F(): Text
    var B: Record "CL Blob"; O: OutStream; I: InStream; Text: Text; Ok: Boolean; Rep: Report "CL Report"; Row: Record "CL Row"; RecRef: RecordRef;
    begin
        B.Insert();
        B.Data.CreateOutStream(O);
        {save}
        B.Modify();
        B.CalcFields(Data);
        B.Data.CreateInStream(I);
        I.ReadText(Text);
        exit(Format(Ok) + '|' + Text);
    end;
}}
"#
    )
}

fn run(subscriber_body: &str) -> Result<Value, Signal> {
    run_save(subscriber_body, STATIC_SAVE)
}

const STATIC_SAVE: &str = "Ok := Report.SaveAs(Report::\"CL Report\", '', ReportFormat::Pdf, O);";

fn run_save(subscriber_body: &str, save: &str) -> Result<Value, Signal> {
    run_src(fixtures(subscriber_body, save))
}

fn run_src(src: String) -> Result<Value, Signal> {
    let apps = vec![AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), src)],
    }];
    let (ws, mut l) = al_sema::lower::compile(apps, &Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    std::sync::Arc::make_mut(&mut l.program).apps[0].layout_files = vec![("x/mine.cl".into(), b"LAYOUT-TEXT".to_vec())];
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
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

#[test]
fn a_subscriber_that_does_not_read_the_payload_renders_the_layout_text() {
    match run("") {
        Ok(Value::Text(t)) => assert_eq!(t.to_string(), "Yes|rendered:LAYOUT-TEXT"),
        other => panic!("{other:?}"),
    }
}

/// The ObjectPayload holds 19 keys in this order, with the report's caption, the format's mime type, the layout's name
/// and mime type (report/lt-report-layout-merger/ReportMergerPayload.al:37-38, :119; ReportMergerLayoutChoice.al:94).
#[test]
fn the_payload_holds_the_pinned_keys_and_values() {
    let body = "foreach K in ObjectPayload.Keys() do Keys += K + ',';
        foreach K in ObjectPayload.Keys() do
            if K in ['invokedby', 'invokeddatetime', 'companyname', 'layoutapplicationid', 'reportrunid'] then
                ObjectPayload.Remove(K);
        ObjectPayload.WriteTo(Line);
        Layout := Keys + '|' + Line + '|' + Layout;";
    match run(body) {
        Ok(Value::Text(t)) => assert_eq!(
            t.to_string(),
            "Yes|rendered:filterviews,version,objectname,objectid,documenttype,invokedby,invokeddatetime,companyname,printername,\
             duplex,color,defaultcopies,papertray,intent,layoutmodel,layoutname,layoutmimetype,layoutapplicationid,reportrunid,|\
             {\"filterviews\":[],\"version\":1,\"objectname\":\"CL Caption\",\"objectid\":68401,\"documenttype\":\"application/pdf\",\
             \"printername\":\"\",\"duplex\":false,\"color\":false,\"defaultcopies\":1,\"papertray\":null,\"intent\":\"Save\",\
             \"layoutmodel\":\"Custom\",\"layoutname\":\"Mine\",\"layoutmimetype\":\"application/x-cl\"}|LAYOUT-TEXT"
        ),
        other => panic!("{other:?}"),
    }
}

/// The filterviews of a SaveAs whose data item has a descending view the caller set (SetTableView) are not pinned (only an
/// ascending view on the primary key is, ReportMergerUnmasked.al:262-286): the payload is refused on use, and a subscriber
/// that does not read it still renders.
#[test]
fn the_payload_of_a_save_after_a_descending_set_table_view_is_refused_on_use() {
    let save =
        "Row.SetRange(Id, 1); Row.Ascending(false); Rep.SetTableView(Row); Ok := Rep.SaveAs('', ReportFormat::Pdf, O);";
    match run_save("if ObjectPayload.Get('layoutname', Probe) then;", save) {
        Err(Signal::Unsupported(why)) => assert!(why.contains("filterviews"), "{why}"),
        other => panic!("{other:?}"),
    }
    match run_save("", save) {
        Ok(Value::Text(t)) => assert_eq!(t.to_string(), "Yes|rendered:LAYOUT-TEXT"),
        other => panic!("{other:?}"),
    }
}

/// What a subscriber reads of one payload key, as raw JSON.
const RAW_KEY: &str = "ObjectPayload.Get(KEYNAME, Probe); Probe.WriteTo(Line); Layout := Line;";

fn raw_key(key: &str, save: &str) -> String {
    match run_save(&RAW_KEY.replace("KEYNAME", &format!("'{key}'")), save) {
        Ok(Value::Text(t)) => t.to_string(),
        other => panic!("{other:?}"),
    }
}

/// invokeddatetime is the clock as `yyyy-mm-ddThh:mm:ss.fff+hh:mm`, an offset and not `Z`
/// (report/lt-report-layout-merger/ReportMergerUnmasked.al:229-245, Payload_Unmasked_InvokedDateTime).
#[test]
fn invokeddatetime_has_an_offset() {
    let t = raw_key("invokeddatetime", STATIC_SAVE);
    let shape: String = t.chars().map(|c| if c.is_ascii_digit() { 'd' } else { c }).collect();
    assert_eq!(shape, "Yes|rendered:\"dddd-dd-ddTdd:dd:dd.ddd+dd:dd\"", "{t}");
}

/// filterviews after SetTableView: one object per data item with its name, table id and view
/// (ReportMergerUnmasked.al:262-286, Payload_FilterViews_AfterSetTableView).
#[test]
fn filterviews_after_set_table_view() {
    let save = "Row.SetRange(Id, 1); Rep.SetTableView(Row); Ok := Rep.SaveAs('', ReportFormat::Pdf, O);";
    assert_eq!(
        raw_key("filterviews", save),
        "Yes|rendered:[{\"name\":\"R\",\"tableid\":68400,\"view\":\"VERSION(1) SORTING(Field1) WHERE(Field1=1(1))\"}]"
    );
}

/// filterviews of a SaveAs with a RecordRef (ReportMergerUnmasked.al:288-310, Payload_FilterViews_WithRecordRef).
#[test]
fn filterviews_with_a_record_ref() {
    let save = "RecRef.Open(68400); RecRef.Field(1).SetRange(2); \
                Ok := Report.SaveAs(Report::\"CL Report\", '', ReportFormat::Pdf, O, RecRef);";
    assert_eq!(
        raw_key("filterviews", save),
        "Yes|rendered:[{\"name\":\"R\",\"tableid\":68400,\"view\":\"VERSION(1) SORTING(Field1) WHERE(Field1=1(2))\"}]"
    );
}

/// A report without a Caption gives its object name as objectname (ReportMergerUnmasked.al:398-425,
/// Payload_ReportWithoutCaption).
#[test]
fn objectname_of_a_report_without_caption_is_its_name() {
    let src = fixtures(&RAW_KEY.replace("KEYNAME", "'objectname'"), STATIC_SAVE).replace("Caption = 'CL Caption';", "");
    match run_src(src) {
        Ok(Value::Text(t)) => assert_eq!(t.to_string(), "Yes|rendered:\"CL Report\""),
        other => panic!("{other:?}"),
    }
}
