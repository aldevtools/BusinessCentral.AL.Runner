BitmapPictureNotDrawn => {
    id: "bitmap-picture-not-drawn",
    assumed: "O365 Brand Color.MakePicture (new Bitmap(100, 100), Graphics.FromImage, ColorTranslator.FromHtml, new SolidBrush, FillEllipse, Graphics.Dispose, Bitmap.Save(InStream, ImageFormat.Png), Bitmap.Dispose) runs without error on real BC, draws nothing here, and Media.ImportStream of the saved stream gives a field with a value (HasValue true); AL that looks at the picture (stream read, length, export, Tenant Media row) is refused (ADR-044).",
    tests: "O365BrandColorTests.Codeunit.al:74 and :158 (Sample Picture.HasValue after MakePicture), and every Contoso Coffee Demo Dataset step that reaches Create Company Information.CreateSetupCompany (O365 Brand Color.CreateDefaultBrandColors).",
    probe: "On a Windows tier: new Bitmap(100, 100), FillEllipse, Save(InStream of a Temp Blob, ImageFormat.Png), Media.ImportStream: HasValue, then ExportStream length and its first 8 bytes (PNG signature 89 50 4E 47 0D 0A 1A 0A); the PNG header width and height (expect 100 by 100) and the pixel at the center versus a corner; Save with Jpeg and Bmp.",
}
