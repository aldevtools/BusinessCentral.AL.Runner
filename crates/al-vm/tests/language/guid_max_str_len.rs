//! MaxStrLen of a Guid, and a Guid concatenated with a Text (types/lt-aj-probes/AjMisc.al:54-90,
//! Guid_TextConcatenationAndMaxStrLen).

use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 69970 GuidLen
{
    procedure Probe(): Text
    var
        G: Guid;
        Empty: Guid;
        T: Text;
    begin
        Evaluate(G, '{11111111-2222-3333-4444-555555555555}');
        T := 'x' + G;
        exit(Format(MaxStrLen(G)) + '|' + Format(MaxStrLen(Empty)) + '|' + T + '|' + Format(StrLen(T)));
    end;
}
"#;

#[test]
fn max_str_len_of_a_guid_is_the_text_maximum() {
    assert_eq!(
        text(run_in(SRC, "GuidLen", "Probe")),
        "2147483647|2147483647|x{11111111-2222-3333-4444-555555555555}|39"
    );
}
