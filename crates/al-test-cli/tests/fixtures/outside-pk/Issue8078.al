// The reporter's repro of al-runner issue 8078 (app and test files in one); alc compiles the anchor.
table 50100 "Repro Document"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[100]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

page 50100 "Repro Document"
{
    PageType = Card;
    SourceTable = "Repro Document";
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            group(Functions)
            {
                Caption = 'Functions';
                action(SendDocument)
                {
                    ApplicationArea = All;
                    Caption = 'Send';

                    trigger OnAction()
                    begin
                    end;
                }
                // Same shape as Base Application's page 6640 "Purchase Return Order", separator(Action134)
                separator(Action134)
                {
                }
            }
        }
    }
}

pageextension 50150 "Repro Document Ext" extends "Repro Document"
{
    actions
    {
        // The anchor is a separator of the base page; alc accepts it.
        addafter(Action134)
        {
            action(ReproExtraAction)
            {
                ApplicationArea = All;
                Caption = 'Extra Action';

                trigger OnAction()
                begin
                    Message('Extra action');
                end;
            }
        }
    }
}

codeunit 50151 "Repro Separator Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    [HandlerFunctions('MessageHandler')]
    procedure ActionAddedAfterSeparatorRuns()
    var
        ReproDocument: Record "Repro Document";
        ReproDocumentPage: TestPage "Repro Document";
    begin
        // [GIVEN] a document
        ReproDocument."No." := 'REPRO1';
        ReproDocument.Insert();

        // [WHEN] the page is opened and the extension's action is invoked
        ReproDocumentPage.OpenEdit();
        ReproDocumentPage.GoToRecord(ReproDocument);
        ReproDocumentPage.ReproExtraAction.Invoke();
        ReproDocumentPage.Close();

        // [THEN] the action ran (MessageHandler)
    end;

    [MessageHandler]
    procedure MessageHandler(Msg: Text[1024])
    begin
    end;
}
