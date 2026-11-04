//! A page control named exactly like a field of the source table, beside another control on that field: the bare
//! name in `field(<Control>; <Source>)` binds the FIELD, so both controls read and write it
//! (testpage/R6858ControlNamedLikeField.al, al-runner issue 6858). Names with a space, a dot and one word.

use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 69980 "NC Rec"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; "Container location"; Code[10]) { }
        field(3; Status; Code[10]) { }
        field(4; "Other value"; Text[30]) { }
        field(5; "Doc no."; Code[10]) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}
page 69981 "NC Card"
{
    PageType = Card; SourceTable = "NC Rec"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            field(ctlContainerlocation; "Container location") { Visible = ctlContainerlocationVisible; }
            field("Container location"; "Container location")
            {
                trigger OnValidate()
                begin
                    ValidateSeen := 'named:' + "Container location" + ':' + Status;
                end;
            }
            field(ctlStatus; Status) { }
            field(Status; Status) { }
            field(ctlOther; "Other value") { }
            field("Other value"; Rec."No.") { }
            field(Docno; "Doc no.") { }
            field("Doc no."; "Doc no.") { }
            field(ctlGuarded; Status) { Editable = "Container location" = 'DOCK'; Visible = Status <> 'HIDE'; }
            field(ctlSeen; TriggerSeen) { }
            field(ctlValidateSeen; ValidateSeen) { }
        }
    }
    trigger OnOpenPage() begin ctlContainerlocationVisible := true; end;
    trigger OnAfterGetRecord() begin TriggerSeen := "Container location" + '/' + Status + '/' + "Doc no."; end;
    var
        ctlContainerlocationVisible: Boolean;
        TriggerSeen: Text[50];
        ValidateSeen: Text[50];
}
codeunit 69982 T
{
    Subtype = Test;
    local procedure Setup()
    var R: Record "NC Rec";
    begin
        R.DeleteAll(); R."No." := 'R1'; R."Container location" := 'DOCK'; R.Status := 'OPEN';
        R."Other value" := 'OTHERVAL'; R."Doc no." := 'DOC1'; R.Insert();
    end;
    local procedure Eq(Name: Text; Got: Text; Want: Text)
    begin
        if Got <> Want then Error('%1: got %2, want %3', Name, Got, Want);
    end;
    [Test]
    procedure ReadsAndWrites()
    var Card: TestPage "NC Card"; R: Record "NC Rec";
    begin
        Setup();
        Card.OpenEdit();
        Eq('ctl', Card.ctlContainerlocation.Value(), 'DOCK');
        Eq('named', Card."Container location".Value(), 'DOCK');
        Eq('ctlStatus', Card.ctlStatus.Value(), 'OPEN');
        Eq('Status', Card.Status.Value(), 'OPEN');
        Eq('Docno', Card.Docno.Value(), 'DOC1');
        Eq('Doc no.', Card."Doc no.".Value(), 'DOC1');
        Eq('Other value shows No.', Card."Other value".Value(), 'R1');
        Eq('ctlOther', Card.ctlOther.Value(), 'OTHERVAL');
        Eq('trigger', Card.ctlSeen.Value(), 'DOCK/OPEN/DOC1');
        Eq('visible', Format(Card.ctlContainerlocation.Visible()), 'Yes');
        Eq('editable', Format(Card.ctlGuarded.Editable()), 'Yes');
        Card."Container location".SetValue('BYNAME');
        Eq('other control', Card.ctlContainerlocation.Value(), 'BYNAME');
        Eq('validate', Card.ctlValidateSeen.Value(), 'named:BYNAME:OPEN');
        Card.ctlContainerlocation.SetValue('BYCTL');
        Eq('named control', Card."Container location".Value(), 'BYCTL');
        Card.ctlStatus.SetValue('CLOSED');
        Eq('Status control', Card.Status.Value(), 'CLOSED');
        Card.ctlOther.SetValue('CHANGED');
        Eq('Other value control', Card."Other value".Value(), 'R1');
        Card.Docno.SetValue('DOC2');
        Eq('Doc no. control', Card."Doc no.".Value(), 'DOC2');
        Card."Container location".SetValue('YARD');
        Eq('editable after', Format(Card.ctlGuarded.Editable()), 'No');
        Card.Close();
        R.Get('R1');
        Eq('stored container', R."Container location", 'YARD');
        Eq('stored other', R."Other value", 'CHANGED');
    end;
}
"#;

#[test]
fn a_control_named_like_its_source_field_beside_a_control_on_that_field_binds_the_field() {
    let out = al_testkit::outcomes_in(SRC, "T");
    assert_eq!(out.len(), 1);
    for (n, o) in out {
        assert_eq!(o, Outcome::Pass, "{n}");
    }
}
