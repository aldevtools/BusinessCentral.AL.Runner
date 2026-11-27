codeunit 98996 "RV8993 Insert Buffer Tests"
{
    Subtype = Test;

    [Test]
    procedure Refused_InsertBufferAfterSend_ComparedWithZero()
    var
        Item: Record "RV8993 Item";
        Item2: Record "RV8993 Item";
        Setup: Record "RV8993 Setup";
    begin
        Item.DeleteAll();
        Setup.DeleteAll();
        Item.Code := 'A';
        Item.Insert();
        Setup.Code := 'S';
        Setup.Insert();
        Setup.Get('S');
        Item2.FindFirst();
        if Item.SystemRowVersion > 0 then
            Error('above 0');
    end;

    [Test]
    procedure Refused_InsertBufferAfterCommit_ComparedWithZero()
    var
        Item: Record "RV8993 Item";
        Setup: Record "RV8993 Setup";
    begin
        Item.DeleteAll();
        Setup.DeleteAll();
        Item.Code := 'A';
        Item.Insert();
        Setup.Code := 'S';
        Setup.Insert();
        Setup.Get('S');
        Commit();
        if Item.SystemRowVersion > 0 then
            Error('above 0');
    end;

    local procedure InsertTwoTablesGetOther()
    var
        Item: Record "RV8993 Item";
        Setup: Record "RV8993 Setup";
    begin
        Item.DeleteAll();
        Setup.DeleteAll();
        Commit();
        Item.Code := 'A';
        Item.Insert();
        Setup.Code := 'S';
        Setup.Insert();
        Setup.Get('S');
    end;

    [Test]
    procedure Refused_GetAfterGet_ComparedWithZero()
    var
        Item: Record "RV8993 Item";
    begin
        InsertTwoTablesGetOther();
        Item.Get('A');
        if not (Item.SystemRowVersion > 0) then
            Error('not above 0');
    end;

    [Test]
    procedure Refused_FindReRead_ComparedWithZero()
    var
        Item: Record "RV8993 Item";
    begin
        InsertTwoTablesGetOther();
        Item.FindFirst();
        if not (Item.SystemRowVersion > 0) then
            Error('not above 0');
    end;

    [Test]
    procedure Refused_GetAfterCommit_ComparedWithZero()
    var
        Item: Record "RV8993 Item";
    begin
        InsertTwoTablesGetOther();
        Commit();
        Item.Get('A');
        if not (Item.SystemRowVersion > 0) then
            Error('not above 0');
    end;

    [Test]
    procedure Refused_CopyOfGetBuffer_ComparedWithZero()
    var
        Item: Record "RV8993 Item";
        Item2: Record "RV8993 Item";
    begin
        InsertTwoTablesGetOther();
        Item.Get('A');
        Item2 := Item;
        if not (Item2.SystemRowVersion > 0) then
            Error('not above 0');
    end;
}
