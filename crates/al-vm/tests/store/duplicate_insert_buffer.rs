//! An asserterror body that ends with a pending duplicate-key Insert, the 5th or a later insert into its table in the transaction,
//! is refused, not failed with "An error was expected" (al-runner issue 7407). The corpus pins the deferred error for a few inserts in
//! a row (errors/last-error/InsertDuplicateKeyExcludeFlag.al, record/TestRecordRuntimeTextsAndViews.al); whether BC raises it inside
//! the asserterror once the insert buffer of the table fills ("up to 5 rows will be buffered in the table queue",
//! BufferedInsertEnabled) is queued as a probe (corpus pull request 1279), so the runner does not answer it.

use al_builtins::native::Signal;
use al_testkit::run_in;

const SRC: &str = r#"
table 50891 "DB Row" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 50891 C
{
    local procedure Insert(First: Integer; Last: Integer)
    var R: Record "DB Row"; I: Integer;
    begin
        for I := First to Last do begin R.Code := 'K' + Format(I); R.Insert(); end;
    end;
    local procedure DupAfter(Rows: Integer): Text
    var R: Record "DB Row";
    begin
        R.DeleteAll();
        Insert(1, Rows);
        asserterror Insert(1, 1);
        exit(GetLastErrorText());
    end;
    procedure DupAfter1(): Text begin exit(DupAfter(1)); end;
    procedure DupAfter2(): Text begin exit(DupAfter(2)); end;
    procedure DupAfter3(): Text begin exit(DupAfter(3)); end;
    procedure DupAfter4(): Text begin exit(DupAfter(4)); end;
    procedure DupThenMany(): Text
    var R: Record "DB Row";
    begin
        R.DeleteAll();
        Insert(1, 1);
        asserterror begin Insert(1, 1); Insert(2, 9); end;
        exit(GetLastErrorText());
    end;
}
"#;

#[test]
fn a_duplicate_among_the_first_four_inserts_stays_a_failed_asserterror() {
    for rows in 1..=3 {
        let r = run_in(SRC, "C", &format!("DupAfter{rows}"));
        assert!(
            matches!(&r, Err(Signal::Error(e)) if e.code == "AssertError"),
            "{rows}: {r:?}"
        );
    }
}

#[test]
fn a_pending_duplicate_that_is_the_fifth_insert_is_refused() {
    let r = run_in(SRC, "C", "DupAfter4");
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("insert buffer is full")),
        "{r:?}"
    );
}

#[test]
fn inserts_after_a_pending_duplicate_that_fill_the_buffer_are_refused() {
    let r = run_in(SRC, "C", "DupThenMany");
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("insert buffer is full")),
        "{r:?}"
    );
}
