//! Behaviors that Microsoft's Tests-SCM-Service, Tests-Misc, Tests-Marketing and Tests-Resource need (issues 150 and 151).
//! Each test names the Microsoft code that needs it and the assumption it rests on.

use al_test_cli::report::Outcome;

fn passes(objects: &str, tests: &str) {
    let src = format!("{objects}\ncodeunit 62900 T {{ Subtype = Test; {tests} }}");
    let o = al_testkit::outcomes_in(&src, "T");
    assert!(!o.is_empty());
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

const CAPTIONED: &str = r#"
enum 62901 "BS Usage" { Extensible = true; value(0; "S.Quote") { Caption = 'Sales Quote'; } value(1; "S.Order") { Caption = 'Sales Order'; } }
table 62902 "BS Selection" { fields { field(1; Id; Integer) { } field(2; Usage; Enum "BS Usage") { } } keys { key(PK; Id) { } } }
"#;

// Base Application's Report Selections.GetCustomReportSelectionByUsageFilter: SetFilter(Usage, <GetFilter(Usage)>), the caption.
// Pinned by record/lt-enum-filter-caption/EnumFilterByCaption.al (a filter by caption, by name, and the round trip).
#[test]
fn an_enum_filter_written_by_get_filter_finds_the_same_rows_again() {
    passes(
        CAPTIONED,
        "[Test] procedure CaptionRoundTrip() var R: Record \"BS Selection\"; S: Record \"BS Selection\"; begin
            R.DeleteAll(); R.Id := 1; R.Usage := R.Usage::\"S.Quote\"; R.Insert(); R.Id := 2; R.Usage := R.Usage::\"S.Order\"; R.Insert();
            R.SetRange(Usage, R.Usage::\"S.Quote\");
            if R.GetFilter(Usage) <> 'Sales Quote' then Error('filter <%1>', R.GetFilter(Usage));
            S.SetFilter(Usage, R.GetFilter(Usage));
            if S.Count() <> 1 then Error('count %1', S.Count());
            S.SetFilter(Usage, 'S.Order');
            if S.Count() <> 1 then Error('by name %1', S.Count());
        end;",
    );
}

const TWO_ENUMS: &str = r#"
enum 62910 "BS Status A" { Extensible = true; value(0; Open) { } value(1; Done) { } }
enum 62911 "BS Status B" { Extensible = true; value(0; Open) { } value(1; Done) { } }
table 62912 "BS Header" { fields { field(1; "No."; Code[10]) { } field(2; Status; Enum "BS Status A") { } } keys { key(PK; "No.") { } } }
table 62913 "BS Period" { fields { field(1; Id; Integer) { } field(2; "Header No."; Code[10]) { }
    field(3; "Header Status"; Enum "BS Status B") { FieldClass = FlowField; CalcFormula = lookup("BS Header".Status where("No." = field("Header No."))); } }
  keys { key(PK; Id) { } } }
"#;

// Base Application's VAT Return Period: "VAT Return Status" (an Enum) looks up "VAT Report Header".Status (another Enum).
// Pinned by record/lt-flowfield-lookup-enum/FlowFieldLookupOtherEnum.al (the looked-up member stays as it is).
#[test]
fn a_flowfield_looks_up_a_field_of_another_enum() {
    passes(
        TWO_ENUMS,
        "[Test] procedure LookupAcrossEnums() var H: Record \"BS Header\"; P: Record \"BS Period\"; begin
            H.DeleteAll(); P.DeleteAll();
            H.\"No.\" := 'H1'; H.Status := H.Status::Done; H.Insert();
            P.Id := 1; P.\"Header No.\" := 'H1'; P.Insert();
            P.CalcFields(\"Header Status\");
            if P.\"Header Status\" <> P.\"Header Status\"::Done then Error('status %1', P.\"Header Status\");
        end;",
    );
}

// Microsoft's Guided Experience tests (Tests-Misc NoGuidanceNotificationNonEvaluationCompany and 35 more) write
// `Notification.Id := '9d730988-ff4a-43ca-8b4f-80ee47000000'`; a Text assigns into a Guid the same way.
#[test]
fn a_notification_takes_its_id_from_a_text() {
    passes(
        "",
        "[Test] procedure IdFromText() var N: Notification; begin
            N.Id := '9d730988-ff4a-43ca-8b4f-80ee47000000';
            if Format(N.Id) <> '{9D730988-FF4A-43CA-8B4F-80EE47000000}' then Error('id %1', Format(N.Id));
        end;",
    );
}

// Library - Variable Storage.DequeueDecimal hands back what a test page handler enqueued as the control's text.
// Pinned by types/lt-variant-time-enum/VariantTextTimeEnumFilter.al:Variant_Text_ToDecimal.
#[test]
fn a_text_variant_converts_into_a_decimal() {
    passes(
        "",
        "[Test] procedure TextVariantToDecimal() var V: Variant; D: Decimal; begin
            V := '-111.69'; D := V;
            if D <> -111.69 then Error('d %1', D);
        end;",
    );
}
