// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-singleinstance-property
// Scope: in-scope
// The third test codeunit: what was kept and what the globals hold now.

codeunit 73479 "SIR Keep Final"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure KeepFinal()
    var
        S: Codeunit "SIR State";
        K: Codeunit "SIR Keep";
        Observer: Codeunit "Probe Observer";
    begin
        Observer.Observe('kept int', Format(K.KeptIntNow()));
        Observer.Observe('kept text', K.KeptTextNow());
        Observer.Observe('kept list', Format(K.KeptListCount()));
        Observer.Observe('global int', Format(S.GetInt()));
        Observer.Observe('global text', S.GetTxt());
        Observer.Observe('global list', Format(S.GetListNew().Count()));
        Observer.Expect('kept int', '1');
        Observer.Expect('kept text', 'set');
        Observer.Expect('kept list', '2');
        Observer.Expect('global int', '2');
        Observer.Expect('global text', 'second');
        Observer.Expect('global list', '1');
        Observer.Verify();
    end;
}
