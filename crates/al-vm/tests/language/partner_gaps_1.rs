use al_builtins::value::Value;

fn text(vars: &str, body: &str) -> String {
    let src = format!("codeunit 50100 C {{ procedure F(): Text var {vars} begin {body} end; }}");
    match al_testkit::run_in(&src, "C", "F") {
        Ok(Value::Text(t) | Value::Code(t)) => t.to_string(),
        Err(al_builtins::native::Signal::Error(e)) => format!("error:{}|{}", e.message, e.code),
        other => format!("{other:?}"),
    }
}

#[test]
fn foreach_over_a_never_used_json_array_runs_zero_times() {
    // json/JsonArrayBasics.al:274-281 pins the cleared array; a never-used one is the same state.
    let r = text(
        "A: JsonArray; T: JsonToken; R: Text;",
        "foreach T in A do R += 'x'; exit('[' + R + ']');",
    );
    assert_eq!(r, "[]");
}

#[test]
fn foreach_over_a_json_array_copied_from_a_never_used_one() {
    let r = text(
        "A: JsonArray; B: JsonArray; T: JsonToken; R: Text;",
        "B := A; foreach T in B do R += 'x'; exit('[' + R + ']');",
    );
    assert_eq!(r, "[]");
}

#[test]
fn text_builder_append_of_a_variant_number_raises_the_type_conversion() {
    // format-evaluate/NumberToTextIsFormat.al:193-216 (TextBuilderAppend_VariantNumber_Result): Integer, BigInteger
    // and Decimal field values appended through a Variant raise and leave the builder unchanged.
    for (ty, lit) in [("Integer", "4222"), ("BigInteger", "4222"), ("Decimal", "1.5")] {
        let r = text(
            &format!("V: Variant; N: {ty}; B: TextBuilder;"),
            &format!("N := {lit}; V := N; B.Append(V); exit('no error');"),
        );
        assert_eq!(
            r,
            "error:The expression Microsoft.Dynamics.Nav.Runtime.NavIndirectValue cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.NavText value.|TypeConversion",
            "{ty}"
        );
    }
}

#[test]
fn text_builder_append_of_a_variant_returning_call_stays_refused() {
    // Only a Variant variable is pinned (NumberToTextIsFormat.al:193-216); a call result such as FieldRef.Value() is not
    // (its case is pinned on the staging corpus branch and refused until that bump lands).
    let src = "codeunit 50100 C { procedure G(): Variant var N: Integer; begin N := 5; exit(N); end; procedure F(): Text var B: TextBuilder; begin B.Append(G()); exit(B.ToText()); end; }";
    let r = al_testkit::run_in(src, "C", "F");
    match r {
        Err(al_builtins::native::Signal::Unsupported(m)) => assert!(m.contains("TextBuilder.Append#1"), "{m}"),
        other => panic!("{other:?}"),
    }
}
