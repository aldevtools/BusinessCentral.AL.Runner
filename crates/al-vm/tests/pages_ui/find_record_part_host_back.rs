//! A linked FactBox that serves its rows from OnFindRecord is read again when its host moves back with First() after GoToKey
//! (pages/lt-part-reads/PartReadsOpen.al SH_FindRecordPart_HostFirstAfterGoToKey, SH_FindRecordPart_HostNextLastPrevious): it
//! reads as after a forward move, and
//! reads nothing on a move that keeps its link value.

const SRC: &str = r#"
table 51510 "FB Line" { fields { field(1; "Line No."; Integer) { } field(2; "Group Code"; Code[10]) { } } keys { key(PK; "Line No.") { } } }
table 51511 "FB Buffer" { fields { field(1; "Group Code"; Code[10]) { } field(2; Description; Text[50]) { } } keys { key(PK; "Group Code") { } } }
codeunit 51512 "FB Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin E += T + ';'; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
page 51513 "FB Ws"
{
    PageType = Worksheet; SourceTable = "FB Line";
    layout
    {
        area(Content) { repeater(Lines) { field(LineNo; Rec."Line No.") { } field(GroupCode; Rec."Group Code") { } } }
        area(FactBoxes) { part(GroupPart; "FB Part") { SubPageLink = "Group Code" = field("Group Code"); } }
    }
}
page 51514 "FB Part"
{
    PageType = ListPart; SourceTable = "FB Buffer"; SourceTableTemporary = true;
    layout { area(Content) { repeater(Rows) { field(Description; Rec.Description) { } } } }
    trigger OnFindRecord(Which: Text): Boolean
    var L: Codeunit "FB Log";
    begin
        L.Add('Find(' + Which + ')');
        Rec.DeleteAll(); Rec.Init();
        Rec."Group Code" := CopyStr(Rec.GetRangeMin("Group Code"), 1, MaxStrLen(Rec."Group Code"));
        Rec.Description := 'Filled for ' + Rec."Group Code"; Rec.Insert();
        exit(Rec.Find(Which));
    end;
    trigger OnNextRecord(Steps: Integer): Integer var L: Codeunit "FB Log"; begin L.Add('Next(' + Format(Steps) + ')'); exit(Rec.Next(Steps)); end;
    trigger OnAfterGetRecord() var L: Codeunit "FB Log"; begin L.Add('AGR(' + Rec."Group Code" + ')'); end;
}
codeunit 51515 C
{
    procedure F(): Text
    var Line: Record "FB Line"; L: Codeunit "FB Log"; Ws: TestPage "FB Ws"; s: Text;
    begin
        Line."Line No." := 10000; Line."Group Code" := 'G1'; Line.Insert();
        Line."Line No." := 20000; Line."Group Code" := 'G2'; Line.Insert();
        Line."Line No." := 30000; Line."Group Code" := 'G2'; Line.Insert();
        Ws.OpenEdit();
        Ws.GoToKey(20000);
        s := Ws.GroupPart.Description.Value() + '|';
        L.Take();
        Ws.First();
        s += Ws.GroupPart.Description.Value() + '|' + L.Take() + '|';
        Ws.Next();
        s += L.Take() + '|';
        Ws.Next();
        exit(s + Ws.GroupPart.Description.Value() + '|' + L.Take());
    end;
}
"#;

#[test]
fn the_part_reads_as_after_a_forward_move() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "F")),
        "Filled for G2|Filled for G1|Find(=><);AGR(G1);Next(1);Next(-1);|Find(=><);AGR(G2);Next(1);Next(-1);|Filled for G2|"
    );
}
