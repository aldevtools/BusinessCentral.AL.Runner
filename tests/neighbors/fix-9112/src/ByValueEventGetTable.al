table 62910 "GV Line"
{
    fields
    {
        field(1; "Transaction No."; Integer) { }
        field(2; "Line No."; Integer) { }
        field(3; Description; Text[100]) { }
    }
    keys { key(PK; "Transaction No.", "Line No.") { Clustered = true; } }
}

// The shape of al-runner issue 9112: a record walked with FindSet under a group-0 SetRange is raised by value through an
// integration event with two parameters; the subscriber hands it on as a Variant, and the receiver reads it by RecordRef.
codeunit 62910 "GV Copy"
{
    var
        ModeHolder: Codeunit "GV Mode";

    procedure SetMode(NewMode: Integer)
    begin
        ModeHolder.SetMode(NewMode);
    end;

    procedure ReadByRef(FromRec: Variant): Text
    var
        RecRef: RecordRef;
        Second: RecordRef;
        Rec: Record "GV Line";
        V: Variant;
        FPB: FilterPageBuilder;
        Url: Text;
    begin
        if not FromRec.IsRecord() then
            exit('');
        RecRef.GetTable(FromRec);
        case ModeHolder.GetMode() of
            0:
                exit(Format(RecRef.Field(3).Value));
            1:
                exit(RecRef.GetFilters());
            2:
                exit(Format(RecRef.Count()));
            3:
                begin
                    RecRef.FindFirst();
                    exit(Format(RecRef.Field(3).Value));
                end;
            4:
                exit(RecRef.Field(1).GetFilter());
            5:
                begin
                    RecRef.Field(2).SetRange(10000);
                    exit(Format(RecRef.Field(3).Value));
                end;
            6:
                exit(Format(RecRef.Duplicate().Field(3).Value));
            7:
                begin
                    RecRef.SetTable(Rec);
                    exit(Rec.Description);
                end;
            8:
                exit(RecRef.GetView());
            9:
                begin
                    RecRef.Reset();
                    exit(Format(RecRef.Field(3).Value));
                end;
            10:
                exit(RecRef.Name() + ':' + RecRef.Field(3).Name() + ':' + Format(RecRef.Number()));
            11:
                begin
                    V := RecRef;
                    Second.GetTable(V);
                    exit(Format(Second.Field(3).Value));
                end;
            12:
                begin
                    V := RecRef;
                    Second.GetTable(V);
                    exit(Second.GetFilters());
                end;
            13:
                begin
                    RecRef.Field(3).Value := 'Changed';
                    exit(Format(RecRef.Field(3).Value));
                end;
            14:
                exit(Format(RecRef.HasFilter()));
            15:
                exit(Format(RecRef.CurrentKeyIndex()));
            // The demo company build: Record Restriction Mgt. reads IsTemporary and the RecordId of the ref.
            16:
                begin
                    if RecRef.IsTemporary() then
                        exit('temporary');
                    exit(KeyOf(RecRef));
                end;
            17:
                exit(RecRef.Field(3).Record().Name());
            // RecordRef.Get by a RecordId on the ref that took the mark: refused like RecordRef.Get by key values.
            18:
                if RecRef.Get(RecRef.RecordId()) then
                    exit(Format(RecRef.Field(3).Value));
            // The hand-offs of the ref that take its view: each refuses with the mark.
            19:
                begin
                    Rec.CopyLinks(RecRef);
                    exit('copied');
                end;
            20:
                begin
                    FPB.AddRecordRef('Lines', RecRef);
                    exit('added');
                end;
            21:
                begin
                    Url := GetUrl(ClientType::Web, CompanyName(), ObjectType::Table, 62910, RecRef);
                    exit('url');
                end;
            22:
                begin
                    Report.Execute(62913, '<?xml version="1.0" standalone="yes"?><ReportParameters/>', RecRef);
                    exit('ran');
                end;
        end;
    end;

    local procedure KeyOf(RecordRef: RecordRef): Text
    begin
        OnKeyOf(RecordRef);
        exit(Format(RecordRef.RecordId));
    end;

    [IntegrationEvent(false, false)]
    local procedure OnKeyOf(RecordRef: RecordRef)
    begin
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"GV Copy", 'OnKeyOf', '', false, false)]
    local procedure ReadNameOnKeyOf(RecordRef: RecordRef)
    begin
        if RecordRef.Name() = '' then
            Error('no name');
    end;

    procedure HandleLines(TransactionNo: Integer): Text
    var
        GVLine: Record "GV Line";
        Result: Text;
    begin
        GVLine.SetRange("Transaction No.", TransactionNo);
        if GVLine.FindSet() then
            repeat
                OnBeforeHandleLine(GVLine, Result);
            until GVLine.Next() = 0;
        exit(Result);
    end;

    procedure HandleLinesVar(TransactionNo: Integer): Text
    var
        GVLine: Record "GV Line";
        Result: Text;
    begin
        GVLine.SetRange("Transaction No.", TransactionNo);
        if GVLine.FindSet() then
            repeat
                OnBeforeHandleLineVar(GVLine, Result);
            until GVLine.Next() = 0;
        exit(Result);
    end;

    // A by-value copy handed to a Record parameter of a procedure of a descending order: the view is not measured.
    procedure HandleLinesByProcedure(TransactionNo: Integer): Text
    var
        GVLine: Record "GV Line";
        Result: Text;
    begin
        GVLine.SetRange("Transaction No.", TransactionNo);
        GVLine.SetAscending("Line No.", false);
        if GVLine.FindSet() then
            repeat
                Result += ReadCopy(GVLine) + ';';
            until GVLine.Next() = 0;
        exit(Result);
    end;

    procedure ReadCopy(GVLine: Record "GV Line"): Text
    begin
        exit(ReadByRef(GVLine));
    end;

    // A SetLoadFields and a FindSet, then the hand-off to a procedure: the ref reads a field the row did not load.
    procedure HandleLinesLoadFields(TransactionNo: Integer): Text
    var
        GVLine: Record "GV Line";
        Result: Text;
    begin
        GVLine.SetRange("Transaction No.", TransactionNo);
        GVLine.SetLoadFields(Description);
        GVLine.SetAscending("Line No.", false);
        if GVLine.FindSet() then
            repeat
                Result += ReadLine2ByRef(GVLine) + ';';
            until GVLine.Next() = 0;
        exit(Result);
    end;

    procedure ReadLine2ByRef(GVLine: Record "GV Line"): Text
    var
        RecRef: RecordRef;
        V: Variant;
    begin
        V := GVLine;
        RecRef.GetTable(V);
        exit(Format(RecRef.Field(2).Value));
    end;

    // The one-parameter publisher of the measured cell (record/filters/ByValueRecordView.al:286-308).
    procedure RaiseOne(TransactionNo: Integer): Text
    var
        GVLine: Record "GV Line";
        Result: Text;
    begin
        GVLine.SetRange("Transaction No.", TransactionNo);
        GVLine.FindFirst();
        OnOne(GVLine, Result);
        exit(Result);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnOne(GVLine: Record "GV Line"; var Result: Text)
    begin
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"GV Copy", 'OnOne', '', false, false)]
    local procedure CopyOnOne(GVLine: Record "GV Line"; var Result: Text)
    begin
        Result := ReadByRef(GVLine);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnBeforeHandleLine(GVLine: Record "GV Line"; var Result: Text)
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnBeforeHandleLineVar(var GVLine: Record "GV Line"; var Result: Text)
    begin
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"GV Copy", 'OnBeforeHandleLine', '', false, false)]
    local procedure CopyOnHandleLine(GVLine: Record "GV Line"; var Result: Text)
    var
        RecRef: RecordRef;
    begin
        case ModeHolder.GetMode() of
            100:
                Result += GVLine.Description + ';';
            101:
                begin
                    RecRef.GetTable(GVLine);
                    Result += Format(RecRef.Field(3).Value) + ';';
                end;
            else
                Result += ReadByRef(GVLine) + ';';
        end;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"GV Copy", 'OnBeforeHandleLineVar', '', false, false)]
    local procedure CopyOnHandleLineVar(var GVLine: Record "GV Line"; var Result: Text)
    begin
        Result += ReadByRef(GVLine) + ';';
    end;
}

report 62913 "GV Report"
{
    ProcessingOnly = true;
    dataset { dataitem(L; "GV Line") { } }
}

// Holds the mode across the instances of "GV Copy" (a subscriber runs on its own instance).
codeunit 62912 "GV Mode"
{
    SingleInstance = true;

    var
        Mode: Integer;

    procedure SetMode(NewMode: Integer)
    begin
        Mode := NewMode;
    end;

    procedure GetMode(): Integer
    begin
        exit(Mode);
    end;
}

codeunit 62911 "GV GetTable Test"
{
    Subtype = Test;

    local procedure Setup()
    var
        GVLine: Record "GV Line";
    begin
        GVLine.DeleteAll();
        GVLine."Transaction No." := 1;
        GVLine."Line No." := 10000;
        GVLine.Description := 'First';
        GVLine.Insert();
        GVLine."Line No." := 20000;
        GVLine.Description := 'Second';
        GVLine.Insert();
    end;

    local procedure Run(Mode: Integer): Text
    var
        GVCopy: Codeunit "GV Copy";
    begin
        Setup();
        GVCopy.SetMode(Mode);
        exit(GVCopy.HandleLines(1));
    end;

    local procedure Check(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure Issue9112_VariantGetTable_FieldValue()
    begin
        Check('First;Second;', Run(0));
    end;

    [Test]
    procedure DirectGetTable_FieldValue()
    begin
        Check('First;Second;', Run(101));
    end;

    [Test]
    procedure RecordFieldRead_InSubscriber()
    begin
        Check('First;Second;', Run(100));
    end;

    [Test]
    procedure VariantGetTable_GetFilters()
    begin
        Run(1);
    end;

    [Test]
    procedure VariantGetTable_Count()
    begin
        Run(2);
    end;

    [Test]
    procedure VariantGetTable_FindFirst()
    begin
        Run(3);
    end;

    [Test]
    procedure VariantGetTable_FieldRefGetFilter()
    begin
        Run(4);
    end;

    [Test]
    procedure VariantGetTable_FieldRefSetRange()
    begin
        Run(5);
    end;

    [Test]
    procedure VariantGetTable_Duplicate()
    begin
        Run(6);
    end;

    [Test]
    procedure VariantGetTable_SetTable()
    begin
        Run(7);
    end;

    [Test]
    procedure VariantGetTable_GetView()
    begin
        Run(8);
    end;

    [Test]
    procedure VariantGetTable_Reset()
    begin
        Run(9);
    end;

    [Test]
    procedure VariantGetTable_Metadata()
    begin
        Check('GV Line:Description:62910;GV Line:Description:62910;', Run(10));
    end;

    [Test]
    procedure VariantGetTable_IntoSecondRef_FieldValue()
    begin
        Check('First;Second;', Run(11));
    end;

    [Test]
    procedure VariantGetTable_IntoSecondRef_GetFilters()
    begin
        Run(12);
    end;

    [Test]
    procedure VariantGetTable_FieldValueWrite()
    begin
        Check('Changed;Changed;', Run(13));
    end;

    [Test]
    procedure VariantGetTable_HasFilter()
    begin
        Run(14);
    end;

    [Test]
    procedure VariantGetTable_CurrentKeyIndex()
    begin
        Run(15);
    end;

    [Test]
    procedure VariantGetTable_IsTemporaryAndRecordId()
    begin
        Check('GV Line: 1,10000;GV Line: 1,20000;', Run(16));
    end;

    [Test]
    procedure VariantGetTable_FieldRefRecord()
    begin
        Run(17);
    end;

    [Test]
    procedure VariantGetTable_GetByRecordId()
    begin
        Run(18);
    end;

    [Test]
    procedure VarPublisherParam_VariantGetTable_FieldValue()
    var
        GVCopy: Codeunit "GV Copy";
    begin
        Setup();
        GVCopy.SetMode(0);
        Check('First;Second;', GVCopy.HandleLinesVar(1));
    end;

    [Test]
    procedure ProcedureCopy_Descending_VariantGetTable_FieldValue()
    var
        GVCopy: Codeunit "GV Copy";
    begin
        Setup();
        GVCopy.SetMode(0);
        Check('Second;First;', GVCopy.HandleLinesByProcedure(1));
    end;

    [Test]
    procedure ProcedureCopy_Descending_GetFilters()
    var
        GVCopy: Codeunit "GV Copy";
    begin
        Setup();
        GVCopy.SetMode(1);
        GVCopy.HandleLinesByProcedure(1);
    end;

    [Test]
    procedure LoadFieldsCopy_NotLoadedField_FieldRefValue()
    var
        GVCopy: Codeunit "GV Copy";
    begin
        Setup();
        Check('20000;10000;', GVCopy.HandleLinesLoadFields(1));
    end;

    [Test]
    procedure OneParameterPublisher_VariantGetTable_FieldValue()
    var
        GVCopy: Codeunit "GV Copy";
    begin
        Setup();
        GVCopy.SetMode(0);
        Check('First', GVCopy.RaiseOne(1));
    end;

    [Test]
    procedure OneParameterPublisher_GetFilters()
    var
        GVCopy: Codeunit "GV Copy";
    begin
        Setup();
        GVCopy.SetMode(1);
        GVCopy.RaiseOne(1);
    end;

    [Test]
    procedure MarkedRef_CopyLinks()
    begin
        Run(19);
    end;

    [Test]
    procedure MarkedRef_FilterPageBuilderAddRecordRef()
    begin
        Run(20);
    end;

    [Test]
    procedure MarkedRef_GetUrl()
    begin
        Run(21);
    end;

    [Test]
    procedure MarkedRef_ReportExecuteWithParameters()
    begin
        Run(22);
    end;
}
