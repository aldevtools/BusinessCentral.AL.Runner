//! A stored record handed to a Card or NavigatePage whose source table is temporary (crates/al-vm/src/pages/open/record_refusals.rs,
//! pages/open/close.rs). Each test names the corpus test of pages/lt-y61-page-probes/Y61StoredRecToTempCard.al that pins it.

const SRC: &str = r#"
table 69970 "TC Log" { fields { field(1; Id; Integer) { } field(2; Txt; Text[250]) { } } keys { key(PK; Id) { } } }
codeunit 69971 "TC Log Mgt"
{
    procedure Add(Value: Text)
    var L: Record "TC Log"; N: Integer;
    begin
        if L.FindLast() then N := L.Id;
        L.Init(); L.Id := N + 1; L.Txt := CopyStr(Value, 1, 250); L.Insert();
    end;
    procedure Take(): Text
    var L: Record "TC Log"; R: Text;
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
table 69972 "TC Row" { fields { field(1; Code; Code[20]) { } field(2; Name; Text[50]) { } field(3; Enabled; Boolean) { } } keys { key(PK; Code) { } } }
page 69973 "TC Card"
{
    PageType = Card; SourceTable = "TC Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(CodeCtl; Rec.Code) { } field(NameCtl; Rec.Name) { } } }
    actions
    {
        area(Processing)
        {
            action(Agree) { trigger OnAction() var L: Codeunit "TC Log Mgt"; begin L.Add('act agree'); Rec.Enabled := true; Rec.Modify(); CurrPage.Close(); end; }
            action(Rename) { trigger OnAction() var L: Codeunit "TC Log Mgt"; begin L.Add('act rename'); Rec.Name := 'renamed'; Rec.Modify(); end; }
            action(DeleteIt) { trigger OnAction() var L: Codeunit "TC Log Mgt"; begin L.Add('act delete'); Rec.Delete(); end; }
        }
    }
    trigger OnOpenPage()
    var L: Codeunit "TC Log Mgt";
    begin
        L.Add('open ' + Rec.Code + '/' + Rec.Name + '/temp=' + Format(Rec.IsTemporary()) + '/count=' + Format(Rec.Count()));
    end;
    trigger OnAfterGetRecord() var L: Codeunit "TC Log Mgt"; begin L.Add('agr ' + Rec.Code + '/' + Rec.Name); end;
    procedure Result(): Text begin exit(Rec.Code + '/' + Rec.Name + '/' + Format(Rec.Enabled) + '/count=' + Format(Rec.Count())); end;
}
codeunit 69974 C
{
    Subtype = Test;
    var
        Mode: Text;
        Seen: Text;

    local procedure Seed()
    var R: Record "TC Row"; L: Codeunit "TC Log Mgt";
    begin
        R.Code := 'N1'; R.Name := 'stored1'; R.Insert();
        R.Code := 'N2'; R.Name := 'stored2'; R.Insert();
        L.Take();
    end;

    local procedure Stored(): Text
    var R: Record "TC Row"; T: Text;
    begin
        if R.FindSet() then
            repeat
                T += R.Code + '/' + R.Name + '/' + Format(R.Enabled) + ',';
            until R.Next() = 0;
        exit(T);
    end;

    [TryFunction]
    local procedure Try(var P: TestPage "TC Card"; Which: Text)
    begin
        case Which of
            'agree': P.Agree.Invoke();
            'rename': P.Rename.Invoke();
            'delete': P.DeleteIt.Invoke();
            'ok': P.OK().Invoke();
        end;
    end;

    local procedure Outcome(Ok: Boolean): Text
    begin
        if Ok then exit('ok');
        exit(GetLastErrorText() + '/' + GetLastErrorCode());
    end;

    [ModalPageHandler]
    procedure Handler(var P: TestPage "TC Card")
    var L: Codeunit "TC Log Mgt";
    begin
        Seen := L.Take() + '|' + P.CodeCtl.Value() + '/' + P.NameCtl.Value();
        case Mode of
            'agree': Seen += '|' + Outcome(Try(P, 'agree'));
            'ok': Seen += '|' + Outcome(Try(P, 'ok'));
            'rename':
                begin
                    Seen += '|' + Outcome(Try(P, 'rename')) + '|' + P.NameCtl.Value() + '|' + Stored();
                    Seen += '|' + Outcome(Try(P, 'ok'));
                end;
            'delete':
                begin
                    Seen += '|' + Outcome(Try(P, 'delete')) + '|' + Stored();
                    Seen += '|' + Outcome(Try(P, 'ok'));
                end;
        end;
        Seen += '|' + L.Take();
    end;

    local procedure SetRecordAgree(): Text
    var PN: Record "TC Row"; R2: Record "TC Row"; P: Page "TC Card"; R: Text;
    begin
        Seed();
        Mode := 'agree';
        PN.Get('N1');
        P.SetRecord(PN);
        R := Format(P.RunModal());
        P.GetRecord(R2);
        exit(Seen + '|' + R + '|' + Stored() + '|' + P.Result() + '|' + R2.Code + '/' + Format(R2.Enabled) + '|' + PN.Name);
    end;

    local procedure SetRecordDelete(): Text
    var PN: Record "TC Row"; P: Page "TC Card"; R: Text;
    begin
        Seed();
        Mode := 'delete';
        PN.Get('N1');
        P.SetRecord(PN);
        R := Format(P.RunModal());
        exit(Seen + '|' + R + '|' + Stored());
    end;

    local procedure RunModalIdRecRename(): Text
    var PN: Record "TC Row"; R: Text;
    begin
        Seed();
        Mode := 'rename';
        PN.Get('N1');
        R := Format(Page.RunModal(Page::"TC Card", PN));
        exit(Seen + '|' + R + '|' + Stored() + '|' + PN.Code + '/' + PN.Name);
    end;

    local procedure RunIdRecTrap(): Text
    var PN: Record "TC Row"; TP: TestPage "TC Card"; L: Codeunit "TC Log Mgt"; R: Text;
    begin
        Seed();
        PN.Get('N1');
        TP.Trap();
        Page.Run(Page::"TC Card", PN);
        R := L.Take() + '|' + TP.CodeCtl.Value() + '|' + Format(TP.Next()) + '|' + L.Take() + '|' + TP.CodeCtl.Value();
        R += '|' + Format(TP.Previous()) + '|' + TP.CodeCtl.Value();
        TP.Close();
        exit(R);
    end;

    local procedure FilteredToNoRow(): Text
    var PN: Record "TC Row"; R: Text;
    begin
        Seed();
        Mode := 'ok';
        PN.SetRange(Code, 'N9');
        R := Format(Page.RunModal(Page::"TC Card", PN));
        exit(Seen + '|' + R + '|' + Stored());
    end;
    [Test] [HandlerFunctions('Handler')] procedure T_SetRecordAgree() begin Error('%1', SetRecordAgree()); end;
    [Test] [HandlerFunctions('Handler')] procedure T_SetRecordDelete() begin Error('%1', SetRecordDelete()); end;
    [Test] [HandlerFunctions('Handler')] procedure T_RunModalIdRecRename() begin Error('%1', RunModalIdRecRename()); end;
    [Test] procedure T_RunIdRecTrap() begin Error('%1', RunIdRecTrap()); end;
    [Test] [HandlerFunctions('Handler')] procedure T_FilteredToNoRow() begin Error('%1', FilteredToNoRow()); end;
}
"#;

const STORED: &str = "N1/stored1/No,N2/stored2/No,";
const GONE: &str =
    "The TC Row does not exist. Identification fields and values: Code=''/TestWrapped:CSideRecordNotFound";

use al_test_cli::report::Outcome;

/// Each test of codeunit C raises its result as the error text.
fn run(proc_name: &str) -> Vec<String> {
    let outcomes = al_testkit::outcomes_in(SRC, "C");
    let (_, outcome) = outcomes
        .iter()
        .find(|(n, _)| n == &format!("T_{proc_name}"))
        .unwrap_or_else(|| panic!("no test T_{proc_name}: {outcomes:?}"));
    match outcome {
        Outcome::Fail(msg) => msg.split('|').map(str::to_string).collect(),
        other => panic!("{other:?}"),
    }
}

/// Y61StoredRecToTempCard.al:160 G617b_SetRecord_RunModal_Agree: SetRecord keeps the page temporary and empty; OnOpenPage reads the
/// record's fields, the page shows a blank row, and an action's Modify does not find it.
#[test]
fn set_record_of_a_stored_row_opens_the_temporary_card_empty() {
    let out = run("SetRecordAgree");
    assert_eq!(
        out,
        [
            "open N1/stored1/temp=Yes/count=0",
            "/",
            GONE,
            "act agree",
            "OK",
            STORED,
            "//Yes/count=0",
            "/Yes",
            "stored1"
        ]
    );
}

/// Y61StoredRecToTempCard.al:244 G617b_SetRecord_StoredRow_Delete.
#[test]
fn set_record_then_delete_does_not_find_the_row() {
    let out = run("SetRecordDelete");
    assert_eq!(
        out,
        [
            "open N1/stored1/temp=Yes/count=0",
            "/",
            GONE,
            STORED,
            "ok",
            "act delete",
            "OK",
            STORED
        ]
    );
}

/// Y61StoredRecToTempCard.al:216 G617b_PageRunModalIdRec_Rename: Page.RunModal(Id, Rec) makes Rec a database record.
#[test]
fn run_modal_with_a_stored_record_writes_to_the_database() {
    let out = run("RunModalIdRecRename");
    assert_eq!(
        out,
        [
            "open N1/stored1/temp=No/count=2;agr N1/stored1",
            "N1/stored1",
            "ok",
            "renamed",
            "N1/renamed/No,N2/stored2/No,",
            "ok",
            "agr N1/stored1;act rename;agr N1/renamed;agr N1/renamed",
            "LookupOK",
            "N1/renamed/No,N2/stored2/No,",
            "N1/renamed"
        ]
    );
}

/// Y61StoredRecToTempCard.al:276 G617b_PageRunIdRec_TrapTestPage_CountAndStep: Page.Run(Id, Rec) walks the database table.
#[test]
fn run_with_a_stored_record_walks_the_table() {
    let out = run("RunIdRecTrap");
    assert_eq!(
        out,
        [
            "open N1/stored1/temp=No/count=2;agr N1/stored1",
            "N1",
            "Yes",
            "agr N2/stored2",
            "N2",
            "Yes",
            "N1"
        ]
    );
}

/// Y61StoredRecToTempCard.al:340 G617b_RecordFilteredToNoRow: a record filtered to no row opens on no row, OK is not found.
#[test]
fn run_modal_with_a_record_filtered_to_no_row() {
    let out = run("FilteredToNoRow");
    assert_eq!(
        out,
        [
            "open //temp=No/count=0",
            "/",
            "The built-in action = OK is not found on the page./TestActionNotFound",
            "",
            "LookupCancel",
            STORED
        ]
    );
}
