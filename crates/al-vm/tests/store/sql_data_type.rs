//! A Code field stored as an integer (`SQLDataType = Integer`) and a BigInteger field stored as the row version
//! (`SQLTimestamp = true`): corpus record/lt-x-table-props/XTablePropsTests.al:293-415.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69992 "SDT Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; AsInteger; Code[20]) { SQLDataType = Integer; }
        field(3; AsVarchar; Code[20]) { SQLDataType = Varchar; }
        field(4; AsBigInteger; Code[20]) { SQLDataType = BigInteger; }
        field(5; Stamp; BigInteger) { SQLTimestamp = true; }
        field(6; Note; Text[20]) { }
    }
    keys { key(PK; Id) { } key(K1; AsInteger) { } key(K2; AsVarchar) { } key(K3; AsBigInteger) { } }
}
codeunit 69993 C
{
    local procedure Seed()
    var Row: Record "SDT Row";
    begin
        Row.Init(); Row.Id := 1; Row.AsInteger := '10'; Row.AsVarchar := '10'; Row.AsBigInteger := '10'; Row.Insert();
        Row.Init(); Row.Id := 2; Row.AsInteger := '9'; Row.AsVarchar := '9'; Row.AsBigInteger := '9'; Row.Insert();
        Row.Init(); Row.Id := 3; Row.AsVarchar := 'ABC'; Row.Insert();
    end;

    procedure Order(): Text
    var Row: Record "SDT Row"; s: Text;
    begin
        Seed();
        Row.SetCurrentKey(AsInteger);
        Row.FindFirst(); s := Format(Row.Id);
        Row.FindLast(); s += Format(Row.Id);
        if Row.FindSet() then repeat s += ',' + Format(Row.Id); until Row.Next() = 0;
        Row.SetCurrentKey(AsVarchar);
        Row.FindFirst(); s += '|' + Format(Row.Id);
        Row.FindLast(); s += Format(Row.Id);
        exit(s);
    end;

    procedure ModifyTo(Value: Code[20])
    var Row: Record "SDT Row";
    begin
        Seed();
        Row.Get(1);
        Row.AsInteger := Value;
        Row.Modify();
    end;

    procedure InsertText()
    var Row: Record "SDT Row";
    begin
        Row.Init(); Row.Id := 7; Row.AsInteger := 'ABC'; Row.Insert();
    end;

    procedure RangeFilter(): Integer
    var Row: Record "SDT Row";
    begin
        Seed();
        Row.SetRange(AsInteger, '9', '10');
        exit(Row.Count());
    end;

    procedure EqualFilter(): Integer
    var Row: Record "SDT Row";
    begin
        Seed();
        Row.SetRange(AsInteger, '9');
        exit(Row.Count());
    end;

    procedure BigIntegerOrder(): Integer
    var Row: Record "SDT Row";
    begin
        Seed();
        Row.SetCurrentKey(AsBigInteger);
        Row.FindFirst();
        exit(Row.Id);
    end;

    procedure Stamp(): Text
    var Row: Record "SDT Row"; First: BigInteger; Second: BigInteger;
    begin
        Row.Init(); Row.Id := 1; Row.Insert();
        Row.Get(1);
        First := Row.Stamp;
        Row.Note := 'x';
        Row.Modify();
        Row.Get(1);
        Second := Row.Stamp;
        Row.Stamp := 5;
        Row.Modify();
        Row.Get(1);
        exit(Format(First > 0) + Format(Second > First) + Format(Row.Stamp <> 5));
    end;

    procedure TempStamp(): Text
    var Temp: Record "SDT Row" temporary; s: Text;
    begin
        Temp.Init(); Temp.Id := 1; Temp.Insert();
        s := Format(Temp.Stamp) + Format(Temp.SystemRowVersion);
        Temp.Note := 'b'; Temp.Modify();
        s += Format(Temp.Stamp) + Format(Temp.SystemRowVersion);
        Temp.Rename(3); Temp.Get(3);
        s += Format(Temp.Stamp) + Format(Temp.SystemRowVersion);
        Temp.Stamp := 7; Temp.Modify(); Temp.Get(3);
        s += '|' + Format(Temp.Stamp);
        exit(s);
    end;

    procedure TempCopyOfDbRow(): Text
    var Db: Record "SDT Row"; Temp: Record "SDT Row" temporary; Temp2: Record "SDT Row" temporary; V: BigInteger;
    begin
        Db.Init(); Db.Id := 1; Db.Insert(); Db.Get(1);
        V := Db.SystemRowVersion;
        Temp := Db; Temp.Insert(); Temp.Note := 'c'; Temp.Modify(); Temp.Get(1);
        Temp2.Copy(Temp, true); Temp2.Get(1); Temp2.Init(); Temp2.Id := 2; Temp2.Insert();
        exit(Format((Temp.Stamp = V) and (Temp.SystemRowVersion = V)) + Format((Temp2.Stamp = V) and (Temp2.SystemRowVersion = V)));
    end;
}
"#;

fn run(routine: &str) -> Result<al_builtins::value::Value, Signal> {
    al_testkit::run_in(SRC, "C", routine)
}

// XTablePropsTests.al:322-325: by the integer key the empty value comes first and 9 before 10; by the Varchar key text order.
#[test]
fn an_integer_code_key_sorts_by_number() {
    assert_eq!(text(run("Order")), "31,3,2,1|13");
}

// XTablePropsTests.al:352-353 ('0'), :368-369 ('007'), :384-385 ('ABC').
#[test]
fn modify_to_a_value_that_is_no_integer_fails() {
    let head = "The SDT Row table contains a value that is not valid in an integer Code field:\\\\Id='1'\\Error code: ";
    for (value, number, code) in [
        ("0", "22928070", "NDBCS:BadCSideCodeData"),
        ("007", "22928070", "NDBCS:BadCSideCodeData"),
        ("ABC", "22928072", "NDBCS:BadSqlIntCodeData"),
    ] {
        match al_testkit::run_with(
            SRC,
            "C",
            "ModifyTo",
            vec![al_builtins::value::Value::Code(value.into())],
        ) {
            Err(Signal::Error(e)) => {
                assert_eq!(e.message, format!("{head}{number}"), "{value}");
                assert_eq!(e.code.as_str(), code, "{value}");
            }
            other => panic!("{value}: {other:?}"),
        }
    }
}

// Not pinned: an Insert of such a value, a range filter on the field, and the order of a BigInteger Code key.
#[test]
fn unpinned_uses_of_an_integer_code_field_are_refused() {
    for routine in ["InsertText", "RangeFilter", "BigIntegerOrder"] {
        let r = run(routine);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{routine}: {r:?}");
    }
    assert_eq!(al_testkit::int(run("EqualFilter")), 1);
}

// XTablePropsTests.al:412-415.
#[test]
fn an_sql_timestamp_field_reads_the_row_version() {
    assert_eq!(text(run("Stamp")), "YesYesYes");
}

// record/lt-qr-temp-sqltimestamp/QrTempTimestamp.al:76-85 (stays 0 through Insert, Modify, Rename and Get), :111-115 (an assigned
// value is kept).
#[test]
fn a_temporary_record_never_stamps_its_sql_timestamp_field() {
    assert_eq!(text(run("TempStamp")), "000000|7");
}

// QrTempTimestamp.al:160-170 (a copy of a database row keeps its values), :221-226 (Init of the shared copy keeps them).
#[test]
fn a_temporary_copy_of_a_database_row_keeps_its_stamps() {
    assert_eq!(text(run("TempCopyOfDbRow")), "YesYes");
}
