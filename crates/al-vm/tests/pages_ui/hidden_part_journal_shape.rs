//! A part that a Visible expression hid, turned visible by an action on a Worksheet or a List host opened with OpenView (al-runner
//! issue 7922, the journal shape of Base Application's workflow status FactBox): the objects of open corpus PR 1367
//! testpage/parts/HiddenPartJournalShape.al (codeunit 75320), driven step by step. The logs are what that file expects per BC
//! major: on BC 29 the temporary part reads its rows a third time after the action and runs OnAfterGetCurrRecord on its empty
//! rows after Next; the stored part gives the same log on every major.

use al_builtins::assumptions::{Assumption, take_hits};

const SRC: &str = r#"

table 75310 "PG6 HJ Doc"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[10]) { DataClassification = SystemMetadata; }
        field(2; Name; Text[30]) { DataClassification = SystemMetadata; }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 75311 "PG6 HJ Request"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { DataClassification = SystemMetadata; }
        field(2; "Doc No."; Code[10]) { DataClassification = SystemMetadata; }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

table 75312 "PG6 HJ Status"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { DataClassification = SystemMetadata; }
        field(2; "Doc No."; Code[10]) { DataClassification = SystemMetadata; }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

// A temporary ListPart that a procedure refills from the stored requests of one document and answers whether it has rows.
page 75313 "PG6 HJ Status Part"
{
    PageType = ListPart;
    SourceTable = "PG6 HJ Status";
    SourceTableTemporary = true;
    Editable = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Entry No."; Rec."Entry No.") { ApplicationArea = All; }
                field("Doc No."; Rec."Doc No.") { ApplicationArea = All; }
            }
        }
    }

    var
        Log: Codeunit "PG6 HJ Log";

    trigger OnOpenPage()
    begin
        Log.Add('st-open');
    end;

    trigger OnAfterGetRecord()
    begin
        Log.Add('st-agr(' + Format(Rec."Entry No.") + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('st-agcr(' + Format(Rec."Entry No.") + ')');
    end;

    trigger OnClosePage()
    begin
        Log.Add('st-close');
    end;

    procedure Fill(DocNo: Code[10]): Boolean
    var
        Request: Record "PG6 HJ Request";
    begin
        Rec.DeleteAll();
        Request.SetRange("Doc No.", DocNo);
        if Request.FindSet() then
            repeat
                Rec.Init();
                Rec."Entry No." := Request."Entry No.";
                Rec."Doc No." := Request."Doc No.";
                Rec.Insert();
            until Request.Next() = 0;
        Log.Add('st-fill(' + DocNo + ',' + Format(Rec.Count()) + ')');
        exit(not Rec.IsEmpty());
    end;
}

// The same over the stored requests: the procedure filters instead of filling.
page 75314 "PG6 HJ Stored Part"
{
    PageType = ListPart;
    SourceTable = "PG6 HJ Request";
    Editable = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Entry No."; Rec."Entry No.") { ApplicationArea = All; }
                field("Doc No."; Rec."Doc No.") { ApplicationArea = All; }
            }
        }
    }

    var
        Log: Codeunit "PG6 HJ Log";

    trigger OnOpenPage()
    begin
        Log.Add('st-open');
    end;

    trigger OnAfterGetRecord()
    begin
        Log.Add('st-agr(' + Format(Rec."Entry No.") + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('st-agcr(' + Format(Rec."Entry No.") + ')');
    end;

    trigger OnClosePage()
    begin
        Log.Add('st-close');
    end;

    procedure Fill(DocNo: Code[10]): Boolean
    begin
        Rec.SetRange("Doc No.", DocNo);
        Log.Add('st-fill(' + DocNo + ',' + Format(Rec.Count()) + ')');
        exit(not Rec.IsEmpty());
    end;
}

page 75315 "PG6 HJ Worksheet"
{
    PageType = Worksheet;
    SourceTable = "PG6 HJ Doc";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Status; "PG6 HJ Status Part")
            {
                ApplicationArea = All;
                Editable = false;
                Enabled = false;
                ShowFilter = false;
                Visible = ShowStatus;
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(SendVal)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Send: Codeunit "PG6 HJ Send";
                begin
                    Log.Add('act-send');
                    Send.SendByValue(Rec);
                    RefreshStatus();
                end;
            }
            action(SendVar)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Send: Codeunit "PG6 HJ Send";
                begin
                    Log.Add('act-send');
                    Send.SendByVar(Rec);
                    RefreshStatus();
                end;
            }
        }
    }

    var
        Log: Codeunit "PG6 HJ Log";
        ShowStatus: Boolean;

    local procedure RefreshStatus()
    begin
        ShowStatus := CurrPage.Status.Page.Fill(Rec."No.");
    end;

    trigger OnOpenPage()
    begin
        Log.Add('host-open');
        RefreshStatus();
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('host-agcr(' + Rec."No." + ')');
        RefreshStatus();
    end;

    trigger OnClosePage()
    begin
        Log.Add('host-close');
    end;
}

page 75316 "PG6 HJ List"
{
    PageType = List;
    SourceTable = "PG6 HJ Doc";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Status; "PG6 HJ Status Part")
            {
                ApplicationArea = All;
                Editable = false;
                Enabled = false;
                ShowFilter = false;
                Visible = ShowStatus;
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(SendVal)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Send: Codeunit "PG6 HJ Send";
                begin
                    Log.Add('act-send');
                    Send.SendByValue(Rec);
                    RefreshStatus();
                end;
            }
            action(SendVar)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Send: Codeunit "PG6 HJ Send";
                begin
                    Log.Add('act-send');
                    Send.SendByVar(Rec);
                    RefreshStatus();
                end;
            }
        }
    }

    var
        Log: Codeunit "PG6 HJ Log";
        ShowStatus: Boolean;

    local procedure RefreshStatus()
    begin
        ShowStatus := CurrPage.Status.Page.Fill(Rec."No.");
    end;

    trigger OnOpenPage()
    begin
        Log.Add('host-open');
        RefreshStatus();
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('host-agcr(' + Rec."No." + ')');
        RefreshStatus();
    end;

    trigger OnClosePage()
    begin
        Log.Add('host-close');
    end;
}

page 75317 "PG6 HJ Worksheet Stored"
{
    PageType = Worksheet;
    SourceTable = "PG6 HJ Doc";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Status; "PG6 HJ Stored Part")
            {
                ApplicationArea = All;
                Editable = false;
                Enabled = false;
                ShowFilter = false;
                Visible = ShowStatus;
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(SendVal)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Send: Codeunit "PG6 HJ Send";
                begin
                    Log.Add('act-send');
                    Send.SendByValue(Rec);
                    RefreshStatus();
                end;
            }
        }
    }

    var
        Log: Codeunit "PG6 HJ Log";
        ShowStatus: Boolean;

    local procedure RefreshStatus()
    begin
        ShowStatus := CurrPage.Status.Page.Fill(Rec."No.");
    end;

    trigger OnOpenPage()
    begin
        Log.Add('host-open');
        RefreshStatus();
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('host-agcr(' + Rec."No." + ')');
        RefreshStatus();
    end;

    trigger OnClosePage()
    begin
        Log.Add('host-close');
    end;
}

codeunit 75318 "PG6 HJ Send"
{
    procedure SendByValue(Doc: Record "PG6 HJ Doc")
    begin
        AddRequest(Doc."No.");
    end;

    procedure SendByVar(var Doc: Record "PG6 HJ Doc")
    begin
        AddRequest(Doc."No.");
    end;

    // Not in HiddenPartJournalShape.al: for the neighbors of the Worksheet with more actions.
    procedure SendByValueAndCommit(Doc: Record "PG6 HJ Doc")
    begin
        AddRequest(Doc."No.");
        Commit();
    end;

    procedure DeleteRequests()
    var
        Request: Record "PG6 HJ Request";
    begin
        Request.DeleteAll();
    end;

    // Not in HiddenPartJournalShape.al: the send also changes the stored row the host stands on, inserts a host row, or
    // inserts a row of a third table.
    procedure SendAndModifyDoc(Doc: Record "PG6 HJ Doc")
    var
        Stored: Record "PG6 HJ Doc";
    begin
        AddRequest(Doc."No.");
        Stored.Get(Doc."No.");
        Stored.Name := 'X';
        Stored.Modify();
    end;

    procedure SendAndInsertDoc(Doc: Record "PG6 HJ Doc")
    var
        Stored: Record "PG6 HJ Doc";
    begin
        AddRequest(Doc."No.");
        Stored.Init();
        Stored."No." := Doc."No." + 'A';
        Stored.Insert();
    end;

    procedure SendAndInsertStatus(Doc: Record "PG6 HJ Doc")
    var
        Status: Record "PG6 HJ Status";
    begin
        AddRequest(Doc."No.");
        Status.Init();
        Status."Entry No." := 1;
        Status.Insert();
    end;

    local procedure AddRequest(DocNo: Code[10])
    var
        Request: Record "PG6 HJ Request";
        Log: Codeunit "PG6 HJ Log";
        NextNo: Integer;
    begin
        if Request.FindLast() then
            NextNo := Request."Entry No.";
        Request.Init();
        Request."Entry No." := NextNo + 1;
        Request."Doc No." := DocNo;
        Request.Insert();
        Log.Add('cu-send(' + DocNo + ')');
    end;
}

codeunit 75319 "PG6 HJ Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Reset()
    begin
        Entries := '';
    end;

    procedure Add(Line: Text)
    begin
        if Entries <> '' then
            Entries += ' ';
        Entries += Line;
    end;

    procedure Take() Result: Text
    begin
        Result := '[' + Entries + ']';
        Entries := '';
    end;
}

// Neighbors the probe did not measure (not in HiddenPartJournalShape.al): the Worksheet with more actions.
page 75322 "PG6 HJ Worksheet More"
{
    PageType = Worksheet;
    SourceTable = "PG6 HJ Doc";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Status; "PG6 HJ Status Part")
            {
                ApplicationArea = All;
                Editable = false;
                Enabled = false;
                ShowFilter = false;
                Visible = ShowStatus;
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(SendVal)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Send: Codeunit "PG6 HJ Send";
                begin
                    Log.Add('act-send');
                    Send.SendByValue(Rec);
                    RefreshStatus();
                end;
            }
            action(SendTwice)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Send: Codeunit "PG6 HJ Send";
                begin
                    Send.SendByValue(Rec);
                    Send.SendByValue(Rec);
                    RefreshStatus();
                end;
            }
            action(ShowOnly)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Log.Add('act-show');
                end;
            }
            action(Noop)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Log.Add('act-noop');
                end;
            }
            action(PartCall)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    RefreshStatus();
                end;
            }
            action(SendModHost)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Send: Codeunit "PG6 HJ Send";
                begin
                    Send.SendAndModifyDoc(Rec);
                    RefreshStatus();
                end;
            }
            action(SendInsHost)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Send: Codeunit "PG6 HJ Send";
                begin
                    Send.SendAndInsertDoc(Rec);
                    RefreshStatus();
                end;
            }
            action(SendInsStatus)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Send: Codeunit "PG6 HJ Send";
                begin
                    Send.SendAndInsertStatus(Rec);
                    RefreshStatus();
                end;
            }
            action(SendCommit)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Send: Codeunit "PG6 HJ Send";
                begin
                    Send.SendByValueAndCommit(Rec);
                    RefreshStatus();
                end;
            }
            action(SendThenDelete)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Send: Codeunit "PG6 HJ Send";
                begin
                    Send.SendByValue(Rec);
                    RefreshStatus();
                    Send.DeleteRequests();
                end;
            }
            action(SendVar)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Send: Codeunit "PG6 HJ Send";
                begin
                    Log.Add('act-send');
                    Send.SendByVar(Rec);
                    RefreshStatus();
                end;
            }
        }
    }

    var
        Log: Codeunit "PG6 HJ Log";
        ShowStatus: Boolean;

    local procedure RefreshStatus()
    begin
        ShowStatus := CurrPage.Status.Page.Fill(Rec."No.");
    end;

    trigger OnOpenPage()
    begin
        Log.Add('host-open');
        RefreshStatus();
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('host-agcr(' + Rec."No." + ')');
        RefreshStatus();
    end;

    trigger OnClosePage()
    begin
        Log.Add('host-close');
    end;
}

codeunit 75321 "PG6 HJ Driver"
{
    var
        Log: Codeunit "PG6 HJ Log";
        Seen: Text;

    local procedure Initialize()
    var
        Doc: Record "PG6 HJ Doc";
        Request: Record "PG6 HJ Request";
    begin
        Request.DeleteAll();
        Doc.DeleteAll();
        Doc.Init();
        Doc."No." := 'H1';
        Doc.Insert();
        Doc.Init();
        Doc."No." := 'H2';
        Doc.Insert();
        Log.Reset();
        Seen := '';
    end;

    local procedure Say(Name: Text; PartVisible: Boolean)
    var
        Request: Record "PG6 HJ Request";
    begin
        Seen += Name + '=' + Format(Request.Count()) + ' ' + Format(PartVisible) + ' ' + Log.Take() + '|';
    end;

    local procedure Say(Name: Text)
    var
        Request: Record "PG6 HJ Request";
    begin
        Seen += Name + '=' + Format(Request.Count()) + ' ' + Log.Take();
    end;

    procedure WorksheetSendVal(): Text
    var
        Doc: Record "PG6 HJ Doc";
        P: TestPage "PG6 HJ Worksheet";
    begin
        Initialize();
        P.OpenView();
        Say('open', P.Status.Visible());
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Say('gotorecord', P.Status.Visible());
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.Next();
        Say('next', P.Status.Visible());
        P.Close();
        Say('closed');
        exit(Seen);
    end;

    procedure WorksheetSendVar(): Text
    var
        Doc: Record "PG6 HJ Doc";
        P: TestPage "PG6 HJ Worksheet";
    begin
        Initialize();
        P.OpenView();
        Say('open', P.Status.Visible());
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Say('gotorecord', P.Status.Visible());
        P.SendVar.Invoke();
        Say('send', P.Status.Visible());
        P.Next();
        Say('next', P.Status.Visible());
        P.Close();
        Say('closed');
        exit(Seen);
    end;

    procedure ListSendVal(): Text
    var
        Doc: Record "PG6 HJ Doc";
        P: TestPage "PG6 HJ List";
    begin
        Initialize();
        P.OpenView();
        Say('open', P.Status.Visible());
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Say('gotorecord', P.Status.Visible());
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.Next();
        Say('next', P.Status.Visible());
        P.Close();
        Say('closed');
        exit(Seen);
    end;

    procedure ListSendVar(): Text
    var
        Doc: Record "PG6 HJ Doc";
        P: TestPage "PG6 HJ List";
    begin
        Initialize();
        P.OpenView();
        Say('open', P.Status.Visible());
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Say('gotorecord', P.Status.Visible());
        P.SendVar.Invoke();
        Say('send', P.Status.Visible());
        P.Next();
        Say('next', P.Status.Visible());
        P.Close();
        Say('closed');
        exit(Seen);
    end;

    procedure WorksheetRequestBeforeOpen(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet";
    begin
        Initialize();
        Doc.Get('H1');
        Send.SendByValue(Doc);
        Log.Take();
        P.OpenView();
        Say('open', P.Status.Visible());
        P.GoToRecord(Doc);
        Say('gotorecord', P.Status.Visible());
        P.Next();
        Say('next', P.Status.Visible());
        P.Previous();
        Say('previous', P.Status.Visible());
        P.Close();
        Say('closed');
        exit(Seen);
    end;

    // Neighbors the probe did not measure (not in HiddenPartJournalShape.al).

    procedure WorksheetOpenEditSendVal(): Text
    var
        Doc: Record "PG6 HJ Doc";
        P: TestPage "PG6 HJ Worksheet";
    begin
        Initialize();
        P.OpenEdit();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        P.SendVal.Invoke();
        exit(Log.Take());
    end;

    procedure BothRequestsBeforeOpenNext(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet";
    begin
        Initialize();
        Doc.Get('H1');
        Send.SendByValue(Doc);
        Doc.Get('H2');
        Send.SendByValue(Doc);
        P.OpenView();
        P.Next();
        exit(Log.Take());
    end;

    procedure WorksheetGoToKeyStays(): Text
    var
        P: TestPage "PG6 HJ Worksheet";
    begin
        Initialize();
        P.OpenView();
        Log.Take();
        P.GoToKey('H1');
        exit(Log.Take());
    end;

    procedure WorksheetStoredSendVal(): Text
    var
        Doc: Record "PG6 HJ Doc";
        P: TestPage "PG6 HJ Worksheet Stored";
    begin
        Initialize();
        P.OpenView();
        Say('open', P.Status.Visible());
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Say('gotorecord', P.Status.Visible());
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.Next();
        Say('next', P.Status.Visible());
        P.Close();
        Say('closed');
        exit(Seen);
    end;

    procedure NextHiddenToHidden(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet";
    begin
        Initialize();
        P.OpenView();
        Log.Take();
        P.Next();
        Say('next', P.Status.Visible());
        exit(Seen);
    end;

    procedure LastThenPrevious(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet";
    begin
        Initialize();
        Doc.Get('H1');
        Send.SendByValue(Doc);
        Doc.Get('H2');
        Send.SendByValue(Doc);
        P.OpenView();
        P.Last();
        P.Previous();
        exit(Seen);
    end;

    procedure SecondGoToRecordStays(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.GoToRecord(Doc);
        Say('gotorecord2', P.Status.Visible());
        exit(Seen);
    end;

    procedure StoredRequestBeforeOpen(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet Stored";
    begin
        Initialize();
        Doc.Get('H1');
        Send.SendByValue(Doc);
        P.OpenView();
        Log.Take();
        P.GoToRecord(Doc);
        Say('gotorecord', P.Status.Visible());
        P.Next();
        Say('next', P.Status.Visible());
        exit(Seen);
    end;

    procedure StoredRequestBeforeOpenPrevious(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet Stored";
    begin
        Initialize();
        Doc.Get('H1');
        Send.SendByValue(Doc);
        P.OpenView();
        P.Next();
        P.Previous();
        exit(Seen);
    end;

    procedure TurnWithoutGoToRecord(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet";
    begin
        Initialize();
        P.OpenView();
        P.SendVal.Invoke();
        exit(Seen);
    end;

    procedure TurnAfterTwoGoToRecords(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        P.GoToRecord(Doc);
        P.SendVal.Invoke();
        exit(Seen);
    end;

    procedure TurnTwoRows(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        P.SendTwice.Invoke();
        exit(Seen);
    end;

    procedure TurnZeroRows(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        P.ShowOnly.Invoke();
        exit(Seen);
    end;

    procedure TurnWithCommit(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        P.SendCommit.Invoke();
        exit(Seen);
    end;

    procedure TurnCuModifiesHostRow(): Text
    var
        Doc: Record "PG6 HJ Doc";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        P.SendModHost.Invoke();
        exit(Seen);
    end;

    procedure TurnCuInsertsHostRow(): Text
    var
        Doc: Record "PG6 HJ Doc";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        P.SendInsHost.Invoke();
        exit(Seen);
    end;

    procedure TurnCuInsertsThirdTable(): Text
    var
        Doc: Record "PG6 HJ Doc";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        P.SendInsStatus.Invoke();
        exit(Seen);
    end;

    procedure TurnThenRefreshHides(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        P.SendThenDelete.Invoke();
        exit(Seen);
    end;

    procedure TurnOnLastRowThenNext(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet";
    begin
        Initialize();
        Doc.Get('H2');
        Doc.Delete();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        P.SendVal.Invoke();
        P.Next();
        exit(Seen);
    end;

    procedure TurnThenVisibleNextVisibleClose(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.Next();
        Say('next', P.Status.Visible());
        P.Close();
        Say('closed');
        exit(Seen);
    end;

    procedure AfterTurnSendAgain(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.SendVal.Invoke();
        exit(Seen);
    end;

    procedure AfterTurnNoop(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.Noop.Invoke();
        exit(Seen);
    end;

    procedure AfterTurnPartCall(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.PartCall.Invoke();
        exit(Seen);
    end;

    procedure AfterTurnGoToKey(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.GoToKey('H1');
        exit(Seen);
    end;

    procedure AfterTurnGoToRecord(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.GoToRecord(Doc);
        exit(Seen);
    end;

    procedure AfterTurnFirst(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.First();
        exit(Seen);
    end;

    procedure AfterTurnLast(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.Last();
        exit(Seen);
    end;

    procedure AfterTurnPrevious(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.Previous();
        exit(Seen);
    end;

    procedure AfterTurnHostRead(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        Seen += P.Name.Value();
        exit(Seen);
    end;

    procedure AfterTurnPartRead(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        Seen += P.Status."Doc No.".Value();
        exit(Seen);
    end;

    procedure AfterTurnPartFirst(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.Status.First();
        exit(Seen);
    end;

    procedure AfterTurnPartNext(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.Status.Next();
        exit(Seen);
    end;

    procedure AfterTurnNextThenNext(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.Next();
        P.Next();
        exit(Seen);
    end;

    procedure AfterTurnNextThenPrevious(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.Next();
        P.Previous();
        exit(Seen);
    end;

    procedure AfterTurnNextThenSendAgain(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet More";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.SendVal.Invoke();
        Say('send', P.Status.Visible());
        P.Next();
        P.SendVal.Invoke();
        exit(Seen);
    end;

    // Neighbors of the review of al-runner PR 7939, round 2 (shapes W05, W04, W22).

    procedure ListRequestBeforeOpen(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ List";
    begin
        Initialize();
        Doc.Get('H1');
        Send.SendByValue(Doc);
        Log.Take();
        P.OpenView();
        Say('open', P.Status.Visible());
        P.GoToRecord(Doc);
        Say('gotorecord', P.Status.Visible());
        exit(Seen);
    end;

    procedure ListRequestBeforeOpenNext(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ List";
    begin
        Initialize();
        Doc.Get('H1');
        Send.SendByValue(Doc);
        P.OpenView();
        P.GoToRecord(Doc);
        P.Next();
        exit(Seen);
    end;

    procedure NeverShownThenPrevious(): Text
    var
        Doc: Record "PG6 HJ Doc";
        Send: Codeunit "PG6 HJ Send";
        P: TestPage "PG6 HJ Worksheet";
    begin
        Initialize();
        P.OpenView();
        Doc.Get('H1');
        P.GoToRecord(Doc);
        Log.Take();
        P.Next();
        Say('next', P.Status.Visible());
        Send.SendByValue(Doc);
        Log.Take();
        P.Previous();
        exit(Seen);
    end;

    procedure GoToRecordMovesThenTurn(): Text
    var
        Doc: Record "PG6 HJ Doc";
        P: TestPage "PG6 HJ Worksheet";
    begin
        Initialize();
        Doc.Init();
        Doc."No." := 'H3';
        Doc.Insert();
        P.OpenView();
        Doc.Get('H2');
        P.GoToRecord(Doc);
        Say('gotorecord', P.Status.Visible());
        P.SendVal.Invoke();
        exit(Seen);
    end;
}
"#;

/// Runs `routine` of the driver on a BC `bc` target: its text and the assumptions it marked.
fn run_on(bc: &str, routine: &str) -> (Result<String, String>, Vec<Assumption>) {
    run_src(SRC, bc, routine)
}

/// [`run_on`] over the objects `src`.
fn run_src(src: &str, bc: &str, routine: &str) -> (Result<String, String>, Vec<Assumption>) {
    take_hits();
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
    let apps = vec![al_sema::workspace::AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), src.into())],
    }];
    let (ws, l) = al_sema::lower::compile_for(apps, &Default::default(), &target);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "PG6 HJ Driver")
        .unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    let engine = al_vm::Engine::with_target(l.program, blocked, Some(target));
    let mut s = al_vm::Session::new(engine);
    let inst = s.instance(oi);
    let out = s
        .call(oi, ri, inst, vec![])
        .map(|(v, _)| al_testkit::text(Ok(v)))
        .map_err(|e| format!("{e:?}"));
    (out, take_hits())
}

const OPEN: &str =
    "open=0 No [host-open st-fill(,0) st-open host-agcr(H1) st-fill(H1,0) st-agcr(0)]|gotorecord=0 No []|";
const SEND: &str = "send=1 Yes [act-send cu-send(H1) st-fill(H1,1) st-agr(1) st-agr(1) st-agcr(1) host-agcr(H1) st-fill(H1,1) st-agr(1) st-agr(1) st-agcr(1)";
const THIRD_READ: &str = " st-agr(1) st-agr(1) st-agcr(1)";
const CLOSED: &str = "closed=1 [st-close host-close]";

/// The log of HiddenPartJournalShape.al :540, :574, :608, :642 (temporary part), BC 27 and 28 or BC 29.
fn temporary_part_log(third_read: bool) -> String {
    let (extra, next_agcr) = if third_read {
        (THIRD_READ, " st-agcr(0)")
    } else {
        ("", "")
    };
    format!("{OPEN}{SEND}{extra}]|next=1 No [host-agcr(H2) st-fill(H2,0){next_agcr}]|{CLOSED}")
}

#[test]
fn temporary_part_turned_visible_by_the_action_follows_the_major() {
    for routine in ["WorksheetSendVal", "WorksheetSendVar", "ListSendVal", "ListSendVar"] {
        for (bc, third_read) in [("27.5", false), ("28.5", false), ("29.0", true)] {
            let (out, hits) = run_on(bc, routine);
            assert_eq!(out, Ok(temporary_part_log(third_read)), "{routine} on {bc}");
            assert!(hits.contains(&Assumption::HiddenPartJournalShape), "{routine} on {bc}");
        }
    }
}

/// HiddenPartJournalShape.al:716: the stored part filtered with SetRange gives one log on every major.
#[test]
fn stored_part_turned_visible_by_the_action_is_the_same_on_every_major() {
    let want = format!("{OPEN}{SEND}]|next=1 No [host-agcr(H2) st-fill(H2,0)]|{CLOSED}");
    for bc in ["27.5", "28.5", "29.0"] {
        let (out, hits) = run_on(bc, "WorksheetStoredSendVal");
        assert_eq!(out, Ok(want.clone()), "on {bc}");
        assert!(hits.contains(&Assumption::HiddenPartJournalShape), "on {bc}");
    }
}

/// HiddenPartJournalShape.al:677: the request exists before the open, so the part is visible from the open on; only Next differs.
#[test]
fn part_visible_from_the_open_follows_the_major_on_next() {
    for (bc, next_agcr) in [("27.5", ""), ("28.5", ""), ("29.0", " st-agcr(0)")] {
        let want = format!(
            "open=1 Yes [host-open st-fill(,0) st-open host-agcr(H1) st-fill(H1,1) st-agr(1) st-agr(1) st-agcr(1)]|gotorecord=1 Yes []|next=1 No [host-agcr(H2) st-fill(H2,0){next_agcr}]|previous=1 Yes [host-agcr(H1) st-fill(H1,1) st-agr(1) st-agr(1) st-agcr(1)]|{CLOSED}"
        );
        let (out, hits) = run_on(bc, "WorksheetRequestBeforeOpen");
        assert_eq!(out, Ok(want), "on {bc}");
        assert!(hits.contains(&Assumption::HiddenPartJournalShape), "on {bc}");
    }
}

fn refused(out: &Result<String, String>, reason: &str) -> bool {
    matches!(out, Err(e) if e.contains("Unsupported") && e.contains(reason))
}

/// `SRC` with `from` replaced by `to` in its first place.
fn variant(from: &str, to: &str) -> String {
    let src = SRC.replacen(from, to, 1);
    assert_ne!(src, SRC, "{from}");
    src
}

/// The host opened with OpenEdit is outside the journal shape: the turn stays refused, and on a Worksheet the refusal names
/// the journal host's missing fact (al-runner issue 7922, ask 2).
#[test]
fn host_opened_for_edit_is_refused_by_name() {
    let (out, hits) = run_on("28.5", "WorksheetOpenEditSendVal");
    assert!(
        refused(&out, "the host page was not opened with OpenView (not pinned)"),
        "{out:?}"
    );
    assert!(!hits.contains(&Assumption::HiddenPartJournalShape));
}

const HOST_CODE: &str =
    "the host page or its source table has other code, an extension, a subscriber or other properties";
const HOST_LAYOUT: &str = "the host page has other controls than one repeater of fields of Rec, the part and actions";

/// A ListPlus host keeps main's refusal. A Worksheet host that differs from the measured one in one fact is refused with that
/// fact: another trigger, a trigger on its source table, a control property that is an expression or another literal, a group
/// with a variable control, the part in the Content area (review of al-runner PR 7939, shapes T18, U01, U02).
#[test]
fn other_host_pages_stay_refused() {
    let list_plus = SRC.replace(
        "PageType = Worksheet;\n    SourceTable = \"PG6 HJ Doc\";",
        "PageType = ListPlus;\n    SourceTable = \"PG6 HJ Doc\";",
    );
    assert_ne!(list_plus, SRC);
    let (out, _) = run_src(&list_plus, "28.5", "WorksheetSendVal");
    assert!(refused(&out, "the host page is not a Document (not pinned)"), "{out:?}");
    let host_field = "                field(\"No.\"; Rec.\"No.\") { ApplicationArea = All; }\n                field(Name; Rec.Name) { ApplicationArea = All; }\n            }\n        }\n        area(FactBoxes)\n        {\n            part(Status; \"PG6 HJ Status Part\")\n            {\n";
    let cases = [
        (
            variant(
                "    trigger OnClosePage()\n    begin\n        Log.Add('host-close');",
                "    trigger OnAfterGetRecord()\n    begin\n    end;\n\n    trigger OnClosePage()\n    begin\n        Log.Add('host-close');",
            ),
            HOST_CODE,
        ),
        (
            variant(
                "        key(PK; \"No.\") { Clustered = true; }\n    }\n}",
                "        key(PK; \"No.\") { Clustered = true; }\n    }\n\n    trigger OnModify()\n    begin\n    end;\n}",
            ),
            HOST_CODE,
        ),
        (
            variant(
                host_field,
                &host_field.replacen("ApplicationArea = All; }", "ApplicationArea = All; Visible = ShowStatus; }", 1),
            ),
            HOST_CODE,
        ),
        (
            variant(
                host_field,
                &host_field.replacen("ApplicationArea = All; }", "ApplicationArea = All; Editable = false; }", 1),
            ),
            HOST_LAYOUT,
        ),
        (
            variant(
                host_field,
                &host_field.replacen(
                    "            }\n        }\n        area(FactBoxes)",
                    "            }\n            group(G)\n            {\n                field(Shown; ShowStatus) { ApplicationArea = All; }\n            }\n        }\n        area(FactBoxes)",
                    1,
                ),
            ),
            HOST_CODE,
        ),
        (
            variant(
                host_field,
                &host_field.replacen("            }\n        }\n        area(FactBoxes)\n        {\n", "            }\n", 1),
            ),
            "the host page does not show exactly one part, in its FactBoxes area",
        ),
    ];
    for (src, reason) in cases {
        let (out, hits) = run_src(&src, "28.5", "WorksheetSendVal");
        assert!(refused(&out, reason), "{reason}: {out:?}");
        assert!(!hits.contains(&Assumption::HiddenPartJournalShape), "{reason}");
    }
}

/// The part page of Base Application's Workflow Status FactBox declares more properties (InsertAllowed and the others) than
/// the measured part: the turn is refused and names that fact (al-runner issue 7922, ask 2). So is a part page whose trigger
/// does more than log, or whose field has another property (review of al-runner PR 7939, shape U00).
#[test]
fn other_part_pages_are_refused_by_name() {
    let cases = [
        (
            variant(
                "    SourceTableTemporary = true;\n    Editable = false;",
                "    SourceTableTemporary = true;\n    Editable = false;\n    InsertAllowed = false;",
            ),
            "the part page declares other properties than Editable = false",
        ),
        (
            variant(
                "        Log.Add('st-agcr(' + Format(Rec.\"Entry No.\") + ')');",
                "        Log.Add('st-agcr(' + Format(Rec.\"Entry No.\") + ')');\n        Rec.\"Doc No.\" := 'ZZ';",
            ),
            "a trigger of the part page does more than call one procedure of a codeunit variable",
        ),
        (
            variant(
                "                field(\"Entry No.\"; Rec.\"Entry No.\") { ApplicationArea = All; }",
                "                field(\"Entry No.\"; Rec.\"Entry No.\") { ApplicationArea = All; Visible = false; }",
            ),
            "the part page has other controls than one repeater of fields of Rec with no property but ApplicationArea",
        ),
        (
            variant(
                "                Visible = ShowStatus;",
                "                Visible = ShowStatus and (Rec.\"No.\" <> '');",
            ),
            "the part control declares other properties than the measured ones",
        ),
        (
            variant("                Enabled = false;\n", ""),
            "the part control declares other properties than the measured ones",
        ),
    ];
    for (src, reason) in cases {
        let (out, hits) = run_src(&src, "28.5", "WorksheetSendVal");
        assert!(refused(&out, reason), "{reason}: {out:?}");
        assert!(!hits.contains(&Assumption::HiddenPartJournalShape), "{reason}");
    }
}

/// A part whose read writes to the database (here through the codeunit its trigger calls) is refused at the turn.
#[test]
fn part_read_that_writes_is_refused() {
    let src = variant(
        "    procedure Add(Line: Text)\n    begin\n",
        "    procedure Add(Line: Text)\n    var\n        Doc: Record \"PG6 HJ Doc\";\n    begin\n        if Line = 'st-agcr(1)' then begin\n            Doc.Init();\n            Doc.\"No.\" := 'H3';\n            Doc.Insert();\n        end;\n",
    );
    let (out, _) = run_src(&src, "28.5", "WorksheetSendVal");
    assert!(
        refused(&out, "a read of the hidden part of the journal shape that writes"),
        "{out:?}"
    );
}

const UNMEASURED_MOVE: &str =
    "a host move with the hidden part of the journal shape in a state its probe did not measure";

/// Next and Previous read by the part's state before and after the move (review of al-runner PR 7939, MED-1):
/// - hidden before and after (shape T04): main's result on every major, nothing marked;
/// - shown before and after, by Next and by Previous (shape T11): refused;
/// - a stored part shown from the open: its Next keeps main's result, its Previous to the shown row is refused.
#[test]
fn host_moves_read_only_between_the_measured_part_states() {
    for bc in ["27.5", "28.5", "29.0"] {
        let (out, hits) = run_on(bc, "NextHiddenToHidden");
        assert_eq!(
            out,
            Ok("next=0 No [host-agcr(H2) st-fill(H2,0)]|".to_string()),
            "on {bc}"
        );
        assert!(!hits.contains(&Assumption::HiddenPartJournalShape), "on {bc}");
    }
    for routine in [
        "BothRequestsBeforeOpenNext",
        "LastThenPrevious",
        "StoredRequestBeforeOpenPrevious",
    ] {
        let (out, _) = run_on("29.0", routine);
        assert!(refused(&out, UNMEASURED_MOVE), "{routine}: {out:?}");
    }
    let (out, hits) = run_on("29.0", "StoredRequestBeforeOpen");
    assert_eq!(
        out,
        Ok("gotorecord=1 Yes [st-agr(1) st-agr(1) st-agcr(1)]|next=1 No [host-agcr(H2) st-fill(H2,0)]|".to_string())
    );
    assert!(!hits.contains(&Assumption::HiddenPartJournalShape), "{hits:?}");
}

/// A GoToKey that stays is not the measured GoToRecord, and a second GoToRecord is not the test's first call: both keep main's
/// path (the part reads).
#[test]
fn other_stays_keep_mains_path() {
    let (out, hits) = run_on("28.5", "WorksheetGoToKeyStays");
    assert_eq!(out, Ok("[st-agcr(0)]".to_string()));
    assert!(!hits.contains(&Assumption::HiddenPartJournalShape), "{hits:?}");
    let (out, _) = run_on("28.5", "SecondGoToRecordStays");
    assert_eq!(out, Ok("gotorecord2=0 No [st-agcr(0)]|".to_string()));
}

/// The turn is the measured one only after one GoToRecord, without a commit in the body, with one row in the part, and when
/// the refresh leaves the part shown with that row (review of al-runner PR 7939, MED-2, MED-4; shapes T01, T02, T17, T20).
/// The commit and the delete are in the codeunit the action calls: an action body with such a statement is outside the shape
/// (`turns_outside_the_measured_cells_are_refused`).
#[test]
fn other_turns_are_refused() {
    let calls =
        "the test's calls since the open are not one GoToRecord that stayed on the host's row, then this action";
    let rows = "turns the hidden part of the journal shape visible with other than one row";
    const WRITES: &str = "an action that writes to the host's table or to other than one table and turns the hidden part of the journal shape visible (not pinned)";
    for (routine, reason) in [
        ("TurnWithoutGoToRecord", calls),
        ("TurnAfterTwoGoToRecords", calls),
        ("TurnTwoRows", rows),
        (
            "TurnWithCommit",
            "an action that commits and turns the hidden part of the journal shape visible",
        ),
        (
            "TurnThenRefreshHides",
            "the page left the journal shape of its hidden part during an action",
        ),
        // The codeunit the action calls also changes the host's table (review of al-runner PR 7939, round 3, X00 and X01),
        // or a third table.
        ("TurnCuModifiesHostRow", WRITES),
        ("TurnCuInsertsHostRow", WRITES),
        ("TurnCuInsertsThirdTable", WRITES),
    ] {
        for bc in ["28.5", "29.0"] {
            let (out, _) = run_on(bc, routine);
            assert!(refused(&out, reason), "{routine} on {bc}: {out:?}");
        }
    }
}

/// After the turn the host lets through only the part's Visible, one Next that moves and hides the part, and Close (review of
/// al-runner PR 7939, MED-3; shapes T03, T05 to T10, T13, T14, T21).
#[test]
fn after_the_turn_only_the_measured_steps_run() {
    let (out, _) = run_on("29.0", "TurnThenVisibleNextVisibleClose");
    assert_eq!(out, Ok(temporary_part_log(true).replacen(OPEN, "", 1)));
    let end = "after an action turned the hidden part of the journal shape visible";
    for step in [
        "SendAgain",
        "Noop",
        "PartCall",
        "GoToKey",
        "GoToRecord",
        "First",
        "Last",
        "Previous",
        "HostRead",
        "PartRead",
        "PartFirst",
        "PartNext",
        "NextThenNext",
        "NextThenPrevious",
        "NextThenSendAgain",
    ] {
        let (out, _) = run_on("29.0", &format!("AfterTurn{step}"));
        assert!(refused(&out, end), "{step}: {out:?}");
    }
    // The host's only row: the one Next after the turn stays. (A turn on the last of two rows needs a GoToRecord that moves,
    // which the turn refuses: `turns_outside_the_measured_cells_are_refused`, W22.)
    let (out, _) = run_on("29.0", "TurnOnLastRowThenNext");
    assert!(refused(&out, end), "{out:?}");
}

/// A major the probe did not measure is outside the shape for the temporary and for the stored part: the turn is refused as
/// main refuses it, before the part reads, and a move keeps main's path (review of al-runner PR 7939, MED-5).
#[test]
fn unmeasured_major_is_outside_the_shape() {
    for routine in ["WorksheetSendVal", "WorksheetStoredSendVal"] {
        let (out, hits) = run_on("30.0", routine);
        assert!(
            refused(
                &out,
                "the probe of the journal shape did not measure this BC major (not pinned)"
            ),
            "{routine}: {out:?}"
        );
        assert!(!hits.contains(&Assumption::HiddenPartJournalShape), "{routine}");
    }
    let (out, hits) = run_on("30.0", "WorksheetRequestBeforeOpen");
    assert!(out.is_ok(), "{out:?}");
    assert!(!hits.contains(&Assumption::HiddenPartJournalShape), "{hits:?}");
}

/// `SRC` with `from` replaced by `to` in its first place after the header `object`.
fn variant_in(object: &str, from: &str, to: &str) -> String {
    let at = SRC.find(object).unwrap();
    let tail = SRC[at..].replacen(from, to, 1);
    assert_ne!(tail, SRC[at..], "{from}");
    format!("{}{tail}", &SRC[..at])
}

const HOST_BODIES: &str = "a trigger or an action of the host page does more than call procedures of codeunit variables and the one local procedure that sets the Visible variable from the part's procedure";
const PART_FILL: &str = "the part page does not have exactly one procedure, or it names CurrPage";
const HOST_OPEN: &str = "        Log.Add('host-open');\n        RefreshStatus();";
const HOST_OPEN_FILTERS: &str = "        Log.Add('host-open');\n        Rec.FilterGroup(2);\n        Rec.SetFilter(\"No.\", 'H1|H2');\n        Rec.FilterGroup(0);\n        RefreshStatus();";
const HOST_AGCR: &str = "        Log.Add('host-agcr(' + Rec.\"No.\" + ')');\n        RefreshStatus();";
const HOST_AGCR_WRITES: &str =
    "        Log.Add('host-agcr(' + Rec.\"No.\" + ')');\n        Audit(Rec.\"No.\");\n        RefreshStatus();";
const REFRESH: &str = "    local procedure RefreshStatus()";
const AUDIT_THEN_REFRESH: &str = "    local procedure Audit(DocNo: Code[10])\n    var\n        A: Record \"PG6 HJ Status\";\n    begin\n        A.\"Entry No.\" := A.Count() + 1;\n        A.\"Doc No.\" := DocNo;\n        A.Insert();\n    end;\n\n    local procedure RefreshStatus()";
const FILL_END: &str = "        exit(not Rec.IsEmpty());";
const FILL_END_FINDS: &str =
    "        Rec.SetRange(\"Doc No.\", DocNo);\n        if Rec.FindLast() then;\n        exit(not Rec.IsEmpty());";
const FILL_END_FILTERS: &str = "        Rec.SetRange(\"Doc No.\", DocNo);\n        exit(not Rec.IsEmpty());";
const FILL_END_UPDATES: &str = "        CurrPage.Update(false);\n        exit(not Rec.IsEmpty());";

/// The turn runs only on the measured cells (review of al-runner PR 7939, round 2, MED-1). Each of these differs from them in
/// one fact and is refused, as on main, with nothing marked:
/// - W06: the List host with the stored part (measured pairs: Worksheet and List with the temporary part, Worksheet with the
///   stored part);
/// - W22: the GoToRecord before the action moved the host;
/// - W11: the host's OnOpenPage filters Rec (refused by the host's view);
/// - W15: the action's body also modifies the stored row of Rec through another variable;
/// - W16: the part's procedure leaves a filter and a position on the part's temporary Rec (FindLast is no measured method;
///   with the filter alone the view of the part's Rec refuses);
/// - the part page has a second procedure, or its procedure calls CurrPage.Update (W07).
#[test]
fn turns_outside_the_measured_cells_are_refused() {
    let send = "                    Log.Add('act-send');\n                    Send.SendByValue(Rec);\n                    RefreshStatus();";
    let send_var = "                var\n                    Send: Codeunit \"PG6 HJ Send\";";
    let renames = variant(
        send,
        "                    Log.Add('act-send');\n                    Send.SendByValue(Rec);\n                    Other.Get(Rec.\"No.\");\n                    Other.Name := 'X';\n                    Other.Modify();\n                    RefreshStatus();",
    )
    .replacen(
        send_var,
        "                var\n                    Send: Codeunit \"PG6 HJ Send\";\n                    Other: Record \"PG6 HJ Doc\";",
        1,
    );
    let cases = [
        (
            "W06",
            variant_in(
                "page 75316 \"PG6 HJ List\"",
                "part(Status; \"PG6 HJ Status Part\")",
                "part(Status; \"PG6 HJ Stored Part\")",
            ),
            "ListSendVal",
            "the host page type and the part's source are not a pair the probe of the journal shape measured",
        ),
        (
            "W22",
            SRC.to_string(),
            "GoToRecordMovesThenTurn",
            "the test's calls since the open are not one GoToRecord that stayed on the host's row, then this action",
        ),
        (
            "W11",
            variant(HOST_OPEN, HOST_OPEN_FILTERS),
            "WorksheetSendVal",
            HOST_VIEW,
        ),
        ("W15", renames, "WorksheetSendVal", HOST_BODIES),
        (
            "an action that sets the Visible variable itself (shape T02)",
            variant(
                "                    Log.Add('act-show');",
                "                    ShowStatus := true;",
            ),
            "TurnZeroRows",
            HOST_BODIES,
        ),
        ("W16", variant(FILL_END, FILL_END_FINDS), "WorksheetSendVal", PART_FILL),
        (
            "W16 without the FindLast",
            variant(FILL_END, FILL_END_FILTERS),
            "WorksheetSendVal",
            "the temporary part's Rec is not in its plain view after the part's procedure",
        ),
        (
            "second procedure",
            variant(
                "    procedure Fill(DocNo: Code[10]): Boolean",
                "    procedure Other()\n    begin\n    end;\n\n    procedure Fill(DocNo: Code[10]): Boolean",
            ),
            "WorksheetSendVal",
            PART_FILL,
        ),
        (
            "W07",
            variant(FILL_END, FILL_END_UPDATES),
            "WorksheetSendVal",
            PART_FILL,
        ),
    ];
    for (shape, src, routine, reason) in cases {
        for bc in ["28.5", "29.0"] {
            let (out, hits) = run_src(&src, bc, routine);
            assert!(refused(&out, reason), "{shape} on {bc}: {out:?}");
            assert!(!hits.contains(&Assumption::HiddenPartJournalShape), "{shape} on {bc}");
        }
    }
}

/// The host's Rec holds a filter, a mark or another key (`rec_view_is`).
const HOST_VIEW: &str = "the host's Rec is not in its plain view";

/// Next and Previous run only on the measured cells (review of al-runner PR 7939, round 2, MED-2):
/// - W05: the List host with a part shown from the open: its GoToRecord keeps main's path (the part reads), its Next is refused
///   (only the Worksheet measured a part shown from the open, HiddenPartJournalShape.al:677);
/// - W04: Previous shows a part that never showed since the open: refused (the measured Previous shows a part that showed
///   before, :677);
/// - W10 (the host's OnAfterGetCurrRecord writes), W12 (the host's OnOpenPage filters Rec), W17 (the part's procedure leaves a
///   filter), W08 (the part's procedure calls CurrPage.Update): the page is outside the shape, main's path, nothing marked.
#[test]
fn moves_outside_the_measured_cells_keep_mains_path_or_are_refused() {
    for bc in ["27.5", "28.5", "29.0"] {
        let (out, hits) = run_on(bc, "ListRequestBeforeOpen");
        assert_eq!(
            out,
            Ok("open=1 Yes [host-open st-fill(,0) st-open host-agcr(H1) st-fill(H1,1) st-agr(1) st-agr(1) st-agcr(1)]|gotorecord=1 Yes [st-agr(1) st-agr(1) st-agcr(1)]|".to_string()),
            "W05 on {bc}"
        );
        assert!(!hits.contains(&Assumption::HiddenPartJournalShape), "W05 on {bc}");
        let (out, hits) = run_on(bc, "ListRequestBeforeOpenNext");
        assert!(refused(&out, UNMEASURED_MOVE), "W05 Next on {bc}: {out:?}");
        assert!(!hits.contains(&Assumption::HiddenPartJournalShape), "W05 Next on {bc}");
        // The measured GoToRecord before it marks the assumption.
        let (out, _) = run_on(bc, "NeverShownThenPrevious");
        assert!(refused(&out, UNMEASURED_MOVE), "W04 on {bc}: {out:?}");
        let mains = "open=1 Yes [host-open st-fill(,0) st-open host-agcr(H1) st-fill(H1,1) st-agr(1) st-agr(1) st-agcr(1)]|gotorecord=1 Yes [st-agr(1) st-agr(1) st-agcr(1)]|next=1 No [host-agcr(H2) st-fill(H2,0)]|previous=1 Yes [host-agcr(H1) st-fill(H1,1)]|closed=1 [st-close host-close]";
        for (shape, src) in [
            (
                "W10",
                variant(HOST_AGCR, HOST_AGCR_WRITES).replacen(REFRESH, AUDIT_THEN_REFRESH, 1),
            ),
            ("W12", variant(HOST_OPEN, HOST_OPEN_FILTERS)),
        ] {
            let (out, hits) = run_src(&src, bc, "WorksheetRequestBeforeOpen");
            assert_eq!(out, Ok(mains.to_string()), "{shape} on {bc}");
            assert!(!hits.contains(&Assumption::HiddenPartJournalShape), "{shape} on {bc}");
        }
        for (shape, end) in [
            ("W17", FILL_END_FINDS),
            ("W17 without the FindLast", FILL_END_FILTERS),
            ("W08", FILL_END_UPDATES),
        ] {
            let (out, hits) = run_src(&variant(FILL_END, end), bc, "WorksheetRequestBeforeOpen");
            assert!(out.is_ok(), "{shape} on {bc}: {out:?}");
            assert!(!hits.contains(&Assumption::HiddenPartJournalShape), "{shape} on {bc}");
        }
    }
}

/// A measured Next whose host trigger writes to the database, here through the codeunit it calls, is refused: the measured
/// moves write nothing (review of al-runner PR 7939, round 2, MED-2).
#[test]
fn move_that_writes_is_refused() {
    let src = variant(
        "    procedure Add(Line: Text)\n    begin\n",
        "    procedure Add(Line: Text)\n    var\n        Status: Record \"PG6 HJ Status\";\n    begin\n        if Line = 'host-agcr(H2)' then begin\n            Status.Init();\n            Status.\"Entry No.\" := 1;\n            Status.Insert();\n        end;\n",
    );
    for bc in ["28.5", "29.0"] {
        let (out, _) = run_src(&src, bc, "WorksheetRequestBeforeOpen");
        assert!(refused(&out, UNMEASURED_MOVE), "on {bc}: {out:?}");
    }
}
