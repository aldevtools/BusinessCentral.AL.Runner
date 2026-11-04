//! `modify(Field) { TableRelation = ...; }` in a table extension adds its arms after the base field's arms; the first arm
//! that applies decides (corpus table-objects/extensions/TestTableExtFieldTableRelation.al:241-298, objects in
//! table-objects/BsObjects.al): an unconditional base relation keeps deciding, and a conditional one is
//! extended by the extension's condition. Other combinations are not pinned and are refused.

use al_builtins::native::Signal;
use al_testkit::{run, text};

fn src(base_ref: &str, ext_ref: &str, body: &str) -> String {
    format!(
        r#"
table 69970 "RM Base"
{{
    fields
    {{
        field(1; Id; Integer) {{ }}
        field(5; Ref; Code[20]) {{ TableRelation = {base_ref}; }}
        field(6; Kind; Option) {{ OptionMembers = X,Y,Z; }}
    }}
    keys {{ key(PK; Id) {{ }} }}
}}
table 69971 "RM Target A" {{ fields {{ field(1; Code; Code[20]) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69972 "RM Target B" {{ fields {{ field(1; Code; Code[20]) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
tableextension 69973 "RM Ext" extends "RM Base"
{{
    fields {{ modify(Ref) {{ TableRelation = {ext_ref}; }} }}
}}
codeunit 69974 C
{{
    local procedure Try(Kind: Option X,Y,Z; Value: Code[20]): Text
    var B: Record "RM Base";
    begin
        B.Init(); B.Kind := Kind;
        ClearLastError();
        asserterror begin B.Validate(Ref, Value); Error('ok'); end;
        exit(GetLastErrorText());
    end;
    procedure F(): Text
    var A: Record "RM Target A"; T: Record "RM Target B";
    begin
        A.Code := 'ONLYA'; A.Insert(); T.Code := 'ONLYB'; T.Insert(); Commit();
        {body}
    end;
}}
"#
    )
}

#[test]
fn an_unconditional_base_relation_keeps_deciding() {
    let s = src(
        "\"RM Target A\"",
        "\"RM Target B\"",
        "exit(Try(0, 'ONLYA') + '|' + Try(0, 'ONLYB'));",
    );
    assert_eq!(
        text(run(&s, "F")),
        "ok|The field Ref of table RM Base contains a value (ONLYB) that cannot be found in the related table (RM Target A)."
    );
}

#[test]
fn a_conditional_base_relation_gets_the_extension_condition_after_it() {
    let s = src(
        "if (Kind = const(X)) \"RM Target A\"",
        "if (Kind = const(Y)) \"RM Target B\"",
        "exit(Try(0, 'ONLYA') + '|' + Try(0, 'ONLYB') + '|' + Try(1, 'ONLYB') + '|' + Try(1, 'ONLYA') + '|' + Try(2, 'NONE'));",
    );
    assert_eq!(
        text(run(&s, "F")),
        "ok|The field Ref of table RM Base contains a value (ONLYB) that cannot be found in the related table (RM Target A).|ok|\
         The field Ref of table RM Base contains a value (ONLYA) that cannot be found in the related table (RM Target B).|ok"
    );
}

#[test]
fn a_mixed_combination_is_refused() {
    let s = src(
        "if (Kind = const(X)) \"RM Target A\"",
        "\"RM Target B\"",
        "exit(Try(1, 'ONLYB'));",
    );
    match run(&s, "F") {
        Err(Signal::Unsupported(r)) => assert!(r.contains("TableRelation"), "{r}"),
        other => panic!("{other:?}"),
    }
}
