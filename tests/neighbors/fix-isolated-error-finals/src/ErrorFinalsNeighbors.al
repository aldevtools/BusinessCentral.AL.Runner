// Neighbors of al-runner issue 8584: an error that AL code catches ends where it is caught, so the parameters of the
// routine it left are not written into the `var` arguments of a later call that fails before its routine starts (a
// Variant holding an Integer meets a by-value Text parameter: TypeConversion). The errors caught first: one an Isolated
// event swallows, one asserterror catches out of a table event subscriber under Validate, one a TryFunction catches, one
// a guarded Codeunit.Run catches, one a TestPage swallows (OnNewRecord on New, Error('') in an action), and a duplicate
// Insert(true) whose answer is used. The controls: the same calls with nothing failing first, and the pinned write-backs of
// a routine that ran and failed (TryFunction and asserterror).
table 61080 "EF Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Integer) { }
    }
    keys
    {
        key(PK; Id) { }
    }
}

page 61085 "EF Card"
{
    PageType = Card;
    SourceTable = "EF Row";
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(Id; Rec.Id) { ApplicationArea = All; }
        }
    }

    actions
    {
        area(Processing)
        {
            action(SilentError)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Error('');
                end;
            }
        }
    }

    trigger OnNewRecord(BelowxRec: Boolean)
    var
        R: Record "EF Row";
    begin
        if R.Get(99) then
            Error('first');
    end;
}

codeunit 61081 "EF Publisher"
{
    procedure RaiseFail()
    var
        A: Integer;
    begin
        OnIsoFail(A);
    end;

    procedure RaiseOk()
    var
        A: Integer;
    begin
        OnIsoOk(A);
    end;

    [IntegrationEvent(false, false, true)]
    local procedure OnIsoFail(var A: Integer)
    begin
    end;

    [IntegrationEvent(false, false, true)]
    local procedure OnIsoOk(var A: Integer)
    begin
    end;
}

codeunit 61082 "EF Subscribers"
{
    EventSubscriberInstance = StaticAutomatic;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"EF Publisher", 'OnIsoFail', '', false, false)]
    local procedure FailSub(var A: Integer)
    begin
        A := 999;
        Error('first');
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"EF Publisher", 'OnIsoOk', '', false, false)]
    local procedure OkSub(var A: Integer)
    begin
        A := 999;
    end;

    [EventSubscriber(ObjectType::Table, Database::"EF Row", 'OnBeforeValidateEvent', 'Amount', false, false)]
    local procedure FailValidate(var Rec: Record "EF Row"; var xRec: Record "EF Row"; CurrFieldNo: Integer)
    begin
        if Rec.Amount <> 13 then
            exit;
        Rec.Id := 888;
        Rec.Amount := 888;
        Error('first');
    end;
}

codeunit 61083 "EF Failing Run"
{
    TableNo = "EF Row";

    trigger OnRun()
    begin
        Rec.Id := 777;
        Rec.Amount := 777;
        Error('first');
    end;
}

codeunit 61084 "EF Error Finals Tests"
{
    Subtype = Test;

    local procedure Check(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('expected <%1>, got <%2>', Expected, Actual);
    end;

    local procedure TakeVarAndText(var V: Integer; T: Text)
    begin
        V := 3;
    end;

    local procedure TakeVarRecAndText(var R: Record "EF Row"; T: Text)
    begin
        R.Amount := 3;
    end;

    local procedure SetVarThenFail(var V: Integer; T: Text)
    begin
        V := 3;
        Error('second');
    end;

    [TryFunction]
    local procedure TrySetVarThenFail(var V: Integer)
    begin
        V := 42;
        Error('first');
    end;

    local procedure LateCall(): Text
    var
        V: Integer;
        Vnt: Variant;
    begin
        V := 1;
        Vnt := 7;
        asserterror TakeVarAndText(V, Vnt);
        exit(Format(V) + ' ' + GetLastErrorCode());
    end;

    local procedure LateRecCall(): Text
    var
        R: Record "EF Row";
        Vnt: Variant;
    begin
        R.Id := 1;
        R.Amount := 5;
        Vnt := 7;
        asserterror TakeVarRecAndText(R, Vnt);
        exit(Format(R.Id) + '/' + Format(R.Amount) + ' ' + GetLastErrorCode());
    end;

    [Test]
    procedure IsolatedEventOk_ThenCallFailsBeforeStart()
    var
        P: Codeunit "EF Publisher";
    begin
        P.RaiseOk();
        Check('1 TypeConversion', LateCall());
    end;

    [Test]
    procedure IsolatedEventFailed_ThenCallFailsBeforeStart()
    var
        P: Codeunit "EF Publisher";
    begin
        P.RaiseFail();
        Check('1 TypeConversion', LateCall());
    end;

    [Test]
    procedure IsolatedEventFailed_ThenCallRunsAndFails()
    var
        P: Codeunit "EF Publisher";
        V: Integer;
    begin
        P.RaiseFail();
        V := 1;
        asserterror SetVarThenFail(V, 'x');
        Check('3 second', Format(V) + ' ' + GetLastErrorText());
    end;

    [Test]
    procedure ValidateOk_ThenRecCallFailsBeforeStart()
    var
        R: Record "EF Row";
    begin
        R.Validate(Amount, 14);
        Check('1/5 TypeConversion', LateRecCall());
    end;

    [Test]
    procedure AsserterrorValidateSubscriberFailed_ThenRecCallFailsBeforeStart()
    var
        R: Record "EF Row";
    begin
        asserterror R.Validate(Amount, 13);
        Check('1/5 TypeConversion', LateRecCall());
    end;

    [Test]
    procedure AsserterrorValidateSubscriberFailed_ThenScalarCallFailsBeforeStart()
    var
        R: Record "EF Row";
    begin
        asserterror R.Validate(Amount, 13);
        Check('1 TypeConversion', LateCall());
    end;

    [Test]
    procedure TryFunctionFailed_WritesItsVarBack()
    var
        V: Integer;
    begin
        V := 0;
        if TrySetVarThenFail(V) then;
        Check('42', Format(V));
    end;

    [Test]
    procedure TryFunctionFailed_ThenCallFailsBeforeStart()
    var
        V: Integer;
    begin
        if TrySetVarThenFail(V) then;
        Check('1 TypeConversion', LateCall());
    end;

    [Test]
    procedure AsserterrorFailed_WritesItsVarBack()
    var
        V: Integer;
    begin
        V := 0;
        asserterror SetVarThenFail(V, 'x');
        Check('3', Format(V));
    end;

    [Test]
    procedure AsserterrorFailed_ThenCallFailsBeforeStart()
    var
        V: Integer;
    begin
        asserterror SetVarThenFail(V, 'x');
        Check('1 TypeConversion', LateCall());
    end;

    [Test]
    procedure GuardedCodeunitRunFailed_ThenRecCallFailsBeforeStart()
    var
        R: Record "EF Row";
    begin
        if Codeunit.Run(Codeunit::"EF Failing Run", R) then;
        Check('1/5 TypeConversion', LateRecCall());
    end;

    local procedure OpenCard(var Card: TestPage "EF Card"; FailNew: Boolean)
    var
        R: Record "EF Row";
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Insert();
        if FailNew then begin
            R.Id := 99;
            R.Insert();
        end;
        Card.OpenEdit();
    end;

    [Test]
    procedure TestPageNewOk_ThenCallFailsBeforeStart()
    var
        Card: TestPage "EF Card";
    begin
        OpenCard(Card, false);
        Card.New();
        Check('1 TypeConversion', LateCall());
    end;

    [Test]
    procedure TestPageNewSwallowedOnNewRecordError_ThenCallFailsBeforeStart()
    var
        Card: TestPage "EF Card";
    begin
        OpenCard(Card, true);
        Card.New();
        Check('1 TypeConversion', LateCall());
    end;

    [Test]
    procedure TestPageActionSilentError_ThenCallFailsBeforeStart()
    var
        Card: TestPage "EF Card";
    begin
        OpenCard(Card, false);
        Card.SilentError.Invoke();
        Check('1 TypeConversion', LateCall());
    end;

    [Test]
    procedure DuplicateInsertTrueAnswerUsed_ThenCallFailsBeforeStart()
    var
        R: Record "EF Row";
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Insert();
        if R.Insert(true) then
            Error('duplicate inserted');
        Check('1 TypeConversion', LateCall());
    end;
}
