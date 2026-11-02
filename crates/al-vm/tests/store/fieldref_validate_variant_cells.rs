//! FieldRef.Validate and FieldRef.Value := of a Variant into a field of another type. Every literal is what real BC 29 answered
//! in the corpus: fieldref/lt-y67-fieldref-validate-variant/Y67FieldRefValidateVariant.al (codeunit 93631, writer Y67).

use al_testkit::text;

const SRC: &str = r#"
table 50203 FV
{
    fields
    {
        field(1; Id; Integer) { }
        field(3; Cod; Code[20]) { }
        field(4; Int; Integer) { trigger OnValidate() begin Hits += 1; end; }
        field(7; Bool; Boolean) { }
        field(8; D; Date) { }
        field(10; DT; DateTime) { }
        field(13; G; Guid) { }
        field(14; Dur; Duration) { }
        field(15; Hits; Integer) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50203 C
{
    [TryFunction]
    local procedure TryValidate(var FRef: FieldRef; V: Variant)
    begin
        FRef.Validate(V);
    end;

    [TryFunction]
    local procedure TryAssign(var FRef: FieldRef; V: Variant)
    begin
        FRef.Value := V;
    end;

    local procedure Cell(FieldNo: Integer; V: Variant): Text
    var
        Row: Record FV;
        RecRef: RecordRef;
        FRef: FieldRef;
        Res: Text;
    begin
        RecRef.GetTable(Row);
        FRef := RecRef.Field(FieldNo);
        if TryValidate(FRef, V) then
            Res := Format(FRef.Value, 0, 9) + '/' + Format(RecRef.Field(15).Value)
        else
            Res := GetLastErrorCode();
        Clear(Row);
        RecRef.GetTable(Row);
        FRef := RecRef.Field(FieldNo);
        if TryAssign(FRef, V) then
            Res += ',' + Format(FRef.Value, 0, 9) + '/' + Format(RecRef.Field(15).Value)
        else
            Res += ',' + GetLastErrorText() + '|' + GetLastErrorCode();
        exit(Res + ' ');
    end;

    procedure Cells(): Text
    var
        Big: BigInteger;
        Du: Duration;
    begin
        Evaluate(Big, '5000000000');
        Du := 1500;
        exit(Cell(4, true) + Cell(4, 20260115D) + Cell(4, Big) + Cell(4, 'C1') + Cell(7, 42) + Cell(8, 120000T) + Cell(10, 120000T)
            + Cell(13, 42) + Cell(14, Big) + Cell(3, Du));
    end;
}
"#;

/// Validate_Into_Int_A `boolean true`, `date 20260115D`, `biginteger`; Validate_Into_Int_B `code C1` (BC 28 and later);
/// Validate_Into_Bool_A `integer 42`; Validate_Into_D_A `time 120000T`; Validate_Into_DT_A `time 120000T`; Validate_Into_G_A
/// `integer 42` (BC 28 and later); Validate_Into_Dur_A `biginteger`; Validate_Into_Cod_B `duration 1500`.
#[test]
fn a_variant_into_fields_of_other_types() {
    assert_eq!(
        text(al_testkit::run(SRC, "Cells")),
        "1/1,1/0 1479993/1,1479993/0 AL:Overflow,Arithmetic operation resulted in an overflow.|AL:Overflow \
         Evaluate:Format,The value \"C1\" can't be evaluated into type Integer.|Evaluate:Format \
         true/0,true/0 DateInvalid,The date is not valid.|DateInvalid 0001-01-02T12:00:00Z/0,0001-01-02T12:00:00Z/0 \
         Evaluate:Argument,The value \"42\" can't be evaluated into type GUID.|Evaluate:Argument \
         P57DT20H53M20.0S/0,P57DT20H53M20.0S/0 \
         StringLengthExceeded,The length of the string is 25, but it must be less than or equal to 20 characters. Value: 1 second 500 milliseconds|StringLengthExceeded "
    );
}
