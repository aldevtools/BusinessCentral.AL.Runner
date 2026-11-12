//! The allow-list of rendered content (ADR-040 amendment 2026-11-12, A3, A5; loss modes L1, L6, L12, L16).

use al_builtins::builtin_id::BuiltinId;
use al_builtins::native::Signal;
use al_builtins::rendered_text::{self, Form};
use al_builtins::value::Value;

fn marked() -> Value {
    let n = rendered_text::register("Guard Report", "Html", "WordOne").expect("a render");
    Value::Text(rendered_text::mark_text(n, Form::Whole).into())
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
    // P5, P6 are answered by `ops::compare`, P9 by `Variant.Is*`, P12 by `DOTNET_ALLOWED`; P11 (a page field) is not built.
    assert_eq!(cites, ["A2", "P1", "P10", "P2", "P3", "P4", "P7", "P8"]);
    // No row changes characters today: every result moves the value unchanged (whole form kept).
    assert!(rendered_text::allowed_rows().all(|(_, _, form)| form.is_none()));
}

#[test]
fn a_dotnet_call_with_a_marked_argument_refuses_unless_it_is_the_rgba_replace() {
    let t = |s: &str| Value::Text(s.to_string().into());
    let regex = "System.Text.RegularExpressions.Regex";
    let entry = &rendered_text::DOTNET_ALLOWED[0];
    let args = [marked(), t(entry.pattern), t(entry.replacement)];
    assert!(matches!(rendered_text::dotnet_guard(regex, "Replace", &args), Ok(true)));
    assert!(rendered_text::dotnet_guard(regex, "Replace", &[marked(), t("x"), t(entry.replacement)]).is_err());
    assert!(rendered_text::dotnet_guard(regex, "Match", &args).is_err());
    assert!(rendered_text::dotnet_guard("System.String", "Concat", &[marked()]).is_err());
    assert!(matches!(
        rendered_text::dotnet_guard(regex, "Replace", &[t("a"), t("b"), t("c")]),
        Ok(false)
    ));
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
