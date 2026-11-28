// al-runner issue 9111: the Icon the Guided Experience imports from an app's logo (GetExtensionLogo + Icon.ImportStream with the
// app id as the description). Corpus PR 1594 (UsAppLogoJpegSizes.al UsLogo_GuidedExperienceIcon_*) measured its Tenant Media row:
// Height and Width of the picture, Mime Type image/jpeg on Windows BC 27.5 and 29.0, the bytes rewritten in 3 header bytes on
// Windows and unchanged on BC on Linux. G is the 535 x 534 logo of the probe, E the 8 x 8 logo of UsAppLogoJpeg.al.
table 50183 "Logo Icon Holder"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Picture; Media) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

codeunit 50184 "Logo Icon Tests"
{
    Subtype = Test;

    var
        AppGId: Label '9111d000-0000-4000-8000-000000000001', Locked = true;
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

    // Bytes 14, 16 and 18 (1-based) of the stored content: JFIF density units and the low bytes of X and Y density. The files
    // hold 0, 1, 1. Windows BC 27.5 and 29.0 store 1, 0, 0 (corpus PR 1594 at d030ce03, 'icon vs logo media, differing bytes'
    // = '14:1/0 16:0/1 18:0/1 count 3'); BC on Linux 27 and 28 stores a re-encoded icon of another length (corpus CI of 12c6a982); the runner serves Windows.
    local procedure HeaderBytes(var TenantMedia: Record "Tenant Media"): Text
    var
        InS: InStream;
        B: Byte;
        N: Integer;
        I: Integer;
        Result: Text;
    begin
        TenantMedia.CalcFields(Content);
        TenantMedia.Content.CreateInStream(InS);
        while (not InS.EOS()) and (N < 18) do begin
            InS.Read(B);
            N += 1;
            I := B;
            if N in [14, 16, 18] then
                Result += Format(I);
        end;
        exit(Result);
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

    [Test]
    procedure IconG_Size()
    var
        TenantMedia: Record "Tenant Media";
    begin
        FindIcon(AppGId, TenantMedia);
        CheckSize(TenantMedia, 535, 534);
    end;

    [Test]
    procedure IconG_Mime()
    var
        TenantMedia: Record "Tenant Media";
    begin
        FindIcon(AppGId, TenantMedia);
        if TenantMedia."Mime Type" <> 'image/jpeg' then
            Error('mime %1', TenantMedia."Mime Type");
    end;

    // The length is the file's on Windows (corpus PR 1594 'icon length'; BC on Linux 27 and 28 differs); Windows rewrites 3 header bytes in place.
    [Test]
    procedure IconG_ContentLength()
    var
        TenantMedia: Record "Tenant Media";
    begin
        FindIcon(AppGId, TenantMedia);
        if ContentLength(TenantMedia) <> 14191 then
            Error('length');
    end;

    [Test]
    procedure IconG_HeaderBytes()
    var
        TenantMedia: Record "Tenant Media";
    begin
        FindIcon(AppGId, TenantMedia);
        if HeaderBytes(TenantMedia) <> '100' then
            Error('header bytes %1', HeaderBytes(TenantMedia));
    end;

    [Test]
    procedure IconG_ContentHasValue()
    var
        TenantMedia: Record "Tenant Media";
    begin
        FindIcon(AppGId, TenantMedia);
        TenantMedia.CalcFields(Content);
        if not TenantMedia.Content.HasValue() then
            Error('no content');
    end;

    [Test]
    procedure IconG_Description()
    var
        TenantMedia: Record "Tenant Media";
    begin
        FindIcon(AppGId, TenantMedia);
        if TenantMedia.Description <> '{9111D000-0000-4000-8000-000000000001}' then
            Error('description %1', TenantMedia.Description);
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

    // The same call outside the Guided Experience: a two-argument import with a Guid description into an empty Media field.
    [Test]
    procedure DirectImportG_GuidDescription_Size()
    var
        TenantMedia: Record "Tenant Media";
    begin
        ImportLogo(AppGId, true, TenantMedia);
        CheckSize(TenantMedia, 535, 534);
    end;

    // A text description: not the measured call, main's answer (0 x 0) stays.
    [Test]
    procedure DirectImportG_TextDescription_Size()
    var
        TenantMedia: Record "Tenant Media";
    begin
        ImportLogo(AppGId, false, TenantMedia);
        CheckSize(TenantMedia, 0, 0);
    end;
}
