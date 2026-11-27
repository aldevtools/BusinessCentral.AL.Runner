// Its own codeunit: a refusal in codeunit Tests stops the tests after it there.
codeunit 71861 TestsNoRead
{
    Subtype = Test;

    var
        Log: Codeunit "N9185 Log";

    // Not measured: Delete(true) on a record that was never read (RunDelete at XebXRecAfterExtBeforeTrigger.al:268-290 Gets
    // the row first). Main's behavior: xRec is the buffer after the subscribers.
    [Test]
    procedure Neighbor_DeleteWithoutRead()
    var
        Other: Record "N9185 Row";
        R: Record "N9185 Row";
    begin
        Other.DeleteAll();
        Other.Init();
        Other.Id := 'A';
        Other.Name := 'N0';
        Log.Set(0, 0);
        Other.Insert(false);
        R.Id := 'A';
        R.Name := 'BUF';
        Log.Set(1, 1);
        R.Delete(true);
        if Log.Text() <> 'SUB/EXT||Id' then
            Error('Expected SUB/EXT||Id, got %1', Log.Text());
    end;
}
