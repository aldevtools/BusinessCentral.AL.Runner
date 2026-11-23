// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-singleinstance-property
// Scope: in-scope
// A value read from the SingleInstance state and kept in a variable of the test codeunit: read in one test, the global
// overwritten in a later test, the kept value observed in a third test (the tests of a codeunit run in declaration order).

codeunit 73475 "SIR Across Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        KeptInt: Integer;
        KeptText: Text;
        KeptList: List of [Text];

    [Test]
    procedure T1_ReadAndKeep()
    var
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        KeptInt := S.GetInt();
        KeptText := S.GetTxt();
        KeptList := S.GetListNew();
        Observer.Observe('kept int', Format(KeptInt));
        Observer.Observe('kept text', KeptText);
        Observer.Observe('kept list', Format(KeptList.Count()));
        Observer.Expect('kept int', '1');
        Observer.Expect('kept text', 'set');
        Observer.Expect('kept list', '2');
        Observer.Verify();
    end;

    [Test]
    procedure T2_KeptVariablesInTheNextTest()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Observer.Observe('kept int', Format(KeptInt));
        Observer.Observe('kept text', KeptText);
        Observer.Observe('kept list', Format(KeptList.Count()));
        Observer.Expect('kept int', '1');
        Observer.Expect('kept text', 'set');
        Observer.Expect('kept list', '2');
        Observer.Verify();
    end;

    [Test]
    procedure T3_OverwriteTheGlobals()
    var
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        S.WInt();
        S.WTxt();
        S.WListNew();
        Observer.Observe('global int', Format(S.GetInt()));
        Observer.Observe('global text', S.GetTxt());
        Observer.Observe('global list', Format(S.GetListNew().Count()));
        Observer.Expect('global int', '2');
        Observer.Expect('global text', 'second');
        Observer.Expect('global list', '1');
        Observer.Verify();
    end;

    [Test]
    procedure T4_ObserveKeptAndGlobal()
    var
        S: Codeunit "SIR State";
        Observer: Codeunit "Probe Observer";
    begin
        Observer.Observe('kept int', Format(KeptInt));
        Observer.Observe('kept text', KeptText);
        Observer.Observe('kept list', Format(KeptList.Count()));
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
