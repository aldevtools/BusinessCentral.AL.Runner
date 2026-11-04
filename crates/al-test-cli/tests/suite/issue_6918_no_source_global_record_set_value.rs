//! `SetValue` on a TestPage control bound to a field of a global record, on a card with no source table (al-runner issue
//! 6918, the reporter's repro): the value goes into the page's record, `OK` closes the dialog and a procedure of the page
//! reads it back.

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50100 "Repro Railcar"
{
    fields { field(1; Railcar; Code[20]) { } }
    keys { key(PK; Railcar) { Clustered = true; } }
}

page 50100 "Repro Orphan Card"
{
    PageType = Card;
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            field(ctlRailcar; RailcarRec.Railcar)
            {
                ApplicationArea = All;
                Caption = 'Railcar';
            }
        }
    }

    procedure GetRailcar(): Code[20]
    begin
        exit(RailcarRec.Railcar);
    end;

    var
        RailcarRec: Record "Repro Railcar";
}

codeunit 50100 "Repro SetValue"
{
    Subtype = Test;

    var
        CardRailcar: Code[20];

    [Test]
    [HandlerFunctions('ConfirmCardHandler')]
    procedure SetValue_OnAControlOfAPageWithNoSourceTable_StoresTheValue()
    var
        Card: Page "Repro Orphan Card";
    begin
        CardRailcar := 'RC-001';
        Card.RunModal();
        if Card.GetRailcar() <> 'RC-001' then
            Error('Expected RC-001, got %1', Card.GetRailcar());
    end;

    [ModalPageHandler]
    procedure ConfirmCardHandler(var OrphanCard: TestPage "Repro Orphan Card")
    begin
        OrphanCard.ctlRailcar.SetValue(CardRailcar);
        OrphanCard.OK().Invoke();
    end;
}
"#;

#[test]
fn set_value_on_a_global_record_field_of_a_card_with_no_source_table_stores_the_value() {
    let outcomes = outcomes_in(SRC, "Repro SetValue");
    assert_eq!(
        outcomes,
        vec![(
            "SetValue_OnAControlOfAPageWithNoSourceTable_StoresTheValue".to_string(),
            al_testrun::Outcome::Pass
        )]
    );
}

/// testpage/no-source-controls/NoSourceGlobalRecordSetValue.al:180,258 (corpus PR 1121), with what BC
/// 29.0 gave.
const PROBE: &str = r#"
table 50140 "NSRow"
{
    fields
    {
        field(1; Railcar; Code[20]) { }
        field(2; Txt; Text[30]) { }
        field(3; Int; Integer) { }
        field(4; Dec; Decimal) { }
        field(5; Flag; Boolean) { }
        field(6; Dt; Date) { }
        field(7; Opt; Option) { OptionMembers = " ",Alpha,Beta; OptionCaption = ' ,Alpha cap,Beta cap'; }
        field(8; Big; BigInteger) { }
        field(9; Tm; Time) { }
        field(10; Val; Code[10]) { trigger OnValidate() begin Note := 'validated ' + Val; end; }
        field(11; Note; Text[50]) { }
    }
    keys { key(PK; Railcar) { } }
}
page 50140 "NSCard"
{
    PageType = Card; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(ctlRailcar; RowRec.Railcar) { ApplicationArea = All; }
            field(ctlTxt; RowRec.Txt) { ApplicationArea = All; }
            field(ctlInt; RowRec.Int) { ApplicationArea = All; }
            field(ctlDec; RowRec.Dec) { ApplicationArea = All; }
            field(ctlFlag; RowRec.Flag) { ApplicationArea = All; }
            field(ctlDt; RowRec.Dt) { ApplicationArea = All; }
            field(ctlOpt; RowRec.Opt) { ApplicationArea = All; }
            field(ctlBig; RowRec.Big) { ApplicationArea = All; }
            field(ctlTm; RowRec.Tm) { ApplicationArea = All; }
            field(ctlVal; RowRec.Val) { ApplicationArea = All; }
            field(ctlNote; RowRec.Note) { ApplicationArea = All; }
        }
    }
    var RowRec: Record "NSRow";
}
codeunit 50141 C
{
    procedure TextArgs(): Text
    var Card: TestPage "NSCard"; R: Text;
    begin
        Card.OpenEdit();
        Card.ctlRailcar.SetValue('rc-003'); R := Card.ctlRailcar.Value();
        Card.ctlTxt.SetValue('Some text'); R += '|' + Card.ctlTxt.Value();
        Card.ctlInt.SetValue('42'); R += '|' + Card.ctlInt.Value();
        Card.ctlDec.SetValue('1234.5'); R += '|' + Card.ctlDec.Value();
        Card.ctlFlag.SetValue('Yes'); R += '|' + Card.ctlFlag.Value();
        Card.ctlDt.SetValue('03/15/2026'); R += '|' + Card.ctlDt.Value();
        Card.ctlOpt.SetValue('Alpha'); R += '|' + Card.ctlOpt.Value();
        Card.ctlBig.SetValue('1234567890123'); R += '|' + Card.ctlBig.Value();
        Card.ctlTm.SetValue('10:30:00'); R += '|' + Card.ctlTm.Value();
        Card.Close();
        exit(R);
    end;
    procedure TypedArgs(): Text
    var Card: TestPage "NSCard"; R: Text;
    begin
        Card.OpenEdit();
        Card.ctlRailcar.SetValue(123); R := Card.ctlRailcar.Value();
        Card.ctlTxt.SetValue(7.25); R += '|' + Card.ctlTxt.Value();
        Card.ctlInt.SetValue(42); R += '|' + Card.ctlInt.Value();
        Card.ctlDec.SetValue(12.5); R += '|' + Card.ctlDec.Value();
        Card.ctlFlag.SetValue(true); R += '|' + Card.ctlFlag.Value();
        Card.ctlDt.SetValue(DMY2Date(15, 3, 2026)); R += '|' + Card.ctlDt.Value();
        Card.ctlOpt.SetValue(2); R += '|' + Card.ctlOpt.Value();
        Card.ctlBig.SetValue(1234567890123L); R += '|' + Card.ctlBig.Value();
        Card.ctlTm.SetValue(103000T); R += '|' + Card.ctlTm.Value();
        Card.Close();
        exit(R);
    end;
    procedure Errors(): Text
    var Card: TestPage "NSCard"; R: Text;
    begin
        Card.OpenEdit();
        Card.ctlInt.SetValue('5');
        asserterror Card.ctlInt.SetValue('abc');
        R := GetLastErrorCode() + ':' + GetLastErrorText() + '|' + Card.ctlInt.Value();
        asserterror Card.ctlFlag.SetValue('maybe');
        R += '|' + GetLastErrorText();
        asserterror Card.ctlRailcar.SetValue('ABCDEFGHIJKLMNOPQRSTU');
        R += '|' + GetLastErrorText() + '|' + Card.ctlRailcar.Value();
        asserterror Card.ctlTxt.SetValue('1234567890123456789012345678901');
        R += '|' + GetLastErrorText();
        asserterror Card.ctlOpt.SetValue('Alpha cap');
        R += '|' + GetLastErrorText();
        Card.Close();
        exit(R);
    end;
    procedure TableOnValidate(): Text
    var Card: TestPage "NSCard";
    begin
        Card.OpenEdit();
        Card.ctlVal.SetValue('v1');
        exit(Card.ctlVal.Value() + '|' + Card.ctlNote.Value());
    end;
}
"#;

/// SetValue_EveryKind_TextArgs_ValueReadBack.
#[test]
fn text_arguments_on_global_record_controls_read_back() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(PROBE, "C", "TextArgs")),
        "RC-003|Some text|42|1,234.50|Yes|3/15/2026|Alpha|1234567890123|10:30:00 AM"
    );
}

/// SetValue_EveryKind_TypedArgs_ValueReadBack.
#[test]
fn typed_arguments_on_global_record_controls_read_back() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(PROBE, "C", "TypedArgs")),
        "123|7.25|42|12.50|Yes|3/15/2026|Beta|1234567890123|10:30:00 AM"
    );
}

/// SetValue_TextThatDoesNotFit_Errors.
#[test]
fn a_text_that_does_not_fit_a_global_record_field_is_a_validation_error() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(PROBE, "C", "Errors")),
        "TestValidation:Validation error for Field: ctlInt,  Message = 'Your entry of 'abc' is not an acceptable value for 'ctlInt'. The value \"abc\" can't be evaluated into type Integer.'|5\
         |Validation error for Field: ctlFlag,  Message = 'Your entry of 'maybe' is not an acceptable value for 'ctlFlag'.'\
         |Validation error for Field: ctlRailcar,  Message = 'The length of the string is 21, but it must be less than or equal to 20 characters. Value: ABCDEFGHIJKLMNOPQRSTU'|\
         |Validation error for Field: ctlTxt,  Message = 'The length of the string is 31, but it must be less than or equal to 30 characters. Value: 1234567890123456789012345678901'\
         |Validation error for Field: ctlOpt,  Message = 'Your entry of 'Alpha cap' is not an acceptable value for 'ctlOpt'.'"
    );
}

/// SetValue_FieldWithTableOnValidate: the table field's OnValidate does not run.
#[test]
fn set_value_on_a_global_record_control_does_not_run_the_table_on_validate() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(PROBE, "C", "TableOnValidate")),
        "V1|"
    );
}
