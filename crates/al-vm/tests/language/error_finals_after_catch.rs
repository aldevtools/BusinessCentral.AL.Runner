//! An error that AL code catches ends where it is caught: the parameters of the routine it left are not written back into
//! the arguments of a later call. A caller writes its `var` arguments back from the routine the error left
//! (errors/TestTryFunctionContracts.al:85-95,203-208; codeunits/TestScopeIsolationContracts.al:63-77,248-252). Here the
//! later call fails before its routine starts: a Variant holding an Integer meets a by-value Text parameter, TypeConversion
//! (types/variant/VariantNumberToTextParam.al:53-80). Its `var` argument keeps its value, as it does when nothing failed
//! before. The errors caught first: one an Isolated event swallows (`[IntegrationEvent(false, false, true)]`, corpus
//! codeunits/ISOEventPublisher.Codeunit.al), and one asserterror catches out of a table event subscriber under Validate.

const SRC: &str = "
table 50740 Row { fields { field(1; Id; Integer) { } field(2; Amount; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50741 Pub {
    procedure RaiseFail() var A: Integer; begin OnIsoFail(A); end;
    procedure RaiseOk() var A: Integer; begin OnIsoOk(A); end;
    [IntegrationEvent(false, false, true)] local procedure OnIsoFail(var A: Integer) begin end;
    [IntegrationEvent(false, false, true)] local procedure OnIsoOk(var A: Integer) begin end; }
codeunit 50742 Subs {
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnIsoFail', '', false, false)]
    local procedure FailSub(var A: Integer) begin A := 999; Error('first'); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnIsoOk', '', false, false)]
    local procedure OkSub(var A: Integer) begin A := 999; end;
    [EventSubscriber(ObjectType::Table, Database::Row, 'OnBeforeValidateEvent', 'Amount', false, false)]
    local procedure FailValidate(var Rec: Record Row; var xRec: Record Row; CurrFieldNo: Integer)
    begin if Rec.Amount <> 13 then exit; Rec.Id := 888; Rec.Amount := 888; Error('first'); end; }
codeunit 50743 C {
    local procedure TakeVarAndText(var V: Integer; T: Text) begin V := 3; end;
    local procedure TakeVarRecAndText(var R: Record Row; T: Text) begin R.Amount := 3; end;
    local procedure Late(): Text var V: Integer; Vnt: Variant; begin
        V := 1; Vnt := 7;
        asserterror TakeVarAndText(V, Vnt);
        exit(Format(V) + ' ' + GetLastErrorCode()); end;
    local procedure LateRec(): Text var R: Record Row; Vnt: Variant; begin
        R.Id := 1; R.Amount := 5; Vnt := 7;
        asserterror TakeVarRecAndText(R, Vnt);
        exit(Format(R.Id) + '/' + Format(R.Amount) + ' ' + GetLastErrorCode()); end;
    procedure IsolatedFail(): Text var P: Codeunit Pub; begin P.RaiseFail(); exit(Late()); end;
    procedure IsolatedOk(): Text var P: Codeunit Pub; begin P.RaiseOk(); exit(Late()); end;
    procedure ValidateFail(): Text var R: Record Row; begin asserterror R.Validate(Amount, 13); exit(LateRec()); end;
    procedure ValidateOk(): Text var R: Record Row; begin R.Validate(Amount, 14); exit(LateRec()); end; }";

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

#[test]
fn an_error_an_isolated_event_swallowed_is_not_written_back_later() {
    assert_eq!(run("IsolatedOk"), "1 TypeConversion");
    assert_eq!(run("IsolatedFail"), "1 TypeConversion");
}

#[test]
fn an_error_asserterror_caught_out_of_a_builtin_is_not_written_back_later() {
    assert_eq!(run("ValidateOk"), "1/5 TypeConversion");
    assert_eq!(run("ValidateFail"), "1/5 TypeConversion");
}
