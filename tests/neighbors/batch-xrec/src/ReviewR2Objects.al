// Review round 2 of runner PR 9328: the cell's shape (corpus XRecTableProcThroughSubscriber.al:1042-1050: the trigger of
// "Item No." nests Validate(Mode) through two table procedures) on tables of their own, each with one more piece of code that
// runs while both Validates run and touches xRec: a table extension trigger of either field, or a validate event subscriber.
// The cell has no table extension trigger on either field and no validate event subscriber of "Item No.".

table 50040 "NBX HookIn Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; Mode; Option)
        {
            OptionMembers = Purchase,Production;

            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('T x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
            end;
        }
        field(3; Series; Code[20]) { }
        field(4; "Item No."; Code[20])
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('I x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
                Series := 'MID';
                CopyFromItem();
                Cfg.Add('I after x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
            end;
        }
    }

    keys
    {
        key(PK; "Line No.") { Clustered = true; }
    }

    local procedure CopyFromItem()
    begin
        UpdateMode();
    end;

    local procedure UpdateMode()
    begin
        Validate(Mode, Mode::Production);
    end;
}

tableextension 50041 "NBX HookIn Ext" extends "NBX HookIn Line"
{
    fields
    {
        modify(Mode)
        {
            trigger OnAfterValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() <> '' then
                    xRec.Series := 'HOOK';
            end;
        }
    }
}

table 50042 "NBX HookOut Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; Mode; Option)
        {
            OptionMembers = Purchase,Production;

            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('T x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
            end;
        }
        field(3; Series; Code[20]) { }
        field(4; "Item No."; Code[20])
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('I x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
                Series := 'MID';
                CopyFromItem();
                Cfg.Add('I after x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
            end;
        }
    }

    keys
    {
        key(PK; "Line No.") { Clustered = true; }
    }

    local procedure CopyFromItem()
    begin
        UpdateMode();
    end;

    local procedure UpdateMode()
    begin
        Validate(Mode, Mode::Production);
    end;
}

tableextension 50043 "NBX HookOut Ext" extends "NBX HookOut Line"
{
    fields
    {
        modify("Item No.")
        {
            trigger OnBeforeValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() <> '' then
                    xRec.Series := 'HOOK';
            end;
        }
    }
}

table 50044 "NBX SubOut Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; Mode; Option)
        {
            OptionMembers = Purchase,Production;

            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('T x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
            end;
        }
        field(3; Series; Code[20]) { }
        field(4; "Item No."; Code[20])
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('I x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
                Series := 'MID';
                CopyFromItem();
                Cfg.Add('I after x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
            end;
        }
    }

    keys
    {
        key(PK; "Line No.") { Clustered = true; }
    }

    local procedure CopyFromItem()
    begin
        UpdateMode();
    end;

    local procedure UpdateMode()
    begin
        Validate(Mode, Mode::Production);
    end;
}

codeunit 50045 "NBX SubOut Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"NBX SubOut Line", 'OnBeforeValidateEvent', 'Item No.', false, false)]
    local procedure OnEvent(var Rec: Record "NBX SubOut Line"; var xRec: Record "NBX SubOut Line"; CurrFieldNo: Integer)
    var
        Cfg: Codeunit "NBX Cfg";
    begin
        if Cfg.Get() <> '' then
            xRec.Series := 'SUB';
    end;
}

table 50046 "NBX SubIn Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; Mode; Option)
        {
            OptionMembers = Purchase,Production;

            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('T x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
            end;
        }
        field(3; Series; Code[20]) { }
        field(4; "Item No."; Code[20])
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('I x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
                Series := 'MID';
                CopyFromItem();
                Cfg.Add('I after x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
            end;
        }
    }

    keys
    {
        key(PK; "Line No.") { Clustered = true; }
    }

    local procedure CopyFromItem()
    begin
        UpdateMode();
    end;

    local procedure UpdateMode()
    begin
        Validate(Mode, Mode::Production);
    end;
}

codeunit 50047 "NBX SubIn Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"NBX SubIn Line", 'OnAfterValidateEvent', 'Mode', false, false)]
    local procedure OnEvent(var Rec: Record "NBX SubIn Line"; var xRec: Record "NBX SubIn Line"; CurrFieldNo: Integer)
    var
        Cfg: Codeunit "NBX Cfg";
    begin
        if Cfg.Get() <> '' then
            xRec.Series := 'SUB';
    end;
}

table 50048 "NBX SubInRead Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; Mode; Option)
        {
            OptionMembers = Purchase,Production;

            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('T x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
            end;
        }
        field(3; Series; Code[20]) { }
        field(4; "Item No."; Code[20])
        {
            trigger OnValidate()
            var
                Cfg: Codeunit "NBX Cfg";
            begin
                if Cfg.Get() = '' then
                    exit;
                Cfg.Add('I x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
                Series := 'MID';
                xRec.Series := 'OUT';
                CopyFromItem();
                Cfg.Add('I after x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
            end;
        }
    }

    keys
    {
        key(PK; "Line No.") { Clustered = true; }
    }

    local procedure CopyFromItem()
    begin
        UpdateMode();
    end;

    local procedure UpdateMode()
    begin
        Validate(Mode, Mode::Production);
    end;
}

codeunit 50049 "NBX SubInRead Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"NBX SubInRead Line", 'OnBeforeValidateEvent', 'Mode', false, false)]
    local procedure OnEvent(var Rec: Record "NBX SubInRead Line"; var xRec: Record "NBX SubInRead Line"; CurrFieldNo: Integer)
    var
        Cfg: Codeunit "NBX Cfg";
    begin
        if Cfg.Get() <> '' then
            Cfg.Add('S x=' + Format(xRec.Mode, 0, 2) + '/' + xRec.Series + '/' + xRec."Item No.");
    end;
}
