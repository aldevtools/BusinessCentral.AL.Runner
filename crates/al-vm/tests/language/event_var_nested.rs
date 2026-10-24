//! A subscriber's `var` parameter that is the publisher's global (events/lt-event-var-through-sender/EventVarThroughSender.al)
//! stays that variable when the subscriber hands it on by `var` to a procedure of its own: a value the procedure sets is seen
//! through Sender, and a value Sender sets is seen in the procedure's parameter; a procedure that takes it by value gets a copy
//! (al-runner issue 5754; EventVarThroughSender.al:284-306 RO_EventVar_NestedProcedure_SetsParams_ReadsThroughSender and
//! :308-330 RO_EventVar_InternalEvent_Repro).

const SRC: &str = r#"
codeunit 50300 Pub
{
    var
        LineCreated: Boolean;
        Counter: Integer;
        Note: Text;

    procedure Start()
    begin
        LineCreated := false;
        Counter := 1;
        Note := 'initial';
    end;

    procedure RaiseWithGlobals()
    begin
        OnWork(LineCreated, Counter, Note);
    end;

    procedure SetFromSender()
    begin
        LineCreated := true;
        Counter := 5;
        Note := 'set by sender';
    end;

    procedure Seen(): Text
    begin
        exit(Format(LineCreated) + '/' + Format(Counter) + '/' + Note);
    end;

    [IntegrationEvent(true, false)]
    local procedure OnWork(var Created: Boolean; var Count: Integer; var Txt: Text)
    begin
    end;
}

codeunit 50301 Sub
{
    EventSubscriberInstance = Manual;

    var
        Mode: Integer;
        Trace: Text;

    procedure SetMode(NewMode: Integer)
    begin
        Mode := NewMode;
        Trace := '';
    end;

    procedure Seen(): Text
    begin
        exit(Trace);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnWork', '', false, false)]
    local procedure OnWork(var Sender: Codeunit Pub; var Created: Boolean; var Count: Integer; var Txt: Text)
    begin
        case Mode of
            2:
                SetThenReadVarSender(Created, Count, Txt, Sender);
            3:
                SetThenReadValueSender(Created, Count, Txt, Sender);
            4:
                Outer(Created, Count, Txt, Sender);
            5:
                SenderSetsThenReadParams(Created, Count, Txt, Sender);
            6:
                SetCopiesThenReadSender(Created, Count, Txt, Sender);
            7:
                begin
                    SetOnly(Created, Count, Txt);
                    Trace += 'sender after return:' + Sender.Seen() + ';';
                end;
        end;
        Trace += 'params at end:' + Format(Created) + '/' + Format(Count) + '/' + Txt + ';';
    end;

    local procedure SetOnly(var Created: Boolean; var Count: Integer; var Txt: Text)
    begin
        Created := true;
        Count := 42;
        Txt := 'set by nested';
    end;

    local procedure SetThenReadVarSender(var Created: Boolean; var Count: Integer; var Txt: Text; var Sender: Codeunit Pub)
    begin
        SetOnly(Created, Count, Txt);
        Trace += 'sender:' + Sender.Seen() + ';';
    end;

    local procedure SetThenReadValueSender(var Created: Boolean; var Count: Integer; var Txt: Text; Sender: Codeunit Pub)
    begin
        Created := true;
        Count := 42;
        Txt := 'set by nested';
        Trace += 'value sender:' + Sender.Seen() + ';';
    end;

    local procedure Outer(var Created: Boolean; var Count: Integer; var Txt: Text; var Sender: Codeunit Pub)
    begin
        SetThenReadVarSender(Created, Count, Txt, Sender);
        Trace += 'sender in outer:' + Sender.Seen() + ';';
    end;

    local procedure SenderSetsThenReadParams(var Created: Boolean; var Count: Integer; var Txt: Text; var Sender: Codeunit Pub)
    begin
        Sender.SetFromSender();
        Trace += 'nested params:' + Format(Created) + '/' + Format(Count) + '/' + Txt + ';';
    end;

    local procedure SetCopiesThenReadSender(Created: Boolean; Count: Integer; Txt: Text; var Sender: Codeunit Pub)
    begin
        Created := true;
        Count := 42;
        Txt := 'set by nested copy';
        Trace += 'sender:' + Sender.Seen() + ';';
    end;
}

codeunit 50302 Other
{
    procedure SetIt(var Created: Boolean; var Sender: Codeunit Pub): Text
    begin
        Created := true;
        exit(Sender.Seen());
    end;
}

codeunit 50303 OtherSub
{
    EventSubscriberInstance = Manual;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnWork', '', false, false)]
    local procedure OnWork(var Sender: Codeunit Pub; var Created: Boolean; var Count: Integer; var Txt: Text)
    var
        O: Codeunit Other;
    begin
        O.SetIt(Created, Sender);
    end;
}

codeunit 50304 T
{
    local procedure Nested(Mode: Integer): Text
    var
        P: Codeunit Pub;
        S: Codeunit Sub;
    begin
        P.Start();
        S.SetMode(Mode);
        BindSubscription(S);
        P.RaiseWithGlobals();
        UnbindSubscription(S);
        exit(S.Seen() + 'globals:' + P.Seen());
    end;

    procedure Modes(): Text
    var
        I: Integer;
        R: Text;
    begin
        for I := 2 to 7 do
            R += Format(I) + ' ' + Nested(I) + '#';
        exit(R);
    end;

    procedure ToOtherInstance(): Text
    var
        P: Codeunit Pub;
        S: Codeunit OtherSub;
    begin
        P.Start();
        BindSubscription(S);
        P.RaiseWithGlobals();
        UnbindSubscription(S);
        exit(P.Seen());
    end;
}
"#;

#[test]
fn a_var_parameter_handed_on_to_a_procedure_stays_the_publishers_global() {
    let want = [
        "2 sender:Yes/42/set by nested;params at end:Yes/42/set by nested;globals:Yes/42/set by nested",
        "3 value sender:Yes/42/set by nested;params at end:Yes/42/set by nested;globals:Yes/42/set by nested",
        "4 sender:Yes/42/set by nested;sender in outer:Yes/42/set by nested;params at end:Yes/42/set by nested;globals:Yes/42/set by nested",
        "5 nested params:Yes/5/set by sender;params at end:Yes/5/set by sender;globals:Yes/5/set by sender",
        "6 sender:No/1/initial;params at end:No/1/initial;globals:No/1/initial",
        "7 sender after return:Yes/42/set by nested;params at end:Yes/42/set by nested;globals:Yes/42/set by nested",
    ];
    let got = al_testkit::text(al_testkit::run_in(SRC, "T", "Modes"));
    let got: Vec<&str> = got.trim_end_matches('#').split('#').collect();
    assert_eq!(got, want);
}

/// Handed on to a procedure of another codeunit instance: whether it stays the publisher's global there is not measured, so
/// the sender reading that global while the procedure runs is refused.
#[test]
fn a_var_parameter_handed_on_to_another_instance_is_refused_when_the_sender_reads_it() {
    match al_testkit::run_in(SRC, "T", "ToOtherInstance") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("LineCreated"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
