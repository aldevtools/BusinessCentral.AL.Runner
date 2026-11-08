//! TestField.Lookup with a deferred refresh (`defer_lookup_refresh`) on the part layouts writer Y87 measured beyond issue 6806's:
//! four ListParts with the linked one second or third, the linked ListPart with two FactBox CardParts, and three ListParts with
//! two CardParts (corpus testpage/Y87LookupParts.al Y87_{ALBC,ABLC,LKM,ABLKM}_ReadEach and _ReadReverse: `resource no` `R2`,
//! every part `R1`). The fixture is the one of `lookup_deferred_refresh_many_parts.rs` with more hosts.

use al_testkit::outcomes_in;

fn host(id: u32, name: &str, content: &str, factboxes: &str) -> String {
    format!(
        r#"
page {id} "{name}"
{{
    PageType = List; SourceTable = "Repro Lookup Resource"; UsageCategory = None;
    layout {{
        area(Content) {{
        group(Filters) {{ field(ResourceNo; Setting."Resource No.") {{
            trigger OnLookup(var Text: Text): Boolean
            var Resource: Record "Repro Lookup Resource";
            begin
                if Page.RunModal(Page::"Repro Lookup Resource List", Resource) <> Action::LookupOK then exit(false);
                Text := Resource."No."; exit(true);
            end;
            trigger OnValidate() begin Setting.Modify(); CurrPage.Update(false); end;
        }} }}
        repeater(Lines) {{ field("No."; Rec."No.") {{ }} field(CurrentNo; CurrentNo) {{ }} }}
        {content}
        }}
        area(FactBoxes) {{ {factboxes} }}
    }}
    trigger OnOpenPage() begin if not Setting.Get() then Setting.Insert(); end;
    trigger OnAfterGetCurrRecord() begin CurrentNo := Rec."No."; end;
    var CurrentNo: Code[20]; Setting: Record "Repro Lookup Setting";
}}
"#
    )
}

const L: &str = r#"part(Resources; "Repro Lookup Resource Part") { SubPageLink = "No." = field("No."), "Group Code" = field("Group Code"), "Region Code" = field("Region Code"); }"#;
const KM: &str = r#"part(DetailsK; "Repro Lookup Card Part") { SubPageLink = "No." = field("No."); } part(DetailsM; "Repro Lookup Card Part") { SubPageLink = "No." = field("No."); }"#;

fn part(name: &str) -> String {
    format!(r#"part({name}; "Repro Lookup Resource Part") {{ }}"#)
}

fn test(name: &str, page: &str, parts: &[&str]) -> String {
    let reads: String = parts
        .iter()
        .map(|p| format!(" AssertPart(Schedule.{p}.\"No.\".Value());"))
        .collect();
    format!(
        r#"
    [Test] [HandlerFunctions('PickSecondResource')]
    procedure {name}()
    var Schedule: TestPage "{page}";
    begin
        Setup(); Schedule.OpenEdit(); Schedule.ResourceNo.Lookup();
        AssertPicked(Schedule.ResourceNo.Value());{reads}
        Schedule.Close();
    end;"#
    )
}

#[test]
fn a_lookup_on_the_layouts_y87_measured_shows_the_picked_value() {
    let base = super::lookup_deferred_refresh_many_parts::SRC;
    let tests_at = base.find("codeunit 50110").expect("the test codeunit");
    let (a, b) = (part("OtherA"), part("OtherB"));
    let c = part("OtherC");
    let mut src = base[..tests_at].to_string();
    src += &host(50130, "Repro A L B C Schedule", &format!("{a} {L} {b} {c}"), "");
    src += &host(50131, "Repro A B L C Schedule", &format!("{a} {b} {L} {c}"), "");
    src += &host(50132, "Repro L K M Schedule", L, KM);
    src += &host(50133, "Repro A B L K M Schedule", &format!("{a} {b} {L}"), KM);
    let body = test(
        "PartsALBC",
        "Repro A L B C Schedule",
        &["OtherA", "Resources", "OtherB", "OtherC"],
    ) + &test(
        "PartsABLC",
        "Repro A B L C Schedule",
        &["OtherA", "OtherB", "Resources", "OtherC"],
    ) + &test(
        "PartsLKM",
        "Repro L K M Schedule",
        &["Resources", "DetailsK", "DetailsM"],
    ) + &test(
        "PartsABLKM",
        "Repro A B L K M Schedule",
        &["OtherA", "OtherB", "Resources", "DetailsK", "DetailsM"],
    );
    let tail = &base[tests_at..];
    let open = tail.find("[ModalPageHandler]").expect("the handler");
    src += &tail[..open].replace(
        "codeunit 50110 \"Repro Lookup Link Tests\"",
        "codeunit 50140 \"Repro More Layouts\"",
    );
    // Keep only the shared helpers of the codeunit: drop its own tests, add these.
    let start = src.find("codeunit 50140").unwrap();
    let first_test = src[start..].find("    [Test]").unwrap() + start;
    src.truncate(first_test);
    src += &body;
    src += "\n    ";
    src += &tail[open..];
    let o = outcomes_in(&src, "Repro More Layouts");
    assert_eq!(o.len(), 4, "{o:?}");
    for (n, x) in &o {
        assert_eq!(format!("{x:?}"), "Pass", "{n}");
    }
}
