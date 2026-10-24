//! A Modify of a buffer that still holds what its own last Modify wrote, after a rollback undid that write
//! (corpus record/lt-record-conversion-assumptions/StaleBuffer.al).

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69997 "SB Row"
{
    fields { field(1; Id; Code[10]) { } field(2; Counter; Integer) { } field(3; Note; Text[20]) { } }
    keys { key(PK; Id) { } }
    trigger OnModify() begin end;
}
codeunit 69998 C
{
    procedure NotReassigned(): Text
    var R: Record "SB Row"; Check: Record "SB Row"; s: Text;
    begin
        R.Id := 'S2'; R.Counter := 10; R.Note := 'orig'; R.Insert();
        Commit();
        R.Counter := 11;
        R.Modify();
        asserterror Error('boom');
        s := Format(R.Counter) + '|' + Format(R.Modify());
        Check.Get('S2');
        exit(s + '|' + Format(Check.Counter) + '|' + Check.Note);
    end;

    procedure ChangedAfter(): Text
    var R: Record "SB Row"; Check: Record "SB Row";
    begin
        R.Id := 'S5'; R.Counter := 1; R.Insert();
        Commit();
        R.Counter := 3;
        R.Modify();
        asserterror Error('boom');
        R.Note := 'late';
        R.Modify();
        Check.Get('S5');
        exit(Format(Check.Counter) + '|' + Check.Note);
    end;

    procedure WithTrigger()
    var R: Record "SB Row";
    begin
        R.Id := 'S6'; R.Counter := 1; R.Insert();
        Commit();
        R.Counter := 2;
        R.Modify();
        asserterror Error('boom');
        R.Modify(true);
    end;
}
"#;

// StaleBuffer.al:76-86: the buffer keeps 11, the Modify answers true and the row keeps the value the rollback restored.
#[test]
fn modify_of_the_buffer_its_own_rolled_back_write_left_writes_nothing() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "NotReassigned")), "11|Yes|10|orig");
    // StaleBuffer.al:167-168: a buffer changed after the rollback writes the whole row.
    assert_eq!(text(al_testkit::run_in(SRC, "C", "ChangedAfter")), "3|late");
}

// Not pinned: the same Modify of a table with an OnModify trigger, with the triggers.
#[test]
fn the_same_modify_with_triggers_is_refused() {
    let r = al_testkit::run_in(SRC, "C", "WithTrigger");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

const MEDIA: &str = r#"
table 2000000184 "Tenant Media"
{
    fields
    {
        field(1; ID; Guid) { } field(2; Description; Text[250]) { } field(3; Content; Blob) { }
        field(4; "Mime Type"; Text[100]) { } field(5; Height; Integer) { } field(6; Width; Integer) { }
    }
    keys { key(PK; ID) { } }
}
table 69999 "SB Media"
{
    fields { field(1; Id; Integer) { } field(2; Pic; Media) { } field(3; Src; Blob) { } }
    keys { key(PK; Id) { } }
}
codeunit 69989 M
{
    procedure Replace(): Text
    var Row: Record "SB Media"; TenantMedia: Record "Tenant Media"; ins: InStream; outs: OutStream; First: Guid; Second: Guid; s: Text;
    begin
        Row.Id := 1; Row.Insert();
        Row.Src.CreateOutStream(outs); outs.WriteText('one'); Row.Src.CreateInStream(ins);
        Row.Pic.ImportStream(ins, 'one');
        Row.Modify();
        First := Row.Pic.MediaId();
        Row.Src.CreateOutStream(outs); outs.WriteText('two'); Row.Src.CreateInStream(ins);
        Row.Pic.ImportStream(ins, 'two');
        Second := Row.Pic.MediaId();
        s := Format(TenantMedia.Get(First));
        Row.Modify();
        exit(s + Format(TenantMedia.Get(First)) + Format(TenantMedia.Get(Second)));
    end;
}
"#;

// record/lt-ar-records/ArRecordProbes.al:141-143: the Modify that stores the field's new media removes the Tenant Media row of
// the old one.
#[test]
fn modify_that_replaces_a_media_removes_the_old_media() {
    assert_eq!(text(al_testkit::run_in(MEDIA, "M", "Replace")), "YesNoYes");
}

// AL Runner issue 5805: a buffer made stale before a Commit, written after the Commit, depending on what the new
// transaction did to the row first (corpus record/TestRecordStaleBufferWrite.al:395-690, writer RU).
const RU: &str = r#"
table 69995 "RU Row"
{
    fields { field(1; Id; Integer) { } field(2; Note; Text[20]) { } field(3; Qty; Integer) { } }
    keys { key(PK; Id) { } }
}
codeunit 69996 RU
{
    local procedure InsertRow(var Row: Record "RU Row")
    begin
        Row.Init(); Row.Id := 5; Row.Note := 'start'; Row.Insert(false);
    end;

    local procedure ChangeAndCommit()
    var Other: Record "RU Row";
    begin
        Other.Get(5); Other.Note := 'other'; Other.Modify(false); Commit();
    end;

    local procedure Write(var Row: Record "RU Row"): Text
    begin
        Row.Qty := 5;
        Row.Modify(false);
        exit('ok');
    end;

    local procedure Modified(var Row: Record "RU Row"): Text
    var Stored: Record "RU Row";
    begin
        Row.Qty := 5; Row.Modify(false);
        Stored.Get(5);
        exit('ok|' + Stored.Note + '|' + Format(Stored.Qty));
    end;

    local procedure Refused(var Row: Record "RU Row"): Text
    var Stored: Record "RU Row";
    begin
        Row.Qty := 5;
        asserterror Row.Modify(false);
        Stored.Get(5);
        exit(GetLastErrorCode() + '|' + Stored.Note + '|' + Format(Stored.Qty));
    end;

    procedure InsertNothing(): Text
    var Row: Record "RU Row";
    begin
        InsertRow(Row); ChangeAndCommit(); exit(Refused(Row));
    end;

    procedure InsertLaterUnchanged(): Text
    var Row: Record "RU Row"; Later: Record "RU Row";
    begin
        InsertRow(Row); ChangeAndCommit(); Later.Get(5); Later.Modify(false); exit(Modified(Row));
    end;

    procedure InsertWriterAgain(): Text
    var Row: Record "RU Row"; Other: Record "RU Row";
    begin
        InsertRow(Row);
        Other.Get(5); Other.Note := 'other'; Other.Modify(false); Commit();
        Other.Modify(false);
        exit(Modified(Row));
    end;

    procedure InsertLockTableGet(): Text
    var Row: Record "RU Row"; Later: Record "RU Row";
    begin
        InsertRow(Row); ChangeAndCommit(); Later.LockTable(); Later.Get(5); exit(Modified(Row));
    end;

    procedure InsertGetOnly(): Text
    var Row: Record "RU Row"; Later: Record "RU Row";
    begin
        InsertRow(Row); ChangeAndCommit(); Later.Get(5); exit(Refused(Row));
    end;

    procedure InsertGetChange(): Text
    var Row: Record "RU Row"; Later: Record "RU Row";
    begin
        InsertRow(Row); ChangeAndCommit(); Later.Get(5); Later.Note := 'later'; Later.Modify(false); exit(Modified(Row));
    end;

    procedure InsertUnchangedThenCommit(): Text
    var Row: Record "RU Row"; Later: Record "RU Row";
    begin
        InsertRow(Row); ChangeAndCommit(); Later.Get(5); Later.Modify(false); Commit(); exit(Refused(Row));
    end;

    procedure GetBufferUnchanged(): Text
    var Row: Record "RU Row"; Later: Record "RU Row";
    begin
        InsertRow(Row); Commit(); Row.Get(5); ChangeAndCommit(); Later.Get(5); Later.Modify(false); exit(Refused(Row));
    end;

    procedure GetBufferLockTableGet(): Text
    var Row: Record "RU Row"; Later: Record "RU Row";
    begin
        InsertRow(Row); Commit(); Row.Get(5); ChangeAndCommit(); Later.LockTable(); Later.Get(5); exit(Refused(Row));
    end;

    procedure GetBufferChange(): Text
    var Row: Record "RU Row"; Later: Record "RU Row";
    begin
        InsertRow(Row); Commit(); Row.Get(5); ChangeAndCommit(); Later.Get(5); Later.Note := 'later'; Later.Modify(false); exit(Refused(Row));
    end;

    procedure DeleteNothing(): Text
    var Row: Record "RU Row"; Stored: Record "RU Row";
    begin
        InsertRow(Row); ChangeAndCommit();
        asserterror Row.Delete(false);
        exit(GetLastErrorCode() + '|' + Format(Stored.Get(5)));
    end;

    procedure DeleteAfterUnchanged(): Text
    var Row: Record "RU Row"; Later: Record "RU Row"; Stored: Record "RU Row";
    begin
        InsertRow(Row); ChangeAndCommit(); Later.Get(5); Later.Modify(false);
        Row.Delete(false);
        exit('ok|' + Format(Stored.Get(5)));
    end;

    // Not pinned: another row of the table written in the new transaction.
    procedure InsertOtherRowWritten(): Text
    var Row: Record "RU Row"; Two: Record "RU Row";
    begin
        InsertRow(Row); ChangeAndCommit(); Two.Id := 6; Two.Insert(); exit(Write(Row));
    end;

    // Not pinned (assumption stale-buffer-other-origin-written): a buffer read by FindFirst, stale after a Commit, whose row
    // the new transaction wrote.
    procedure FindBufferWritten(): Text
    var Row: Record "RU Row"; Later: Record "RU Row"; Stored: Record "RU Row";
    begin
        InsertRow(Row); Commit(); Row.FindFirst(); ChangeAndCommit(); Later.Get(5); Later.Note := 'later'; Later.Modify(false);
        Row.Qty := 5; Row.Modify(false);
        Stored.Get(5);
        exit(Stored.Note + '|' + Format(Stored.Qty));
    end;

    // Not pinned: a second Commit between the Insert and the transaction that modified the row.
    procedure InsertTwoCommits(): Text
    var Row: Record "RU Row"; Later: Record "RU Row";
    begin
        InsertRow(Row); ChangeAndCommit(); Commit(); Later.Get(5); Later.Modify(false); exit(Write(Row));
    end;
}
"#;

#[test]
fn a_stale_insert_buffer_is_written_once_the_new_transaction_modified_or_locked_the_row() {
    let run = |p: &str| text(al_testkit::run_in(RU, "RU", p));
    // TestRecordStaleBufferWrite.al:481, :551, :586: refused, the row keeps the committed change.
    assert_eq!(run("InsertNothing"), "DB:RecordChanged|other|0");
    assert_eq!(run("InsertGetOnly"), "DB:RecordChanged|other|0");
    assert_eq!(run("InsertUnchangedThenCommit"), "DB:RecordChanged|other|0");
    // :495, :513, :533, :567: accepted, the stale buffer wins.
    assert_eq!(run("InsertLaterUnchanged"), "ok|start|5");
    assert_eq!(run("InsertWriterAgain"), "ok|start|5");
    assert_eq!(run("InsertLockTableGet"), "ok|start|5");
    assert_eq!(run("InsertGetChange"), "ok|start|5");
}

#[test]
fn a_stale_get_buffer_stays_refused_whatever_the_new_transaction_did() {
    let run = |p: &str| text(al_testkit::run_in(RU, "RU", p));
    // TestRecordStaleBufferWrite.al:604, :623, :642.
    assert_eq!(run("GetBufferUnchanged"), "DB:RecordChanged|other|0");
    assert_eq!(run("GetBufferLockTableGet"), "DB:RecordChanged|other|0");
    assert_eq!(run("GetBufferChange"), "DB:RecordChanged|other|0");
}

#[test]
fn a_stale_delete_follows_the_same_rule() {
    let run = |p: &str| text(al_testkit::run_in(RU, "RU", p));
    // TestRecordStaleBufferWrite.al:662, :676.
    assert_eq!(run("DeleteNothing"), "DB:RecordChanged|Yes");
    assert_eq!(run("DeleteAfterUnchanged"), "ok|No");
}

// Microsoft's No. Series batch writes such a buffer (Tests-ERM, 110 tests): the last writer wins under the assumption.
#[test]
fn a_stale_buffer_of_another_origin_whose_row_the_transaction_wrote_is_written() {
    assert_eq!(text(al_testkit::run_in(RU, "RU", "FindBufferWritten")), "start|5");
}

#[test]
fn a_stale_insert_buffer_after_unpinned_steps_is_refused() {
    for p in ["InsertOtherRowWritten", "InsertTwoCommits"] {
        let r = al_testkit::run_in(RU, "RU", p);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{p}: {r:?}");
    }
}
