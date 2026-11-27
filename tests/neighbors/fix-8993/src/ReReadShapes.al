// The re-read shapes of corpus PR 1613 (RowVersionReReadShapes.al): read/before same after an Insert buffer read.
codeunit 98997 "RV8993 ReRead Steps"
{
    procedure Run(Send: Text; Read: Text)
    var
        Row: Record "RV8993 Item";
        First: Record "RV8993 Item";
        Row2: Record "RV8993 Item";
        Other: Record "RV8993 Setup";
        Before: BigInteger;
        Value: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        if Read = 'next' then begin
            First.Code := '0';
            First.Insert();
        end;
        if Send = 'same' then begin
            Row2.Code := '2';
            Row2.Insert();
        end;
        Commit();
        if Send = 'same' then
            Row2.Get('2');
        Row.Code := '1';
        Row.Insert();
        Before := Row.SystemRowVersion;
        case Send of
            'other', 'othercommit':
                begin
                    Other.Code := 'S';
                    Other.Insert();
                    Other.Note := 'changed';
                    Other.Modify();
                    if Send = 'othercommit' then
                        Commit();
                end;
            'otherget':
                begin
                    Other.Code := 'S';
                    Other.Insert();
                    Other.Get('S');
                end;
            'same':
                begin
                    Row2.Note := 'changed';
                    Row2.Modify();
                end;
        end;
        Value := ReRead(Read);
        if not (Before > 0) then
            Error('before not above 0');
        if Value <> Before then
            Error('read %1 before %2', Value, Before);
    end;

    procedure RunNoBefore()
    var
        Row: Record "RV8993 Item";
        Other: Record "RV8993 Setup";
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Row.Code := '1';
        Row.Insert();
        Other.Code := 'S';
        Other.Insert();
        Other.Note := 'changed';
        Other.Modify();
        if not (ReRead('recref') > 0) then
            Error('not above 0');
    end;

    local procedure ReRead(Read: Text): BigInteger
    var
        Rd: Record "RV8993 Item";
        Rd2: Record "RV8993 Item";
        RecRef: RecordRef;
        FldRef: FieldRef;
    begin
        case Read of
            'get':
                Rd.Get('1');
            'findfirst':
                Rd.FindFirst();
            'findset':
                Rd.FindSet();
            'findminus':
                Rd.Find('-');
            'next':
                begin
                    Rd.FindSet();
                    Rd.Next();
                end;
            'recref':
                begin
                    RecRef.Open(Database::"RV8993 Item");
                    RecRef.FindFirst();
                    FldRef := RecRef.Field(0);
                    exit(FldRef.Value());
                end;
            'getrecref':
                begin
                    Rd.Get('1');
                    RecRef.GetTable(Rd);
                    FldRef := RecRef.Field(0);
                    exit(FldRef.Value());
                end;
            'assign':
                begin
                    Rd.Get('1');
                    Rd2 := Rd;
                    exit(Rd2.SystemRowVersion);
                end;
            'copy':
                begin
                    Rd.Get('1');
                    Rd2.Copy(Rd);
                    exit(Rd2.SystemRowVersion);
                end;
        end;
        exit(Rd.SystemRowVersion);
    end;
}

codeunit 98998 "RV8993 ReRead Tests"
{
    Subtype = Test;

    var
        Steps: Codeunit "RV8993 ReRead Steps";

    [Test]
    procedure Measured_other_get()
    begin
        Steps.Run('other', 'get');
    end;

    [Test]
    procedure Measured_other_findfirst()
    begin
        Steps.Run('other', 'findfirst');
    end;

    [Test]
    procedure Measured_other_findset()
    begin
        Steps.Run('other', 'findset');
    end;

    [Test]
    procedure Measured_other_findminus()
    begin
        Steps.Run('other', 'findminus');
    end;

    [Test]
    procedure Measured_other_next()
    begin
        Steps.Run('other', 'next');
    end;

    [Test]
    procedure Measured_other_recref()
    begin
        Steps.Run('other', 'recref');
    end;

    [Test]
    procedure Measured_other_getrecref()
    begin
        Steps.Run('other', 'getrecref');
    end;

    [Test]
    procedure Measured_other_assign()
    begin
        Steps.Run('other', 'assign');
    end;

    [Test]
    procedure Measured_other_copy()
    begin
        Steps.Run('other', 'copy');
    end;

    [Test]
    procedure Measured_othercommit_get()
    begin
        Steps.Run('othercommit', 'get');
    end;

    [Test]
    procedure Measured_othercommit_findfirst()
    begin
        Steps.Run('othercommit', 'findfirst');
    end;

    [Test]
    procedure Measured_othercommit_findset()
    begin
        Steps.Run('othercommit', 'findset');
    end;

    [Test]
    procedure Measured_othercommit_findminus()
    begin
        Steps.Run('othercommit', 'findminus');
    end;

    [Test]
    procedure Measured_othercommit_next()
    begin
        Steps.Run('othercommit', 'next');
    end;

    [Test]
    procedure Measured_othercommit_recref()
    begin
        Steps.Run('othercommit', 'recref');
    end;

    [Test]
    procedure Measured_othercommit_getrecref()
    begin
        Steps.Run('othercommit', 'getrecref');
    end;

    [Test]
    procedure Measured_othercommit_assign()
    begin
        Steps.Run('othercommit', 'assign');
    end;

    [Test]
    procedure Measured_othercommit_copy()
    begin
        Steps.Run('othercommit', 'copy');
    end;

    [Test]
    procedure Measured_same_get()
    begin
        Steps.Run('same', 'get');
    end;

    [Test]
    procedure Measured_same_findfirst()
    begin
        Steps.Run('same', 'findfirst');
    end;

    [Test]
    procedure Measured_same_findset()
    begin
        Steps.Run('same', 'findset');
    end;

    [Test]
    procedure Measured_same_findminus()
    begin
        Steps.Run('same', 'findminus');
    end;

    [Test]
    procedure Measured_same_next()
    begin
        Steps.Run('same', 'next');
    end;

    [Test]
    procedure Measured_same_recref()
    begin
        Steps.Run('same', 'recref');
    end;

    [Test]
    procedure Measured_same_getrecref()
    begin
        Steps.Run('same', 'getrecref');
    end;

    [Test]
    procedure Measured_same_assign()
    begin
        Steps.Run('same', 'assign');
    end;

    [Test]
    procedure Measured_same_copy()
    begin
        Steps.Run('same', 'copy');
    end;

    [Test]
    procedure Refused_BeforeRead_ThenGetOfOtherTable()
    begin
        Steps.Run('otherget', 'get');
    end;

    [Test]
    procedure Refused_NoBeforeRead_Issue8993Shape()
    begin
        Steps.RunNoBefore();
    end;
}
