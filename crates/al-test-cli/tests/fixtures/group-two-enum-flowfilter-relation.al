enum 50981 "Group Relation Kind"
{
    value(0; Other) { }
    value(1; Selected) { }
}

table 50981 "Group Relation Target"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Kind; Enum "Group Relation Kind") { }
    }
    keys { key(PK; "Code") { } }
}

table 50982 "Group Relation Source"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Kind Filter"; Enum "Group Relation Kind") { FieldClass = FlowFilter; }
        field(3; Target; Code[20])
        {
            TableRelation = "Group Relation Target".Code where(Kind = field("Kind Filter"));
        }
    }
    keys { key(PK; Id) { } }
}

page 50981 "Group Relation Card"
{
    PageType = Card;
    SourceTable = "Group Relation Source";
    ApplicationArea = All;
    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Id; Rec.Id) { }
                field(Target; Rec.Target) { }
            }
        }
    }
}

codeunit 50985 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure GroupTwoFlowFilterMatchesOnTrappedNewCard()
    var
        Card: TestPage "Group Relation Card";
    begin
        OpenFilteredNewCard(Card, 1);
        Card.Target.SetValue('MATCH');
        if Card.Target.Value() <> 'MATCH' then
            Error('The matching relation was not assigned.');
        Card.Close();
    end;

    [Test]
    procedure GroupTwoFlowFilterRejectsMismatchOnTrappedNewCard()
    var
        Card: TestPage "Group Relation Card";
    begin
        OpenFilteredNewCard(Card, 2);
        asserterror Card.Target.SetValue('OTHER');
        if StrPos(GetLastErrorText(), 'Target') = 0 then
            Error('Expected validation of Target to fail, got %1', GetLastErrorText());
        Card.Close();
    end;

    [Test]
    procedure GroupTwoFlowFilterConstrainsDirectValidation()
    var
        Source: Record "Group Relation Source";
    begin
        Seed();
        SetGroupTwoFilter(Source);
        Source.Validate(Target, 'MATCH');
        Source.TestField(Target, 'MATCH');
        asserterror Source.Validate(Target, 'OTHER');
        if StrPos(GetLastErrorText(), 'OTHER') = 0 then
            Error('Expected a relation error for OTHER, got %1', GetLastErrorText());
    end;

    local procedure OpenFilteredNewCard(var Card: TestPage "Group Relation Card"; Id: Integer)
    var
        Source: Record "Group Relation Source";
    begin
        Seed();
        SetGroupTwoFilter(Source);
        Card.Trap();
        Page.Run(Page::"Group Relation Card", Source);
        Card.New();
        Card.Id.SetValue(Id);
    end;

    local procedure Seed()
    var
        Target: Record "Group Relation Target";
    begin
        Target.DeleteAll();
        Target.Code := 'MATCH';
        Target.Kind := Target.Kind::Selected;
        Target.Insert();
        Target.Code := 'OTHER';
        Target.Kind := Target.Kind::Other;
        Target.Insert();
    end;

    local procedure SetGroupTwoFilter(var Source: Record "Group Relation Source")
    begin
        Source.FilterGroup(2);
        Source.SetRange("Kind Filter", Source."Kind Filter"::Selected);
        Source.FilterGroup(0);
    end;
}
