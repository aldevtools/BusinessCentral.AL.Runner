// XApp C of al-runner issue 8945 (corpus PR 1567, LsXcLoginSubscribers.al, renumbered): two OnAfterLogin subscribers that
// change XApp A's memos; the first also writes a row of this app, which XApp A's "Global Triggers" subscribers watch.
table 60960 "LS XC Write"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; Tag; Text[30]) { }
    }

    keys { key(PK; "Entry No.") { Clustered = true; } }
}

codeunit 60961 "LS XC First Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        Hub: Codeunit "LS XA Login Hub";
        Many: Codeunit "LS XA State Many";
        NoOwn: Codeunit "LS XA State NoOwn";
        Row: Record "LS XC Write";
    begin
        Hub.Enter('C1');
        Many.Touch('C1', 3);
        NoOwn.Touch('C1');
        if not Hub.CanWrite() then
            exit;
        Row.Init();
        Row.Tag := 'C1';
        Row.Insert(true);
    end;
}

codeunit 60962 "LS XC Second Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure OnAfterLogin()
    var
        Hub: Codeunit "LS XA Login Hub";
    begin
        Hub.Enter('C2');
    end;
}
