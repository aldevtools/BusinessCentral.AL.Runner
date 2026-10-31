//! `Format` in code of an amount (AutoFormatType 1) or unit amount (2) field whose AutoFormatExpression is a procedure of the
//! table that returns a currency code (gap G5: Reminder Line GetCurrencyCodeFromHeader, G/L Entry
//! GetAdditionalReportingCurrencyCode, Bank Acc. Reconciliation GetCurrencyCode). BC calls the procedure once per Format on the
//! formatted record, also a local one; the table globals of that record variable keep their values between calls; an error
//! in it is the error of the Format. Pinned by corpus tests/al-language/types/lt-auto-format-currency/Y17AfProcedure.al
//! (each test cites its line). The AL below is my own smaller version of that file.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 4 Currency
{
    fields { field(1; Code; Code[10]) { } field(2; "Amount Decimal Places"; Text[5]) { } field(3; "Unit-Amount Decimal Places"; Text[5]) { } }
    keys { key(PK; Code) { } }
}
table 98 "General Ledger Setup"
{
    fields { field(1; "Primary Key"; Code[10]) { } field(2; "Amount Decimal Places"; Text[5]) { } field(3; "Unit-Amount Decimal Places"; Text[5]) { } }
    keys { key(PK; "Primary Key") { } }
}
table 50110 "AFP Header"
{
    fields { field(1; "No."; Code[20]) { } field(2; "Currency Code"; Code[10]) { } }
    keys { key(PK; "No.") { } }
}
table 50111 "AFP Line"
{
    fields
    {
        field(1; "Doc No."; Code[20]) { }
        field(3; A1; Decimal) { AutoFormatType = 1; AutoFormatExpression = FromHeader(); }
        field(4; A2; Decimal) { AutoFormatType = 2; AutoFormatExpression = LocalFromHeader(); }
        field(5; A1Cached; Decimal) { AutoFormatType = 1; AutoFormatExpression = Cached(); }
        field(6; A1Dp; Decimal) { DecimalPlaces = 0 : 5; AutoFormatType = 1; AutoFormatExpression = FromHeader(); }
        field(7; AErr; Decimal) { AutoFormatType = 1; AutoFormatExpression = Failing(); }
    }
    keys { key(PK; "Doc No.") { } }
    var
        Calls: Text;
        Read: Boolean;
    procedure FromHeader(): Code[10]
    var H: Record "AFP Header";
    begin
        Calls += 'hdr(' + Rec."Doc No." + ');';
        if H.Get(Rec."Doc No.") then
            exit(H."Currency Code");
        exit('');
    end;
    local procedure LocalFromHeader(): Code[10]
    begin
        exit(FromHeader());
    end;
    procedure Cached(): Code[10]
    begin
        Calls += 'cached;';
        if not Read then begin
            Read := true;
            Calls += 'read;';
        end;
        exit('C3');
    end;
    procedure Failing(): Code[10]
    begin
        if Rec."Doc No." = 'E' then
            Error('AF failed');
        exit('');
    end;
    procedure TakeCalls(): Text
    var T: Text;
    begin
        T := Calls;
        Calls := '';
        exit(T);
    end;
}
codeunit 50112 "AFP Test"
{
    local procedure Seed()
    var GL: Record "General Ledger Setup"; Cur: Record Currency; H: Record "AFP Header"; L: Record "AFP Line";
    begin
        GL."Amount Decimal Places" := '2:2';
        GL."Unit-Amount Decimal Places" := '2:5';
        GL.Insert();
        Cur.Code := 'C3';
        Cur."Amount Decimal Places" := '3:3';
        Cur."Unit-Amount Decimal Places" := '4:4';
        Cur.Insert();
        H."No." := 'H1'; H."Currency Code" := 'C3'; H.Insert();
        H."No." := 'H2'; H."Currency Code" := ''; H.Insert();
        L.A1 := 1234.56789; L.A2 := 1234.56789; L.A1Cached := 1234.56789; L.A1Dp := 1234.56789; L.AErr := 1234.56789;
        L."Doc No." := 'H1'; L.Insert();
        L."Doc No." := 'H2'; L.Insert();
        L."Doc No." := 'HX'; L.Insert();
        L."Doc No." := 'E'; L.Insert();
    end;

    procedure Amounts(): Text
    var L: Record "AFP Line"; T: Text;
    begin
        Seed();
        L.SetFilter("Doc No.", '<>E');
        L.FindSet();
        repeat
            T += Format(L.A1) + '/' + Format(L.A2) + '/' + L.TakeCalls();
        until L.Next() = 0;
        exit(T);
    end;

    procedure Cache(): Text
    var L: Record "AFP Line"; L2: Record "AFP Line"; T: Text;
    begin
        Seed();
        L.Get('H1');
        L2.Get('H2');
        T := Format(L.A1Cached) + L.TakeCalls();
        T += Format(L.A1Cached) + L.TakeCalls();
        T += Format(L2.A1Cached) + L2.TakeCalls();
        exit(T);
    end;

    procedure Forms(): Text
    var L: Record "AFP Line";
    begin
        Seed();
        L.Get('H1');
        exit(StrSubstNo('%1', L.A1) + '|' + Format(L.A1, 0, 1) + '|' + Format(L.A1, 0, 9) + '|' + L.TakeCalls());
    end;

    procedure DecimalPlaces(): Text
    var L: Record "AFP Line";
    begin
        Seed();
        L.Get('H1');
        exit(Format(L.A1Dp) + '|' + L.TakeCalls());
    end;

    procedure Fails(): Text
    var L: Record "AFP Line"; T: Text;
    begin
        Seed();
        L.Get('E');
        T := 'unset';
        asserterror T := Format(L.AErr);
        exit(T + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;
}
"#;

fn call(routine: &str) -> Result<al_builtins::value::Value, Signal> {
    al_testkit::run_in(SRC, "AFP Test", routine)
}

#[test]
fn the_procedure_runs_once_per_format_on_the_formatted_record() {
    // Y17AfProcedure.al:275 AF_Proc_T1_Header, :300 AF_Proc_T2_Header, :325 AF_Proc_T2_LocalProcedure.
    assert_eq!(
        text(call("Amounts")),
        "1,234.568/1,234.5679/hdr(H1);hdr(H1);1,234.57/1,234.56789/hdr(H2);hdr(H2);1,234.57/1,234.56789/hdr(HX);hdr(HX);"
    );
}

#[test]
fn the_table_globals_of_the_record_variable_keep_their_values() {
    // Y17AfProcedure.al:350 AF_Proc_T1_SetupCached.
    assert_eq!(
        text(call("Cache")),
        "1,234.568cached;read;1,234.568cached;1,234.568cached;read;"
    );
}

#[test]
fn strsubstno_and_the_standard_formats_call_it_too() {
    // Y17AfProcedure.al:377 AF_Proc_StrSubstNo.
    assert_eq!(
        text(call("Forms")),
        "1,234.568|1234.568|1234.568|hdr(H1);hdr(H1);hdr(H1);"
    );
}

#[test]
fn the_auto_format_wins_over_decimal_places() {
    // Y17AfProcedure.al:441 AF_DecimalPlaces_T1_Procedure.
    assert_eq!(text(call("DecimalPlaces")), "1,234.568|hdr(H1);");
}

#[test]
fn an_error_in_the_procedure_is_the_error_of_the_format() {
    // Y17AfProcedure.al:511 AF_Proc_Error.
    assert_eq!(text(call("Fails")), "unset|AF failed|Dialog");
}
