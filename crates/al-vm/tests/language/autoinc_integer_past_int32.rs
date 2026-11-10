//! An Integer AutoIncrement key whose next value is past 2147483647 (al-runner issue 5521). On BC the Insert ends the
//! transaction and the test fails, which no AL code can catch, so no passing test can pin it
//! (record/TestAutoIncrementEdges.al:21-22). The runtime refuses the Insert: an AL error here would be caught by
//! `asserterror`, and a test that fails on BC would pass. A BigInteger key past Int32 follows the counter (same file).

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn out(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Integer(i)) => i.to_string(),
        Ok(Value::BigInteger(i)) => i.to_string(),
        Err(Signal::Error(e)) => format!("error:{}|{}", e.message, e.code),
        Err(Signal::Unsupported(why)) => format!("refused:{why}"),
        other => format!("{other:?}"),
    }
}

const SRC: &str = r#"
table 50100 "AI Int"
{
    fields
    {
        field(1; Id; Integer) { AutoIncrement = true; }
        field(2; Note; Text[10]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50101 "AI Big"
{
    fields
    {
        field(1; Id; BigInteger) { AutoIncrement = true; }
        field(2; Note; Text[10]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50100 C
{
    local procedure Seed()
    var
        R: Record "AI Int";
    begin
        R.Id := 2147483647;
        R.Insert(true);
    end;

    procedure Plain(): Integer
    var
        R: Record "AI Int";
    begin
        Seed();
        R.Id := 0;
        R.Insert(true);
        exit(R.Id);
    end;

    procedure UnderAssertError(): Integer
    var
        R: Record "AI Int";
    begin
        Seed();
        R.Id := 0;
        asserterror R.Insert(true);
        exit(1);
    end;

    procedure ResultUsed(): Integer
    var
        R: Record "AI Int";
    begin
        Seed();
        R.Id := 0;
        if R.Insert(true) then
            exit(1);
        exit(2);
    end;

    procedure LastThatFits(): Integer
    var
        R: Record "AI Int";
    begin
        R.Id := 2147483646;
        R.Insert(true);
        R.Id := 0;
        R.Insert(true);
        exit(R.Id);
    end;

    procedure Big(): BigInteger
    var
        R: Record "AI Big";
    begin
        R.Id := 2147483647;
        R.Insert(true);
        R.Id := 0;
        R.Insert(true);
        exit(R.Id);
    end;
}
"#;

fn run(routine: &str) -> String {
    out(al_testkit::run_in(SRC, "C", routine))
}

const REFUSED: &str = "refused:Insert into table 'AI Int': the next value of the Integer AutoIncrement field 'Id' is past 2147483647; BC ends the transaction and fails the test, which AL code cannot catch (record/TestAutoIncrementEdges.al:21-22)";

#[test]
fn an_integer_key_past_int32_is_refused_and_no_al_error() {
    assert_eq!(run("Plain"), REFUSED);
    assert_eq!(run("ResultUsed"), REFUSED);
}

#[test]
fn asserterror_does_not_absorb_it() {
    assert_eq!(run("UnderAssertError"), REFUSED);
}

#[test]
fn the_last_integer_value_and_a_biginteger_key_still_count_on() {
    assert_eq!(run("LastThatFits"), "2147483647");
    assert_eq!(run("Big"), "2147483648");
}
