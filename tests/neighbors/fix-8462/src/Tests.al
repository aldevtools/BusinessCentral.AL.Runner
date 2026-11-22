codeunit 50150 "N8462 Tests"
{
    Subtype = Test;

    local procedure Names(TableNumber: Integer; NameFilter: Text): Text
    var
        Fld: Record Field;
        R: Text;
    begin
        Fld.SetRange(TableNo, TableNumber);
        Fld.SetFilter(FieldName, NameFilter);
        if Fld.FindSet() then
            repeat
                R += Fld.FieldName + ';';
            until Fld.Next() = 0;
        exit(R);
    end;

    local procedure Check(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('got %1, expected %2', Actual, Expected);
    end;

    // The reporter's filter (al-runner issue 8462): FindSet, Count, FindFirst, FindLast and IsEmpty.
    [Test]
    procedure NameOrWildcard_FindSet()
    var
        Fld: Record Field;
        R: Text;
    begin
        Fld.SetRange(TableNo, Database::"N8462 Entry");
        Fld.SetFilter(FieldName, '%1|%2', 'Description', 'KeyField*');
        Fld.SetLoadFields("No.", FieldName);
        if Fld.FindSet() then
            repeat
                R += Fld.FieldName + ';';
            until Fld.Next() = 0;
        Check(R, 'Description;KeyField1;KeyField2;');
    end;

    [Test]
    procedure NameOrWildcard_Count()
    var
        Fld: Record Field;
    begin
        Fld.SetRange(TableNo, Database::"N8462 Entry");
        Fld.SetFilter(FieldName, '%1|%2', 'Description', 'KeyField*');
        Check(Format(Fld.Count()), '3');
    end;

    [Test]
    procedure NameOrWildcard_FindFirstAndLast()
    var
        Fld: Record Field;
    begin
        Fld.SetRange(TableNo, Database::"N8462 Entry");
        Fld.SetFilter(FieldName, '%1|%2', 'Description', 'KeyField*');
        Fld.FindFirst();
        Check(Fld.FieldName, 'Description');
        Fld.FindLast();
        Check(Fld.FieldName, 'KeyField2');
        Check(Format(Fld.IsEmpty()), 'No');
    end;

    [Test]
    procedure NameOrWildcard_NoMatchIsEmpty()
    var
        Fld: Record Field;
    begin
        Fld.SetRange(TableNo, Database::"N8462 Entry");
        Fld.SetFilter(FieldName, 'Nothing|Other*');
        Check(Format(Fld.IsEmpty()), 'Yes');
    end;

    [Test]
    procedure NameOrWildcard_TemporaryTable()
    begin
        Check(Names(Database::"N8462 Temporary", 'Description|KeyField*'), 'Description;KeyField1;');
    end;

    [Test]
    procedure NameOrWildcard_TableNoRange()
    var
        Fld: Record Field;
    begin
        Fld.SetRange(TableNo, Database::"N8462 Entry", Database::"N8462 Temporary");
        Fld.SetFilter(FieldName, 'Description|KeyField*');
        Check(Format(Fld.Count()), '5');
    end;

    [Test]
    procedure NameOrCaseInsensitiveWildcard()
    begin
        Check(Names(Database::"N8462 Entry", 'Description|@keyfield*'), 'Description;KeyField1;KeyField2;');
    end;

    [Test]
    procedure NameOrNotEqual_Refused()
    begin
        Names(Database::"N8462 Entry", 'Description|<>KeyField1');
    end;

    // Main's answer kept: an alternative of names only.
    [Test]
    procedure NameOrName()
    begin
        Check(Names(Database::"N8462 Entry", 'Description|KeyField1'), 'Description;KeyField1;');
    end;

    // Main's refusal kept: a part that matches timestamp in some case, a part of other letters, the OR group.
    [Test]
    procedure WildcardMatchingTimestamp_Refused()
    begin
        Names(Database::"N8462 Entry", 'Description|*stamp');
    end;

    [Test]
    procedure WildcardMatchingTimestampWithoutCase_Refused()
    begin
        Names(Database::"N8462 Entry", 'Description|TIME*');
    end;

    [Test]
    procedure TimestampWithoutCaseOrWildcard_Refused()
    begin
        Names(Database::"N8462 Entry", 'TIMESTAMP|Key*');
    end;

    [Test]
    procedure TimestampOrWildcard_Refused()
    begin
        Names(Database::"N8462 Entry", 'timestamp|Key*');
    end;

    [Test]
    procedure NonAsciiWildcard_Refused()
    begin
        Names(Database::"N8462 Entry", 'Beschreibung|Schlüssel*');
    end;

    [Test]
    procedure OrGroup_Refused()
    var
        Fld: Record Field;
    begin
        Fld.SetRange(TableNo, Database::"N8462 Entry");
        Fld.FilterGroup(-1);
        Fld.SetFilter(FieldName, 'Description|KeyField*');
        Fld.SetFilter("No.", '1');
        if Fld.FindFirst() then;
    end;
}
