// A neighbor of al-runner issue 8637 past BQ Tests 9b and 9c, apart from Neighbors.al because the runner's unit tests compile
// that file without the platform tables: @ alone on the Text field Object Caption of the platform table AllObjWithCaption,
// whether FindFirst finds a row and whether Find('=') finds it again (BC: Yes|Yes).
codeunit 97534 "BQ Gate Platform Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure AtSignAlone_PlatformTable_FindsTheRowAgain()
    var
        Objects: Record AllObjWithCaption;
        Found: Boolean;
        Again: Boolean;
    begin
        Objects.SetFilter("Object Caption", '@');
        Found := Objects.FindFirst();
        if Found then
            Again := Objects.Find('=');
        Assert.AreEqual('Yes|Yes', Format(Found) + '|' + Format(Again), 'FindFirst, Find(=)');
    end;
}
