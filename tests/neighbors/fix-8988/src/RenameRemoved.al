// Rename of a parent row while a field of another table relates to it (issue 8988). One test per shape next to the
// reporter's: a removed Text[4] field, a removed Integer field, a Pending and a live Text[4] field, a removed field next
// to a live field with a row, and RecordRef.Rename.

table 50200 "N8988 Parent"
{
    DataClassification = SystemMetadata;
    fields { field(1; "Code"; Code[20]) { } }
    keys { key(PK; "Code") { Clustered = true; } }
}

table 50201 "N8988 Removed Child"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Old Parent Code"; Text[4])
        {
            TableRelation = "N8988 Parent"."Code";
            ObsoleteState = Removed;
            ObsoleteReason = 'Too short.';
        }
        field(3; "Parent Code"; Code[20]) { TableRelation = "N8988 Parent"."Code"; }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 50202 "N8988 Parent2"
{
    DataClassification = SystemMetadata;
    fields { field(1; "Code"; Code[20]) { } }
    keys { key(PK; "Code") { Clustered = true; } }
}

table 50203 "N8988 Removed Int Child"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Old Ref"; Integer)
        {
            TableRelation = "N8988 Parent2"."Code";
            ObsoleteState = Removed;
            ObsoleteReason = 'Wrong type.';
        }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 50204 "N8988 Parent3"
{
    DataClassification = SystemMetadata;
    fields { field(1; "Code"; Code[20]) { } }
    keys { key(PK; "Code") { Clustered = true; } }
}

table 50205 "N8988 Pending Child"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Old Parent Code"; Text[4])
        {
            TableRelation = "N8988 Parent3"."Code";
            ObsoleteState = Pending;
            ObsoleteReason = 'Too short.';
        }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 50206 "N8988 Parent4"
{
    DataClassification = SystemMetadata;
    fields { field(1; "Code"; Code[20]) { } }
    keys { key(PK; "Code") { Clustered = true; } }
}

table 50207 "N8988 Live Child"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Short Parent Code"; Text[4]) { TableRelation = "N8988 Parent4"."Code"; }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

codeunit 50200 "N8988 Tests"
{
    Subtype = Test;

    [Test]
    procedure RemovedText4_NoRows_Reporter()
    var
        Parent: Record "N8988 Parent";
    begin
        Parent.Code := 'A';
        Parent.Insert();
        Parent.Rename('GU00000031');
        Parent.Get('GU00000031');
    end;

    [Test]
    procedure RemovedText4_NoRows_RecordRefRename()
    var
        Parent: Record "N8988 Parent";
        RecRef: RecordRef;
    begin
        Parent.Code := 'B';
        Parent.Insert();
        RecRef.GetTable(Parent);
        RecRef.Rename('GU00000032');
        Parent.Get('GU00000032');
    end;

    [Test]
    procedure RemovedText4_NextToLiveFieldWithRow()
    var
        Parent: Record "N8988 Parent";
        Child: Record "N8988 Removed Child";
    begin
        Parent.Code := 'C';
        Parent.Insert();
        Child."Entry No." := 1;
        Child."Parent Code" := 'C';
        Child.Insert();
        Parent.Rename('GU00000033');
        Child.Get(1);
        if Child."Parent Code" <> 'GU00000033' then
            Error('live field not renamed: %1', Child."Parent Code");
    end;

    [Test]
    procedure RemovedInteger_NoRows()
    var
        Parent: Record "N8988 Parent2";
    begin
        Parent.Code := 'A';
        Parent.Insert();
        Parent.Rename('GU00000034');
        Parent.Get('GU00000034');
    end;

    [Test]
    procedure PendingText4_NoRows()
    var
        Parent: Record "N8988 Parent3";
    begin
        Parent.Code := 'A';
        Parent.Insert();
        Parent.Rename('GU00000035');
    end;

    [Test]
    procedure LiveText4_NoRows()
    var
        Parent: Record "N8988 Parent4";
    begin
        Parent.Code := 'A';
        Parent.Insert();
        Parent.Rename('GU00000036');
    end;
}
