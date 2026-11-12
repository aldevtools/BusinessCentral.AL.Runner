//! IsolatedStorage reads the corpus pins (a key of more than 200 characters) and the shapes next to them it does not pin,
//! which refuse: a missing key as a statement outside an app with an id, the user scopes, Get of a long key, isSecret of a value stored from a SecretText, and
//! a page part that enqueues a second background task (al-runner issue 7576).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69980 "ISR Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69981 "ISR Worker"
{
    trigger OnRun()
    var Results: Dictionary of [Text, Text];
    begin
        Results.Add('ok', 'yes');
        Page.SetBackgroundTaskResult(Results);
    end;
}
page 69982 "ISR Part"
{
    PageType = CardPart; SourceTable = "ISR Row";
    layout { area(Content) { field(Id; Rec.Id) { ApplicationArea = All; } } }
    var TaskId: Integer;
    trigger OnAfterGetCurrRecord()
    begin
        if Rec.Id = 0 then exit;
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"ISR Worker");
    end;
}
page 69983 "ISR Host"
{
    PageType = List; SourceTable = "ISR Row"; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Rows) { field(Id; Rec.Id) { ApplicationArea = All; } } }
        area(FactBoxes) { part(Details; "ISR Part") { ApplicationArea = All; SubPageLink = Id = field(Id); } }
    }
}
codeunit 69984 T
{
    Subtype = Test;
    TestPermissions = Disabled;
    local procedure Check(Want: Text; Got: Text) begin if Want <> Got then Error('want [%1] got [%2]', Want, Got); end;
    [Test] procedure DeleteMissingModule()
    begin
        asserterror IsolatedStorage.Delete('ISRmissing', DataScope::Module);
        Check('DB:RecordNotFound', GetLastErrorCode());
        Check('Key=''ISRmissing''', CopyStr(GetLastErrorText(), StrLen(GetLastErrorText()) - 16));
    end;
    [Test] procedure SetLongKey()
    begin
        asserterror IsolatedStorage.Set(PadStr('', 201, 'k'), 'v');
        Check('Key must not be longer than 200.', GetLastErrorText());
    end;
    [Test] procedure GetMissingUserScopeRefused()
    var V: Text;
    begin
        IsolatedStorage.Get('ISRmissing', DataScope::User, V);
    end;
    [Test] procedure GetLongKeyRefused()
    var V: Text;
    begin
        IsolatedStorage.Get(PadStr('', 201, 'k'), V);
    end;
    [Test] procedure ContainsSecretOfSecretTextRefused()
    var S: SecretText; IsSecret: Boolean;
    begin
        S := 'hidden';
        IsolatedStorage.SetEncrypted('ISRsecret', S);
        IsolatedStorage.Contains('ISRsecret', DataScope::Module, IsSecret);
    end;
    [Test] procedure PartSecondEnqueueRefused()
    var Row: Record "ISR Row"; Host: TestPage "ISR Host";
    begin
        Row.Id := 1; Row.Insert(); Commit();
        Host.OpenView();
        Host.GoToRecord(Row);
        Host.Close();
    end;
}
"#;

#[test]
fn isolated_storage_pinned_errors_and_refusals() {
    let out = outcomes_in(SRC, "T");
    let get = |n: &str| out.iter().find(|(t, _)| t == n).map(|(_, o)| o.clone()).unwrap();
    assert_eq!(get("SetLongKey"), Outcome::Pass);
    for (name, why) in [
        // The snippet's app has no id: the error text would name it.
        ("DeleteMissingModule", "outside an app with an id"),
        ("GetMissingUserScopeRefused", "in a user scope"),
        ("GetLongKeyRefused", "more than 200 characters"),
        ("ContainsSecretOfSecretTextRefused", "stored from a SecretText"),
        ("PartSecondEnqueueRefused", "a second time on a page part"),
    ] {
        match get(name) {
            Outcome::NotRun(reason) => assert!(reason.contains(why), "{name}: {reason}"),
            other => panic!("{name}: {other:?}"),
        }
    }
}
