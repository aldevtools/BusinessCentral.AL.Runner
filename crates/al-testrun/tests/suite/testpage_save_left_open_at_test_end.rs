//! A value set on a card TestPage that is still open when the test ends is saved when the card closes then, as `Close()`
//! saves it, with and without OnQueryClosePage, for OpenEdit, OpenNew on a new key and OpenNew then GoToRecord of an existing
//! row; a value whose OnValidate failed is not saved, a valid value set before it is lost too, and OnQueryClosePage does not run
//! (corpus pages/lifecycle/Y7345SaveOnLeftOpen.al, al-runner issue 7345). The source is the corpus file with its observer
//! replaced by plain checks.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 99740 "LT SC Setting"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Url; Text[100])
        {
            trigger OnValidate()
            begin
                if Url.StartsWith('bad') then
                    Error('Url must not start with bad.');
            end;
        }
    }

    keys
    {
        key(PK; "Code") { Clustered = true; }
    }
}

page 99741 "LT SC Card"
{
    PageType = Card;
    SourceTable = "LT SC Setting";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("Code"; Rec."Code") { ApplicationArea = All; }
            field(Url; Rec.Url) { ApplicationArea = All; }
        }
    }
}

page 99742 "LT SC Card Q"
{
    PageType = Card;
    SourceTable = "LT SC Setting";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("Code"; Rec."Code") { ApplicationArea = All; }
            field(Url; Rec.Url) { ApplicationArea = All; }
        }
    }

    trigger OnQueryClosePage(CloseAction: Action): Boolean
    var
        Log: Codeunit "LT SC Log";
    begin
        Log.Add('Q:' + Rec."Code" + ':' + Format(CloseAction) + ':' + Rec.Url);
        exit(true);
    end;
}

codeunit 99743 "LT SC Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Take() Result: Text
    begin
        Result := Entries;
        Entries := '';
    end;
}

codeunit 99744 "LT SC Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT SC Log";

    local procedure Seed(Code: Code[20])
    var
        Setting: Record "LT SC Setting";
    begin
        Setting."Code" := Code;
        Setting.Url := 'old';
        Setting.Insert();
    end;

    local procedure ReadRow(Code: Code[20]): Text
    var
        Setting: Record "LT SC Setting";
    begin
        if not Setting.Get(Code) then
            exit('missing');
        exit(Setting.Url);
    end;

    [TryFunction]
    local procedure TrySetUrl(var P: TestPage "LT SC Card"; Value: Text)
    begin
        P.Url.SetValue(Value);
    end;

    [TryFunction]
    local procedure TrySetUrlQ(var P: TestPage "LT SC Card Q"; Value: Text)
    begin
        P.Url.SetValue(Value);
    end;

    local procedure Note(Name: Text; Done: Boolean)
    begin
        if Done then
            Log.Add(Name + ':ok')
        else
            Log.Add(Name + ':' + GetLastErrorText());
        ClearLastError();
    end;

    // The card stays open when these tests end.

    [Test]
    procedure Y7345_Write1_EditValid_NoQuery()
    var
        P: TestPage "LT SC Card";
    begin
        Seed('E1');
        P.OpenEdit();
        P.GoToRecord(GetSetting('E1'));
        Note('E1', TrySetUrl(P, 'new1'));
    end;

    [Test]
    procedure Y7345_Read1_NextTestSeesEdit()
    begin
        Check('E1 after the test that left the card open', ReadRow('E1'), 'new1');
    end;

    [Test]
    procedure Y7345_Write2_EditValid_Query()
    var
        P: TestPage "LT SC Card Q";
    begin
        Seed('E2');
        P.OpenEdit();
        P.GoToRecord(GetSetting('E2'));
        Note('E2', TrySetUrlQ(P, 'new2'));
    end;

    [Test]
    procedure Y7345_Write3_EditInvalid_NoQuery()
    var
        P: TestPage "LT SC Card";
    begin
        Seed('E3');
        P.OpenEdit();
        P.GoToRecord(GetSetting('E3'));
        Note('E3', TrySetUrl(P, 'bad3'));
    end;

    [Test]
    procedure Y7345_Write4_EditInvalid_Query()
    var
        P: TestPage "LT SC Card Q";
    begin
        Seed('E4');
        P.OpenEdit();
        P.GoToRecord(GetSetting('E4'));
        Note('E4', TrySetUrlQ(P, 'bad4'));
    end;

    [Test]
    procedure Y7345_Write5_EditValidThenInvalid_Query()
    var
        P: TestPage "LT SC Card Q";
    begin
        Seed('E5');
        P.OpenEdit();
        P.GoToRecord(GetSetting('E5'));
        Note('E5a', TrySetUrlQ(P, 'new5'));
        Note('E5b', TrySetUrlQ(P, 'bad5'));
    end;

    [Test]
    procedure Y7345_Write6_OpenNewNewKey_NoQuery()
    var
        P: TestPage "LT SC Card";
    begin
        P.OpenNew();
        P."Code".SetValue('N6');
        Note('N6', TrySetUrl(P, 'new6'));
    end;

    [Test]
    procedure Y7345_Write7_OpenNewNewKey_Query()
    var
        P: TestPage "LT SC Card Q";
    begin
        P.OpenNew();
        P."Code".SetValue('N7');
        Note('N7', TrySetUrlQ(P, 'new7'));
    end;

    [Test]
    procedure Y7345_Write8_OpenNewGoToExisting_NoQuery()
    var
        P: TestPage "LT SC Card";
    begin
        Seed('E8');
        P.OpenNew();
        P.GoToRecord(GetSetting('E8'));
        Note('E8', TrySetUrl(P, 'new8'));
    end;

    [Test]
    procedure Y7345_Write9_OpenNewGoToExisting_Query()
    var
        P: TestPage "LT SC Card Q";
    begin
        Seed('E9');
        P.OpenNew();
        P.GoToRecord(GetSetting('E9'));
        Note('E9', TrySetUrlQ(P, 'new9'));
    end;

    [Test]
    procedure Y7345_Write10_EditValid_ExplicitClose()
    var
        P: TestPage "LT SC Card";
    begin
        Seed('E10');
        P.OpenEdit();
        P.GoToRecord(GetSetting('E10'));
        Note('E10', TrySetUrl(P, 'new10'));
        P.Close();
    end;

    [Test]
    procedure Y7345_Write11_OpenNewNewKeyInvalid_NoQuery()
    var
        P: TestPage "LT SC Card";
    begin
        P.OpenNew();
        P."Code".SetValue('N11');
        Note('N11', TrySetUrl(P, 'bad11'));
    end;

    [Test]
    procedure Y7345_Write12_OpenNewNewKeyInvalid_Query()
    var
        P: TestPage "LT SC Card Q";
    begin
        P.OpenNew();
        P."Code".SetValue('N12');
        Note('N12', TrySetUrlQ(P, 'bad12'));
    end;

    [Test]
    procedure Y7345_Read2_RowsAfterAllCards()
    begin
        Check('row E1', ReadRow('E1'), 'new1');
        Check('row E2', ReadRow('E2'), 'new2');
        Check('row E3', ReadRow('E3'), 'old');
        Check('row E4', ReadRow('E4'), 'old');
        Check('row E5', ReadRow('E5'), 'old');
        Check('row N6', ReadRow('N6'), 'new6');
        Check('row N7', ReadRow('N7'), 'new7');
        Check('row E8', ReadRow('E8'), 'new8');
        Check('row E9', ReadRow('E9'), 'new9');
        Check('row E10', ReadRow('E10'), 'new10');
        Check('row N11', ReadRow('N11'), '');
        Check('row N12', ReadRow('N12'), '');
        Check('log', Log.Take(), 'E1:ok;E2:ok;Q:E2:OK:new2;E3:Validation error for Field: Url,  Message = ''Url must not start with bad. (Select Refresh to discard errors)'';E4:Validation error for Field: Url,  Message = ''Url must not start with bad. (Select Refresh to discard errors)'';E5a:ok;E5b:Validation error for Field: Url,  Message = ''Url must not start with bad. (Select Refresh to discard errors)'';N6:ok;N7:ok;Q:N7:OK:new7;E8:ok;E9:ok;Q:E9:OK:new9;E10:ok;N11:Validation error for Field: Url,  Message = ''Url must not start with bad. (Select Refresh to discard errors)'';N12:Validation error for Field: Url,  Message = ''Url must not start with bad. (Select Refresh to discard errors)'';');
    end;

    local procedure Check(What: Text; Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('%1: got %2, want %3', What, Got, Want);
    end;

    local procedure GetSetting(Code: Code[20]) Setting: Record "LT SC Setting"
    begin
        Setting.Get(Code);
    end;
}
"#;

#[test]
fn a_value_set_on_a_card_left_open_is_saved_when_the_test_ends() {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    let wrong = [
        "Y7345_Write1_EditValid_NoQuery",
        "Y7345_Read1_NextTestSeesEdit",
        "Y7345_Write2_EditValid_Query",
        "Y7345_Write3_EditInvalid_NoQuery",
        "Y7345_Write4_EditInvalid_Query",
        "Y7345_Write5_EditValidThenInvalid_Query",
        "Y7345_Write6_OpenNewNewKey_NoQuery",
        "Y7345_Write7_OpenNewNewKey_Query",
        "Y7345_Write8_OpenNewGoToExisting_NoQuery",
        "Y7345_Write9_OpenNewGoToExisting_Query",
        "Y7345_Write10_EditValid_ExplicitClose",
        "Y7345_Write11_OpenNewNewKeyInvalid_NoQuery",
        "Y7345_Write12_OpenNewNewKeyInvalid_Query",
        "Y7345_Read2_RowsAfterAllCards",
    ]
    .into_iter()
    .filter_map(|name| {
        let o = rt.first_outcome(&CaseId {
            codeunit: 99744,
            name: name.into(),
        });
        (o != Outcome::Pass).then(|| format!("{name}: {o:?}"))
    })
    .collect::<Vec<_>>();
    assert!(wrong.is_empty(), "{wrong:#?}");
}
