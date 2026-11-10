//! A duplicate-key Insert is pending until the table's insert queue is sent: with its 5th row, a Commit or the next access
//! (al-runner issue 7407). Measured on BC 27.5, 28.5 and 29.0, corpus pull request 1279,
//! record/duplicate-insert/Y7407DuplicateInsertBuffer.al: Dup_2ndInsert_OfFirstRow to Dup_4thInsert_OfFirstRow (pending),
//! Dup_5thInsert_OfFirstRow (raises at the Insert), Dup_6thInsert_OfFirstRow and Dup_7thInsert_* (pending),
//! Dup_5thInsert_OfCommittedRow_CommitAfter4 (pending), Report_FourRowsTwice_NoRead and Report_FiveRowsTwice_NoRead (raise inside the
//! asserterror), Report_FourRowsTwice_CommitBetween (nothing raises before the next read).

use al_builtins::native::Signal;
use al_testkit::run_in;

const SRC: &str = r#"
table 50893 "DB Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50891 "DB Row" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 50892 "DB Count"
{
    SingleInstance = true;
    var N: Integer;
    procedure Add() begin N += 1; end;
    procedure Rows(): Integer begin exit(N); end;
}
xmlport 50891 "DB Port"
{
    Format = Xml;
    Direction = Import;
    schema
    {
        textelement(Root)
        {
            tableelement(T; "DB Row")
            {
                fieldelement(Code; T.Code) { }
                trigger OnBeforeInsertRecord()
                var Count: Codeunit "DB Count";
                begin
                    Count.Add();
                end;
            }
        }
    }
}
codeunit 50891 C
{
    local procedure Insert(First: Integer; Last: Integer)
    var R: Record "DB Row"; I: Integer;
    begin
        for I := First to Last do begin R.Code := 'K' + Format(I); R.Insert(); end;
    end;
    // Rows inserts, a Commit after CommitAfter of them (0: none), then the key K1 again inside an asserterror that does not read.
    local procedure DupAfter(Rows: Integer; CommitAfter: Integer): Text
    var R: Record "DB Row";
    begin
        R.DeleteAll();
        Commit();
        Insert(1, Rows);
        if CommitAfter > 0 then Commit();
        asserterror Insert(1, 1);
        exit(GetLastErrorText());
    end;
    procedure Dup2(): Text begin exit(DupAfter(1, 0)); end;
    procedure Dup4(): Text begin exit(DupAfter(3, 0)); end;
    procedure Dup5(): Text begin exit(DupAfter(4, 0)); end;
    procedure Dup6(): Text begin exit(DupAfter(5, 0)); end;
    procedure Dup7(): Text begin exit(DupAfter(6, 0)); end;
    procedure Dup5AfterCommit(): Text begin exit(DupAfter(4, 4)); end;
    procedure FourTwice(): Text
    var R: Record "DB Row";
    begin
        R.DeleteAll();
        Commit();
        Insert(1, 4);
        asserterror Insert(1, 4);
        exit(GetLastErrorText());
    end;
    procedure FiveTwice(): Text
    var R: Record "DB Row";
    begin
        R.DeleteAll();
        Commit();
        Insert(1, 5);
        asserterror Insert(1, 5);
        exit(GetLastErrorText());
    end;
    procedure FourCommitFour(): Text
    var R: Record "DB Row";
    begin
        R.DeleteAll();
        Commit();
        Insert(1, 4);
        Commit();
        Insert(1, 4);
        asserterror R.Count();
        exit(GetLastErrorText());
    end;
    // K1, K2, K1 again (pending), then K3 and K4: K4 is the 5th queued row and is a valid row.
    procedure ValidRowSends(): Text
    var R: Record "DB Row";
    begin
        R.DeleteAll();
        Commit();
        Insert(1, 2);
        Insert(1, 1);
        asserterror Insert(3, 4);
        exit(GetLastErrorText());
    end;
    // Seven rows through an XmlPort, the 5th a duplicate of the 1st: how many rows reached OnBeforeInsertRecord.
    procedure PortImport(): Text
    var R: Record "DB Row"; Count: Codeunit "DB Count"; TempBlob: Record "DB Blob"; OutStr: OutStream; InStr: InStream; Ok: Boolean;
    begin
        R.DeleteAll();
        Commit();
        TempBlob.Data.CreateOutStream(OutStr, TextEncoding::UTF8);
        OutStr.WriteText('<Root><T><Code>K1</Code></T><T><Code>K2</Code></T><T><Code>K3</Code></T><T><Code>K4</Code></T>' +
            '<T><Code>K1</Code></T><T><Code>K5</Code></T><T><Code>K6</Code></T></Root>');
        TempBlob.Data.CreateInStream(InStr, TextEncoding::UTF8);
        Ok := TryImport(InStr);
        // The duplicate is pending after the import: the next read of the table raises it.
        asserterror R.Count();
        exit(Format(Count.Rows()) + '|' + Format(Ok) + '|' + GetLastErrorText());
    end;
    [TryFunction]
    local procedure TryImport(var InStr: InStream)
    begin
        Xmlport.Import(Xmlport::"DB Port", InStr);
    end;
    procedure ReadBetween(): Text
    var R: Record "DB Row";
    begin
        R.DeleteAll();
        Commit();
        Insert(1, 3);
        R.Count();
        Insert(1, 1);
        asserterror R.Count();
        exit(GetLastErrorText());
    end;
}
"#;

fn assert_error(routine: &str) {
    let t = al_testkit::text(run_in(SRC, "C", routine));
    assert!(
        t.contains("The record in table DB Row already exists"),
        "{routine}: {t}"
    );
}

fn assert_expected_error_missing(routine: &str) {
    let r = run_in(SRC, "C", routine);
    assert!(
        matches!(&r, Err(Signal::Error(e)) if e.code == "AssertError"),
        "{routine}: {r:?}"
    );
}

#[test]
fn a_duplicate_among_the_first_four_inserts_is_pending() {
    assert_expected_error_missing("Dup2");
    assert_expected_error_missing("Dup4");
}

#[test]
fn a_duplicate_as_the_fifth_insert_raises_at_the_insert() {
    assert_error("Dup5");
}

#[test]
fn a_duplicate_after_the_queue_was_sent_is_pending_again() {
    assert_expected_error_missing("Dup6");
    assert_expected_error_missing("Dup7");
    assert_expected_error_missing("Dup5AfterCommit");
}

#[test]
fn the_same_four_or_five_keys_twice_raise_inside() {
    assert_error("FourTwice");
    assert_error("FiveTwice");
}

#[test]
fn four_inserts_a_commit_and_the_same_four_keys_raise_at_the_next_read() {
    assert_error("FourCommitFour");
}

#[test]
fn a_read_sends_the_queue() {
    assert_error("ReadBetween");
}

// Every measured cell has a duplicate as the Insert that sends the queue (Dup_5thInsert_OfFirstRow, Report_FourRowsTwice_NoRead,
// Report_FiveRowsTwice_NoRead). A valid row that sends it while a duplicate is pending is not measured: whether the row stays.
#[test]
fn a_valid_row_that_sends_the_queue_with_a_duplicate_pending_is_refused() {
    match run_in(SRC, "C", "ValidRowSends") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("a valid row"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// The probe measured the Insert statements of AL code. An XmlPort import saves its rows itself: its duplicate stays pending
// as before the queue (xmlports/import/XpTImportBasic.al Import_DuplicateKeyWithinInput: it comes out when the guarded run
// ends), and every row's trigger runs.
#[test]
fn an_xmlport_import_is_not_queued() {
    let t = al_testkit::text(run_in(SRC, "C", "PortImport"));
    assert!(t.starts_with("7|Yes|The record in table DB Row already exists"), "{t}");
}
