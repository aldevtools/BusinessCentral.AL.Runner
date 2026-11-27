// Field.RelationTableNo and RelationFieldNo of a TableRelation that names no field (issue 8592), one test per shape.
codeunit 50150 "NB RFN Tests"
{
    Subtype = Test;

    local procedure Check(TableNo: Integer; FieldNo: Integer; RelTable: Integer; RelField: Integer)
    var
        F: Record Field;
    begin
        F.Get(TableNo, FieldNo);
        if (F.RelationTableNo <> RelTable) or (F.RelationFieldNo <> RelField) then
            Error('got %1/%2, expected %3/%4', F.RelationTableNo, F.RelationFieldNo, RelTable, RelField);
    end;

    // The shapes corpus PR 1546 measured.
    [Test]
    procedure Measured_CodeToIntCode()
    begin
        Check(Database::"NB RFN Source", 13, Database::"NB RFN Int Code", 0);
    end;

    [Test]
    procedure Measured_CodeToCodeInt()
    begin
        Check(Database::"NB RFN Source", 14, Database::"NB RFN Code Int", 0);
    end;

    [Test]
    procedure Measured_IntToIntCode()
    begin
        Check(Database::"NB RFN Source", 12, Database::"NB RFN Int Code", 0);
    end;

    [Test]
    procedure Measured_IntToCodeInt()
    begin
        Check(Database::"NB RFN Source", 15, Database::"NB RFN Code Int", 0);
    end;

    [Test]
    procedure Measured_CodeToCodeText()
    begin
        Check(Database::"NB RFN Source", 11, Database::"NB RFN Code Text", 0);
    end;

    [Test]
    procedure Measured_InsideOwnIntCodeKey()
    begin
        Check(Database::"NB RFN Source", 2, Database::"NB RFN One Code", 0);
    end;

    // The source's own key is read too: a source keyed on Id alone was not measured onto a two-field key.
    [Test]
    procedure Refused_SourceKeyOnId_CodeToIntCode()
    begin
        Check(Database::"NB RFN Source Plain", 13, Database::"NB RFN Int Code", 0);
    end;

    [Test]
    procedure Refused_SourceKeyOnId_IntToCodeInt()
    begin
        Check(Database::"NB RFN Source Plain", 15, Database::"NB RFN Code Int", 0);
    end;

    // A read through a scan rather than Get.
    [Test]
    procedure Measured_CodeToIntCode_FindFirst()
    var
        F: Record Field;
    begin
        F.SetRange(TableNo, Database::"NB RFN Source");
        F.SetRange("No.", 13);
        F.FindFirst();
        if F.RelationFieldNo <> 0 then
            Error('got %1', F.RelationFieldNo);
    end;

    // Pinned before this change: same on both.
    [Test]
    procedure Pinned_ControlCodeKey()
    begin
        Check(Database::"NB RFN Source", 10, Database::"NB RFN One Code", 0);
    end;

    [Test]
    procedure Pinned_NamedSecondKeyField()
    begin
        Check(Database::"NB RFN Source", 16, Database::"NB RFN Int Code", 2);
    end;

    // Not measured: refused on both.
    [Test]
    procedure Refused_Code10ToIntCode()
    begin
        Check(Database::"NB RFN Source", 20, Database::"NB RFN Int Code", 0);
    end;

    [Test]
    procedure Refused_CodeToIntCode10()
    begin
        Check(Database::"NB RFN Source", 21, Database::"NB RFN Int Code10", 0);
    end;

    [Test]
    procedure Refused_CodeToCodeText50()
    begin
        Check(Database::"NB RFN Source", 22, Database::"NB RFN Code Text50", 0);
    end;

    [Test]
    procedure Refused_TextToIntCode()
    begin
        Check(Database::"NB RFN Source", 23, Database::"NB RFN Int Code", 0);
    end;

    [Test]
    procedure Refused_CodeToIntCodeWhereConst()
    begin
        Check(Database::"NB RFN Source", 24, Database::"NB RFN Int Code", 0);
    end;

    [Test]
    procedure Refused_CodeToThreeFieldKey()
    begin
        Check(Database::"NB RFN Source", 25, Database::"NB RFN Three", 0);
    end;

    [Test]
    procedure Refused_Code30ToCodeText()
    begin
        Check(Database::"NB RFN Source", 26, Database::"NB RFN Code Text", 0);
    end;

    [Test]
    procedure Refused_BigIntegerToIntCode()
    begin
        Check(Database::"NB RFN Source", 27, Database::"NB RFN Int Code", 0);
    end;

    [Test]
    procedure Refused_InsideOwnKeyOntoTwoFieldKey()
    begin
        Check(Database::"NB RFN Source Own Two", 2, Database::"NB RFN Int Code", 0);
    end;

    [Test]
    procedure Refused_InsideOwnCodeIntKey()
    begin
        Check(Database::"NB RFN Source Code First", 1, Database::"NB RFN One Code", 0);
    end;

    [Test]
    procedure Refused_WholeOwnKey()
    begin
        Check(Database::"NB RFN Source Own One", 1, Database::"NB RFN One Code", 0);
    end;
}
