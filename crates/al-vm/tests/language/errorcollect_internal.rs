//! A collected `ErrorInfo` read back through `GetCollectedErrors` (al-runner issue 5766; corpus
//! misc/lt-probes-needed-2/PnBErrorsCollections.al:263-375, RO_CollectedErrorInfo_*): an Internal error's Message is the generic
//! text with its correlation id after a line break, its other properties are kept, and Callstack lists the frames from the
//! `Error` call out to the caller, innermost first; the rest of its text is not pinned. Left at the end of the scope it raises the generic
//! text with a backslash before the correlation id.

const SRC: &str = r#"
codeunit 70550 "LT PnB ErrorInfo"
{
    local procedure RoRaiseFull(ErrType: ErrorType; IsCollectible: Boolean)
    var
        Info: ErrorInfo;
    begin
        Info.Message := 'Full message';
        Info.DetailedMessage := 'Detailed message';
        Info.ErrorType := ErrType;
        Info.CustomDimensions.Add('Dim1', 'Val1');
        Info.Collectible := IsCollectible;
        Error(Info);
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure RoCollect(ErrType: ErrorType; Clear: Boolean; var Count: Integer; var Info: ErrorInfo)
    var
        Errors: List of [ErrorInfo];
    begin
        RoRaiseFull(ErrType, true);
        Errors := GetCollectedErrors(Clear);
        Count := Errors.Count();
        Errors.Get(1, Info);
    end;

    [ErrorBehavior(ErrorBehavior::Collect)]
    local procedure RoCollectInternalAndLeave()
    begin
        RoRaiseFull(ErrorType::Internal, true);
    end;

    local procedure Show(Info: ErrorInfo): Text
    var
        DimKey: Text;
        Dims: Text;
    begin
        foreach DimKey in Info.CustomDimensions.Keys() do
            Dims += DimKey + '=' + Info.CustomDimensions.Get(DimKey) + ',';
        exit('type=' + Format(Info.ErrorType) + ';dims=' + Format(Info.CustomDimensions.Count()) + ':' + Dims +
            ';detailed=' + Info.DetailedMessage + ';collectible=' + Format(Info.Collectible) + '#' + Info.Message);
    end;

    procedure Internal(): Text
    var
        Info: ErrorInfo;
        Count: Integer;
    begin
        RoCollect(ErrorType::Internal, true, Count, Info);
        exit(Format(Count) + '#' + Show(Info));
    end;

    procedure Client(): Text
    var
        Info: ErrorInfo;
        Count: Integer;
    begin
        RoCollect(ErrorType::Client, true, Count, Info);
        exit(Format(Count) + '#' + Show(Info));
    end;

    procedure Stack(): Text
    var
        Info: ErrorInfo;
        Count: Integer;
    begin
        RoCollect(ErrorType::Internal, true, Count, Info);
        exit(Info.Callstack);
    end;

    procedure StackNotEmpty(): Boolean
    var
        Info: ErrorInfo;
        Count: Integer;
    begin
        RoCollect(ErrorType::Client, true, Count, Info);
        exit(Info.Callstack <> '');
    end;

    procedure StackLines(): Integer
    var
        Info: ErrorInfo;
        Count: Integer;
        Lf: Text[1];
    begin
        RoCollect(ErrorType::Client, true, Count, Info);
        Lf[1] := 10;
        exit(Info.Callstack.Split(Lf).Count());
    end;

    procedure KeptAtScopeExit(): Text
    var
        Info: ErrorInfo;
        Count: Integer;
    begin
        asserterror RoCollect(ErrorType::Internal, false, Count, Info);
        exit(Info.Message + '#' + GetLastErrorText() + '#' + GetLastErrorCode());
    end;

    procedure LeftAtScopeExit(): Text
    begin
        asserterror RoCollectInternalAndLeave();
        exit(GetLastErrorText() + '#' + GetLastErrorCode());
    end;
}
"#;

const GENERIC: &str = "An error has occurred. If the error persists, please contact your system administrator.";

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "LT PnB ErrorInfo", routine))
}

/// The correlation id is a fresh Guid each time: `{8-4-4-4-12}`.
fn strip_id(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("Correlation ID: ") {
        let after = &rest[i + "Correlation ID: ".len()..];
        assert!(
            after.starts_with('{') && after.len() >= 38 && after.as_bytes()[37] == b'}',
            "{s}"
        );
        out.push_str(&rest[..i]);
        out.push_str("Correlation ID: x");
        rest = &after[38..];
    }
    out.push_str(rest);
    out
}

/// PnBErrorsCollections.al:329-358 RO_CollectedErrorInfo_Internal_And_Client `internal`, `internal message`, `client`.
#[test]
fn a_collected_internal_error_hides_its_message_and_keeps_the_rest() {
    assert_eq!(
        strip_id(&run("Internal")),
        format!(
            "1#type=Internal;dims=1:Dim1=Val1,;detailed=Detailed message;collectible=Yes#{GENERIC}\r\nCorrelation ID: x"
        )
    );
    assert_eq!(
        run("Client"),
        "1#type=Client;dims=1:Dim1=Val1,;detailed=Detailed message;collectible=Yes#Full message"
    );
}

/// RO_CollectedErrorInfo_Internal_And_Client `internal callstack`: from the Error call out, innermost first. The test keeps only
/// the lines of its own app after it drops CR and splits at LF, so the rest of the text (other apps' frames, CR, a trailing
/// separator) is not pinned: the stack ends with the untracked mark and no text function reads it.
#[test]
fn a_collected_error_has_the_call_stack_of_its_error_call() {
    let stack = run("Stack");
    let (frames, rest) = stack
        .split_once(al_builtins::native::UNTRACKED_CALL_STACK)
        .unwrap_or_else(|| panic!("no untracked mark: {stack:?}"));
    assert_eq!(rest, "", "{stack:?}");
    let frames: Vec<&str> = frames.split('\n').filter(|f| !f.is_empty()).collect();
    assert_eq!(frames.len(), 3, "{stack:?}");
    assert!(
        frames[0].starts_with("\"LT PnB ErrorInfo\"(CodeUnit 70550).RoRaiseFull line 9"),
        "{stack:?}"
    );
    assert!(
        frames[1].starts_with("\"LT PnB ErrorInfo\"(CodeUnit 70550).RoCollect line 4"),
        "{stack:?}"
    );
    assert!(
        frames[2].starts_with("\"LT PnB ErrorInfo\"(CodeUnit 70550).Stack line 5"),
        "{stack:?}"
    );
}

/// al-runner issue 5766 Q4_CollectedErrorInfo_HasCallstack: the Callstack of a collected error is not empty. How many lines it
/// has is not pinned: the split refuses.
#[test]
fn a_collected_call_stack_is_not_empty_and_its_lines_are_not_read() {
    assert!(al_testkit::boolean(al_testkit::run_in(
        SRC,
        "LT PnB ErrorInfo",
        "StackNotEmpty"
    )));
    let r = al_testkit::run_in(SRC, "LT PnB ErrorInfo", "StackLines");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}

/// PnBErrorsCollections.al:360-375 RO_CollectedErrorInfo_Internal_NotCleared_ScopeExit.
#[test]
fn a_collected_internal_error_left_at_the_end_of_the_scope_raises_the_generic_text() {
    assert_eq!(
        strip_id(&run("KeptAtScopeExit")),
        format!("{GENERIC}\r\nCorrelation ID: x#{GENERIC}\\Correlation ID: x#Dialog")
    );
    assert_eq!(
        strip_id(&run("LeftAtScopeExit")),
        format!("{GENERIC}\\Correlation ID: x#Dialog")
    );
}
