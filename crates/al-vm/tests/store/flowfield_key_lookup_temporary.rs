//! A key that starts with a Lookup FlowField, on a temporary record and with SetAutoCalcFields (al-runner issue 7427). Pinned:
//! on a temporary record, SetCurrentKey with a Boolean lookup first and its answer used answers false (record/keys-sorting/
//! KeyLookupFlowFieldFirst.al:383-386), and with a Text lookup first as a statement raises "Sorting cannot be done"
//! (SetCurrentKeyFlowFieldOrder.al:316); SetAutoCalcFields under a lookup key (KeyLookupFlowFieldFirst.al:351-352). Every other
//! lookup type, sign and use on a temporary record, and SetAutoCalcFields under a sum or count key, keep the refusal.

use al_testkit::run_in;

const SRC: &str = r#"
table 69940 "LKT Setup"
{
    fields { field(1; "Code"; Code[10]) { } field(2; Flag; Boolean) { } field(3; Rank; Integer) { } field(4; Name; Text[20]) { } }
    keys { key(PK; "Code") { } }
}
table 69941 "LKT Part"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Type Code"; Code[10]) { }
        field(10; Flag; Boolean) { FieldClass = FlowField; CalcFormula = lookup("LKT Setup".Flag where("Code" = field("Type Code"))); }
        field(11; Rank; Integer) { FieldClass = FlowField; CalcFormula = lookup("LKT Setup".Rank where("Code" = field("Type Code"))); }
        field(12; SetupName; Text[20]) { FieldClass = FlowField; CalcFormula = lookup("LKT Setup".Name where("Code" = field("Type Code"))); }
        field(13; SetupCode; Code[10]) { FieldClass = FlowField; CalcFormula = lookup("LKT Setup"."Code" where("Code" = field("Type Code"))); }
        field(14; NegRank; Integer) { FieldClass = FlowField; CalcFormula = -lookup("LKT Setup".Rank where("Code" = field("Type Code"))); }
        field(20; Lines; Integer) { FieldClass = FlowField; CalcFormula = count("LKT Setup" where("Code" = field("Type Code"))); }
        field(21; Total; Integer) { FieldClass = FlowField; CalcFormula = sum("LKT Setup".Rank where("Code" = field("Type Code"))); }
    }
    keys { key(PK; Id) { } }
}
codeunit 69942 C
{
    local procedure Seed(var T: Record "LKT Part" temporary)
    var S: Record "LKT Setup";
    begin
        S."Code" := 'S1'; S.Flag := true; S.Rank := 2; S.Name := 'b'; S.Insert();
        S."Code" := 'S2'; S.Flag := false; S.Rank := 1; S.Name := 'a'; S.Insert();
        T.Id := 1; T."Type Code" := 'S1'; T.Insert();
        T.Id := 2; T."Type Code" := 'S2'; T.Insert();
    end;

    procedure BooleanUsed(): Boolean
    var T: Record "LKT Part" temporary;
    begin
        Seed(T);
        exit(T.SetCurrentKey(Flag, Id));
    end;

    procedure TextStatement(): Text
    var T: Record "LKT Part" temporary;
    begin
        Seed(T);
        asserterror T.SetCurrentKey(SetupName, Id);
        exit(GetLastErrorText());
    end;

    procedure CodeUsed(): Boolean
    var T: Record "LKT Part" temporary;
    begin
        Seed(T);
        exit(T.SetCurrentKey(SetupCode, Id));
    end;

    procedure NegatedUsed(): Boolean
    var T: Record "LKT Part" temporary;
    begin
        Seed(T);
        exit(T.SetCurrentKey(NegRank, Id));
    end;

    procedure BooleanStatement(): Text
    var T: Record "LKT Part" temporary;
    begin
        Seed(T);
        asserterror T.SetCurrentKey(Flag, Id);
        exit(GetLastErrorText());
    end;

    procedure TextUsed(): Boolean
    var T: Record "LKT Part" temporary;
    begin
        Seed(T);
        exit(T.SetCurrentKey(SetupName, Id));
    end;

    local procedure AutoCalc(var P: Record "LKT Part"): Text
    var Line: Text;
    begin
        P.SetAutoCalcFields(Rank);
        if P.FindSet() then
            repeat
                Line += Format(P.Id) + ':' + Format(P.Rank) + ' ';
            until P.Next() = 0;
        exit(Line);
    end;

    local procedure SeedStored()
    var S: Record "LKT Setup"; P: Record "LKT Part";
    begin
        S."Code" := 'S1'; S.Flag := true; S.Rank := 2; S.Insert();
        S."Code" := 'S2'; S.Flag := false; S.Rank := 1; S.Insert();
        P.Id := 1; P."Type Code" := 'S1'; P.Insert();
        P.Id := 2; P."Type Code" := 'S2'; P.Insert();
    end;

    procedure AutoCalcLookupKey(): Text
    var P: Record "LKT Part";
    begin
        SeedStored();
        P.SetCurrentKey(Flag, Id);
        exit(AutoCalc(P));
    end;

    procedure AutoCalcCountKey(): Text
    var P: Record "LKT Part";
    begin
        SeedStored();
        P.SetCurrentKey(Lines, Id);
        exit(AutoCalc(P));
    end;

    procedure AutoCalcSumKey(): Text
    var P: Record "LKT Part";
    begin
        SeedStored();
        P.SetCurrentKey(Total, Id);
        exit(AutoCalc(P));
    end;
}
"#;

fn refused(routine: &str) {
    let r = run_in(SRC, "C", routine);
    assert!(
        matches!(r, Err(al_builtins::native::Signal::Unsupported(_))),
        "{routine}: {r:?}"
    );
}

#[test]
fn the_pinned_temporary_lookup_keys_answer_as_bc() {
    assert!(matches!(
        run_in(SRC, "C", "BooleanUsed"),
        Ok(al_builtins::value::Value::Boolean(false))
    ));
    assert_eq!(
        al_testkit::text(run_in(SRC, "C", "TextStatement")),
        "Sorting cannot be done based on the SetupName field(s) in the LKT Part table."
    );
}

#[test]
fn a_temporary_lookup_key_of_another_type_sign_or_use_stays_refused() {
    for routine in ["CodeUsed", "NegatedUsed", "BooleanStatement", "TextUsed"] {
        refused(routine);
    }
}

#[test]
fn set_auto_calc_fields_runs_under_a_lookup_key_only() {
    assert_eq!(al_testkit::text(run_in(SRC, "C", "AutoCalcLookupKey")), "2:1 1:2 ");
    for routine in ["AutoCalcCountKey", "AutoCalcSumKey"] {
        refused(routine);
    }
}
