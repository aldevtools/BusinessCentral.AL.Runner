// Review of al-runner PR 9098 (MED-1): a company copy carries the system fields of the rows it copies. This folder runs
// with test data (the default), so the copy reads the source's test data rows; the folder fix-9039-copy-sysfields-no-test-data
// runs the same apps without (flags.txt). The store-level shapes are Rust tests in al-vm baseline/install_order_tests.rs.
// Through install code, main and head both find that the orders differ in Company (the row CopyCompany inserts gets an
// Id of its own in each order) and in IO9039S Setup (X's row in the copy is in one order only). The three tests that read
// those tables are not run on either side. Main refused the whole install; head names the tables, so the fourth test,
// which reads none of them, runs.
codeunit 53865 "IO9039S Tests"
{
    Subtype = Test;

    // X's setup row in the copy has the SystemId of the source's row when Y copied after X, a SystemId of its own when Y
    // copied before X. The other fields match in both orders.
    [Test]
    procedure CopiedBeforeOrAfterInsertSystemIdCompared()
    var
        Source: Record "IO9039S Setup";
        Copy: Record "IO9039S Setup";
    begin
        Source.Get('ROW');
        Copy.ChangeCompany('IO9039S COPY');
        Copy.Get('ROW');
        if Copy.SystemId <> Source.SystemId then Error('the copy has a SystemId of its own');
    end;

    // Y's own row is written before the copy in both orders: the copy holds it in both. Only the other field is read.
    [Test]
    procedure CopiedAlikeInBothOrdersValueRead()
    var
        Copy: Record "IO9039S Copied Alike";
    begin
        Copy.ChangeCompany('IO9039S COPY');
        Copy.Get('ROW');
        if Copy.Value <> 'ALIKE' then Error('Value %1', Copy.Value);
    end;

    // The source company's setup row: X inserts it in both orders, with a SystemId of its own each time.
    [Test]
    procedure SourceCompanyRowValueRead()
    var
        Source: Record "IO9039S Setup";
    begin
        Source.Get('ROW');
        if Source.Value <> 'SAME' then Error('Value %1', Source.Value);
    end;

    // Reads no table the orders wrote: it runs when the comparison names the tables that differ.
    [Test]
    procedure NoTableOfTheOrdersRead()
    begin
        if CompanyName() = '' then Error('no company');
    end;
}
