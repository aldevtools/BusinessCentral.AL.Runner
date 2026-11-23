// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-singleinstance-property
// Scope: in-scope
// Puts every global of "SIR State" into its start value (what an earlier test codeunit leaves for the readers).

codeunit 73476 "SIR Setter D"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure PutStartValues()
    var
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        S.InitAll();
        Observer.Observe('start values set', 'yes');
        Observer.Expect('start values set', 'yes');
        Observer.Verify();
    end;
}
