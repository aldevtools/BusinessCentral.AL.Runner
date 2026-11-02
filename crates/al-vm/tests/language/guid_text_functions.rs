//! A Guid passed to the Text parameter of a string function is its upper-case text form
//! (types/lt-guid-as-text-argument/GuidAsTextArgument.al:108-146, Guid_AsTextArgument_MoreStringFunctions).

use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 70970 C
{
    procedure All(): Text
    var G: Guid; T: Text; R: Text;
    begin
        Evaluate(G, '{1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D}');
        R := DelStr(G, 1, 1) + '|' + DelStr(G, 10) + '|' + PadStr(G, 40, '*') + '|' + PadStr(G, 5) + '|';
        R += ConvertStr(G, 'ABC-', 'xyz_') + '|' + IncStr(G) + '|' + SelectStr(1, G) + '|' + InsStr(G, 'x', 2) + '|';
        R += DelChr(G, '=', '{}-') + '|';
        T := 'k{1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D}k';
        R += Format(T.Contains(G)) + Format(LowerCase(T).Contains(G)) + Format(T.IndexOf(G)) + Format(T.StartsWith(G));
        R += '|' + T.Replace(G, '#');
        exit(R);
    end;
}
"#;

#[test]
fn string_functions_read_a_guid_as_its_text() {
    let g = "{1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D}";
    let expected = [
        "1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D}",
        "{1A2B3C4D",
        "{1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D}**",
        "{1A2B",
        "{1x2y3z4D_5E6F_7x8y_9z0D_1E2F3x4y5z6D}",
        "{1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C7D}",
        g,
        "{x1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D}",
        "1A2B3C4D5E6F7A8B9C0D1E2F3A4B5C6D",
        "YesNo2No",
        "k#k",
    ]
    .join("|");
    assert_eq!(text(run_in(SRC, "C", "All")), expected);
}

/// EndsWith, LastIndexOf, Split, TrimStart, TrimEnd and IndexOfAny read a Guid as its text too
/// (misc/lt-y70-assumptions/Y70Probes.al:264-311, GuidAsText_OtherFunctions).
const MORE: &str = r#"
codeunit 70971 C
{
    procedure All(): Text
    var G: Guid; S: Text; Parts: List of [Text]; R: Text;
    begin
        Evaluate(G, '{1a2b3c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d}');
        S := 'x{1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D}y{1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D}';
        R := Format(('ab' + Format(G)).EndsWith(G)) + '|' + Format(S.LastIndexOf(G)) + '|';
        Parts := S.Split(G);
        R += Format(Parts.Count()) + Parts.Get(1) + Parts.Get(2) + Parts.Get(3) + '|';
        R += Format(StrLen(Format(G).TrimStart(G))) + Format(StrLen(Format(G).TrimEnd(G))) + '|' + Format(S.IndexOfAny(G));
        exit(R);
    end;
}
"#;

#[test]
fn more_string_functions_read_a_guid_as_its_text() {
    assert_eq!(text(run_in(MORE, "C", "All")), "Yes|41|3xy|00|2");
}
