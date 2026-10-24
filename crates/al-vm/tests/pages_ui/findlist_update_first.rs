//! `CurrPage.Update(false)` from an action on a descending List served by OnFindRecord and OnNextRecord, then `First()` (al-runner
//! issue 5802, the reporter's shape). The open read and the moves are pinned (misc/lt-probes-needed-6/PnF06FindListMoves.al:1137,
//! :1248); after an Update the client asks the triggers again (pagefindrecord/TestPageFindRecord.al:113).

fn src(rows: u32) -> String {
    format!(
        r#"
table 69970 "UF Entry"
{{
    fields {{ field(1; "No."; Code[20]) {{ }} field(2; Kind; Option) {{ OptionMembers = A,B; }} }}
    keys {{ key(PK; "No.") {{ Clustered = true; }} }}
}}
page 69971 "UF Entry List"
{{
    PageType = List;
    SourceTable = "UF Entry";
    SourceTableView = sorting("No.") order(descending) where(Kind = filter(A));
    UsageCategory = None;
    layout {{ area(Content) {{ repeater(Rows) {{ field("No."; Rec."No.") {{ }} }} }} }}
    actions {{ area(Processing) {{ action(Refresh) {{ trigger OnAction() begin CurrPage.Update(false); end; }} }} }}
    trigger OnFindRecord(Which: Text): Boolean
    begin
        exit(Rec.Find(Which));
    end;
    trigger OnNextRecord(Steps: Integer): Integer
    begin
        exit(Rec.Next(Steps));
    end;
}}
codeunit 69972 C
{{
    procedure Run(): Text
    var
        Entry: Record "UF Entry";
        EntryList: TestPage "UF Entry List";
        i: Integer;
        Result: Text;
    begin
        Entry.DeleteAll();
        for i := 1 to {rows} do begin
            Entry.Init();
            Entry."No." := 'R' + PadStr('', 3 - StrLen(Format(i)), '0') + Format(i);
            Entry.Kind := Entry.Kind::A;
            Entry.Insert();
        end;
        EntryList.OpenView();
        EntryList.Refresh.Invoke();
        Result := Format(EntryList.First()) + ':' + EntryList."No.".Value();
        EntryList.Close();
        exit(Result);
    end;
}}
"#
    )
}

/// 25 rows: the open read stopped a window above its last row. It used to panic in `first_above_the_find_list_window`. After the
/// Update the client asks the triggers again, and First lands on the first row of the descending view (pagefindrecord/
/// TestPageFindRecord.al:113 OnFindRecord_FirstAnswersTheFirstRowOfTheTriggerRowset: an action that refreshes the page, then
/// First on a descending find list).
#[test]
fn first_after_update_on_a_descending_find_list_longer_than_a_window_lands_on_the_first_row() {
    assert_eq!(al_testkit::text(al_testkit::run_in(&src(25), "C", "Run")), "Yes:R025");
}

#[test]
fn first_after_update_on_a_descending_find_list_of_one_window_lands_on_the_first_row() {
    assert_eq!(al_testkit::text(al_testkit::run_in(&src(20), "C", "Run")), "Yes:R020");
}
