//! GoToRecord of a row inserted while a list is open on row B (corpus branch al-runtime-rs-staging-y65 eefaf01b,
//! pages/lt-y65-gotorecord-action-insert/Y65GoToRecordActionInsert.al:593-657, al-runner issue 6671):
//! - a row an action inserted, before the current row (A) or after it (C), with `CurrPage.Update()`, `Update(false)` or no
//!   update: GoToRecord finds it on a Worksheet with both row triggers, with neither, with only one of them, and on a List with
//!   both;
//! - a row the test inserted itself: 'The row does not exist on the TestPage.' (TestRowNotFound) on the pages with
//!   OnAfterGetCurrRecord, found on the pages without it.

use al_builtins::value::Value;

fn page(id: u32, name: &str, page_type: &str, triggers: &str) -> String {
    format!(
        r#"
page {id} "{name}"
{{
    PageType = {page_type}; SourceTable = "GTR Item"; ApplicationArea = All; UsageCategory = None;
    layout {{ area(Content) {{ repeater(Lines) {{ field("No."; Rec."No.") {{ }} field(Description; Rec.Description) {{ }} }} }} }}
    actions {{ area(Processing) {{
        action(InsAUpdate) {{ trigger OnAction() begin Rec.AddRow('A'); CurrPage.Update(); end; }}
        action(InsAUpdateFalse) {{ trigger OnAction() begin Rec.AddRow('A'); CurrPage.Update(false); end; }}
        action(InsANoUpdate) {{ trigger OnAction() begin Rec.AddRow('A'); end; }}
        action(InsCUpdate) {{ trigger OnAction() begin Rec.AddRow('C'); CurrPage.Update(); end; }}
    }} }}
    {triggers}
}}"#
    )
}

fn run(name: &str, page: &str) -> String {
    format!(
        r#"
    [TryFunction]
    local procedure TryGo{name}(var P: TestPage "{page}"; Mode: Integer)
    var Item: Record "GTR Item";
    begin
        Item.Get(Target(Mode));
        P.GoToRecord(Item);
    end;
    procedure Run{name}(Mode: Integer): Text
    var Item: Record "GTR Item"; P: TestPage "{page}"; Shown: Text;
    begin
        Seed();
        P.OpenEdit();
        case Mode of
            1: P.InsAUpdate.Invoke();
            2: P.InsAUpdateFalse.Invoke();
            3: P.InsANoUpdate.Invoke();
            4: P.InsCUpdate.Invoke();
            5: Item.AddRow('A');
        end;
        Shown := 'on ' + P."No.".Value() + ' ';
        ClearLastError();
        if TryGo{name}(P, Mode) then
            Shown := 'ok:' + Shown + P.Description.Value()
        else
            Shown := Shown + '|error:' + GetLastErrorText() + '|' + GetLastErrorCode() + '|now ' + P."No.".Value();
        P.Close();
        exit(Shown);
    end;"#
    )
}

fn src() -> String {
    let get = "trigger OnAfterGetRecord() begin end;";
    let curr = "trigger OnAfterGetCurrRecord() begin end;";
    let mut s = String::from(
        r#"
table 69980 "GTR Item"
{
    fields { field(1; "No."; Code[20]) { } field(2; Description; Text[50]) { } }
    keys { key(PK; "No.") { } }
    procedure AddRow(NewNo: Code[20])
    var Item: Record "GTR Item";
    begin
        Item."No." := NewNo; Item.Description := 'Inserted ' + NewNo; Item.Insert();
    end;
}"#,
    );
    s += &page(69981, "GTR Both", "Worksheet", &format!("{get} {curr}"));
    s += &page(69982, "GTR None", "Worksheet", "");
    s += &page(69983, "GTR GetRecord", "Worksheet", get);
    s += &page(69984, "GTR GetCurrRecord", "Worksheet", curr);
    s += &page(69985, "GTR List Both", "List", &format!("{get} {curr}"));
    s += r#"
codeunit 69986 C
{
    local procedure Seed()
    var Item: Record "GTR Item";
    begin
        Item.DeleteAll(); Item."No." := 'B'; Item.Description := 'Existing'; Item.Insert();
    end;
    local procedure Target(Mode: Integer): Code[20]
    begin
        if Mode = 4 then exit('C');
        exit('A');
    end;"#;
    for (n, p) in [
        ("Both", "GTR Both"),
        ("None", "GTR None"),
        ("GetRecord", "GTR GetRecord"),
        ("GetCurrRecord", "GTR GetCurrRecord"),
        ("ListBoth", "GTR List Both"),
    ] {
        s += &run(n, p);
    }
    s += "\n}\n";
    s
}

fn outcomes(name: &str) -> Vec<String> {
    let mut b = al_testkit::Built::from_source(&src());
    (1..=5)
        .map(|mode| {
            let (o, r) = b.routine_in("C", &format!("Run{name}"));
            match b.call(o, r, vec![Value::Integer(mode)]) {
                Ok((v, _)) => al_testkit::text(Ok(v)),
                Err(e) => format!("signal: {e:?}"),
            }
        })
        .collect()
}

const FOUND_BY_ACTION: [&str; 4] = [
    "ok:on B Inserted A",
    "ok:on B Inserted A",
    "ok:on B Inserted A",
    "ok:on B Inserted C",
];
const TEST_INSERT_FOUND: &str = "ok:on B Inserted A";
const TEST_INSERT_NOT_FOUND: &str = "on B |error:The row does not exist on the TestPage.|TestRowNotFound|now B";

fn expect(name: &str, test_insert: &str) {
    let mut want: Vec<String> = FOUND_BY_ACTION.iter().map(|s| s.to_string()).collect();
    want.push(test_insert.to_string());
    assert_eq!(outcomes(name), want);
}

#[test]
fn a_worksheet_with_both_row_triggers() {
    // Y65GoToRecordActionInsert.al:593.
    expect("Both", TEST_INSERT_NOT_FOUND);
}

#[test]
fn a_worksheet_without_row_triggers() {
    // :609.
    expect("None", TEST_INSERT_FOUND);
}

#[test]
fn a_worksheet_with_on_after_get_record_only() {
    // :625.
    expect("GetRecord", TEST_INSERT_FOUND);
}

#[test]
fn a_worksheet_with_on_after_get_curr_record_only() {
    // :641.
    expect("GetCurrRecord", TEST_INSERT_NOT_FOUND);
}

#[test]
fn a_list_with_both_row_triggers() {
    // :657.
    expect("ListBoth", TEST_INSERT_NOT_FOUND);
}
