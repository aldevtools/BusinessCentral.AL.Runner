//! `field(filter(F))` in a CalcFormula over a FlowFilter F: the caller's filter on F applies (record/flowfields/
//! FfmSumAverage.al:66, :780-844; FfmCountExist.al:66, :640-712; al-runner issue 7433). On a Date FlowFilter the forms of
//! record/flowfields/FlowFilterDateFilterForms.al are pinned (the corpus runs them); `>=` is not.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69980 "FFD Doc"
{
    fields { field(1; Id; Integer) { } field(2; Kind; Code[10]) { } field(3; Day; Date) { } field(4; Qty; Integer) { } field(5; At; Time) { } }
    keys { key(PK; Id) { } }
}
table 69981 "FFD Cue"
{
    fields
    {
        field(1; PK; Code[10]) { }
        field(20; KindF; Code[10]) { FieldClass = FlowFilter; }
        field(30; Docs; Integer) { FieldClass = FlowField; CalcFormula = count("FFD Doc" where(Kind = field(KindF), Day = field(filter(DateF)))); }
        field(31; ByQty; Integer) { FieldClass = FlowField; CalcFormula = count("FFD Doc" where(Qty = field(filter(QtyF)))); }
        field(32; ByKind; Integer) { FieldClass = FlowField; CalcFormula = count("FFD Doc" where(Kind = field(filter(KindF)))); }
        field(33; ByTime; Integer) { FieldClass = FlowField; CalcFormula = count("FFD Doc" where(At = field(filter(TimeF)))); }
        field(50; DateF; Date) { FieldClass = FlowFilter; }
        field(51; QtyF; Integer) { FieldClass = FlowFilter; }
        field(52; TimeF; Time) { FieldClass = FlowFilter; }
        field(60; Txt; Text[100]) { }
        field(61; ByTxt; Integer) { FieldClass = FlowField; CalcFormula = count("FFD Doc" where(Day = field(filter(Txt)))); }
        field(62; Cd; Code[50]) { }
        field(63; ByCd; Integer) { FieldClass = FlowField; CalcFormula = count("FFD Doc" where(Day = field(filter(Cd)))); }
    }
    keys { key(K; PK) { } }
}
codeunit 69982 T
{
    local procedure Seed(var C: Record "FFD Cue")
    begin
        Add(1, 'A', 20260110D, 1); Add(2, 'A', 20260210D, 3); Add(3, 'A', 20260310D, 5); Add(4, 'B', 20260210D, 7);
        C.PK := 'X'; C.Insert();
    end;
    local procedure Add(Id: Integer; K: Code[10]; D: Date; Q: Integer)
    var R: Record "FFD Doc";
    begin
        R.Id := Id; R.Kind := K; R.Day := D; R.Qty := Q; R.Insert();
    end;

    procedure UnsetAndRange(): Text
    var C: Record "FFD Cue"; T: Text;
    begin
        Seed(C);
        C.CalcFields(Docs, ByQty, ByKind);
        T := Format(C.Docs) + Format(C.ByQty) + Format(C.ByKind) + '|';
        C.SetFilter(KindF, 'A');
        C.SetRange(DateF, 20260201D, 20260430D);
        C.SetFilter(QtyF, '3..5');
        C.CalcFields(Docs, ByQty, ByKind);
        exit(T + Format(C.Docs) + Format(C.ByQty) + Format(C.ByKind));
    end;

    procedure AtLeast(): Integer
    var C: Record "FFD Cue";
    begin
        Seed(C);
        C.SetFilter(DateF, '>=%1', 20260210D);
        C.CalcFields(Docs);
        exit(C.Docs);
    end;

    procedure ByText(Value: Text[100]): Integer
    var C: Record "FFD Cue";
    begin
        Seed(C);
        C.Txt := Value;
        C.CalcFields(ByTxt);
        exit(C.ByTxt);
    end;

    procedure ByCode(Value: Code[50]): Integer
    var C: Record "FFD Cue";
    begin
        Seed(C);
        C.Cd := Value;
        C.CalcFields(ByCd);
        exit(C.ByCd);
    end;

    procedure TimeFilter(): Integer
    var C: Record "FFD Cue";
    begin
        Seed(C);
        C.SetRange(TimeF, 100000T);
        C.CalcFields(ByTime);
        exit(C.ByTime);
    end;
}
"#;

#[test]
fn an_unset_flowfilter_imposes_nothing_and_a_set_one_applies() {
    assert_eq!(text(run_in(SRC, "T", "UnsetAndRange")), "444|223");
}

// `>=` on a Date FlowFilter is not among the pinned forms.
#[test]
fn an_at_least_filter_on_a_date_flowfilter_stays_refused() {
    let r = run_in(SRC, "T", "AtLeast");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn a_flowfilter_of_a_type_no_test_pins_stays_refused() {
    let r = run_in(SRC, "T", "TimeFilter");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}

fn with_text(routine: &str, value: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let mut b = al_testkit::Built::from_source(SRC);
    let (oi, ri) = b.routine_in("T", routine);
    b.call(oi, ri, vec![al_builtins::value::Value::Text(value.into())])
        .map(|(v, _)| v)
}

// A Text field over a Date source field: one date, a range, an open range, two quotes and `abc` (an error) are pinned
// (record/flowfields/FlowFilterDateFilterForms.al:342-353). A text of digits only, a date formula and any text of a Code field
// are not: they keep the earlier refusal.
#[test]
fn a_text_field_date_filter_serves_the_pinned_forms() {
    let n = |v: &str| match with_text("ByText", v) {
        Ok(al_builtins::value::Value::Integer(n)) => n,
        other => panic!("{v}: {other:?}"),
    };
    assert_eq!(n("02/10/26"), 2);
    assert_eq!(n("01/10/26..02/10/26"), 3);
    assert_eq!(n("03/10/26.."), 1);
    assert!(matches!(
        with_text("ByText", "abc"),
        Err(al_builtins::native::Signal::Error(_))
    ));
}

#[test]
fn a_text_field_date_filter_of_an_unpinned_form_stays_refused() {
    for (routine, v) in [("ByText", "021026"), ("ByText", "t"), ("ByCode", "ABC")] {
        let r = with_text(routine, v);
        assert!(
            matches!(r, Err(al_builtins::native::Signal::Unsupported(_))),
            "{routine} {v}: {r:?}"
        );
    }
}
