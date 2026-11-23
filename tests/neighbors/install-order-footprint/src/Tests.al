codeunit 50930 "Order Probe Tests"
{
    Subtype = Test;

    [Test]
    procedure FeatureRowExists()
    var
        Feature: Record "Order Probe Feature";
    begin
        // A's install inserts it whatever B's subscriber does
        Feature.Get('A-FEATURE');
    end;

    [Test]
    procedure Feature2RowExists()
    var
        Feature: Record "Order Probe Feature 2";
    begin
        Feature.Get('A2-FEATURE');
    end;
}
