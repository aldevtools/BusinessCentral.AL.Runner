//! `Media.ExportStream` of the Layout of a Report Layout List row writes the bytes of the layout file the app holds,
//! answers true and leaves HasValue No, the same bytes a second time (report/lt-y17-layout-export/Y17LayoutExportTests.al:70
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
    let apps = vec![AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), SRC.into())],
    }];
    let (ws, mut l) = al_sema::lower::compile(apps, &Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    std::sync::Arc::make_mut(&mut l.program).apps[0].layout_files = vec![
        ("x/a.rdl".into(), b"RDL-BYTES".to_vec()),
        ("x/legacy.rdl".into(), b"LEGACY-BYTES".to_vec()),
    ];
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
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

// A layout whose file the app does not hold (a Microsoft app read from its package keeps no layout files) is refused.
#[test]
fn a_layout_whose_file_the_app_does_not_hold_is_refused() {
    let r = run(1);
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("layout file")),
        "{r:?}"
    );
}

// A layout of the legacy RDLCLayout property is a row named with the LayoutFile as written, and exports the file
// (Y17LayoutExportTests.al:105 Export_LegacyProperties).
#[test]
fn a_layout_of_the_legacy_property_is_a_row_that_exports_its_file() {
    match run_routine("Legacy", vec![]) {
        Ok(Value::Text(t)) => assert_eq!(t.to_string(), "1|./x/../x/legacy.rdl|Yes|LEGACY-BYTES|No"),
        other => panic!("{other:?}"),
    }
}
