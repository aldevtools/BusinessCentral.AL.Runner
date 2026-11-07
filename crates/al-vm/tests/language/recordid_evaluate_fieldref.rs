//! `Evaluate(FieldRef, Text)` on a RecordId field, as a statement and with the result used, and the text forms of the table name
//! (recordref-fieldref/N582EvalFieldRefUnused.al:362-662, waldo run 2 of al-runner issue 6924, corpus master 176baa02).

const SRC: &str = r#"
table 97240 "LT WA RecId Row" { fields { field(1; Id; Integer) { } field(2; Rid; RecordId) { } field(3; Rid2; RecordId) { } } keys { key(PK; Id) { } } }
table 97241 "LT WA RecId Two" { fields { field(1; Code; Code[10]) { } field(2; Num; Integer) { } } keys { key(PK; Code, Num) { } } }
codeunit 97242 C
{
    [TryFunction]
    local procedure TryEval(FldRef: FieldRef; Txt: Text)
    begin
        Evaluate(FldRef, Txt);
    end;

    local procedure One(FieldNo: Integer; Txt: Text; Used: Boolean): Text
    var
        Row: Record "LT WA RecId Row";
        RecRef: RecordRef;
        FldRef: FieldRef;
        Res: Text;
    begin
        Row.Id := 1;
        Row.Rid := Row.RecordId();
        RecRef.GetTable(Row);
        FldRef := RecRef.Field(FieldNo);
        ClearLastError();
        if Used then
            Res := Format(Evaluate(FldRef, Txt))
        else
            if TryEval(FldRef, Txt) then
                Res := 'ok'
            else
                Res := 'err';
        exit(Res + ' ' + GetLastErrorText() + ' [' + Format(FldRef.Value) + ']');
    end;

    procedure Statement(): Text
    begin
        exit(One(2, 'LT WA RecId Row: 2', false) + '|' + One(2, 'lt wa recid row:3', false) + '|' + One(2, 'zzz', false) + '|' +
            One(2, '', false) + '|' + One(2, ' ', false) + '|' + One(2, 'LT WA RecId Row: abc', false) + '|' +
            One(2, 'LT WA RecId Row: 1,2', false) + '|' + One(2, 'LT WA RecId Two: A', false) + '|' + One(2, 'LT WA RecId Row', false));
    end;

    procedure Names(): Text
    begin
        exit(One(2, 'LT WA RecId R: 2', false) + '|' + One(2, 'LT WA RecId R', false) + '|' + One(2, 'LT WA RecId: 2', false) + '|' +
            One(2, 'LT WA RecId Row Extra: 2', false) + '|' + One(2, '"LT WA RecId Row": 2', false) + '|' +
            One(3, 'zzz', false) + '|' + One(2, 'Table 97240: 2', false));
    end;

    procedure Used(): Text
    begin
        exit(One(2, 'LT WA RecId Row: 2', true) + '|' + One(2, 'zzz', true) + '|' + One(2, '', true));
    end;
}
"#;

#[test]
fn a_record_id_field_evaluates_as_a_statement() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Statement")),
        [
            "ok  [LT WA RecId Row: 2]",
            "ok  [LT WA RecId Row: 3]",
            "err The value \"zzz\" can't be evaluated into type RecordID. [LT WA RecId Row: 1]",
            "ok  []",
            "ok  []",
            "err The value \"abc\" can't be evaluated into type Integer. [LT WA RecId Row: 1]",
            "err The value \"LT WA RecId Row: 1,2\" can't be evaluated into type RecordID. [LT WA RecId Row: 1]",
            "ok  [LT WA RecId Two: A]",
            "ok  [LT WA RecId Row]",
        ]
        .join("|")
    );
}

#[test]
fn a_table_is_named_by_a_prefix_quotes_or_its_id() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Names")),
        [
            "ok  [LT WA RecId Row: 2]",
            "ok  [LT WA RecId Row]",
            "ok  [LT WA RecId Row: 2]",
            "err The value \"LT WA RecId Row Extra: 2\" can't be evaluated into type RecordID. [LT WA RecId Row: 1]",
            "ok  [LT WA RecId Row: 2]",
            "err The value \"zzz\" can't be evaluated into type RecordID. []",
            "ok  [LT WA RecId Row: 2]",
        ]
        .join("|")
    );
}

#[test]
fn a_record_id_field_evaluates_with_the_result_used() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Used")),
        [
            "Yes  [LT WA RecId Row: 2]",
            "No The value \"zzz\" can't be evaluated into type RecordID. [LT WA RecId Row: 1]",
            "Yes  []",
        ]
        .join("|")
    );
}
