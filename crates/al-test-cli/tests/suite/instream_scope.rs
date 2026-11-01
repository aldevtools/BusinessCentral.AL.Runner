//! al-runner issue 5688: an InStream handed back from a routine whose local owns its bytes. The pinned shapes read as empty
//! (streams/lt-dc-stream-leads/DcInStreamScope.al:107-166); the others are refused when they run, never because a test can
//! reach them (a page handler refused for `OCR Service Mgt.RsoRequest`, which the test never runs).

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62790 "IS Row" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } }
table 62791 "IS TB" { TableType = Temporary; fields { field(1; Id; Integer) { } field(2; Blob; Blob) { } } }
codeunit 4100 "Temp Blob"
{
    var R: Record "IS TB" temporary;
    procedure CreateInStream(var S: InStream) begin R.Blob.CreateInStream(S); end;
}
codeunit 62792 "IS Web Mgt"
{
    procedure CreateInstream(var InStr: InStream) var TempBlob: Codeunit "Temp Blob"; begin TempBlob.CreateInStream(InStr); end;
}
codeunit 62793 "IS Ocr"
{
    procedure RsoRequest(var S: InStream) var Web: Codeunit "IS Web Mgt"; begin Web.CreateInstream(S); end;
    procedure NotTemp(var S: InStream) var R: Record "IS Row"; begin R.Data.CreateInStream(S); end;
    procedure Maybe(N: Integer) var S: InStream; begin if N = 1 then NotTemp(S); end;
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62794 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

#[test]
fn a_stream_of_a_local_is_refused_only_when_the_unpinned_shape_runs() {
    let o = outcomes(
        "[Test] procedure WebEmpty() var Ocr: Codeunit \"IS Ocr\"; S: InStream; T: Text; begin
             Ocr.RsoRequest(S); S.ReadText(T); if (T <> '') or not S.EOS() then Error('not empty'); end;
         [Test] procedure NotReached() var Ocr: Codeunit \"IS Ocr\"; begin Ocr.Maybe(0); end;
         [Test] procedure Reached() var Ocr: Codeunit \"IS Ocr\"; begin Ocr.Maybe(1); end;",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0]);
    assert_eq!(o[1].1, Outcome::Pass, "{:?}", o[1]);
    assert!(
        matches!(&o[2].1, Outcome::NotRun(why) if why.contains("out of scope")),
        "{:?}",
        o[2]
    );
}
