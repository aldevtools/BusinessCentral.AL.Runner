// Neighbors of the by-id subscribers to UI Helper Triggers CaptionClassTranslate, Reporting Triggers SubstituteReport and
// Filter Helper Triggers MakeDateFilter (corpus SbiSubscriberByIdTests.al:171, :231, :245). The pinned shapes, another raise
// site of SubstituteReport, a test that cannot see the subscriber (a reader of the mark), another event of Filter Helper
// Triggers by id, and a manual by-id subscriber while bound. A dependency app's by-id subscriber would refuse every test of
// this app that raises the event, so it is a Rust test (crates/al-vm/tests/language/platform_subscribers_by_id.rs).
table 61020 "SBN Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; D; Date) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

report 61021 "SBN Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset { dataitem(I; Integer) { DataItemTableView = where(Number = const(1)); } }
}

report 61022 "SBN Swapped"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset { dataitem(I; Integer) { DataItemTableView = where(Number = const(1)); } }
}

report 61023 "SBN Unseen"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset { dataitem(I; Integer) { DataItemTableView = where(Number = const(1)); } }
}

codeunit 61024 "SBN Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Entries;
        Entries := '';
        exit(Result);
    end;
}

codeunit 61025 "SBN Subs"
{
    [EventSubscriber(ObjectType::Codeunit, 2000000005, 'SubstituteReport', '', false, false)]
    local procedure R1_SubstituteId(ReportId: Integer; RunMode: Option Normal,ParametersOnly,Execute,Print,SaveAs,RunModal; RequestPageXml: Text; RecordRef: RecordRef; var NewReportId: Integer)
    var
        L: Codeunit "SBN Log";
    begin
        if ReportId = Report::"SBN Report" then
            L.Add('R1(' + Format(RunMode) + ')');
        if ReportId = Report::"SBN Swapped" then
            NewReportId := Report::"SBN Report";
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SubstituteReport', '', false, false)]
    local procedure R2_SubstituteName(ReportId: Integer; RunMode: Option Normal,ParametersOnly,Execute,Print,SaveAs,RunModal; RequestPageXml: Text; RecordRef: RecordRef; var NewReportId: Integer)
    var
        L: Codeunit "SBN Log";
    begin
        if ReportId = Report::"SBN Report" then
            L.Add('R2(' + Format(RunMode) + ')');
    end;

    [EventSubscriber(ObjectType::Codeunit, 2000000007, 'MakeDateFilter', '', false, false)]
    local procedure M1_DateFilterId(var DateFilterText: Text)
    var
        L: Codeunit "SBN Log";
    begin
        if DateFilterText <> 'sbnword' then
            exit;
        L.Add('M1(' + DateFilterText + ')');
        DateFilterText := '2020-12-31';
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Filter Helper Triggers", 'MakeDateFilter', '', false, false)]
    local procedure M2_DateFilterName(var DateFilterText: Text)
    var
        L: Codeunit "SBN Log";
    begin
        if (DateFilterText <> 'sbnword') and (DateFilterText <> '2020-12-31') then
            exit;
        L.Add('M2(' + DateFilterText + ')');
    end;

    [EventSubscriber(ObjectType::Codeunit, 2000000007, 'MakeIntFilter', '', false, false)]
    local procedure N1_IntFilterId(var IntFilterText: Text)
    begin
        if IntFilterText = 'sbnint' then
            IntFilterText := '5';
    end;

    [EventSubscriber(ObjectType::Codeunit, 2000000004, 'CaptionClassTranslate', '', false, false)]
    local procedure U1_TranslateId(Language: Integer; CaptionExpr: Text[1024]; var Translation: Text[1024])
    var
        L: Codeunit "SBN Log";
    begin
        if CaptionExpr.StartsWith('SBNU,') then
            L.Add('U1(' + Translation + ')');
    end;
}

codeunit 61026 "SBN Manual Subs"
{
    EventSubscriberInstance = Manual;

    [EventSubscriber(ObjectType::Codeunit, 2000000007, 'MakeDateFilter', '', false, false)]
    local procedure ManualId(var DateFilterText: Text)
    begin
        if DateFilterText = 'sbnmanual' then
            DateFilterText := '2021-01-01';
    end;
}

codeunit 61030 "SBN Tests"
{
    Subtype = Test;

    local procedure Expect(What: Text; Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('%1: expected [%2] but was [%3]', What, Expected, Actual);
    end;

    // The pinned shape (SbiSubscriberByIdTests.al:231): Report.Run, by id before by name.
    [Test]
    procedure SubstituteReport_ById_ReportRun_Logged()
    var
        L: Codeunit "SBN Log";
    begin
        L.Take();
        Report.Run(Report::"SBN Report", false, false);
        Expect('run', L.Take(), 'R1(Normal);R2(Normal);');
    end;

    // Another raise site: GetSubstituteReportId asks the same subscribers.
    [Test]
    procedure SubstituteReport_ById_GetSubstituteReportId_Swaps()
    begin
        Expect('substitute', Format(Report.GetSubstituteReportId(Report::"SBN Swapped")), Format(Report::"SBN Report"));
    end;

    // A reader of the mark: the test runs a report the subscribers leave alone and cannot see them.
    [Test]
    procedure SubstituteReport_ById_OtherReport_Unseen()
    begin
        Report.Run(Report::"SBN Unseen", false, false);
    end;

    // The pinned shape (SbiSubscriberByIdTests.al:245): the by-id subscriber repairs the text, the name form sees the repair.
    [Test]
    procedure MakeDateFilter_ById_Repairs()
    var
        Row: Record "SBN Row";
        L: Codeunit "SBN Log";
    begin
        L.Take();
        Row.SetFilter(D, 'sbnword');
        Expect('filter', Row.GetFilter(D), Format(DMY2Date(31, 12, 2020)));
        Expect('log', L.Take(), 'M1(sbnword);M2(2020-12-31);');
    end;

    // Another event of Filter Helper Triggers by id: not measured, refused on main and head.
    [Test]
    procedure MakeIntFilter_ById_OtherEvent()
    var
        Row: Record "SBN Row";
    begin
        Row.SetFilter(Id, 'sbnint');
        Expect('filter', Row.GetFilter(Id), '5');
    end;

    // A manual by-id subscriber while bound: not measured, refused on main and head.
    [Test]
    procedure MakeDateFilter_ById_ManualBound()
    var
        Row: Record "SBN Row";
        Manual: Codeunit "SBN Manual Subs";
    begin
        BindSubscription(Manual);
        Row.SetFilter(D, 'sbnmanual');
        UnbindSubscription(Manual);
        Expect('filter', Row.GetFilter(D), Format(DMY2Date(1, 1, 2021)));
    end;

    // The UI Helper shape (SbiSubscriberByIdTests.al:171) with a subscriber that leaves Translation alone.
    [Test]
    procedure CaptionClassTranslate_ById_Seen()
    var
        L: Codeunit "SBN Log";
    begin
        L.Take();
        Expect('result', CaptionClassTranslate('SBNU,x'), 'SBNU,x');
        Expect('log', L.Take(), 'U1(SBNU,x);');
    end;
}
