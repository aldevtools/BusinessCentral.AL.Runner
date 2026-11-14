//! al-runner issue 6924, the reporter's List host with page triggers: what Rec holds in a ListPart that shows no row when the
//! host has OnOpenPage (CurrPage.Editable from a value read at run time, a notification) and OnAfterGetCurrRecord (the same
//! notification). The objects and tests are corpus pull request 1378 (pages/parts/EmptyPartRecListHostTriggers.al, merged, not
//! yet in the pinned corpus), unchanged, with the corpus's Probe Observer. Its values were asserted on BC 27.5, 28.5 and 29.0.
//! The cells run through the assumption `empty-part-rec-list-host-triggers` until the file is pinned.

use al_builtins::assumptions::Assumption;
use al_testkit::{blocked, compile};
use al_testrun::Outcome;
use al_testrun::isolation::TestIsolation;

/// The corpus's Probe Observer (codeunits 69160 and 69163).
const OBSERVER: &str = include_str!("../../../../corpus/tests/al-language/_fixtures/ProbeObserver.Codeunit.al");

/// Each test of the codeunits named `codeunit`, its outcome and whether it went through `flag`.
fn run(src: &str, codeunit: &str, flag: Assumption) -> Vec<(String, Outcome, bool)> {
    let l = compile(&format!("{src}\n{OBSERVER}"));
    let engine = al_vm::Engine::new(l.program.clone(), blocked(&l));
    let mut hits = Vec::new();
    let mut o = Vec::new();
    for (oi, _) in l.program.objects.iter().enumerate().filter(|(_, o)| o.name == codeunit) {
        o.extend(al_testrun::runtime::run_codeunit_full(
            &engine,
            &l,
            oi,
            None,
            TestIsolation::Codeunit,
            None,
            &mut Vec::new(),
            &mut Vec::new(),
            &mut hits,
            None,
        ));
    }
    o.into_iter()
        .map(|(name, outcome)| {
            let flagged = hits
                .iter()
                .any(|(n, h): &(String, Vec<Assumption>)| *n == name && h.contains(&flag));
            (name, outcome, flagged)
        })
        .collect()
}

/// pages/parts/EmptyPartRecListHostTriggers.al of corpus pull request 1378 (merge commit 2e8dd762).
const PROBE: &str = r##"
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/properties/devenv-subpagelink-property
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); own objects declared in this file (PG EPT *)
// local object: the List hosts with page triggers, the ListParts and their action that reads Rec are under test; the SingleInstance
//   log is owned by test codeunit 76872 only.
// Probe (al-runner issue 6924, List hosts with page triggers): what Rec holds in the action of a ListPart that shows NO row, when
//   the host is a PageType = List page with the reporter's page triggers. EmptyPartRecLinkList.al measures List hosts without
//   triggers (and one with OnOpenPage, OnAfterGetRecord and OnAfterGetCurrRecord that only count). The reporter's host has exactly
//   two triggers: OnOpenPage sets CurrPage.Editable from a value computed at run time, then sends a notification when a setup flag
//   is off; OnAfterGetCurrRecord sends the same notification. Neither touches the part or the link fields.
// Area (one cell per test): two host forms (the first List host shape of EmptyPartRecLinkList.al and the reporter's form with a part
//   of page 1178's shape), each with both triggers, only OnOpenPage and only OnAfterGetCurrRecord; CurrPage.Editable true and false;
//   the notification sent (with a SendNotificationHandler, and once without one) and not sent; OpenEdit and OpenView; the host as
//   opened, after GoToRecord and after Next.
// Fact: (observed locally on BC 27.5 and 28.5; BC 29.0 by the pull request run): in every cell Rec in the action of the empty
//   part holds the link values of the row the host stands on, as on the hosts without triggers: "Table ID" 76860, Line No. 0 and
//   "No." = the host's "No." on the first form (A as opened, B after GoToRecord or Next), the host's "Ref Code" on the reporter's
//   form (RA as opened, RC after GoToRecord to the last row). Neither CurrPage.Editable (true or false), the notification (sent or
//   not, with or without a SendNotificationHandler) nor either trigger alone changes it. The part's OnAfterGetCurrRecord sees the
//   same Rec. Order (seq): OnOpenPage and its notification, then the host's OnAfterGetCurrRecord and its notification, then the
//   part's OnAfterGetCurrRecord; a GoToRecord or Next runs both OnAfterGetCurrRecord triggers again for every row it passes (B, or
//   RB and RC); invoking the part's action runs the part's OnAfterGetCurrRecord once more after it. A notification sent without a
//   SendNotificationHandler raises no error. TestPage.Editable() is No after OpenView and follows CurrPage.Editable after OpenEdit.
//   On BC 27.5 the first host open of the codeunit ran OnOpenPage twice: the first test takes that open and does not observe the
//   order.
// BC versions: 27-29

table 76860 "PG EPT Head"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { DataClassification = SystemMetadata; }
        field(2; "Ref Code"; Code[20]) { DataClassification = SystemMetadata; }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 76861 "PG EPT Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Table ID"; Integer) { DataClassification = SystemMetadata; }
        field(2; "No."; Code[20]) { DataClassification = SystemMetadata; }
        field(3; "Line No."; Integer) { DataClassification = SystemMetadata; }
        field(4; Kind; Code[10]) { DataClassification = SystemMetadata; }
    }

    keys
    {
        key(PK; "Table ID", "No.", "Line No.") { Clustered = true; }
    }
}

// The setup of the reporter's host: Licensed decides CurrPage.Editable in OnOpenPage, Enabled = false sends the notification.
table 76862 "PG EPT Setup"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Primary Key"; Code[10]) { DataClassification = SystemMetadata; }
        field(2; Licensed; Boolean) { DataClassification = SystemMetadata; }
        field(3; Enabled; Boolean) { DataClassification = SystemMetadata; }
    }

    keys
    {
        key(PK; "Primary Key") { Clustered = true; }
    }

    procedure IsLicensed(): Boolean
    begin
        if not Get() then
            exit(false);
        exit(Licensed);
    end;

    procedure ShowDisabledNotification()
    var
        Log: Codeunit "PG EPT Log";
        DisabledNotification: Notification;
    begin
        if Get() then
            if Enabled then
                exit;
        Log.AddSeq('ntf');
        DisabledNotification.Message := 'The EPT feature is disabled.';
        DisabledNotification.Scope := NotificationScope::LocalScope;
        DisabledNotification.Send();
    end;
}

// The shape of page 1178: a ListPart without the new-row line (as EmptyPartRecLinkList.al page 76842 "PG EPL Part").
page 76863 "PG EPT Part"
{
    PageType = ListPart;
    SourceTable = "PG EPT Line";
    InsertAllowed = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(LineNoCtl; Rec."Line No.") { ApplicationArea = All; }
                field(KindCtl; Rec.Kind) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ReadRec)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Log: Codeunit "PG EPT Log";
                begin
                    Log.AddAction('act=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No.") + '/' + Rec.Kind);
                end;
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('pcur=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No."));
    end;
}

// The triggers and properties of page 1178 (as EmptyPartRecLinkList.al page 76851 "PG EPL Part Doc").
page 76864 "PG EPT Part Doc"
{
    PageType = ListPart;
    SourceTable = "PG EPT Line";
    DeleteAllowed = true;
    DelayedInsert = true;
    InsertAllowed = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(LineNoCtl; Rec."Line No.")
                {
                    ApplicationArea = All;

                    trigger OnDrillDown()
                    begin
                        DrillDowns += 1;
                    end;
                }
                field(KindCtl; Rec.Kind)
                {
                    ApplicationArea = All;
                    Editable = false;
                }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ReadRec)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Log: Codeunit "PG EPT Log";
                begin
                    Log.AddAction('act=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No.") + '/' + Rec.Kind);
                end;
            }
        }
    }

    trigger OnInit()
    begin
        DrillDowns := 0;
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    trigger OnAfterGetCurrRecord()
    var
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('pcur=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No."));
    end;

    var
        DrillDowns: Integer;
}

// The issue's host form on the first List host shape (EmptyPartRecLinkList.al page 76843, FactBox only, as page 76853 "PG EPL List
// Triggers"): OnOpenPage sets CurrPage.Editable from the setup and sends the disabled notification; OnAfterGetCurrRecord sends it.
page 76865 "PG EPT List Both"
{
    PageType = List;
    SourceTable = "PG EPT Head";
    Caption = 'EPT List Both';
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Heads)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(RefCtl; Rec."Ref Code") { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPT Part")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = const(76860), "No." = field("No.");
            }
        }
    }

    trigger OnOpenPage()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('open');
        CurrPage.Editable := Setup.IsLicensed();
        Setup.ShowDisabledNotification();
    end;

    trigger OnAfterGetCurrRecord()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('hcur');
        Setup.ShowDisabledNotification();
    end;
}

// As "PG EPT List Both" with only OnOpenPage, which sets CurrPage.Editable and sends no notification.
page 76866 "PG EPT List OpenOnly"
{
    PageType = List;
    SourceTable = "PG EPT Head";
    Caption = 'EPT List OpenOnly';
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Heads)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(RefCtl; Rec."Ref Code") { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPT Part")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = const(76860), "No." = field("No.");
            }
        }
    }

    trigger OnOpenPage()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('open');
        CurrPage.Editable := Setup.IsLicensed();
    end;
}

// As "PG EPT List Both" with only OnAfterGetCurrRecord, which sends the disabled notification.
page 76867 "PG EPT List CurrOnly"
{
    PageType = List;
    SourceTable = "PG EPT Head";
    Caption = 'EPT List CurrOnly';
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Heads)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(RefCtl; Rec."Ref Code") { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPT Part")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = const(76860), "No." = field("No.");
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('hcur');
        Setup.ShowDisabledNotification();
    end;
}

// The issue's host triggers on the reporter's host form (EmptyPartRecLinkList.al page 76855 "PG EPL List Rep": a FactBox part of page
// 1178's shape with Caption and UpdatePropagation = Both, linked to the host's "Ref Code").
page 76868 "PG EPT Rep Both"
{
    PageType = List;
    SourceTable = "PG EPT Head";
    Caption = 'EPT Rep Both';
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Heads)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(RefCtl; Rec."Ref Code") { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPT Part Doc")
            {
                ApplicationArea = All;
                Caption = 'Attachments';
                SubPageLink = "Table ID" = const(Database::"PG EPT Head"), "No." = field("Ref Code");
                UpdatePropagation = Both;
            }
        }
    }

    trigger OnOpenPage()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('open');
        CurrPage.Editable := Setup.IsLicensed();
        Setup.ShowDisabledNotification();
    end;

    trigger OnAfterGetCurrRecord()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('hcur');
        Setup.ShowDisabledNotification();
    end;
}

// As "PG EPT Rep Both" with only OnOpenPage.
page 76869 "PG EPT Rep OpenOnly"
{
    PageType = List;
    SourceTable = "PG EPT Head";
    Caption = 'EPT Rep OpenOnly';
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Heads)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(RefCtl; Rec."Ref Code") { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPT Part Doc")
            {
                ApplicationArea = All;
                Caption = 'Attachments';
                SubPageLink = "Table ID" = const(Database::"PG EPT Head"), "No." = field("Ref Code");
                UpdatePropagation = Both;
            }
        }
    }

    trigger OnOpenPage()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('open');
        CurrPage.Editable := Setup.IsLicensed();
    end;
}

// As "PG EPT Rep Both" with only OnAfterGetCurrRecord.
page 76870 "PG EPT Rep CurrOnly"
{
    PageType = List;
    SourceTable = "PG EPT Head";
    Caption = 'EPT Rep CurrOnly';
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Heads)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(RefCtl; Rec."Ref Code") { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPT Part Doc")
            {
                ApplicationArea = All;
                Caption = 'Attachments';
                SubPageLink = "Table ID" = const(Database::"PG EPT Head"), "No." = field("Ref Code");
                UpdatePropagation = Both;
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('hcur');
        Setup.ShowDisabledNotification();
    end;
}

// The log of the actions and of the trigger order (open, hcur: the host's OnOpenPage and OnAfterGetCurrRecord; ntf: a notification
// sent; hnd: the SendNotificationHandler; pcur: the part's OnAfterGetCurrRecord with its Rec; act: the part's action with its Rec).
codeunit 76871 "PG EPT Log"
{
    SingleInstance = true;

    var
        ActionLines: Text;
        SeqLines: Text;

    procedure Reset()
    begin
        ActionLines := '';
        SeqLines := '';
    end;

    procedure AddAction(Line: Text)
    begin
        if ActionLines <> '' then
            ActionLines += ' | ';
        ActionLines += Line;
        AddSeq(Line);
    end;

    procedure AddSeq(Line: Text)
    begin
        if SeqLines <> '' then
            SeqLines += ' | ';
        SeqLines += Line;
    end;

    procedure ActionLog(): Text
    begin
        exit(ActionLines);
    end;

    procedure SeqLog(): Text
    begin
        exit(SeqLines);
    end;
}

codeunit 76872 "PG EPT Tests"
{
    Subtype = Test;

    var
        Log: Codeunit "PG EPT Log";

    // Heads A and B (Ref Code RA and RB), no line; the setup with the given flags.
    local procedure Initialize(Licensed: Boolean; Enabled: Boolean)
    var
        Head: Record "PG EPT Head";
        Line: Record "PG EPT Line";
        Setup: Record "PG EPT Setup";
    begin
        Head.DeleteAll();
        Line.DeleteAll();
        Setup.DeleteAll();
        Head."No." := 'A';
        Head."Ref Code" := 'RA';
        Head.Insert();
        Head."No." := 'B';
        Head."Ref Code" := 'RB';
        Head.Insert();
        Setup."Primary Key" := '';
        Setup.Licensed := Licensed;
        Setup.Enabled := Enabled;
        Setup.Insert();
        Log.Reset();
    end;

    // Heads A, B and C (Ref Code RA, RB and RC), no line: C is the last row of the host.
    local procedure InitializeThree(Licensed: Boolean; Enabled: Boolean)
    var
        Head: Record "PG EPT Head";
    begin
        Initialize(Licensed, Enabled);
        Head."No." := 'C';
        Head."Ref Code" := 'RC';
        Head.Insert();
    end;

    [SendNotificationHandler]
    procedure NotificationHandler(var TheNotification: Notification): Boolean
    begin
        Log.AddSeq('hnd');
        exit(true);
    end;

    [Test]
    procedure EmptyPart_ListBoth_FirstOpen_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListBoth_EditableNoNotify_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | hcur | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListBoth_EditableNoNotify_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListBoth_EditableNoNotify_OpenEdit_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | hcur | pcur=76860/A/0 | hcur | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListBoth_EditableNoNotify_OpenView_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/A/0 | hcur | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListBoth_EditableNoNotify_OpenEdit_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenEdit();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | hcur | pcur=76860/A/0 | hcur | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListBoth_EditableNoNotify_OpenView_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenView();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/A/0 | hcur | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListBoth_EditableNotify_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, false);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListBoth_EditableNotify_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, false);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListBoth_EditableNotify_OpenEdit_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, false);
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/A/0 | hcur | ntf | hnd | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListBoth_EditableNotify_OpenView_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, false);
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/A/0 | hcur | ntf | hnd | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListBoth_EditableNotify_OpenEdit_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, false);
        Host.OpenEdit();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/A/0 | hcur | ntf | hnd | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListBoth_EditableNotify_OpenView_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, false);
        Host.OpenView();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/A/0 | hcur | ntf | hnd | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListBoth_NotEditableNoNotify_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, true);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListBoth_NotEditableNoNotify_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, true);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListBoth_NotEditableNoNotify_OpenEdit_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, true);
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/A/0 | hcur | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListBoth_NotEditableNoNotify_OpenView_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, true);
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/A/0 | hcur | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListBoth_NotEditableNoNotify_OpenEdit_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, true);
        Host.OpenEdit();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/A/0 | hcur | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListBoth_NotEditableNoNotify_OpenView_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, true);
        Host.OpenView();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/A/0 | hcur | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListBoth_NotEditableNotify_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, false);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListBoth_NotEditableNotify_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, false);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListBoth_NotEditableNotify_OpenEdit_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, false);
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/A/0 | hcur | ntf | hnd | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListBoth_NotEditableNotify_OpenView_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, false);
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/A/0 | hcur | ntf | hnd | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListBoth_NotEditableNotify_OpenEdit_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, false);
        Host.OpenEdit();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/A/0 | hcur | ntf | hnd | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListBoth_NotEditableNotify_OpenView_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, false);
        Host.OpenView();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/A/0 | hcur | ntf | hnd | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListBoth_Notify_OpenEdit_NoHandler_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List Both";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, false);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | ntf | hcur | ntf | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListOpenOnly_Editable_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List OpenOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListOpenOnly_Editable_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List OpenOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListOpenOnly_Editable_OpenEdit_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List OpenOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | pcur=76860/A/0 | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListOpenOnly_Editable_OpenView_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List OpenOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/A/0 | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListOpenOnly_Editable_OpenEdit_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List OpenOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenEdit();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | pcur=76860/A/0 | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListOpenOnly_Editable_OpenView_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List OpenOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenView();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/A/0 | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListOpenOnly_NotEditable_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List OpenOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, true);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListOpenOnly_NotEditable_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List OpenOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, true);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListOpenOnly_NotEditable_OpenEdit_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List OpenOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, true);
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/A/0 | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListOpenOnly_NotEditable_OpenView_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List OpenOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, true);
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/A/0 | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListOpenOnly_NotEditable_OpenEdit_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List OpenOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, true);
        Host.OpenEdit();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/A/0 | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListOpenOnly_NotEditable_OpenView_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List OpenOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(false, true);
        Host.OpenView();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/A/0 | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListCurrOnly_NoNotify_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List CurrOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'hcur | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListCurrOnly_NoNotify_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List CurrOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'hcur | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListCurrOnly_NoNotify_OpenEdit_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List CurrOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'hcur | pcur=76860/A/0 | hcur | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListCurrOnly_NoNotify_OpenView_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List CurrOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'hcur | pcur=76860/A/0 | hcur | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListCurrOnly_NoNotify_OpenEdit_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List CurrOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenEdit();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'hcur | pcur=76860/A/0 | hcur | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListCurrOnly_NoNotify_OpenView_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List CurrOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, true);
        Host.OpenView();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'hcur | pcur=76860/A/0 | hcur | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListCurrOnly_Notify_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List CurrOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, false);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'hcur | ntf | hnd | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListCurrOnly_Notify_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List CurrOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, false);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'hcur | ntf | hnd | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListCurrOnly_Notify_OpenEdit_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List CurrOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, false);
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'hcur | ntf | hnd | pcur=76860/A/0 | hcur | ntf | hnd | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListCurrOnly_Notify_OpenView_GoToRecord_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List CurrOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, false);
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'hcur | ntf | hnd | pcur=76860/A/0 | hcur | ntf | hnd | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListCurrOnly_Notify_OpenEdit_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List CurrOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, false);
        Host.OpenEdit();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'hcur | ntf | hnd | pcur=76860/A/0 | hcur | ntf | hnd | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_ListCurrOnly_Notify_OpenView_Next_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List CurrOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, false);
        Host.OpenView();
        Host.Next();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/B/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'hcur | ntf | hnd | pcur=76860/A/0 | hcur | ntf | hnd | pcur=76860/B/0 | act=76860/B/0/ | pcur=76860/B/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_ListCurrOnly_Notify_OpenEdit_NoHandler_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT List CurrOnly";
        Head: Record "PG EPT Head";
    begin
        Initialize(true, false);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/A/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'hcur | ntf | pcur=76860/A/0 | act=76860/A/0/ | pcur=76860/A/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepBoth_EditableNoNotify_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, true);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | hcur | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepBoth_EditableNoNotify_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, true);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepBoth_EditableNoNotify_OpenEdit_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, true);
        Host.OpenEdit();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | hcur | pcur=76860/RA/0 | hcur | pcur=76860/RB/0 | hcur | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepBoth_EditableNoNotify_OpenView_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, true);
        Host.OpenView();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/RA/0 | hcur | pcur=76860/RB/0 | hcur | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_RepBoth_EditableNotify_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, false);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_RepBoth_EditableNotify_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, false);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_RepBoth_EditableNotify_OpenEdit_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, false);
        Host.OpenEdit();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/RA/0 | hcur | ntf | hnd | pcur=76860/RB/0 | hcur | ntf | hnd | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_RepBoth_EditableNotify_OpenView_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, false);
        Host.OpenView();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/RA/0 | hcur | ntf | hnd | pcur=76860/RB/0 | hcur | ntf | hnd | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepBoth_NotEditableNoNotify_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(false, true);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepBoth_NotEditableNoNotify_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(false, true);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepBoth_NotEditableNoNotify_OpenEdit_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(false, true);
        Host.OpenEdit();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/RA/0 | hcur | pcur=76860/RB/0 | hcur | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepBoth_NotEditableNoNotify_OpenView_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(false, true);
        Host.OpenView();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | hcur | pcur=76860/RA/0 | hcur | pcur=76860/RB/0 | hcur | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_RepBoth_NotEditableNotify_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(false, false);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_RepBoth_NotEditableNotify_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(false, false);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_RepBoth_NotEditableNotify_OpenEdit_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(false, false);
        Host.OpenEdit();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/RA/0 | hcur | ntf | hnd | pcur=76860/RB/0 | hcur | ntf | hnd | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_RepBoth_NotEditableNotify_OpenView_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(false, false);
        Host.OpenView();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | ntf | hnd | hcur | ntf | hnd | pcur=76860/RA/0 | hcur | ntf | hnd | pcur=76860/RB/0 | hcur | ntf | hnd | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepBoth_Notify_OpenEdit_NoHandler_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep Both";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, false);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | ntf | hcur | ntf | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepOpenOnly_Editable_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep OpenOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, true);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepOpenOnly_Editable_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep OpenOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, true);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepOpenOnly_Editable_OpenEdit_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep OpenOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, true);
        Host.OpenEdit();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'open | pcur=76860/RA/0 | pcur=76860/RB/0 | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepOpenOnly_Editable_OpenView_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep OpenOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, true);
        Host.OpenView();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/RA/0 | pcur=76860/RB/0 | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepOpenOnly_NotEditable_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep OpenOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(false, true);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepOpenOnly_NotEditable_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep OpenOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(false, true);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepOpenOnly_NotEditable_OpenEdit_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep OpenOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(false, true);
        Host.OpenEdit();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/RA/0 | pcur=76860/RB/0 | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepOpenOnly_NotEditable_OpenView_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep OpenOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(false, true);
        Host.OpenView();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'open | pcur=76860/RA/0 | pcur=76860/RB/0 | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepCurrOnly_NoNotify_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep CurrOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, true);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'hcur | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepCurrOnly_NoNotify_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep CurrOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, true);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'hcur | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepCurrOnly_NoNotify_OpenEdit_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep CurrOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, true);
        Host.OpenEdit();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'hcur | pcur=76860/RA/0 | hcur | pcur=76860/RB/0 | hcur | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepCurrOnly_NoNotify_OpenView_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep CurrOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, true);
        Host.OpenView();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'hcur | pcur=76860/RA/0 | hcur | pcur=76860/RB/0 | hcur | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_RepCurrOnly_Notify_OpenEdit_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep CurrOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, false);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'hcur | ntf | hnd | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_RepCurrOnly_Notify_OpenView_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep CurrOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, false);
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'hcur | ntf | hnd | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_RepCurrOnly_Notify_OpenEdit_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep CurrOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, false);
        Host.OpenEdit();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'hcur | ntf | hnd | pcur=76860/RA/0 | hcur | ntf | hnd | pcur=76860/RB/0 | hcur | ntf | hnd | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NotificationHandler')]
    procedure EmptyPart_RepCurrOnly_Notify_OpenView_GoToLastRow_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep CurrOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, false);
        Host.OpenView();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RC/0/');
        Observer.Expect('editable', 'No');
        Observer.Expect('seq', 'hcur | ntf | hnd | pcur=76860/RA/0 | hcur | ntf | hnd | pcur=76860/RB/0 | hcur | ntf | hnd | pcur=76860/RC/0 | act=76860/RC/0/ | pcur=76860/RC/0');
        Observer.Verify();
    end;

    [Test]
    procedure EmptyPart_RepCurrOnly_Notify_OpenEdit_NoHandler_ActionReadsRec()
    var
        Observer: Codeunit "Probe Observer";
        Host: TestPage "PG EPT Rep CurrOnly";
        Head: Record "PG EPT Head";
    begin
        InitializeThree(true, false);
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Observer.Observe('action', Log.ActionLog());
        Observer.Observe('editable', Format(Host.Editable()));
        Observer.Observe('seq', Log.SeqLog());
        Host.Close();
        Observer.Expect('action', 'act=76860/RA/0/');
        Observer.Expect('editable', 'Yes');
        Observer.Expect('seq', 'hcur | ntf | pcur=76860/RA/0 | act=76860/RA/0/ | pcur=76860/RA/0');
        Observer.Verify();
    end;
}
"##;

/// Every cell passes through the assumption: the action of the part reads BC's Rec (observation 1), TestPage.Editable is BC's
/// (observation 2), and the trigger order is BC's (observation 3, `seq`), the part's OnAfterGetCurrRecord included: on a part
/// in edit mode, at the row a GoToRecord passes on the reporter's form, and again after the part's action.
#[test]
fn empty_list_part_rec_trigger_hosts_cells() {
    let o = run(PROBE, "PG EPT Tests", Assumption::EmptyPartRecListHostTriggers);
    assert_eq!(o.len(), 85, "{o:?}");
    let wrong: Vec<_> = o
        .iter()
        .filter(|(_, outcome, flagged)| *outcome != Outcome::Pass || !flagged)
        .collect();
    assert!(wrong.is_empty(), "{} wrong: {wrong:#?}", wrong.len());
}

/// Neighbors of the probe's hosts, each one fact away from a cell: a trigger body the probe does not have (CurrPage.Editable from
/// a constant, a log that reads Rec, a notification with GlobalScope, a page procedure that sets CurrPage.Editable as the
/// reporter's license check does), another trigger set, a handler that answers false, Next on the reporter's form, other numbers
/// of host rows, a GoToRecord that stays on the row, page 1178's part shape on the first host form, the reporter's form with
/// another part, area or link, the first form with another area or link. The read of the part's Rec is refused there, as before
/// this change, each with the fact that left the cells.
const NEIGHBORS: &str = r##"
table 76880 "PG EPTN Setup2"
{
    fields { field(1; "Primary Key"; Code[10]) { } field(2; Licensed; Boolean) { } field(3; Enabled; Boolean) { } }
    keys { key(PK; "Primary Key") { Clustered = true; } }

    procedure ShowGlobal()
    var
        Log: Codeunit "PG EPT Log";
        N: Notification;
    begin
        if Get() then
            if Enabled then
                exit;
        Log.AddSeq('ntf');
        N.Message := 'The EPT feature is disabled.';
        N.Scope := NotificationScope::GlobalScope;
        N.Send();
    end;
}
page 76881 "PG EPTN ConstEditable"
{
    PageType = List; SourceTable = "PG EPT Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPT Part") { ApplicationArea = All; SubPageLink = "Table ID" = const(76860), "No." = field("No."); } }
    }
    trigger OnOpenPage()
    var
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('open');
        CurrPage.Editable := true;
    end;
}
page 76882 "PG EPTN ReadsRec"
{
    PageType = List; SourceTable = "PG EPT Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPT Part") { ApplicationArea = All; SubPageLink = "Table ID" = const(76860), "No." = field("No."); } }
    }
    trigger OnAfterGetCurrRecord()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('hcur' + Rec."No.");
        Setup.ShowDisabledNotification();
    end;
}
page 76883 "PG EPTN GlobalScope"
{
    PageType = List; SourceTable = "PG EPT Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPT Part") { ApplicationArea = All; SubPageLink = "Table ID" = const(76860), "No." = field("No."); } }
    }
    trigger OnAfterGetCurrRecord()
    var
        Setup: Record "PG EPTN Setup2";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('hcur');
        Setup.ShowGlobal();
    end;
}
page 76884 "PG EPTN PageProc"
{
    PageType = List; SourceTable = "PG EPT Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPT Part") { ApplicationArea = All; SubPageLink = "Table ID" = const(76860), "No." = field("No."); } }
    }
    trigger OnOpenPage()
    var
        Setup: Record "PG EPT Setup";
    begin
        CheckLicense();
        Setup.ShowDisabledNotification();
    end;
    trigger OnAfterGetCurrRecord()
    var
        Setup: Record "PG EPT Setup";
    begin
        Setup.ShowDisabledNotification();
    end;
    local procedure CheckLicense()
    var
        Setup: Record "PG EPT Setup";
    begin
        CurrPage.Editable := Setup.IsLicensed();
    end;
}
page 76885 "PG EPTN OpenGetRecord"
{
    PageType = List; SourceTable = "PG EPT Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPT Part") { ApplicationArea = All; SubPageLink = "Table ID" = const(76860), "No." = field("No."); } }
    }
    trigger OnOpenPage()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('open');
        CurrPage.Editable := Setup.IsLicensed();
    end;
    trigger OnAfterGetRecord()
    var
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('hrec');
    end;
}
page 76887 "PG EPTN DocPart"
{
    PageType = List; SourceTable = "PG EPT Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPT Part Doc") { ApplicationArea = All; SubPageLink = "Table ID" = const(76860), "No." = field("No."); } }
    }
    trigger OnOpenPage()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('open');
        CurrPage.Editable := Setup.IsLicensed();
    end;
}
page 76888 "PG EPTN RepPlainPart"
{
    PageType = List; SourceTable = "PG EPT Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPT Part") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "Table ID" = const(Database::"PG EPT Head"), "No." = field("Ref Code"); UpdatePropagation = Both; } }
    }
    trigger OnOpenPage()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('open');
        CurrPage.Editable := Setup.IsLicensed();
    end;
}
page 76889 "PG EPTN RepContent"
{
    PageType = List; SourceTable = "PG EPT Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } part(Box; "PG EPT Part Doc") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "Table ID" = const(Database::"PG EPT Head"), "No." = field("Ref Code"); UpdatePropagation = Both; } }
    }
    trigger OnOpenPage()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('open');
        CurrPage.Editable := Setup.IsLicensed();
    end;
}
page 76890 "PG EPTN RepFilterLink"
{
    PageType = List; SourceTable = "PG EPT Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPT Part Doc") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "Table ID" = filter(76860), "No." = field("Ref Code"); UpdatePropagation = Both; } }
    }
    trigger OnOpenPage()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('open');
        CurrPage.Editable := Setup.IsLicensed();
    end;
}
page 76891 "PG EPTN RepSwapLink"
{
    PageType = List; SourceTable = "PG EPT Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPT Part Doc") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "No." = field("Ref Code"), "Table ID" = const(76860); UpdatePropagation = Both; } }
    }
    trigger OnOpenPage()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('open');
        CurrPage.Editable := Setup.IsLicensed();
    end;
}
page 76892 "PG EPTN FirstContent"
{
    PageType = List; SourceTable = "PG EPT Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } part(Box; "PG EPT Part") { ApplicationArea = All; SubPageLink = "Table ID" = const(76860), "No." = field("No."); } }
    }
    trigger OnOpenPage()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('open');
        CurrPage.Editable := Setup.IsLicensed();
    end;
}
page 76893 "PG EPTN FirstFilterLink"
{
    PageType = List; SourceTable = "PG EPT Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPT Part") { ApplicationArea = All; SubPageLink = "Table ID" = filter(76860), "No." = field("No."); } }
    }
    trigger OnOpenPage()
    var
        Setup: Record "PG EPT Setup";
        Log: Codeunit "PG EPT Log";
    begin
        Log.AddSeq('open');
        CurrPage.Editable := Setup.IsLicensed();
    end;
}
codeunit 76886 "PG EPTN Tests"
{
    Subtype = Test;
    var Log: Codeunit "PG EPT Log";
    local procedure Init(Rows: Integer)
    var H: Record "PG EPT Head"; L: Record "PG EPT Line"; S: Record "PG EPT Setup"; S2: Record "PG EPTN Setup2";
    begin
        H.DeleteAll(); L.DeleteAll(); S.DeleteAll(); S2.DeleteAll();
        H."No." := 'A'; H."Ref Code" := 'RA'; H.Insert();
        if Rows >= 2 then begin H."No." := 'B'; H."Ref Code" := 'RB'; H.Insert(); end;
        if Rows = 3 then begin H."No." := 'C'; H."Ref Code" := 'RC'; H.Insert(); end;
        S.Licensed := true; S.Enabled := false; S.Insert();
        S2.Licensed := true; S2.Enabled := false; S2.Insert();
        Log.Reset();
    end;
    [SendNotificationHandler] procedure Handler(var N: Notification): Boolean begin Log.AddSeq('hnd'); exit(true); end;
    [SendNotificationHandler] procedure FalseHandler(var N: Notification): Boolean begin Log.AddSeq('hnd'); exit(false); end;
    [Test] procedure ConstEditable() var P: TestPage "PG EPTN ConstEditable"; begin Init(2); P.OpenEdit(); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] [HandlerFunctions('Handler')] procedure ReadsRec() var P: TestPage "PG EPTN ReadsRec"; begin Init(2); P.OpenEdit(); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] [HandlerFunctions('Handler')] procedure GlobalScope() var P: TestPage "PG EPTN GlobalScope"; begin Init(2); P.OpenEdit(); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] [HandlerFunctions('Handler')] procedure PageProc() var P: TestPage "PG EPTN PageProc"; begin Init(2); P.OpenEdit(); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] procedure OpenGetRecord() var P: TestPage "PG EPTN OpenGetRecord"; begin Init(2); P.OpenEdit(); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] [HandlerFunctions('FalseHandler')] procedure HandlerFalse() var P: TestPage "PG EPT List Both"; begin Init(2); P.OpenEdit(); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] [HandlerFunctions('Handler')] procedure RepNext() var P: TestPage "PG EPT Rep Both"; begin Init(3); P.OpenEdit(); P.Next(); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] [HandlerFunctions('Handler')] procedure FirstFormThreeRows() var P: TestPage "PG EPT List Both"; H: Record "PG EPT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] [HandlerFunctions('Handler')] procedure FirstFormOpenOneRow() var P: TestPage "PG EPT List Both"; begin Init(1); P.OpenView(); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] [HandlerFunctions('Handler')] procedure GoToSameRow() var P: TestPage "PG EPT List Both"; H: Record "PG EPT Head"; begin Init(2); P.OpenEdit(); H.Get('A'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] procedure DocPart() var P: TestPage "PG EPTN DocPart"; begin Init(2); P.OpenEdit(); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] [HandlerFunctions('Handler')] procedure ReporterTwoRows() var P: TestPage "PG EPT Rep Both"; begin Init(2); P.OpenEdit(); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] procedure RepPlainPart() var P: TestPage "PG EPTN RepPlainPart"; H: Record "PG EPT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] procedure RepContent() var P: TestPage "PG EPTN RepContent"; H: Record "PG EPT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] procedure RepFilterLink() var P: TestPage "PG EPTN RepFilterLink"; H: Record "PG EPT Head"; begin Init(3); P.OpenView(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] procedure RepSwapLink() var P: TestPage "PG EPTN RepSwapLink"; H: Record "PG EPT Head"; begin Init(3); P.OpenEdit(); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] procedure FirstContent() var P: TestPage "PG EPTN FirstContent"; begin Init(2); P.OpenEdit(); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
    [Test] procedure FirstFilterLink() var P: TestPage "PG EPTN FirstFilterLink"; begin Init(2); P.OpenView(); P.Box.ReadRec.Invoke(); Error(Log.ActionLog()); end;
}
"##;

#[test]
fn empty_list_part_rec_trigger_hosts_neighbors_refuse() {
    // The fact each neighbor is refused with.
    let triggers = "the List host has a set of triggers no probe host has";
    let landing = "with a number of host rows no probe cell of a List host with page triggers has";
    let rep_form = "differ from the cells of its host form";
    let first_form = "differ from the cells of a List host with page triggers";
    let want: [(&str, &str); 18] = [
        ("ConstEditable", triggers),
        ("ReadsRec", triggers),
        ("GlobalScope", triggers),
        ("PageProc", triggers),
        ("OpenGetRecord", triggers),
        (
            "HandlerFalse",
            "the test binds a SendNotificationHandler no probe test has",
        ),
        // RepNext and GoToSameRow land in a cell as the host opens (a flagged landing) and are refused after the move.
        ("RepNext", landing),
        ("FirstFormThreeRows", landing),
        ("FirstFormOpenOneRow", landing),
        ("GoToSameRow", landing),
        ("DocPart", "differ from the cells of a List host with page triggers"),
        ("ReporterTwoRows", landing),
        // The reporter's form with the part of the first form, in area(Content), with a filter(..) link or the link's entries
        // swapped; the first form with its part in area(Content) or with a filter(..) link.
        ("RepPlainPart", rep_form),
        ("RepContent", rep_form),
        ("RepFilterLink", rep_form),
        ("RepSwapLink", rep_form),
        ("FirstContent", first_form),
        ("FirstFilterLink", first_form),
    ];
    let o = run(
        &format!("{PROBE}\n{NEIGHBORS}"),
        "PG EPTN Tests",
        Assumption::EmptyPartRecListHostTriggers,
    );
    assert_eq!(o.len(), want.len(), "{o:?}");
    let wrong: Vec<_> = o
        .iter()
        .filter(|(name, outcome, _)| {
            let fact = want.iter().find(|(n, _)| n == name).map_or("?", |(_, f)| *f);
            !matches!(outcome, Outcome::NotRun(m) if m.contains("a read of Rec of a part that shows no row") && m.contains(fact))
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Neighbors of the rows a GoToRecord passes on the reporter's form (`walk_parts.rs`): a GoToKey in place of the GoToRecord, a
/// GoToRecord to a row the host does not have, and a host of four rows (the passed rows have other rows around them than RB of
/// three). The part reads nothing at a passed row there, as before this change; with three rows and GoToRecord it does
/// (EmptyPartRecListHostTriggers.al:2080). Each test fails with the trigger order it logged.
const WALK_NEIGHBORS: &str = r##"
codeunit 76894 "PG EPTW Tests"
{
    Subtype = Test;
    var Log: Codeunit "PG EPT Log";
    local procedure Init(Rows: Integer)
    var H: Record "PG EPT Head"; L: Record "PG EPT Line"; S: Record "PG EPT Setup"; I: Integer;
    begin
        H.DeleteAll(); L.DeleteAll(); S.DeleteAll();
        for I := 1 to Rows do begin H."No." := CopyStr('ABCD', I, 1); H."Ref Code" := 'R' + H."No."; H.Insert(); end;
        S.Licensed := true; S.Enabled := true; S.Insert();
        Log.Reset();
    end;
    [Test] procedure ThreeRowsGoToRecordBoth() var P: TestPage "PG EPT Rep Both"; H: Record "PG EPT Head"; begin Init(3); P.OpenView(); H.Get('C'); P.GoToRecord(H); Error(Log.SeqLog()); end;
    [Test] procedure ThreeRowsGoToRecordOpenOnly() var P: TestPage "PG EPT Rep OpenOnly"; H: Record "PG EPT Head"; begin Init(3); P.OpenView(); H.Get('C'); P.GoToRecord(H); Error(Log.SeqLog()); end;
    [Test] procedure GoToKeyBoth() var P: TestPage "PG EPT Rep Both"; begin Init(3); P.OpenView(); P.GoToKey('C'); Error(Log.SeqLog()); end;
    [Test] procedure GoToKeyOpenOnly() var P: TestPage "PG EPT Rep OpenOnly"; begin Init(3); P.OpenView(); P.GoToKey('C'); Error(Log.SeqLog()); end;
    [Test] procedure MissBoth() var P: TestPage "PG EPT Rep Both"; H: Record "PG EPT Head"; begin Init(3); P.OpenView(); H."No." := 'Z'; if P.GoToRecord(H) then; Error(Log.SeqLog()); end;
    [Test] procedure MissOpenOnly() var P: TestPage "PG EPT Rep OpenOnly"; H: Record "PG EPT Head"; begin Init(3); P.OpenView(); H."No." := 'Z'; if P.GoToRecord(H) then; Error(Log.SeqLog()); end;
    [Test] procedure FourRowsBoth() var P: TestPage "PG EPT Rep Both"; H: Record "PG EPT Head"; begin Init(4); P.OpenView(); H.Get('D'); P.GoToRecord(H); Error(Log.SeqLog()); end;
    [Test] procedure FourRowsOpenOnly() var P: TestPage "PG EPT Rep OpenOnly"; H: Record "PG EPT Head"; begin Init(4); P.OpenView(); H.Get('D'); P.GoToRecord(H); Error(Log.SeqLog()); end;
}
"##;

#[test]
fn empty_list_part_rec_trigger_hosts_walk_neighbors() {
    let o = run(
        &format!("{PROBE}\n{WALK_NEIGHBORS}"),
        "PG EPTW Tests",
        Assumption::EmptyPartRecListHostTriggers,
    );
    assert_eq!(o.len(), 8, "{o:?}");
    let passed_rb = |outcome: &Outcome| matches!(outcome, Outcome::Fail(m) if m.contains("pcur=76860/RB/0"));
    let wrong: Vec<_> = o
        .iter()
        .filter(|(name, outcome, _)| {
            let three_rows = name.starts_with("ThreeRowsGoToRecord");
            !matches!(outcome, Outcome::Fail(_)) || passed_rb(outcome) != three_rows
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}
