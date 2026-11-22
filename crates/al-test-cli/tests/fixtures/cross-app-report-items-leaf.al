reportextension 50880 "Cross Report Uses Added Rows" extends "Cross Report Base"
{
    dataset
    {
        modify(AddedRow)
        {
            trigger OnAfterAfterGetRecord()
            var
                Trace: Codeunit "Cross Report Trace";
            begin
                Trace.Push('dependent=' + Format(AddedRow.Number));
            end;
        }
        add(AddedRow)
        {
            column(DependentNo; AddedRow.Number) { }
        }
        addbefore(FooterRow)
        {
            dataitem(BeforeFooter; "Cross Report Number")
            {
                DataItemTableView = where(Number = const(4));

                trigger OnAfterGetRecord()
                var
                    Trace: Codeunit "Cross Report Trace";
                begin
                    Trace.Push('before');
                end;
            }
        }
        addafter(FooterRow)
        {
            dataitem(AfterFooter; "Cross Report Number")
            {
                DataItemLinkReference = AddedRow;
                DataItemLink = Number = field(Number);

                trigger OnAfterGetRecord()
                var
                    Trace: Codeunit "Cross Report Trace";
                begin
                    Trace.Push('after=' + Format(Number));
                end;
            }
        }
    }
}
