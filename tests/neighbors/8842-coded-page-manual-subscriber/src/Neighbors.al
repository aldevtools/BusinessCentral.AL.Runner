// Neighbors of ListNewLineExtraCodeCodedPageSubscriber.al (al-runner issue 8842): a manual subscriber to another event of the
// probe's page, not bound, and the same subscriber bound in the handler after the open, on a copy of the probe's page over its
// table and parts. Each test holds Business Central's answer, measured on a local BC 29.1 container (3 runs of 3).

// The probe's page, copied for its own subscriber.
page 96576 "LT NU5 Lines"
{
    PageType = List;
    SourceTable = "LT NU4 Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NU4 Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NU4 Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NU4 Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NU4 Line")
    var
        Other: Record "LT NU4 Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// A manual subscriber to the copy's OnAfterGetRecordEvent: it logs each record the page reads while it is bound.
codeunit 96577 "LT NU5 Spy"
{
    SingleInstance = true;
    EventSubscriberInstance = Manual;

    var
        Entries: Text;

    [EventSubscriber(ObjectType::Page, Page::"LT NU5 Lines", 'OnAfterGetRecordEvent', '', false, false)]
    local procedure OnAfterGetLine(var Rec: Record "LT NU4 Line")
    begin
        Entries += 'AGR(' + Rec.Name + '/' + Rec.Note + '/' + Rec.Mark + '/' + Rec.Value + ');';
    end;

    procedure Log(): Text
    begin
        exit(Entries);
    end;

    procedure Reset()
    begin
        Entries := '';
    end;
}

codeunit 96578 "LT NU5 Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NU4 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NU5 Lines", Line);
    end;
}

codeunit 96579 "LT NU5 Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_UnboundSubscriberToAnotherPageEvent_ShowsAndSavesTheRow()
    var
        Opener: Codeunit "LT NU5 Opener";
        Spy: Codeunit "LT NU5 Spy";
    begin
        Initialize();
        Opener.Open('G1');
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('spy', '', Spy.Log());
    end;

    [Test]
    [HandlerFunctions('BindHandler')]
    procedure NewLine_SubscriberBoundInTheHandlerAfterTheOpen_RunsFromThere()
    var
        Opener: Codeunit "LT NU5 Opener";
        Spy: Codeunit "LT NU5 Spy";
    begin
        Initialize();
        Opener.Open('G1');
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('spy', 'AGR(A/Resolved/Stamped/);', Spy.Log());
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NU5 Lines")
    begin
        Drive(Lines);
    end;

    [ModalPageHandler]
    procedure BindHandler(var Lines: TestPage "LT NU5 Lines")
    var
        Spy: Codeunit "LT NU5 Spy";
    begin
        Lines.New();
        BindSubscription(Spy);
        Lines.Name.SetValue('A');
        Read(Lines, 'name=');
        Lines.Value.SetValue('V');
        Read(Lines, 'value=');
        Lines.OK().Invoke();
    end;

    local procedure Drive(var Lines: TestPage "LT NU5 Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Read(Lines, 'name=');
        Lines.Value.SetValue('V');
        Read(Lines, 'value=');
        Lines.OK().Invoke();
    end;

    local procedure Read(var Lines: TestPage "LT NU5 Lines"; What: Text)
    begin
        Shown += What + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
    end;

    local procedure Initialize()
    var
        Line: Record "LT NU4 Line";
        Detail: Record "LT NU4 Detail";
        Spy: Codeunit "LT NU5 Spy";
        Probe: Codeunit "LT NU4 Spy";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        Spy.Reset();
        Probe.Reset();
        Commit();
        Shown := '';
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NU4 Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
