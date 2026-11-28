// Each test asserts the outcome BC gives in the measured cells of corpus PR 1646 or, for a neighbor, the outcome that
// rule would predict. A test the head binary refuses on purpose names its reason in differences.tsv.
codeunit 70937 "NB TSV Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        RequestCtl: Integer;
        RequestTyped: Text;
        RequestReading: Text;

    local procedure Seed()
    var
        Book: Record "NB TSV Book";
        Pair: Record "NB TSV Pair";
        Num: Record "NB TSV Num";
        Capt: Record "NB TSV Capt";
        Host: Record "NB TSV Host";
    begin
        Book.DeleteAll();
        Pair.DeleteAll();
        Num.DeleteAll();
        Capt.DeleteAll();
        Host.DeleteAll();
        AddBook('ABC', 'Shared', true);
        AddBook('ABD', 'Shared', true);
        AddBook('MNO', 'Unique Name', true);
        AddBook('XYZ', 'Hidden', false);
        Pair."Group" := 'G1';
        Pair."Code" := 'P1';
        Pair.Insert();
        Num."No" := 1;
        Num.Insert();
        Capt."Code" := 'CAP';
        Capt.Insert();
        Host.Id := 1;
        Host.Insert();
    end;

    local procedure AddBook(C: Code[10]; N: Text[30]; A: Boolean)
    var
        Book: Record "NB TSV Book";
    begin
        Book."Code" := C;
        Book.Name := N;
        Book.Active := A;
        Book.Insert();
    end;

    local procedure Check(Ok: Boolean; Expected: Text)
    var
        Actual: Text;
    begin
        if Ok then
            Actual := 'ok'
        else
            Actual := 'E: ' + GetLastErrorText();
        ClearLastError();
        if Actual <> Expected then
            Error('expected [%1] but was [%2]', Expected, Actual);
    end;

    local procedure Same(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('expected [%1] but was [%2]', Expected, Actual);
    end;

    [TryFunction]
    local procedure SetText(var TP: TestPage "NB TSV Text Var"; Typed: Text)
    begin
        TP.BookCtl.SetValue(Typed);
    end;

    [TryFunction]
    local procedure SetCode(var TP: TestPage "NB TSV Code Var"; Ctl: Integer; Typed: Text)
    begin
        case Ctl of
            1:
                TP.BookCtl.SetValue(Typed);
            2:
                TP.PairCtl.SetValue(Typed);
            3:
                TP.NumCtl.SetValue(Typed);
            4:
                TP.CaptCtl.SetValue(Typed);
            5:
                TP.ShortCtl.SetValue(Typed);
            6:
                TP.ExtCtl.SetValue(Typed);
            7:
                TP.FiltCtl.SetValue(Typed);
            8:
                TP.NamedCtl.SetValue(Typed);
        end;
    end;

    [TryFunction]
    local procedure SetFiltered(var TP: TestPage "NB TSV Filtered"; Typed: Text)
    begin
        TP.BookCtl.SetValue(Typed);
    end;

    [TryFunction]
    local procedure SetName(var TP: TestPage "NB TSV Name Var"; Typed: Text)
    begin
        TP.BookCtl.SetValue(Typed);
    end;

    [TryFunction]
    local procedure SetHost(var TP: TestPage "NB TSV Host Card"; Active: Boolean; Typed: Text)
    begin
        if Active then
            TP.ActiveCtl.SetValue(Typed)
        else
            TP.BookCtl.SetValue(Typed);
    end;

    [TryFunction]
    local procedure SetHostList(var TP: TestPage "NB TSV Host List"; Typed: Text)
    begin
        TP.BookCtl.SetValue(Typed);
    end;

    local procedure NoRowText(Ctl: Text; Typed: Text): Text
    begin
        exit('E: Validation error for Field: ' + Ctl + ',  Message = ''The NB TSV Book does not exist. Identification fields and values: Code=''' + Typed + '''''');
    end;

    // The served cells (corpus PR 1646).
    [Test]
    procedure TextVar_Exact()
    var
        TP: TestPage "NB TSV Text Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetText(TP, 'abc'), 'ok');
        Same(TP.BookCtl.Value(), 'ABC');
        Same(TP.ValidatedCtl.Value(), '1');
        TP.Close();
    end;

    [Test]
    procedure TextVar_NoRow()
    var
        TP: TestPage "NB TSV Text Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetText(TP, 'ZZZ'), NoRowText('BookCtl', 'ZZZ'));
        Same(TP.BookCtl.Value(), '');
        Same(TP.ValidatedCtl.Value(), '0');
        TP.Close();
    end;

    [Test]
    procedure CodeVar_Exact()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetCode(TP, 1, 'ABC'), 'ok');
        Same(TP.BookCtl.Value(), 'ABC');
        TP.Close();
    end;

    [Test]
    procedure CodeVar_Prefix()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetCode(TP, 1, 'mn'), 'ok');
        Same(TP.BookCtl.Value(), 'MNO');
        TP.Close();
    end;

    [Test]
    procedure CodeVar_PrefixTwoKeys()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetCode(TP, 1, 'AB'), 'ok');
        Same(TP.BookCtl.Value(), 'ABC');
        TP.Close();
    end;

    [Test]
    procedure CodeVar_NoRow()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetCode(TP, 1, 'ZZZ'), NoRowText('BookCtl', 'ZZZ'));
        Same(TP.BookCtl.Value(), '');
        TP.Close();
    end;

    [Test]
    procedure CodeVar_Blank()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetCode(TP, 1, ''), 'ok');
        Same(TP.BookCtl.Value(), '');
        TP.Close();
    end;

    [Test]
    procedure Filtered_ExactOutside()
    var
        TP: TestPage "NB TSV Filtered";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetFiltered(TP, 'XYZ'), 'ok');
        Same(TP.BookCtl.Value(), 'XYZ');
        TP.Close();
    end;

    [Test]
    procedure Filtered_PrefixOutside()
    var
        TP: TestPage "NB TSV Filtered";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetFiltered(TP, 'XY'), NoRowText('BookCtl', 'XY'));
        TP.Close();
    end;

    [Test]
    procedure NameVar_AnyValue()
    var
        TP: TestPage "NB TSV Name Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetName(TP, 'Shared'), 'E: Validation error for Field: BookCtl,  Message = ''The following field must be included into the table''s primary key: Field: Name Table: NB TSV Book''');
        Same(TP.BookCtl.Value(), '');
        TP.Close();
    end;

    [Test]
    procedure HostField_Exact()
    var
        Host: Record "NB TSV Host";
        TP: TestPage "NB TSV Host Card";
    begin
        Seed();
        Host.Get(1);
        TP.OpenEdit();
        TP.GoToRecord(Host);
        Check(SetHost(TP, false, 'ABC'), 'ok');
        TP.Close();
        Host.Get(1);
        Same(Host."Book Code", 'ABC');
    end;

    [Test]
    procedure HostField_Prefix()
    var
        Host: Record "NB TSV Host";
        TP: TestPage "NB TSV Host Card";
    begin
        Seed();
        Host.Get(1);
        TP.OpenEdit();
        TP.GoToRecord(Host);
        Check(SetHost(TP, false, 'MN'), 'ok');
        Same(TP.BookCtl.Value(), 'MNO');
        TP.Close();
        Host.Get(1);
        Same(Host."Book Code", 'MNO');
        Same(Format(Host.ValidateCount), '1');
    end;

    [Test]
    procedure HostField_NoRow()
    var
        Host: Record "NB TSV Host";
        TP: TestPage "NB TSV Host Card";
    begin
        Seed();
        Host.Get(1);
        TP.OpenEdit();
        TP.GoToRecord(Host);
        Check(SetHost(TP, false, 'ZZZ'), 'E: Validation error for Field: BookCtl,  Message = ''The field Book Code of table NB TSV Host contains a value (ZZZ) that cannot be found in the related table (NB TSV Book). (Select Refresh to discard errors)''');
        TP.Close();
    end;

    // Neighbors: captions, a held value, another field holding the text, rows written out of key order; and the shapes
    // that stay refused (not measured).
    [Test]
    procedure CodeVar_CompoundKey()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetCode(TP, 2, 'P1'), 'ok');
        TP.Close();
    end;

    [Test]
    procedure CodeVar_IntegerKey()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetCode(TP, 3, '1'), 'ok');
        TP.Close();
    end;

    [Test]
    procedure CodeVar_TableWithCaption()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetCode(TP, 4, 'ZZZ'), 'E: Validation error for Field: CaptCtl,  Message = ''The Captioned Book does not exist. Identification fields and values: Book No.=''ZZZ''''');
        TP.Close();
    end;

    [Test]
    procedure CodeVar_LongerThanVariable()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetCode(TP, 5, 'ABCD'), 'ok');
        TP.Close();
    end;

    [Test]
    procedure CodeVar_LowerNoRow()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetCode(TP, 1, 'zzz'), NoRowText('BookCtl', 'ZZZ'));
        TP.Close();
    end;

    [Test]
    procedure CodeVar_HeldValueThenNoRow()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetCode(TP, 1, 'ABC'), 'ok');
        Check(SetCode(TP, 1, 'ZZZ'), NoRowText('BookCtl', 'ZZZ'));
        Same(TP.BookCtl.Value(), 'ABC');
        TP.Close();
    end;

    [Test]
    procedure CodeVar_FilteredRelation()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetCode(TP, 7, 'MNO'), 'ok');
        TP.Close();
    end;

    [Test]
    procedure CodeVar_TextInAnotherField()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        AddBook('QRS', 'Omni', true);
        TP.OpenEdit();
        Check(SetCode(TP, 1, 'MN'), 'ok');
        Same(TP.BookCtl.Value(), 'MNO');
        TP.Close();
    end;

    [Test]
    procedure CodeVar_PrefixTwoKeysWrittenOutOfOrder()
    var
        Book: Record "NB TSV Book";
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        Book.Get('ABC');
        Book.Delete();
        AddBook('ABC', 'Shared', true);
        TP.OpenEdit();
        Check(SetCode(TP, 1, 'AB'), 'ok');
        Same(TP.BookCtl.Value(), 'ABC');
        TP.Close();
    end;

    [Test]
    procedure TextVar_LowerPrefix()
    var
        TP: TestPage "NB TSV Text Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetText(TP, 'mn'), 'ok');
        Same(TP.BookCtl.Value(), 'MNO');
        TP.Close();
    end;

    [Test]
    procedure Filtered_PrefixInside()
    var
        TP: TestPage "NB TSV Filtered";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetFiltered(TP, 'MN'), 'ok');
        Same(TP.BookCtl.Value(), 'MNO');
        TP.Close();
    end;

    [Test]
    procedure HostField_LowerPrefix()
    var
        Host: Record "NB TSV Host";
        TP: TestPage "NB TSV Host Card";
    begin
        Seed();
        Host.Get(1);
        TP.OpenEdit();
        TP.GoToRecord(Host);
        Check(SetHost(TP, false, 'mn'), 'ok');
        Same(TP.BookCtl.Value(), 'MNO');
        TP.Close();
    end;

    // Neighbors that keep main's path (the rule does not read them).
    [Test]
    procedure CodeVar_TextKeyKeepsMain()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetCode(TP, 8, 'mn'), 'ok');
        Same(TP.NamedCtl.Value(), 'MN');
        TP.Close();
    end;

    [Test]
    procedure ExtensionControl_Prefix()
    var
        TP: TestPage "NB TSV Code Var";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetCode(TP, 6, 'mn'), 'ok');
        Same(TP.ExtCtl.Value(), 'MN');
        TP.Close();
    end;

    [Test]
    procedure HostList_Prefix()
    var
        Host: Record "NB TSV Host";
        TP: TestPage "NB TSV Host List";
    begin
        Seed();
        Host.Get(1);
        TP.OpenEdit();
        TP.GoToRecord(Host);
        Check(SetHostList(TP, 'MN'), 'E: Validation error for Field: BookCtl,  Message = ''The field Book Code of table NB TSV Host contains a value (MN) that cannot be found in the related table (NB TSV Book).''');
        TP.Close();
    end;

    [Test]
    procedure HostField_FilteredExactOutside()
    var
        Host: Record "NB TSV Host";
        TP: TestPage "NB TSV Host Card";
    begin
        Seed();
        Host.Get(1);
        TP.OpenEdit();
        TP.GoToRecord(Host);
        Check(SetHost(TP, true, 'XYZ'), 'E: Validation error for Field: ActiveCtl,  Message = ''The field Active Code of table NB TSV Host contains a value (XYZ) that cannot be found in the related table (NB TSV Book). (Select Refresh to discard errors)''');
        TP.Close();
    end;

    [RequestPageHandler]
    procedure RequestHandler(var RP: TestRequestPage "NB TSV Request")
    begin
        if RequestCtl = 1 then begin
            RP.BookCtl.SetValue(RequestTyped);
            RequestReading := RP.BookCtl.Value();
        end else begin
            RP.PairCtl.SetValue(RequestTyped);
            RequestReading := RP.PairCtl.Value();
        end;
        RP.Cancel().Invoke();
    end;

    // The rule's order and the shapes the merged probe's review settled (prefix inside the filter before an exact key
    // outside it, a table field over a held value or typed in lower case, a request page's Code variable).
    [Test]
    procedure Filtered_PrefixInsideBeforeExactOutside()
    var
        TP: TestPage "NB TSV Filtered";
    begin
        Seed();
        AddBook('XYZA', 'Inside', true);
        TP.OpenEdit();
        Check(SetFiltered(TP, 'XYZ'), 'ok');
        Same(TP.BookCtl.Value(), 'XYZA');
        TP.Close();
    end;

    [Test]
    procedure Filtered_LowerExactInside()
    var
        TP: TestPage "NB TSV Filtered";
    begin
        Seed();
        TP.OpenEdit();
        Check(SetFiltered(TP, 'mno'), 'ok');
        Same(TP.BookCtl.Value(), 'MNO');
        TP.Close();
    end;

    [Test]
    procedure HostField_HeldThenPrefix()
    var
        Host: Record "NB TSV Host";
        TP: TestPage "NB TSV Host Card";
    begin
        Seed();
        Host.Get(1);
        Host."Book Code" := 'ABC';
        Host.Modify();
        TP.OpenEdit();
        TP.GoToRecord(Host);
        Check(SetHost(TP, false, 'MN'), 'ok');
        TP.Close();
        Host.Get(1);
        Same(Host."Book Code", 'MNO');
    end;

    [Test]
    [HandlerFunctions('RequestHandler')]
    procedure RequestCodeVar_Prefix()
    begin
        Seed();
        RequestCtl := 1;
        RequestTyped := 'mn';
        Commit();
        Report.RunModal(Report::"NB TSV Request", true);
        Same(RequestReading, 'MNO');
    end;

    [Test]
    [HandlerFunctions('RequestHandler')]
    procedure RequestCodeVar_CompoundKeyKeepsMain()
    begin
        Seed();
        RequestCtl := 2;
        RequestTyped := 'P1';
        Commit();
        Report.RunModal(Report::"NB TSV Request", true);
        Same(RequestReading, 'P1');
    end;
}
