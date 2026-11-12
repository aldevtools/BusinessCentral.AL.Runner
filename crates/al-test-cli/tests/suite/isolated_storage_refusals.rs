//! IsolatedStorage reads the corpus pins (a key of more than 200 characters) and the shapes next to them it does not pin,
//! which refuse: a missing key as a statement outside an app with an id, the user scopes, Get of a long key, isSecret of a value stored from a SecretText.
//! A page part that enqueues a second background task for the same record runs it (al-runner issue 7576; Microsoft's tests
//! need it, see `bg_task_part_moves.rs`).

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
    [Test] procedure PartSecondEnqueue()
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
    assert_eq!(get("PartSecondEnqueue"), Outcome::Pass);
    for (name, why) in [
        // The snippet's app has no id: the error text would name it.
        ("DeleteMissingModule", "outside an app with an id"),
        ("GetMissingUserScopeRefused", "in a user scope"),
        ("GetLongKeyRefused", "more than 200 characters"),
        ("ContainsSecretOfSecretTextRefused", "stored from a SecretText"),
    ] {
        match get(name) {
            Outcome::NotRun(reason) => assert!(reason.contains(why), "{name}: {reason}"),
            other => panic!("{name}: {other:?}"),
        }
    }
}

/// The objects of an app with a Guid id (the missing-key error text names it) that the cases below share: a card to run a
/// page background task on and a worker that reads a key with Get as a statement.
const OBJECTS: &str = r#"
table 69970 "ISR2 Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
page 69971 "ISR2 Card" { PageType = Card; SourceTable = "ISR2 Row"; layout { area(Content) { field(Id; Rec.Id) { ApplicationArea = All; } } } }
codeunit 69972 "ISR2 Worker"
{
    trigger OnRun()
    var Params: Dictionary of [Text, Text]; Results: Dictionary of [Text, Text]; V: Text;
    begin
        Params := Page.GetBackgroundParameters();
        IsolatedStorage.Get(Params.Get('Key'), DataScope::Module, V);
        Results.Add('V', V);
        Page.SetBackgroundTaskResult(Results);
    end;
}
"#;

/// The outcome of the one test `P` with body `body` (and local variables `vars`) in its own test codeunit, so a refusal
/// cannot change the outcome of another case.
fn one(vars: &str, body: &str) -> Outcome {
    let src = format!(
        "{OBJECTS}\ncodeunit 69979 T {{ Subtype = Test; TestPermissions = Disabled;\n  local procedure Check(Want: Text; Got: Text) begin if Want <> Got then Error('want [%1] got [%2]', Want, Got); end;\n  [Test] procedure P() {vars} begin {body} end; }}"
    );
    let app = al_sema::workspace::AppSource {
        id: "11111111-2222-3333-4444-555555555579".into(),
        name: "ISR2".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), src)],
    };
    let out = al_testkit::outcomes_in_app(app, "T");
    out.into_iter().find(|(t, _)| t == "P").map(|(_, o)| o).unwrap()
}

/// A worker reads a key the test deleted without a commit after an earlier commit stored it (review of pull request
/// 7828): only a committed Delete is pinned (IsolatedStorageInBackgroundTask.al:513-526).
const WORKER_RUN: &str = "P.Add('Key', 'ISR2k'); Card.OpenView(); R := Card.RunPageBackgroundTask(Codeunit::\"ISR2 Worker\", P, false); Card.Close();";

#[test]
fn isolated_storage_pinned_key_forms_run_and_the_unpinned_ones_refuse() {
    let long201 = "PadStr('', 201, 'k')";
    // Pinned: Set and Contains of a key of more than 200 characters in the default scope (errors/errorinfo/EnTSecret.al:
    // 250-283), Contains of a key Set with a trailing space (:182-183, :199), a missing key as a statement in the Module
    // scope (:127-128).
    for (name, vars, body) in [
        ("long Set", "", format!("asserterror IsolatedStorage.Set({long201}, 'v'); Check('Key must not be longer than 200.', GetLastErrorText());")),
        ("long Contains", "", format!("asserterror IsolatedStorage.Contains({long201}); Check('Key must not be longer than 200.', GetLastErrorText());")),
        ("trailing Contains", "", "IsolatedStorage.Set('ISR2 trail ', 't'); if not IsolatedStorage.Contains('ISR2 trail') then Error('not found');".to_string()),
        ("missing Get", "var V: Text;", "asserterror IsolatedStorage.Get('ISR2none', V); Check('DB:RecordNotFound', GetLastErrorCode());".to_string()),
    ] {
        assert_eq!(one(vars, &body), Outcome::Pass, "{name}");
    }
    let worker_vars = "var Card: TestPage \"ISR2 Card\"; P: Dictionary of [Text, Text]; R: Dictionary of [Text, Text];";
    for (vars, body, why) in [
        (
            "",
            format!("IsolatedStorage.SetEncrypted({long201}, 'v');"),
            "200 characters",
        ),
        (
            "",
            format!("IsolatedStorage.Set({long201}, 'v', DataScope::User);"),
            "200 characters",
        ),
        (
            "",
            format!("IsolatedStorage.Set({long201}, 'v', DataScope::Module);"),
            "200 characters",
        ),
        (
            "",
            "IsolatedStorage.Set(PadStr('', 200, 'k') + ' ', 'v');".to_string(),
            "200 characters",
        ),
        (
            "var K: Text; i: Integer;",
            "for i := 1 to 150 do K += '😀'; IsolatedStorage.Set(K, 'v');".to_string(),
            "200 characters",
        ),
        (
            "var V: Text;",
            "IsolatedStorage.Set('ISR2 t2 ', 'v'); IsolatedStorage.Get('ISR2 t2', V);".to_string(),
            "trailing spaces",
        ),
        (
            "",
            "IsolatedStorage.Set('ISR2 t3 ', 'v'); IsolatedStorage.Delete('ISR2 t3');".to_string(),
            "trailing spaces",
        ),
        (
            "",
            "IsolatedStorage.Set('ISR2 t4', 'v'); IsolatedStorage.Contains('ISR2 t4 ');".to_string(),
            "trailing spaces",
        ),
        (
            "",
            "IsolatedStorage.Set('', 'e'); IsolatedStorage.Contains('   ');".to_string(),
            "trailing spaces",
        ),
        (
            "var V: Text; G: Guid;",
            "G := '{11111111-2222-3333-4444-555555555555}'; IsolatedStorage.Get(G, DataScope::Module, V);".to_string(),
            "Guid key",
        ),
        (
            "var S: SecretText;",
            "IsolatedStorage.Get('ISR2none', DataScope::Module, S);".to_string(),
            "SecretText",
        ),
        (
            worker_vars,
            format!(
                "IsolatedStorage.Set('ISR2k', 'v', DataScope::Module); Commit(); IsolatedStorage.Delete('ISR2k', DataScope::Module); {WORKER_RUN}"
            ),
            "committed copy",
        ),
    ] {
        match one(vars, &body) {
            Outcome::NotRun(reason) => assert!(reason.contains(why), "{body}: {reason}"),
            other => panic!("{body}: {other:?}"),
        }
    }
}
