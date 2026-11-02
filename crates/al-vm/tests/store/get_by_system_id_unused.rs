//! `Record.GetBySystemId(Guid)` of a missing row with the result unused raises BC's not-found error
//! (misc/lt-probes-needed-4/PnD04_GetBySystemId.al:36-47 `missUnused`, :111-124 `nullUnused`).

use al_builtins::value::Value;

const SRC: &str = r#"
table 60020 "Sys Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}
codeunit 50120 C
{
    procedure Miss(): Text
    var
        Row: Record "Sys Row";
        G: Guid;
    begin
        Evaluate(G, '{11111111-2222-3333-4444-555555555555}');
        asserterror Row.GetBySystemId(G);
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure Null(): Text
    var
        Row: Record "Sys Row";
        G: Guid;
    begin
        asserterror Row.GetBySystemId(G);
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
}
"#;

fn text(routine: &str) -> String {
    match al_testkit::run_in(SRC, "C", routine) {
        Ok(Value::Text(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_miss_with_the_result_unused_raises_record_not_found() {
    assert_eq!(
        text("Miss"),
        "The Sys Row does not exist. Identification fields and values: {11111111-2222-3333-4444-555555555555}|DB:RecordNotFound"
    );
    assert_eq!(
        text("Null"),
        "The Sys Row does not exist. Identification fields and values: {00000000-0000-0000-0000-000000000000}|DB:RecordNotFound"
    );
}
