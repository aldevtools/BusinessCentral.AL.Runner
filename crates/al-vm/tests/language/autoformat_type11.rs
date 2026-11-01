//! Format, StrSubstNo and Error in code of a Decimal table field with AutoFormatType 11: the AutoFormatExpression is the
//! format string, evaluated at each call (types/lt-y43-auto-format-11/Y43AfType11.al, corpus PR 966, en-US).

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
codeunit 69970 "AF Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Take(): Text var T: Text; begin T := L; L := ''; exit(T); end;
}
table 69971 "AF Setup"
{
    fields { field(1; PK; Code[10]) { } field(2; Places; Text[5]) { InitValue = '2:5'; } }
    keys { key(PK; PK) { } }
    procedure GetFormat(FieldNo: Integer): Text
    var Log: Codeunit "AF Log";
    begin
        Log.Add('getformat');
        if not Get() then
            Init();
        case FieldNo of
            FieldNo(Places):
                exit(StrSubstNo('<Precision,%1><Standard Format,0>', Places));
        end;
        exit('');
    end;
}
table 69972 "AF Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Lit11; Decimal) { AutoFormatType = 11; AutoFormatExpression = '<Precision,2:5><Standard Format,0>'; }
        field(4; Own11; Decimal) { AutoFormatType = 11; AutoFormatExpression = GetOwnFormat(); }
    }
    keys { key(PK; "No.") { } }
    procedure GetOwnFormat(): Text
    begin
        exit('<Precision,1:1><Standard Format,0>');
    end;
}
tableextension 69973 "AF Row Ext" extends "AF Row"
{
    fields
    {
        field(69973; Ext11; Decimal) { AutoFormatType = 11; AutoFormatExpression = S.GetFormat(S.FieldNo(Places)); }
        field(69974; Ext11Blank; Decimal) { AutoFormatType = 11; AutoFormatExpression = S.GetFormat(0); }
        field(69975; Ext11Int; Decimal) { AutoFormatType = 11; AutoFormatExpression = '<Integer Thousand><Decimals,3>'; }
    }
    var
        S: Record "AF Setup";
}
codeunit 69974 "AF Probe"
{
    procedure Ext(): Text
    var T: Record "AF Row"; S: Record "AF Setup"; Log: Codeunit "AF Log"; R: Text;
    begin
        S.Places := '2:5';
        S.Insert();
        T.Ext11 := 1234.5;
        R := StrSubstNo('%1|%2|', T.Ext11, T.Ext11) + Log.Take();
        R += Format(T.Ext11) + '|' + Format(T.Ext11, 0, 1) + '|' + Format(T.Ext11, 0, 2) + '|' + Format(T.Ext11, 0, 9) + '|';
        T.Ext11 := 0.123456;
        R += Format(T.Ext11) + '|';
        T.Ext11 := -2.5;
        R += StrSubstNo('%1', T.Ext11) + '|';
        S.Places := '0:2';
        S.Modify();
        T.Ext11 := 10;
        R += StrSubstNo('%1', T.Ext11) + '|';
        T.Ext11 := 1234.5;
        asserterror Error('%1', T.Ext11);
        exit(R + GetLastErrorText());
    end;

    procedure Others(): Text
    var T: Record "AF Row"; R: Text;
    begin
        T.Lit11 := 0.123456;
        T.Own11 := 10;
        T.Ext11Blank := 1234.5;
        T.Ext11Int := 1234.5;
        R := StrSubstNo('%1|%2|', T.Lit11, Format(T.Lit11));
        R += StrSubstNo('%1|%2|', T.Own11, Format(T.Own11));
        R += StrSubstNo('%1|%2|', T.Ext11Blank, Format(T.Ext11Blank));
        R += StrSubstNo('%1|', T.Ext11Int);
        T.Ext11Int := -2.5;
        exit(R + Format(T.Ext11Int));
    end;

    // Only the format number 0 and, for a <Precision,..><Standard Format,0> expression, 1, 2 and 9 are pinned.
    procedure Unpinned(): Text
    var T: Record "AF Row";
    begin
        T.Ext11Int := 1;
        exit(Format(T.Ext11Int, 0, 1));
    end;
}
"#;

#[test]
fn the_expression_of_a_table_extension_global_is_the_format_string() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "AF Probe", "Ext")),
        "1,234.50|1,234.50|getformat;getformat;1,234.50|1234.50|1234.50|1234.50|0.12346|-2.50|10|1,234.5"
    );
}

#[test]
fn a_literal_a_row_procedure_a_blank_result_and_a_string_without_precision() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "AF Probe", "Others")),
        "0.12346|0.12346|10.0|10.0|1,234.5|1,234.5|1,234.50|2.50"
    );
}

#[test]
fn another_format_number_is_refused() {
    let r = al_testkit::run_in(SRC, "AF Probe", "Unpinned");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}
