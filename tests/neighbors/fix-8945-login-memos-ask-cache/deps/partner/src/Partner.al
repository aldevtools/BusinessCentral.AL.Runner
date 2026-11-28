// Partner app (publisher LS Partner Co): two login subscribers write its tables, so the test session asks their setup
// in the login; SingleInstance caches change in those asks, each in one neighbor shape of the setup-ask cache gate.
table 60970 "LSP Tab"
{
    fields { field(1; K; Integer) { } }
    keys { key(PK; K) { Clustered = true; } }
}

table 60973 "LSP Tab2"
{
    fields { field(1; K; Integer) { } }
    keys { key(PK; K) { Clustered = true; } }
}

table 60974 "LSP Tab3"
{
    fields { field(1; K; Integer) { } }
    keys { key(PK; K) { Clustered = true; } }
}

// First login subscriber: writes LSP Tab and bumps the mixed cache outside any raise.
codeunit 60971 "LSP Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure Login()
    var
        T: Record "LSP Tab";
        M: Codeunit "LSP Mixed Cache";
    begin
        T.K := 1;
        T.Insert();
        M.Bump();
    end;
}

// Second login subscriber: writes LSP Tab2 and bumps the later cache, which the first one's ask filled.
codeunit 60975 "LSP Login Two"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure Login()
    var
        T: Record "LSP Tab2";
        L: Codeunit "LSP Later Cache";
    begin
        T.K := 1;
        T.Insert();
        L.Bump();
    end;
}

// Filled by its own setup subscriber only.
codeunit 60972 "LSP Setup Cache"
{
    SingleInstance = true;
    var
        LastAsk: Code[20];

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId = Database::"LSP Tab" then
            LastAsk := 'ASKED';
    end;

    procedure Last(): Code[20]
    begin
        exit(LastAsk);
    end;
}

// Counts the asks of both login tables.
codeunit 60979 "LSP Count Cache"
{
    SingleInstance = true;
    var
        Asks: Integer;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId in [Database::"LSP Tab", Database::"LSP Tab2"] then
            Asks += 1;
    end;

    procedure Count(): Integer
    begin
        exit(Asks);
    end;
}

// No subscriber of its own: the setup subscriber of LSP Ask Sub, same app, fills it.
codeunit 60976 "LSP Side Cache"
{
    SingleInstance = true;
    var
        Notes: Integer;

    procedure Note()
    begin
        Notes += 1;
    end;

    procedure Count(): Integer
    begin
        exit(Notes);
    end;
}

codeunit 60977 "LSP Ask Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    var
        S: Codeunit "LSP Side Cache";
    begin
        if TableId <> Database::"LSP Tab" then
            exit;
        S.Note();
        OnAsked();
    end;

    [IntegrationEvent(false, false)]
    procedure OnAsked()
    begin
    end;
}

// Changed by a subscriber of another event (LSP Ask Sub's OnAsked) that the ask raises.
codeunit 60978 "LSP Event Cache"
{
    SingleInstance = true;
    var
        N: Integer;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LSP Ask Sub", 'OnAsked', '', false, false)]
    local procedure Asked()
    begin
        N += 1;
    end;

    procedure Count(): Integer
    begin
        exit(N);
    end;
}

// Changed in its setup ask and by the first login subscriber outside any raise.
codeunit 60990 "LSP Mixed Cache"
{
    SingleInstance = true;
    var
        N: Integer;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId = Database::"LSP Tab" then
            N += 1;
    end;

    procedure Bump()
    begin
        N += 10;
    end;

    procedure Count(): Integer
    begin
        exit(N);
    end;
}

// Filled in the ask of LSP Tab (first login subscriber), then bumped by the second login subscriber.
codeunit 60991 "LSP Later Cache"
{
    SingleInstance = true;
    var
        N: Integer;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId = Database::"LSP Tab" then
            N += 1;
    end;

    procedure Bump()
    begin
        N += 10;
    end;

    procedure Count(): Integer
    begin
        exit(N);
    end;
}

// Changed in the ask of LSP Tab3, which the insert event of LSP Tab writes: an ask nested in an insert raise.
codeunit 60992 "LSP Nested Cache"
{
    SingleInstance = true;
    var
        Hit: Boolean;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId = Database::"LSP Tab3" then
            Hit := true;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure Inserted(RecRef: RecordRef)
    var
        T3: Record "LSP Tab3";
    begin
        if RecRef.Number <> Database::"LSP Tab" then
            exit;
        T3.K := 1;
        if T3.Insert() then;
    end;

    procedure Seen(): Boolean
    begin
        exit(Hit);
    end;
}

// Changed in the ask of LSP Tab by a setup subscriber of the app under test (LS Foreign Sub).
codeunit 60993 "LSP Foreign Cache"
{
    SingleInstance = true;
    var
        N: Integer;

    procedure Bump()
    begin
        N += 1;
    end;

    procedure Count(): Integer
    begin
        exit(N);
    end;
}
