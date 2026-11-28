// al-runner issue 9111: the Published Application logo and its Media row, one dependency app per logo shape. Each test checks
// what Windows BC 27.5 and 29.0 stored for the logos of corpus PR 1594 (UsAppLogoJpegSizes.al): a value, the file's bytes
// unchanged, Mime Type image/jpeg, the picture's size.
codeunit 50160 "Logo Shapes Tests"
{
    Subtype = Test;

    local procedure CheckLogo(AppIdText: Text; Width: Integer; Height: Integer; Length: Integer; Mime: Text)
    var
        App: Record "Published Application";
        Media: Record Media;
        AppId: Guid;
        First: Guid;
        InS: InStream;
        B: Byte;
        N: Integer;
    begin
        Evaluate(AppId, AppIdText);
        App.SetRange(ID, AppId);
        App.SetRange("Tenant Visible", true);
        if not App.FindFirst() then
            Error('no row');
        if not App.Logo.HasValue() then
            Error('no logo');
        First := App.Logo.MediaId();
        App.FindFirst();
        if App.Logo.MediaId() <> First then
            Error('other media id');
        Media.SetRange(ID, First);
        if not Media.FindFirst() then
            Error('no media row');
        if (Mime <> '') and (Media."Mime Type" <> Mime) then
            Error('mime %1', Media."Mime Type");
        if (Media.Width <> Width) or (Media.Height <> Height) then
            Error('size %1 x %2', Media.Width, Media.Height);
        Media.CalcFields(Content);
        Media.Content.CreateInStream(InS);
        while not InS.EOS() do begin
            InS.Read(B);
            N += 1;
        end;
        if N <> Length then
            Error('length %1', N);
    end;

    // The pinned 8 x 8 logo (UsAppLogoJpeg.al): unchanged by the change; its Mime Type differs per version, so not checked.
    [Test]
    procedure Baseline8x8_Pinned()
    begin
        CheckLogo('9111b000-0000-4000-8000-000000000001', 8, 8, 307, '');
    end;

    [Test]
    procedure Baseline16x8_Measured()
    begin
        CheckLogo('9111b000-0000-4000-8000-000000000002', 16, 8, 633, 'image/jpeg');
    end;

    [Test]
    procedure Baseline400x300_Measured()
    begin
        CheckLogo('9111b000-0000-4000-8000-000000000003', 400, 300, 62851, 'image/jpeg');
    end;

    [Test]
    procedure Progressive535x534_Measured()
    begin
        CheckLogo('9111b000-0000-4000-8000-000000000004', 535, 534, 9857, 'image/jpeg');
    end;

    // The measured 535 x 534 logo with a JFIF header of units 1 and 96 x 96 dpi: not measured.
    [Test]
    procedure Baseline535x534_Units1Dpi96_NotMeasured()
    begin
        CheckLogo('9111b000-0000-4000-8000-000000000005', 535, 534, 14191, 'image/jpeg');
    end;

    // A baseline logo of 300 x 300: a size not measured.
    [Test]
    procedure Baseline300x300_NotMeasured()
    begin
        CheckLogo('9111b000-0000-4000-8000-000000000006', 300, 300, 14191, 'image/jpeg');
    end;

    // A progressive logo of 16 x 8: a progressive size not measured.
    [Test]
    procedure Progressive16x8_NotMeasured()
    begin
        CheckLogo('9111b000-0000-4000-8000-000000000007', 16, 8, 9857, 'image/jpeg');
    end;

    // A read of Media.Description of an assumed logo: corpus PR 1594 does not observe it, so head refuses (main refuses the logo).
    [Test]
    procedure DescriptionOfAssumedLogo()
    var
        App: Record "Published Application";
        Media: Record Media;
        AppId: Guid;
    begin
        Evaluate(AppId, '9111b000-0000-4000-8000-000000000002');
        App.SetRange(ID, AppId);
        App.SetRange("Tenant Visible", true);
        App.FindFirst();
        Media.SetRange(ID, App.Logo.MediaId());
        Media.FindFirst();
        if Media.Description <> '' then
            Error('description');
    end;

    // A read of the Name of the row of an app whose logo is assumed: the row read is flagged (the mark is per row).
    [Test]
    procedure NameOfRowWithAssumedLogo()
    var
        App: Record "Published Application";
        AppId: Guid;
    begin
        Evaluate(AppId, '9111b000-0000-4000-8000-000000000002');
        App.SetRange(ID, AppId);
        App.SetRange("Tenant Visible", true);
        App.FindFirst();
        if App.Name <> 'Logo H16' then
            Error('name %1', App.Name);
    end;
}
