//! `Validate` of a table procedure's own Rec, in a procedure the code called on the record variable (a plain call): xRec in
//! OnValidate is the procedure's blank xRec, not the record as it was before the call (types/lt-up-this-field/UpThisField.al:175
//! UP_This_ValidateTestFieldCompound: Qty := 3, ValidateQty(5) sees old=0; this.Qty := 7, this.Validate(this.Qty) sees old=0).
//! The same Validate called from a codeunit sees the record before the call (record/TestTableEventSequenceContracts.al
//! Validate_Events_SeeTheNewValueInRec_TheOldInXRec_AndCurrFieldNoZero).

use al_testkit::{run, text};

const SRC: &str = r#"
table 50992 "VXF Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Qty; Decimal) { trigger OnValidate() begin Trail += 'Q(' + Format(Qty, 0, 9) + ';old=' + Format(xRec.Qty, 0, 9) + ')'; end; }
        field(3; Trail; Text[250]) { }
    }
    keys { key(PK; "No.") { } }
    procedure ValidateQty(NewQty: Decimal) begin this.Validate(Qty, NewQty); end;
    procedure AssignAndValidateQty(NewQty: Decimal) begin this.Qty := NewQty; this.Validate(this.Qty); end;
}
codeunit 50993 P {
    procedure InProcedure(): Text
    var
        Row: Record "VXF Row";
    begin
        Row.Qty := 3;
        Row.ValidateQty(5);
        Row.AssignAndValidateQty(7);
        exit(Row.Trail);
    end;

    procedure FromCodeunit(): Text
    var
        Row: Record "VXF Row";
    begin
        Row.Qty := 3;
        Row.Validate(Qty, 5);
        Row.Qty := 7;
        Row.Validate(Qty);
        exit(Row.Trail);
    end;
}
"#;

#[test]
fn validate_in_a_plain_table_procedure_passes_its_blank_xrec() {
    assert_eq!(text(run(SRC, "InProcedure")), "Q(5;old=0)Q(7;old=0)");
    assert_eq!(text(run(SRC, "FromCodeunit")), "Q(5;old=3)Q(7;old=7)");
}
