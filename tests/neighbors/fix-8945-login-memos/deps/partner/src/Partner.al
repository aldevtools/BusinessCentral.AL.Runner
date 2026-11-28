// Partner app (publisher LM Partner Co): its login subscribers fill the stand-in Microsoft caches.
table 60930 "LMP Tab"
{
    fields { field(1; K; Integer) { } }
    keys { key(PK; K) { Clustered = true; } }
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
