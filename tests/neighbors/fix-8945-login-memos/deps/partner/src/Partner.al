// Partner app (publisher LM Partner Co): its login subscribers fill the stand-in Microsoft caches and load-once memos
// reached through the wrapper's events, which pass on GlobalTriggerManagement's setup ask and insert events and do nothing
// else (the probe's wrapper). Each memo below differs from the measured one ("LMP Memo Measured", the shape of
// cell L1 of corpus PR 1571) in one fact; its name says which.
table 60930 "LMP Tab"
{
    fields { field(1; K; Integer) { } }
    keys { key(PK; K) { Clustered = true; } }
}

table 60931 "LMP Setup"
{
    fields { field(1; "Table Id"; Integer) { } }
    keys { key(PK; "Table Id") { Clustered = true; } }
}

// The real table memo 60975 writes: its own, so the other memos' setup table stays empty, as measured.
table 60976 "LMP Real"
{
    fields { field(1; "Table Id"; Integer) { } }
    keys { key(PK; "Table Id") { Clustered = true; } }
}

table 60932 "LMP Sub"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Company Name"; Text[30]) { }
    }
    keys { key(PK; "Code", "Company Name") { Clustered = true; } }
}

codeunit 60933 "LMP Wrapper"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::GlobalTriggerManagement, 'OnAfterGetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        OnWrapperSetup(TableId);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::GlobalTriggerManagement, 'OnAfterOnDatabaseInsert', '', false, false)]
    local procedure DatabaseInsert(RecRef: RecordRef)
    begin
        OnWrapperWrite(RecRef.Number);
    end;

    [IntegrationEvent(false, false)]
    procedure OnWrapperSetup(TableId: Integer)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnWrapperWrite(TableId: Integer)
    begin
    end;
}

// The first login subscriber: one write of the partner table (setup ask and insert event), the direct fills, IsSandbox().
codeunit 60934 "LMP Login First"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure Login()
    var
        T: Record "LMP Tab";
        E: Codeunit "Environment Information Impl.";
        P: Codeunit "Conf./Personalization Mgt.";
        A: Codeunit "Application Area Cache";
    begin
        T.K := 1;
        T.Insert();
        P.Fetch();
        A.Fill();
        E.IsSandbox();
    end;
}

// The second login subscriber: IsSaaS().
codeunit 60935 "LMP Login Second"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure Login()
    var
        E: Codeunit "Environment Information Impl.";
    begin
        E.IsSaaS();
    end;
}


codeunit 60940 "LMP Memo Measured"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        
        if not Loaded then begin
            Loaded := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    
}


codeunit 60941 "LMP Memo Second Filter"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        
        if not Loaded then begin
            Loaded := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        TempSub.SetFilter(Code, '<>%1', 'X');
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    
}


codeunit 60942 "LMP Memo Filter Not Blank"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        
        if not Loaded then begin
            Loaded := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', 'X');
        
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    
}


codeunit 60943 "LMP Memo Row Written"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        
        if not Loaded then begin
            Loaded := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        TempSub.Code := 'A'; TempSub.Insert();
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    
}


codeunit 60944 "LMP Memo Decimal Global"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        Rate: Decimal;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        
        if not Loaded then begin
            Loaded := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    
}


codeunit 60945 "LMP Memo Publisher"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        
        if not Loaded then begin
            Loaded := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    [IntegrationEvent(false, false)] procedure OnLoaded() begin end;
}


codeunit 60946 "LMP Memo Public Writer"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        
        if not Loaded then begin
            Loaded := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    procedure Unload() begin Loaded := true; end;
}


codeunit 60947 "LMP Memo Company Guard"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        if CompanyName() = 'X' then exit;
        if not Loaded then begin
            Loaded := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    
}


codeunit 60948 "LMP Memo Asked Table"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        if not Loaded then begin
            Loaded := true;
            LoadTable := TableId;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    
}


codeunit 60949 "LMP Memo No Ask Change"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        
        if not Loaded then begin
            Loaded := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    
}


codeunit 60938 "LMP Memo Second Insert Flag"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        Extra: Boolean;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        if not Loaded then begin
            Loaded := true;
            Extra := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    
}


codeunit 60939 "LMP Memo Flag Cleared"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        
        if not Loaded then begin
            Loaded := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    local procedure Reset() begin Loaded := false; end;
}

// Review round 1 neighbors: a guard in the wrapper (temporary record, company name), a public writer of a field of a
// temporary global, and an insert into a real table with its trigger in the load.
codeunit 60970 "LMP Wrapper Temp Guard"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::GlobalTriggerManagement, 'OnAfterGetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        OnWrapperSetup(TableId);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::GlobalTriggerManagement, 'OnAfterOnDatabaseInsert', '', false, false)]
    local procedure DatabaseInsert(RecRef: RecordRef)
    begin
        if RecRef.IsTemporary then
            exit;
        OnWrapperWrite(RecRef.Number);
    end;

    [IntegrationEvent(false, false)]
    procedure OnWrapperSetup(TableId: Integer)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnWrapperWrite(TableId: Integer)
    begin
    end;
}

codeunit 60971 "LMP Memo Behind Temp Guard"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper Temp Guard", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper Temp Guard", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        
        if not Loaded then begin
            Loaded := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    
}

codeunit 60972 "LMP Wrapper Company Guard"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::GlobalTriggerManagement, 'OnAfterGetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if CompanyName() = 'X' then
            exit;
        OnWrapperSetup(TableId);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::GlobalTriggerManagement, 'OnAfterOnDatabaseInsert', '', false, false)]
    local procedure DatabaseInsert(RecRef: RecordRef)
    begin
        OnWrapperWrite(RecRef.Number);
    end;

    [IntegrationEvent(false, false)]
    procedure OnWrapperSetup(TableId: Integer)
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnWrapperWrite(TableId: Integer)
    begin
    end;
}

codeunit 60973 "LMP Memo Behind Company Guard"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper Company Guard", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper Company Guard", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        
        if not Loaded then begin
            Loaded := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    
}

codeunit 60974 "LMP Memo Field Writer"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        
        if not Loaded then begin
            Loaded := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        
    end;

    procedure SetSub()
    begin
        TempSub."Company Name" := 'X';
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    
}

codeunit 60975 "LMP Memo Real Insert"
{
    SingleInstance = true;

    var
        TempSetup: Record "LMP Setup" temporary;
        TempSub: Record "LMP Sub" temporary;
        Checked: Boolean;
        Activated: Boolean;
        Loaded: Boolean;
        LoadTable: Integer;
        

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperSetup', '', false, false)]
    local procedure WrapperSetup(TableId: Integer)
    var
        Setup: Record "LMP Setup";
    begin
        if not Checked then begin Checked := true; Activated := not Setup.IsEmpty(); end;
        if not Activated then
            exit;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LMP Wrapper", 'OnWrapperWrite', '', false, false)]
    local procedure WrapperWrite(TableId: Integer)
    begin
        
        if not Loaded then begin
            Loaded := true;
            LoadTemps();
        end;
        if not TempSetup.Get(TableId) then
            exit;
    end;

    local procedure LoadTemps()
    var
        Setup: Record "LMP Setup";
        Real: Record "LMP Real";
    begin
        TempSetup.DeleteAll();
        if Setup.FindSet() then
            repeat
                TempSetup := Setup;
                TempSetup.Insert();
            until Setup.Next() = 0;
        Real."Table Id" := 7;
        Real.Insert(true);
        Clear(TempSub);
        TempSub.SetFilter("Company Name", '=%1', '');
        
    end;

    procedure IsLoaded(): Boolean
    begin
        exit(Loaded);
    end;
    
}

