// The table's own OnInsert/OnDelete xRec after a table extension's OnBefore trigger changed Rec
// (corpus PR 1611, record/events/XebXRecAfterExtBeforeTrigger.al). Only the table's trigger reads xRec here.
table 71850 "N9185 Row"
{
    fields
    {
        field(1; Id; Code[10]) { }
        field(2; Name; Text[30]) { }
        field(3; Tag; Text[30]) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
        key(ByName; Name) { }
    }
    trigger OnInsert()
    begin
        N9185Log.Seen(Rec, xRec);
    end;

    trigger OnDelete()
    begin
        N9185Log.Seen(Rec, xRec);
    end;

    var
        N9185Log: Codeunit "N9185 Log";
}

tableextension 71851 "N9185 Row Ext" extends "N9185 Row"
{
    trigger OnBeforeInsert()
    var
        Cfg: Codeunit "N9185 Log";
    begin
        Cfg.ApplyExt(Rec);
    end;

    trigger OnBeforeDelete()
    var
        Cfg: Codeunit "N9185 Log";
    begin
        Cfg.ApplyExt(Rec);
    end;
}

// A table with two extensions that both declare OnBeforeInsert: not measured.
table 71852 "N9185 Two"
{
    fields
    {
        field(1; Id; Code[10]) { }
        field(2; Name; Text[30]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
    trigger OnInsert()
    var
        N9185Log: Codeunit "N9185 Log";
    begin
        N9185Log.SetText(xRec.Name);
    end;
}

tableextension 71853 "N9185 Two A" extends "N9185 Two"
{
    trigger OnBeforeInsert()
    begin
        Rec.Name := 'EXT';
    end;
}

tableextension 71854 "N9185 Two B" extends "N9185 Two"
{
    trigger OnBeforeInsert()
    begin
    end;
}

// A table whose extension declares no OnBeforeInsert: main's behavior.
table 71855 "N9185 NoBefore"
{
    fields
    {
        field(1; Id; Code[10]) { }
        field(2; Name; Text[30]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
    trigger OnInsert()
    var
        N9185Log: Codeunit "N9185 Log";
    begin
        N9185Log.SetText(xRec.Name);
    end;
}

tableextension 71856 "N9185 NoBefore Ext" extends "N9185 NoBefore"
{
    trigger OnAfterInsert()
    begin
    end;
}

codeunit 71857 "N9185 Log"
{
    SingleInstance = true;

    var
        ExtMode: Integer;
        SubMode: Integer;
        Seen: Text;

    procedure Set(NewExtMode: Integer; NewSubMode: Integer)
    begin
        ExtMode := NewExtMode;
        SubMode := NewSubMode;
        Seen := '';
    end;

    // 0 nothing, 1 Name := 'EXT', 3 SetRange(Name, 'N0'), 4 SetCurrentKey(Name), 5 Get another row, 6 FilterGroup 2 SetRange
    procedure ApplyExt(var Rec: Record "N9185 Row")
    var
        Mode: Integer;
    begin
        Mode := ExtMode;
        case Mode of
            1:
                Rec.Name := 'EXT';
            3:
                Rec.SetRange(Name, 'N0');
            4:
                Rec.SetCurrentKey(Name);
            5:
                Rec.Get('OTHER');
            6:
                begin
                    Rec.FilterGroup(2);
                    Rec.SetRange(Name, 'N0');
                    Rec.FilterGroup(0);
                end;
        end;
    end;

    procedure ApplySub(var Rec: Record "N9185 Row")
    begin
        if SubMode = 1 then
            Rec.Name := 'SUB';
    end;

    procedure ApplySubNoBefore(var Rec: Record "N9185 NoBefore")
    begin
        if SubMode = 1 then
            Rec.Name := 'SUB';
    end;

    procedure Seen(var Rec: Record "N9185 Row"; var xRec: Record "N9185 Row")
    begin
        Seen := StrSubstNo('%1/%2|%3|%4', xRec.Name, Rec.Name, xRec.GetFilters(), xRec.CurrentKey());
    end;

    procedure SetText(Value: Text)
    begin
        Seen := Value;
    end;

    procedure Text(): Text
    begin
        exit(Seen);
    end;

    [EventSubscriber(ObjectType::Table, Database::"N9185 Row", 'OnBeforeInsertEvent', '', false, false)]
    local procedure BeforeInsert(var Rec: Record "N9185 Row"; RunTrigger: Boolean)
    begin
        ApplySub(Rec);
    end;

    [EventSubscriber(ObjectType::Table, Database::"N9185 Row", 'OnBeforeDeleteEvent', '', false, false)]
    local procedure BeforeDelete(var Rec: Record "N9185 Row"; RunTrigger: Boolean)
    begin
        ApplySub(Rec);
    end;

    [EventSubscriber(ObjectType::Table, Database::"N9185 NoBefore", 'OnBeforeInsertEvent', '', false, false)]
    local procedure BeforeInsertNoBefore(var Rec: Record "N9185 NoBefore"; RunTrigger: Boolean)
    begin
        ApplySubNoBefore(Rec);
    end;
}
