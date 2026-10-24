//! An action that changes nothing on a list with ModifyAllowed = false, over a table with a Blob field, also after a
//! CalcFields of a Blob that holds a value: the action runs, no modify trigger runs and the row is unchanged (corpus
//! pages/lt-outside-worksheet/RmReadOnlyBlob.al:316-:470, al-runner issue 5781). An action that writes the Blob through a
//! stream is not pinned.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69995 "RB Row"
{
    fields { field(1; Code; Code[10]) { } field(2; Stack; Blob) { } }
    keys { key(PK; Code) { } }
    trigger OnModify() var L: Codeunit "RB Log"; begin L.Add('T'); end;
}
codeunit 69996 "RB Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(S: Text) begin E += S + ';'; end;
    procedure Take(): Text var R: Text; begin R := E; E := ''; exit(R); end;
}
page 69997 "RB List"
{
    PageType = List; SourceTable = "RB Row"; Editable = false; InsertAllowed = false; DeleteAllowed = false; ModifyAllowed = false;
    ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } } }
    actions { area(Processing) {
        action(Nothing) { trigger OnAction() var L: Codeunit "RB Log"; begin L.Add('A'); end; }
        action(ReadStack) { trigger OnAction() var I: InStream; S: Text; L: Codeunit "RB Log"; begin Rec.CalcFields(Stack); Rec.Stack.CreateInStream(I); I.ReadText(S); L.Add(S); end; }
        action(WriteStack) { trigger OnAction() var O: OutStream; begin Rec.Stack.CreateOutStream(O); O.WriteText('new'); end; }
    } }
    trigger OnModifyRecord(): Boolean var L: Codeunit "RB Log"; begin L.Add('M'); exit(true); end;
}
codeunit 69998 C
{
    local procedure Seed(WithStack: Boolean)
    var R: Record "RB Row"; O: OutStream;
    begin
        R.Code := 'A';
        if WithStack then begin R.Stack.CreateOutStream(O); O.WriteText('line 1'); end;
        R.Insert();
    end;
    procedure Nothing(): Text
    var P: TestPage "RB List"; L: Codeunit "RB Log";
    begin
        Seed(false);
        P.OpenView();
        P.Nothing.Invoke();
        P.Close();
        exit(L.Take());
    end;
    procedure ReadStack(): Text
    var P: TestPage "RB List"; L: Codeunit "RB Log";
    begin
        Seed(true);
        P.OpenEdit();
        P.ReadStack.Invoke();
        P.Nothing.Invoke();
        P.Close();
        exit(L.Take());
    end;
    procedure WriteStack(): Text
    var P: TestPage "RB List";
    begin
        Seed(false);
        P.OpenView();
        P.WriteStack.Invoke();
        P.Close();
    end;
}
"#;

#[test]
fn an_action_that_changes_nothing_saves_nothing() {
    assert_eq!(text(run_in(SRC, "C", "Nothing")), "A;");
}

#[test]
fn an_action_that_reads_a_stored_blob_saves_nothing() {
    assert_eq!(text(run_in(SRC, "C", "ReadStack")), "line 1;A;");
}

#[test]
fn an_action_that_writes_the_blob_is_refused() {
    assert!(matches!(
        run_in(SRC, "C", "WriteStack"),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}
