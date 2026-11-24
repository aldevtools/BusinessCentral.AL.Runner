// XApp B of al-runner issue 8945 (corpus PR 1567, LsXbLoginSubscribers.al, renumbered): two OnAfterLogin subscribers that
// change XApp A's memos; the first also writes a row of this app, which XApp A's "Global Triggers" subscribers watch.
table 60950 "LS XB Write"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; Tag; Text[30]) { }
    }

    keys { key(PK; "Entry No.") { Clustered = true; } }
}

codeunit 60951 "LS XB First Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        Hub: Codeunit "LS XA Login Hub";
        Many: Codeunit "LS XA State Many";
        NoOwn: Codeunit "LS XA State NoOwn";
        Row: Record "LS XB Write";
    begin
        Hub.Enter('B1');
        Many.Touch('B1', 2);
        NoOwn.Touch('B1');
        if not Hub.CanWrite() then
            exit;
        Row.Init();
        Row.Tag := 'B1';
        Row.Insert(true);
    end;
}

codeunit 60952 "LS XB Second Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        Hub.Enter('B2');
    end;
}
