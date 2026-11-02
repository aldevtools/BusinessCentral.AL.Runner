//! A set filtered down to nothing: Find('-') and Find('+') say "There is no ... within the filter", and Find(), Find('='),
//! Find('>') and Find('<') say the record does not exist, with the record's own key (writer Y81,
//! record/lt-y81-values/Y81EmptySetFinds.al Y81_RecRef_FilteredOut_Unused, on a RecordRef and a temporary Record; needs corpus
//! bump).

use al_testkit::text;

const SRC: &str = r#"
table 69973 "Y Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69973 C
{
    [TryFunction]
    local procedure TryRef(var RecRef: RecordRef; Op: Text)
    begin
        case Op of
            'Find-': RecRef.Find('-');
            'Find+': RecRef.Find('+');
            'Find': RecRef.Find();
            'Find=': RecRef.Find('=');
            'Find>': RecRef.Find('>');
            'Find<': RecRef.Find('<');
        end;
    end;

    local procedure Unused(var RecRef: RecordRef; Op: Text): Text
    begin
        ClearLastError();
        if TryRef(RecRef, Op) then exit('ok');
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure Filtered(Temp: Boolean): Text
    var
        Row: Record "Y Row" temporary;
        Perm: Record "Y Row";
        RecRef: RecordRef;
        Ops: List of [Text];
        Op: Text;
        All: Text;
    begin
        if Temp then begin
            Row.Id := 10; Row.Insert(); Row.Id := 0;
            RecRef.GetTable(Row);
        end else begin
            Perm.DeleteAll();
            Perm.Id := 10; Perm.Insert();
            RecRef.Open(Database::"Y Row");
        end;
        RecRef.Field(1).SetRange(99);
        Ops.Add('Find-'); Ops.Add('Find+'); Ops.Add('Find'); Ops.Add('Find='); Ops.Add('Find>'); Ops.Add('Find<');
        foreach Op in Ops do
            All += Unused(RecRef, Op) + '#';
        exit(All);
    end;
}
"#;

fn expected() -> String {
    let none = "There is no Y Row within the filter.\\\\Filters: Id: 99|DB:NothingInsideFilter#";
    let eq = "The Y Row does not exist. Identification fields and values: Id='0'|DB:RecordNotFound#";
    let gt = "The Y Row does not exist. >\\\\Identification fields and values:\\\\Id='0'|DB:RecordNotFound#";
    let lt = "The Y Row does not exist. <\\\\Identification fields and values:\\\\Id='0'|DB:RecordNotFound#";
    format!("{none}{none}{eq}{eq}{gt}{lt}")
}

#[test]
fn a_permanent_record_ref_filtered_to_nothing() {
    use al_builtins::value::Value;
    let r = al_testkit::run_with(SRC, "C", "Filtered", vec![Value::Boolean(false)]).map(|x| x.0);
    assert_eq!(text(r), expected());
}

#[test]
fn a_temporary_record_ref_filtered_to_nothing() {
    use al_builtins::value::Value;
    let r = al_testkit::run_with(SRC, "C", "Filtered", vec![Value::Boolean(true)]).map(|x| x.0);
    assert_eq!(text(r), expected());
}
