codeunit 50410 "GL Entry IsEmpty Tests"
{
    Subtype = Test;

    [Test]
    procedure Plain_IsEmpty()
    var
        GLEntry: Record "G/L Entry";
    begin
        if GLEntry.IsEmpty() then
            Error('G/L Entry is empty');
    end;

    [Test]
    procedure Filtered_IsEmpty()
    var
        GLEntry: Record "G/L Entry";
    begin
        GLEntry.SetRange("Entry No.", 1);
        if GLEntry.IsEmpty() then
            Error('empty');
    end;

    [Test]
    procedure FilterGroup2_IsEmpty()
    var
        GLEntry: Record "G/L Entry";
    begin
        GLEntry.FilterGroup(2);
        GLEntry.SetRange("Entry No.", 1);
        GLEntry.FilterGroup(0);
        if GLEntry.IsEmpty() then
            Error('empty');
    end;

    [Test]
    procedure MarkedOnly_IsEmpty()
    var
        GLEntry: Record "G/L Entry";
    begin
        GLEntry.MarkedOnly(true);
        if not GLEntry.IsEmpty() then
            Error('not empty');
    end;

    [Test]
    procedure ChangeCompany_IsEmpty()
    var
        GLEntry: Record "G/L Entry";
    begin
        GLEntry.ChangeCompany(CompanyName());
        if GLEntry.IsEmpty() then
            Error('empty');
    end;

    [Test]
    procedure Count_GLEntry()
    var
        GLEntry: Record "G/L Entry";
    begin
        if GLEntry.Count() = 0 then
            Error('empty');
    end;

    [Test]
    procedure Temporary_IsEmpty()
    var
        GLEntry: Record "G/L Entry" temporary;
    begin
        if not GLEntry.IsEmpty() then
            Error('not empty');
    end;

    [Test]
    procedure IsEmpty_OfOtherDemoTable_AccountingPeriod()
    var
        AccountingPeriod: Record "Accounting Period";
    begin
        if AccountingPeriod.IsEmpty() then
            Error('empty');
    end;

    [Test]
    procedure FindFirst_GLEntry()
    var
        GLEntry: Record "G/L Entry";
    begin
        if not GLEntry.FindFirst() then
            Error('no row');
    end;
}
