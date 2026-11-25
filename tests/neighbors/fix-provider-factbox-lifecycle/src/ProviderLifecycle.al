table 69970 "PN Header"
{
    fields { field(1; "No."; Code[10]) { } }
    keys { key(PK; "No.") { } }
}
table 69971 "PN Line"
{
    fields { field(1; "Doc No."; Code[10]) { } field(2; "Line No."; Integer) { } }
    keys { key(PK; "Doc No.", "Line No.") { } }
}
table 69972 "PN Detail"
{
    fields { field(1; "Line Ref"; Integer) { } field(2; Note; Text[30]) { } }
    keys { key(PK; "Line Ref") { } }
}
codeunit 69973 "PN Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Value: Text) begin Entries += Value + '|'; end;
    procedure Take(): Text var Result: Text; begin Result := Entries; Clear(Entries); exit(Result); end;
}
page 69974 "PN Lines"
{
    PageType = ListPart;
    SourceTable = "PN Line";
    layout { area(Content) { repeater(R) { field(LineNo; Rec."Line No.") { } } } }
    trigger OnAfterGetCurrRecord()
    var Log: Codeunit "PN Log";
    begin
        Log.Add('line=' + Format(Rec."Line No."));
    end;
}
page 69975 "PN Details"
{
    PageType = CardPart;
    SourceTable = "PN Detail";
    layout { area(Content) { field(Note; Rec.Note) { } } }
    trigger OnAfterGetCurrRecord()
    var Log: Codeunit "PN Log";
    begin
        Log.Add('detail=' + Rec.Note);
    end;
}
page 69976 "PN Host"
{
    PageType = Card;
    SourceTable = "PN Header";
    layout
    {
        area(Content) { field("No."; Rec."No.") { } }
        area(FactBoxes)
        {
            part(Lines; "PN Lines") { SubPageLink = "Doc No." = field("No."); }
            part(Detail; "PN Details") { Provider = Lines; SubPageLink = "Line Ref" = field("Line No."); }
        }
    }
}
codeunit 69977 "PN Lifecycle Tests"
{
    Subtype = Test;

    [Test]
    procedure StoredProviderRow()
    var P: TestPage "PN Host";
    begin
        Seed();
        P.OpenView();
        P.GoToKey('H1');
        if P.Detail.Note.Value <> 'ten' then Error('Stored detail was not selected.');
        P.Close();
    end;

    [Test]
    procedure EmptyProviderBlankKeyAndRecovery()
    var P: TestPage "PN Host";
    begin
        Seed();
        P.OpenView();
        P.GoToKey('H2');
        if P.Detail.Note.Value <> '' then Error('Empty Provider selected a blank-key detail.');
        if P.Detail.First() then Error('Empty Provider exposed a stored detail.');
        P.GoToKey('H1');
        if P.Detail.Note.Value <> 'ten' then Error('Provider did not recover its stored detail.');
        P.Close();
    end;

    [Test]
    procedure NewBaseFactBoxReadsAndInsert()
    var P: TestPage "PN Host"; Header: Record "PN Header"; Log: Codeunit "PN Log"; Trace: Text;
    begin
        Seed();
        P.OpenNew();
        Trace := P.Detail.Note.Value + ':' + Log.Take();
        P."No.".SetValue('H3');
        Trace += ':' + Log.Take();
        P.Close();
        if Trace <> ':line=0|detail=|:line=0|' then Error('Unexpected trigger trace: %1', Trace);
        if not Header.Get('H3') then Error('The typed host was not inserted.');
    end;

    local procedure Seed()
    var Header: Record "PN Header"; Line: Record "PN Line"; Detail: Record "PN Detail"; Log: Codeunit "PN Log";
    begin
        Detail.DeleteAll();
        Line.DeleteAll();
        Header.DeleteAll();
        Header."No." := 'H1'; Header.Insert();
        Header."No." := 'H2'; Header.Insert();
        Line."Doc No." := 'H1'; Line."Line No." := 10; Line.Insert();
        Detail."Line Ref" := 10; Detail.Note := 'ten'; Detail.Insert();
        Detail."Line Ref" := 0; Detail.Note := 'wrong blank match'; Detail.Insert();
        Log.Take();
    end;
}
