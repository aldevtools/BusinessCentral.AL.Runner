//! al-runner issue 6021 (the reporter's shape): a bare `Update()` in a page's OnAfterGetRecord, and bare page methods in
//! an action, run as their `CurrPage.` forms do. The resolution is alc's (a bare `Activate()` in a page compiles:
//! pagefindrecord/PageFindRecordList.Page.al:65); what each call does is the `CurrPage.` behavior.

fn src(prefix: &str) -> String {
    format!(
        r#"
table 69980 "BP Customer"
{{
    fields {{ field(1; "No."; Code[20]) {{ }} field(2; Name; Text[100]) {{ }} }}
    keys {{ key(PK; "No.") {{ Clustered = true; }} }}
}}
page 69981 "BP Card"
{{
    PageType = Card;
    SourceTable = "BP Customer";
    layout {{ area(Content) {{ field(Name; Rec.Name) {{ }} }} }}
    actions {{ area(Processing) {{ action(Refresh) {{ trigger OnAction() begin {prefix}Update(false); end; }} }} }}
    trigger OnAfterGetRecord()
    begin
        {prefix}Update();
    end;
}}
codeunit 69982 C
{{
    procedure Run(): Text
    var
        Customer: Record "BP Customer";
        Card: TestPage "BP Card";
        Result: Text;
    begin
        Customer.DeleteAll();
        Customer.Init();
        Customer."No." := 'REPRO1';
        Customer.Name := 'Repro';
        Customer.Insert();
        Card.OpenView();
        Card.GoToRecord(Customer);
        Result := Card.Name.Value();
        Card.Refresh.Invoke();
        Result += ':' + Card.Name.Value();
        Card.Close();
        exit(Result);
    end;
}}
"#
    )
}

#[test]
fn a_bare_update_in_on_after_get_record_runs_as_currpage_update() {
    let qualified = format!("{:?}", al_testkit::run_in(&src("CurrPage."), "C", "Run"));
    let bare = format!("{:?}", al_testkit::run_in(&src(""), "C", "Run"));
    assert_eq!(bare, qualified);
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src(""), "C", "Run")),
        "Repro:Repro"
    );
}
