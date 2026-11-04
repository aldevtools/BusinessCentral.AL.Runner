//! BindSubscription on a copy of a bound instance and on a second variable of a SingleInstance manual codeunit raises
//! "already been bound" (corpus branch al-runtime-rs-staging-p6924c, events/TestProbeEventContracts.al
//! BSK_Bind_AfterClear_And_OnCopy and BSK_Bind_SingleInstanceManual_TwoVariables, objects in
//! events/BindSubscriptionKinds.Objects.al; BC 27.5, 28.5, 29.0). Clear of a bound variable drops its binding.

const SRC: &str = r#"
codeunit 95390 "LT BSK Pub"
{
    procedure Ping(): Text
    var
        Hits: Text;
    begin
        OnPing(Hits);
        exit(Hits);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnPing(var Hits: Text)
    begin
    end;
}

codeunit 95391 "LT BSK Manual SI"
{
    SingleInstance = true;
    EventSubscriberInstance = Manual;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LT BSK Pub", 'OnPing', '', false, false)]
    local procedure OnPing(var Hits: Text)
    begin
        Hits += 'S';
    end;
}

codeunit 95393 "LT BSK Manual"
{
    EventSubscriberInstance = Manual;

    var
        Tag: Text;

    procedure SetTag(NewTag: Text)
    begin
        Tag := NewTag;
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"LT BSK Pub", 'OnPing', '', false, false)]
    local procedure OnPing(var Hits: Text)
    begin
        Hits += 'M' + Tag;
    end;
}

codeunit 50100 "BSK Probe"
{
    [TryFunction]
    local procedure TryBind(var C: Codeunit "LT BSK Manual")
    begin
        BindSubscription(C);
    end;

    [TryFunction]
    local procedure TryBindSI(var C: Codeunit "LT BSK Manual SI")
    begin
        BindSubscription(C);
    end;

    local procedure Result(Ok: Boolean): Text
    var
        R: Text;
    begin
        if Ok then
            R := 'ok'
        else
            R := GetLastErrorText() + '|' + GetLastErrorCode();
        ClearLastError();
        exit(R);
    end;

    procedure OnCopy(): Text
    var
        Pub: Codeunit "LT BSK Pub";
        A: Codeunit "LT BSK Manual";
        B: Codeunit "LT BSK Manual";
        Out: Text;
    begin
        A.SetTag('1');
        Out := Result(TryBind(A));
        Clear(A);
        A.SetTag('C');
        Out += ';' + Pub.Ping();
        Out += ';' + Result(TryBind(A));
        Out += ';' + Pub.Ping();
        Out += ';' + Format(UnbindSubscription(A));
        A.SetTag('3');
        Out += ';' + Result(TryBind(A));
        B := A;
        Out += ';' + Result(TryBind(B));
        Out += ';' + Pub.Ping();
        Out += ';' + Format(UnbindSubscription(B));
        Out += ';' + Pub.Ping();
        Out += ';' + Format(UnbindSubscription(A));
        exit(Out);
    end;

    procedure SingleInstance(): Text
    var
        Pub: Codeunit "LT BSK Pub";
        A: Codeunit "LT BSK Manual SI";
        B: Codeunit "LT BSK Manual SI";
        Out: Text;
    begin
        Out := Result(TryBindSI(A));
        Out += ';' + Result(TryBindSI(B));
        Out += ';' + Pub.Ping();
        Out += ';' + Format(UnbindSubscription(B));
        Out += ';' + Pub.Ping();
        Out += ';' + Format(UnbindSubscription(A));
        exit(Out);
    end;
}
"#;

#[test]
fn bind_of_a_copy_of_a_bound_instance_is_already_bound() {
    let got = al_testkit::text(al_testkit::run_in(SRC, "BSK Probe", "OnCopy"));
    assert_eq!(
        got,
        "ok;;ok;MC;Yes;ok;The binding of codeunit 95393 was unsuccessful. The codeunit has already been bound.|EventBinding;M3;Yes;;Yes"
    );
}

#[test]
fn bind_of_a_second_variable_of_a_single_instance_manual_codeunit_is_already_bound() {
    let got = al_testkit::text(al_testkit::run_in(SRC, "BSK Probe", "SingleInstance"));
    assert_eq!(
        got,
        "ok;The binding of codeunit 95391 was unsuccessful. The codeunit has already been bound.|EventBinding;S;Yes;;Yes"
    );
}
