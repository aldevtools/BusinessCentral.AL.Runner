//! A table extension uses a `protected var` global that another extension of the same table in the same app declares
//! (al-runner issue 6485). Measured on BC by corpus writer Y20 (tableextension/lt-y20-sibling-globals/Y20TableExtSiblingGlobals.al):
//! it compiles, and both extensions read and write one variable per record variable: a value one writes the other reads; a
//! second record variable has its own; record assignment and Copy do not copy it, Init keeps it, Clear resets it. A plain `var`
//! of the sibling does not compile (must-not-compile 2356, AL0161).

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn out(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(t)) => t.to_string(),
        Err(Signal::Unsupported(why)) => format!("refused:{why}"),
        other => format!("{other:?}"),
    }
}

const OBJECTS: &str = r#"
table 50510 "TSG Center"
{
    fields { field(1; Code; Code[20]) { } field(2; Name; Text[50]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}
table 50511 "TSG Name"
{
    fields { field(1; Code; Code[20]) { } field(2; Name; Text[50]) { } }
    keys { key(PK; Code) { Clustered = true; } }

    procedure InsertName(NewCode: Code[20]; NewName: Text[50])
    begin
        Code := NewCode;
        Name := NewName;
        Insert();
    end;
}
tableextension 50512 "TSG Center A" extends "TSG Center"
{
    trigger OnInsert()
    begin
        CostAccName.InsertName(Code, Name);
        Counter := Counter + 1;
    end;

    procedure CounterSeenByB(): Integer
    begin
        exit(ReadCounter());
    end;

    procedure SetCounterFromA(Value: Integer)
    begin
        Counter := Value;
    end;

    procedure NameCodeSeenByA(): Code[20]
    begin
        exit(CostAccName.Code);
    end;
}
tableextension 50513 "TSG Center B" extends "TSG Center"
{
    protected var
        CostAccName: Record "TSG Name";
        Counter: Integer;

    procedure ReadCounter(): Integer
    begin
        exit(Counter);
    end;

    procedure SetCounterFromB(Value: Integer)
    begin
        Counter := Value;
    end;
}
"#;

const CODE: &str = r#"
codeunit 50514 T
{
    procedure Trigger(): Text
    var
        C: Record "TSG Center";
        N: Record "TSG Name";
        R: Text;
    begin
        C.Code := 'CC1';
        C.Name := 'Center one';
        C.Insert(true);
        R := Format(N.Get('CC1')) + '/' + N.Name + '|' + Format(C.CounterSeenByB()) + '|' + Format(C.ReadCounter()) + '|' + C.NameCodeSeenByA();
        C.Code := 'CC2';
        C.Name := 'Center two';
        C.Insert(true);
        exit(R + '|' + Format(C.ReadCounter()) + '|' + C.NameCodeSeenByA());
    end;

    procedure PerVariable(): Text
    var
        C: Record "TSG Center";
        Other: Record "TSG Center";
        R: Text;
    begin
        C.SetCounterFromA(7);
        R := Format(C.ReadCounter());
        C.SetCounterFromB(9);
        R += '|' + Format(C.CounterSeenByB()) + '|' + Format(Other.ReadCounter());
        Other := C;
        R += '|' + Format(Other.ReadCounter());
        Other.SetCounterFromA(0);
        Other.Copy(C);
        R += '|' + Format(Other.ReadCounter());
        C.Init();
        R += '|' + Format(C.ReadCounter());
        Clear(C);
        exit(R + '|' + Format(C.ReadCounter()));
    end;

    procedure WithoutTrigger(): Text
    var
        C: Record "TSG Center";
        N: Record "TSG Name";
    begin
        C.Code := 'CC3';
        C.Insert(false);
        exit(Format(N.Count()) + '|' + Format(C.ReadCounter()));
    end;
}
"#;

fn run(routine: &str) -> String {
    out(al_testkit::run_in(&format!("{OBJECTS}{CODE}"), "T", routine))
}

#[test]
fn a_trigger_writes_the_sibling_extensions_globals_and_the_sibling_reads_them() {
    assert_eq!(run("Trigger"), "Yes/Center one|1|1|CC1|2|CC2");
    assert_eq!(run("WithoutTrigger"), "0|0");
}

#[test]
fn the_sibling_global_is_one_per_record_variable() {
    assert_eq!(run("PerVariable"), "7|9|0|0|0|9|0");
}

#[test]
fn a_plain_var_of_the_sibling_is_not_reachable() {
    let src =
        format!("{OBJECTS}{CODE}").replace("    protected var\n        CostAccName", "    var\n        CostAccName");
    let r = out(al_testkit::run_in(&src, "T", "Trigger"));
    assert!(r.starts_with("refused:"), "{r}");
}
