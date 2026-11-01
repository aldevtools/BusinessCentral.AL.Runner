//! TestPage SetValue on controls over TABLE fields that declare ValuesAllowed (al-runner issue 6672; corpus writer Y65,
//! pages/lt-y65-values-allowed-table-field/Y65ValuesAllowedTableField.al:236-574 on corpus branch al-runtime-rs-staging-y65, not
//! pinned yet): an Enum with gaps in its ordinals, an Option, an Integer and a Decimal, and an Enum field whose control declares
//! a ValuesAllowed of its own, on a NavigatePage over a temporary table, a Card and a List. Each case opens a fresh page.

use al_testkit::text;

const OBJECTS: &str = r#"
enum 69980 "VT Mode"
{
    Extensible = true;
    value(0; All) { }
    value(2; "Mapped Only") { }
    value(3; None) { }
    value(4; "Published Only") { }
}
table 69980 "VT Settings"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; "Push Mode"; Enum "VT Mode") { ValuesAllowed = "Mapped Only", None; }
        field(3; Opt; Option) { OptionMembers = A,B,C,D; ValuesAllowed = B, D; }
        field(4; Int; Integer) { ValuesAllowed = 1, 5; }
        field(5; Dec; Decimal) { ValuesAllowed = 1.5, 2; }
        field(6; "Pull Mode"; Enum "VT Mode") { ValuesAllowed = "Mapped Only", None; }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}
page 69981 "VT Wizard"
{
    PageType = NavigatePage; SourceTable = "VT Settings"; SourceTableTemporary = true; ApplicationArea = All;
    layout
    {
        area(Content)
        {
            field("Push Mode"; Rec."Push Mode") { }
            field(Opt; Rec.Opt) { }
            field(Int; Rec.Int) { }
            field(Dec; Rec.Dec) { }
            field("Pull Mode"; Rec."Pull Mode") { ValuesAllowed = All, None; }
        }
    }
    trigger OnOpenPage()
    begin
        Rec.Init();
        Rec."Code" := 'A';
        Rec."Push Mode" := Rec."Push Mode"::"Mapped Only";
        Rec.Insert();
    end;
}
page 69982 "VT Card"
{
    PageType = Card; SourceTable = "VT Settings"; ApplicationArea = All;
    layout
    {
        area(Content)
        {
            field("Code"; Rec."Code") { }
            field("Push Mode"; Rec."Push Mode") { }
            field(Opt; Rec.Opt) { }
            field(Int; Rec.Int) { }
            field(Dec; Rec.Dec) { }
            field("Pull Mode"; Rec."Pull Mode") { ValuesAllowed = All, None; }
        }
    }
}
page 69983 "VT List"
{
    PageType = List; SourceTable = "VT Settings"; ApplicationArea = All;
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Code"; Rec."Code") { }
                field("Push Mode"; Rec."Push Mode") { }
                field(Opt; Rec.Opt) { }
                field(Int; Rec.Int) { }
                field(Dec; Rec.Dec) { }
                field("Pull Mode"; Rec."Pull Mode") { ValuesAllowed = All, None; }
            }
        }
    }
}
"#;

/// The test codeunit for page `page` ("VT Wizard", "VT Card" or "VT List"): `Case(Ctl, V)` opens a fresh page, sets control
/// `Ctl` to `V` and answers `ok:<Value>` or `error:<text>|<code>|now <Value>`, then ` stored=<field>` on a stored row.
fn codeunit(page: &str, stored: bool) -> String {
    let seed = if stored {
        "Seed(Settings); P.OpenEdit(); P.GoToRecord(Settings);"
    } else {
        "P.OpenEdit();"
    };
    let tail = if stored { "R += ' stored=' + Stored(Ctl);" } else { "" };
    format!(
        r#"
codeunit 69984 C
{{
    var Mode: Enum "VT Mode";
    local procedure Seed(var Settings: Record "VT Settings")
    begin
        Settings.DeleteAll();
        Settings.Init();
        Settings."Code" := 'A';
        Settings."Push Mode" := Settings."Push Mode"::"Mapped Only";
        Settings.Opt := Settings.Opt::B;
        Settings.Int := 1;
        Settings.Dec := 1.5;
        Settings."Pull Mode" := Settings."Pull Mode"::"Mapped Only";
        Settings.Insert();
    end;
    local procedure Stored(Ctl: Text): Text
    var Settings: Record "VT Settings";
    begin
        Settings.Get('A');
        case Ctl of
            'push': exit(Format(Settings."Push Mode"));
            'opt': exit(Format(Settings.Opt));
            'int': exit(Format(Settings.Int));
            'dec': exit(Format(Settings.Dec));
            'pull': exit(Format(Settings."Pull Mode"));
        end;
    end;
    [TryFunction]
    local procedure TrySet(var P: TestPage "{page}"; Ctl: Text; V: Variant)
    begin
        case Ctl of
            'push': P."Push Mode".SetValue(V);
            'opt': P.Opt.SetValue(V);
            'int': P.Int.SetValue(V);
            'dec': P.Dec.SetValue(V);
            'pull': P."Pull Mode".SetValue(V);
        end;
    end;
    local procedure GetCtl(var P: TestPage "{page}"; Ctl: Text): Text
    begin
        case Ctl of
            'push': exit(P."Push Mode".Value());
            'opt': exit(P.Opt.Value());
            'int': exit(P.Int.Value());
            'dec': exit(P.Dec.Value());
            'pull': exit(P."Pull Mode".Value());
        end;
    end;
    [TryFunction]
    local procedure TryClose(var P: TestPage "{page}")
    begin
        P.Close();
    end;
    local procedure Run1(Ctl: Text; V: Variant) R: Text
    var Settings: Record "VT Settings"; P: TestPage "{page}";
    begin
        {seed}
        ClearLastError();
        if TrySet(P, Ctl, V) then
            R := 'ok:' + GetCtl(P, Ctl)
        else
            R := 'error:' + GetLastErrorText() + '|' + GetLastErrorCode() + '|now ' + GetCtl(P, Ctl);
        ClearLastError();
        if not TryClose(P) then
            R += ' close=error:' + GetLastErrorText() + '|' + GetLastErrorCode();
        {tail}
    end;
    procedure EnumValues(): Text
    begin
        exit(Run1('push', Mode::None) + '#' + Run1('push', Mode::All) + '#' + Run1('push', Mode::"Published Only") + '#'
            + Run1('push', Mode::"Mapped Only"));
    end;
    procedure EnumTextAndInteger(): Text
    begin
        exit(Run1('push', 'None') + '#' + Run1('push', 'published only') + '#' + Run1('push', 3) + '#' + Run1('push', 1) + '#'
            + Run1('push', 0));
    end;
    procedure OptionAndInteger(): Text
    begin
        exit(Run1('opt', 'D') + '#' + Run1('opt', 'A') + '#' + Run1('opt', 1) + '#' + Run1('opt', 2) + '#' + Run1('int', 5) + '#'
            + Run1('int', 3) + '#' + Run1('int', '1'));
    end;
    procedure DecimalAndControlList(): Text
    begin
        exit(Run1('dec', 2) + '#' + Run1('dec', 1.7) + '#' + Run1('pull', Mode::All) + '#' + Run1('pull', Mode::"Mapped Only") + '#'
            + Run1('pull', Mode::None) + '#' + Run1('pull', Mode::"Published Only"));
    end;
}}
"#
    )
}

fn run(page: &str, stored: bool, routine: &str) -> Vec<String> {
    let src = format!("{OBJECTS}{}", codeunit(page, stored));
    text(al_testkit::run_in(&src, "C", routine))
        .split('#')
        .map(str::to_string)
        .collect()
}

/// `Validation error for Field: <f>,  Message = '<m><suffix>'|TestValidation|now <now>`.
fn invalid(field: &str, msg: &str, suffix: &str, now: &str) -> String {
    format!("error:Validation error for Field: {field},  Message = '{msg}{suffix}'|TestValidation|now {now}")
}

fn not_acceptable(field: &str, entry: &str, suffix: &str) -> String {
    invalid(
        field,
        &format!("Your entry of '{entry}' is not an acceptable value for '{field}'."),
        suffix,
        entry,
    )
}

fn not_evaluated(text: &str, now: &str) -> String {
    format!("error:The value \"{text}\" can't be evaluated into type Option.|Evaluate:Format|now {now}")
}

const REFRESH: &str = " (Select Refresh to discard errors)";

/// Y65ValuesAllowedTableField.al:236-306 (the Wizard tests): no Refresh hint, nothing stored.
#[test]
fn a_navigate_page_over_table_fields_with_values_allowed() {
    let w = |r| run("VT Wizard", false, r);
    assert_eq!(
        w("EnumValues"),
        [
            "ok:None".into(),
            not_acceptable("Push Mode", "2", ""),
            not_acceptable("Push Mode", "2", ""),
            "ok:Mapped Only".into()
        ]
    );
    assert_eq!(
        w("EnumTextAndInteger"),
        [
            not_evaluated("None", "Mapped Only"),
            not_evaluated("published only", "Mapped Only"),
            "ok:None".into(),
            not_acceptable("Push Mode", "2", ""),
            not_acceptable("Push Mode", "2", "")
        ]
    );
    assert_eq!(
        w("OptionAndInteger"),
        [
            not_evaluated("D", "0"),
            not_evaluated("A", "0"),
            "ok:B".into(),
            not_acceptable("Opt", "0", ""),
            "ok:5".into(),
            invalid(
                "Int",
                "Value '3' is not a valid value. The following values can be used: 1;5.",
                "",
                "0"
            ),
            "ok:1".into()
        ]
    );
    assert_eq!(
        w("DecimalAndControlList"),
        [
            invalid(
                "Dec",
                "Value '2.00' is not a valid value. The following values can be used: 1.5;2.",
                "",
                "0.00"
            ),
            invalid(
                "Dec",
                "Value '1.70' is not a valid value. The following values can be used: 1.5;2.",
                "",
                "0.00"
            ),
            "ok:All".into(),
            not_acceptable("Pull Mode", "0", ""),
            "ok:None".into(),
            not_acceptable("Pull Mode", "0", "")
        ]
    );
}

/// Y65ValuesAllowedTableField.al:351-421 (the Card tests): the Refresh hint, and the stored row.
#[test]
fn a_card_over_table_fields_with_values_allowed() {
    let c = |r| run("VT Card", true, r);
    let st = |s: String, v: &str| format!("{s} stored={v}");
    assert_eq!(
        c("EnumValues"),
        [
            "ok:None stored=None".into(),
            st(not_acceptable("Push Mode", "2", REFRESH), "Mapped Only"),
            st(not_acceptable("Push Mode", "2", REFRESH), "Mapped Only"),
            "ok:Mapped Only stored=Mapped Only".into()
        ]
    );
    assert_eq!(
        c("EnumTextAndInteger"),
        [
            st(not_evaluated("None", "Mapped Only"), "Mapped Only"),
            st(not_evaluated("published only", "Mapped Only"), "Mapped Only"),
            "ok:None stored=None".into(),
            st(not_acceptable("Push Mode", "2", REFRESH), "Mapped Only"),
            st(not_acceptable("Push Mode", "2", REFRESH), "Mapped Only")
        ]
    );
    assert_eq!(
        c("OptionAndInteger"),
        [
            st(not_evaluated("D", "B"), "B"),
            st(not_evaluated("A", "B"), "B"),
            "ok:B stored=B".into(),
            st(not_acceptable("Opt", "1", REFRESH), "B"),
            "ok:5 stored=5".into(),
            st(
                invalid(
                    "Int",
                    "Value '3' is not a valid value. The following values can be used: 1;5.",
                    REFRESH,
                    "1"
                ),
                "1"
            ),
            "ok:1 stored=1".into()
        ]
    );
    assert_eq!(
        c("DecimalAndControlList"),
        [
            st(
                invalid(
                    "Dec",
                    "Value '2.00' is not a valid value. The following values can be used: 1.5;2.",
                    REFRESH,
                    "1.50"
                ),
                "1.5"
            ),
            st(
                invalid(
                    "Dec",
                    "Value '1.70' is not a valid value. The following values can be used: 1.5;2.",
                    REFRESH,
                    "1.50"
                ),
                "1.5"
            ),
            "ok:All stored=All".into(),
            st(not_acceptable("Pull Mode", "2", REFRESH), "Mapped Only"),
            "ok:None stored=None".into(),
            st(not_acceptable("Pull Mode", "2", REFRESH), "Mapped Only")
        ]
    );
}

/// Y65ValuesAllowedTableField.al:470-540 (the List tests): no Refresh hint, the stored row.
#[test]
fn a_list_over_table_fields_with_values_allowed() {
    let l = |r| run("VT List", true, r);
    let st = |s: String, v: &str| format!("{s} stored={v}");
    assert_eq!(
        l("EnumValues"),
        [
            "ok:None stored=None".into(),
            st(not_acceptable("Push Mode", "2", ""), "Mapped Only"),
            st(not_acceptable("Push Mode", "2", ""), "Mapped Only"),
            "ok:Mapped Only stored=Mapped Only".into()
        ]
    );
    assert_eq!(
        l("EnumTextAndInteger"),
        [
            st(not_evaluated("None", "Mapped Only"), "Mapped Only"),
            st(not_evaluated("published only", "Mapped Only"), "Mapped Only"),
            "ok:None stored=None".into(),
            st(not_acceptable("Push Mode", "2", ""), "Mapped Only"),
            st(not_acceptable("Push Mode", "2", ""), "Mapped Only")
        ]
    );
    assert_eq!(
        l("OptionAndInteger"),
        [
            st(not_evaluated("D", "B"), "B"),
            st(not_evaluated("A", "B"), "B"),
            "ok:B stored=B".into(),
            st(not_acceptable("Opt", "1", ""), "B"),
            "ok:5 stored=5".into(),
            st(
                invalid(
                    "Int",
                    "Value '3' is not a valid value. The following values can be used: 1;5.",
                    "",
                    "1"
                ),
                "1"
            ),
            "ok:1 stored=1".into()
        ]
    );
    assert_eq!(
        l("DecimalAndControlList"),
        [
            st(
                invalid(
                    "Dec",
                    "Value '2.00' is not a valid value. The following values can be used: 1.5;2.",
                    "",
                    "1.50"
                ),
                "1.5"
            ),
            st(
                invalid(
                    "Dec",
                    "Value '1.70' is not a valid value. The following values can be used: 1.5;2.",
                    "",
                    "1.50"
                ),
                "1.5"
            ),
            "ok:All stored=All".into(),
            st(not_acceptable("Pull Mode", "2", ""), "Mapped Only"),
            "ok:None stored=None".into(),
            st(not_acceptable("Pull Mode", "2", ""), "Mapped Only")
        ]
    );
}
