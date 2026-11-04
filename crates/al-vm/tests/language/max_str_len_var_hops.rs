//! MaxStrLen of a `var` Text or Code parameter that was handed on from another `var` parameter, or that an event
//! subscriber receives.

use al_testkit::{run_in, text};

const HOPS: &str = r#"
table 69960 MH
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Description; Text[100]) { }
    }
}

codeunit 69961 MHops
{
    procedure Hops(): Text
    var
        Item: Record MH;
        T30: Text[30];
        T: Text;
        C20: Code[20];
    begin
        exit(Format(Hop1(T30)) + '|' + Format(Hop1(Item.Description)) + '|' + Format(Hop1(T)) + '|' + Format(Hop0(T30)) + '|' +
            Format(Hop30(T30)) + '|' + Format(Hop30(Item.Description)) + '|' + Format(HopCode(C20)));
    end;

    procedure Events(): Text
    var
        Sub: Codeunit MHEvents;
        Item: Record MH;
        T30: Text[30];
        T: Text;
        Seen: Text;
    begin
        BindSubscription(Sub);
        Sub.OnProbe(T30);
        Seen := Sub.Seen();
        Sub.OnProbe(Item.Description);
        Seen += '|' + Sub.Seen();
        Sub.Raise(T30);
        Seen += '|' + Sub.Seen();
        Sub.Raise(Item.Description);
        Seen += '|' + Sub.Seen();
        Sub.Raise(T);
        Seen += '|' + Sub.Seen();
        Sub.RaiseFromVar(T30);
        Seen += '|' + Sub.Seen();
        UnbindSubscription(Sub);
        exit(Seen);
    end;

    local procedure Hop0(var s: Text): Integer
    begin
        exit(Hop1(s));
    end;

    local procedure Hop1(var s: Text): Integer
    begin
        exit(MaxOfVarText(s));
    end;

    local procedure Hop30(var s: Text[30]): Integer
    begin
        exit(MaxOfVarText(s));
    end;

    local procedure HopCode(var s: Code[30]): Integer
    begin
        exit(MaxOfVarCode30(s));
    end;

    local procedure MaxOfVarText(var s: Text): Integer
    begin
        exit(MaxStrLen(s));
    end;

    local procedure MaxOfVarCode30(var s: Code[30]): Integer
    begin
        exit(MaxStrLen(s));
    end;
}

codeunit 69962 MHEvents
{
    EventSubscriberInstance = Manual;

    var
        Last: Text;

    procedure Raise(var Value: Text)
    begin
        Last := '';
        OnProbe(Value);
    end;

    procedure RaiseFromVar(var Value: Text)
    begin
        Raise(Value);
    end;

    procedure Seen() Result: Text
    begin
        Result := Last;
        Last := '<none>';
    end;

    [IntegrationEvent(false, false)]
    procedure OnProbe(var Value: Text)
    begin
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::MHEvents, 'OnProbe', '', false, false)]
    local procedure HandleProbe(var Value: Text)
    begin
        Last := Format(MaxStrLen(Value));
    end;
}

"#;

// corpus text/text-methods/MaxStrLen.Codeunit.al:146-170, PYC_MaxStrLen_VarParameterHops: a `var` parameter handed on
// from another `var` parameter keeps the declared length of the first caller's variable or field, through two and three
// hops and through a `var` Text[30] parameter on the way (`hop through text30 param field` = 100).
#[test]
fn max_str_len_of_a_var_parameter_handed_on_is_the_length_of_the_first_callers_variable() {
    assert_eq!(text(run_in(HOPS, "MHops", "Hops")), "30|100|2147483647|30|30|100|20");
}

// corpus text/text-methods/MaxStrLen.Codeunit.al:172-201, PYC_MaxStrLen_EventSubscriberVarParameter: the `var` Text
// parameter of a subscriber reports the declared length of the publisher's argument, raised directly, from a procedure,
// and from a procedure that got it through a `var` parameter.
#[test]
fn max_str_len_of_a_subscribers_var_parameter_is_the_length_of_the_publishers_argument() {
    assert_eq!(text(run_in(HOPS, "MHops", "Events")), "30|100|30|100|2147483647|30");
}
