//! A raw TAB inside a JSON string: the reporter's repro of al-runner issue 7812, unchanged. Microsoft's Sales Lines
//! Suggestions test app reads a JSONL dataset with such lines in its install codeunit. Until the corpus probe
//! (json/JsonReadFrom.al, corpus pull request 1327) is pinned, the reads pass through the assumption
//! `json-raw-tab-in-string` and count as flagged passes.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
codeunit 50280 "Probe Json Tab Tests"
{
    Subtype = Test;

    [Test]
    procedure TokenReadFromRawTab()
    var
        Token: JsonToken;
        Value: JsonToken;
        Tab: Text[1];
    begin
        Tab[1] := 9;
        if not Token.ReadFrom('{"question": "Name' + Tab + 'Price"}') then
            Error('JsonToken.ReadFrom returned false for a raw TAB inside a string');
        Token.AsObject().Get('question', Value);
        if Value.AsValue().AsText() <> 'Name' + Tab + 'Price' then
            Error('the value lost its TAB');
    end;

    [Test]
    procedure ObjectReadFromRawTab()
    var
        Obj: JsonObject;
        Tab: Text[1];
    begin
        Tab[1] := 9;
        if not Obj.ReadFrom('{"question": "Name' + Tab + 'Price"}') then
            Error('JsonObject.ReadFrom returned false for a raw TAB inside a string');
    end;

    [Test]
    procedure TokenReadFromEscapedTab()
    var
        Token: JsonToken;
    begin
        if not Token.ReadFrom('{"question": "Name\tPrice"}') then
            Error('JsonToken.ReadFrom returned false for an escaped TAB');
    end;
}
"#;

#[test]
fn a_raw_tab_inside_a_json_string_reads_as_a_flagged_pass() {
    let out = outcomes_in(SRC, "Probe Json Tab Tests");
    for name in ["TokenReadFromRawTab", "ObjectReadFromRawTab", "TokenReadFromEscapedTab"] {
        let o = out.iter().find(|(t, _)| t == name).map(|(_, o)| o.clone());
        assert_eq!(o, Some(Outcome::Pass), "{name}");
    }
    // The test runner takes the marks per case; a direct call shows the mark of the read.
    al_builtins::assumptions::clear_hits();
    assert!(al_testkit::run_in(SRC, "Probe Json Tab Tests", "ObjectReadFromRawTab").is_ok());
    let hits = al_builtins::assumptions::take_hits();
    assert!(
        hits.contains(&al_builtins::assumptions::Assumption::JsonRawTabInString),
        "{hits:?}"
    );
}
