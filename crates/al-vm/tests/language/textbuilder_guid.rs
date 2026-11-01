//! A Guid given to TextBuilder.Append, AppendLine, Insert and Replace is its upper-case braced text, also inside a Variant
//! (text/lt-y49-textbuilder-guid/Y49TextBuilderGuid.al:60-219, corpus master).

const SRC: &str = r#"
codeunit 50770 C {
    procedure All(): Text
    var TB: TextBuilder; G: Guid; E: Guid; V: Variant;
    begin
        Evaluate(G, '3f2504e0-4f89-11d3-9a0c-0305e82c3301');
        TB.Append(G); TB.Append('|'); TB.Append(E); TB.Append('|');
        TB.AppendLine(G);
        TB.Insert(1, G);
        V := G; TB.Append(V);
        TB.Replace(G, 'g');
        TB.Append('#'); TB.Replace('#', E);
        exit(TB.ToText());
    end;
}
"#;

#[test]
fn a_guid_appended_or_inserted_into_a_textbuilder_is_its_braced_upper_case_text() {
    let e = "{00000000-0000-0000-0000-000000000000}";
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "All")),
        format!("gg|{e}|g\r\ng{e}")
    );
}
