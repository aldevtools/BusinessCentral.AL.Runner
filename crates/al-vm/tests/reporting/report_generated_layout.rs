//! `Report.WordLayout` as a statement for the one Word layout of a report whose file the app does not hold. An app built from
//! source: the AL compiler generates the missing file, BC fills the stream without an error
//! (reports/layouts-saveas/UserDefinedWordExcelLayoutRows.al:60, UserDefinedLayoutAppIdAndNames.al:146-157), and the runner
//! carries it unread: any look at its bytes is refused. An app read from a package holds its layout files in the package; a file
//! the runner does not hold there is not generated, and the call is refused as before.

use al_builtins::native::Signal;
use al_vm::{Engine, Session};

const SRC: &str = r#"
report 50950 "GL Rep"
{
    DefaultRenderingLayout = W;
    dataset { dataitem(I; "GL Row") { column(N; Id) { } } }
    rendering { layout(W) { Type = Word; LayoutFile = './x/missing.docx'; } }
}
table 50951 "GL Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50952 "GL Tests"
{
    procedure ReadGenerated(): Text
    var
        InS: InStream;
        T: Text;
    begin
        Report.WordLayout(Report::"GL Rep", InS);
        InS.ReadText(T);
        exit(T);
    end;
}
"#;

struct NoFiles;

impl al_ir::LayoutSource for NoFiles {
    fn file(&self, _path: &str) -> Option<Vec<u8>> {
        None
    }
}

fn run(package: bool) -> Result<al_builtins::value::Value, Signal> {
    let mut lowered = al_testkit::compile(SRC);
    if package {
        for app in &mut std::sync::Arc::make_mut(&mut lowered.program).apps {
            app.layout_source = Some(al_ir::LayoutSourceRef(std::sync::Arc::new(NoFiles)));
        }
    }
    let engine = Engine::new(lowered.program.clone(), al_testkit::blocked(&lowered));
    let mut session = Session::new(engine);
    let oi = lowered
        .program
        .objects
        .iter()
        .position(|o| o.name == "GL Tests")
        .expect("the test codeunit");
    let ri = lowered.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "ReadGenerated")
        .expect("the routine");
    let inst = session.instance(oi);
    session.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

/// The generated file's bytes are not pinned: reading them is refused with the generated-layout reason (stream.rs
/// `render_pending_error`).
#[test]
fn reading_a_generated_layout_file_is_refused() {
    match run(false) {
        Err(Signal::Unsupported(m)) if m.contains("layout file the AL compiler generates") => {}
        other => panic!("expected the generated-layout refusal, got {other:?}"),
    }
}

/// An app read from a package: the compiler did not generate the file, the call keeps the refusal it had.
#[test]
fn a_package_layout_file_the_runner_does_not_hold_is_refused() {
    match run(true) {
        Err(Signal::Unsupported(m)) if m.contains("Report.WordLayout of a layout whose file the app does not hold") => {
        }
        other => panic!("expected the not-held refusal, got {other:?}"),
    }
}
