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

    // A count of the image/jpeg Tenant Media rows (the shape of corpus UsLogo_Jpeg_GuidedExperienceIcon_AppE, which discards it):
    // the icon of the 8 x 8 logo is one of them on head, and the count is flagged.
    [Test]
    procedure CountOfJpegTenantMedia()
    var
        TenantMedia: Record "Tenant Media";
    begin
        TenantMedia.SetRange("Mime Type", 'image/jpeg');
        if TenantMedia.Count() < 1 then
            Error('count %1', TenantMedia.Count());
    end;

    // A user import of the 16 x 8 logo file of corpus PR 1594 (not an app logo), two arguments with a Guid description, into an
    // empty Media field: the measured call, so head gives the measured size and refuses a look at the content.
    local procedure ImportH16x8File(No: Integer; var TenantMedia: Record "Tenant Media")
    var
        Holder: Record "Logo Icon Holder";
        TempBlob: Codeunit "Temp Blob";
        Base64: Codeunit "Base64 Convert";
        OutS: OutStream;
        InS: InStream;
        B64: Text;
    begin
        B64 += '/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAx';
        B64 += 'NDQ0Hyc5PTgyPC4zNDL/2wBDAQkJCQwLDBgNDRgyIRwhMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIy';
        B64 += 'MjIyMjIyMjL/wAARCAAIABADASIAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUF';
        B64 += 'BAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVW';
        B64 += 'V1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi';
        B64 += '4+Tl5ufo6erx8vP09fb3+Pn6/8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/8QAtREAAgECBAQDBAcFBAQAAQJ3AAEC';
        B64 += 'AxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVm';
        B64 += 'Z2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq';
        B64 += '8vP09fb3+Pn6/9oADAMBAAIRAxEAPwDkKKKK8U/TD//Z';
        TempBlob.CreateOutStream(OutS);
        Base64.FromBase64(B64, OutS);
        TempBlob.CreateInStream(InS);
        Holder."No." := No;
        Holder.Picture.ImportStream(InS, CreateGuid());
        Holder.Insert();
        TenantMedia.SetRange(ID, Holder.Picture.MediaId());
        TenantMedia.FindFirst();
    end;

    [Test]
    procedure DirectImportH16x8File_Size()
    var
        TenantMedia: Record "Tenant Media";
    begin
        ImportH16x8File(3, TenantMedia);
        CheckSize(TenantMedia, 16, 8);
    end;

    // Main answers the file's bytes; head refuses (Windows rewrites 3 header bytes).
    [Test]
    procedure DirectImportH16x8File_ContentLength()
    var
        TenantMedia: Record "Tenant Media";
    begin
        ImportH16x8File(4, TenantMedia);
        if ContentLength(TenantMedia) <> 633 then
            Error('length');
    end;
}
