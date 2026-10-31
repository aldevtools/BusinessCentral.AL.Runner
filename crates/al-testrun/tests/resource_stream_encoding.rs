//! `NavApp.GetResource(Name, InStream, Encoding)` with UTF16, Windows, MSDos and no encoding (al-runner issue 6492: an ISV
//! test library reads UTF-16LE XML configuration resources with a byte order mark this way). The values are the ones the
//! corpus pins (session/Y18NavAppResourceEncodings.al).

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

const MAIN: &str = r#"
codeunit 50830 "T"
{
    Subtype = Test;

    [Test]
    procedure ReadUtf16Resource()
    var
        S: InStream;
        Line: Text;
    begin
        NavApp.GetResource('cfg.xml', S, TextEncoding::UTF16);
        S.ReadText(Line);
        if Line <> '<?xml version="1.0"?>' then Error('first line %1', Line);
        S.ReadText(Line);
        if Line <> '<cfg name="Zürich"/>' then Error('second line %1', Line);
    end;

    [Test]
    procedure ReadWindowsResource()
    var
        S: InStream;
        Line: Text;
        N: Integer;
    begin
        NavApp.GetResource('w.txt', S, TextEncoding::Windows);
        N := S.ReadText(Line);
        if (N <> 6) or (Line <> 'Zürich') then Error('first line %1 %2', N, Line);
        S.ReadText(Line);
        if Line <> 'Größe 5 €' then Error('second line %1', Line);
    end;

    [Test]
    procedure ReadWithoutEncodingIsMsDos()
    var
        S: InStream;
        Line: Text;
    begin
        // FC reads as 253 as with TextEncoding::MSDos, not as the UTF-8 decoding would.
        NavApp.GetResource('w.txt', S);
        S.ReadText(Line);
        if Line[2] <> 253 then Error('second character %1', Line[2]);
    end;

    [Test]
    procedure Utf16OverUnpairedSurrogate()
    var
        S: InStream;
        Line: Text;
    begin
        NavApp.GetResource('w.txt', S, TextEncoding::UTF16);
        S.ReadText(Line);
    end;
}
"#;

fn utf16_with_bom(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    bytes
}

fn run(resources: Vec<(String, Vec<u8>)>) -> CorpusRuntime {
    CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), MAIN.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[Manifest {
            publisher: "Me".into(),
            version: "1.0.0.0".into(),
            resources,
            ..Default::default()
        }],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
}

/// 'Zürich' CR LF 'Größe 5 €' in Windows-1252.
const WINDOWS: &[u8] = b"Z\xFCrich\r\nGr\xF6\xDFe 5 \x80";

fn outcome<'a>(rt: &'a CorpusRuntime, test: &str) -> &'a al_testrun::runtime::TestResult {
    rt.results()
        .iter()
        .find(|r| r.test.contains(test))
        .expect("the test ran")
}

#[test]
fn resources_read_with_every_text_encoding() {
    let rt = run(vec![
        (
            "cfg.xml".into(),
            utf16_with_bom("<?xml version=\"1.0\"?>\r\n<cfg name=\"Zürich\"/>\r\n"),
        ),
        ("w.txt".into(), WINDOWS.to_vec()),
    ]);
    for test in ["ReadUtf16Resource", "ReadWindowsResource", "ReadWithoutEncodingIsMsDos"] {
        let r = outcome(&rt, test);
        assert_eq!(r.outcome, Outcome::Pass, "{test}: {:?}", r.outcome);
        // Pinned by session/Y18NavAppResourceEncodings.al: a clean pass, not a flagged one.
        assert!(r.assumptions.is_empty(), "{test}: {:?}", r.assumptions);
    }
    // BC raises InvalidStreamData here, but the bytes also have an odd length: which of the two raises is not pinned.
    let r = outcome(&rt, "Utf16OverUnpairedSurrogate");
    assert!(
        matches!(&r.outcome, Outcome::NotRun(w) if w.contains("unpaired surrogate")),
        "{:?}",
        r.outcome
    );
}
