codeunit 42 "Caption Class" { [IntegrationEvent(false, false)] procedure OnResolveCaptionClass(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) begin end; }

table 50320 "Probe Literal Caption Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Payload; Text[20]) { CaptionClass = 'PRB503,Literal'; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50321 "Probe Literal Caption Page"
{
    PageType = Card;
    SourceTable = "Probe Literal Caption Row";
    layout
    {
        area(Content)
        {
            group(General)
            {
                field(Payload; Rec.Payload) { ApplicationArea = All; }
                field(EditValue; EditValue) { ApplicationArea = All; }
            }
        }
    }
    var
        EditValue: Text[20];
}

codeunit 50322 "Probe Caption Subscriber"
{
    EventSubscriberInstance = Manual;
    var
        Calls: Integer;

    procedure Count(): Integer
    begin
        exit(Calls);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnResolveCaptionClass', '', false, false)]
    local procedure Resolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    begin
        if CaptionArea <> 'PRB503' then
            exit;
        Calls += 1;
        Caption := CaptionExpr + Format(Calls);
        Resolved := true;
    end;
}

codeunit 50323 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure LiteralFieldCaptionReadsUseOneResolution()
    var
        Row: Record "Probe Literal Caption Row";
        Pg: TestPage "Probe Literal Caption Page";
        Subscriber: Codeunit "Probe Caption Subscriber";
        AtOpen: Integer;
        FirstCaption: Text;
        SecondCaption: Text;
        ThirdCaption: Text;
    begin
        Row.Id := 1;
        Row.Insert();
        BindSubscription(Subscriber);
        Pg.OpenEdit();
        AtOpen := Subscriber.Count();
        FirstCaption := Pg.Payload.Caption();
        SecondCaption := Pg.Payload.Caption();
        Pg.EditValue.SetValue('X');
        ThirdCaption := Pg.Payload.Caption();
        if (FirstCaption <> 'Literal1') or (SecondCaption <> FirstCaption) or (ThirdCaption <> FirstCaption) or (Subscriber.Count() <> 1) then
            Error('Caption observation: %1|%2|%3|%4|%5', AtOpen, FirstCaption, SecondCaption, ThirdCaption, Subscriber.Count());
        Pg.Close();
        UnbindSubscription(Subscriber);
    end;
}
