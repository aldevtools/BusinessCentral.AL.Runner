//! ADR-040 amendment 2026-11-15: the Html of a Word layout is BC's own render only for the measured cells (al-vm
//! `report_render`). A save outside every cell keeps the opaque value of Part A even when the run has a renderer, and the
//! renderer is not asked.

use al_builtins::rendered_text::RENDERED_TEXT_MARK;
use al_host::render::{RenderFailure, RenderNeed, RenderUnavailable, ReportRenderer, WordMerge};
use al_vm::dotnet::DotNetConfig;
use al_vm::{Engine, Session};
use std::sync::{Arc, Mutex};

/// A renderer that has every need and records each call.
#[derive(Default)]
struct Recording {
    calls: Mutex<Vec<&'static str>>,
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
        Ok(b"<html>BC</html>".to_vec())
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
    procedure Body(): Text
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

#[test]
fn an_html_save_outside_every_measured_cell_keeps_the_opaque_value_and_asks_no_renderer() {
    let mut l = al_testkit::compile(SRC);
    // The app holds the layout file, so every input of a cell can be read; its bytes are in no cell.
    Arc::make_mut(&mut l.program).apps[0].layout_files =
        vec![("HrWord.docx".into(), b"PK-not-a-measured-layout".to_vec())];
    let renderer = Arc::new(Recording::default());
    let dotnet = DotNetConfig {
        renderer: renderer.clone(),
        ..DotNetConfig::default()
    };
    let target = al_version::Target::for_bc(al_version::BcVersion::new(27, 5));
    let engine = Engine::with_config(l.program.clone(), al_testkit::blocked(&l), Some(target), dotnet);
    let mut b = al_testkit::Built {
        session: Session::new(engine),
        lowered: l,
    };
    let got = al_testkit::text(b.run_in("HR Tests", "Body"));
    assert!(got.starts_with("Yes|"), "{got}");
    assert!(got.contains(RENDERED_TEXT_MARK), "Part A's opaque value: {got:?}");
    assert!(!got.contains("<html>BC</html>"), "{got}");
    assert!(
        renderer.calls.lock().unwrap().is_empty(),
        "{:?}",
        renderer.calls.lock().unwrap()
    );
}
