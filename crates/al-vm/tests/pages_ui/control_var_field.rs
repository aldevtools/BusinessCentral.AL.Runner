//! A control bound to a field of another record variable of the page (`Settings."Avail. Filter"`, al-runner issue 5822): it takes
//! the field's OptionCaption (BC 29), DecimalPlaces and AutoFormat, in a group and in a repeater, but not BlankZero
//! (pages/lt-sa-other-record-control/SaOtherRecordControl.al:9-15). OptionCaptionML there is refused.

use al_builtins::assumptions::take_hits;
use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50100 "Repro Avail. Settings"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; "Avail. Filter"; Option)
        {
            OptionCaption = 'All,Blank,All available,Some available,None available';
            OptionMembers = " ",Blank,All,Some,"None";
        }
        field(3; Places; Decimal) { DecimalPlaces = 3 : 3; }
        field(4; Bare; Option) { OptionMembers = " ",One,Two; }
        field(5; BZ; Decimal) { BlankZero = true; DecimalPlaces = 2 : 2; }
        field(6; "Currency Code"; Code[10]) { }
        field(7; Amt; Decimal) { AutoFormatType = 1; AutoFormatExpression = Rec."Currency Code"; }
        field(8; Ml; Option) { OptionMembers = A,B; OptionCaptionML = ENU = 'x,y'; }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}
table 4 Currency
{
    fields { field(1; Code; Code[10]) { } field(2; "Amount Decimal Places"; Text[5]) { } field(3; "Unit-Amount Decimal Places"; Text[5]) { } }
    keys { key(PK; Code) { } }
}
table 50101 "Repro Avail. Line"
{
    fields { field(1; "Entry No."; Integer) { } }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}
page 50102 "Repro Avail. Worksheet"
{
    PageType = Worksheet;
    SourceTable = "Repro Avail. Line";
    layout
    {
        area(Content)
        {
            group(Filters)
            {
                field(AvailFilter; Settings."Avail. Filter") { Caption = 'Material Availability'; }
                field(PlacesCtl; Settings.Places) { }
                field(BareCtl; Settings.Bare) { }
                field(BZCtl; Settings.BZ) { }
                field(AmtCtl; Settings.Amt) { }
                field(MlCtl; Settings.Ml) { }
            }
            repeater(Lines)
            {
                field("Entry No."; Rec."Entry No.") { }
                field(LineFilter; Settings."Avail. Filter") { }
            }
        }
    }
    trigger OnOpenPage()
    begin
        if not Settings.Get('') then
            Settings.Init();
    end;
    var
        Settings: Record "Repro Avail. Settings";
}
codeunit 50104 T
{
    procedure Blank(): Text
    var Settings: Record "Repro Avail. Settings"; W: TestPage "Repro Avail. Worksheet";
    begin
        Settings.DeleteAll();
        W.OpenView();
        exit(W.AvailFilter.Value());
    end;

    procedure Named(): Text
    var Settings: Record "Repro Avail. Settings"; W: TestPage "Repro Avail. Worksheet";
    begin
        Settings.DeleteAll();
        Settings.Init();
        Settings."Avail. Filter" := Settings."Avail. Filter"::"None";
        Settings.Insert();
        W.OpenView();
        exit(W.AvailFilter.Value());
    end;

    procedure BareMembers(): Text
    var Settings: Record "Repro Avail. Settings"; W: TestPage "Repro Avail. Worksheet";
    begin
        Settings.DeleteAll();
        Settings.Init();
        Settings.Bare := Settings.Bare::Two;
        Settings.Insert();
        W.OpenView();
        exit(W.BareCtl.Value());
    end;

    procedure DecimalPlaces(): Text
    var W: TestPage "Repro Avail. Worksheet";
    begin
        W.OpenView();
        exit(W.PlacesCtl.Value());
    end;

    procedure BlankZeroAndCurrency(): Text
    var Currency: Record Currency; Settings: Record "Repro Avail. Settings"; W: TestPage "Repro Avail. Worksheet";
    begin
        Currency.Code := 'RPX';
        Currency."Amount Decimal Places" := '3:3';
        Currency."Unit-Amount Decimal Places" := '4:4';
        Currency.Insert();
        Settings.DeleteAll();
        Settings.Init();
        Settings."Currency Code" := 'RPX';
        Settings.Amt := 1234.5;
        Settings.Insert();
        W.OpenView();
        exit(W.BZCtl.Value() + '|' + W.AmtCtl.Value());
    end;

    procedure OptionCaptionMl(): Text
    var W: TestPage "Repro Avail. Worksheet";
    begin
        W.OpenView();
        exit(W.MlCtl.Value());
    end;

    procedure InRepeater(): Text
    var Line: Record "Repro Avail. Line"; W: TestPage "Repro Avail. Worksheet";
    begin
        Line."Entry No." := 1;
        Line.Insert();
        W.OpenView();
        W.First();
        exit(W.LineFilter.Value());
    end;
}
"#;

#[test]
fn an_option_field_of_another_record_shows_its_option_caption() {
    take_hits();
    assert_eq!(text(run_in(SRC, "T", "Blank")), "All");
    assert_eq!(text(run_in(SRC, "T", "Named")), "None available");
    // In a repeater too (SaOtherRecordControl.al SA_Option_EveryMember `R1`, `R2`).
    assert_eq!(text(run_in(SRC, "T", "InRepeater")), "All");
    assert!(take_hits().is_empty());
    // Without an OptionCaption the member names show, as before.
    assert_eq!(text(run_in(SRC, "T", "BareMembers")), "Two");
}

// SaOtherRecordControl.al SA_Decimal_Zero `zGBZ` (BlankZero is not taken), SA_Decimal_Currency `aGA1` (the record's currency).
#[test]
fn decimal_places_and_auto_format_of_another_records_field_are_taken() {
    assert_eq!(text(run_in(SRC, "T", "DecimalPlaces")), "0.000");
    assert_eq!(text(run_in(SRC, "T", "BlankZeroAndCurrency")), "0.00|1,234.500");
}

#[test]
fn option_caption_ml_of_another_records_field_is_refused() {
    let r = run_in(SRC, "T", "OptionCaptionMl");
    assert!(matches!(&r, Err(Signal::Unsupported(_))), "{r:?}");
}
