//! A Guid handed as the filter text of SetFilter (corpus record/lt-y77-guid-filter/Y77GuidFilter.al Record_*Field `*_sf_g1`).

use al_testkit::text;

const SRC: &str = r#"
table 69980 "SG Row" { fields { field(1; Id; Integer) { } field(2; G; Guid) { } field(3; T; Text[50]) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 69981 C
{
    procedure F(): Text
    var R: Record "SG Row"; V: Guid; s: Text;
    begin
        Evaluate(V, '{1a2b3c4d-5e6f-7a8b-9c0d-1e2f3a4b5c6d}');
        R.Id := 1; R.G := V; R.T := Format(V); R.Insert();
        R.Init(); R.Id := 2; R.Insert();
        R.Reset(); R.SetFilter(G, V); s := R.GetFilter(G) + '|' + Format(R.Count());
        R.Reset(); R.SetFilter(T, V); s += '|' + R.GetFilter(T) + '|' + Format(R.Count());
        exit(s);
    end;
}
"#;

#[test]
fn a_guid_as_the_filter_text_is_its_braced_text() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "F")),
        "{1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D}|1|{1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D}|1"
    );
}
