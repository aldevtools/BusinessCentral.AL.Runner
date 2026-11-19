//! `Media.ExportStream` of the Layout of a Report Layout List row writes the bytes of the layout file the app holds,
//! answers true and leaves HasValue No, the same bytes a second time (reports/layouts-saveas/Y17LayoutExportTests.al:70
//! RDLC, :88 Word, :129 Excel, :146 twice; corpus master). The platform table is not in the test workspace: a table of its
//! number and name stands in.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_vm::{Engine, Session};

const SRC: &str = r#"
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
report 50200 "RL Probe" { DefaultRenderingLayout = Rdl1;
  dataset { dataitem(I; Integer) { column(N; Number) { } } }
  rendering {
    layout(Rdl1) { Type = RDLC; LayoutFile = './x/a.rdl'; }
    layout(Doc1) { Type = Word; LayoutFile = './x/missing.docx'; } } }
report 50201 "RL Legacy" { RDLCLayout = './x/../x/legacy.rdl';
  dataset { dataitem(I; Integer) { column(N; Number) { } } } }
report 50203 "RL Legacy Rdlc" { RDLCLayout = './x/sync.rdl';
  dataset { dataitem(I; B) { column(N; K) { } } } }
report 50204 "RL Legacy Word" { WordLayout = './x/w.docx';
  dataset { dataitem(I; B) { column(N; K) { } } } }
report 50205 "RL Legacy Excel" { ExcelLayout = './x/l.xlsx';
  dataset { dataitem(I; Integer) { column(N; Number) { } } } }
table 60981 B { fields { field(1; K; Integer) { } field(2; Src; Blob) { } } keys { key(PK; K) { } } }
codeunit 50202 C {
    procedure Legacy(): Text
    var L: Record "Report Layout List"; B: Record B; outs: OutStream; ins: InStream; Ok: Boolean; T: Text;
    begin
        L.SetRange("Report ID", 50201);
        L.SetRange("Layout Format", L."Layout Format"::RDLC);
        L.FindFirst();
        B.Src.CreateOutStream(outs);
        Ok := L.Layout.ExportStream(outs);
        B.Insert();
        B.CalcFields(Src);
        B.Src.CreateInStream(ins);
        ins.ReadText(T);
        exit(Format(L.Count()) + '|' + L.Name + '|' + Format(Ok) + '|' + T + '|' + Format(L.Layout.HasValue()));
    end;
    procedure ExportOf(ReportId: Integer; LayoutFormat: Integer): Text
    var L: Record "Report Layout List"; B: Record B; outs: OutStream; ins: InStream; Ok: Boolean; T: Text;
    begin
        L.SetRange("Report ID", ReportId);
        L.SetRange("Layout Format", LayoutFormat);
        L.FindFirst();
        B.Src.CreateOutStream(outs);
        Ok := L.Layout.ExportStream(outs);
        B.Insert();
        B.CalcFields(Src);
        B.Src.CreateInStream(ins);
        ins.ReadText(T);
        exit(Format(Ok) + '|' + T);
    end;
    procedure Part(Extended: Boolean): Text
    begin
        exit(Report.WordXmlPart(50204, Extended));
    end;
    procedure Export(LayoutFormat: Integer): Text
    var L: Record "Report Layout List"; B: Record B; outs: OutStream; ins: InStream; Ok: Boolean; T: Text; All: Text; i: Integer;
    begin
        L.SetRange("Report ID", 50200);
        L.SetRange("Layout Format", LayoutFormat);
        L.FindFirst();
        for i := 1 to 2 do begin
            B.K := i;
            B.Src.CreateOutStream(outs);
            Ok := L.Layout.ExportStream(outs);
            B.Insert();
            B.CalcFields(Src);
            B.Src.CreateInStream(ins);
            ins.ReadText(T);
            All += Format(Ok) + '|' + T + '|' + Format(L.Layout.HasValue()) + ';';
        end;
        exit(All);
    end;
}
"#;

fn run(format: i32) -> Result<Value, Signal> {
    run_routine("Export", vec![Value::Integer(format)])
}

fn run_routine(routine: &str, args: Vec<Value>) -> Result<Value, Signal> {
    run_routine_with(routine, args, false)
}

fn run_routine_with(routine: &str, args: Vec<Value>, from_package: bool) -> Result<Value, Signal> {
    run_with_files(routine, args, from_package, held_files())
}

/// The files the app holds in every test: a rendering section RDLC and a legacy RDLC, neither as the compiler writes it.
fn held_files() -> Vec<(String, Vec<u8>)> {
    vec![
        ("x/a.rdl".into(), b"RDL-BYTES".to_vec()),
        ("x/legacy.rdl".into(), b"LEGACY-BYTES".to_vec()),
    ]
}

/// Serves `x/missing.docx` on demand, as a Microsoft app's package does.
struct Package;

impl al_ir::LayoutSource for Package {
    fn file(&self, path: &str) -> Option<Vec<u8>> {
        (path == "x/missing.docx").then(|| b"PACKAGE-DOC-BYTES".to_vec())
    }
}

fn run_with_files(
    routine: &str,
    args: Vec<Value>,
    from_package: bool,
    files: Vec<(String, Vec<u8>)>,
) -> Result<Value, Signal> {
    let apps = vec![AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), SRC.into())],
    }];
    let (ws, mut l) = al_sema::lower::compile(apps, &Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    std::sync::Arc::make_mut(&mut l.program).apps[0].layout_files = files;
    if from_package {
        std::sync::Arc::make_mut(&mut l.program).apps[0].layout_source =
            Some(al_ir::LayoutSourceRef(std::sync::Arc::new(Package)));
    }
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, args).map(|(v, _)| v)
}

#[test]
fn the_export_writes_the_layout_file_the_app_holds() {
    match run(0) {
        Ok(Value::Text(t)) => assert_eq!(t.to_string(), "Yes|RDL-BYTES|No;Yes|RDL-BYTES|No;"),
        other => panic!("{other:?}"),
    }
}

// A layout whose file the app does not hold (no file at its path under the app's folder, no package that keeps it) is refused.
#[test]
fn a_layout_whose_file_the_app_does_not_hold_is_refused() {
    let r = run(1);
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("layout file")),
        "{r:?}"
    );
}

// A layout of the legacy RDLCLayout property is a row named with the LayoutFile as written. Its file has no DataSet section
// as the compiler writes it for the report, so the compiler would rewrite it at build (Y17LayoutExportObjects.al:6-8): refused.
#[test]
fn a_legacy_file_the_compiler_would_rewrite_is_refused() {
    let r = run_routine("Legacy", vec![]);
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("the compiler rewrites it at build")),
        "{r:?}"
    );
}

// A layout file the app does not hold in the program but its package serves on demand (a Microsoft app: Custom Report
// Layout.InitBuiltInLayout of Base Application reaches this export, al-runner issue 5712; Microsoft's own tests call it, for
// example MailServiceTest.Codeunit.al `CustomReportLayout.InitBuiltInLayout(ReportID, Type::Word)`). The bytes are the
// package's file: the registered assumption `package-layout-file-is-the-compiled-layout` marks that BC may export the
// compiler's rewrite of the source file.
#[test]
fn a_layout_file_the_package_serves_on_demand_is_exported() {
    match run_routine_with("Export", vec![Value::Integer(1)], true) {
        Ok(Value::Text(t)) => assert_eq!(t.to_string(), "Yes|PACKAGE-DOC-BYTES|No;Yes|PACKAGE-DOC-BYTES|No;"),
        other => panic!("{other:?}"),
    }
}

/// An RDLC file with one data set whose Fields list names `fields`, in the shape the compiler writes
/// (Y17LayoutExportTests.al:83: CRLF, two spaces per level, DataSet_Result, the DataSource query).
fn rdlc(fields: &[&str]) -> Vec<u8> {
    let mut f = String::new();
    for n in fields {
        f.push_str(&format!(
            "        <Field Name=\"{n}\">\r\n          <DataField>{n}</DataField>\r\n        </Field>\r\n"
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<Report>\r\n  <DataSets>\r\n    <DataSet Name=\"DataSet_Result\">\r\n      <Fields>\r\n{f}      </Fields>\r\n      <Query>\r\n        <DataSourceName>DataSource</DataSourceName>\r\n        <CommandText />\r\n      </Query>\r\n    </DataSet>\r\n  </DataSets>\r\n</Report>"
    )
    .into_bytes()
}

fn export_of(report: i32, format: i32, files: Vec<(String, Vec<u8>)>) -> Result<Value, Signal> {
    run_with_files(
        "ExportOf",
        vec![Value::Integer(report), Value::Integer(format)],
        false,
        files,
    )
}

fn assert_rewrite_refused(r: Result<Value, Signal>) {
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("the compiler rewrites it at build")),
        "{r:?}"
    );
}

// A legacy RDLC file whose DataSet section is the one the compiler writes for the report exports its bytes
// (Y17LayoutExportTests.al:105-124: the file alc wrote, exported unchanged).
#[test]
fn a_legacy_rdlc_file_as_the_compiler_writes_it_exports_its_bytes() {
    match export_of(50203, 0, vec![("x/sync.rdl".into(), rdlc(&["N"]))]) {
        Ok(Value::Text(t)) => assert_eq!(t.to_string(), "Yes|<?xml version=\"1.0\" encoding=\"utf-8\"?>"),
        other => panic!("{other:?}"),
    }
}

// A legacy RDLC file whose Fields list names a column the report does not have (review probe A07_StaleRdlc): refused.
#[test]
fn a_legacy_rdlc_file_with_a_stale_fields_list_is_refused() {
    assert_rewrite_refused(export_of(50203, 0, vec![("x/sync.rdl".into(), rdlc(&["Stale_"]))]));
}

// A legacy RDLC file whose data set has another name than the one the compiler writes: refused.
#[test]
fn a_legacy_rdlc_file_with_another_data_set_name_is_refused() {
    let bytes = String::from_utf8(rdlc(&["N"]))
        .unwrap()
        .replace("DataSet_Result", "Other");
    assert_rewrite_refused(export_of(50203, 0, vec![("x/sync.rdl".into(), bytes.into_bytes())]));
}

/// A zip archive of stored entries, as far as the export's reader reads one (the CRC is not checked).
fn zip(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let (mut out, mut dir) = (Vec::new(), Vec::new());
    for (name, data) in entries {
        let at = out.len() as u32;
        let (n, size) = (name.len() as u16, data.len() as u32);
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&n.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        dir.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        dir.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        dir.extend_from_slice(&size.to_le_bytes());
        dir.extend_from_slice(&size.to_le_bytes());
        dir.extend_from_slice(&n.to_le_bytes());
        dir.extend_from_slice(&[0; 12]);
        dir.extend_from_slice(&at.to_le_bytes());
        dir.extend_from_slice(name.as_bytes());
    }
    let (start, len) = (out.len() as u32, dir.len() as u32);
    out.extend_from_slice(&dir);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// `text` as a data set part is stored: UTF-16 LE after a byte order mark.
fn utf16(text: &str) -> Vec<u8> {
    let mut out = vec![0xFF, 0xFE];
    out.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    out
}

/// The data set part of report 50204 as `Report.WordXmlPart` gives it on BC 29, with LF line ends as the compiler writes it
/// (Y17LayR2.docx, Y17LayoutExportTests.al:9).
fn part(extended: bool) -> String {
    part_as_word_xml_part(extended).replace("\r\n", "\n")
}

fn part_as_word_xml_part(extended: bool) -> String {
    match run_routine("Part", vec![Value::Boolean(extended)]) {
        Ok(Value::Text(t)) => t.to_string(),
        other => panic!("{other:?}"),
    }
}

fn docx(parts: &[(&str, String)]) -> Vec<(String, Vec<u8>)> {
    let mut entries = vec![("word/document.xml", b"<w:document/>".to_vec())];
    entries.extend(parts.iter().map(|(n, t)| (*n, utf16(t))));
    vec![("x/w.docx".into(), zip(&entries))]
}

// A legacy Word file whose two data set parts are the ones the compiler writes for the report exports its bytes
// (Y17LayoutExportTests.al:105-124: the entries customXml/item.xml and customXml/item2.xml of the file alc wrote).
#[test]
fn a_legacy_word_file_as_the_compiler_writes_it_exports_its_bytes() {
    let files = docx(&[("customXml/item.xml", part(false)), ("customXml/item2.xml", part(true))]);
    match export_of(50204, 1, files) {
        Ok(Value::Text(t)) => assert!(t.to_string().starts_with("Yes|"), "{t}"),
        other => panic!("{other:?}"),
    }
}

// A legacy Word file whose data set part names this report but a column it does not have (review probe A17_StaleWord).
#[test]
fn a_legacy_word_file_with_a_stale_column_is_refused() {
    let files = docx(&[
        (
            "customXml/item.xml",
            part(false)
                .replace(">N<", ">Stale<")
                .replace("<N>", "<Stale>")
                .replace("</N>", "</Stale>"),
        ),
        ("customXml/item2.xml", part(true)),
    ]);
    assert_rewrite_refused(export_of(50204, 1, files));
}

// A legacy Word file with no data set part (review probe A09_WordNoPart), and one with only the plain part.
#[test]
fn a_legacy_word_file_without_both_data_set_parts_is_refused() {
    assert_rewrite_refused(export_of(50204, 1, docx(&[])));
    assert_rewrite_refused(export_of(50204, 1, docx(&[("customXml/item.xml", part(false))])));
}

// A legacy Word file whose data set parts have CRLF line ends (no corpus file has them): refused.
#[test]
fn a_legacy_word_file_with_crlf_data_set_parts_is_refused() {
    let files = docx(&[
        ("customXml/item.xml", part_as_word_xml_part(false)),
        ("customXml/item2.xml", part_as_word_xml_part(true)),
    ]);
    assert!(part_as_word_xml_part(false).contains("\r\n"));
    assert_rewrite_refused(export_of(50204, 1, files));
}

// A legacy ExcelLayout file: no corpus case exports one (probe wanted, docs/probes-needed.md), so the loader does not hold it
// and the export is refused with that reason.
#[test]
fn a_legacy_excel_file_is_refused() {
    let r = export_of(50205, 2, vec![]);
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("ExcelLayout")),
        "{r:?}"
    );
}
