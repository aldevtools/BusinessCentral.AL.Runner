// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-singleinstance-property
// Scope: in-scope
// Reads three globals into the SingleInstance "SIR Keep" (a value that outlives the test codeunit).

codeunit 73477 "SIR Keep Read"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure KeepRead()
    var
        S: Codeunit "SIR State";
        K: Codeunit "SIR Keep";
        Observer: Codeunit "Probe Observer";
    begin
        K.KeepInt(S.GetInt());
        K.KeepText(S.GetTxt());
        K.KeepList(S.GetListNew());
        Observer.Observe('kept int', Format(K.KeptIntNow()));
        Observer.Observe('kept text', K.KeptTextNow());
        Observer.Observe('kept list', Format(K.KeptListCount()));
        Observer.Expect('kept int', '1');
        Observer.Expect('kept text', 'set');
        Observer.Expect('kept list', '2');
        Observer.Verify();
    end;
}
