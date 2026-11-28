// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/errorinfo/errorinfo-errortype-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); self-contained codeunits 77400 (tests) and 77401 (raises an ErrorInfo in OnRun)
// Probe: written against local BC on Linux legs 27.5, 28.5 and 29.0 that mirror the corpus CI (al-runner issue 9409); all three
// produced the same values. Every literal asserted below is what those legs produced.
// Facts: the text of an error raised with an ErrorInfo of ErrorType::Internal. The correlation id changes on every raise, so the tests
// assert its shape (the Shape helper below writes each hex digit as h or H), not the id.
//   - GetLastErrorText() and GetLastErrorText(false): 'An error has occurred. If the error persists, please contact your system
//     administrator.' + '\' (one backslash, char 92) + 'Correlation ID: ' + the id. The id has 36 characters: lower case hex with
//     dashes, NO braces; it parses as a Guid and equals LowerCase(Format(Guid, 0, 4)). Text length 140.
//   - The same text with the Message set through the method, through ErrorInfo.Create, with no Message at all, and with a
//     DetailedMessage: neither Message nor DetailedMessage appears. Same text after asserterror, after a [TryFunction] that returns
//     false, after Codeunit.Run of an instance that returns false, and after a collecting procedure that ends with the collected
//     error. GetLastErrorCode() is Dialog. The ErrorInfo variable keeps its own Message.
//   - GetLastErrorText(true) gives the personal data removed text, as for a Client ErrorInfo.
//   - Reading the text twice gives the same id; two raises of one ErrorInfo give different ids and the same shape.
//   - The Message of a collected Internal ErrorInfo (GetCollectedErrors): the same text, but CR LF (13, 10) instead of the backslash
//     (length 141); same id format.
//   - Client neighbours: GetLastErrorText() is the Message ('' without one); a DetailedMessage does not appear; code Dialog.
// BC versions: 27.5, 28.5, 29.0 (identical)

codeunit 77401 "LT EIT Raiser"
{
    // Raises the ErrorInfo given to SetInfo when it runs (Codeunit.Run form of the probe).
    var
        Info: ErrorInfo;

    trigger OnRun()
    begin
        Error(Info);
    end;

    procedure SetInfo(NewInfo: ErrorInfo)
    begin
        Info := NewInfo;
    end;
}

codeunit 77400 "LT EIT Internal Text"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // The Probe Observer replaces everything after "Correlation ID: " up to the next backslash by x. Shape keeps the text
    // around the id, writes the label as "Correlation ID=" and replaces each hex digit of the id by h (0-9, a-f) or H (A-F).
    // Braces and dashes stay as they are.
    local procedure Shape(T: Text): Text
    var
        P: Integer;
        Head: Text;
        Rest: Text;
    begin
        P := StrPos(T, CidLbl);
        if P = 0 then
            exit('[no id] ' + T);
        Head := CopyStr(T, 1, P - 1);
        Rest := CopyStr(T, P + StrLen(CidLbl));
        exit(Head + 'Correlation ID=' + HexShape(IdPart(Rest)) + CopyStr(Rest, StrLen(IdPart(Rest)) + 1));
    end;

    local procedure IdPart(Rest: Text): Text
    var
        i: Integer;
    begin
        i := 1;
        while (i <= StrLen(Rest)) and (StrPos('0123456789abcdefABCDEF{}-', CopyStr(Rest, i, 1)) > 0) do
            i += 1;
        exit(CopyStr(Rest, 1, i - 1));
    end;

    local procedure IdOf(T: Text): Text
    var
        P: Integer;
    begin
        P := StrPos(T, CidLbl);
        if P = 0 then
            exit('');
        exit(IdPart(CopyStr(T, P + StrLen(CidLbl))));
    end;

    local procedure HexShape(Id: Text): Text
    var
        i: Integer;
        C: Integer;
        Result: Text;
    begin
        for i := 1 to StrLen(Id) do begin
            C := Id[i];
            case true of
                (C >= 48) and (C <= 57), (C >= 97) and (C <= 102):
                    Result += 'h';
                (C >= 65) and (C <= 70):
                    Result += 'H';
                else
                    Result += CopyStr(Id, i, 1);
            end;
        end;
        exit(Result);
    end;

    // Facts about the id inside one error text: its length, whether it parses as a Guid, and whether it equals the
    // Guid's Format number 4 (lower case, no braces). The character in front of the label as a code (92 = backslash).
    local procedure ObserveId(var Observer: Codeunit "Probe Observer"; Name: Text; T: Text)
    var
        Id: Text;
        G: Guid;
        P: Integer;
        C: Integer;
    begin
        Id := IdOf(T);
        Observer.Observe(Name + ' shape', Shape(T));
        Observer.Observe(Name + ' len', Format(StrLen(T)));
        Observer.Observe(Name + ' id len', Format(StrLen(Id)));
        Observer.Observe(Name + ' id is guid', Format(Evaluate(G, Id)));
        Observer.Observe(Name + ' id = lower format 4', Format(LowerCase(Format(G, 0, 4)) = Id));
        Observer.Observe(Name + ' brace pos', Format(StrPos(T, '{')));
        P := StrPos(T, CidLbl);
        if P > 1 then begin
            C := T[P - 1];
            Observer.Observe(Name + ' char before label', Format(C));
        end else
            Observer.Observe(Name + ' char before label', 'none');
        if P > 2 then begin
            C := T[P - 2];
            Observer.Observe(Name + ' 2nd char before label', Format(C));
        end;
    end;

    local procedure Internal(Msg: Text): ErrorInfo
    var
        EI: ErrorInfo;
    begin
        EI.Message(Msg);
        EI.ErrorType(ErrorType::Internal);
        exit(EI);
    end;

    [TryFunction]
    local procedure TryRaise(EI: ErrorInfo)
    begin
        Error(EI);
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectOne(EI: ErrorInfo; var Collected: ErrorInfo; var Count: Integer)
    var
        Errors: List of [ErrorInfo];
    begin
        Error(EI);
        Errors := GetCollectedErrors(true);
        Count := Errors.Count();
        if Count > 0 then
            Errors.Get(1, Collected);
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure CollectAndLeave(EI: ErrorInfo)
    begin
        Error(EI);
    end;

    // The reporter's shape: Message set through the method, ErrorType Internal, asserterror Error(EI), GetLastErrorText().
    [Test]
    procedure Internal_Asserterror_MessageMethod_Text()
    var
        Observer: Codeunit "Probe Observer";
        EI: ErrorInfo;
        G: Guid;
    begin
        EI := Internal('E2 internal');
        asserterror Error(EI);
        ObserveId(Observer, 'text', GetLastErrorText());
        Observer.Observe('code', GetLastErrorCode());
        Observer.Observe('message after raise', EI.Message);
        Evaluate(G, '{6F9619FF-8B86-D011-B42D-00C04FC964FF}');
        Observer.Observe('format 4 of a fixed guid', Format(G, 0, 4));
        Observer.Expect('text shape', 'An error has occurred. If the error persists, please contact your system administrator.\Correlation ID=hhhhhhhh-hhhh-hhhh-hhhh-hhhhhhhhhhhh');
        Observer.Expect('text len', '140');
        Observer.Expect('text id len', '36');
        Observer.Expect('text id is guid', 'Yes');
        Observer.Expect('text id = lower format 4', 'Yes');
        Observer.Expect('text brace pos', '0');
        Observer.Expect('text char before label', '92');
        Observer.Expect('text 2nd char before label', '46');
        Observer.Expect('code', 'Dialog');
        Observer.Expect('message after raise', 'E2 internal');
        Observer.Expect('format 4 of a fixed guid', '6F9619FF-8B86-D011-B42D-00C04FC964FF');
        Observer.Verify();
    end;

    // ErrorInfo.Create and the Message property instead of the method.
    [Test]
    procedure Internal_Asserterror_Create_Text()
    var
        Observer: Codeunit "Probe Observer";
        EI: ErrorInfo;
    begin
        EI := ErrorInfo.Create('created internal');
        EI.ErrorType := ErrorType::Internal;
        asserterror Error(EI);
        ObserveId(Observer, 'text', GetLastErrorText());
        Observer.Expect('text shape', 'An error has occurred. If the error persists, please contact your system administrator.\Correlation ID=hhhhhhhh-hhhh-hhhh-hhhh-hhhhhhhhhhhh');
        Observer.Expect('text len', '140');
        Observer.Expect('text id len', '36');
        Observer.Expect('text id is guid', 'Yes');
        Observer.Expect('text id = lower format 4', 'Yes');
        Observer.Expect('text brace pos', '0');
        Observer.Expect('text char before label', '92');
        Observer.Expect('text 2nd char before label', '46');
        Observer.Verify();
    end;

    // No Message at all.
    [Test]
    procedure Internal_Asserterror_NoMessage_Text()
    var
        Observer: Codeunit "Probe Observer";
        EI: ErrorInfo;
    begin
        EI.ErrorType(ErrorType::Internal);
        asserterror Error(EI);
        ObserveId(Observer, 'text', GetLastErrorText());
        Observer.Observe('code', GetLastErrorCode());
        Observer.Expect('text shape', 'An error has occurred. If the error persists, please contact your system administrator.\Correlation ID=hhhhhhhh-hhhh-hhhh-hhhh-hhhhhhhhhhhh');
        Observer.Expect('text len', '140');
        Observer.Expect('text id len', '36');
        Observer.Expect('text id is guid', 'Yes');
        Observer.Expect('text id = lower format 4', 'Yes');
        Observer.Expect('text brace pos', '0');
        Observer.Expect('text char before label', '92');
        Observer.Expect('text 2nd char before label', '46');
        Observer.Expect('code', 'Dialog');
        Observer.Verify();
    end;

    // A DetailedMessage beside the Message.
    [Test]
    procedure Internal_Asserterror_DetailedMessage_Text()
    var
        Observer: Codeunit "Probe Observer";
        EI: ErrorInfo;
    begin
        EI := Internal('main internal');
        EI.DetailedMessage('detail of the internal error');
        asserterror Error(EI);
        ObserveId(Observer, 'text', GetLastErrorText());
        Observer.Expect('text shape', 'An error has occurred. If the error persists, please contact your system administrator.\Correlation ID=hhhhhhhh-hhhh-hhhh-hhhh-hhhhhhhhhhhh');
        Observer.Expect('text len', '140');
        Observer.Expect('text id len', '36');
        Observer.Expect('text id is guid', 'Yes');
        Observer.Expect('text id = lower format 4', 'Yes');
        Observer.Expect('text brace pos', '0');
        Observer.Expect('text char before label', '92');
        Observer.Expect('text 2nd char before label', '46');
        Observer.Verify();
    end;

    // GetLastErrorText(true) (exclude customer content), GetLastErrorText(false) and GetLastErrorText() of one raise.
    [Test]
    procedure Internal_LastErrorTextForms_SameRaise()
    var
        Observer: Codeunit "Probe Observer";
        TextTrue: Text;
        TextFalse: Text;
        TextDefault: Text;
        TextAgain: Text;
    begin
        asserterror Error(Internal('forms internal'));
        TextTrue := GetLastErrorText(true);
        TextFalse := GetLastErrorText(false);
        TextDefault := GetLastErrorText();
        TextAgain := GetLastErrorText();
        Observer.Observe('true shape', Shape(TextTrue));
        Observer.Observe('false shape', Shape(TextFalse));
        Observer.Observe('default shape', Shape(TextDefault));
        Observer.Observe('true = default', Format(TextTrue = TextDefault));
        Observer.Observe('false = default', Format(TextFalse = TextDefault));
        Observer.Observe('second read = first', Format(TextAgain = TextDefault));
        ClearLastError();
        Observer.Observe('after clear', GetLastErrorText());
        Observer.Expect('true shape', '[no id] This message had personal data removed. The original may still be in transient telemetry. Find it using the transientTelemetryId.');
        Observer.Expect('false shape', 'An error has occurred. If the error persists, please contact your system administrator.\Correlation ID=hhhhhhhh-hhhh-hhhh-hhhh-hhhhhhhhhhhh');
        Observer.Expect('default shape', 'An error has occurred. If the error persists, please contact your system administrator.\Correlation ID=hhhhhhhh-hhhh-hhhh-hhhh-hhhhhhhhhhhh');
        Observer.Expect('true = default', 'No');
        Observer.Expect('false = default', 'Yes');
        Observer.Expect('second read = first', 'Yes');
        Observer.Expect('after clear', '');
        Observer.Verify();
    end;

    // TryFunction: the text after a false return.
    [Test]
    procedure Internal_TryFunction_Text()
    var
        Observer: Codeunit "Probe Observer";
    begin
        Observer.Observe('try result', Format(TryRaise(Internal('try internal'))));
        ObserveId(Observer, 'text', GetLastErrorText());
        Observer.Observe('code', GetLastErrorCode());
        Observer.Expect('try result', 'No');
        Observer.Expect('text shape', 'An error has occurred. If the error persists, please contact your system administrator.\Correlation ID=hhhhhhhh-hhhh-hhhh-hhhh-hhhhhhhhhhhh');
        Observer.Expect('text len', '140');
        Observer.Expect('text id len', '36');
        Observer.Expect('text id is guid', 'Yes');
        Observer.Expect('text id = lower format 4', 'Yes');
        Observer.Expect('text brace pos', '0');
        Observer.Expect('text char before label', '92');
        Observer.Expect('text 2nd char before label', '46');
        Observer.Expect('code', 'Dialog');
        Observer.Verify();
    end;

    // Codeunit.Run of an instance that raises the ErrorInfo in OnRun.
    [Test]
    procedure Internal_CodeunitRun_Text()
    var
        Observer: Codeunit "Probe Observer";
        Raiser: Codeunit "LT EIT Raiser";
    begin
        Raiser.SetInfo(Internal('run internal'));
        Observer.Observe('run result', Format(Raiser.Run()));
        ObserveId(Observer, 'text', GetLastErrorText());
        Observer.Observe('code', GetLastErrorCode());
        Observer.Expect('run result', 'No');
        Observer.Expect('text shape', 'An error has occurred. If the error persists, please contact your system administrator.\Correlation ID=hhhhhhhh-hhhh-hhhh-hhhh-hhhhhhhhhhhh');
        Observer.Expect('text len', '140');
        Observer.Expect('text id len', '36');
        Observer.Expect('text id is guid', 'Yes');
        Observer.Expect('text id = lower format 4', 'Yes');
        Observer.Expect('text brace pos', '0');
        Observer.Expect('text char before label', '92');
        Observer.Expect('text 2nd char before label', '46');
        Observer.Expect('code', 'Dialog');
        Observer.Verify();
    end;

    // A collected Internal error: Message of the collected ErrorInfo.
    [Test]
    procedure Internal_Collected_Message()
    var
        Observer: Codeunit "Probe Observer";
        EI: ErrorInfo;
        Collected: ErrorInfo;
        Count: Integer;
    begin
        EI := Internal('collected internal');
        EI.Collectible(true);
        CollectOne(EI, Collected, Count);
        Observer.Observe('count', Format(Count));
        ObserveId(Observer, 'message', Collected.Message);
        Observer.Observe('type', Format(Collected.ErrorType));
        Observer.Expect('count', '1');
        Observer.Expect('message shape', 'An error has occurred. If the error persists, please contact your system administrator.\r\nCorrelation ID=hhhhhhhh-hhhh-hhhh-hhhh-hhhhhhhhhhhh');
        Observer.Expect('message len', '141');
        Observer.Expect('message id len', '36');
        Observer.Expect('message id is guid', 'Yes');
        Observer.Expect('message id = lower format 4', 'Yes');
        Observer.Expect('message brace pos', '0');
        Observer.Expect('message char before label', '10');
        Observer.Expect('message 2nd char before label', '13');
        Observer.Expect('type', 'Internal');
        Observer.Verify();
    end;

    // A collected Internal error that leaves the collecting procedure: the text of the error raised at its end.
    [Test]
    procedure Internal_CollectedLeave_Text()
    var
        Observer: Codeunit "Probe Observer";
        EI: ErrorInfo;
    begin
        EI := Internal('leave internal');
        EI.Collectible(true);
        asserterror CollectAndLeave(EI);
        ObserveId(Observer, 'text', GetLastErrorText());
        Observer.Observe('code', GetLastErrorCode());
        Observer.Expect('text shape', 'An error has occurred. If the error persists, please contact your system administrator.\Correlation ID=hhhhhhhh-hhhh-hhhh-hhhh-hhhhhhhhhhhh');
        Observer.Expect('text len', '140');
        Observer.Expect('text id len', '36');
        Observer.Expect('text id is guid', 'Yes');
        Observer.Expect('text id = lower format 4', 'Yes');
        Observer.Expect('text brace pos', '0');
        Observer.Expect('text char before label', '92');
        Observer.Expect('text 2nd char before label', '46');
        Observer.Expect('code', 'Dialog');
        Observer.Verify();
    end;

    // Two raises of one ErrorInfo: the ids differ, the shape does not.
    [Test]
    procedure Internal_TwoRaises_IdsDiffer()
    var
        Observer: Codeunit "Probe Observer";
        EI: ErrorInfo;
        First: Text;
        Second: Text;
    begin
        EI := Internal('twice internal');
        asserterror Error(EI);
        First := GetLastErrorText();
        asserterror Error(EI);
        Second := GetLastErrorText();
        Observer.Observe('ids equal', Format(IdOf(First) = IdOf(Second)));
        Observer.Observe('shapes equal', Format(Shape(First) = Shape(Second)));
        Observer.Expect('ids equal', 'No');
        Observer.Expect('shapes equal', 'Yes');
        Observer.Verify();
    end;

    // Client neighbours: with Message, without Message, with DetailedMessage, and GetLastErrorText(true).
    [Test]
    procedure Client_Asserterror_Texts()
    var
        Observer: Codeunit "Probe Observer";
        EI: ErrorInfo;
        Empty: ErrorInfo;
    begin
        EI.Message('client text');
        EI.ErrorType(ErrorType::Client);
        asserterror Error(EI);
        Observer.Observe('message', GetLastErrorText());
        Observer.Observe('message true', GetLastErrorText(true));
        Observer.Observe('message code', GetLastErrorCode());
        ClearLastError();
        Empty.ErrorType(ErrorType::Client);
        asserterror Error(Empty);
        Observer.Observe('no message', GetLastErrorText());
        Observer.Observe('no message code', GetLastErrorCode());
        ClearLastError();
        EI.DetailedMessage('client detail');
        asserterror Error(EI);
        Observer.Observe('detailed', GetLastErrorText());
        ClearLastError();
        Observer.Observe('try client', Format(TryRaise(EI)));
        Observer.Observe('try client text', GetLastErrorText());
        Observer.Expect('message', 'client text');
        Observer.Expect('message true', 'This message had personal data removed. The original may still be in transient telemetry. Find it using the transientTelemetryId.');
        Observer.Expect('message code', 'Dialog');
        Observer.Expect('no message', '');
        Observer.Expect('no message code', 'Dialog');
        Observer.Expect('detailed', 'client text');
        Observer.Expect('try client', 'No');
        Observer.Expect('try client text', 'client text');
        Observer.Verify();
    end;

    var
        CidLbl: Label 'Correlation ID: ', Locked = true;
}
