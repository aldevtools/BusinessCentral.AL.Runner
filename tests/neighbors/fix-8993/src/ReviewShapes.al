// Shapes next to corpus PR 1613 `Run` that it does not measure (review of PR 9186): each reads the row with Get, FindFirst
// or FindSet after a send and compares it with the value read before. Each must keep main's refusal.
codeunit 98995 "RV8993 Review Steps"
{
    procedure Run(Shape: Text)
    var
        Row: Record "RV8993 Item";
        Row2: Record "RV8993 Item";
        Other: Record "RV8993 Setup";
        Third: Record "RV8993 Third";
        Rd: Record "RV8993 Item";
        Before: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Third.DeleteAll();
        if Shape in ['committedmodify', 'modifyall'] then begin
            Other.Code := 'S';
            Other.Insert();
        end;
        Commit();
        Row.Code := '1';
        Row.Insert();
        case Shape of
            'assign':
                begin
                    Row2 := Row;
                    Before := Row2.SystemRowVersion;
                end;
            'copy':
                begin
                    Row2.Copy(Row);
                    Before := Row2.SystemRowVersion;
                end;
            'byvalue':
                Before := ReadByValue(Row);
            else
                Before := Row.SystemRowVersion;
        end;
        case Shape of
            'committedmodify':
                begin
                    Other.Get('S');
                    Other.Note := 'changed';
                    Other.Modify();
                end;
            'modifyall':
                Other.ModifyAll(Note, 'changed');
            'getthenmodify':
                begin
                    Other.Code := 'S';
                    Other.Insert();
                    Other.Get('S');
                    Other.Note := 'changed';
                    Other.Modify();
                end;
            else begin
                Other.Code := 'S';
                Other.Insert();
                Other.Note := 'changed';
                Other.Modify();
            end;
        end;
        if Shape = 'thirdinsert' then begin
            Third.Code := 'T';
            Third.Insert();
        end;
        case Shape of
            'filteredfindfirst':
                begin
                    Rd.SetRange(Code, '1');
                    Rd.FindFirst();
                end;
            'filteredfindset':
                begin
                    Rd.SetRange(Code, '1');
                    Rd.FindSet();
                end;
            'keyedfindset':
                begin
                    Rd.SetCurrentKey(Note);
                    Rd.FindSet();
                end;
            else
                Rd.Get('1');
        end;
        if Rd.SystemRowVersion <> Before then
            Error('read %1 before %2', Rd.SystemRowVersion, Before);
    end;

    local procedure ReadByValue(R: Record "RV8993 Item"): BigInteger
    begin
        exit(R.SystemRowVersion);
    end;
}

codeunit 98999 "RV8993 Review Tests"
{
    Subtype = Test;

    var
        Steps: Codeunit "RV8993 Review Steps";

    [Test]
    procedure Refused_CopyAssignRead_ThenOtherModify()
    begin
        Steps.Run('assign');
    end;

    [Test]
    procedure Refused_CopyRead_ThenOtherModify()
    begin
        Steps.Run('copy');
    end;

    [Test]
    procedure Refused_ByValueParamRead_ThenOtherModify()
    begin
        Steps.Run('byvalue');
    end;

    [Test]
    procedure Refused_BeforeRead_OtherGetThenModify()
    begin
        Steps.Run('getthenmodify');
    end;

    [Test]
    procedure Refused_BeforeRead_OtherModifyAll()
    begin
        Steps.Run('modifyall');
    end;

    [Test]
    procedure Refused_BeforeRead_ModifyOfCommittedOtherRow()
    begin
        Steps.Run('committedmodify');
    end;

    [Test]
    procedure Refused_BeforeRead_ThirdInsertBeforeSend()
    begin
        Steps.Run('thirdinsert');
    end;

    [Test]
    procedure Refused_BeforeRead_FilteredFindFirstSend()
    begin
        Steps.Run('filteredfindfirst');
    end;

    [Test]
    procedure Refused_BeforeRead_FilteredFindSetSend()
    begin
        Steps.Run('filteredfindset');
    end;

    [Test]
    procedure Refused_BeforeRead_KeyedFindSetSend()
    begin
        Steps.Run('keyedfindset');
    end;

    [Test]
    procedure Measured_Other_ReadByDefaultGet()
    begin
        Steps.Run('other');
    end;
}
