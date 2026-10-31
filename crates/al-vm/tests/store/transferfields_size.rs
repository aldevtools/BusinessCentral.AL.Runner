//! al-runner issue 5773: TransferFields of a text longer than the target Text field
//! (errors/lt-ms-refusals-130/P130TransferTypes.al:424-480 RN_Transfer_Text100ToText10_ByValueLength).

const SRC: &str = r#"
table 50940 "Tf Src" { fields { field(1; Id; Integer) { } field(19; TextLong; Text[100]) { } } keys { key(PK; Id) { } } }
table 50941 "Tf Dst" { fields { field(1; Id; Integer) { } field(19; Short; Text[10]) { } } keys { key(PK; Id) { } } }
codeunit 50100 C
{
    [TryFunction]
    local procedure Try(var D: Record "Tf Dst"; var S: Record "Tf Src"; Key: Boolean) begin D.TransferFields(S, Key); end;
    local procedure One(Value: Text; Key: Boolean): Text
    var S: Record "Tf Src"; D: Record "Tf Dst";
    begin
        S.Id := 1; S.TextLong := Value; D.Id := 5; D.Short := 'old';
        if Try(D, S, Key) then exit('ok ' + Format(D.Id) + ' ' + D.Short);
        exit(GetLastErrorText() + '|' + GetLastErrorCode() + ' ' + Format(D.Id) + ' ' + D.Short);
    end;
    procedure F(): Text
    begin
        exit(One('abcdefghij', true) + '#' + One('abcdefghijk', true) + '#' + One('abcdefghijk', false));
    end;
}
"#;

#[test]
fn a_text_too_long_for_the_target_fails_with_type_mismatch_after_the_key() {
    let e = "The contents of the TextLong field cannot be copied to the Short field because the size of the Short field is too small.\\\\Type: Text10 <-- Text100\\Field: Short <-- TextLong\\Table: Tf Dst <-- Tf Src|DB:TypeMisMatch";
    assert_eq!(
        al_testkit::text(al_testkit::run(SRC, "F")),
        format!("ok 1 abcdefghij#{e} 1 old#{e} 5 old")
    );
}

// A Boolean and a Text field of the same number, either way (misc/lt-probes-needed-6/PnF07TransferEnumOption.al:185-206).
const BOOL_TEXT: &str = r#"
table 50942 "Tf Bool" { fields { field(1; Id; Integer) { } field(2; Flag; Boolean) { } } keys { key(PK; Id) { } } }
table 50943 "Tf Text" { fields { field(1; Id; Integer) { } field(2; Flag; Text[30]) { } } keys { key(PK; Id) { } } }
codeunit 50101 C
{
    [TryFunction]
    local procedure BoolToText(var T: Record "Tf Text"; B: Record "Tf Bool") begin T.TransferFields(B); end;
    [TryFunction]
    local procedure TextToBool(var B: Record "Tf Bool"; T: Record "Tf Text") begin B.TransferFields(T); end;
    procedure F(): Text
    var B: Record "Tf Bool"; T: Record "Tf Text"; R: Text;
    begin
        B.Flag := true;
        if not BoolToText(T, B) then R := GetLastErrorText() + '|' + GetLastErrorCode();
        T.Flag := 'Yes';
        if not TextToBool(B, T) then R += '#' + GetLastErrorText() + '|' + GetLastErrorCode();
        exit(R);
    end;
}
"#;

#[test]
fn boolean_and_text_fields_fail_with_type_mismatch() {
    assert_eq!(
        al_testkit::text(al_testkit::run(BOOL_TEXT, "F")),
        "The following fields must have the same type:\\\\Field: Flag <-- Flag\\Table: Tf Text <-- Tf Bool\\Type: Text <-- Boolean|DB:TypeMisMatch\
         #The following fields must have the same type:\\\\Field: Flag <-- Flag\\Table: Tf Bool <-- Tf Text\\Type: Boolean <-- Text|DB:TypeMisMatch"
    );
}
