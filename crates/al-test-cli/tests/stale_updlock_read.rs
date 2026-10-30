//! A stale buffer from the variable's own Insert, four Commits later, after a transaction that reads another table under
//! ReadIsolation UpdLock (Find, as No. Series takes a number) and writes other tables only: the Modify raises DB:RecordChanged,
//! as after a plain Get (record/lt-wb-6287/WbStaleUpdLockTests.al:16-25 and the contrast :29-38; al-runner issue 6287).
//! Five Commits and an UpdLock read of the buffer's own table are not pinned and stay refused.

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50100 "P4 Row" { fields { field(1; "No."; Code[20]) { } field(2; Note; Text[30]) { } field(3; Qty; Integer) { } } keys { key(PK; "No.") { } } }
table 50101 "P4 Other" { fields { field(1; "No."; Code[20]) { } field(2; Qty; Integer) { } } keys { key(PK; "No.") { } } }
table 50102 "P4 Series" { fields { field(1; "Code"; Code[20]) { } field(2; "Last No. Used"; Code[20]) { } } keys { key(PK; "Code") { } } }
codeunit 50103 T
{
    Subtype = Test;
    [Test] procedure UpdLockFour()
    var Row: Record "P4 Row";
    begin
        Prepare(Row, 4);
        WriteOthers(true);
        Row.Qty := 5;
        asserterror Row.Modify(true);
        Verify();
    end;
    [Test] procedure PlainFour()
    var Row: Record "P4 Row";
    begin
        Prepare(Row, 4);
        WriteOthers(false);
        Row.Qty := 5;
        asserterror Row.Modify(true);
        Verify();
    end;
    [Test] procedure UpdLockFiveIsRefused()
    var Row: Record "P4 Row";
    begin
        Prepare(Row, 5);
        WriteOthers(true);
        Row.Qty := 5;
        asserterror Row.Modify(true);
        Verify();
    end;
    [Test] procedure UpdLockOwnTableIsRefused()
    var Row: Record "P4 Row"; Again: Record "P4 Row";
    begin
        Prepare(Row, 4);
        WriteOthers(false);
        Again."No." := 'P4';
        Again.ReadIsolation(IsolationLevel::UpdLock);
        Again.Find();
        Row.Qty := 5;
        asserterror Row.Modify(true);
        Verify();
    end;
    local procedure Prepare(var Row: Record "P4 Row"; Commits: Integer)
    var Changer: Record "P4 Row"; Other: Record "P4 Other"; Series: Record "P4 Series"; i: Integer;
    begin
        Row.DeleteAll(); Other.DeleteAll(); Series.DeleteAll();
        Other."No." := 'ITEM-1'; Other.Insert();
        Series.Code := 'ITEM'; Series."Last No. Used" := 'X0000'; Series.Insert();
        Commit();
        Row."No." := 'P4'; Row.Note := 'start'; Row.Insert(true);
        Row.Qty := 1; Row.Modify(true);
        Changer.Get('P4'); Changer.Note := 'other'; Changer.Modify(false);
        for i := 1 to Commits do begin
            if i mod 2 = 0 then InsertOther('MIDDLE-' + Format(i));
            Commit();
        end;
    end;
    local procedure WriteOthers(UpdLock: Boolean)
    var Other: Record "P4 Other"; Series: Record "P4 Series";
    begin
        Other.Get('ITEM-1'); Other.Qty := 12; Other.Modify(true);
        if UpdLock then begin
            Series.Code := 'ITEM';
            Series.ReadIsolation(IsolationLevel::UpdLock);
            Series.Find();
        end else
            Series.Get('ITEM');
        Series."Last No. Used" := IncStr(Series."Last No. Used");
        Series.Modify();
        InsertOther(Series."Last No. Used");
    end;
    local procedure InsertOther(No: Code[20])
    var Other: Record "P4 Other";
    begin
        Other."No." := No; Other.Insert(true);
    end;
    local procedure Verify()
    begin
        if GetLastErrorCode() <> 'DB:RecordChanged' then
            Error('Expected DB:RecordChanged, got %1: %2', GetLastErrorCode(), GetLastErrorText());
    end;
}
"#;

#[test]
fn an_updlock_read_of_another_table_four_commits_later_raises_record_changed() {
    let o = outcomes_in(SRC, "T");
    let get = |n: &str| format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x));
    assert_eq!(get("UpdLockFour"), "Some(Pass)");
    assert_eq!(get("PlainFour"), "Some(Pass)");
    for t in ["UpdLockFiveIsRefused", "UpdLockOwnTableIsRefused"] {
        let g = get(t);
        assert!(g.contains("NotRun"), "{t}: {g}");
    }
}
