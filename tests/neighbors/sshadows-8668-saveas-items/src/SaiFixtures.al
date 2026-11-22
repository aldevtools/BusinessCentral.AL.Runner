table 61100 "SAI A"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 61101 "SAI B"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 61102 "SAI C"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 61103 "SAI D"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 61104 "SAI Blob"
{
    TableType = Temporary;
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

report 61100 "SAI Word Two"
{
    UsageCategory = None;
    UseRequestPage = false;
    DefaultRenderingLayout = L;
    dataset
    {
        dataitem(A; "SAI A") { column(AId; Id) { } }
        dataitem(B; "SAI B") { column(BId; Id) { } }
    }
    rendering { layout(L) { Type = Word; LayoutFile = './src/SaiWordTwo.docx'; } }
}

report 61101 "SAI Word Three"
{
    UsageCategory = None;
    UseRequestPage = false;
    DefaultRenderingLayout = L;
    dataset
    {
        dataitem(A; "SAI A") { column(AId; Id) { } }
        dataitem(B; "SAI B") { column(BId; Id) { } }
        dataitem(C; "SAI C") { column(CId; Id) { } }
    }
    rendering { layout(L) { Type = Word; LayoutFile = './src/SaiWordThree.docx'; } }
}

report 61102 "SAI Word Four"
{
    UsageCategory = None;
    UseRequestPage = false;
    DefaultRenderingLayout = L;
    dataset
    {
        dataitem(A; "SAI A") { column(AId; Id) { } }
        dataitem(B; "SAI B") { column(BId; Id) { } }
        dataitem(C; "SAI C") { column(CId; Id) { } }
        dataitem(D; "SAI D") { column(DId; Id) { } }
    }
    rendering { layout(L) { Type = Word; LayoutFile = './src/SaiWordFour.docx'; } }
}

report 61103 "SAI Rdlc Two"
{
    UsageCategory = None;
    UseRequestPage = false;
    DefaultRenderingLayout = L;
    dataset
    {
        dataitem(A; "SAI A") { column(AId; Id) { } }
        dataitem(B; "SAI B") { column(BId; Id) { } }
    }
    rendering { layout(L) { Type = RDLC; LayoutFile = './src/SaiRdlcTwo.rdl'; } }
}

report 61104 "SAI Rdlc Three"
{
    UsageCategory = None;
    UseRequestPage = false;
    DefaultRenderingLayout = L;
    dataset
    {
        dataitem(A; "SAI A") { column(AId; Id) { } }
        dataitem(B; "SAI B") { column(BId; Id) { } }
        dataitem(C; "SAI C") { column(CId; Id) { } }
    }
    rendering { layout(L) { Type = RDLC; LayoutFile = './src/SaiRdlcThree.rdl'; } }
}

// The issue's report: two Integer items that stop in OnPreDataItem.
report 61105 "SAI Word Break"
{
    UsageCategory = None;
    DefaultRenderingLayout = L;
    dataset
    {
        dataitem(R; Integer)
        {
            DataItemTableView = where(Number = const(1));
            column(RNum; Number) { }
            trigger OnPreDataItem() begin CurrReport.Break(); end;
        }
        dataitem(S; Integer)
        {
            DataItemTableView = where(Number = const(1));
            column(SNum; Number) { }
            trigger OnPreDataItem() begin CurrReport.Break(); end;
        }
    }
    rendering { layout(L) { Type = Word; LayoutFile = './src/SaiWordBreak.docx'; } }
}

// Two items that skip every record in OnAfterGetRecord (no data item trigger of this kind is measured).
report 61106 "SAI Word Skip"
{
    UsageCategory = None;
    UseRequestPage = false;
    DefaultRenderingLayout = L;
    dataset
    {
        dataitem(R; Integer)
        {
            DataItemTableView = where(Number = const(1));
            column(RNum; Number) { }
            trigger OnAfterGetRecord() begin CurrReport.Skip(); end;
        }
        dataitem(S; Integer)
        {
            DataItemTableView = where(Number = const(1));
            column(SNum; Number) { }
            trigger OnAfterGetRecord() begin CurrReport.Skip(); end;
        }
    }
    rendering { layout(L) { Type = Word; LayoutFile = './src/SaiWordSkip.docx'; } }
}

// Two top-level items, one with a child item: three items, not all top-level.
report 61107 "SAI Word Nested"
{
    UsageCategory = None;
    UseRequestPage = false;
    DefaultRenderingLayout = L;
    dataset
    {
        dataitem(A; "SAI A")
        {
            column(AId; Id) { }
            dataitem(C; "SAI C") { column(CId; Id) { } }
        }
        dataitem(B; "SAI B") { column(BId; Id) { } }
    }
    rendering { layout(L) { Type = Word; LayoutFile = './src/SaiWordNested.docx'; } }
}

// Two items and OnPreReport.
report 61108 "SAI Word Trigger"
{
    UsageCategory = None;
    UseRequestPage = false;
    DefaultRenderingLayout = L;
    dataset
    {
        dataitem(A; "SAI A") { column(AId; Id) { } }
        dataitem(B; "SAI B") { column(BId; Id) { } }
    }
    rendering { layout(L) { Type = Word; LayoutFile = './src/SaiWordTrigger.docx'; } }
    trigger OnPreReport() begin end;
}

// Two items, the layout named by DefaultLayout.
report 61109 "SAI Word Legacy"
{
    UsageCategory = None;
    UseRequestPage = false;
    DefaultLayout = Word;
    WordLayout = './src/SaiWordLegacy.docx';
    dataset
    {
        dataitem(A; "SAI A") { column(AId; Id) { } }
        dataitem(B; "SAI B") { column(BId; Id) { } }
    }
}

report 61110 "SAI Word Ext Base"
{
    UsageCategory = None;
    UseRequestPage = false;
    DefaultRenderingLayout = L;
    dataset
    {
        dataitem(A; "SAI A") { column(AId; Id) { } }
        dataitem(B; "SAI B") { column(BId; Id) { } }
    }
    rendering { layout(L) { Type = Word; LayoutFile = './src/SaiWordExtBase.docx'; } }
}

reportextension 61100 "SAI Word Ext" extends "SAI Word Ext Base"
{
    dataset
    {
        add(A) { column(AId2; Id) { } }
    }
}

report 61111 "SAI Word Labels"
{
    UsageCategory = None;
    UseRequestPage = false;
    DefaultRenderingLayout = L;
    dataset
    {
        dataitem(A; "SAI A") { column(AId; Id) { } }
        dataitem(B; "SAI B") { column(BId; Id) { } }
    }
    rendering { layout(L) { Type = Word; LayoutFile = './src/SaiWordLabels.docx'; } }
    labels { TitleLbl = 'Title'; BodyLbl = 'Body'; }
}

report 61112 "SAI Word ReqPage"
{
    UsageCategory = None;
    DefaultRenderingLayout = L;
    dataset
    {
        dataitem(A; "SAI A") { column(AId; Id) { } }
        dataitem(B; "SAI B") { column(BId; Id) { } }
    }
    requestpage
    {
        layout { area(Content) { field(ShowDetails; ShowDetails) { ApplicationArea = All; } } }
    }
    rendering { layout(L) { Type = Word; LayoutFile = './src/SaiWordReqPage.docx'; } }
    var
        ShowDetails: Boolean;
}

codeunit 61100 "SAI Helper"
{
    procedure Fill(ACount: Integer; BCount: Integer; CCount: Integer)
    var
        A: Record "SAI A";
        B: Record "SAI B";
        C: Record "SAI C";
        D: Record "SAI D";
        I: Integer;
    begin
        A.DeleteAll();
        B.DeleteAll();
        C.DeleteAll();
        D.DeleteAll();
        for I := 1 to ACount do begin
            A.Id := I;
            A.Insert();
        end;
        for I := 1 to BCount do begin
            B.Id := I;
            B.Insert();
        end;
        for I := 1 to CCount do begin
            C.Id := I;
            C.Insert();
        end;
    end;

    // Saves with the answer used; returns 'ok=<answer> len=<bytes> code=<last error code>'.
    procedure SaveUsed(ReportId: Integer; Fmt: ReportFormat): Text
    var
        Blob: Record "SAI Blob";
        OutS: OutStream;
        Ok: Boolean;
    begin
        ClearLastError();
        Blob.Id := 1;
        Blob.Data.CreateOutStream(OutS);
        Ok := Report.SaveAs(ReportId, '', Fmt, OutS);
        Blob.Insert();
        Blob.CalcFields(Data);
        exit(StrSubstNo('ok=%1 len=%2 code=%3', Ok, Blob.Data.Length(), GetLastErrorCode()));
    end;

    // Saves as a statement; returns the length written.
    procedure SaveStatement(ReportId: Integer; Fmt: ReportFormat): Integer
    var
        Blob: Record "SAI Blob";
        OutS: OutStream;
    begin
        Blob.Id := 1;
        Blob.Data.CreateOutStream(OutS);
        Report.SaveAs(ReportId, '', Fmt, OutS);
        Blob.Insert();
        Blob.CalcFields(Data);
        exit(Blob.Data.Length());
    end;
}
