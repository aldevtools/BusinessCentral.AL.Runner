use super::*;
#[test]
fn the_table_finds_what_the_scan_found_for_every_id_and_nothing_for_other_text() {
    let mut ids = registered_ids();
    ids.extend(crate::platform_native::entries().map(|(i, _)| i));
    assert!(ids.len() > 500, "{}", ids.len());
    for id in ids {
        let (a, b) = (source_in_table(id), source_by_scan(id));
        assert!(a.is_some() && b.is_some() && lookup(id).is_some(), "{id}");
        assert_eq!(a, b, "{id}");
    }
    for id in ["", "Nope.Nothing#9", "System.StrLen", "system.strlen#1"] {
        assert!(lookup(id).is_none() && source_by_scan(id).is_none(), "{id}");
    }
}

// TestField with an ErrorInfo is implemented (issue 41): a check that holds raises nothing, a failing one
// is refused at run time (al-vm recmeta.rs `test_field`, al-vm/tests/store/scm_service_41.rs). Issue 42 refused
// it at lowering; that entry is gone because `lookup` now finds the ids.
#[test]
fn test_field_with_an_error_info_is_implemented_not_refused_at_lowering() {
    for id in ["Record.TestField#2", "Record.TestField#4"] {
        assert!(refusal(id).is_none(), "{id}");
        assert!(lookup(id).is_some(), "{id}");
    }
}

// GetDotNetType is pinned by the OnPrem DotNet corpus app (tests/al-language-onprem-dotnet/lt-dotnet-interop,
// DotNetType*.al); the VM runs it through `Ctx::session_op`.
#[test]
fn get_dot_net_type_is_implemented_not_refused_at_lowering() {
    assert!(refusal("System.GetDotNetType#1").is_none());
    assert!(lookup("System.GetDotNetType#1").is_some());
}

#[derive(Default)]
pub(crate) struct TestCtx {
    err: AlError,
    install: bool,
    in_test: bool,
    result_ignored: bool,
}
impl FormatCtx for TestCtx {
    fn enum_name(&self, _id: u32, o: i32) -> String {
        format!("E{o}")
    }
}
impl Ctx for TestCtx {
    fn last_error(&self) -> AlError {
        self.err.clone()
    }
    fn set_last_error(&mut self, e: AlError) {
        self.err = e;
    }
    fn record(&mut self, m: RecordOp, _: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
        Err(Signal::Unsupported(m.name().into()))
    }
    fn commit(&mut self) -> Result<(), Signal> {
        Ok(())
    }
    fn new_guid(&mut self) -> [u8; 16] {
        [1; 16]
    }
    fn install_scope(&self) -> bool {
        self.install
    }
    fn in_test_run(&self) -> bool {
        self.in_test
    }
    fn ret_used(&self) -> bool {
        !self.result_ignored
    }
}

#[test]
fn variant_conversions_and_log_message() {
    assert!(matches!(
        call("System.Variant2Date#1", vec![Value::Date(5, false)]),
        Ok(Value::Date(5, false))
    ));
    assert!(matches!(
        call("System.Variant2Time#1", vec![Value::Time(7)]),
        Ok(Value::Time(7))
    ));
    assert!(matches!(
        call("System.Variant2Date#1", vec![Value::Integer(1)]),
        Err(Signal::Unsupported(_))
    ));
    assert!(matches!(
        call("Session.LogMessage#2", vec![t("E"), t("m")]),
        Ok(Value::Null)
    ));
    // The security audit log has no effect AL can see, like LogSecurityAudit.
    assert!(matches!(
        call(
            "Session.LogAuditMessage#1",
            vec![
                t("D"),
                Value::Integer(0),
                Value::Integer(0),
                Value::Integer(1),
                Value::Integer(0)
            ]
        ),
        Ok(Value::Null)
    ));
}

#[test]
fn start_session_answers_false_in_install_and_is_refused_in_a_test_run() {
    let run = |ctx: &mut TestCtx| {
        let mut args = vec![Value::Integer(777), Value::Integer(60448)];
        lookup("Session.StartSession#1").unwrap()(ctx, None, &mut args)
    };
    // install/TestInstallStartSession_Tests.al:51-65
    let mut install = TestCtx {
        install: true,
        ..Default::default()
    };
    assert!(matches!(run(&mut install), Ok(Value::Boolean(false))));
    let mut plain = TestCtx {
        install: true,
        result_ignored: true,
        ..Default::default()
    };
    assert!(matches!(run(&mut plain), Ok(Value::Null)));
    // session/TestStartSessionRecord.al:57-66
    let mut test = TestCtx {
        in_test: true,
        ..Default::default()
    };
    assert!(matches!(run(&mut test), Err(Signal::Error(e)) if e.message.contains(
        "can only be started in tests that are run by a TestRunner that has TestIsolation set to Disabled")));
    assert!(matches!(run(&mut TestCtx::default()), Err(Signal::Unsupported(_))));
}

pub(crate) fn call(id: &str, args: Vec<Value>) -> Result<Value, Signal> {
    let f = lookup(id).unwrap_or_else(|| panic!("{id} not registered"));
    let mut args = args;
    f(&mut TestCtx::default(), None, &mut args)
}
pub(crate) fn t(s: &str) -> Value {
    Value::Text(s.into())
}
fn text(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}
fn int(r: Result<Value, Signal>) -> i32 {
    match r {
        Ok(Value::Integer(i)) => i,
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_registered_id_is_in_the_catalog() {
    let cat = crate::catalog::Catalog::load();
    let ids: Vec<String> = cat.signatures.iter().map(|s| cat.id(s)).collect();
    // Ids lowering picks for a call of a catalog overload that needs another implementation (`Evaluate` into an Option variable).
    let derived = [
        "System.EvaluateVariableOption#1",
        "System.EvaluateBounded#1",
        "System.EvaluateOptionField#1",
        "System.FormatVariant#1",
        "TextBuilder.AppendVariant#1",
        "RecordRef.GetTableVariant#1",
        "TestField.SetValueVariant#1",
        "System.FormatDecimalField#1",
        "System.FormatAutoFormatField#1",
        "System.OptionCaptionML#1",
        "Dialog.ErrorNotLiteral#1",
        "Dialog.ErrorConstant#1",
        "Dialog.ErrorComputed#1",
        "Record.FieldErrorConstant#1",
        "Record.FieldErrorComputed#1",
        "TextBuilder.AppendFieldValue#1",
        "Media.ToVariant#1",
        "Interface.AssertAssigned#1",
        "Variant.IntoInteger#1",
        "Variant.IntoOption#1",
        "Variant.Into#1",
        "InStream.OwnerOutOfScope#1",
        "System.TranslatedText#1",
        "System.TextConstText#1",
    ];
    for id in registered_ids().into_iter().filter(|id| !derived.contains(id)) {
        assert!(ids.iter().any(|i| i == id), "{id} is not a catalog overload id");
    }
}

#[test]
fn format_with_length_and_standard_format() {
    assert_eq!(text(call("System.Format#1", vec![Value::Integer(42)])), "42");
    assert_eq!(
        text(call("System.Format#1", vec![Value::Integer(42), Value::Integer(5)])),
        "   42"
    );
    assert_eq!(
        text(call(
            "System.Format#1",
            vec![Value::Boolean(true), Value::Integer(0), Value::Integer(2)]
        )),
        "1"
    );
}

// A standard format number a type does not have raises the pinned error, not a panic that takes the whole codeunit down
// (text/TestProbeTextDateFormatContracts.al:Dates_Format_Time, Dates_Format_DateTime, Dates_Format_Duration).
#[test]
fn format_number_a_type_does_not_have_raises_the_pinned_error() {
    for (v, ty) in [
        (Value::Duration(3_600_000), "Duration"),
        (Value::Time(43_200_001), "Time"),
        (Value::DateTime(5), "DateTime"),
    ] {
        let args = vec![v, Value::Integer(0), Value::Integer(7)];
        let Err(Signal::Error(e)) = call("System.Format#1", args) else {
            panic!("{ty}: format 7 must raise");
        };
        assert_eq!(
            (e.message.as_str(), e.code.as_str()),
            (
                format!("Standard format number 7 does not exist for the type '{ty}'.").as_str(),
                "InvalidStandardFormat"
            )
        );
    }
}

#[test]
fn a_duration_format_number_that_does_not_exist_raises() {
    // text/TestProbeTextDateFormatContracts.al:Dates_Format_Duration, `duration_fmt_5`.
    let Err(Signal::Error(e)) = call(
        "System.Format#1",
        vec![Value::Duration(86_400_000), Value::Integer(0), Value::Integer(5)],
    ) else {
        panic!("format 5 of a Duration must raise");
    };
    assert_eq!(e.code, "InvalidStandardFormat");
}

#[test]
fn the_call_stack_of_a_recorded_error_is_refused_not_invented() {
    // codeunit/TestCodeunitAlCallStack.al expects AL frames such as `(CodeUnit 60211)`. The VM
    // fills them in (al-vm/tests/language/call_stack.rs); an error that has no frame gets a marker no text function reads.
    let mut ctx = TestCtx::default();
    let f = lookup("System.GetLastErrorCallStack#1").unwrap();
    assert!(matches!(f(&mut ctx, None, &mut []), Ok(Value::Text(ref s)) if s.is_empty()));
    ctx.set_last_error(AlError {
        message: "boom".into(),
        code: "Dialog".into(),
        call_stack: String::new(),
        ..Default::default()
    });
    let Ok(stack @ Value::Text(_)) = f(&mut ctx, None, &mut []) else {
        panic!("a call stack is returned after an error")
    };
    assert!(matches!(&stack, Value::Text(s) if !s.is_empty()));
    assert!(text_of(&stack).is_none(), "its content cannot be inspected");
}

// session/TestSystemExtended.al:218-256 pins the range; the values are the documentation's formula.
#[test]
fn strchecksum_follows_the_documented_formula() {
    let n = |v: Result<Value, Signal>| int(v);
    assert_eq!(n(call("Text.StrCheckSum#1", vec![t("12345")])), 5);
    assert_eq!(n(call("Text.StrCheckSum#1", vec![t("12345"), t("5432")])), 5); // weights 54321: 35 mod 10 = 5
    assert_eq!(
        n(call(
            "Text.StrCheckSum#1",
            vec![t("12345"), t("5432"), Value::Integer(10)]
        )),
        5
    );
    // The example of the documentation: (7 - (4*1 + 3*2 + 7*3 + 8*4) mod 7) mod 7 = 0.
    assert_eq!(
        n(call(
            "Text.StrCheckSum#1",
            vec![t("4378"), t("1234"), Value::Integer(7)]
        )),
        0
    );
    assert_eq!(n(call("Text.StrCheckSum#1", vec![t("")])), 0);
    // types/TestProbeScalarContracts.al:StrCheckSum_NonDigitSource and the StrCheckSum tests after it.
    for (args, message) in [
        (vec![t("12a")], "The value string must be numeric."),
        (
            vec![t("12"), t("123")],
            "The weight parameter cannot be longer than the value string.",
        ),
    ] {
        let Err(Signal::Error(e)) = call("Text.StrCheckSum#1", args) else {
            panic!("{message}");
        };
        assert_eq!((e.message.as_str(), e.code.as_str()), (message, "ALSystemString"));
    }
}

#[test]
fn strlen_counts_utf16() {
    assert_eq!(int(call("Text.StrLen#1", vec![t("abc")])), 3);
    assert_eq!(int(call("Text.StrLen#1", vec![t("😀")])), 2);
}

#[test]
fn strpos_is_one_based_and_zero_when_absent() {
    assert_eq!(int(call("Text.StrPos#1", vec![t("hello"), t("ll")])), 3);
    assert_eq!(int(call("Text.StrPos#1", vec![t("hello"), t("x")])), 0);
}

#[test]
fn copystr() {
    assert_eq!(
        text(call(
            "Text.CopyStr#1",
            vec![t("Hello World"), Value::Integer(1), Value::Integer(5)]
        )),
        "Hello"
    );
    assert_eq!(text(call("Text.CopyStr#1", vec![t("Hello"), Value::Integer(3)])), "llo");
    assert_eq!(
        text(call(
            "Text.CopyStr#1",
            vec![t("Hello"), Value::Integer(4), Value::Integer(10)]
        )),
        "lo"
    );
    // text/TestProbeTextDateFormatContracts.al:Text_SubstringBounds, `copystr_start0`.
    let Err(Signal::Error(e)) = call("Text.CopyStr#1", vec![t("Hello"), Value::Integer(0)]) else {
        panic!("CopyStr from position 0 must raise");
    };
    assert_eq!(e.code, "OutsidePermittedRange");
}

#[test]
fn strsubstno_formats_its_arguments() {
    assert_eq!(
        text(call(
            "Text.StrSubstNo#1",
            vec![t("%1 and %2"), Value::Integer(7), Value::Boolean(true)]
        )),
        "7 and Yes"
    );
    assert_eq!(text(call("Text.StrSubstNo#1", vec![t("no args")])), "no args");
}

#[test]
fn case_conversion() {
    assert_eq!(text(call("Text.UpperCase#1", vec![t("abc")])), "ABC");
    assert_eq!(text(call("Text.LowerCase#1", vec![t("ABC")])), "abc");
}

// types/lt-evaluate-format-lasterror-setview/P86StrSubstNo.al:Sub_FieldWidth and
// text/lt-text-functions/StrSubstNo.Codeunit.al:SubstNoHash.
#[test]
fn strsubstno_fills_hash_fields_to_their_width() {
    let sub = |f: &str, a: Vec<Value>| {
        let mut args = vec![t(f)];
        args.extend(a);
        text(call("Text.StrSubstNo#1", args))
    };
    let a = || vec![t("a")];
    assert_eq!(sub("#1####", a()), "a     ");
    assert_eq!(sub("#1#", a()), "a  ");
    assert_eq!(sub("#1", a()), "a ");
    assert_eq!(sub("#####", a()), "#####");
    assert_eq!(sub("#1####x#2####", vec![t("a"), t("b")]), "a     xb     ");
    assert_eq!(sub("#2####", a()), "#2####");
    assert_eq!(sub("#1####%1", a()), "a     a");
    assert_eq!(sub("%1#1####", a()), "aa     ");
    assert_eq!(sub("#1####", vec![]), "#1####");
    assert_eq!(sub("# 1###", a()), "# 1###");
    assert_eq!(sub("#0###", a()), "#0###");
    assert_eq!(sub("##1##", a()), "a    ");
    assert_eq!(sub("#####1", a()), "a     ");
    assert_eq!(sub("#1#1", a()), "a  1");
    assert_eq!(sub("#1", vec![t("abcdef")]), "**");
    assert_eq!(sub("#1###", vec![t("abcdef")]), "*****");
    assert_eq!(sub("#1####", vec![Value::Integer(12)]), "12    ");
    assert_eq!(sub("#1", vec![t("")]), "  ");
    assert_eq!(sub("#1###", vec![Value::Boolean(true)]), "Yes  ");
}

#[test]
fn error_substitutes_and_raises() {
    let Err(Signal::Error(e)) = call("Dialog.Error#2", vec![t("bad %1"), Value::Integer(3)]) else {
        panic!()
    };
    assert_eq!(e.message, "bad 3");
}

#[test]
fn last_error_round_trip() {
    let mut ctx = TestCtx::default();
    ctx.set_last_error(AlError {
        message: "boom".into(),
        code: "Dialog".into(),
        call_stack: String::new(),
        ..Default::default()
    });
    let get = lookup("System.GetLastErrorText#1").unwrap();
    assert!(matches!(get(&mut ctx, None, &mut []), Ok(Value::Text(ref s)) if s == "boom"));
    let code = lookup("System.GetLastErrorCode#1").unwrap();
    assert!(matches!(code(&mut ctx, None, &mut []), Ok(Value::Text(ref s)) if s == "Dialog"));
    lookup("System.ClearLastError#1").unwrap()(&mut ctx, None, &mut []).unwrap();
    assert_eq!(ctx.last_error().message, "");
    assert_eq!(ctx.last_error().exception, ExceptionKind::Cleared);
}

#[test]
fn abs() {
    let r = call("System.Abs#1", vec![Value::Decimal((-2).into())]);
    assert!(matches!(r, Ok(Value::Decimal(d)) if d == 2.into()));
}

#[test]
fn variant_type_tests_use_the_receiver() {
    let f = lookup("Variant.IsInteger#1").unwrap();
    let mut v = Value::Integer(1);
    assert!(matches!(
        f(&mut TestCtx::default(), Some(&mut v), &mut []),
        Ok(Value::Boolean(true))
    ));
    let mut v = Value::Option(0, std::sync::Arc::from(vec![]));
    let f = lookup("Variant.IsOption#1").unwrap();
    assert!(matches!(
        f(&mut TestCtx::default(), Some(&mut v), &mut []),
        Ok(Value::Boolean(true))
    ));
}
