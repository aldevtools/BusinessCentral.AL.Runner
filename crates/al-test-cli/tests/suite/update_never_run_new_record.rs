//! CurrPage.Update(true) or Update() called by a procedure of a Page variable that never ran, after the procedure set the fields
//! of a new record (the Base Application "Table Filter" page fills its lines this way; al-runner issue 6951). On BC it inserts
//! the record (OnInsertRecord, the table's OnInsert, AutoSplitKey numbers 10000, 20000): corpus branch
//! al-runtime-rs-staging-r6951 format-evaluate/tablefilter-neutral/TableFilterEnumForms.al
//! UpdateNeverRunPage_NewRecord_UpdateTrue, _UpdateNoArgument. Update(false) and no Update insert nothing (_UpdateFalse,
//! _NoUpdate); with a blank record Update(true) does nothing (pages/currpage/UnopenedCurrPage.al Direct_Part_UpdateTrue).
//! Implemented on the assumption `never-run-update-inserts-new-record` until corpus PR 1133 is in the pin.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

fn outcome(mode: &str, set_fields: bool) -> Outcome {
    outcome_with(mode, set_fields, "Error('rows=%1', P.Count());")
}

fn outcome_with(mode: &str, set_fields: bool, body: &str) -> Outcome {
    let set = if set_fields {
        "Rec.\"Table Number\" := 95452; Rec.\"Line No.\" := 0; Rec.\"Field Number\" := No; Rec.\"Field Filter\" := Filter;"
    } else {
        ""
    };
    let src = format!(
        r#"
table 62970 "UN Line" {{ fields {{ field(1; "Table Number"; Integer) {{ }} field(2; "Line No."; Integer) {{ }} field(3; "Field Number"; Integer) {{ }} field(4; "Field Filter"; Text[250]) {{ }} }}
    keys {{ key(PK; "Table Number", "Line No.") {{ Clustered = true; }} }}
    trigger OnInsert() var Log: Codeunit "UN Log"; begin Log.Add('T.OnInsert:' + Format("Line No.") + '/' + Format("Field Number")); end;
    trigger OnModify() var Log: Codeunit "UN Log"; begin Log.Add('T.OnModify'); end; }}
codeunit 62973 "UN Log" {{ SingleInstance = true; var L: Text;
    procedure Add(E: Text) begin if L <> '' then L += ' | '; L += E; end;
    procedure Take(): Text var R: Text; begin R := '[' + L + ']'; L := ''; exit(R); end; }}
page 62971 "UN Lines" {{
    PageType = List; SourceTable = "UN Line"; SourceTableTemporary = true; AutoSplitKey = true; DelayedInsert = true;
    layout {{ area(Content) {{ repeater(R) {{ field("Field Number"; Rec."Field Number") {{ }} }} }} }}
    trigger OnNewRecord(BelowxRec: Boolean) var Log: Codeunit "UN Log"; begin Log.Add('P.OnNewRecord'); end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean var Log: Codeunit "UN Log";
    begin Log.Add('P.OnInsertRecord:' + Format(Rec."Line No.") + '/' + Format(Rec."Field Number") + ' below=' + Format(BelowxRec)); end;
    trigger OnModifyRecord(): Boolean var Log: Codeunit "UN Log"; begin Log.Add('P.OnModifyRecord'); end;
    procedure AddLine(Mode: Text)
    begin
        AddLineOf(Mode, 1, 'A..B');
    end;
    procedure AddLineOf(Mode: Text; No: Integer; Filter: Text)
    begin
        Rec.SetRange("Field Number");
        Rec.Init();
        {set}
        case Mode of
            'true': CurrPage.Update(true);
            'false': CurrPage.Update(false);
            'none': CurrPage.Update();
        end;
    end;
    procedure Count(): Integer begin Rec.Reset(); exit(Rec.Count()); end;
    procedure RowsText(): Text var R: Text;
    begin
        Rec.Reset();
        if Rec.FindSet() then repeat R += Format(Rec."Line No.") + ':' + Format(Rec."Field Number") + '=' + Rec."Field Filter" + ';'; until Rec.Next() = 0;
        exit('[' + R + ']');
    end;
}}
codeunit 62972 T
{{
    Subtype = Test;
    [Test] procedure Run()
    var P: Page "UN Lines"; Log: Codeunit "UN Log"; A: Text; B: Text;
    begin
        Log.Take();
        P.AddLine('{mode}');
        {body}
    end;
}}"#
    );
    outcomes_in(&src, "T")[0].1.clone()
}

const TWO_LINES: &str =
    "A := Log.Take(); P.AddLineOf('MODE', 4, '<>XYZ'); B := Log.Take(); Error('%1 %2 %3', A, B, P.RowsText());";

/// TableFilterEnumForms.al UpdateNeverRunPage_NewRecord_UpdateTrue and _UpdateNoArgument: `[P.OnInsertRecord:10000/1 below=No |
/// T.OnInsert:10000/1]`, `[P.OnInsertRecord:20000/4 below=Yes | T.OnInsert:20000/4]`, rows `[10000:1=A..B;20000:4=<>XYZ;]`.
#[test]
fn update_true_or_without_argument_inserts_the_new_record() {
    for mode in ["true", "none"] {
        assert_eq!(
            outcome_with(mode, true, &TWO_LINES.replace("MODE", mode)),
            Outcome::Fail(
                "[P.OnInsertRecord:10000/1 below=No | T.OnInsert:10000/1] [P.OnInsertRecord:20000/4 below=Yes | T.OnInsert:20000/4] [10000:1=A..B;20000:4=<>XYZ;]"
                    .into()
            ),
            "mode {mode}"
        );
    }
}

/// UpdateNeverRunPage_NewRecord_UpdateFalse: no trigger runs and no row is inserted.
#[test]
fn update_false_logs_nothing() {
    assert_eq!(
        outcome_with("false", true, &TWO_LINES.replace("MODE", "false")),
        Outcome::Fail("[] [] []".into())
    );
}

#[test]
fn update_false_and_a_blank_record_insert_nothing() {
    assert_eq!(outcome("false", true), Outcome::Fail("rows=0".into()));
    assert_eq!(outcome("skip", true), Outcome::Fail("rows=0".into()));
    assert_eq!(outcome("true", false), Outcome::Fail("rows=0".into()));
}
