//! SetLoadFields and AddLoadFields with FlowFields and FlowFilters among several fields, the function forms' answers and
//! AreFieldsLoaded of a FlowField after a whole-row fetch. Every literal is what real BC answered in the corpus:
//! record/lt-y67-addloadfields-flow/Y67AddLoadFlow.al (codeunit 93628, writer Y67).

use al_testkit::text;

const SRC: &str = r#"
table 50200 LR
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Qty; Decimal) { }
        field(4; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count(LL where("Row Id" = field(Id))); }
        field(6; Flt; Integer) { FieldClass = FlowFilter; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50201 LL
{
    fields
    {
        field(1; "Row Id"; Integer) { }
        field(2; "Line No."; Integer) { }
    }
    keys { key(PK; "Row Id", "Line No.") { Clustered = true; } }
}

codeunit 50200 C
{
    local procedure Seed()
    var
        Row: Record LR;
    begin
        Row.Id := 1;
        Row.Name := 'one';
        Row.Insert();
    end;

    local procedure Flags(var Row: Record LR): Text
    begin
        exit(Format(Row.AreFieldsLoaded(Row.Id)) + Format(Row.AreFieldsLoaded(Row.Name)) + Format(Row.AreFieldsLoaded(Row.Qty)) + Format(Row.AreFieldsLoaded(Row.Cnt)) + Format(Row.AreFieldsLoaded(Row.Flt)));
    end;

    [TryFunction]
    local procedure AddStatement(var Row: Record LR; Which: Integer)
    begin
        case Which of
            1:
                Row.AddLoadFields(Row.Name, Row.Cnt);
            2:
                Row.AddLoadFields(Row.Flt, Row.Name);
        end;
    end;

    procedure NoRequest(): Text
    var
        Row: Record LR;
        Txt: Text;
    begin
        Seed();
        Txt := Format(Row.SetLoadFields(Row.Name, Row.Qty)) + Format(Row.SetLoadFields(Row.Name)) + ' ';
        Clear(Row);
        Txt += Format(Row.AddLoadFields()) + Format(Row.AddLoadFields(Row.Name)) + Format(Row.AddLoadFields(Row.Cnt, Row.Name)) + ' ';
        if not AddStatement(Row, 1) then
            Txt += GetLastErrorCode() + ':' + GetLastErrorText() + ' ';
        if not AddStatement(Row, 2) then
            Txt += GetLastErrorCode() + ':' + GetLastErrorText() + ' ';
        Row.FindFirst();
        exit(Txt + Flags(Row));
    end;

    procedure AfterSetName(): Text
    var
        Row: Record LR;
        Txt: Text;
    begin
        Seed();
        Row.SetLoadFields(Row.Name);
        Txt := Format(Row.AddLoadFields(Row.Name, Row.Cnt, Row.Qty)) + Format(Row.SetLoadFields(Row.Name, Row.Cnt)) + Format(Row.SetLoadFields(Row.Cnt)) + ' ';
        Row.SetLoadFields(Row.Name);
        Row.FindFirst();
        exit(Txt + Flags(Row));
    end;
}
"#;

/// Record_NoRequest_SetFunction `5 Name+Qty` No, `2 Name` Yes; Record_AfterSetEmpty_AddFunction `1 none` Yes, `2 Name` No, `7 Cnt+Name`
/// No; Record_*_AddStatement `6 Name+Cnt` raised Cnt, `12 Flt+Name+Tot` raised Flt (DB:Ok); `1 none` loaded=YesYesYesYesYes.
#[test]
fn without_a_request() {
    let msg = |f: &str| {
        format!(
            "DB:Ok:The field '{f}' in the 'LR' table is not valid for auto loading and cannot be passed to calls to AddLoadFields, SetLoadFields, or LoadFields. "
        )
    };
    assert_eq!(
        text(al_testkit::run(SRC, "NoRequest")),
        format!("NoYes YesNoNo {}{}YesYesYesYesYes", msg("Cnt"), msg("Flt"))
    );
}

/// Record_AfterSetName_AddFunction `9 Name+Cnt+Qty` No (nothing added), Record_AfterSetName_SetFunction `6 Name+Cnt` No, `3 Cnt` Yes;
/// Record_AfterSetName_SetStatement `2 Name` loaded=YesYesNoYesYes.
#[test]
fn after_set_load_fields_of_name() {
    assert_eq!(text(al_testkit::run(SRC, "AfterSetName")), "NoNoYes YesYesNoYesYes");
}
