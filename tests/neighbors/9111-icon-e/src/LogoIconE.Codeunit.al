// al-runner issue 9111: only the 8 x 8 logo app of 9111-icon, so main answers it (in 9111-icon the 535 x 534 logo refuses the
// whole run on main). BC gives the icon of the 8 x 8 logo 8 x 8 (corpus PR 1594, UsLogo_GuidedExperienceIcon_Jpeg8x8_AppE).
table 50183 "Logo Icon Holder"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Picture; Media) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

codeunit 50184 "Logo Icon E Tests"
{
    Subtype = Test;

    var
        AppEId: Label '9111d000-0000-4000-8000-000000000002', Locked = true;

    // The Tenant Media row of the Icon of the app's Guided Experience Item, after SetupForExtensionExists registered it.
    local procedure FindIcon(AppIdText: Text; var TenantMedia: Record "Tenant Media")
    var
        GuidedExperience: Codeunit "Guided Experience";
        Item: RecordRef;
        Id: Guid;
        IconId: Guid;
    begin
        Evaluate(Id, AppIdText);
        GuidedExperience.SetupForExtensionExists(Id);
        Item.Open(1990); // Guided Experience Item
        Item.Field(10).SetRange(Id); // Extension ID
        if not Item.FindFirst() then
            Error('no item');
        Evaluate(IconId, Format(Item.Field(20).Value())); // Icon
        Item.Close();
        TenantMedia.SetRange(ID, IconId);
        if not TenantMedia.FindFirst() then
            Error('no tenant media row');
    end;

    local procedure ContentLength(var TenantMedia: Record "Tenant Media"): Integer
    var
        InS: InStream;
        B: Byte;
        N: Integer;
    begin
        TenantMedia.CalcFields(Content);
        TenantMedia.Content.CreateInStream(InS);
        while not InS.EOS() do begin
            InS.Read(B);
            N += 1;
        end;
        exit(N);
    end;

    local procedure CheckSize(var TenantMedia: Record "Tenant Media"; Width: Integer; Height: Integer)
    begin
        if (TenantMedia.Width <> Width) or (TenantMedia.Height <> Height) then
            Error('size %1 x %2', TenantMedia.Width, TenantMedia.Height);
    end;

    // Imports the bytes of an app's logo Media row into a Media field of a new row, with the app id (a Guid) or a text as the
    // description.
    local procedure ImportLogo(AppIdText: Text; GuidDescription: Boolean; var TenantMedia: Record "Tenant Media")
    var
        App: Record "Published Application";
        Media: Record Media;
        Holder: Record "Logo Icon Holder";
        InS: InStream;
        Id: Guid;
    begin
        Evaluate(Id, AppIdText);
        App.SetRange(ID, Id);
        App.SetRange("Tenant Visible", true);
        App.FindFirst();
        Media.SetRange(ID, App.Logo.MediaId());
        Media.FindFirst();
        Media.CalcFields(Content);
        Media.Content.CreateInStream(InS);
        // One row per call: the tests of a codeunit share the database.
        if GuidDescription then
            Holder."No." := 1
        else
            Holder."No." := 2;
        if GuidDescription then
            Holder.Picture.ImportStream(InS, Id)
        else
            Holder.Picture.ImportStream(InS, 'a logo');
        Holder.Insert();
        TenantMedia.SetRange(ID, Holder.Picture.MediaId());
        TenantMedia.FindFirst();
    end;

    // The control: the 8 x 8 logo. Main answers 0 x 0 for its icon; BC gives 8 x 8 (UsLogo_GuidedExperienceIcon_Jpeg8x8_AppE).
    [Test]
    procedure IconE_Size()
    var
        TenantMedia: Record "Tenant Media";
    begin
        FindIcon(AppEId, TenantMedia);
        CheckSize(TenantMedia, 8, 8);
    end;

    [Test]
    procedure IconE_Mime()
    var
        TenantMedia: Record "Tenant Media";
    begin
        FindIcon(AppEId, TenantMedia);
        if TenantMedia."Mime Type" <> 'image/jpeg' then
            Error('mime %1', TenantMedia."Mime Type");
    end;

    // Main's answer for the 8 x 8 icon's content (the file's bytes) stays.
    [Test]
    procedure IconE_ContentLength()
    var
        TenantMedia: Record "Tenant Media";
    begin
        FindIcon(AppEId, TenantMedia);
        if ContentLength(TenantMedia) <> 307 then
            Error('length');
    end;

    // A user import of the 8 x 8 logo's bytes, two arguments with a Guid description, outside the Guided Experience.
    [Test]
    procedure DirectImportE_GuidDescription_Size()
    var
        TenantMedia: Record "Tenant Media";
    begin
        ImportLogo(AppEId, true, TenantMedia);
        CheckSize(TenantMedia, 8, 8);
    end;

    // A text description: not the measured call, main's answer (0 x 0) stays.
    [Test]
    procedure DirectImportE_TextDescription_Size()
    var
        TenantMedia: Record "Tenant Media";
    begin
        ImportLogo(AppEId, false, TenantMedia);
        CheckSize(TenantMedia, 0, 0);
    end;
}
