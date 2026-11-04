//! A Variant handed to `Get` of a key of another type converts as the corpus pins it
//! (record/Y70Probes.al RecordGet_BooleanOnTextKey, RecordGet_TextOnIntegerKey).

use al_testkit::text;

const SRC: &str = r#"
table 69970 "GV Int Key" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
table 69971 "GV Text Key" { fields { field(1; Id; Text[10]) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 69972 C
{
    [TryFunction]
    local procedure TryInt(V: Variant; var Found: Text)
    var Row: Record "GV Int Key";
    begin
        if Row.Get(V) then Found := Format(Row.Id) else Found := '<none>';
    end;

    [TryFunction]
    local procedure TryText(V: Variant; var Found: Text)
    var Row: Record "GV Text Key";
    begin
        if Row.Get(V) then Found := Row.Id else Found := '<none>';
    end;

    local procedure GetInt(V: Variant): Text
    var Found: Text;
    begin
        if TryInt(V, Found) then exit(Found);
        exit('ERR:' + GetLastErrorCode());
    end;

    local procedure GetText(V: Variant): Text
    var Found: Text;
    begin
        if TryText(V, Found) then exit(Found);
        exit('ERR:' + GetLastErrorCode());
    end;

    procedure TextOnInt(): Text
    var I: Record "GV Int Key";
    begin
        I.Id := 5; I.Insert();
        exit(GetInt(Format(5)) + '|' + GetInt('5') + '|' + GetInt(' 5 ') + '|' + GetInt('+5'));
    end;

    procedure IntOnText(): Text
    var T: Record "GV Text Key";
    begin
        T.Id := '1'; T.Insert(); T.Id := '0'; T.Insert();
        exit(GetText(1) + '|' + GetText(0));
    end;
}
"#;

#[test]
fn a_text_of_digits_finds_the_row_of_an_integer_key() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "TextOnInt")), "5|5|5|5");
}

#[test]
fn an_integer_finds_the_row_of_a_text_key() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "IntOnText")), "1|0");
}
