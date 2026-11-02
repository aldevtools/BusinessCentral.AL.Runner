//! A Code field with `SQLDataType = Integer`, `BigInteger` or `Variant` in the primary key or a secondary key: the order,
//! the filters and the values an Insert takes. Pinned by the corpus writer's probe (not in the corpus pin yet: needs corpus
//! bump) tests/al-language/record/lt-y82-sqldatatype-code/ Y82PkTests.al (`Pk*_NumericOrder`, `Pk*_FilterLetters_*`,
//! `Pk*_Blank`, `Pk*_Mixed`), Y82InsertTests.al (`Pk*_Insert_*`, `SecF*_Insert_*`), Y82SecTests.al (`Sec*_NumericOrder`).

use al_builtins::value::Value;
use al_testkit::text;

const SRC: &str = r#"
table 69960 "NV Var" { fields { field(1; K; Code[10]) { SQLDataType = Varchar; } field(2; Note; Text[30]) { } } keys { key(PK; K) { Clustered = true; } } }
table 69961 "NV Int" { fields { field(1; K; Code[10]) { SQLDataType = Integer; } field(2; Note; Text[30]) { } } keys { key(PK; K) { Clustered = true; } } }
table 69962 "NV Big" { fields { field(1; K; Code[10]) { SQLDataType = BigInteger; } field(2; Note; Text[30]) { } } keys { key(PK; K) { Clustered = true; } } }
table 69963 "NV Vnt" { fields { field(1; K; Code[10]) { SQLDataType = Variant; } field(2; Note; Text[30]) { } } keys { key(PK; K) { Clustered = true; } } }
table 69964 "NV Sec"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; FVar; Code[10]) { SQLDataType = Varchar; }
        field(3; FInt; Code[10]) { SQLDataType = Integer; }
        field(4; FBig; Code[10]) { SQLDataType = BigInteger; }
        field(5; FVnt; Code[10]) { SQLDataType = Variant; }
    }
    keys { key(PK; Id) { Clustered = true; } key(K2; FVar) { } key(K3; FInt) { } key(K4; FBig) { } key(K5; FVnt) { } }
}
codeunit 69966 "NV Step"
{
    var
        TableNo: Integer;
        FieldNo: Integer;
        Id: Integer;
        Value: Text;
        RenameTo: Text;
        Renaming: Boolean;

    procedure Setup(NewTableNo: Integer; NewFieldNo: Integer; NewId: Integer; NewValue: Text)
    begin
        Id := NewId;
        TableNo := NewTableNo;
        FieldNo := NewFieldNo;
        Value := NewValue;
    end;

    procedure SetupRename(NewTableNo: Integer; NewValue: Text; NewNew: Text)
    begin
        TableNo := NewTableNo;
        Value := NewValue;
        RenameTo := NewNew;
        Renaming := true;
    end;

    trigger OnRun()
    var RR: RecordRef;
    begin
        RR.Open(TableNo);
        if Renaming then begin
            RR.Field(1).Value := Value;
            RR.Find('=');
            RR.Rename(RenameTo);
            exit;
        end;
        RR.Init();
        if FieldNo <> 1 then
            RR.Field(1).Value := Id;
        RR.Field(FieldNo).Value := Value;
        RR.Insert();
    end;
}
codeunit 69965 C
{
    local procedure Wipe(TableNo: Integer)
    var RR: RecordRef;
    begin
        RR.Open(TableNo);
        RR.DeleteAll();
    end;

    procedure Ins(TableNo: Integer; FieldNo: Integer; Id: Integer; Value: Text): Text
    var Step: Codeunit "NV Step";
    begin
        Commit();
        ClearLastError();
        Step.Setup(TableNo, FieldNo, Id, Value);
        if Step.Run() then
            exit('ok');
        exit('ERR[' + GetLastErrorCode() + ']: ' + GetLastErrorText());
    end;

    procedure Seed(TableNo: Integer; FieldNo: Integer; Values: Text): Text
    var Parts: List of [Text]; Part: Text; Id: Integer; Result: Text; Sep: Text;
    begin
        Wipe(TableNo);
        Parts := Values.Split(',');
        foreach Part in Parts do begin
            Id += 1;
            Result += Sep + Ins(TableNo, FieldNo, Id, Part);
            Sep := ' | ';
        end;
        exit(Result);
    end;

    local procedure Walk(var RR: RecordRef; ShowField: Integer): Text
    var Result: Text; Sep: Text;
    begin
        if RR.FindSet() then
            repeat
                Result += Sep + Format(RR.Field(ShowField).Value);
                Sep := ',';
            until RR.Next() = 0;
        if Result = '' then
            exit('(none)');
        exit(Result);
    end;

    // Rows in the order of the key whose first field is SortField (the primary key when SortField = 1).
    procedure Dump(TableNo: Integer; SortField: Integer; Ascending: Boolean): Text
    var RR: RecordRef; I: Integer;
    begin
        RR.Open(TableNo);
        for I := 1 to RR.KeyCount() do
            if RR.KeyIndex(I).FieldIndex(1).Number = SortField then begin
                RR.CurrentKeyIndex(I);
                break;
            end;
        RR.Ascending(Ascending);
        exit(Walk(RR, SortField));
    end;

    procedure DumpFilter(TableNo: Integer; FieldNo: Integer; Expression: Text): Text
    var RR: RecordRef;
    begin
        RR.Open(TableNo);
        RR.Field(FieldNo).SetFilter(Expression);
        exit(Walk(RR, FieldNo));
    end;

    procedure DumpRange(TableNo: Integer; FieldNo: Integer; FromValue: Text; ToValue: Text): Text
    var RR: RecordRef;
    begin
        RR.Open(TableNo);
        RR.Field(FieldNo).SetRange(FromValue, ToValue);
        exit(Walk(RR, FieldNo));
    end;

    procedure RenameKey(TableNo: Integer; Value: Text; NewValue: Text): Text
    var Step: Codeunit "NV Step";
    begin
        Commit();
        ClearLastError();
        Step.SetupRename(TableNo, Value, NewValue);
        if Step.Run() then
            exit('ok');
        exit('ERR[' + GetLastErrorCode() + ']: ' + GetLastErrorText());
    end;

    procedure TypedDesc(): Text
    var R: Record "NV Int"; Result: Text; Sep: Text;
    begin
        R.Init(); R.K := '10'; R.Insert();
        R.Init(); R.K := '9'; R.Insert();
        R.Init(); R.K := '100'; R.Insert();
        R.Reset();
        R.SetCurrentKey(K);
        R.SetAscending(K, false);
        if R.FindSet() then
            repeat
                Result += Sep + R.K;
                Sep := ',';
            until R.Next() = 0;
        exit(Result);
    end;

    procedure FindKey(TableNo: Integer; Value: Text): Text
    var RR: RecordRef;
    begin
        RR.Open(TableNo);
        RR.Field(1).Value := Value;
        if RR.Find('=') then
            exit('found ' + Format(RR.Field(1).Value));
        exit('not found');
    end;
}
"#;

fn t(s: &str) -> Value {
    Value::Text(s.into())
}

fn i(n: i32) -> Value {
    Value::Integer(n)
}

/// One session for several calls.
fn session() -> al_testkit::Built {
    al_testkit::Built::from_source(SRC)
}

fn on(b: &mut al_testkit::Built, routine: &str, args: Vec<Value>) -> String {
    let (oi, ri) = b.routine_in("C", routine);
    text(b.call(oi, ri, args).map(|(v, _)| v))
}

const INT: i32 = 69961;
const BIG: i32 = 69962;
const VNT: i32 = 69963;
const VAR: i32 = 69960;

// Y82PkTests.al:15-54 (PkVar_NumericOrder), :245-284 (PkInt_NumericOrder), :475-514 (PkBig), :705-744 (PkVnt): the rows 10, 9, 2,
// 1, 100, 20 by the primary key.
#[test]
fn a_primary_key_of_a_number_stored_code_field_sorts_and_filters_by_number() {
    for table in [INT, BIG, VNT] {
        let mut b = session();
        on(&mut b, "Seed", vec![i(table), i(1), t("10,9,2,1,100,20")]);
        let mut dump = |asc: bool| on(&mut b, "Dump", vec![i(table), i(1), Value::Boolean(asc)]);
        assert_eq!(dump(true), "1,2,9,10,20,100", "{table}");
        assert_eq!(dump(false), "100,20,10,9,2,1", "{table}");
        let mut filter = |e: &str| on(&mut b, "DumpFilter", vec![i(table), i(1), t(e)]);
        assert_eq!(filter("2..10"), "2,9,10", "{table}");
        assert_eq!(filter("<10"), "1,2,9", "{table}");
        assert_eq!(filter(">=9"), "9,10,20,100", "{table}");
        assert_eq!(filter("1|9|100"), "1,9,100", "{table}");
        assert_eq!(filter("1.."), "1,2,9,10,20,100", "{table}");
        assert_eq!(filter("..9"), "1,2,9", "{table}");
        assert_eq!(filter("<>10"), "1,2,9,20,100", "{table}");
        assert_eq!(filter("1*"), "1,10,100", "{table}");
        assert_eq!(
            on(&mut b, "DumpRange", vec![i(table), i(1), t("2"), t("10")]),
            "2,9,10",
            "{table}"
        );
        assert_eq!(on(&mut b, "FindKey", vec![i(table), t("9")]), "found 9", "{table}");
        assert_eq!(on(&mut b, "FindKey", vec![i(table), t("3")]), "not found", "{table}");
    }
}

// PkVar_NumericOrder (:15-54): Varchar sorts and filters as text.
#[test]
fn a_varchar_primary_key_keeps_the_text_order() {
    let mut b = session();
    on(&mut b, "Seed", vec![i(VAR), i(1), t("10,9,2,1,100,20")]);
    assert_eq!(
        on(&mut b, "Dump", vec![i(VAR), i(1), Value::Boolean(true)]),
        "1,10,100,2,20,9"
    );
    assert_eq!(on(&mut b, "DumpRange", vec![i(VAR), i(1), t("2"), t("10")]), "(none)");
}

// PkInt_FilterLetters_Range/_Text/_Mixed/_Count (Y82PkTests.al:354-404), PkInt_Lowercase (:335-352): letters in a filter find no row.
#[test]
fn a_filter_of_letters_on_a_number_stored_key_finds_no_row_and_no_error() {
    for table in [INT, BIG, VNT] {
        let mut b = session();
        on(&mut b, "Seed", vec![i(table), i(1), t("1,2,10")]);
        assert_eq!(
            on(&mut b, "DumpRange", vec![i(table), i(1), t("A"), t("B")]),
            "(none)",
            "{table}"
        );
        assert_eq!(
            on(&mut b, "DumpFilter", vec![i(table), i(1), t("A..B")]),
            "(none)",
            "{table}"
        );
        assert_eq!(
            on(&mut b, "DumpFilter", vec![i(table), i(1), t("1A")]),
            "(none)",
            "{table}"
        );
        assert_eq!(
            on(&mut b, "DumpFilter", vec![i(table), i(1), t("A*")]),
            "(none)",
            "{table}"
        );
    }
}

// Y82SecTests.al SecFInt_NumericOrder and the BigInteger and Variant ones: a secondary key sorts by number too.
#[test]
fn a_secondary_key_of_a_number_stored_code_field_sorts_by_number() {
    for field in [3, 4, 5] {
        let mut b = session();
        on(&mut b, "Seed", vec![i(69964), i(field), t("10,9,2,1,100,20")]);
        assert_eq!(
            on(&mut b, "Dump", vec![i(69964), i(field), Value::Boolean(true)]),
            "1,2,9,10,20,100",
            "{field}"
        );
    }
    let mut b = session();
    on(&mut b, "Seed", vec![i(69964), i(2), t("10,9,2,1,100,20")]);
    assert_eq!(
        on(&mut b, "Dump", vec![i(69964), i(2), Value::Boolean(true)]),
        "1,10,100,2,20,9"
    );
}

const BAD_CS: &str = "NDBCS:BadCSideCodeData";
const BAD_SQL: &str = "NDBCS:BadSqlIntCodeData";

fn invalid(table: &str, key: &str, value: &str, code_name: &str, number: &str) -> String {
    format!(
        "ERR[{code_name}]: The {table} table contains a value that is not valid in an integer Code field:\\\\{key}='{value}'\\Error code: {number}"
    )
}

// Y82InsertTests.al PkInt_Insert_* (:15-169) and PkBig_Insert_* (:171-325).
#[test]
fn an_insert_into_a_primary_key_stored_as_a_number_checks_the_value() {
    for (table, name) in [(INT, "NV Int"), (BIG, "NV Big")] {
        let mut b = session();
        let mut ins = |v: &str| on(&mut b, "Ins", vec![i(table), i(1), i(0), t(v)]);
        for v in ["0", "007"] {
            assert_eq!(ins(v), invalid(name, "K", v, BAD_CS, "22928070"), "{name} {v}");
        }
        for v in ["ABC", "-5", "1.5", "12A", "1E3"] {
            assert_eq!(ins(v), invalid(name, "K", v, BAD_SQL, "22928072"), "{name} {v}");
        }
        for v in ["+5", " 12", "", "7"] {
            assert_eq!(ins(v), "ok", "{name} {v}");
        }
        let over = ins("3000000000");
        if table == INT {
            assert_eq!(over, invalid(name, "K", "3000000000", BAD_SQL, "22928072"));
        } else {
            assert_eq!(over, "ok");
        }
        // '+5' and ' 12' are stored as 5 and 12.
        let rows = on(&mut b, "Dump", vec![i(table), i(1), Value::Boolean(true)]);
        assert_eq!(
            rows,
            if table == INT { ",5,7,12" } else { ",5,7,12,3000000000" },
            "{name}"
        );
    }
}

// Y82InsertTests.al SecFInt_Insert_*: the error names the primary key of the table.
#[test]
fn an_insert_into_a_secondary_key_stored_as_a_number_names_the_primary_key() {
    let mut b = session();
    assert_eq!(
        on(&mut b, "Ins", vec![i(69964), i(3), i(1), t("ABC")]),
        invalid("NV Sec", "Id", "1", BAD_SQL, "22928072")
    );
    assert_eq!(
        on(&mut b, "Ins", vec![i(69964), i(4), i(1), t("007")]),
        invalid("NV Sec", "Id", "1", BAD_CS, "22928070")
    );
}

// Y82InsertTests.al PkVnt_Insert_* (:327-481), Y82PkTests.al PkVnt_LeadingZeros (:767-779), PkVnt_Mixed (:780-794).
#[test]
fn a_variant_primary_key_takes_every_value_and_sorts_text_before_numbers() {
    let mut b = session();
    let seed = on(&mut b, "Seed", vec![i(VNT), i(1), t("A,B,1A,A1,10,9,a,")]);
    assert!(seed.contains("ERR[DB:RecordExists]"), "{seed}");
    assert_eq!(
        on(&mut b, "Dump", vec![i(VNT), i(1), Value::Boolean(true)]),
        ",1A,A,A1,B,9,10"
    );
    assert_eq!(
        on(&mut b, "Dump", vec![i(VNT), i(1), Value::Boolean(false)]),
        "10,9,B,A1,A,1A,"
    );
    for v in ["ABC", "-5", "1.5", "12A", "1E3", "0"] {
        assert_eq!(on(&mut b, "Ins", vec![i(VNT), i(1), i(0), t(v)]), "ok", "{v}");
    }
}

#[test]
fn a_leading_zero_variant_value_is_stored_as_the_number() {
    let mut b = session();
    on(&mut b, "Seed", vec![i(VNT), i(1), t("007,7,07")]);
    assert_eq!(on(&mut b, "Dump", vec![i(VNT), i(1), Value::Boolean(true)]), "7");
}

// Y82PkTests.al PkInt_Rename (:406-419), PkInt_Rename_ToAbc (:421-434), PkBig_Rename_ToAbc (:651-664).
#[test]
fn a_rename_of_a_number_stored_key_checks_the_new_value() {
    for (table, name) in [(INT, "NV Int"), (BIG, "NV Big")] {
        let mut b = session();
        on(&mut b, "Seed", vec![i(table), i(1), t("10,9")]);
        assert_eq!(
            on(&mut b, "RenameKey", vec![i(table), t("9"), t("ABC")]),
            invalid(name, "K", "ABC", BAD_SQL, "22928072")
        );
        assert_eq!(
            on(&mut b, "Dump", vec![i(table), i(1), Value::Boolean(true)]),
            "9,10",
            "{name}"
        );
        assert_eq!(on(&mut b, "RenameKey", vec![i(table), t("9"), t("11")]), "ok", "{name}");
        assert_eq!(
            on(&mut b, "Dump", vec![i(table), i(1), Value::Boolean(true)]),
            "10,11",
            "{name}"
        );
    }
}

// Y82PkTests.al Y82SecTests.al SecFVnt_Mixed: a text in a Variant key is found by equality, a range of texts holds the texts of the range.
#[test]
fn a_variant_key_finds_a_text_by_equality() {
    let mut b = session();
    on(&mut b, "Seed", vec![i(VNT), i(1), t("A,B,9")]);
    assert_eq!(on(&mut b, "DumpFilter", vec![i(VNT), i(1), t("A")]), "A");
    assert_eq!(on(&mut b, "DumpFilter", vec![i(VNT), i(1), t("9")]), "9");
    assert_eq!(on(&mut b, "DumpFilter", vec![i(VNT), i(1), t("A|9")]), "A,9");
    assert_eq!(on(&mut b, "DumpFilter", vec![i(VNT), i(1), t(">5")]), "9");
    assert_eq!(on(&mut b, "DumpFilter", vec![i(VNT), i(1), t("A..B")]), "A,B");
    // A comparison that reaches below the numbers would keep the texts: not pinned.
    let (oi, ri) = b.routine_in("C", "DumpFilter");
    let r = b.call(oi, ri, vec![i(VNT), i(1), t("<5")]);
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}

// Y82PkTests.al PkInt_Typed (:449-473): SetAscending(K, false) on a typed record runs the key backwards by number.
#[test]
fn a_typed_record_in_descending_order_sorts_by_number() {
    let mut b = session();
    assert_eq!(on(&mut b, "TypedDesc", vec![]), "100,10,9");
}
