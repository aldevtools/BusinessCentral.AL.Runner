//! The allow-list of rendered content (ADR-040 amendment 2026-11-12, A3, A5; loss modes L1, L6, L12, L16).

use al_builtins::builtin_id::BuiltinId;
use al_builtins::native::Signal;
use al_builtins::rendered_text::{self, Form, RgbaReplace};
use al_builtins::value::Value;

fn marked() -> Value {
    let n = rendered_text::register("Guard Report", "Html", "WordOne").expect("a render");
    Value::Text(rendered_text::mark_text(n, Form::Whole).into())
}

/// A render whose save row measured the rgba replace (al-vm `report_save`, the rgba column).
const MEASURED: RgbaReplace = RgbaReplace::Measured;

fn marked_with(rgba: RgbaReplace, form: Form) -> Value {
    let n = rendered_text::register_with("Guard Report", "Html", "WordOne", rgba).expect("a render");
    Value::Text(rendered_text::mark_text(n, form).into())
}

/// The builtins table A3 allows a marked argument or receiver for (one name per row, with its row's position). Every other
/// builtin of the catalog refuses a marked value in every position (L1): the guard runs before the native, whatever the native
/// does with its argument.
const ALLOWED_AT_ARG0: &[&str] = &[
    "TextBuilder.Append#1",
    "TextBuilder.AppendLine#1",
    "BigText.AddText#1",
    "BigText.AddText#2",
    "System.Format#1",
    "List.Add#1",
    "Session.LogMessage#1",
    "Session.LogMessage#2",
    "Variant.Into#1",
    "System.Clear#1",
];

#[test]
fn every_builtin_outside_the_table_refuses_a_marked_argument() {
    let v = marked();
    for name in BuiltinId::all().map(BuiltinId::name) {
        let r = rendered_text::guard(name, None, std::slice::from_ref(&v), false);
        // The InStream.Read overloads take it only as the variable they overwrite (A2; ReadText reads a line and refuses); the
        // stream writes need an OutStream receiver.
        let listed = ALLOWED_AT_ARG0.contains(&name) || name.starts_with("InStream.Read#");
        match (listed, r) {
            (true, Ok(())) | (false, Err(Signal::Unsupported(_))) => {}
            (listed, r) => panic!("{name}: listed {listed}, got {r:?}"),
        }
    }
}

#[test]
fn every_builtin_outside_the_table_refuses_a_marked_receiver() {
    let v = marked();
    for name in BuiltinId::all().map(BuiltinId::name) {
        let r = rendered_text::guard(name, Some(&v), &[], false);
        let listed = name.starts_with("Variant.Is")
            || matches!(
                name,
                "TextBuilder.AppendLine#1" | "TextBuilder.ToText#1" | "Variant.Into#1"
            )
            || name.starts_with("List.")
            || name.starts_with("Dictionary.");
        match r {
            Ok(()) if listed => {}
            Err(Signal::Unsupported(_)) => {}
            r => panic!("{name}: listed {listed}, got {r:?}"),
        }
    }
}

#[test]
fn a_write_needs_a_utf8_stream_and_an_unused_count() {
    // No OutStream receiver: refused.
    assert!(rendered_text::guard("OutStream.WriteText#1", None, &[marked()], false).is_err());
}

#[test]
fn strsubstno_takes_a_marked_value_only_as_a_percent_field() {
    let t = |s: &str| Value::Text(s.to_string().into());
    let ok = rendered_text::guard("Text.StrSubstNo#1", None, &[t("<%1>"), marked()], false);
    assert!(ok.is_ok(), "{ok:?}");
    for template in ["#1####", "%1 #1", "%10", "%2"] {
        let r = rendered_text::guard("Text.StrSubstNo#1", None, &[t(template), marked(), t("x")], false);
        assert!(r.is_err(), "{template}");
    }
    assert!(rendered_text::guard("Text.StrSubstNo#1", None, &[marked(), t("x")], false).is_err());
}

#[test]
fn every_row_names_its_amendment_row_and_the_form_of_its_result() {
    let mut cites: Vec<&str> = rendered_text::allowed_rows().map(|(_, cite, _)| cite).collect();
    cites.sort_unstable();
    cites.dedup();
    // P5 and P6 on a text are answered by `ops::compare` (P5 has the row of `CompressArray`), P9 by `Variant.Is*` (and the row
    // of `ArrayLen`), P12 by `DOTNET_ALLOWED`, P11 by `pages/rendered_field.rs`.
    assert_eq!(cites, ["A2", "P1", "P10", "P2", "P3", "P4", "P5", "P7", "P8", "P9"]);
    // One row gives the changed form, `Format(BigText)`; every other result moves the value unchanged (whole form kept).
    let changed: Vec<&str> = rendered_text::allowed_rows()
        .filter(|(_, _, form)| *form == Some(Form::Changed))
        .map(|(name, _, _)| name)
        .collect();
    assert_eq!(changed, ["System.Format#1"]);
    assert!(
        rendered_text::allowed_rows().all(|(_, _, form)| form != Some(Form::Whole)),
        "no row turns a changed text into a whole one"
    );
}

fn big_text(s: &str) -> Value {
    use al_builtins::library_kind::LibraryKind;
    use al_builtins::value::LibraryValue;
    Value::Library(
        LibraryKind::BigText,
        std::rc::Rc::new(std::cell::RefCell::new(LibraryValue::Text(s.to_string()))),
    )
}

#[test]
fn getsubtext_takes_a_marked_bigtext_only_into_a_text_from_position_one_with_the_count_unused() {
    let Value::Text(m) = marked() else { unreachable!() };
    let recv = big_text(m.as_str());
    let t = || Value::Text(String::new().into());
    let guard = |name: &str, args: &[Value], used: bool| rendered_text::guard(name, Some(&recv), args, used);
    let ok = guard("BigText.GetSubText#2", &[t(), Value::Integer(1)], false);
    assert!(ok.is_ok(), "{ok:?}");
    // Another position, a length, a used count, a position that is no Integer, the BigText target overload.
    for pos in [0, 2, 3, 4, 5, 99] {
        assert!(
            guard("BigText.GetSubText#2", &[t(), Value::Integer(pos)], false).is_err(),
            "{pos}"
        );
    }
    assert!(
        guard(
            "BigText.GetSubText#2",
            &[t(), Value::Integer(1), Value::Integer(5)],
            false
        )
        .is_err()
    );
    assert!(guard("BigText.GetSubText#2", &[t(), Value::Integer(1)], true).is_err());
    assert!(guard("BigText.GetSubText#2", &[t(), Value::BigInteger(1)], false).is_err());
    assert!(guard("BigText.GetSubText#1", &[big_text(""), Value::Integer(1)], false).is_err());
    assert!(guard("BigText.GetSubText#1", &[big_text(""), Value::Integer(4)], false).is_err());
}

#[test]
fn format_takes_a_marked_text_or_bigtext_alone() {
    let Value::Text(m) = marked() else { unreachable!() };
    for v in [marked(), big_text(m.as_str())] {
        assert!(rendered_text::guard("System.Format#1", None, std::slice::from_ref(&v), false).is_ok());
        let with_length = [v, Value::Integer(10)];
        assert!(rendered_text::guard("System.Format#1", None, &with_length, false).is_err());
    }
}

#[test]
fn a_collection_the_table_allows_is_not_walked() {
    // `List.Add` and `Dictionary.Add` of a known text: the guard answers from the table and never looks into the receiver.
    // A receiver that would panic when borrowed shows it: the list is mutably borrowed while the guard runs.
    let list = std::rc::Rc::new(std::cell::RefCell::new(al_builtins::value::ListVec::default()));
    let v = Value::List(list.clone());
    let _held = list.borrow_mut();
    let known = Value::Text("known".to_string().into());
    assert!(rendered_text::guard("List.Add#1", Some(&v), std::slice::from_ref(&known), false).is_ok());
    assert!(rendered_text::guard("List.Get#1", Some(&v), &[Value::Integer(1)], false).is_ok());
}

#[test]
fn a_dotnet_call_with_a_marked_argument_refuses_unless_it_is_the_rgba_replace() {
    let t = |s: &str| Value::Text(s.to_string().into());
    let regex = "System.Text.RegularExpressions.Regex";
    let entry = &rendered_text::DOTNET_ALLOWED[0];
    let measured = || marked_with(MEASURED, Form::Whole);
    let args = [measured(), t(entry.pattern), t(entry.replacement)];
    assert!(matches!(rendered_text::dotnet_guard(regex, "Replace", &args), Ok(true)));
    assert!(rendered_text::dotnet_guard(regex, "Replace", &[measured(), t("x"), t(entry.replacement)]).is_err());
    assert!(
        rendered_text::dotnet_guard(regex, "Replace", &[measured(), t(entry.pattern), t("rgb($1,$2,$3)")]).is_err()
    );
    assert!(rendered_text::dotnet_guard(regex, "Match", &args).is_err());
    assert!(rendered_text::dotnet_guard("System.String", "Concat", &[measured()]).is_err());
    assert!(matches!(
        rendered_text::dotnet_guard(regex, "Replace", &[t("a"), t("b"), t("c")]),
        Ok(false)
    ));
}

/// The pattern and replacement are System Application's, character for character (corpus pull request 1371,
/// reports/html-email-body/RxEmailBodyTests.al:5-6).
#[test]
fn the_rgba_entry_holds_the_pattern_and_replacement_of_email_message_impl() {
    let entry = &rendered_text::DOTNET_ALLOWED[0];
    assert_eq!(
        entry.pattern,
        r"rgba\((\d{1,3}),\s*(\d{1,3}),\s*(\d{1,3}),\s*1(\.0{0,2})?\)"
    );
    assert_eq!(entry.replacement, "rgb($1, $2, $3)");
}

#[track_caller]
fn replace_refused(input: Value, want: &str) {
    let t = |s: &str| Value::Text(s.to_string().into());
    let entry = &rendered_text::DOTNET_ALLOWED[0];
    let regex = "System.Text.RegularExpressions.Regex";
    match rendered_text::dotnet_guard(regex, "Replace", &[input, t(entry.pattern), t(entry.replacement)]) {
        Err(Signal::Unsupported(why)) => assert!(why.contains(want), "{why}"),
        other => panic!("expected a refusal naming {want:?}, got {other:?}"),
    }
}

/// The replace takes only a body whose every render is of the measured shape, and marks its assumption each time.
#[test]
fn the_rgba_replace_takes_only_renders_of_the_measured_shape() {
    use al_builtins::assumptions::{self, Assumption};
    let t = |s: &str| Value::Text(s.to_string().into());
    let entry = &rendered_text::DOTNET_ALLOWED[0];
    let regex = "System.Text.RegularExpressions.Regex";
    let call =
        |input: Value| rendered_text::dotnet_guard(regex, "Replace", &[input, t(entry.pattern), t(entry.replacement)]);
    // The body after one pass (the changed form) goes through again: AppendToBody and a second Create.
    for form in [Form::Whole, Form::Changed] {
        assumptions::clear_hits();
        assert!(matches!(call(marked_with(MEASURED, form)), Ok(true)));
        assert!(assumptions::take_hits().contains(&Assumption::EmailBodyRgbaReplaceKeepsContent));
    }
    // A render whose save row did not measure it refuses with the save's reason; which renders those are is the table's
    // (al-vm `report_save`, tested there).
    assumptions::clear_hits();
    replace_refused(
        marked_with(
            RgbaReplace::NotMeasured("the nearest measured renders differ in: data items".into()),
            Form::Whole,
        ),
        "is not measured on the Html of report Guard Report: the nearest measured renders differ in: data items",
    );
    replace_refused(marked(), "nothing is known of the render");
    // A body that joins two renders, even two of the measured shape: one render per body was measured.
    let (Value::Text(a), Value::Text(b)) = (marked_with(MEASURED, Form::Whole), marked_with(MEASURED, Form::Whole))
    else {
        unreachable!()
    };
    replace_refused(
        t(&format!("{}{}", a.audited_str(), b.audited_str())),
        "the body joins 2 renders",
    );
    assert!(!assumptions::take_hits().contains(&Assumption::EmailBodyRgbaReplaceKeepsContent));
}

/// A mark whose number names no render of this process (a text from a stored row of another run) is no entry.
#[test]
fn the_rgba_replace_refuses_a_mark_that_names_no_render() {
    let unknown = rendered_text::mark_text(0xFFF0, Form::Whole);
    replace_refused(Value::Text(unknown.into()), "a mark that names no render");
}

/// The first argument is a Text: a collection or TextBuilder that holds a marked text is no entry.
#[test]
fn the_rgba_replace_takes_the_body_only_as_a_text() {
    let Value::Text(body) = marked_with(MEASURED, Form::Whole) else {
        unreachable!()
    };
    let list = al_builtins::value::ListVec::from(vec![Value::Text(body)]);
    replace_refused(
        Value::List(std::rc::Rc::new(std::cell::RefCell::new(list))),
        "Regex.Replace given the value",
    );
}

#[test]
fn a_dotnet_answer_keeps_every_mark_in_the_changed_form_or_refuses() {
    let n = rendered_text::register("Guard Report", "Html", "WordOne").unwrap();
    let input = format!("a{}", rendered_text::mark_text(n, Form::Whole));
    let out = rendered_text::dotnet_result(&input, &input).unwrap();
    assert_eq!(rendered_text::marks(&out).unwrap(), vec![(n, Form::Changed)]);
    assert!(
        rendered_text::dotnet_result(&input, "a").is_err(),
        "a lost mark refuses (L6)"
    );
}

#[test]
fn the_runner_prints_a_mark_as_its_report() {
    let n = rendered_text::register("Printed Report", "Html", "WordOne").unwrap();
    let shown = rendered_text::describe(&format!("Sig {}", rendered_text::mark_text(n, Form::Whole)));
    assert_eq!(shown, "Sig <rendered content of report Printed Report>");
}

#[test]
fn the_guard_looks_into_an_array_and_allows_only_the_array_builtins_of_the_table() {
    let arr = |v: Value| {
        Value::Array(std::rc::Rc::new(std::cell::RefCell::new(vec![
            Value::Text(String::new().into()),
            v,
        ])))
    };
    let known = arr(Value::Text("known".to_string().into()));
    let marked_arr = arr(marked());
    let pos = Value::Integer(1);
    let copy = rendered_text::guard(
        "System.CopyArray#1",
        None,
        &[known.clone(), marked_arr.clone(), pos.clone()],
        false,
    );
    assert!(copy.is_ok(), "{copy:?}");
    for name in ["System.CompressArray#1", "System.ArrayLen#1"] {
        let r = rendered_text::guard(name, None, std::slice::from_ref(&marked_arr), true);
        assert!(r.is_ok(), "{name}: {r:?}");
    }
    // A builtin with no row for an array refuses it, also in a Variant or a List.
    assert!(rendered_text::guard("System.Format#1", None, std::slice::from_ref(&marked_arr), true).is_err());
    assert!(rendered_text::guard("Text.StrLen#1", None, std::slice::from_ref(&marked_arr), true).is_err());
    assert!(rendered_text::guard("Text.StrLen#1", None, std::slice::from_ref(&known), true).is_ok());
}
