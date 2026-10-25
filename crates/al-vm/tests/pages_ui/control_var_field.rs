//! A control bound to a field of another record variable of the page (`Settings."Avail. Filter"`, al-runner issue 5822, the
//! reporter's real-BC repro on 29.0.54011.55407): an Option field shows its OptionCaption, as a control bound to Rec's field
//! does. Not pinned by the corpus yet (assumption other-record-control-option-caption, awaiting pages/lt-sa-other-record-control/
//! SaOtherRecordControl.al). The field's other formatting properties are refused until probed.

use al_builtins::assumptions::{Assumption, take_hits};
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
    }
    keys { key(PK; "Code") { Clustered = true; } }
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
    assert!(take_hits().contains(&Assumption::OtherRecordControlOptionCaption));
    // Without an OptionCaption the member names show, as before.
    assert_eq!(text(run_in(SRC, "T", "BareMembers")), "Two");
}

#[test]
fn other_formatting_of_another_records_field_is_refused() {
    for p in ["DecimalPlaces", "InRepeater"] {
        let r = run_in(SRC, "T", p);
        assert!(matches!(&r, Err(Signal::Unsupported(_))), "{p}: {r:?}");
    }
}
