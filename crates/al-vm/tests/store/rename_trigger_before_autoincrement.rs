//! Rename of an AutoIncrement key: the OnRename trigger runs before the AutoIncrement check (al-runner issue 6863).
//! Values: record/crud/R6863RenameTrigger.al (BC 28.5; 27.5 and 29.0 as recorded in the PR).

use al_testkit::text;

const SRC: &str = r#"
table 50340 W
{
    fields { field(1; Id; Integer) { AutoIncrement = true; } field(2; Payload; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
    trigger OnRename()
    var
        Cfg: Codeunit Cfg;
    begin
        Cfg.Hit();
        if Cfg.Raise() then
            Error('trigger error');
    end;
}

codeunit 50341 Cfg
{
    SingleInstance = true;
    var
        Hits: Integer;
        RaiseIt: Boolean;

    procedure Reset(NewRaise: Boolean) begin Hits := 0; RaiseIt := NewRaise; end;
    procedure Hit() begin Hits += 1; end;
    procedure Raise(): Boolean begin exit(RaiseIt); end;
    procedure Count(): Integer begin exit(Hits); end;
}

codeunit 50342 T
{
    procedure Outcome(Raise: Boolean; Kind: Text): Text
    var
        W: Record W;
        Other: Record W;
        Cfg: Codeunit Cfg;
        Old: Integer;
    begin
        Cfg.Reset(Raise);
        W.Payload := 'a';
        W.Insert(true);
        Old := W.Id;
        case Kind of
            'new':
                asserterror W.Rename(Old + 1000);
            'same':
                asserterror begin W.Rename(Old); Error('no error'); end;
            'existing':
                begin
                    Other.Payload := 'b';
                    Other.Insert(true);
                    asserterror W.Rename(Other.Id);
                end;
            'missing':
                begin
                    Other.Id := Old + 500;
                    asserterror Other.Rename(Old + 600);
                end;
        end;
        exit(StrSubstNo('hits=%1 %2|%3', Cfg.Count(), GetLastErrorText(), GetLastErrorCode()));
    end;

    procedure OnTemporary(Raise: Boolean): Text
    var
        W: Record W temporary;
        Cfg: Codeunit Cfg;
    begin
        Cfg.Reset(Raise);
        W.Id := 7;
        W.Payload := 'a';
        W.Insert(false);
        asserterror begin W.Rename(8); Error('no error'); end;
        exit(StrSubstNo('hits=%1 %2|%3|%4', Cfg.Count(), GetLastErrorText(), GetLastErrorCode(), W.Id));
    end;

    procedure ViaRecordRef(Raise: Boolean): Text
    var
        W: Record W;
        Ref: RecordRef;
        Cfg: Codeunit Cfg;
    begin
        Cfg.Reset(Raise);
        W.Payload := 'a';
        W.Insert(true);
        Ref.GetTable(W);
        asserterror Ref.Rename(W.Id + 1000);
        exit(StrSubstNo('hits=%1 %2|%3', Cfg.Count(), GetLastErrorText(), GetLastErrorCode()));
    end;
}
"#;

fn outcome(raise: bool, kind: &str) -> String {
    text(
        al_testkit::run_with(
            SRC,
            "T",
            "Outcome",
            vec![
                al_builtins::value::Value::Boolean(raise),
                al_builtins::value::Value::Text(kind.into()),
            ],
        )
        .map(|(v, _)| v),
    )
}

fn autoinc_text(kind: &str) -> bool {
    outcome(false, kind).contains("You cannot change the value of an AutoIncrement field.")
}

#[test]
fn the_trigger_error_wins_over_the_autoincrement_error() {
    assert_eq!(outcome(true, "new"), "hits=1 trigger error|Dialog");
    assert_eq!(outcome(true, "existing"), "hits=1 trigger error|Dialog");
}

#[test]
fn the_trigger_runs_before_the_autoincrement_error() {
    let o = outcome(false, "new");
    assert!(
        o.starts_with("hits=1 You cannot change the value of an AutoIncrement field."),
        "{o}"
    );
    assert!(o.ends_with("|DB:ModifyOnIdentityField"), "{o}");
    assert!(autoinc_text("existing"));
}

#[test]
fn a_rename_that_keeps_the_autoincrement_value_runs_the_trigger_only() {
    assert_eq!(outcome(true, "same"), "hits=1 trigger error|Dialog");
    assert_eq!(outcome(false, "same"), "hits=1 no error|Dialog");
}

#[test]
fn a_missing_row_runs_the_trigger_first() {
    assert_eq!(outcome(true, "missing"), "hits=1 trigger error|Dialog");
    assert!(outcome(false, "missing").starts_with("hits=1 The W does not exist."));
}

#[test]
fn through_a_recordref_the_same_order_holds() {
    assert_eq!(
        text(run_in_with_bool("ViaRecordRef", true)),
        "hits=1 trigger error|Dialog"
    );
    let o = text(run_in_with_bool("ViaRecordRef", false));
    assert!(
        o.starts_with("hits=1 You cannot change the value of an AutoIncrement field."),
        "{o}"
    );
}

fn run_in_with_bool(routine: &str, raise: bool) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    al_testkit::run_with(SRC, "T", routine, vec![al_builtins::value::Value::Boolean(raise)]).map(|(v, _)| v)
}

// record/crud/R6863RenameTrigger.al:202-219 (Rename_TemporaryRecord): a temporary record runs the trigger and
// gets no AutoIncrement error; the new key is kept.
#[test]
fn a_temporary_record_gets_no_autoincrement_error() {
    assert_eq!(text(run_in_with_bool("OnTemporary", false)), "hits=1 no error|Dialog|8");
    assert_eq!(
        text(run_in_with_bool("OnTemporary", true)),
        "hits=1 trigger error|Dialog|8"
    );
}
