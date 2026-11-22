// Objects next to the keyword-variable-names rule (al-runner issue 8539): names and places the AL compiler takes. The
// keywords stand here where the grammar wants them (fieldgroups, keys, area, xmlport elements), as quoted names, after an
// attribute, after a comma, and as near words. The neighbor tests run them; none of them may stop compiling.
table 50000 "NB KV Row"
{
    fields
    {
        field(1; "No."; Code[20])
        {
            trigger OnValidate()
            var
                [NonDebuggable]
                Key: Text;
            begin
                Key := "No.";
            end;
        }
        field(2; "Key"; Integer)
        {
        }
        field(3; "Area"; Integer)
        {
        }
    }
    keys
    {
        key(PK; "No.")
        {
            Clustered = true;
        }
        key(ByKey; "Key")
        {
        }
    }
    fieldgroups
    {
        fieldgroup(DropDown; "No.", "Key")
        {
        }
    }

    trigger OnInsert()
    var
        [NonDebuggable]
        TextElement: Text;
    begin
        TextElement := 'inserted';
        Rec."Key" += 1;
    end;
}

codeunit 50001 "NB KV Names"
{
    var
        [NonDebuggable]
        Key: Text;
        [NonDebuggable]
        Area: Text;
        "Event": Integer;
        Keys: Integer;

    procedure AttributedGlobal(): Text
    begin
        Key := 'k';
        Area := 'a';
        exit(Key + Area);
    end;

    procedure QuotedGlobal(): Integer
    begin
        "Event" := 7;
        exit("Event");
    end;

    procedure NearWordGlobal(): Integer
    begin
        Keys := 3;
        exit(Keys);
    end;

    procedure AttributedLocals(): Text
    var
        [NonDebuggable]
        Actions: Text;
        [NonDebuggable]
        CueGroup: Text;
        [NonDebuggable]
        Extends: Text;
        [NonDebuggable]
        FieldGroups: Text;
        [NonDebuggable]
        PageExtension: Text;
        [NonDebuggable]
        TableElement: Text;
    begin
        Actions := 'a';
        CueGroup := 'c';
        Extends := 'e';
        FieldGroups := 'f';
        PageExtension := 'p';
        TableElement := 't';
        exit(Actions + CueGroup + Extends + FieldGroups + PageExtension + TableElement);
    end;

    procedure QuotedLocals(): Integer
    var
        "Key": Integer;
        "True": Integer;
        "Event": Integer;
        "Begin": Integer;
    begin
        "Key" := 1;
        "True" := 2;
        "Event" := 3;
        "Begin" := 4;
        exit("Key" + "True" + "Event" + "Begin");
    end;

    procedure NearWordLocals(): Integer
    var
        Keys: Integer;
        Fields: Integer;
        Field: Integer;
        Layout: Integer;
        Group: Integer;
        Value: Integer;
    begin
        Keys := 1;
        Fields := 2;
        Field := 3;
        Layout := 4;
        Group := 5;
        Value := 6;
        exit(Keys + Fields + Field + Layout + Group + Value);
    end;

    procedure AfterAComma(): Integer
    var
        Other, Local: Integer;
        First, "If": Integer;
    begin
        Other := 1;
        "Local" := 2;
        First := 3;
        "If" := 4;
        exit(Other + "Local" + First + "If");
    end;

    procedure StatementKeywordName(): Integer
    var
        While: Integer;
    begin
        "While" := 9;
        exit("While");
    end;

    procedure KeysParameter(Keys: Integer; Fields: Integer): Integer
    begin
        exit(Keys * Fields);
    end;

    procedure KeyOf(Row: Record "NB KV Row"): Integer
    begin
        exit(Row."Key");
    end;

    procedure InTrigger(): Text
    begin
        exit(TriggerHelper());
    end;

    local procedure TriggerHelper() Result: Text
    var
        [NonDebuggable]
        TextAttribute: Text;
    begin
        TextAttribute := 'ta';
        Result := TextAttribute;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"NB KV Names", OnNamed, '', false, false)]
    local procedure OnNamedSubscriber(var Seen: Text)
    var
        [NonDebuggable]
        FieldElement: Text;
    begin
        FieldElement := 'fe';
        Seen := FieldElement;
    end;

    procedure RaiseNamed(): Text
    var
        Seen: Text;
    begin
        OnNamed(Seen);
        exit(Seen);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnNamed(var Seen: Text)
    begin
    end;
}

page 50002 "NB KV Card"
{
    PageType = Card;
    SourceTable = "NB KV Row";

    layout
    {
        area(Content)
        {
            field(No; Rec."No.")
            {
                trigger OnValidate()
                var
                    [NonDebuggable]
                    ChartPart: Text;
                begin
                    ChartPart := Rec."No.";
                end;
            }
            field("Key"; Rec."Key")
            {
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(Bump)
            {
                trigger OnAction()
                var
                    [NonDebuggable]
                    ProfileExtension: Text;
                begin
                    ProfileExtension := 'p';
                    Rec."Key" += StrLen(ProfileExtension);
                end;
            }
        }
    }

    var
        [NonDebuggable]
        TableExtension: Text;

    procedure Marker(): Text
    begin
        TableExtension := 'te';
        exit(TableExtension);
    end;
}

report 50003 "NB KV Report"
{
    ProcessingOnly = true;

    dataset
    {
        dataitem(Row; "NB KV Row")
        {
        }
    }
    requestpage
    {
        var
            [NonDebuggable]
            Key: Text;
            "Area": Integer;
            Keys: Integer;
    }

    procedure Names(): Integer
    begin
        exit(1);
    end;
}

xmlport 50004 "NB KV Xml"
{
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "NB KV Row")
            {
                fieldattribute(No; Row."No.")
                {
                }
                textattribute(Kind)
                {
                }
                fieldelement("Key"; Row."Key")
                {
                }
            }
        }
    }
    requestpage
    {
        var
            [NonDebuggable]
            FieldElement: Text;
            Fields: Integer;
    }
}

codeunit 50005 "NB KV Tests"
{
    Subtype = Test;

    [Test]
    procedure AttributedGlobal_Compiles()
    var
        N: Codeunit "NB KV Names";
    begin
        if N.AttributedGlobal() <> 'ka' then
            Error('got %1', N.AttributedGlobal());
    end;

    [Test]
    procedure QuotedGlobal_Compiles()
    var
        N: Codeunit "NB KV Names";
    begin
        if N.QuotedGlobal() <> 7 then
            Error('got %1', N.QuotedGlobal());
    end;

    [Test]
    procedure NearWordGlobal_Compiles()
    var
        N: Codeunit "NB KV Names";
    begin
        if N.NearWordGlobal() <> 3 then
            Error('got %1', N.NearWordGlobal());
    end;

    [Test]
    procedure AttributedLocals_Compile()
    var
        N: Codeunit "NB KV Names";
    begin
        if N.AttributedLocals() <> 'acefpt' then
            Error('got %1', N.AttributedLocals());
    end;

    [Test]
    procedure QuotedLocals_Compile()
    var
        N: Codeunit "NB KV Names";
    begin
        if N.QuotedLocals() <> 10 then
            Error('got %1', N.QuotedLocals());
    end;

    [Test]
    procedure NearWordLocals_Compile()
    var
        N: Codeunit "NB KV Names";
    begin
        if N.NearWordLocals() <> 21 then
            Error('got %1', N.NearWordLocals());
    end;

    [Test]
    procedure LocalAndIfAfterAComma_Compile()
    var
        N: Codeunit "NB KV Names";
    begin
        if N.AfterAComma() <> 10 then
            Error('got %1', N.AfterAComma());
    end;

    [Test]
    procedure StatementKeywordAsName_Compiles()
    var
        N: Codeunit "NB KV Names";
    begin
        if N.StatementKeywordName() <> 9 then
            Error('got %1', N.StatementKeywordName());
    end;

    [Test]
    procedure NearWordParameters_Compile()
    var
        N: Codeunit "NB KV Names";
    begin
        if N.KeysParameter(2, 3) <> 6 then
            Error('got %1', N.KeysParameter(2, 3));
    end;

    [Test]
    procedure FieldNamedKey_TableTriggersAndKeys()
    var
        Row: Record "NB KV Row";
        N: Codeunit "NB KV Names";
    begin
        Row."No." := 'A';
        Row."Key" := 4;
        Row.Insert(true);
        Row.Validate("No.", 'A');
        Row.Get('A');
        if N.KeyOf(Row) <> 5 then
            Error('got %1', N.KeyOf(Row));
        Row.SetCurrentKey("Key");
        Row.SetRange("Key", 5);
        if Row.Count() <> 1 then
            Error('count %1', Row.Count());
    end;

    [Test]
    procedure AttributedLocalOfALocalProcedure_Compiles()
    var
        N: Codeunit "NB KV Names";
    begin
        if N.InTrigger() <> 'ta' then
            Error('got %1', N.InTrigger());
    end;

    [Test]
    procedure AttributedLocalOfASubscriber_Compiles()
    var
        N: Codeunit "NB KV Names";
    begin
        if N.RaiseNamed() <> 'fe' then
            Error('got %1', N.RaiseNamed());
    end;

    [Test]
    procedure PageWithKeywordFieldAndAttributedVars_Compiles()
    var
        P: Page "NB KV Card";
    begin
        if P.Marker() <> 'te' then
            Error('got %1', P.Marker());
    end;

    [Test]
    procedure ReportRequestPageVars_Compile()
    var
        R: Report "NB KV Report";
    begin
        if R.Names() <> 1 then
            Error('got %1', R.Names());
    end;

    [Test]
    procedure XmlportWithElementsAndRequestPageVars_Compiles()
    var
        X: XmlPort "NB KV Xml";
    begin
        Clear(X);
    end;
}
