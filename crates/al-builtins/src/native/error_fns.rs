use super::*;

pub(super) fn get_last_error_text(ctx: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let e = ctx.last_error();
    refuse_unpinned_text(&e, "GetLastErrorText")?;
    Ok(Value::Text(e.message.into()))
}

/// `GetLastErrorText(true)`: the text with the personal data removed. Which text depends on how the error was raised; the
/// pinned origins are in [`ErrorOrigin`]. No error pending, or an empty text, answers the empty text (P86LastError.al:60
/// `cleared_excl`, :192 `empty_excl`, :410 `text_excl`); an origin the corpus does not pin is refused.
pub(super) fn get_last_error_text_without_customer_content(
    ctx: &mut dyn Ctx,
    r: Option<&mut Value>,
    a: &mut [Value],
) -> Result<Value, Signal> {
    if matches!(a.first(), Some(Value::Boolean(false))) {
        return get_last_error_text(ctx, r, &mut []);
    }
    let e = ctx.last_error();
    refuse_unpinned_text(&e, "GetLastErrorText(true)")?;
    if e.message.is_empty() {
        return Ok(Value::Text(String::new().into()));
    }
    match e.origin {
        ErrorOrigin::SameText => Ok(Value::Text(e.message.into())),
        ErrorOrigin::Redacted(t) => Ok(Value::Text(t.into())),
        ErrorOrigin::Unknown => unsupported(&format!(
            "GetLastErrorText(true) of an error raised in a way the corpus does not pin for the flag (code {})",
            e.code
        )),
    }
}

pub(super) fn get_last_error_code(ctx: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let e = ctx.last_error();
    refuse_unpinned(&e, "GetLastErrorCode")?;
    Ok(Value::Text(e.code.into()))
}

/// Slots of the state of a last-error exception object (`DotNetKind::Internal`): the message, the AL call stack
/// text (`GetLastErrorCallStack` form without the trailing backslashes, `\n` between frames; may be empty).
pub const EXC_MESSAGE: usize = 0;
pub const EXC_CALL_STACK: usize = 1;

/// `GetLastErrorObject()`: the last error as the .NET exception BC holds, or null when there is none.
/// Pinned by tests/al-language-onprem-dotnet/lt-dotnet-interop/GetLastErrorObject.al for `Error('literal', args)`
/// (:138-230), `Get` without a row (:232-255) and a division by zero (:257-280); null after `ClearLastError`
/// (:283-301). Any other error is refused, naming its code.
pub(super) fn get_last_error_object(
    ctx: &mut dyn Ctx,
    _: Option<&mut Value>,
    _: &mut [Value],
) -> Result<Value, Signal> {
    use crate::dotnet_sigs::{
        AL_EXCEPTION, NCL_DIALOG_EXCEPTION, NCL_EVALUATE_EXCEPTION, RECORD_NOT_FOUND_EXCEPTION, TEST_FIELD_EXCEPTION,
    };
    let e = ctx.last_error();
    refuse_unpinned(&e, "GetLastErrorObject")?;
    if e.message.is_empty() && e.code.is_empty() {
        if e.exception != ExceptionKind::Cleared {
            return unsupported(
                "GetLastErrorObject before any error happened in the session (what BC answers is not pinned; only the answer after ClearLastError is)",
            );
        }
        return Ok(Value::DotNet(Rc::new(crate::value::DotNetObject::new(
            crate::value::DotNetKind::Internal {
                type_name: crate::dotnet_sigs::NULL_EXCEPTION.into(),
                state: std::cell::RefCell::new(Vec::new()),
            },
        ))));
    }
    let type_name = match e.exception {
        ExceptionKind::Dialog => NCL_DIALOG_EXCEPTION,
        // Error(Label, ...) and Error(Variable) raise the same exception as a literal (GetLastErrorObjectKinds.al:Kind_ErrorWithLabel,
        // _ErrorWithTextVariable, _ErrorWithErrorInfo).
        ExceptionKind::DialogComputed => NCL_DIALOG_EXCEPTION,
        ExceptionKind::RecordNotFound => RECORD_NOT_FOUND_EXCEPTION,
        ExceptionKind::DivideByZero => AL_EXCEPTION,
        // The class follows the error code for these (GetLastErrorObjectKinds.al:Kind_FailingTestFieldWithValue, _WithoutValue,
        // _ModifyOfMissingRow, _DeleteOfMissingRow, _EvaluateWithoutReturnValue).
        ExceptionKind::Unknown if e.code == "Dialog" => NCL_DIALOG_EXCEPTION,
        ExceptionKind::Unknown if e.code == "TestField" => TEST_FIELD_EXCEPTION,
        ExceptionKind::Unknown if e.code == "Evaluate" => NCL_EVALUATE_EXCEPTION,
        ExceptionKind::Unknown if e.code == "DB:RecordNotFound" => RECORD_NOT_FOUND_EXCEPTION,
        ExceptionKind::Unknown | ExceptionKind::Cleared => {
            return unsupported(&format!(
                "GetLastErrorObject after an error of code '{}' (the .NET exception BC holds for it is not pinned; only Error() with a text, Get without a row and a division by zero are)",
                if e.code == UNPINNED_ERROR {
                    "unpinned"
                } else {
                    e.code.as_str()
                }
            ));
        }
    };
    Ok(Value::DotNet(Rc::new(crate::value::DotNetObject::new(
        crate::value::DotNetKind::Internal {
            type_name: type_name.into(),
            state: std::cell::RefCell::new(vec![Value::Text(e.message.into()), Value::Text(e.call_stack.into())]),
        },
    ))))
}

pub(super) fn get_last_error_call_stack(
    ctx: &mut dyn Ctx,
    _: Option<&mut Value>,
    _: &mut [Value],
) -> Result<Value, Signal> {
    let e = ctx.last_error();
    if e.call_stack.is_empty() && !(e.message.is_empty() && e.code.is_empty()) {
        // BC returns the AL frames of the error (codeunit/TestCodeunitAlCallStack.al). The VM adds
        // them as the error unwinds (al-vm/src/callstack.rs), so an error with no frame at all
        // (raised outside any AL routine) is the only one that gets here. The answer is non-empty,
        // as in BC, but its content is a marker that every text function refuses to read
        // ([`text_of`]), so a test that inspects the frames is not run while one that only calls
        // the function (error-handling/TestGetLastError.al) still passes.
        return Ok(Value::Text(UNTRACKED_CALL_STACK.to_string().into()));
    }
    if e.call_stack.is_empty() {
        return Ok(Value::Text(String::new().into()));
    }
    // The frames are separated by a backslash and the text ends with one, the runner's frames
    // below the test's (session/TestSessionInformationCallstackContracts.al:200-218).
    let mut out = String::with_capacity(e.call_stack.len() + 64);
    for frame in e.call_stack.split('\n') {
        out.push_str(frame);
        out.push('\\');
    }
    out.push_str(&ctx.runner_call_stack());
    Ok(Value::Text(out.into()))
}

/// `CodeCoverageRefresh()`: loads what was recorded into the Code Coverage tables. While logging is on a read of the table holds
/// nothing until this call or until logging stops; after logging stopped the call changes nothing (platform/lt-code-coverage-refresh/
/// CodeCoverageRefreshLogged.al: CodeCoverageRefresh_WhileLogging_Result, _AfterLoggingStopped_Result, _NeverLogging_Result).
pub(super) fn code_coverage_refresh(
    ctx: &mut dyn Ctx,
    _: Option<&mut Value>,
    _: &mut [Value],
) -> Result<Value, Signal> {
    ctx.code_coverage_refresh()?;
    Ok(Value::Null)
}

pub(super) fn clear_last_error(ctx: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    ctx.set_last_error(AlError {
        exception: ExceptionKind::Cleared,
        ..Default::default()
    });
    Ok(Value::Null)
}

/// `Error(Text, Args...)`. The code is `Dialog` (probes-needed: Microsoft's
/// test libraries and the corpus Assert's DialogTxt use it; no corpus test
/// pins it directly).
pub(super) fn error_text(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
    error_text_with(ctx, args, ErrorTextForm::Literal)
}

/// `Error(Text, ...)` with a text written in a form whose flag text the corpus does not pin (a global Text variable, a
/// field, a procedure's result, a local Label).
pub(super) fn error_text_not_literal(
    ctx: &mut dyn Ctx,
    _: Option<&mut Value>,
    args: &mut [Value],
) -> Result<Value, Signal> {
    error_text_with(ctx, args, ErrorTextForm::Other)
}

/// `Error(Label, ...)` and `Error(TextConst, ...)` of a global variable: the flag answers the text as written, before the
/// arguments are substituted (LastErrorTextExcludeLabel.al:136, :155, :159, :266; lt-last-error-text-forms/
/// LastErrorTextExcludeTextConst.al:57-66, :81-85).
pub(super) fn error_text_constant(
    ctx: &mut dyn Ctx,
    _: Option<&mut Value>,
    args: &mut [Value],
) -> Result<Value, Signal> {
    error_text_with(ctx, args, ErrorTextForm::Constant)
}

/// `Error(Text, ...)` of a local Text or Code variable, a concatenation or a `StrSubstNo` call: the flag answers what it
/// answers for a literal (LastErrorTextExcludeTextVariable.al:79-93, :131-142; LastErrorTextExcludeLabel.al:163).
pub(super) fn error_text_computed(
    ctx: &mut dyn Ctx,
    _: Option<&mut Value>,
    args: &mut [Value],
) -> Result<Value, Signal> {
    error_text_with(ctx, args, ErrorTextForm::Computed)
}

/// `Record.FieldError(Field, Label)` of a global Label or TextConst (`ErrorTextForm::Constant`).
pub(super) fn field_error_constant(
    ctx: &mut dyn Ctx,
    r: Option<&mut Value>,
    args: &mut [Value],
) -> Result<Value, Signal> {
    ctx.field_error_with_form(r, args, ErrorTextForm::Constant)
}

/// `Record.FieldError(Field, Text)` of a local Text or Code variable, a concatenation or a literal
/// (`ErrorTextForm::Computed`; a literal answers the same, P86LastError.al:273).
pub(super) fn field_error_computed(
    ctx: &mut dyn Ctx,
    r: Option<&mut Value>,
    args: &mut [Value],
) -> Result<Value, Signal> {
    ctx.field_error_with_form(r, args, ErrorTextForm::Computed)
}

pub(super) fn error_text_with(ctx: &mut dyn Ctx, args: &mut [Value], form: ErrorTextForm) -> Result<Value, Signal> {
    let template = text_arg(args, 0)?;
    let mut message = substitute(&template, &args[1..], ctx)?;
    // The error text of `Error(Text)` without arguments shows a CR LF as one backslash (misc/lt-system-functions/ErrorMessage.al:
    // 606-618, LT_BU_Message_BackslashAndAppendLine: 17 characters), and so does a CR LF in an argument, alone, between other text
    // or after a backslash the text holds (ErrorMessage.al:684-718, PYC_Error_CrLfAndLoneCrLf: `a\b`, `xa\by`, `p\q a\b`). A
    // CR LF in the text itself next to arguments is not pinned.
    if message.contains("\r\n") {
        if args.len() > 1 && template.contains("\r\n") {
            return unsupported(
                "Error with arguments whose own text holds a CR LF: the error text is not pinned (docs/probes-needed.md)",
            );
        }
        message = message.replace("\r\n", "\\");
    }
    let (origin, exception) = match form {
        ErrorTextForm::Literal => (
            ErrorOrigin::Redacted(USE_ERROR_WITH_A_TEXT_CONSTANT.into()),
            ExceptionKind::Dialog,
        ),
        ErrorTextForm::Computed => (
            ErrorOrigin::Redacted(USE_ERROR_WITH_A_TEXT_CONSTANT.into()),
            ExceptionKind::DialogComputed,
        ),
        // A CR LF in the label is not pinned for the flag.
        ErrorTextForm::Constant if !template.contains("\r\n") => (
            ErrorOrigin::Redacted(template.to_string()),
            ExceptionKind::DialogComputed,
        ),
        ErrorTextForm::Constant | ErrorTextForm::Other => (ErrorOrigin::Unknown, ExceptionKind::DialogComputed),
    };
    Err(Signal::Error(Box::new(AlError {
        message,
        code: "Dialog".into(),
        origin,
        exception,
        ..Default::default()
    })))
}
