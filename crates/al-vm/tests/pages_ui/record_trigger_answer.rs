//! What the client reads from a page's Boolean OnInsertRecord or OnModifyRecord that does not end with `exit(value)`
//! (al-runner issue 7628; open corpus PR 1367, pages/triggers/RecordTriggerReturnValue.al, group G6). The pages copy the probe's
//! three cards; the neighbors flip one fact each.

use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 75350 "RT Row"
{
    fields { field(1; "Code"; Code[10]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "Code") { Clustered = true; } }
    trigger OnInsert() var Log: Codeunit "RT Log"; begin Log.Add('T-ins ' + Rec."Code" + ' ' + Rec.Name); end;
    trigger OnModify() var Log: Codeunit "RT Log"; begin Log.Add('T-mod ' + Rec."Code" + ' ' + Rec.Name); end;
}
page 75351 "RT Cond Card"
{
    PageType = Card; SourceTable = "RT Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { group(General) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } } }
    var Log: Codeunit "RT Log"; Blocked: Boolean;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Log.Add('I ' + Rec."Code" + ' ' + Rec.Name);
        if Blocked then
            exit(false);
    end;
    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('M ' + Rec."Code" + ' ' + Rec.Name);
        if Blocked then
            exit(false);
    end;
    trigger OnDeleteRecord(): Boolean
    begin
        Log.Add('D ' + Rec."Code");
        if Blocked then
            exit(false);
    end;
}
page 75352 "RT False Card"
{
    PageType = Card; SourceTable = "RT Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { group(General) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } } }
    var Log: Codeunit "RT Log"; Blocked: Boolean;
    trigger OnInsertRecord(BelowxRec: Boolean) Ok: Boolean
    begin
        Log.Add('I ' + Rec."Code" + ' ' + Rec.Name);
        Ok := false;
    end;
    trigger OnModifyRecord() Ok: Boolean
    begin
        Log.Add('M ' + Rec."Code" + ' ' + Rec.Name);
        Ok := false;
    end;
    trigger OnDeleteRecord() Ok: Boolean
    begin
        Log.Add('D ' + Rec."Code");
        Ok := false;
    end;
}
page 75353 "RT True Card"
{
    PageType = Card; SourceTable = "RT Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { group(General) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } } }
    var Log: Codeunit "RT Log"; Blocked: Boolean;
    trigger OnInsertRecord(BelowxRec: Boolean) Ok: Boolean
    begin
        Log.Add('I ' + Rec."Code" + ' ' + Rec.Name);
        Ok := true;
    end;
    trigger OnModifyRecord() Ok: Boolean
    begin
        Log.Add('M ' + Rec."Code" + ' ' + Rec.Name);
        Ok := true;
    end;
}
page 75356 "RT Cond List"
{
    PageType = List; SourceTable = "RT Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } } }
    var Log: Codeunit "RT Log"; Blocked: Boolean;
    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('M ' + Rec."Code" + ' ' + Rec.Name);
        if Blocked then
            exit(false);
    end;
}
page 75357 "RT Cond Named Card"
{
    PageType = Card; SourceTable = "RT Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { group(General) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } } }
    var Log: Codeunit "RT Log"; Blocked: Boolean;
    trigger OnModifyRecord() Ok: Boolean
    begin
        Log.Add('M ' + Rec."Code" + ' ' + Rec.Name);
        if not Blocked then
            Ok := false;
    end;
}
page 75358 "RT Cond Act Card"
{
    PageType = Card; SourceTable = "RT Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { group(General) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } } }
    actions { area(Processing) { action(Act) { trigger OnAction() begin end; } } }
    var Log: Codeunit "RT Log"; Blocked: Boolean;
    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('M ' + Rec."Code" + ' ' + Rec.Name);
        if Blocked then
            exit(false);
    end;
}
codeunit 75354 "RT Log"
{
    SingleInstance = true;
    var Lines: Text;
    procedure Reset() begin Lines := ''; end;
    procedure Add(Line: Text) begin Lines += Line + '; '; end;
    procedure Take() Result: Text begin Result := Lines; Lines := ''; end;
}
codeunit 75355 T
{
    var Log: Codeunit "RT Log";
    local procedure Setup()
    var Row: Record "RT Row";
    begin
        Row.DeleteAll();
        Row."Code" := 'R1'; Row.Name := 'start'; Row.Insert();
        Row."Code" := 'R2'; Row.Name := 'second'; Row.Insert();
        Commit();
        Log.Reset();
    end;
    local procedure Names() Result: Text
    var Row: Record "RT Row";
    begin
        if Row.FindSet() then
            repeat
                Result += Row."Code" + '=' + Row.Name + ' ';
            until Row.Next() = 0;
        Result += '(' + Format(Row.Count()) + ' rows)';
    end;
    local procedure Done(): Text begin exit(Log.Take() + '| ' + Names()); end;
    procedure CondInsertClose(): Text
    var P: TestPage "RT Cond Card";
    begin
        Setup(); P.OpenNew(); P."Code".SetValue('N1'); P.Name.SetValue('new'); P.Close(); exit(Done());
    end;
    procedure CondInsertNext(): Text
    var P: TestPage "RT Cond Card";
    begin
        Setup(); P.OpenNew(); P."Code".SetValue('N1'); P.Name.SetValue('new'); P.Next(); P.Close(); exit(Done());
    end;
    procedure CondModifyNext(): Text
    var P: TestPage "RT Cond Card";
    begin
        Setup(); P.OpenEdit(); P.GoToKey('R1'); P.Name.SetValue('edited'); P.Next(); P.Close(); exit(Done());
    end;
    procedure CondModifyClose(): Text
    var P: TestPage "RT Cond Card";
    begin
        Setup(); P.OpenEdit(); P.GoToKey('R1'); P.Name.SetValue('edited'); P.Close(); exit(Done());
    end;
    procedure FalseInsertClose(): Text
    var P: TestPage "RT False Card";
    begin
        Setup();
        P.OpenNew(); P."Code".SetValue('N1');
        asserterror P.Name.SetValue('new');
        exit(GetLastErrorText() + ' [' + GetLastErrorCode() + '] ' + Done());
    end;
    procedure FalseModifyNext(): Text
    var P: TestPage "RT False Card";
    begin
        Setup(); P.OpenEdit(); P.GoToKey('R1'); P.Name.SetValue('edited'); P.Next(); P.Close(); exit(Done());
    end;
    procedure FalseModifyClose(): Text
    var P: TestPage "RT False Card";
    begin
        Setup(); P.OpenEdit(); P.GoToKey('R1'); P.Name.SetValue('edited'); P.Close(); exit(Done());
    end;
    procedure TrueInsertClose(): Text
    var P: TestPage "RT True Card";
    begin
        Setup(); P.OpenNew(); P."Code".SetValue('N1'); P.Name.SetValue('new'); P.Close(); exit(Done());
    end;
    procedure TrueModifyNext(): Text
    var P: TestPage "RT True Card";
    begin
        Setup(); P.OpenEdit(); P.GoToKey('R1'); P.Name.SetValue('edited'); P.Next(); P.Close(); exit(Done());
    end;
    procedure FalseInsertKeyOnlyClose(): Text
    var P: TestPage "RT False Card";
    begin
        Setup(); P.OpenNew(); P."Code".SetValue('N1'); P.Close(); exit(Done());
    end;
    procedure FalseInsertThenAgain(): Text
    var P: TestPage "RT False Card";
    begin
        Setup();
        P.OpenNew(); P."Code".SetValue('N1');
        asserterror P.Name.SetValue('new');
        P.Name.SetValue('again');
        exit(Done());
    end;
    procedure CondListModifyNext(): Text
    var P: TestPage "RT Cond List";
    begin
        Setup(); P.OpenEdit(); P.GoToKey('R1'); P.Name.SetValue('edited'); P.Next(); P.Close(); exit(Done());
    end;
    procedure CondNamedModifyNext(): Text
    var P: TestPage "RT Cond Named Card";
    begin
        Setup(); P.OpenEdit(); P.GoToKey('R1'); P.Name.SetValue('edited'); P.Next(); P.Close(); exit(Done());
    end;
    procedure CondModifyAction(): Text
    var P: TestPage "RT Cond Act Card";
    begin
        Setup(); P.OpenEdit(); P.GoToKey('R1'); P.Name.SetValue('edited'); P.Act.Invoke(); P.Close(); exit(Done());
    end;
    procedure CondModifyOpenView(): Text
    var P: TestPage "RT Cond Card";
    begin
        Setup(); P.OpenView(); P.GoToKey('R1'); P.Name.SetValue('edited'); P.Next(); P.Close(); exit(Done());
    end;
}
"#;

fn flagged(proc: &str, want: &str, id: Assumption) {
    take_hits();
    assert_eq!(text(run_in(SRC, "T", proc)), want, "{proc}");
    assert!(take_hits().contains(&id), "{proc}: not flagged");
}

fn refused(proc: &str, why: &str) {
    let r = run_in(SRC, "T", proc);
    assert!(
        matches!(&r, Err(Signal::Unsupported(w)) if w.contains(why)),
        "{proc}: {r:?}"
    );
}

/// RecordTriggerReturnValue.al:252 Cond_Insert_OpenNewSetValueClose and :276 _Next (open corpus PR 1367): an exit(false) that
/// is not taken saves the new row.
#[test]
fn an_exit_not_taken_saves_the_insert() {
    let want = "I N1 ; T-ins N1 ; M N1 new; T-mod N1 new; | N1=new R1=start R2=second (3 rows)";
    flagged("CondInsertClose", want, Assumption::PageRecordTriggerExitNotTakenSaves);
    flagged("CondInsertNext", want, Assumption::PageRecordTriggerExitNotTakenSaves);
}

/// RecordTriggerReturnValue.al:301 Cond_Modify_SetValueNext and :326 _Close: the modify is saved.
#[test]
fn an_exit_not_taken_saves_the_modify() {
    let want = "M R1 edited; T-mod R1 edited; | R1=edited R2=second (2 rows)";
    flagged("CondModifyNext", want, Assumption::PageRecordTriggerExitNotTakenSaves);
    flagged("CondModifyClose", want, Assumption::PageRecordTriggerExitNotTakenSaves);
}

/// RecordTriggerReturnValue.al:350 False_Insert_OpenNewSetValueClose and :373 _Next: `Ok := false` vetoes the insert with the
/// filtered-view error of the SetValue that inserts; nothing is stored.
#[test]
fn a_named_false_vetoes_the_insert_with_the_view_error() {
    flagged(
        "FalseInsertClose",
        "Validation error for Field: ,  Message = 'The view is filtered, and the entry is outside the filter. Some actions may not work.' [TestValidation] I N1 ; | R1=start R2=second (2 rows)",
        Assumption::PageRecordTriggerNamedFalseVetoes,
    );
}

/// RecordTriggerReturnValue.al:397 False_Modify_SetValueNext and :422 _Close: the modify is dropped without an error.
#[test]
fn a_named_false_drops_the_modify() {
    let want = "M R1 edited; | R1=start R2=second (2 rows)";
    flagged("FalseModifyNext", want, Assumption::PageRecordTriggerNamedFalseVetoes);
    flagged("FalseModifyClose", want, Assumption::PageRecordTriggerNamedFalseVetoes);
}

/// RecordTriggerReturnValue.al:446, :495: `Ok := true` saves, as main did.
#[test]
fn a_named_true_saves_as_on_main() {
    assert_eq!(
        text(run_in(SRC, "T", "TrueInsertClose")),
        "I N1 ; T-ins N1 ; M N1 new; T-mod N1 new; | N1=new R1=start R2=second (3 rows)"
    );
    assert_eq!(
        text(run_in(SRC, "T", "TrueModifyNext")),
        "M R1 edited; T-mod R1 edited; | R1=edited R2=second (2 rows)"
    );
}

/// The neighbors the probe did not measure, where main's answer follows the rule the probe showed wrong: refused.
#[test]
fn unmeasured_neighbors_are_refused() {
    let exit = "ended without running its exit(value)";
    let named = "ended with its named return value set to false";
    for (proc, why) in [
        // the insert at Close, with only the key set
        ("FalseInsertKeyOnlyClose", named),
        // a later call after the veto of the insert
        ("FalseInsertThenAgain", "after its OnInsertRecord vetoed the insert"),
        // a List page
        ("CondListModifyNext", exit),
        // the named return value assigned false in a branch
        ("CondNamedModifyNext", named),
        // the save of an action
        ("CondModifyAction", exit),
        // OpenView
        ("CondModifyOpenView", exit),
    ] {
        refused(proc, why);
    }
}
