//! A conditional expression with a Guid branch and a Label, Text, Code or literal branch: the true branch's type is the
//! result, the other branch is converted (al-runner issue 6160; corpus types/lt-uf-ternary-guid-label/UfTernaryGuidLabel.al,
//! writer UF branch al-runtime-rs-staging-uf, commit 3d7d7b66).

use al_testkit::text;

const SRC: &str = r#"
codeunit 69795 C
{
    var
        KeyLbl: Label '00000000-0000-0000-0000-000000000001', Locked = true;
        PlainLbl: Label 'plain label';

    procedure Pairs(): Text
    var K: Guid; Null: Guid; MyKey: Text; MyCode: Code[50]; Other: Text; OtherCode: Code[40]; V: Variant; T: Text;
    begin
        Evaluate(K, '{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}');
        MyKey := true ? K : KeyLbl; T += MyKey + ';';
        MyKey := false ? K : KeyLbl; T += MyKey + ';';
        MyKey := true ? KeyLbl : K; T += MyKey + ';';
        MyKey := false ? KeyLbl : K; T += MyKey + ';';
        T += Format(StrLen(false ? Null : KeyLbl)) + ';';
        MyCode := false ? K : KeyLbl; T += MyCode + ';';
        V := false ? K : KeyLbl; T += Format(V.IsGuid()) + ';';
        Other := '11111111-2222-3333-4444-555555555555';
        OtherCode := '66666666-7777-8888-9999-000000000000';
        MyKey := false ? K : Other; T += MyKey + ';';
        MyKey := false ? K : OtherCode; T += MyKey + ';';
        MyKey := false ? K : '11111111-2222-3333-4444-555555555555'; T += MyKey + ';';
        MyKey := true ? Other : K; T += MyKey + ';';
        MyKey := false ? OtherCode : K; T += MyKey + ';';
        MyKey := true ? PlainLbl : K; T += MyKey;
        exit(T);
    end;

    procedure NotAGuid(): Text
    var K: Guid; MyKey: Text;
    begin
        MyKey := false ? K : PlainLbl;
    end;
}
"#;

#[test]
fn the_guid_branch_sets_the_type() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Pairs")),
        "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE};{00000000-0000-0000-0000-000000000001};00000000-0000-0000-0000-000000000001;\
{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE};38;{00000000-0000-0000-0000-000000000001};Yes;{11111111-2222-3333-4444-555555555555};\
{66666666-7777-8888-9999-000000000000};{11111111-2222-3333-4444-555555555555};11111111-2222-3333-4444-555555555555;\
{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE};plain label"
    );
}

#[test]
fn a_label_that_is_no_guid_fails_when_taken() {
    let msg = al_testkit::error_message(al_testkit::run_in(SRC, "C", "NotAGuid"));
    assert!(msg.starts_with("Invalid format of GUID string."), "{msg}");
}
