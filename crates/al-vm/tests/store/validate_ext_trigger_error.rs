//! An error in the OnBeforeValidate or OnAfterValidate trigger a table extension adds to a field puts the whole record back at its
//! values from before the Validate, as one in the field's own OnValidate does (al-runner issue 6978).

const SRC: &str = r#"
table 50970 "Vx Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Qty; Integer) { }
        field(3; Amount; Decimal) { trigger OnValidate() begin Mark := 'base-ran'; Hits += 100; end; }
        field(4; Note; Text[30]) { }
        field(5; Hits; Integer) { }
        field(6; Mark; Text[30]) { }
    }
    keys { key(PK; Id) { } }
}
tableextension 50971 "Vx Row Ext" extends "Vx Row"
{
    fields
    {
        modify(Qty)
        {
            trigger OnBeforeValidate()
            begin
                Note := 'set-by-before'; Hits += 1;
                if Qty mod 5 <> 0 then Error('Qty %1 is not a step of 5', Qty);
            end;
        }
        modify(Amount)
        {
            trigger OnAfterValidate()
            begin
                Note := 'set-by-after'; Hits += 1;
                if Amount > 100 then Error('Amount %1 is too big', Amount);
            end;
        }
    }
}
codeunit 50972 C
{
    local procedure State(var R: Record "Vx Row"): Text
    begin
        exit(StrSubstNo('%1/%2/%3/%4/%5', R.Qty, R.Amount, R.Note, R.Hits, R.Mark));
    end;

    procedure Before(): Text
    var R: Record "Vx Row"; T: Text;
    begin
        R.Id := 1;
        R.Validate(Qty, 5);
        R.Note := 'orig'; R.Hits := 10;
        asserterror R.Validate(Qty, 4);
        T := GetLastErrorText() + '|' + State(R);
        exit(T);
    end;

    procedure After(): Text
    var R: Record "Vx Row"; T: Text;
    begin
        R.Id := 1;
        R.Validate(Amount, 50);
        R.Note := 'orig'; R.Hits := 10; R.Mark := '';
        asserterror R.Validate(Amount, 500);
        T := GetLastErrorText() + '|' + State(R);
        exit(T);
    end;
}
"#;

#[test]
fn an_error_in_an_extension_on_before_validate_restores_the_record() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Before")),
        "Qty 4 is not a step of 5|5/0/orig/10/"
    );
}

#[test]
fn an_error_in_an_extension_on_after_validate_restores_the_record() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "After")),
        "Amount 500 is too big|0/50/orig/10/"
    );
}
