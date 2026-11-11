//! A buffer made stale by a Commit whose row version came from a Find, an assignment, Copy or TransferFields (not from a Get or
//! from the variable's own Insert). Pinned by corpus record/crud/Y76StaleBuffers.al Y76_Part1_*: a Find, an
//! assignment and Copy behave as a Get; TransferFields from a record is written only after LockTable + Get, from a temporary
//! copy never.

use al_testkit::text;

const SRC: &str = r#"
table 69960 "Y76 Row"
{
    fields { field(1; "Entry No."; Integer) { } field(2; "Text Field"; Text[100]) { } field(3; "Integer Field"; Integer) { } field(4; "Decimal Field"; Decimal) { } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
table 69963 "Y76 Keyed"
{
    fields { field(1; "Entry No."; Integer) { } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
codeunit 69961 C
{
    local procedure Part2Run(OwnModify: Boolean; Commits: Integer; Beside: Text; Plain: Boolean): Text
    var
        Row: Record "Y76 Row"; Another: Record "Y76 Row"; Other: Record "Y76 Row"; Rows: Record "Y76 Row"; Keyed: Record "Y76 Keyed";
        i: Integer; Outcome: Text; S6: Text;
    begin
        Rows.DeleteAll(); Keyed.DeleteAll(); Commit();
        Row.Init(); Row."Entry No." := 6; Row."Text Field" := 'start'; Row.Insert(true);
        if OwnModify then begin Row."Integer Field" := 1; Row.Modify(false); end;
        Another.Init(); Another."Entry No." := 8; Another.Insert(true);
        Other.Get(6); Other."Text Field" := 'other'; Other.Modify(false);
        Commit();
        for i := 2 to Commits do begin Keyed.Init(); Keyed."Entry No." := i; Keyed.Insert(true); Commit(); end;
        Another.Init(); Another."Entry No." := 10; Another.Insert(true);
        case Beside of
            'delete': begin Another.Get(8); Another.Delete(false); end;
            'rename': begin Another.Get(8); Another.Rename(9); end;
            'othertable': begin Keyed.Init(); Keyed."Entry No." := 100; Keyed.Insert(true); end;
        end;
        Row."Decimal Field" := 5;
        if Plain then begin Row.Modify(false); Outcome := 'ok'; end else Outcome := Write(Row, false);
        Other.Get(6);
        Outcome += ' stored6=' + Other."Text Field" + '|' + Format(Other."Decimal Field");
        Rows.DeleteAll(); Keyed.DeleteAll(); Commit();
        exit(Outcome);
    end;

    procedure Part2(OwnModify: Boolean; Commits: Integer; Beside: Text): Text
    var First: Text;
    begin
        First := Part2Run(OwnModify, Commits, Beside, false);
        if First.StartsWith('ok') then exit(Part2Run(OwnModify, Commits, Beside, true));
        exit(First);
    end;

    local procedure Stored(): Text
    var Row: Record "Y76 Row";
    begin
        if not Row.Get(5) then exit('none');
        exit(Row."Text Field" + '|' + Format(Row."Integer Field"));
    end;

    local procedure MakeBuffer(var Row: Record "Y76 Row"; Origin: Text)
    var Src: Record "Y76 Row"; TempCopy: Record "Y76 Row" temporary;
    begin
        case Origin of
            'get': Row.Get(5);
            'find': Row.FindFirst();
            'findlast': Row.FindLast();
            'findset': Row.FindSet();
            'find_equal': begin Row."Entry No." := 5; Row.Find('='); end;
            'find_minus': Row.Find('-');
            'find_plus': Row.Find('+');
            'find_next': begin Row."Entry No." := 4; Row.Find('>'); end;
            'transfer_from_find_equal': begin Src."Entry No." := 5; Src.Find('='); Row.TransferFields(Src); end;
            'assign': begin Src.Get(5); Row := Src; end;
            'copy': begin Src.Get(5); Row.Copy(Src); end;
            'transfer_from_record': begin Src.Get(5); Row.TransferFields(Src); end;
            'get_then_transfer_from_record': begin Src.Get(5); Row.Get(5); Row.TransferFields(Src); end;
            'get_then_transfer_from_temp':
                begin
                    Src.Get(5); TempCopy.Init(); TempCopy.TransferFields(Src); TempCopy.Insert();
                    Row.Get(5); Row.TransferFields(TempCopy);
                end;
            'transfer_from_temp':
                begin
                    Src.Get(5); TempCopy.Init(); TempCopy.TransferFields(Src); TempCopy.Insert();
                    Row.TransferFields(TempCopy);
                end;
        end;
    end;

    local procedure Between(What: Text)
    var Later: Record "Y76 Row";
    begin
        case What of
            'get': Later.Get(5);
            'modify_changed': begin Later.Get(5); Later."Text Field" := 'later'; Later.Modify(false); end;
            'modify_unchanged': begin Later.Get(5); Later.Modify(false); end;
            'lock': begin Later.LockTable(); Later.Get(5); end;
        end;
    end;

    local procedure Write(var Row: Record "Y76 Row"; Deleting: Boolean): Text
    begin
        ClearLastError();
        if Deleting then
            asserterror begin Row.Delete(false); Error('DONE'); end
        else
            asserterror begin Row.Modify(false); Error('DONE'); end;
        if GetLastErrorText() = 'DONE' then exit('ok');
        exit(GetLastErrorCode());
    end;

    local procedure RunOnce(Origin: Text; What: Text; Deleting: Boolean; SameTx: Boolean; Plain: Boolean): Text
    var Row: Record "Y76 Row"; Inserter: Record "Y76 Row"; Other: Record "Y76 Row"; Rows: Record "Y76 Row"; Outcome: Text;
    begin
        Rows.DeleteAll(); Commit();
        Inserter.Init(); Inserter."Entry No." := 5; Inserter."Text Field" := 'start'; Inserter.Insert(false);
        if not SameTx then Commit();
        MakeBuffer(Row, Origin);
        Other.Get(5); Other."Text Field" := 'other'; Other.Modify(false);
        Commit();
        Between(What);
        Row."Integer Field" := 5;
        if Plain then begin
            if Deleting then Row.Delete(false) else Row.Modify(false);
            Outcome := 'ok';
        end else
            Outcome := Write(Row, Deleting);
        Outcome += ' stored=' + Stored();
        Rows.DeleteAll(); Commit();
        exit(Outcome);
    end;

    procedure Case(Origin: Text; What: Text; Deleting: Boolean; SameTx: Boolean): Text
    var First: Text;
    begin
        First := RunOnce(Origin, What, Deleting, SameTx, false);
        if First.StartsWith('ok') then exit(RunOnce(Origin, What, Deleting, SameTx, true));
        exit(First);
    end;
}
"#;

fn case(origin: &str, what: &str, deleting: bool, same_tx: bool) -> String {
    let src = format!(
        "{SRC}\ncodeunit 69962 Caller {{ procedure Go(): Text var C: Codeunit C; begin exit(C.Case('{origin}', '{what}', {deleting}, {same_tx})); end; }}"
    );
    text(al_testkit::run_in(&src, "Caller", "Go"))
}

const RC: &str = "DB:RecordChanged stored=other|0";

// Y76StaleBuffers.al:238-300 (get/find/findlast/findset/assign/copy), the nine modes of Part1Modes.
#[test]
fn a_find_an_assignment_and_a_copy_buffer_follow_the_get_rule() {
    for origin in [
        "get",
        "find",
        "findlast",
        "findset",
        "assign",
        "copy",
        "get_then_transfer_from_record",
    ] {
        let c = |what, del, same| case(origin, what, del, same);
        assert_eq!(c("nothing", false, true), RC, "{origin}/nothing");
        assert_eq!(c("get", false, true), RC, "{origin}/get");
        assert_eq!(
            c("modify_changed", false, true),
            "ok stored=start|5",
            "{origin}/modify_changed"
        );
        assert_eq!(
            c("modify_unchanged", false, true),
            "ok stored=start|5",
            "{origin}/modify_unchanged"
        );
        assert_eq!(c("lock", false, true), "ok stored=start|5", "{origin}/lock");
        assert_eq!(c("nothing", true, true), RC, "{origin}/delete_nothing");
        assert_eq!(
            c("modify_changed", true, true),
            "ok stored=none",
            "{origin}/delete_modify_changed"
        );
        assert_eq!(c("nothing", false, false), RC, "{origin}/committed_nothing");
        assert_eq!(
            c("modify_changed", false, false),
            RC,
            "{origin}/committed_modify_changed"
        );
    }
}

// Y76StaleBuffers.al:302-326 (Y76_Part1_Copy_TransferFromRecord): only LockTable + Get rescues the write.
#[test]
fn a_buffer_that_only_got_transfer_fields_from_a_record_is_written_after_a_lock_only() {
    let c = |what, del, same| case("transfer_from_record", what, del, same);
    assert_eq!(c("nothing", false, true), RC);
    assert_eq!(c("get", false, true), RC);
    assert_eq!(c("modify_changed", false, true), RC);
    assert_eq!(c("modify_unchanged", false, true), RC);
    assert_eq!(c("lock", false, true), "ok stored=start|5");
    assert_eq!(c("nothing", true, true), RC);
    assert_eq!(c("modify_changed", true, true), RC);
    assert_eq!(c("nothing", false, false), RC);
    assert_eq!(c("modify_changed", false, false), RC);
}

// Y76StaleBuffers.al:343-407: TransferFields from a temporary copy is always refused, also after a Get.
#[test]
fn a_buffer_that_got_transfer_fields_from_a_temporary_copy_is_always_refused() {
    for origin in ["transfer_from_temp", "get_then_transfer_from_temp"] {
        for (what, del, same) in [
            ("nothing", false, true),
            ("get", false, true),
            ("modify_changed", false, true),
            ("modify_unchanged", false, true),
            ("lock", false, true),
            ("nothing", true, true),
            ("modify_changed", true, true),
            ("nothing", false, false),
            ("modify_changed", false, false),
        ] {
            assert_eq!(case(origin, what, del, same), RC, "{origin}/{what}/{del}/{same}");
        }
    }
}

fn part2(own_modify: bool, commits: u32, beside: &str) -> String {
    let src = format!(
        "{SRC}\ncodeunit 69962 Caller {{ procedure Go(): Text var C: Codeunit C; begin exit(C.Part2({own_modify}, {commits}, '{beside}')); end; }}"
    );
    text(al_testkit::run_in(&src, "Caller", "Go"))
}

// Y76StaleBuffers.al:Y76_Part2_ThreeCommits_AndOneCommit (the ten shapes it pins): another row inserted, and beside it a Delete of
// another row, a Rename or an Insert into another table: the stale Modify writes the whole buffer (the Decimal 5 shows).
#[test]
fn a_stale_own_insert_buffer_is_written_beside_a_delete_a_rename_or_another_table() {
    for (own_modify, commits, beside) in [
        (true, 3, "delete"),
        (true, 3, "rename"),
        (true, 3, "othertable"),
        (false, 3, "delete"),
        (false, 3, "othertable"),
        (true, 1, "delete"),
        (true, 1, "rename"),
        (true, 1, "othertable"),
        (false, 1, "delete"),
        (false, 1, "rename"),
        (true, 2, "rename"),
        (false, 2, "othertable"),
    ] {
        assert_eq!(
            part2(own_modify, commits, beside),
            "ok stored6=start|5",
            "{own_modify}/{commits}/{beside}"
        );
    }
}

// Another row inserted and another renamed, three Commits after an own Insert: table contact writes
// (record/crud/StaleRuleMatrix.al TableContact_4C_B `oi_od_on`, TableContact_2C_B `oi_od_on`).
#[test]
fn a_stale_own_insert_buffer_three_commits_back_is_written_beside_a_rename() {
    assert_eq!(part2(false, 3, "rename"), "ok stored6=start|5");
}

// Every other form of Find follows the rule of FindFirst, and a TransferFields out of a record one read follows the rule of a
// TransferFields out of a record read by Get (record/stale-find/Y89StaleFind.al:256-560, Y89_Part1_find_* and
// Y89_Part1_transfer_find_*).
#[test]
fn a_buffer_read_by_another_form_of_find_follows_the_get_rule() {
    for origin in ["find_equal", "find_minus", "find_plus", "find_next"] {
        let c = |what, del, same| case(origin, what, del, same);
        assert_eq!(c("nothing", false, true), RC, "{origin}/nothing");
        assert_eq!(c("get", false, true), RC, "{origin}/get");
        assert_eq!(
            c("modify_changed", false, true),
            "ok stored=start|5",
            "{origin}/modify_changed"
        );
        assert_eq!(
            c("modify_unchanged", false, true),
            "ok stored=start|5",
            "{origin}/modify_unchanged"
        );
        assert_eq!(c("lock", false, true), "ok stored=start|5", "{origin}/lock");
        assert_eq!(c("nothing", true, true), RC, "{origin}/delete_nothing");
        assert_eq!(
            c("modify_changed", true, true),
            "ok stored=none",
            "{origin}/delete_modify_changed"
        );
        assert_eq!(c("nothing", false, false), RC, "{origin}/committed_nothing");
        assert_eq!(
            c("modify_changed", false, false),
            RC,
            "{origin}/committed_modify_changed"
        );
    }
}

#[test]
fn a_buffer_that_got_transfer_fields_from_a_found_record_is_written_after_a_lock_only() {
    let c = |what, del, same| case("transfer_from_find_equal", what, del, same);
    assert_eq!(c("nothing", false, true), RC);
    assert_eq!(c("get", false, true), RC);
    assert_eq!(c("modify_changed", false, true), RC);
    assert_eq!(c("modify_unchanged", false, true), RC);
    assert_eq!(c("lock", false, true), "ok stored=start|5");
    assert_eq!(c("nothing", true, true), RC);
    assert_eq!(c("modify_changed", true, true), RC);
    assert_eq!(c("nothing", false, false), RC);
    assert_eq!(c("modify_changed", false, false), RC);
}
