//! A buffer a Next loaded, made stale by a Commit: it follows the rule of a buffer FindFirst loaded, and a TransferFields out of
//! such a record the rule of a TransferFields out of a record read by Get (record/stale-find/Y89StaleFind.al:408-484, Y89_Part1_next,
//! Y89_Part1_findset_next, Y89_Part1_descending_next; :560-578 Y89_Part1_transfer_next).

use al_testkit::text;

const SRC: &str = r#"
table 69970 "Y89 Row"
{
    fields { field(1; "Entry No."; Integer) { } field(2; "Text Field"; Text[100]) { } field(3; "Integer Field"; Integer) { } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
codeunit 69971 C
{
    local procedure Stored(): Text
    var Row: Record "Y89 Row";
    begin
        if not Row.Get(5) then exit('none');
        exit(Row."Text Field" + '|' + Format(Row."Integer Field"));
    end;

    local procedure MakeBuffer(var Row: Record "Y89 Row"; Origin: Text)
    var Src: Record "Y89 Row";
    begin
        case Origin of
            'next': begin Row.FindFirst(); Row.Next(); end;
            'findset_next': begin Row.FindSet(); Row.Next(); end;
            'descending_next': begin Row.Ascending(false); Row.FindSet(); Row.Next(); Row.Next(-1); end;
            'transfer_next': begin Src.FindFirst(); Src.Next(); Row.TransferFields(Src); end;
        end;
    end;

    local procedure Between(What: Text)
    var Later: Record "Y89 Row";
    begin
        case What of
            'get': Later.Get(5);
            'modify_changed': begin Later.Get(5); Later."Text Field" := 'later'; Later.Modify(false); end;
            'modify_unchanged': begin Later.Get(5); Later.Modify(false); end;
            'lock': begin Later.LockTable(); Later.Get(5); end;
        end;
    end;

    local procedure Write(var Row: Record "Y89 Row"; Deleting: Boolean): Text
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
    var Row: Record "Y89 Row"; Inserter: Record "Y89 Row"; Other: Record "Y89 Row"; Rows: Record "Y89 Row"; Outcome: Text;
    begin
        Rows.DeleteAll(); Commit();
        Inserter.Init(); Inserter."Entry No." := 4; Inserter."Text Field" := 'first'; Inserter.Insert(false);
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
        Outcome += ' at=' + Format(Row."Entry No.") + ' stored=' + Stored();
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
        "{SRC}\ncodeunit 69972 Caller {{ procedure Go(): Text var C: Codeunit C; begin exit(C.Case('{origin}', '{what}', {deleting}, {same_tx})); end; }}"
    );
    text(al_testkit::run_in(&src, "Caller", "Go"))
}

const RC: &str = "DB:RecordChanged at=5 stored=other|0";

#[test]
fn a_buffer_a_next_loaded_follows_the_get_rule() {
    for origin in ["next", "findset_next", "descending_next"] {
        let c = |what, del, same| case(origin, what, del, same);
        assert_eq!(c("nothing", false, true), RC, "{origin}/nothing");
        assert_eq!(c("get", false, true), RC, "{origin}/get");
        assert_eq!(
            c("modify_changed", false, true),
            "ok at=5 stored=start|5",
            "{origin}/modify_changed"
        );
        assert_eq!(
            c("modify_unchanged", false, true),
            "ok at=5 stored=start|5",
            "{origin}/modify_unchanged"
        );
        assert_eq!(c("lock", false, true), "ok at=5 stored=start|5", "{origin}/lock");
        assert_eq!(c("nothing", true, true), RC, "{origin}/delete_nothing");
        assert_eq!(
            c("modify_changed", true, true),
            "ok at=5 stored=none",
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
fn a_buffer_that_got_transfer_fields_from_a_record_a_next_loaded_is_written_after_a_lock_only() {
    let c = |what, del, same| case("transfer_next", what, del, same);
    assert_eq!(c("nothing", false, true), RC);
    assert_eq!(c("get", false, true), RC);
    assert_eq!(c("modify_changed", false, true), RC);
    assert_eq!(c("modify_unchanged", false, true), RC);
    assert_eq!(c("lock", false, true), "ok at=5 stored=start|5");
    assert_eq!(c("nothing", true, true), RC);
    assert_eq!(c("modify_changed", true, true), RC);
    assert_eq!(c("nothing", false, false), RC);
    assert_eq!(c("modify_changed", false, false), RC);
}
