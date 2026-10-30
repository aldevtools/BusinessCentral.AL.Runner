//! CurrPage.Update(false) from an action on a list after a GoToKey, with and without OnAfterGetCurrRecord
//! (pages/lt-ai-list-order/AiListOrderTests.al). AL source -> compile -> run as a test codeunit.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62500 "LUW Row" { fields { field(1; "No."; Code[10]) { } field(2; Qty; Integer) { } } keys { key(PK; "No.") { } } }
codeunit 62501 "LUW Log" { SingleInstance = true; var T: Text;
    procedure Clear() begin T := ''; end;
    procedure Add(S: Text) begin if T <> '' then T += ';'; T += S; end;
    procedure Dump(): Text begin exit(T); end;
    procedure Seed() var R: Record "LUW Row"; I: Integer; begin R.DeleteAll(); for I := 1 to 6 do begin R."No." := 'R100' + Format(I); R.Qty := I; R.Insert(); end; end; }
page 62502 "LUW Plain" { PageType = List; SourceTable = "LUW Row"; Editable = false;
    layout { area(Content) { repeater(G) { field(NoCtl; Rec."No.") { } } } }
    actions { area(Processing) {
        action(KeepFilter) { trigger OnAction() var L: Codeunit "LUW Log"; begin L.Add('A:filter'); Rec.SetFilter(Qty, '<=%1', 4); CurrPage.Update(false); L.Add('A:done'); end; }
        action(Drop) { trigger OnAction() var L: Codeunit "LUW Log"; begin L.Add('A:filter'); Rec.SetFilter(Qty, '>%1', Rec.Qty); CurrPage.Update(false); L.Add('A:done'); end; } } }
    trigger OnAfterGetRecord() var L: Codeunit "LUW Log"; begin L.Add('R:' + Rec."No."); end; }
page 62503 "LUW Plain Curr" { PageType = List; SourceTable = "LUW Row"; Editable = false;
    layout { area(Content) { repeater(G) { field(NoCtl; Rec."No.") { } } } }
    actions { area(Processing) {
        action(Drop) { trigger OnAction() var L: Codeunit "LUW Log"; begin L.Add('A:filter'); Rec.SetFilter(Qty, '>%1', Rec.Qty); CurrPage.Update(false); L.Add('A:done'); end; } } }
    trigger OnAfterGetRecord() var L: Codeunit "LUW Log"; begin L.Add('R:' + Rec."No."); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "LUW Log"; begin L.Add('C:' + Rec."No."); end; }
page 62504 "LUW Nav" { PageType = List; SourceTable = "LUW Row"; Editable = false;
    layout { area(Content) { repeater(G) { field(NoCtl; Rec."No.") { } } } }
    actions { area(Processing) {
        action(Same) { trigger OnAction() var L: Codeunit "LUW Log"; begin L.Add('A:update'); CurrPage.Update(false); L.Add('A:done'); end; } } }
    trigger OnFindRecord(Which: Text): Boolean var L: Codeunit "LUW Log"; Found: Boolean; begin Found := Rec.Find(Which); L.Add('F:' + Which + '=' + Format(Found)); exit(Found); end;
    trigger OnNextRecord(Steps: Integer): Integer var L: Codeunit "LUW Log"; N: Integer; begin N := Rec.Next(Steps); L.Add('N:' + Format(Steps) + '=' + Format(N)); exit(N); end;
    trigger OnAfterGetRecord() var L: Codeunit "LUW Log"; begin L.Add('R:' + Rec."No."); end; }
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62510 T {{ Subtype = Test; {tests} }}");
    let o = al_testkit::outcomes_in(&src, "T");
    assert!(!o.is_empty());
    o.into_iter().map(|(n, r)| (n.to_string(), r)).collect()
}

fn refused(tests: &str) {
    for (n, r) in outcomes(tests) {
        assert!(matches!(r, Outcome::NotRun(_)), "{n}: {r:?}");
    }
}

fn passes(tests: &str) {
    let src = format!("{OBJECTS}\ncodeunit 62510 T {{ Subtype = Test; {tests} }}");
    let o = al_testkit::outcomes_in(&src, "T");
    assert!(!o.is_empty());
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

// AiListOrderTests.al:233 Keep_Plain: two reads of the row before the action (one is the GoToKey's), the rows from it on, the way
// back, and nothing more without OnAfterGetCurrRecord.
#[test]
fn update_on_a_plain_list_without_current_record_trigger_after_goto_key() {
    passes(
        r#"[Test] procedure Keep() var L: Codeunit "LUW Log"; P: TestPage "LUW Plain"; begin
            L.Seed(); P.OpenView(); L.Clear(); P.GoToKey('R1002');
            if L.Dump() <> '' then Error('goto %1', L.Dump());
            P.KeepFilter.Invoke();
            if L.Dump() <> 'R:R1002;R:R1002;A:filter;A:done;R:R1002;R:R1002;R:R1003;R:R1004;R:R1002;R:R1001' then Error('update %1', L.Dump());
        end;"#,
    );
}

// AiListOrderTests.al:335 DropCurrent_RowsAfter_Plain and :369 DropCurrent_RowsAfter_PlainCurr: the filter leaves the row, the list
// settles on the next row of the view, and First stays on it.
#[test]
fn update_after_the_filter_left_the_row_settles_on_the_next_row() {
    passes(
        r#"[Test] procedure Plain() var L: Codeunit "LUW Log"; P: TestPage "LUW Plain"; begin
            L.Seed(); P.OpenView(); P.GoToKey('R1002'); L.Clear();
            P.Drop.Invoke();
            if L.Dump() <> 'R:R1002;R:R1002;A:filter;A:done;R:R1003;R:R1003;R:R1004;R:R1005;R:R1006;R:R1003' then Error('update %1', L.Dump());
            L.Clear(); P.First(); if P.NoCtl.Value() <> 'R1003' then Error('first %1', P.NoCtl.Value()); if L.Dump() <> '' then Error('after first %1', L.Dump());
        end;
        [Test] procedure PlainCurr() var L: Codeunit "LUW Log"; P: TestPage "LUW Plain Curr"; begin
            L.Seed(); P.OpenView(); P.GoToKey('R1002'); L.Clear();
            P.Drop.Invoke();
            if L.Dump() <> 'R:R1002;A:filter;A:done;R:R1003;R:R1003;R:R1004;R:R1005;R:R1006;R:R1003;R:R1003;C:R1003' then Error('update %1', L.Dump());
            L.Clear(); P.First(); if P.NoCtl.Value() <> 'R1003' then Error('first %1', P.NoCtl.Value()); if L.Dump() <> '' then Error('after first %1', L.Dump());
        end;"#,
    );
}

// AiListOrderTests.al:174 Same_Nav_OnOpenRow: right after CurrPage.Update on a list with OnFindRecord and OnNextRecord, First on the
// first row reads nothing (a field read between them is allowed). Other shapes keep the
// earlier rules.
#[test]
fn first_on_the_first_row_of_a_find_and_next_list_reads_nothing_only_after_update() {
    passes(
        r#"[Test] procedure AfterUpdate() var L: Codeunit "LUW Log"; P: TestPage "LUW Nav"; begin
            L.Seed(); P.OpenView(); P.Same.Invoke();
            if P.NoCtl.Value() <> 'R1001' then Error('current %1', P.NoCtl.Value());
            L.Clear(); if not P.First() then Error('first'); if L.Dump() <> '' then Error('after first %1', L.Dump());
        end;"#,
    );
}

// AiListOrderTests.al:39, :233 and QgGoToKeyNextCall.al: after a GoToKey that moved a list without OnAfterGetCurrRecord, an action,
// a Value read, SetValue, Previous and a RunObject action are pinned (al-vm tests/pages_ui/goto_next_call.rs); Next, GoToKey, First
// and Last read only what they read (AiListOrderTests.al:1012-1021, TestPageRuntimeContracts.al:342-357). Any other call next, an
// action after a property read other than those pages/lt-wd/WdGoToKeyTypedReads.al pins (ShowMandatory here), and Close right after
// such a Previous are not pinned: refused.
#[test]
fn goto_key_on_a_list_without_current_record_trigger_then_another_call_is_refused() {
    refused(
        r#"[Test] procedure ThenSetFilter() var L: Codeunit "LUW Log"; P: TestPage "LUW Plain"; begin
            L.Seed(); P.OpenView(); P.GoToKey('R1002'); P.Filter.SetFilter(Qty, '<5');
        end;
        [Test] procedure ThenShowMandatory() var L: Codeunit "LUW Log"; P: TestPage "LUW Plain"; begin
            L.Seed(); P.OpenView(); P.GoToKey('R1002'); if P.NoCtl.ShowMandatory() then; P.KeepFilter.Invoke();
        end;
        [Test] procedure ThenPreviousThenClose() var L: Codeunit "LUW Log"; P: TestPage "LUW Plain"; begin
            L.Seed(); P.OpenView(); P.GoToKey('R1002'); P.Previous(); P.Close();
        end;"#,
    );
}

#[test]
fn goto_key_on_a_list_without_current_record_trigger_then_next_reads_nothing_more() {
    passes(
        r#"[Test] procedure ThenNext() var L: Codeunit "LUW Log"; P: TestPage "LUW Plain"; begin
            L.Seed(); P.OpenView(); P.GoToKey('R1002'); L.Clear(); P.Next(); if L.Dump() <> '' then Error('next %1', L.Dump());
        end;"#,
    );
}

// pages/lt-ai-list-order/QgGoToKeyNextCall.al:223 Plain_GoToKey_ThenValueRead, :277 Plain_GoToKey_ThenClose, :1049 Nav_GoToKey_ThenClose:
// a GoToKey that moved a list without OnAfterGetCurrRecord and read nothing (the key is among the rows the client holds): Close reads
// the row once, also after field value reads.
#[test]
fn goto_key_that_read_nothing_then_close_reads_the_row_once() {
    passes(
        r#"[Test] procedure ThenClose() var L: Codeunit "LUW Log"; P: TestPage "LUW Plain"; begin
            L.Seed(); P.OpenView(); L.Clear(); P.GoToKey('R1003'); if L.Dump() <> '' then Error('goto %1', L.Dump());
            P.Close(); if L.Dump() <> 'R:R1003' then Error('close %1', L.Dump());
        end;
        [Test] procedure ThenValueThenClose() var L: Codeunit "LUW Log"; P: TestPage "LUW Plain"; begin
            L.Seed(); P.OpenView(); P.GoToKey('R1003'); L.Clear();
            if P.NoCtl.Value() <> 'R1003' then Error('no'); if L.Dump() <> '' then Error('value %1', L.Dump());
            P.Close(); if L.Dump() <> 'R:R1003' then Error('close %1', L.Dump());
        end;
        [Test] procedure NavThenClose() var L: Codeunit "LUW Log"; P: TestPage "LUW Nav"; begin
            L.Seed(); P.OpenView(); P.GoToKey('R1003'); L.Clear();
            P.Close(); if L.Dump() <> 'R:R1003' then Error('close %1', L.Dump());
        end;"#,
    );
}
