//! `DeleteAll(true)` with OnDelete over a record sorted descending runs the trigger in the scan order (assumption
//! delete-all-row-code-scan-order, corpus PR 1659, al-runner issue 9401), and refuses while it runs when the row code
//! writes a row the delete has still to visit or its filters see, or raises an error (records/delete_all_order.rs).

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69990 "DAD Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } }
    trigger OnDelete()
    var Ctl: Codeunit Ctl; R: Record "DAD Row";
    begin
        Ctl.Add(Format(Id));
        if Id <> 3 then exit;
        case Ctl.Mode() of
            'delete': begin R.Get(1); R.Delete(false); end;
            'modify': begin R.Get(1); R.Name := 'x'; R.Modify(false); end;
            'insert': begin R.Id := 0; R.Insert(false); end;
            'error': Error('stop');
        end;
    end;
}
codeunit 69991 Ctl
{
    SingleInstance = true;
    var Seen: Text; M: Text;
    procedure Add(T: Text) begin Seen += T; end;
    procedure Mode(): Text begin exit(M); end;
    procedure SetMode(T: Text) begin M := T; Seen := ''; end;
    procedure Seen(): Text begin exit(Seen); end;
}
codeunit 69992 C
{
    procedure Run(Mode: Text): Text
    var R: Record "DAD Row"; Ctl: Codeunit Ctl; i: Integer;
    begin
        for i := 1 to 3 do begin R.Id := i; R.Insert(false); end;
        Ctl.SetMode(Mode);
        R.Ascending(false);
        R.DeleteAll(true);
        exit(Ctl.Seen() + '/' + Format(R.Count()));
    end;
    procedure Plain(): Text begin exit(Run('')); end;
    procedure Deletes(): Text begin exit(Run('delete')); end;
    procedure Modifies(): Text begin exit(Run('modify')); end;
    procedure Inserts(): Text begin exit(Run('insert')); end;
    procedure Errors(): Text begin exit(Run('error')); end;
}
"#;

#[test]
fn descending_delete_all_runs_on_delete_in_scan_order() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Plain")), "321/0");
}

#[test]
fn descending_delete_all_refuses_row_code_that_writes_rows_or_errors() {
    for (routine, why) in [
        ("Deletes", "still to visit"),
        ("Modifies", "still to visit"),
        ("Inserts", "its filters see"),
        ("Errors", "an error raised by the row code"),
    ] {
        match al_testkit::run_in(SRC, "C", routine) {
            Err(Signal::Unsupported(r)) => assert!(r.contains(why), "{routine}: {r}"),
            other => panic!("{routine}: expected a refusal, got {other:?}"),
        }
    }
}
