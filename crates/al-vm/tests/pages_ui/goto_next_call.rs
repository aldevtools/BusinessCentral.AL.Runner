//! The call after a GoToKey that moved a list without OnAfterGetCurrRecord (pages/rows.rs, pages/actions.rs, pages/pageops.rs).
//! Each test names the corpus test of pages/list-order/QgGoToKeyNextCall.al that pins what BC does.

const SRC: &str = r#"
table 69960 "GN Log" { fields { field(1; Id; Integer) { } field(2; Txt; Text[250]) { } } keys { key(PK; Id) { } } }
codeunit 69961 "GN Log Mgt"
{
    procedure Add(Value: Text)
    var L: Record "GN Log"; N: Integer;
    begin
        if L.FindLast() then N := L.Id;
        L.Init(); L.Id := N + 1; L.Txt := CopyStr(Value, 1, 250); L.Insert();
    end;
    procedure Take(): Text
    var L: Record "GN Log"; R: Text;
    begin
        if L.FindSet() then
            repeat
                if R <> '' then R += ';';
                R += L.Txt;
            until L.Next() = 0;
        L.DeleteAll();
        exit(R);
    end;
}
table 69962 "GN Row" { fields { field(1; Id; Integer) { } field(2; Qty; Integer) { } } keys { key(PK; Id) { } } }
page 69963 "GN Plain"
{
    PageType = List; SourceTable = "GN Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(Qty; Rec.Qty) { } } } }
    actions
    {
        area(Processing)
        {
            action(Act) { trigger OnAction() var Log: Codeunit "GN Log Mgt"; begin Log.Add('A:' + Format(Rec.Id)); end; }
            action(OpenTarget) { RunObject = page "GN Target"; }
        }
    }
    trigger OnAfterGetRecord() var Log: Codeunit "GN Log Mgt"; begin Log.Add('R:' + Format(Rec.Id)); end;
}
page 69964 "GN Target"
{
    PageType = Card; ApplicationArea = All; UsageCategory = None;
    trigger OnOpenPage() var Log: Codeunit "GN Log Mgt"; begin Log.Add('T:open'); end;
}
codeunit 69965 C
{
    procedure Seed()
    var Row: Record "GN Row"; i: Integer; Log: Codeunit "GN Log Mgt";
    begin
        for i := 1 to 6 do begin Row.Init(); Row.Id := i; Row.Qty := i; Row.Insert(); end;
        Log.Take();
    end;

    procedure SetValueThenActions(): Text
    var P: TestPage "GN Plain"; Log: Codeunit "GN Log Mgt"; R: Text;
    begin
        Seed();
        P.OpenEdit();
        P.GoToKey(3);
        Log.Take();
        P.Qty.SetValue(33);
        R := Log.Take();
        P.Act.Invoke();
        R += '|' + Log.Take();
        P.Act.Invoke();
        R += '|' + Log.Take();
        P.Close();
        R += '|' + Log.Take();
        exit(R);
    end;

    procedure ValueReadThenAction(): Text
    var P: TestPage "GN Plain"; Log: Codeunit "GN Log Mgt"; R: Text;
    begin
        Seed();
        P.OpenView();
        P.GoToKey(3);
        Log.Take();
        R := P.Id.Value() + ':' + Log.Take();
        P.Act.Invoke();
        R += '|' + Log.Take();
        exit(R);
    end;

    procedure PreviousThenAction(): Text
    var P: TestPage "GN Plain"; Log: Codeunit "GN Log Mgt"; R: Text;
    begin
        Seed();
        P.OpenView();
        P.GoToKey(3);
        Log.Take();
        R := Format(P.Previous()) + ':' + Log.Take();
        R += '|' + P.Id.Value() + ':' + Log.Take();
        P.Act.Invoke();
        R += '|' + Log.Take();
        exit(R);
    end;

}
"#;

/// The trigger reads of one step, up to and with the action's own entry (the refresh after it is pinned elsewhere).
fn upto_action(step: &str) -> &str {
    match step.find("A:") {
        Some(i) => &step[..step[i..].find(';').map_or(step.len(), |j| i + j)],
        None => step,
    }
}

/// QgGoToKeyNextCall.al:236 Plain_GoToKey_ThenSetValue_ThenAction (`setvalue_log` empty, `action_log` `R:R1003;R:R1003;A:act`,
/// `close_log` `R:R1003`) and :297 Plain_GoToKey_ThenActionWithoutUpdate_Twice (the second action reads the row twice too).
#[test]
fn every_action_after_the_go_to_key_reads_the_row_twice() {
    let mut built = al_testkit::Built::from_source(SRC);
    let out = al_testkit::text(built.run_in("C", "SetValueThenActions"));
    let steps: Vec<&str> = out.split('|').collect();
    assert_eq!(steps[0], "", "{out}");
    assert_eq!(upto_action(steps[1]), "R:3;R:3;A:3", "{out}");
    assert_eq!(upto_action(steps[2]), "R:3;R:3;A:3", "{out}");
    assert_eq!(steps[3], "R:3", "{out}");
}

/// QgGoToKeyNextCall.al:317 Plain_GoToKey_ThenValueRead_ThenAction.
#[test]
fn an_action_after_a_value_read_reads_the_row_twice() {
    let mut built = al_testkit::Built::from_source(SRC);
    let out = al_testkit::text(built.run_in("C", "ValueReadThenAction"));
    let steps: Vec<&str> = out.split('|').collect();
    assert_eq!(steps[0], "3:", "{out}");
    assert_eq!(upto_action(steps[1]), "R:3;R:3;A:3", "{out}");
}

/// QgGoToKeyNextCall.al:254 Plain_GoToKey_ThenPrevious (`previous_log` and `no_log` empty, the action reads R1002 twice).
#[test]
fn previous_reads_nothing_and_the_action_after_it_reads_the_new_row_twice() {
    let mut built = al_testkit::Built::from_source(SRC);
    let out = al_testkit::text(built.run_in("C", "PreviousThenAction"));
    let steps: Vec<&str> = out.split('|').collect();
    assert_eq!(steps[0], "Yes:", "{out}");
    assert_eq!(steps[1], "2:", "{out}");
    assert_eq!(upto_action(steps[2]), "R:2;R:2;A:2", "{out}");
}
