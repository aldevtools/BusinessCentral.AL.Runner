//! An Option FlowField whose CalcFormula looks up, or takes the max of, an Enum field (al-runner issue 6481). Measured on BC by
//! corpus writer Y20 (record/lt-flowfield-lookup-enum/Y20FlowFieldLookupEnumIntoOption.al): a lookup keeps the Enum member,
//! so Format shows the Enum's member name and the ordinal is the member's (also one the Option lacks); no row gives the
//! Option's first member. A max gives an Option ordinal: Format shows the Option's member, or the number when the Option has
//! no member with it. A filter on the FlowField compares ordinals.

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn out(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(t)) => t.to_string(),
        Err(Signal::Unsupported(why)) => format!("refused:{why}"),
        other => format!("{other:?}"),
    }
}

const SRC: &str = r#"
enum 50831 "FLO Source"
{
    Extensible = true;
    value(0; Zero) { }
    value(1; One) { }
    value(2; Two) { }
    value(5; Five) { }
}
table 50832 "FLO Line"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Status; Enum "FLO Source") { }
        field(3; Grp; Code[10]) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 50833 "FLO Header"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; "Line No."; Code[10]) { }
        field(3; "Line Status"; Option)
        {
            OptionMembers = Nil,Uno,Dos;
            FieldClass = FlowField;
            CalcFormula = lookup("FLO Line".Status where("No." = field("Line No.")));
        }
        field(4; Grp; Code[10]) { }
        field(5; "Max Status"; Option)
        {
            OptionMembers = Nil,Uno,Dos;
            FieldClass = FlowField;
            CalcFormula = max("FLO Line".Status where(Grp = field(Grp)));
        }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}
codeunit 50834 T
{
    local procedure Seed()
    begin
        AddLine('L0', "FLO Source"::Zero, 'G1');
        AddLine('L1', "FLO Source"::One, 'G1');
        AddLine('L2', "FLO Source"::Two, 'G2');
        AddLine('L5', "FLO Source"::Five, 'G2');
        AddLine('L3', "FLO Source"::One, 'G3');
    end;

    local procedure AddLine(No: Code[10]; Status: Enum "FLO Source"; Grp: Code[10])
    var
        Line: Record "FLO Line";
    begin
        Line."No." := No;
        Line.Status := Status;
        Line.Grp := Grp;
        Line.Insert();
    end;

    local procedure Read(LineNo: Code[10]): Text
    var
        Header: Record "FLO Header";
        Ordinal: Integer;
    begin
        Header.DeleteAll();
        Header."No." := 'H';
        Header."Line No." := LineNo;
        Header.Insert();
        Header.CalcFields("Line Status");
        Ordinal := Header."Line Status";
        exit(Format(Header."Line Status") + '/' + Format(Ordinal) + '/' + Format(Header."Line Status", 0, 9));
    end;

    procedure Lookup(): Text
    var
        Header: Record "FLO Header";
        R: Text;
    begin
        Seed();
        R := Read('L0') + '|' + Read('L1') + '|' + Read('L2') + '|' + Read('L5');
        Header.Get('H');
        Header.CalcFields("Line Status");
        R += '|' + Format(Header."Line Status" = Header."Line Status"::Dos);
        exit(R + '|' + Read('NONE'));
    end;

    procedure Max(): Text
    var
        Header: Record "FLO Header";
        Ordinal: Integer;
        R: Text;
    begin
        Seed();
        Header."No." := 'H';
        Header.Grp := 'G1';
        Header.Insert();
        Header.CalcFields("Max Status");
        Ordinal := Header."Max Status";
        R := Format(Header."Max Status") + '/' + Format(Ordinal);
        Header.Grp := 'G2';
        Header.CalcFields("Max Status");
        Ordinal := Header."Max Status";
        R += '|' + Format(Header."Max Status") + '/' + Format(Ordinal);
        Header.Grp := 'G9';
        Header.CalcFields("Max Status");
        Ordinal := Header."Max Status";
        exit(R + '|' + Format(Header."Max Status") + '/' + Format(Ordinal));
    end;

    procedure Filter(): Text
    var
        Header: Record "FLO Header";
        R: Text;
    begin
        Seed();
        AddHeader('H0', 'L0');
        AddHeader('H1', 'L1');
        AddHeader('H2', 'L2');
        AddHeader('H3', 'L3');
        AddHeader('H5', 'L5');
        Header.SetRange("Line Status", Header."Line Status"::Uno);
        R := Format(Header.Count());
        Header.SetRange("Line Status", Header."Line Status"::Dos);
        R += '|' + Format(Header.Count());
        Header.SetFilter("Line Status", '%1', 5);
        exit(R + '|' + Format(Header.Count()));
    end;

    local procedure AddHeader(No: Code[10]; LineNo: Code[10])
    var
        Header: Record "FLO Header";
    begin
        Header."No." := No;
        Header."Line No." := LineNo;
        Header.Insert();
    end;
}
"#;

fn run(routine: &str) -> String {
    out(al_testkit::run_in(SRC, "T", routine))
}

#[test]
fn a_lookup_of_an_enum_into_an_option_flowfield_keeps_the_enum_member() {
    assert_eq!(run("Lookup"), "Zero/0/0|One/1/1|Two/2/2|Five/5/5|No|Nil/0/0");
}

#[test]
fn a_max_of_an_enum_into_an_option_flowfield_is_an_option_ordinal() {
    assert_eq!(run("Max"), "Uno/1|5/5|Nil/0");
}

#[test]
fn a_filter_on_the_option_flowfield_compares_ordinals() {
    assert_eq!(run("Filter"), "2|1|1");
}
