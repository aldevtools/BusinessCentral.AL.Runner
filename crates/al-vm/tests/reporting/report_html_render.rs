//! ADR-040 amendment 2026-11-15: the Html of a Word layout is BC's own render only for the measured shapes (al-vm
//! `report_render::shape`). A save of a measured shape on a measured major writes the renderer's Html as plain text; a save
//! that differs in one fact keeps the opaque value of Part A even when the run has a renderer.

use al_builtins::rendered_text::RENDERED_TEXT_MARK;
use al_host::render::{RenderFailure, RenderNeed, RenderUnavailable, ReportRenderer, WordMerge};
use al_vm::dotnet::DotNetConfig;
use al_vm::{Engine, Session};
use std::sync::{Arc, Mutex};

/// A renderer that has every need and records each call.
struct Recording {
    calls: Mutex<Vec<&'static str>>,
    html: Vec<u8>,
}

impl Recording {
    fn writing(generator: &str) -> Recording {
        Recording {
            calls: Mutex::new(Vec::new()),
            html: format!(
                "<html><head><meta name=\"generator\" content=\"{generator}\" /></head><body>BC</body></html>"
            )
            .into_bytes(),
        }
    }
}

impl ReportRenderer for Recording {
    fn availability(&self, _need: RenderNeed) -> Result<(), RenderUnavailable> {
        Ok(())
    }
    fn merge_word(&self, _req: &WordMerge) -> Result<Vec<u8>, RenderFailure> {
        self.calls.lock().unwrap().push("merge");
        Ok(b"PK".to_vec())
    }
    fn word_to_pdf(&self, _docx: &[u8]) -> Result<Vec<u8>, RenderFailure> {
        self.calls.lock().unwrap().push("pdf");
        Ok(Vec::new())
    }
    fn word_to_html(&self, _docx: &[u8]) -> Result<Vec<u8>, RenderFailure> {
        self.calls.lock().unwrap().push("html");
        Ok(self.html.clone())
    }
}

const SRC: &str = r#"
table 50980 "HR Row"
{
    fields { field(1; Id; Integer) { } field(2; Txt; Text[30]) { } }
    keys { key(PK; Id) { } }
}

report 50981 "HR Word"
{
    UsageCategory = None;
    DefaultRenderingLayout = WordOne;
    dataset { dataitem(T; "HR Row") { column(Id; Id) { } column(Txt; Txt) { } } }
    rendering { layout(WordOne) { Type = Word; LayoutFile = 'HrWord.docx'; } }
}

codeunit 50982 "HR Tests"
{
    procedure One(): Text begin exit(Body(1)); end;
    procedure Two(): Text begin exit(Body(2)); end;

    local procedure Body(N: Integer): Text
    var
        R: Record "HR Row";
        TempBlob: Codeunit "HR Blob";
        OutS: OutStream;
        InS: InStream;
        T: Text;
        Ok: Boolean;
    begin
        R.DeleteAll();
        R.Id := 1; R.Txt := 'RXROW1'; R.Insert();
        if N = 2 then begin R.Id := 2; R.Txt := 'RXROW2'; R.Insert(); end;
        TempBlob.Out(OutS);
        Ok := Report.SaveAs(Report::"HR Word", '', ReportFormat::Html, OutS);
        TempBlob.In(InS);
        InS.Read(T);
        exit(Format(Ok) + '|' + T);
    end;
}

codeunit 50983 "HR Blob"
{
    var B: Record "HR Store" temporary;
    procedure Out(var O: OutStream) begin B.Id := 1; B.Data.CreateOutStream(O, TextEncoding::UTF8); end;
    procedure In(var I: InStream) begin B.Data.CreateInStream(I, TextEncoding::UTF8); end;
}

table 50984 "HR Store"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { } }
}
"#;

/// A stored (uncompressed) zip archive of `entries`; the reader checks no CRC.
fn stored_zip(entries: &[(&str, &str)]) -> Vec<u8> {
    let (mut out, mut dir) = (Vec::new(), Vec::new());
    for (name, data) in entries {
        let offset = out.len() as u32;
        let head = |sig: u32, central: bool| {
            let mut h = sig.to_le_bytes().to_vec();
            if central {
                h.extend_from_slice(&20u16.to_le_bytes());
            }
            // version, flags, method (stored), time, date, CRC
            h.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            h.extend_from_slice(&(data.len() as u32).to_le_bytes());
            h.extend_from_slice(&(data.len() as u32).to_le_bytes());
            h.extend_from_slice(&(name.len() as u16).to_le_bytes());
            h.extend_from_slice(&0u16.to_le_bytes());
            if central {
                // comment length, disk, internal and external attributes
                h.extend_from_slice(&[0; 10]);
                h.extend_from_slice(&offset.to_le_bytes());
            }
            h.extend_from_slice(name.as_bytes());
            h
        };
        out.extend(head(0x0403_4b50, false));
        out.extend_from_slice(data.as_bytes());
        dir.extend(head(0x0201_4b50, true));
    }
    let at = out.len() as u32;
    out.extend_from_slice(&dir);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(dir.len() as u32).to_le_bytes());
    out.extend_from_slice(&at.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}

/// A layout of the measured shape for report `HR Word`: a row section over `T` with the Text column `Txt` bound inside;
/// `extra` goes into the first paragraph.
fn layout(extra: &str) -> Vec<u8> {
    let ns = "xmlns:ns0='urn:microsoft-dynamics-nav/reports/HR_Word/50981/'";
    let doc = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><w:document><w:body><w:p><w:r>{extra}<w:t>RXSTATIC</w:t></w:r></w:p>\
         <w:sdt><w:sdtPr><w:dataBinding w:prefixMappings=\"{ns}\" w:xpath=\"/ns0:NavWordReportXmlPart[1]/ns0:T\" />\
         <w15:repeatingSection /></w:sdtPr><w:sdtContent><w:sdt><w:sdtPr><w15:repeatingSectionItem /></w:sdtPr><w:sdtContent>\
         <w:sdt><w:sdtPr><w:dataBinding w:prefixMappings=\"{ns}\" w:xpath=\"/ns0:NavWordReportXmlPart[1]/ns0:T[1]/ns0:Txt[1]\" />\
         </w:sdtPr><w:sdtContent><w:p><w:r><w:t>Txt</w:t></w:r></w:p></w:sdtContent></w:sdt></w:sdtContent></w:sdt>\
         </w:sdtContent></w:sdt></w:body></w:document>"
    );
    stored_zip(&[
        ("[Content_Types].xml", "<Types/>"),
        ("word/document.xml", &doc),
        ("word/settings.xml", "<w:settings><w:compat /></w:settings>"),
    ])
}

/// Runs `routine` of `HR Tests` on BC `major` with `layout` as the app's layout file and `renderer`.
fn run(src: &str, major: u16, layout: Vec<u8>, renderer: &Arc<Recording>, routine: &str) -> String {
    let dotnet = DotNetConfig {
        renderer: renderer.clone(),
        ..DotNetConfig::default()
    };
    al_testkit::text(run_with(src, major, layout, dotnet, routine))
}

fn run_with(
    src: &str,
    major: u16,
    layout: Vec<u8>,
    dotnet: DotNetConfig,
    routine: &str,
) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let mut l = al_testkit::compile(src);
    Arc::make_mut(&mut l.program).apps[0].layout_files = vec![("HrWord.docx".into(), layout)];
    let target = al_version::Target::for_bc(al_version::BcVersion::new(major, if major == 29 { 0 } else { 5 }));
    let engine = Engine::with_config(l.program.clone(), al_testkit::blocked(&l), Some(target), dotnet);
    let mut b = al_testkit::Built {
        session: Session::new(engine),
        lowered: l,
    };
    b.run_in("HR Tests", routine)
}

const MEASURED: &str = "Aspose.Words for .NET 26.8.0";

#[test]
fn a_measured_shape_on_a_measured_major_writes_the_renderers_html_as_plain_text() {
    let r = Arc::new(Recording::writing(MEASURED));
    let got = run(SRC, 29, layout(""), &r, "Two");
    assert!(
        got.starts_with("Yes|<html>") && got.contains("<body>BC</body>"),
        "{got:?}"
    );
    assert!(!got.contains(RENDERED_TEXT_MARK), "{got:?}");
    assert_eq!(*r.calls.lock().unwrap(), vec!["merge", "html"]);
}

#[test]
fn a_converter_version_no_cell_measured_on_the_major_keeps_the_opaque_value() {
    // The pinned BC 27.5.46862.56249 carries Aspose.Words 26.8.0; the probe measured 26.10.0 on BC 27.
    let r = Arc::new(Recording::writing(MEASURED));
    let got = run(SRC, 27, layout(""), &r, "Two");
    assert!(got.starts_with("Yes|") && got.contains(RENDERED_TEXT_MARK), "{got:?}");
    assert!(!got.contains("<body>BC</body>"), "{got:?}");
}

#[test]
fn a_save_that_differs_in_one_fact_keeps_the_opaque_value_and_asks_no_renderer() {
    let caption = SRC.replace(
        "    UsageCategory = None;",
        "    UsageCategory = None;\n    Caption = 'HR Caption';",
    );
    let cases: [(&str, u16, Vec<u8>, &str); 6] = [
        // A major no cell measured.
        ("major 26", 26, layout(""), "Two"),
        // One row: the cells have two.
        ("one row", 29, layout(""), "One"),
        // An element no measured layout has (a tab), and bytes that are no layout.
        ("a tab", 29, layout("<w:tab />"), "Two"),
        ("no zip", 29, b"PK-not-a-measured-layout".to_vec(), "Two"),
        // A report caption: no measured report has one.
        ("caption", 29, layout(""), "Two"),
        // The column binding outside the row section.
        (
            "unbound section",
            29,
            layout(
                "<w:sdt><w:sdtPr><w:dataBinding w:prefixMappings=\"xmlns:ns0='urn:microsoft-dynamics-nav/reports/HR_Word/50981/'\" w:xpath=\"/ns0:NavWordReportXmlPart[1]/ns0:T[1]/ns0:Txt[1]\" /></w:sdtPr></w:sdt>",
            ),
            "Two",
        ),
    ];
    for (what, major, bytes, routine) in cases {
        let r = Arc::new(Recording::writing(MEASURED));
        let src = if what == "caption" { caption.as_str() } else { SRC };
        let got = run(src, major, bytes, &r, routine);
        assert!(
            got.starts_with("Yes|") && got.contains(RENDERED_TEXT_MARK),
            "{what}: {got:?}"
        );
        assert!(
            r.calls.lock().unwrap().is_empty(),
            "{what}: {:?}",
            r.calls.lock().unwrap()
        );
    }
}

/// A layout for report `HR Word` whose row section over `T` binds each column of `columns` inside it.
fn layout_binding(columns: &[&str]) -> Vec<u8> {
    let ns = "xmlns:ns0='urn:microsoft-dynamics-nav/reports/HR_Word/50981/'";
    let cells: String = columns
        .iter()
        .map(|c| {
            format!(
                "<w:sdt><w:sdtPr><w:dataBinding w:prefixMappings=\"{ns}\" \
                 w:xpath=\"/ns0:NavWordReportXmlPart[1]/ns0:T[1]/ns0:{c}[1]\" /></w:sdtPr>\
                 <w:sdtContent><w:p><w:r><w:t>{c}</w:t></w:r></w:p></w:sdtContent></w:sdt>"
            )
        })
        .collect();
    let doc = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><w:document><w:body>\
         <w:sdt><w:sdtPr><w:dataBinding w:prefixMappings=\"{ns}\" w:xpath=\"/ns0:NavWordReportXmlPart[1]/ns0:T\" />\
         <w15:repeatingSection /></w:sdtPr><w:sdtContent><w:sdt><w:sdtPr><w15:repeatingSectionItem /></w:sdtPr><w:sdtContent>\
         {cells}</w:sdtContent></w:sdt></w:sdtContent></w:sdt></w:body></w:document>"
    );
    stored_zip(&[
        ("[Content_Types].xml", "<Types/>"),
        ("word/document.xml", &doc),
        ("word/settings.xml", "<w:settings><w:compat /></w:settings>"),
    ])
}

/// `SRC` with the columns of RX Hash Bound (corpus pull request 1415, RxHtmlHashTests.al:50): a Decimal expression with
/// `AutoFormatType = 1` and a Date expression after the two field columns; `amount` replaces the Decimal column's text.
fn bound_src(amount: &str) -> String {
    SRC.replace(
        "column(Txt; Txt) { } } }",
        &format!("column(Txt; Txt) {{ }} {amount} column(Day; DMY2Date(1, 2, 2026)) {{ }} }} }}"),
    )
}

const AMOUNT: &str = "column(Amount; Id * 1234.5) { AutoFormatType = 1; }";
const BOUND: [&str; 4] = ["Id", "Txt", "Amount", "Day"];

#[test]
fn the_four_bound_columns_of_the_measured_layout_render_with_one_row() {
    let r = Arc::new(Recording::writing(MEASURED));
    let got = run(&bound_src(AMOUNT), 29, layout_binding(&BOUND), &r, "One");
    assert!(
        got.starts_with("Yes|<html>") && !got.contains(RENDERED_TEXT_MARK),
        "{got:?}"
    );
    assert_eq!(*r.calls.lock().unwrap(), vec!["merge", "html"]);
}

#[test]
fn a_bound_column_whose_source_or_format_differs_keeps_the_opaque_value() {
    let cases: [(&str, String, &[&str]); 5] = [
        // Another AutoFormatType, DecimalPlaces beside it, an AutoFormatExpression.
        (
            "type 2",
            bound_src("column(Amount; Id * 1234.5) { AutoFormatType = 2; }"),
            &BOUND,
        ),
        (
            "places",
            bound_src("column(Amount; Id * 1234.5) { AutoFormatType = 1; DecimalPlaces = 2 : 2; }"),
            &BOUND,
        ),
        (
            "expression",
            bound_src("column(Amount; Id * 1234.5) { AutoFormatType = 1; AutoFormatExpression = 'EUR'; }"),
            &BOUND,
        ),
        // An Integer expression in place of the Decimal one.
        (
            "integer",
            bound_src("column(Amount; Id * 2) { AutoFormatType = 1; }"),
            &BOUND,
        ),
        // The Date column not bound.
        ("unbound date", bound_src(AMOUNT), &["Id", "Txt", "Amount"]),
    ];
    for (what, src, columns) in cases {
        let r = Arc::new(Recording::writing(MEASURED));
        let got = run(&src, 29, layout_binding(columns), &r, "One");
        assert!(
            got.starts_with("Yes|") && got.contains(RENDERED_TEXT_MARK),
            "{what}: {got:?}"
        );
        assert!(r.calls.lock().unwrap().is_empty(), "{what}");
    }
}

#[test]
fn without_a_renderer_a_measured_save_keeps_the_opaque_value_unless_the_renderer_is_required() {
    let got = al_testkit::text(run_with(SRC, 29, layout(""), DotNetConfig::default(), "Two"));
    assert!(got.starts_with("Yes|") && got.contains(RENDERED_TEXT_MARK), "{got:?}");
    let required = DotNetConfig {
        require_renderer: true,
        ..DotNetConfig::default()
    };
    match run_with(SRC, 29, layout(""), required.clone(), "Two") {
        Err(s @ al_builtins::native::Signal::Unsupported(_)) => {
            let why = format!("{s:?}");
            assert!(
                why.contains("--require-report-renderer") && why.contains("HR Word"),
                "{why}"
            )
        }
        other => panic!("{}", al_testkit::text(other)),
    }
    // A save that is not of a measured shape needs no renderer: the flag changes nothing there.
    let got = al_testkit::text(run_with(SRC, 29, layout(""), required, "One"));
    assert!(got.starts_with("Yes|") && got.contains(RENDERED_TEXT_MARK), "{got:?}");
}
