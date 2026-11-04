//! Entry into a TestPage control: the checks the client makes before it validates (CharAllowed, NotBlank, the conversion
//! of the typed text), what it keeps of a refused entry, and what Close does after one. Each test names the corpus test
//! that pins what BC does.

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn refused(r: Result<Value, Signal>, needle: &str) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains(needle), "{why}"),
        other => panic!("expected a refusal naming {needle}, got {other:?}"),
    }
}

const CARD: &str = r#"
table 69960 "CE Row"
{
    fields
    {
        field(1; Id; Code[20]) { }
        field(2; Txt; Text[30]) { }
        field(3; Qty; Integer) { MinValue = 1; MaxValue = 10; }
        field(4; Dec; Decimal) { MinValue = 0; MaxValue = 100; }
        field(5; Digits; Code[20]) { CharAllowed = '09'; }
        field(6; Letters; Text[30]) { }
        field(7; Guard; Text[30]) { trigger OnValidate() begin if Guard = 'bad' then Error('guard refused'); end; }
        field(8; Note; Text[30]) { }
        field(9; LowCode; Code[20]) { CharAllowed = 'az'; }
        field(10; Kind; Enum "CE Kind") { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
enum 69961 "CE Kind" { Extensible = false; value(0; Plain) { } value(1; Named) { Caption = 'Shown Caption'; } value(2; "With Space") { } }
page 69962 "CE Card"
{
    PageType = Card; SourceTable = "CE Row"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(IdCtl; Rec.Id) { }
            field(TxtCtl; Rec.Txt) { }
            field(QtyCtl; Rec.Qty) { }
            field(DecCtl; Rec.Dec) { }
            field(DigitsCtl; Rec.Digits) { }
            field(LettersCtl; Rec.Letters) { CharAllowed = 'azAZ09'; }
            field(OddCtl; Rec.Letters) { CharAllowed = 'azA'; }
            field(BackwardsCtl; Rec.Letters) { CharAllowed = 'za'; }
            field(GuardCtl; Rec.Guard) { }
            field(NoteCtl; Rec.Note) { NotBlank = true; }
            field(VarNoteCtl; VarNote) { NotBlank = true; }
            field(LowCodeCtl; Rec.LowCode) { }
            field(KindCtl; Rec.Kind) { }
        }
    }
    var VarNote: Text[30];
}
codeunit 69963 C
{
    local procedure Seed()
    var R: Record "CE Row";
    begin
        R.DeleteAll();
        R.Id := 'R1'; R.Txt := 'orig'; R.Qty := 5; R.Dec := 1; R.Note := 'n0';
        R.Insert();
        Commit();
    end;
    local procedure Stored(): Text
    var R: Record "CE Row";
    begin
        R.Get('R1');
        exit(R.Txt + '|' + Format(R.Qty) + '|' + R.Note);
    end;
    procedure CodeCharAllowed(): Text
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        asserterror P.DigitsCtl.SetValue('4x6');
        exit(GetLastErrorText() + '#' + GetLastErrorCode());
    end;
    procedure CodeCharAllowedLetters(): Text
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        P.LowCodeCtl.SetValue('abc');
        exit(P.LowCodeCtl.Value());
    end;
    procedure ThreeRanges(): Text
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        P.LettersCtl.SetValue('aZ9bY8');
        asserterror P.LettersCtl.SetValue('a-b');
        exit(P.LettersCtl.Value() + '#' + GetLastErrorText());
    end;
    procedure OddCharAllowed()
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        P.OddCtl.SetValue('a');
    end;
    procedure BackwardsCharAllowed()
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        P.BackwardsCtl.SetValue('a');
    end;
    procedure IntegerConversion(): Text
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        asserterror P.QtyCtl.SetValue('abc');
        exit(GetLastErrorText() + '#' + GetLastErrorCode());
    end;
    procedure DecimalConversion(): Text
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        asserterror P.DecCtl.SetValue('x1');
        exit(GetLastErrorText() + '#' + GetLastErrorCode());
    end;
    procedure CloseAfterMaxValue(): Text
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        P.TxtCtl.SetValue('new');
        asserterror P.QtyCtl.SetValue(99);
        P.NoteCtl.SetValue('n1');
        P.Close();
        exit(Stored());
    end;
    procedure CloseAfterRetry(): Text
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        asserterror P.QtyCtl.SetValue(99);
        P.QtyCtl.SetValue(7);
        P.TxtCtl.SetValue('new');
        P.Close();
        exit(Stored());
    end;
    procedure CloseAfterOnValidate(): Text
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        P.TxtCtl.SetValue('new');
        asserterror P.GuardCtl.SetValue('bad');
        P.Close();
        exit(GetLastErrorText() + '#' + GetLastErrorCode() + '#' + Stored());
    end;
    procedure NotBlankControl(): Text
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        P.NoteCtl.SetValue('filled');
        asserterror P.NoteCtl.SetValue('');
        exit(GetLastErrorText() + '#' + GetLastErrorCode() + '#' + P.NoteCtl.Value());
    end;
    procedure NotBlankVariableControl()
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        P.VarNoteCtl.SetValue('');
    end;
    procedure TwiceOnOne(): Text
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        asserterror P.QtyCtl.SetValue(99);
        asserterror P.QtyCtl.SetValue('abc');
        asserterror P.DecCtl.SetValue(-5);
        exit(Format(P.ValidationErrorCount()) + '#' + Format(P.QtyCtl.ValidationErrorCount()) + '#' + P.GetValidationError(1));
    end;
    procedure InterleavedOnOne()
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        asserterror P.QtyCtl.SetValue(99);
        asserterror P.DecCtl.SetValue(-5);
        asserterror P.QtyCtl.SetValue('abc');
    end;
    procedure EnumEmptyAfterRefusal(): Text
    var P: TestPage "CE Card";
    begin
        Seed(); P.OpenEdit();
        asserterror P.KindCtl.SetValue('Nothing');
        asserterror P.KindCtl.SetValue('');
        exit(GetLastErrorText() + '#' + P.KindCtl.Value());
    end;
}
"#;

/// record/table-properties/XTablePropsTests.al:104: a Code control upper-cases the entry, so the refused character is named
/// upper-cased. A Code control whose CharAllowed holds letters is not pinned and is refused.
#[test]
fn char_allowed_on_a_code_control_names_the_upper_cased_character() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(CARD, "C", "CodeCharAllowed")),
        "Validation error for Field: DigitsCtl,  Message = 'The character X is not valid in this field. You can specify the following characters: 09. (Select Refresh to discard errors)'#TestValidation"
    );
    refused(al_testkit::run_in(CARD, "C", "CodeCharAllowedLetters"), "CharAllowed");
}

/// testpage/AhPageTests.al:176, :808-811: CharAllowed of several pairs takes a character in any of the ranges and
/// lists them comma-separated. An odd number of characters or a range from a higher to a lower character is not pinned.
#[test]
fn char_allowed_with_several_ranges() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(CARD, "C", "ThreeRanges")),
        "aZ9bY8#Validation error for Field: LettersCtl,  Message = 'The character - is not valid in this field. You can specify the following characters: az,AZ,09. (Select Refresh to discard errors)'"
    );
    refused(al_testkit::run_in(CARD, "C", "OddCharAllowed"), "CharAllowed");
    refused(al_testkit::run_in(CARD, "C", "BackwardsCharAllowed"), "CharAllowed");
}

/// testpage/AhPageTests.al:112-113, :133-134: on a card, a text that is not a number says why, with TestValidation.
#[test]
fn a_failed_number_conversion_on_a_card_says_why() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(CARD, "C", "IntegerConversion")),
        "Validation error for Field: QtyCtl,  Message = 'Your entry of 'abc' is not an acceptable value for 'Qty'. The value \"abc\" can't be evaluated into type Integer. (Select Refresh to discard errors)'#TestValidation"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(CARD, "C", "DecimalConversion")),
        "Validation error for Field: DecCtl,  Message = 'Your entry of 'x1' is not an acceptable value for 'Dec'. 'x1' is not a part of a valid decimal expression. (Select Refresh to discard errors)'#TestValidation"
    );
}

/// testpage/AhPageTests.al:73, :94: a card that holds a refused entry stores none of its edits at Close;
/// :220 a corrected entry clears it and the edits are stored. After a refusal raised by OnValidate, Close leaves that
/// error as the last error (:155-159).
#[test]
fn close_after_a_refused_entry_on_a_card_drops_the_edits() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(CARD, "C", "CloseAfterMaxValue")),
        "orig|5|n0"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(CARD, "C", "CloseAfterRetry")),
        "new|7|n0"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(CARD, "C", "CloseAfterOnValidate")),
        "guard refused#ClientValidateField:NSValidateField#orig|5|n0"
    );
}

/// pages/properties/P119PageFields.al:205-207: NotBlank on the page control refuses a blank entry and
/// keeps the value. On a control bound to a variable of a page that is not a request page it is not pinned.
#[test]
fn not_blank_on_the_page_control() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(CARD, "C", "NotBlankControl")),
        "Validation error for Field: NoteCtl,  Message = 'Note must be filled in. Enter a value. (Select Refresh to discard errors)'#TestValidation#filled"
    );
    refused(al_testkit::run_in(CARD, "C", "NotBlankVariableControl"), "NotBlank");
}

/// testpage/AhPageTests.al:444-535 (GVE_TwoOnOne_*): a second refusal on one control replaces the first.
/// When another control's refusal came in between, the order is not pinned: refused.
#[test]
fn a_second_refusal_on_one_control_replaces_the_first() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(CARD, "C", "TwiceOnOne")),
        "2#1#Your entry of 'abc' is not an acceptable value for 'Qty'. The value \"abc\" can't be evaluated into type Integer. (Select Refresh to discard errors)"
    );
    refused(al_testkit::run_in(CARD, "C", "InterleavedOnOne"), "refusal");
}

/// pages/controls/PageEnumControlCaptionless.al:107, :109: after a refusal an Enum control shows the ordinal,
/// and an empty entry is that ordinal's text, which is refused too.
#[test]
fn an_empty_entry_on_an_enum_control_after_a_refusal_is_the_ordinal() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(CARD, "C", "EnumEmptyAfterRefusal")),
        "Validation error for Field: KindCtl,  Message = 'Your entry of '0' is not an acceptable value for 'Kind'. (Select Refresh to discard errors)'#0"
    );
}

/// pages/properties/ABLayouts.al:68, :70: a Text control in a cuegroup with CuegroupLayout = Wide reads '0';
/// pages/controls/AaControls.al:422 a Decimal one reads its value. Other types there are not pinned.
#[test]
fn a_text_control_in_a_wide_cue_group_reads_zero() {
    let src = r#"
table 69965 "CW Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Amt; Decimal) { } field(4; Cnt; Integer) { } } keys { key(PK; Id) { } } }
page 69966 "CW Card"
{
    PageType = Card; SourceTable = "CW Row"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(Id; Rec.Id) { }
            cuegroup(Wide) { CuegroupLayout = Wide; field(WideName; Rec.Name) { } field(WideAmt; Rec.Amt) { } field(WideCnt; Rec.Cnt) { } }
            cuegroup(Plain) { field(PlainName; Rec.Name) { } }
        }
    }
}
codeunit 69967 C
{
    local procedure Open(var P: TestPage "CW Card")
    var R: Record "CW Row";
    begin
        R.Id := 1; R.Name := 'Layout'; R.Amt := 42; R.Cnt := 3; R.Insert();
        P.OpenView();
    end;
    procedure F(): Text
    var P: TestPage "CW Card";
    begin
        Open(P);
        exit(P.WideName.Value() + '|' + P.WideAmt.Value() + '|' + P.PlainName.Value());
    end;
    procedure G(): Text
    var P: TestPage "CW Card";
    begin
        Open(P);
        exit(P.WideCnt.Value());
    end;
}
"#;
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "F")), "0|42.00|Layout");
    refused(al_testkit::run_in(src, "C", "G"), "Wide");
}
