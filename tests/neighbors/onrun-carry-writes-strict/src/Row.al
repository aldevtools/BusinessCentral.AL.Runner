table 86900 "OCW Row"
{
    fields
    {
        field(1; "Key"; Code[20]) { }
    }
    keys
    {
        key(PK; "Key") { Clustered = true; }
    }
}

codeunit 86920 "OCW Subscriber"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"OCW Subscriber Write", 'OnHook', '', false, false)]
    local procedure InsertOnHook()
    var
        Row: Record "OCW Row";
    begin
        Row."Key" := 'SUB';
        Row.Insert();
    end;
}

table 86901 "OCW Temp Table Row"
{
    TableType = Temporary;

    fields
    {
        field(1; "Key"; Code[20]) { }
    }
    keys
    {
        key(PK; "Key") { Clustered = true; }
    }
}
