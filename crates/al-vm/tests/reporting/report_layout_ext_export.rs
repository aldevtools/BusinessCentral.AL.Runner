//! `Media.ExportStream` of the Layout of a Report Layout List row of a layout a reportextension declares: the bytes of the file
//! the extension's app holds (report/lt-y67-layout-ext/Y67LayoutExtExport.al:134 Export_ExtensionLayouts_OwnReport, :187
//! Export_ExtensionLayout_BaseApplicationReport). A Word file whose data set part names another report is refused: the compiler
//! rewrites that part at build, and BC exports the rewritten bytes (the Y67 files name report 93617, the hashes BC answered
//! are not those of the files).

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
report 50300 "RX Base" { DefaultRenderingLayout = Base1;
  dataset { dataitem(I; Integer) { column(N; Number) { } } }
  rendering { layout(Base1) { Type = RDLC; LayoutFile = './x/base.rdl'; } } }
reportextension 50301 "RX Ext" extends "RX Base" {
  rendering {
    layout(ExtRdl) { Type = RDLC; LayoutFile = './x/ext.rdl'; Caption = 'ext rdlc'; }
    layout("Ext.Dotted.docx") { Type = Word; LayoutFile = './x/own.docx'; }
    layout(ExtStale) { Type = Word; LayoutFile = './x/stale.docx'; } } }
table 60982 B { fields { field(1; K; Integer) { } field(2; Src; Blob) { } } keys { key(PK; K) { } } }
codeunit 50302 C {
    procedure Export(LayoutName: Text): Text
    var L: Record "Report Layout List"; B: Record B; outs: OutStream; ins: InStream; Ok: Boolean;
    begin
        L.SetRange("Report ID", 50300);
        L.SetRange(Name, LayoutName);
        L.FindFirst();
        B.Src.CreateOutStream(outs);
        Ok := L.Layout.ExportStream(outs);
        B.Insert();
        B.CalcFields(Src);
        exit(Format(L.Count()) + '|' + Format(L."Layout Format") + '|' + Format(Ok) + '|' + Format(B.Src.Length()) + '|' + Format(L.Layout.HasValue()));
    end;
}
"#;

/// A zip archive of stored entries.
fn zip(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let (mut out, mut dir) = (Vec::new(), Vec::new());
    for (name, data) in entries {
        let offset = out.len() as u32;
        let head = |sig: u32, central: bool| {
            let mut h = sig.to_le_bytes().to_vec();
            if central {
                h.extend_from_slice(&20u16.to_le_bytes());
            }
            h.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            h.extend_from_slice(&(data.len() as u32).to_le_bytes());
            h.extend_from_slice(&(data.len() as u32).to_le_bytes());
            h.extend_from_slice(&(name.len() as u16).to_le_bytes());
            h.extend_from_slice(&0u16.to_le_bytes());
            if central {
                h.extend_from_slice(&[0; 10]);
                h.extend_from_slice(&offset.to_le_bytes());
            }
            h
        };
        out.extend(head(0x0403_4b50, false));
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        dir.extend(head(0x0201_4b50, true));
        dir.extend_from_slice(name.as_bytes());
    }
    let at = out.len() as u32;
    let n = entries.len() as u16;
    out.extend_from_slice(&dir);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&(dir.len() as u32).to_le_bytes());
    out.extend_from_slice(&at.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}

fn docx(report_id: u32) -> Vec<u8> {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"utf-16\"?>\r\n<NavWordReportXmlPart xmlns=\"urn:microsoft-dynamics-nav/reports/RX_Base/{report_id}/\"></NavWordReportXmlPart>"
    );
    let mut part = vec![0xFF, 0xFE];
    for u in xml.encode_utf16() {
        part.extend_from_slice(&u.to_le_bytes());
    }
    zip(&[
        ("word/document.xml", b"<w:document/>".to_vec()),
        ("customXml/item.xml", part),
    ])
}

fn export(name: &str) -> Result<Value, Signal> {
    let apps = vec![AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), SRC.into())],
    }];
    let (ws, mut l) = al_sema::lower::compile(apps, &Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    std::sync::Arc::make_mut(&mut l.program).apps[0].layout_files = vec![
        ("x/base.rdl".into(), b"BASE".to_vec()),
        ("x/ext.rdl".into(), b"EXT-RDLC".to_vec()),
        ("x/own.docx".into(), docx(50300)),
        ("x/stale.docx".into(), docx(93617)),
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
        .position(|r| r.name == "Export")
        .unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![Value::Text(name.into())]).map(|(v, _)| v)
}

fn text(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(t)) => t.to_string(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_rdlc_layout_of_a_reportextension_exports_the_file_its_app_holds() {
    assert_eq!(text(export("ExtRdl")), "1|RDLC|Yes|8|No");
}

#[test]
fn a_word_layout_of_a_reportextension_with_a_dotted_name_exports_its_file() {
    assert_eq!(
        text(export("Ext.Dotted.docx")),
        format!("1|Word|Yes|{}|No", docx(50300).len())
    );
}

#[test]
fn the_report_s_own_layout_still_exports() {
    assert_eq!(text(export("Base1")), "1|RDLC|Yes|4|No");
}

#[test]
fn a_word_file_whose_data_set_part_names_another_report_is_refused() {
    let r = export("ExtStale");
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("names report 93617")),
        "{r:?}"
    );
}
