reportextension 50890 "Cross Report Adds Rows" extends "Cross Report Base"
{
    dataset
    {
        addlast(BaseRow)
        {
            dataitem(AddedRow; "Cross Report Number")
            {
                DataItemTableView = where(Number = const(2));
                dataitem(FooterRow; "Cross Report Number")
                {
                    DataItemTableView = where(Number = const(3));

                    trigger OnAfterGetRecord()
                    var
                        Trace: Codeunit "Cross Report Trace";
                    begin
                        Trace.Push('footer');
                    end;
                }
                column(AddedNo; Number) { }

                trigger OnAfterGetRecord()
                var
                    Trace: Codeunit "Cross Report Trace";
                begin
                    Trace.Push('owner=' + Format(Number));
                end;
            }
        }
    }
}
