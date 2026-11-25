// al-runner issue 9039: the install-order comparison and the DeleteAll-without-filters mark. These apps have no test
// data (no backing), so the deleted and copied companies are not compared here (Rust tests in al-vm
// baseline/install_order_tests.rs cover the backed shapes).
codeunit 53840 "IO9039 Tests"
{
    Subtype = Test;

    // The emptied mark differs between the orders, the rows do not, and the table is not a platform table: same
    // database, as on main.
    [Test]
    procedure EmptiedInOneOrderSameRows()
    var
        R: Record "IO9039 Emptied Same";
    begin
        R.Get('ROW');
        if R.Value <> 'SAME' then Error('Value %1', R.Value);
    end;

    // The DeleteAll leaves different rows in each order: the orders differ, as on main.
    [Test]
    procedure EmptiedInOneOrderDifferentRows()
    var
        R: Record "IO9039 Emptied Differs";
    begin
        if R.IsEmpty() then Error('empty');
    end;
}
