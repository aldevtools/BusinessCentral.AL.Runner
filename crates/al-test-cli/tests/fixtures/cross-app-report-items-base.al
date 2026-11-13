table 50850 "Cross Report Number"
{
    fields { field(1; Number; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Number) { } }
}

codeunit 50850 "Cross Report Trace"
{
    SingleInstance = true;
    var
        Events: Text;

    procedure Push(Value: Text)
    begin
        Events += Value + '|';
    end;

    procedure Read(): Text
    begin
        exit(Events);
    end;
}

report 50851 "Cross Report Base"
{
    ProcessingOnly = false;
    UseRequestPage = false;

    dataset
    {
        dataitem(BaseRow; "Cross Report Number")
        {
            DataItemTableView = where(Number = const(1));
            column(BaseNo; Number) { }
        }
    }
}
