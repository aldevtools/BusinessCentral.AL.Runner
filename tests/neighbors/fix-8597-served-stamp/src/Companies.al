// Companies that serve rows to a copy, and the reads whose answers the store caches (key order, secondary-key reads,
// case-insensitive filters, counts and sums). Every source holds 70 rows, more than the store needs to index a key.
codeunit 50172 "N97 Companies"
{
    procedure Make(CompanyName: Text; Prefix: Text; Shift: Integer; ItemName: Text; Factor: Decimal)
    var
        Company: Record Company;
        Item: Record "N97 Item";
        Grp: Record "N97 Group";
        i: Integer;
    begin
        Company.Init();
        Company.Name := CopyStr(CompanyName, 1, MaxStrLen(Company.Name));
        Company.Insert();
        Item.ChangeCompany(CompanyName);
        for i := 0 to 69 do begin
            Item.Init();
            Item.No := CopyStr(Prefix + Format(100 + i), 1, MaxStrLen(Item.No));
            Item.Group := (i + Shift) mod 7;
            Item.Name := CopyStr(ItemName, 1, MaxStrLen(Item.Name));
            Item.Amount := i * Factor;
            Item.Insert();
        end;
        Grp.ChangeCompany(CompanyName);
        Grp.No := 3;
        Grp.Insert();
    end;

    procedure Drop(CompanyName: Text)
    var
        Company: Record Company;
    begin
        if Company.Get(CompanyName) then
            Company.Delete(true);
        Commit();
    end;

    procedure Exists(CompanyName: Text): Boolean
    var
        Company: Record Company;
    begin
        exit(Company.Get(CompanyName));
    end;

    // The runner copies only when the open transaction has not written to the Company table, so every copy and drop
    // commits, except CopyOpen, which leaves the copy in the transaction for a rollback to take back.
    procedure Copy(Source: Text; Destination: Text)
    begin
        Database.CopyCompany(Source, Destination);
        Commit();
    end;

    procedure CopyOpen(Source: Text; Destination: Text)
    begin
        Database.CopyCompany(Source, Destination);
    end;

    // The keys of one Group, read in the key (Group, No).
    procedure GroupKeys(CompanyName: Text; GroupNo: Integer) Result: Text
    var
        Item: Record "N97 Item";
    begin
        Item.ChangeCompany(CompanyName);
        Item.SetCurrentKey(Group, No);
        Item.SetRange(Group, GroupNo);
        if Item.FindSet() then
            repeat
                Result += Item.No + '/' + Format(Item.Group) + ';';
            until Item.Next() = 0;
    end;

    // Every row, unfiltered, in the key (Group, No).
    procedure SecondaryOrder(CompanyName: Text) Result: Text
    var
        Item: Record "N97 Item";
    begin
        Item.ChangeCompany(CompanyName);
        Item.SetCurrentKey(Group, No);
        if Item.FindSet() then
            repeat
                Result += Item.No + '/' + Format(Item.Group) + ';';
            until Item.Next() = 0;
    end;

    procedure GroupCount(CompanyName: Text; GroupNo: Integer): Integer
    var
        Item: Record "N97 Item";
    begin
        Item.ChangeCompany(CompanyName);
        Item.SetRange(Group, GroupNo);
        exit(Item.Count());
    end;

    procedure NameCount(CompanyName: Text; NameFilter: Text): Integer
    var
        Item: Record "N97 Item";
    begin
        Item.ChangeCompany(CompanyName);
        Item.SetFilter(Name, NameFilter);
        exit(Item.Count());
    end;

    procedure GroupTotal(CompanyName: Text): Decimal
    var
        Grp: Record "N97 Group";
    begin
        Grp.ChangeCompany(CompanyName);
        Grp.Get(3);
        Grp.CalcFields(Total);
        exit(Grp.Total);
    end;

    procedure SetGroup(CompanyName: Text; ItemNo: Code[20]; GroupNo: Integer)
    var
        Item: Record "N97 Item";
    begin
        Item.ChangeCompany(CompanyName);
        Item.Get(ItemNo);
        Item.Group := GroupNo;
        Item.Modify();
    end;

    procedure Check(What: Text; Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('%1: got %2, expected %3', What, Actual, Expected);
    end;
}
